//! Constant-folding pass for the Closure Compiler clone.
//!
//! First concrete optimization pass plugged into the
//! [`coding_adventures_closure_pass_pipeline::Pass`] trait per
//! [CLOC06's canonical pass set](../../../specs/CLOC06-pass-interface-contract.md).
//!
//! # What it folds
//!
//! Walks the [`Program`] recursively and rewrites every expression
//! whose value is determined at compile time:
//!
//! ```text
//! 2 + 3              →  5                       (NumericLiteral + NumericLiteral)
//! "foo" + "bar"      →  "foobar"                (StringLiteral concat)
//! "x" + 1            →  "x1"                    (number coerces to string per ES)
//! 5 * 4              →  20
//! 10 / 2             →  5
//! 7 % 3              →  1
//! 2 ** 8             →  256
//! -3                 →  -3                      (UnaryExpression on NumericLiteral)
//! ~5                 →  -6                      (bitwise NOT, ES ToInt32)
//! !true              →  false
//! 1 < 2              →  true                    (numeric comparison)
//! "a" === "a"        →  true                    (strict equality, same type)
//! 1 == "1"           →  true                    (loose equality, see below)
//! false && expr      →  false                   (LogicalAnd short-circuit; left wins)
//! true  && expr      →  expr                    (right wins after fold)
//! true  || expr      →  true
//! false || expr      →  expr
//! null  ?? expr      →  expr
//! 0     ?? expr      →  0                       (zero is not nullish)
//! true  ? a : b      →  a                       (ConditionalExpression with literal test)
//! "hi".length        →  2                       (string-literal .length, UTF-16 units)
//! "ab".toUpperCase() →  "AB"                     (ASCII string casing methods)
//! "abc".charCodeAt(0)→  97                        (string-literal indexing methods)
//! "abc".charAt(1)    →  "b"
//! "a💩b".codePointAt(1)→ 128169                    (astral code POINT, not unit)
//! ```
//!
//! And recurses through every child node, so `1 + (2 * 3) → 1 + 6 → 7`
//! happens in a single bottom-up walk.
//!
//! # What it skips (intentionally)
//!
//! - `typeof` — fold-able in principle but `typeof undefined` requires
//!   the undefined literal which Phase 1 doesn't have. Tracked for
//!   Phase 1.x.
//! - `void` — produces ES `undefined`, same gap as above.
//! - `delete` — has observable side effects.
//! - Bitwise binary (`&`, `|`, `^`, `<<`, `>>`, `>>>`) — NOW FOLDED on two
//!   numeric literals (CLOC15.D) via ES `ToInt32`/`ToUint32` 32-bit
//!   semantics. See [`to_int32`] / [`to_uint32`]; `>>>` yields an unsigned
//!   result that can exceed `i32::MAX`. The unary bitwise NOT (`~`) is also
//!   folded on a numeric literal, reusing [`to_int32`] so it stays
//!   bit-for-bit consistent with the binary operators.
//! - Equality between non-matching literal types (`1 == "1"` is `true`
//!   but `1 === "1"` is `false`). The pass folds equality only when
//!   both literals are the *same* JS type; mixed-type comparisons are
//!   left alone (sound default).
//! - `AssignmentExpression` — has side effects (writes a binding).
//! - Anything containing an `Identifier`, `CallExpression`,
//!   `MemberExpression`, `ObjectExpression`, `ArrayExpression` —
//!   evaluation requires runtime knowledge; the pass recurses through
//!   their children but doesn't collapse them.
//!
//! # Iteration policy
//!
//! `IterationPolicy::FixedPoint` — `2 + 3 + 4` folds to `5 + 4` then to
//! `9` over two passes. The v1 pipeline scheduler runs FixedPoint
//! passes once (see `closure-pass-pipeline` v0.1.0); the single-pass
//! bottom-up walk in this implementation handles most chains
//! correctly in one go.
//!
//! # CV tracing
//!
//! Per the CLOC09 amendment, every node carries `cv: Option<CvId>`.
//! When a node has `Some(parent_cv)`, the folded replacement's `cv` is
//! derived via [`CVLog::derive`] so the chain
//! `original_expression → folded_literal → emitted_token` is
//! queryable in the CV graph. A `Contribution` is appended for each
//! fold with `source = "constant-fold"`, `tag = "folded"`, and `meta`
//! describing the rewrite.
//!
//! When the input node has `cv: None` (tracing disabled), the
//! replacement also gets `cv: None` and no contribution is emitted —
//! matching the per-program-not-per-pass policy from CLOC09.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, AssignmentOperator,
    AssignmentTarget, BinaryExpression,
    BinaryOperator, BlockStatement, BooleanLiteral, CallExpression, ConditionalExpression, NewExpression, SequenceExpression, SpreadElement, YieldExpression, AwaitExpression, ImportExpression,
    Declaration, Expression, ExpressionStatement, ForInStatement, ForInit, ForOfStatement,
    ForStatement,
    ArrowBody, ArrowFunctionExpression, TaggedTemplateExpression, TemplateLiteral,
    ClassDeclaration, ClassExpression, ClassMember, MethodDefinition, PropertyDefinition,
    AssignmentPattern, FunctionDeclaration, FunctionExpression, FunctionParam, Identifier,
    ChainExpression, IfStatement, LogicalExpression, LogicalOperator, MemberExpression, NullLiteral, NumericLiteral, OptionalCallExpression, OptionalMemberExpression,
    ObjectExpression, ObjectMember, Program, ProgramItem, Property, PropertyKey, PropertyKind, ReturnStatement, Statement,
    StringLiteral, UnaryExpression, UnaryOperator, UndefinedLiteral, UpdateExpression, VariableDeclaration,
    DoWhileStatement, VariableDeclarator, WhileStatement, WithStatement,
};
use serde_json::json;

/// Constant-folding pass — see crate-level docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConstantFoldPass;

impl ConstantFoldPass {
    /// Convenient zero-arg constructor — matches the
    /// `PassPipeline::add(Box::new(ConstantFoldPass::new()))`
    /// registration idiom.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constant-fold"
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Folds expose further folds — `2 + 3 + 4` becomes `5 + 4`
        // becomes `9` over two iterations. FixedPoint signals that
        // intent; v1 pipeline still only runs once but the bottom-up
        // walk handles most chains in a single pass.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Tree walk + small constant work per visit. ~2 pass-units.
        2
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // `fold_program` is a recursive bottom-up tree walk: `fold_binary`
        // folds `b.left`/`b.right`, each of which may be another binary node,
        // once per operator. A deeply left-nested operator chain — the shape
        // the bridge builds for flat source like `1+1+…+1` (thousands of
        // terms) — therefore recurses once per operator and, past a few
        // thousand levels, overflows the caller's ordinary ~2 MiB stack. That
        // is an *uncatchable* abort, and closurec runs this pass over
        // *untrusted* JS, so it must not be crashable by pathological input.
        //
        // Run the recursive fold on a large-stack worker (mirrors the emitter,
        // `coding-adventures-closure-emitter`). Output is *identical* to the
        // caller-thread fold — deep chains still collapse fully (`1+1+…` → one
        // number); only the stack size differs. `std::thread::scope` lets the
        // worker borrow `ctx.program`/`ctx.cv` without `'static`.
        let program = ctx.program;
        let cv = ctx.cv;
        let (new_program, contributions, changed, nodes_touched) = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(FOLD_STACK_SIZE)
                .spawn_scoped(scope, move || {
                    let mut state = FoldState {
                        cv,
                        contributions: Vec::new(),
                        changed: false,
                        nodes_touched: 0,
                    };
                    let new_program = fold_program(program, &mut state);
                    (
                        new_program,
                        state.contributions,
                        state.changed,
                        state.nodes_touched,
                    )
                })
                .expect("failed to spawn constant-fold worker thread")
                .join()
                .expect("constant-fold worker thread panicked")
        });

        Ok(PassOutput {
            program: new_program,
            contributions,
            changed,
            diagnostics: Vec::new(),
            stats: PassStats { nodes_touched },
        })
    }
}

/// Stack size for the constant-fold worker thread (see [`ConstantFoldPass::run`]).
///
/// 128 MiB comfortably absorbs the ~20 000-deep adversarial chains that
/// motivated this worker, with a healthy margin above them. The margin
/// matters: `fold_expression` is a wide `match` whose per-frame footprint
/// grows as new `Expression` variants are handled (e.g. the CLOC12.151
/// `ArrowFunctionExpression` arm), and per-frame cost also differs by target
/// — aarch64 (Apple-silicon CI) lays out larger frames than x86-64, so a
/// stack that merely *just* held 20 000 levels on one target overflowed on
/// another. Sizing to 128 MiB keeps a 2× cushion so a modest future frame
/// increase can't re-break the deep-chain DoS regression test. Costs nothing
/// for real code — pages fault in lazily. Matches the spirit of the emitter's
/// `EMIT_STACK_SIZE`.
const FOLD_STACK_SIZE: usize = 128 * 1024 * 1024;

// =====================================================================
// FoldState — mutable bookkeeping threaded through the recursive walk
// =====================================================================

/// Per-run state. Holds the CV log we contribute to and accumulators
/// for the `PassOutput` fields. Threaded by mutable reference through
/// every `fold_*` helper so the recursive walk doesn't have to
/// thread three independent values.
struct FoldState<'a> {
    cv: &'a mut CVLog,
    contributions: Vec<Contribution>,
    changed: bool,
    nodes_touched: u32,
}

impl FoldState<'_> {
    /// Mint a derived `cv` for a folded replacement node and record a
    /// `"folded"` contribution describing the rewrite. Returns the new
    /// CvId so the caller can stamp it onto the replacement node.
    ///
    /// When `parent` is `None` (tracing disabled on the input), no new
    /// CvId is allocated and no contribution is emitted — the
    /// replacement node will also carry `cv: None`.
    fn fork_cv(&mut self, parent: &Option<String>, before: &str, after: &str) -> Option<String> {
        match parent {
            Some(parent_cv) => {
                let new_cv = self.cv.derive(parent_cv, None);
                let contribution = Contribution {
                    source: "constant-fold".to_string(),
                    tag: "folded".to_string(),
                    meta: [
                        ("before".to_string(), json!(before)),
                        ("after".to_string(), json!(after)),
                        ("parent_cv".to_string(), json!(parent_cv)),
                        ("new_cv".to_string(), json!(new_cv)),
                    ]
                    .into_iter()
                    .collect(),
                };
                self.contributions.push(contribution);
                self.changed = true;
                Some(new_cv)
            }
            None => {
                // Untraced mode: still mark the program as changed so
                // the pipeline knows something happened, but no CV id
                // and no contribution.
                self.changed = true;
                None
            }
        }
    }

    fn visit(&mut self) {
        self.nodes_touched += 1;
    }
}

// =====================================================================
// Program / top-level
// =====================================================================

fn fold_program(prog: &Program, st: &mut FoldState) -> Program {
    st.visit();
    let new_body = prog
        .body
        .iter()
        .map(|item| fold_program_item(item, st))
        .collect();
    Program {
        cv: prog.cv.clone(),
        version: prog.version,
        source_type: prog.source_type,
        body: new_body,
    }
}

fn fold_program_item(item: &ProgramItem, st: &mut FoldState) -> ProgramItem {
    match item {
        ProgramItem::Statement(s) => ProgramItem::Statement(fold_statement(s, st)),
        ProgramItem::Declaration(d) => ProgramItem::Declaration(fold_declaration(d, st)),
    }
}

// =====================================================================
// Statements
// =====================================================================

fn fold_statement(stmt: &Statement, st: &mut FoldState) -> Statement {
    st.visit();
    match stmt {
        Statement::Tagged(t) => Statement::Tagged(fold_tagged_statement(t, st)),
        Statement::Declaration(d) => Statement::Declaration(fold_declaration(d, st)),
    }
}

fn fold_tagged_statement(stmt: &TaggedStatement, st: &mut FoldState) -> TaggedStatement {
    match stmt {
        TaggedStatement::ExpressionStatement(s) => {
            TaggedStatement::ExpressionStatement(ExpressionStatement {
                cv: s.cv.clone(),
                expression: fold_expression(&s.expression, st),
            })
        }
        TaggedStatement::BlockStatement(s) => TaggedStatement::BlockStatement(BlockStatement {
            cv: s.cv.clone(),
            body: s.body.iter().map(|x| fold_statement(x, st)).collect(),
        }),
        TaggedStatement::IfStatement(s) => TaggedStatement::IfStatement(IfStatement {
            cv: s.cv.clone(),
            test: fold_expression(&s.test, st),
            consequent: Box::new(fold_statement(&s.consequent, st)),
            alternate: s
                .alternate
                .as_ref()
                .map(|a| Box::new(fold_statement(a, st))),
        }),
        TaggedStatement::WhileStatement(s) => TaggedStatement::WhileStatement(WhileStatement {
            cv: s.cv.clone(),
            test: fold_expression(&s.test, st),
            body: Box::new(fold_statement(&s.body, st)),
        }),
        // `with (object) body` (CLOC12.187) — fold the object expression and the
        // body, exactly like a `while` head. (Not yet reachable; the bridge
        // still declines `with`.)
        TaggedStatement::WithStatement(s) => TaggedStatement::WithStatement(WithStatement {
            cv: s.cv.clone(),
            object: fold_expression(&s.object, st),
            body: Box::new(fold_statement(&s.body, st)),
        }),
        TaggedStatement::DoWhileStatement(s) => {
            TaggedStatement::DoWhileStatement(DoWhileStatement {
                cv: s.cv.clone(),
                body: Box::new(fold_statement(&s.body, st)),
                test: fold_expression(&s.test, st),
            })
        }
        TaggedStatement::ForStatement(s) => TaggedStatement::ForStatement(ForStatement {
            cv: s.cv.clone(),
            init: s.init.as_ref().map(|i| match i {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
            }),
            test: s.test.as_ref().map(|e| fold_expression(e, st)),
            update: s.update.as_ref().map(|e| fold_expression(e, st)),
            body: Box::new(fold_statement(&s.body, st)),
        }),
        TaggedStatement::ForInStatement(s) => TaggedStatement::ForInStatement(ForInStatement {
            cv: s.cv.clone(),
            left: match &s.left {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
            },
            right: fold_expression(&s.right, st),
            body: Box::new(fold_statement(&s.body, st)),
        }),
        TaggedStatement::ForOfStatement(s) => TaggedStatement::ForOfStatement(ForOfStatement {
            cv: s.cv.clone(),
            left: match &s.left {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
            },
            right: fold_expression(&s.right, st),
            body: Box::new(fold_statement(&s.body, st)),
        }),
        TaggedStatement::ReturnStatement(s) => TaggedStatement::ReturnStatement(ReturnStatement {
            cv: s.cv.clone(),
            argument: s.argument.as_ref().map(|e| fold_expression(e, st)),
        }),
        TaggedStatement::LabeledStatement(s) => {
            // The label is just a name; folding only touches the body.
            // We don't recursively label-rename, so the label survives
            // verbatim. The body recurses through fold_statement so
            // constant folds reach inside `a: { foo(2+3); }`.
            TaggedStatement::LabeledStatement(coding_adventures_javascript_ast::LabeledStatement {
                cv: s.cv.clone(),
                label: s.label.clone(),
                body: Box::new(fold_statement(&s.body, st)),
            })
        }
        TaggedStatement::ThrowStatement(s) => {
            // Fold the thrown expression — `throw 2+3;` becomes
            // `throw 5;` exactly like every other expression-bearing
            // statement.
            TaggedStatement::ThrowStatement(coding_adventures_javascript_ast::ThrowStatement {
                cv: s.cv.clone(),
                argument: fold_expression(&s.argument, st),
            })
        }
        TaggedStatement::SwitchStatement(s) => {
            // Fold the discriminant and each case's test + consequent.
            // No structural peephole here — constant-fold doesn't
            // rewrite control flow; the matching-case-only collapse
            // belongs in fold-control-flow as a follow-up.
            TaggedStatement::SwitchStatement(coding_adventures_javascript_ast::SwitchStatement {
                cv: s.cv.clone(),
                discriminant: fold_expression(&s.discriminant, st),
                cases: s
                    .cases
                    .iter()
                    .map(|c| coding_adventures_javascript_ast::SwitchCase {
                        cv: c.cv.clone(),
                        test: c.test.as_ref().map(|e| fold_expression(e, st)),
                        consequent: c.consequent.iter().map(|s| fold_statement(s, st)).collect(),
                    })
                    .collect(),
            })
        }
        TaggedStatement::TryStatement(s) => {
            // Fold within the protected block, the catch body, and the
            // finalizer — each is an ordinary block. The catch `param` is a
            // binding name, not an expression, so it is preserved verbatim.
            let block = BlockStatement {
                cv: s.block.cv.clone(),
                body: s.block.body.iter().map(|x| fold_statement(x, st)).collect(),
            };
            let handler = s.handler.as_ref().map(|h| {
                coding_adventures_javascript_ast::CatchClause {
                    cv: h.cv.clone(),
                    param: h.param.clone(),
                    body: BlockStatement {
                        cv: h.body.cv.clone(),
                        body: h.body.body.iter().map(|x| fold_statement(x, st)).collect(),
                    },
                }
            });
            let finalizer = s.finalizer.as_ref().map(|f| BlockStatement {
                cv: f.cv.clone(),
                body: f.body.iter().map(|x| fold_statement(x, st)).collect(),
            });
            TaggedStatement::TryStatement(coding_adventures_javascript_ast::TryStatement {
                cv: s.cv.clone(),
                block,
                handler,
                finalizer,
            })
        }
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_)
        | TaggedStatement::DebuggerStatement(_) => stmt.clone(),
    }
}

// =====================================================================
// Declarations
// =====================================================================

/// Fold each parameter's default-value expression. A plain identifier and a
/// rest element carry no sub-expression, so they clone verbatim; a default
/// parameter (`a = 1 + 2`) holds live code — its `right` is folded (→ `a = 3`)
/// through the same [`fold_expression`] path a function body uses. This is what
/// makes `function f(a = 1 + 2){}` shrink to `function f(a = 3){}` instead of
/// carrying the arithmetic to the output.
fn fold_params(params: &[FunctionParam], st: &mut FoldState) -> Vec<FunctionParam> {
    params
        .iter()
        .map(|p| match p {
            FunctionParam::AssignmentPattern(ap) => {
                FunctionParam::AssignmentPattern(AssignmentPattern {
                    cv: ap.cv.clone(),
                    left: ap.left.clone(),
                    right: fold_expression(&ap.right, st),
                })
            }
            // Plain identifier / rest element: no default expression to fold.
            other => other.clone(),
        })
        .collect()
}

fn fold_declaration(decl: &Declaration, st: &mut FoldState) -> Declaration {
    st.visit();
    match decl {
        Declaration::VariableDeclaration(v) => {
            Declaration::VariableDeclaration(fold_variable_declaration(v, st))
        }
        Declaration::FunctionDeclaration(f) => {
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: fold_params(&f.params, st),
                body: BlockStatement {
                    cv: f.body.cv.clone(),
                    body: f.body.body.iter().map(|s| fold_statement(s, st)).collect(),
                },
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        // A class *declaration* (`class C { … }`) folds identically to a class
        // *expression* inside its heritage + member bodies — only the outer node
        // type and the required `id` differ. Both route through the shared
        // `fold_class_body` helper.
        Declaration::ClassDeclaration(c) => fold_class_declaration(c, st),
        // An import declaration has no foldable body — its only payload is the
        // bound names and the module specifier string. Preserve it verbatim.
        Declaration::ImportDeclaration(i) => Declaration::ImportDeclaration(i.clone()),
        // Export declarations (CLOC12.189). PR1 keeps the node unreachable (no
        // bridge yet); preserve each verbatim. The inner declaration of an
        // `export const x = 1` is NOT folded here — descent lands with the
        // bridge PR that makes the node reachable.
        Declaration::ExportNamedDeclaration(e) => Declaration::ExportNamedDeclaration(e.clone()),
        Declaration::ExportDefaultDeclaration(e) => {
            Declaration::ExportDefaultDeclaration(e.clone())
        }
        Declaration::ExportAllDeclaration(e) => Declaration::ExportAllDeclaration(e.clone()),
    }
}

/// Fold a class *declaration* (`class C [extends S] { … }`): the `extends`
/// operand and each method body, via the shared [`fold_class_body`] helper.
/// Mirrors [`fold_class`] (the expression form); `#[inline(never)]` keeps its
/// locals off the caller's frame (DoS lesson).
#[inline(never)]
fn fold_class_declaration(c: &ClassDeclaration, st: &mut FoldState) -> Declaration {
    let (super_class, body) = fold_class_body(&c.super_class, &c.body, st);
    Declaration::ClassDeclaration(ClassDeclaration {
        cv: c.cv.clone(),
        id: c.id.clone(),
        super_class,
        body,
    })
}

fn fold_variable_declaration(v: &VariableDeclaration, st: &mut FoldState) -> VariableDeclaration {
    VariableDeclaration {
        cv: v.cv.clone(),
        kind: v.kind,
        declarations: v
            .declarations
            .iter()
            .map(|d| VariableDeclarator {
                cv: d.cv.clone(),
                id: d.id.clone(),
                init: d.init.as_ref().map(|e| fold_expression(e, st)),
            })
            .collect(),
    }
}

// =====================================================================
// Expressions — the actual folding
// =====================================================================

/// Fold the shared `[extends S] { members }` tail of a class — the heritage
/// operand (an expression) and each method's body (statements). Reused by both
/// the class *expression* ([`fold_class`]) and the class *declaration*
/// ([`fold_class_declaration`]), since the two node forms share their body
/// shape and differ only in the outer node type + whether `id` is optional.
/// Kept `#[inline(never)]` so its locals do not inflate the caller's frame —
/// see the DoS lesson.
#[inline(never)]
fn fold_class_body(
    super_class: &Option<Box<Expression>>,
    body: &[ClassMember],
    st: &mut FoldState,
) -> (Option<Box<Expression>>, Vec<ClassMember>) {
    let super_class = super_class
        .as_ref()
        .map(|s| Box::new(fold_expression(s, st)));
    let body = body
        .iter()
        .map(|m| match m {
            ClassMember::Method(md) => ClassMember::Method(MethodDefinition {
                cv: md.cv.clone(),
                key: md.key.clone(),
                kind: md.kind,
                value: FunctionExpression {
                    cv: md.value.cv.clone(),
                    id: md.value.id.clone(),
                    params: fold_params(&md.value.params, st),
                    body: BlockStatement {
                        cv: md.value.body.cv.clone(),
                        body: md
                            .value
                            .body
                            .body
                            .iter()
                            .map(|s| fold_statement(s, st))
                            .collect(),
                    },
                    generator: md.value.generator,
                    is_async: md.value.is_async,
                },
                computed: md.computed,
                is_static: md.is_static,
            }),
            // A class field folds inside its initializer (a plain expression
            // that runs at construction) — `x = 1 + 2` → `x = 3`. The key is
            // cloned like a method key (not folded); the value is optional.
            ClassMember::Field(fd) => ClassMember::Field(PropertyDefinition {
                cv: fd.cv.clone(),
                key: fd.key.clone(),
                value: fd.value.as_ref().map(|v| fold_expression(v, st)),
                computed: fd.computed,
                is_static: fd.is_static,
            }),
            // A static-init block folds inside each of its statements (they run
            // at class-definition time) — mirroring the method-body fold. It has
            // no key and no binding name; only the statement list is rebuilt.
            ClassMember::StaticBlock(b) => ClassMember::StaticBlock(BlockStatement {
                cv: b.cv.clone(),
                body: b.body.iter().map(|s| fold_statement(s, st)).collect(),
            }),
        })
        .collect();
    (super_class, body)
}

/// Fold inside a class expression: delegates to [`fold_class_body`] for the
/// heritage + method bodies. Kept `#[inline(never)]` so its locals do not
/// inflate `fold_expression`'s frame — see the call site.
#[inline(never)]
fn fold_class(c: &ClassExpression, st: &mut FoldState) -> Expression {
    let (super_class, body) = fold_class_body(&c.super_class, &c.body, st);
    Expression::ClassExpression(ClassExpression {
        cv: c.cv.clone(),
        id: c.id.clone(),
        super_class,
        body,
    })
}

/// Fold a `new` on a **standard built-in constructor** to its shorter
/// equivalent, matching the reference Closure Compiler's
/// `tryFoldStandardConstructors`. Returns `Ok(replacement)` when the fold fires,
/// or `Err(arguments)` (handing the already-folded argument list back) so the
/// caller can rebuild the `new` unchanged.
///
/// Gated to a **bare identifier** callee whose name is exactly `Object` or
/// `Array` (a member callee like `obj.Array`, or any other name, is left alone).
/// Like Closure at SIMPLE, this assumes those globals are not shadowed. Both are
/// spec-safe: calling `Object`/`Array` as an ordinary function constructs the
/// same value as `new` for every argument list.
///
/// ```text
///   new Object(x)        →  Object(x)      (1+ args: drop `new`, keep the call)
///   new Object()         →  {}             (no args: an empty object literal)
///   new Array(x)         →  Array(x)       (exactly ONE arg is a *length* or a
///                                           sole element — ambiguous, so the
///                                           call form is kept, NOT `[x]`)
///   new Array()          →  []             (no args: an empty array literal)
///   new Array(a, b, …)   →  [a, b, …]      (2+ args: an array literal; spread
///                                           args carry across as elements)
/// ```
///
/// **`Error` and `RegExp` are intentionally NOT handled here.** `Error` is folded
/// by its own arm (`new Error(…)` → `Error(…)`); `RegExp` is declined because
/// `RegExp(r)` aliases an existing regex argument instead of copying it, so
/// `new RegExp(x)` → `RegExp(x)` could be an observable change — a potential
/// miscompile the reference compiler tolerates but we do not.
fn fold_standard_constructor(
    callee: &Expression,
    arguments: Vec<Expression>,
    cv: &Option<String>,
    st: &mut FoldState,
) -> Result<Expression, Vec<Expression>> {
    let name = match callee {
        Expression::Identifier(id) => id.name.as_str(),
        _ => return Err(arguments),
    };
    match name {
        // `Object` — no args → `{}`; otherwise drop `new` to a plain call.
        "Object" if arguments.is_empty() => {
            let new_cv = st.fork_cv(cv, "new Object()", "{}");
            Ok(Expression::ObjectExpression(ObjectExpression {
                cv: new_cv,
                properties: vec![],
            }))
        }
        "Object" => {
            let new_cv = st.fork_cv(cv, "new Object(…)", "Object(…)");
            Ok(Expression::CallExpression(CallExpression {
                cv: new_cv,
                callee: Box::new(callee.clone()),
                arguments,
            }))
        }
        // `Array` — 0 args → `[]`; exactly 1 arg keeps the call (a lone argument
        // is a *length*, so `Array(3)` ≠ `[3]`); 2+ args → an array literal.
        "Array" if arguments.is_empty() => {
            let new_cv = st.fork_cv(cv, "new Array()", "[]");
            Ok(Expression::ArrayExpression(ArrayExpression {
                cv: new_cv,
                elements: vec![],
            }))
        }
        "Array" if arguments.len() == 1 => {
            let new_cv = st.fork_cv(cv, "new Array(x)", "Array(x)");
            Ok(Expression::CallExpression(CallExpression {
                cv: new_cv,
                callee: Box::new(callee.clone()),
                arguments,
            }))
        }
        // 2+ args WITH a spread: the array-literal form would be a MISCOMPILE.
        // A spread of unknown runtime length can collapse the construction to a
        // *single* runtime argument — the length form — so `new Array(5, ...[])`
        // is `new Array(5)` (a length-5 array), whereas `[5, ...[]]` is `[5]`
        // (length 1). The reference compiler folds it to `[a, ...xs]` anyway
        // (unsound); we decline to the always-equivalent call form
        // `Array(a, ...xs)` (`Array(args)` ≡ `new Array(args)` for every list).
        // Matching Closure's array-literal spelling here is a byte-identity
        // follow-up.
        "Array"
            if arguments
                .iter()
                .any(|a| matches!(a, Expression::SpreadElement(_))) =>
        {
            let new_cv = st.fork_cv(cv, "new Array(…, ...spread)", "Array(…, ...spread)");
            Ok(Expression::CallExpression(CallExpression {
                cv: new_cv,
                callee: Box::new(callee.clone()),
                arguments,
            }))
        }
        // 2+ plain (non-spread) args: the static count IS the element count, so
        // an array literal is exactly equivalent.
        "Array" => {
            let new_cv = st.fork_cv(cv, "new Array(a,b,…)", "[a,b,…]");
            let elements = arguments.into_iter().map(Some).collect();
            Ok(Expression::ArrayExpression(ArrayExpression {
                cv: new_cv,
                elements,
            }))
        }
        _ => Err(arguments),
    }
}

fn fold_expression(expr: &Expression, st: &mut FoldState) -> Expression {
    st.visit();
    match expr {
        // Leaves: no children to recurse into, nothing to fold.
        // BigIntLiteral joins this list — bigint arithmetic folding
        // is a future enhancement (would need bigint runtime support);
        // for now the literal is itself the folded form.
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        // A regex literal (`/ab+c/gi`) is a leaf like a string: nothing inside
        // to fold, and it is never a foldable constant, so it passes through
        // unchanged exactly like StringLiteral.
        | Expression::RegExpLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // `this` is a leaf keyword — nothing inside to fold, and it is never
        // itself a constant, so it passes through unchanged like the literals.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => expr.clone(),

        Expression::BinaryExpression(b) => fold_binary(b, st),
        Expression::LogicalExpression(l) => fold_logical(l, st),
        Expression::UnaryExpression(u) => fold_unary(u, st),
        // `++x` / `x++`: a read-modify-write with a side effect — never a
        // constant, so we do NOT collapse it (dropping it would drop the
        // mutation). Recurse into the argument only for the member-object case
        // (`a[i].b++` etc.); the update node itself is preserved verbatim.
        Expression::UpdateExpression(u) => Expression::UpdateExpression(UpdateExpression {
            cv: u.cv.clone(),
            operator: u.operator,
            prefix: u.prefix,
            argument: Box::new(fold_expression(&u.argument, st)),
        }),
        Expression::ConditionalExpression(c) => fold_conditional(c, st),

        // `x = …` — recurse into the right-hand side, and additionally contract
        // the compound self-assignment shape `x = x OP E` → `x OP= E`. The body
        // lives out-of-line (see `fold_assignment`) so its locals do not enlarge
        // this hot recursion frame — same DoS-guard discipline as the optional
        // and member arms above.
        Expression::AssignmentExpression(a) => fold_assignment(a, st),
        Expression::CallExpression(c) => fold_call(c, st),
        // `new X(args)` constructs an object — a side-effecting operation, never
        // a constant. Recurse into the callee and each argument (they may hold
        // foldable subexpressions). A `new` on a *standard built-in constructor*
        // then folds to its shorter equivalent: `Error` below, and
        // `Object`/`Array` via [`fold_standard_constructor`]. Everything else
        // preserves the `new`.
        //
        // **Standard-constructor `new`-drop (`new Error(…)` → `Error(…)`).**
        // Closure rewrites `new Error(args)` to a plain call `Error(args)`,
        // saving four bytes. Calling the built-in `Error` as an ordinary
        // function constructs an Error object *identically* to `new`
        // (ECMAScript §20.5.1.1: the `Error` constructor's [[Call]] and
        // [[Construct]] paths converge — `Error(m)` and `new Error(m)` both
        // yield a fresh Error with the same `.message`), so the drop is
        // semantics-preserving for **every** argument list, including the
        // no-arg form (`new Error` → `Error()`).
        //
        // Scope:
        //   * `Object`/`Array` fold via [`fold_standard_constructor`], which
        //     also collapses their no-arg forms to `{}` / `[]` literals (and
        //     keeps `new Array(len)`'s single-arg *length* form as a call).
        //   * `RegExp` is NOT folded: `RegExp(r)` returns its argument unchanged
        //     when `r` is already a regex, whereas `new RegExp(r)` always makes
        //     a fresh copy — so `new RegExp(x)` → `RegExp(x)` would be an
        //     observable change (a potential miscompile). Closure does it
        //     anyway; we decline to stay sound. (Follow-up.)
        //   * The `Error` *subtypes* (`TypeError`, `RangeError`, …) are left
        //     alone because the reference compiler does not fold them.
        //
        // Gated to a **bare `Error` identifier** callee: a member callee
        // (`obj.Error`) or any other name is untouched. Like Closure at SIMPLE,
        // this assumes the global `Error` binding is not shadowed.
        Expression::NewExpression(n) => {
            let callee = fold_expression(&n.callee, st);
            let arguments: Vec<Expression> =
                n.arguments.iter().map(|a| fold_expression(a, st)).collect();
            // `new Error(...)` → `Error(...)` (bare-ident gate; see comment above).
            if matches!(&callee, Expression::Identifier(id) if id.name == "Error") {
                let new_cv = st.fork_cv(&n.cv, "new Error(…)", "Error(…)");
                Expression::CallExpression(CallExpression {
                    cv: new_cv,
                    callee: Box::new(callee),
                    arguments,
                })
            } else {
                // `new Object(...)` / `new Array(...)` fold via the helper; any
                // other callee preserves the `new`.
                match fold_standard_constructor(&callee, arguments, &n.cv, st) {
                    Ok(folded) => folded,
                    Err(arguments) => Expression::NewExpression(NewExpression {
                        cv: n.cv.clone(),
                        callee: Box::new(callee),
                        arguments,
                    }),
                }
            }
        }
        // `a, b, c` — fold each operand independently. We do NOT drop the
        // earlier operands even if they fold to a constant: they may carry side
        // effects, and dropping them would change behaviour (that is a separate
        // useless-code pass's job, gated on purity).
        Expression::SequenceExpression(s) => Expression::SequenceExpression(SequenceExpression {
            cv: s.cv.clone(),
            expressions: s.expressions.iter().map(|e| fold_expression(e, st)).collect(),
        }),
        // `` tag`a${x}b` `` — fold within the tag callee and each `${…}`
        // substitution; the raw quasi strings are opaque text, left untouched.
        Expression::TaggedTemplateExpression(t) => {
            Expression::TaggedTemplateExpression(TaggedTemplateExpression {
                cv: t.cv.clone(),
                tag: Box::new(fold_expression(&t.tag, st)),
                quasi: TemplateLiteral {
                    cv: t.quasi.cv.clone(),
                    quasis: t.quasi.quasis.clone(),
                    expressions: t
                        .quasi
                        .expressions
                        .iter()
                        .map(|e| fold_expression(e, st))
                        .collect(),
                },
            })
        }
        // `...arg` — fold within the spread argument (it may hold a foldable
        // subexpression); the spread itself is structural, never a constant, and
        // is not dropped (its iterable may carry side effects).
        Expression::SpreadElement(s) => Expression::SpreadElement(SpreadElement {
            cv: s.cv.clone(),
            argument: Box::new(fold_expression(&s.argument, st)),
        }),
        Expression::YieldExpression(y) => Expression::YieldExpression(YieldExpression {
            cv: y.cv.clone(),
            delegate: y.delegate,
            argument: y.argument.as_ref().map(|a| Box::new(fold_expression(a, st))),
        }),
        Expression::AwaitExpression(a) => Expression::AwaitExpression(AwaitExpression {
            cv: a.cv.clone(),
            argument: Box::new(fold_expression(&a.argument, st)),
        }),
        Expression::ImportExpression(e) => Expression::ImportExpression(ImportExpression {
            cv: e.cv.clone(),
            source: Box::new(fold_expression(&e.source, st)),
        }),
        Expression::MemberExpression(m) => fold_member(m, st),
        // Optional-chain nodes delegate to out-of-line helpers. This is not
        // cosmetic: `fold_expression` recurses once per AST level, and a debug
        // build gives every match arm its own stack slots, so building these
        // structs *inline* would enlarge the per-level frame on the hot binary
        // recursion path — enough to overflow the deep-chain DoS-guard test.
        // Delegating (as the member/call arms already do) keeps their locals in
        // the helper frame, entered only when an optional node is actually hit.
        Expression::OptionalMemberExpression(m) => fold_optional_member(m, st),
        Expression::OptionalCallExpression(c) => fold_optional_call(c, st),
        Expression::ChainExpression(c) => fold_chain(c, st),
        Expression::ArrayExpression(a) => Expression::ArrayExpression(ArrayExpression {
            cv: a.cv.clone(),
            elements: a
                .elements
                .iter()
                .map(|e| e.as_ref().map(|x| fold_expression(x, st)))
                .collect(),
        }),
        Expression::ObjectExpression(o) => Expression::ObjectExpression(ObjectExpression {
            cv: o.cv.clone(),
            properties: o
                .properties
                .iter()
                .map(|member| match member {
                    ObjectMember::Property(p) => ObjectMember::Property(Property {
                        cv: p.cv.clone(),
                        kind: p.kind,
                        key: match &p.key {
                            // Identifier / literal keys: pass through.
                            PropertyKey::Identifier(i) => PropertyKey::Identifier(i.clone()),
                            // A private name (`#x`) is only legal as a class-member
                            // key, never in an object literal — but the match must
                            // stay exhaustive, so pass it through unchanged.
                            PropertyKey::PrivateName(p) => PropertyKey::PrivateName(p.clone()),
                            PropertyKey::StringLiteral(s) => PropertyKey::StringLiteral(s.clone()),
                            PropertyKey::NumericLiteral(n) => {
                                // A NON-INTEGER numeric object key must be QUOTED:
                                // its property name is the ECMAScript `ToString` of
                                // the value, so Closure prints `{0.5:1}` as
                                // `{"0.5":1}` (a string key), never the bare number.
                                // (An integer key stays numeric — the emitter prints
                                // `{1:1}` unquoted.) We convert only finite
                                // non-integers in JS's plain-decimal `ToString` range
                                // `[1e-6, 1e21)`, where `format_js_number` (shortest
                                // round-trip) equals JS `ToString` exactly. Tiny
                                // (`<1e-6`) or huge non-integers take exponent forms
                                // (`1e-7` → `"1e-7"`), and large integers (`>2^53`)
                                // take precision-quoting forms — both a separate
                                // follow-up, so they stay numeric here (valid output,
                                // just not yet byte-identical).
                                let v = n.value;
                                if v.is_finite()
                                    && v.fract() != 0.0
                                    && v.abs() >= 1e-6
                                    && v.abs() < 1e21
                                {
                                    let name = format_js_number(v);
                                    let new_cv =
                                        st.fork_cv(&p.cv, &format!("{{{name}:…}}"), &format!("{{\"{name}\":…}}"));
                                    let mut key = property_key_for(&name);
                                    if let PropertyKey::StringLiteral(s) = &mut key {
                                        s.cv = new_cv;
                                    }
                                    key
                                } else {
                                    PropertyKey::NumericLiteral(n.clone())
                                }
                            }
                            PropertyKey::Expression(e) => {
                                PropertyKey::Expression(Box::new(fold_expression(e, st)))
                            }
                        },
                        value: Box::new(fold_expression(&p.value, st)),
                        computed: p.computed,
                        shorthand: p.shorthand,
                        method: p.method,
                    }),
                    // Object spread `...expr` — fold through the spread argument
                    // (`{...(1+2 && o)}` is not foldable here, but a foldable
                    // sub-expression inside still simplifies).
                    ObjectMember::Spread(s) => ObjectMember::Spread(SpreadElement {
                        cv: s.cv.clone(),
                        argument: Box::new(fold_expression(&s.argument, st)),
                    }),
                })
                .collect(),
        }),

        // A function *value*: fold constants inside its body exactly as
        // we do for a `FunctionDeclaration` (see `fold_declaration`).
        // The function itself is not a foldable constant; we descend so
        // that `var f = function () { return 1 + 1; }` folds to
        // `... return 2; ...`. `id`/`params` are structural — passed
        // through untouched.
        Expression::FunctionExpression(f) => {
            Expression::FunctionExpression(FunctionExpression {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: fold_params(&f.params, st),
                body: BlockStatement {
                    cv: f.body.cv.clone(),
                    body: f.body.body.iter().map(|s| fold_statement(s, st)).collect(),
                },
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        // A class expression: fold inside the `extends` operand and each
        // method body, exactly as a function expression folds inside its body.
        // The class itself is never a foldable constant. Delegated to an
        // `#[inline(never)]` helper so this arm does not enlarge
        // `fold_expression`'s debug-build stack frame — the whole `match` is on
        // the hot recursive dispatch path, and a fat frame there is a
        // deep-nesting stack-overflow (DoS) hazard (see lessons.md).
        Expression::ClassExpression(c) => fold_class(c, st),
        // An arrow function value: fold inside its body just like a
        // function expression. A block body folds statement-by-statement;
        // a concise (expression) body folds the single expression — so
        // `x => 1 + 1` folds to `x => 2`. `params` are structural.
        Expression::ArrowFunctionExpression(a) => {
            Expression::ArrowFunctionExpression(ArrowFunctionExpression {
                cv: a.cv.clone(),
                params: fold_params(&a.params, st),
                body: match &a.body {
                    ArrowBody::Block(b) => ArrowBody::Block(BlockStatement {
                        cv: b.cv.clone(),
                        body: b.body.iter().map(|s| fold_statement(s, st)).collect(),
                    }),
                    ArrowBody::Expression(e) => {
                        ArrowBody::Expression(Box::new(fold_expression(e, st)))
                    }
                },
                is_async: a.is_async,
            })
        }
        // A template literal: fold each embedded `${…}` expression. The
        // `quasis` are fixed string segments with no sub-expressions to
        // fold. (Folding the *whole* template to a string literal when all
        // parts are constant is a future optimisation; here we only recurse.)
        Expression::TemplateLiteral(t) => fold_template_literal(t, st),
    }
}

/// Contract a compound *self*-assignment: `x = x OP E` → `x OP= E`.
///
/// ## What this fold does
///
/// ```text
///   x = x + 1     ──▶   x += 1
///   n = n - 2     ──▶   n -= 2
///   k = k * 2     ──▶   k *= 2
///   a = a + b     ──▶   a += b
///   s = s + "b"   ──▶   s += "b"
/// ```
///
/// The reference Closure Compiler performs exactly this contraction at
/// `SIMPLE`, and it is a pure size win: `x=x+1` (5 chars) becomes `x+=1`
/// (4 chars) with identical run-time behaviour.
///
/// ## The exact rule (and why the shape matters)
///
/// We rewrite only when ALL of the following hold:
///   1. the assignment operator is plain `=` (not already a compound form),
///   2. the target is a **bare identifier** `x` (`AssignmentTarget::Identifier`),
///   3. the right-hand side is a `BinaryExpression` whose **LEFT** operand is
///      that same identifier `x` (compared by *name*), and
///   4. the binary operator has a compound counterpart — the arithmetic and
///      bitwise operators do (`+ - * / % ** << >> >>> & | ^`); the relational,
///      equality, `in`, and `instanceof` operators do **not**.
///
/// The match is on the **left** operand only. `x = x + 1` folds, but
/// `x = 1 + x` does **not**:
///
/// ```text
///   x = x - 1   ≡   x -= 1        (target on the left — contractable)
///   x = 1 - x   ≡   x -= ??       (NO — `1 - x` ≠ `x - 1` for non-commutative
///                                  operators; there is no "reverse -=" form)
/// ```
///
/// Even for a commutative operator (`+`, `*`) the reference compiler only
/// contracts the left-operand shape, so mirroring that keeps us byte-identical.
///
/// ## Why identifier targets are always sound (and members are deferred)
///
/// For a bare identifier binding, evaluating the *reference* `x` has no
/// user-observable side effect — so `x = x OP E` reads `x` once, computes, and
/// writes `x` once, exactly as `x OP= E` does. The two forms are
/// interchangeable.
///
/// This is **not** true for a member target like `o[f()]`: the expanded form
/// `o[f()] = o[f()] OP E` evaluates the reference sub-expressions (here `f()`)
/// *twice*, whereas `o[f()] OP= E` evaluates them *once*. Contracting a member
/// target is therefore only sound once the object/property sub-expressions are
/// proven side-effect-free — that is a deliberate follow-up (CLOC12.198b), and
/// PR1 restricts itself to identifier targets.
///
/// Kept `#[inline(never)]` and out of the big `fold_expression` match so its
/// locals do not inflate the shared recursive frame (the same DoS-guard
/// discipline the member/optional/template arms follow — see lessons.md).
#[inline(never)]
/// Normalise an assignment **target** — the left-hand side of `=`. For a member
/// target we fold the object (value position) and dot-normalise a computed
/// string key exactly like [`fold_member`] does for a value-position member, so
/// `o["foo"] = 1` → `o.foo = 1` and `a["b"]["c"] = 1` → `a.b.c = 1`. We do NOT
/// route the target through `fold_member` itself: that would run the array-index
/// / `.length` folds, which must not fire on an lvalue (`[1,2,3]["0"] = x` must
/// stay an element write, not fold to the literal `1`). A bare-identifier target
/// is returned unchanged.
fn fold_assignment_target(t: &AssignmentTarget, st: &mut FoldState) -> AssignmentTarget {
    let AssignmentTarget::MemberExpression(m) = t else {
        return t.clone();
    };
    let object = fold_expression(&m.object, st);
    if m.computed {
        if let Expression::StringLiteral(s) = m.property.as_ref() {
            if is_identifier_name(&s.value) && !is_es3_reserved_word(&s.value) {
                let before = format!("member[\"{}\"] (assign target)", s.value);
                let after = format!("member.{} (assign target)", s.value);
                let new_cv = st.fork_cv(&m.cv, &before, &after);
                return AssignmentTarget::MemberExpression(Box::new(MemberExpression {
                    cv: new_cv,
                    object: Box::new(object),
                    property: Box::new(Expression::Identifier(Identifier {
                        cv: s.cv.clone(),
                        name: s.value.clone(),
                    })),
                    computed: false,
                }));
            }
        }
    }
    AssignmentTarget::MemberExpression(Box::new(MemberExpression {
        cv: m.cv.clone(),
        object: Box::new(object),
        property: m.property.clone(),
        computed: m.computed,
    }))
}

fn fold_assignment(a: &AssignmentExpression, st: &mut FoldState) -> Expression {
    // Fold the right-hand side first, consistent with every other recursive
    // arm; the contraction test below then runs on the folded RHS.
    let right = fold_expression(&a.right, st);

    // The contraction only applies to a plain `=` whose target is an identifier
    // and whose RHS is `<that identifier> OP E` for a compound-capable OP.
    if a.operator == AssignmentOperator::Eq {
        if let AssignmentTarget::Identifier(target) = &a.left {
            if let Expression::BinaryExpression(b) = &right {
                if let Expression::Identifier(bin_left) = b.left.as_ref() {
                    if bin_left.name == target.name {
                        if let Some((compound, op_symbol)) =
                            compound_assignment_operator(b.operator)
                        {
                            let parent = a.cv.clone();
                            let before =
                                format!("{0} = {0} {1} E", target.name, op_symbol);
                            let after = format!("{0} {1}= E", target.name, op_symbol);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return Expression::AssignmentExpression(AssignmentExpression {
                                cv: new_cv,
                                operator: compound,
                                // Reuse the original target node verbatim.
                                left: a.left.clone(),
                                // The compound form keeps only the binary's RIGHT
                                // operand; the left operand is now implied by the
                                // target.
                                right: b.right.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    // No contraction: preserve the assignment, carrying the folded RHS and the
    // (dot-normalised) target.
    Expression::AssignmentExpression(AssignmentExpression {
        cv: a.cv.clone(),
        operator: a.operator,
        left: fold_assignment_target(&a.left, st),
        right: Box::new(right),
    })
}

/// Map a [`BinaryOperator`] to its compound-[`AssignmentOperator`] counterpart
/// and the operator's source symbol, or `None` when the operator has no
/// compound assignment form.
///
/// | binary | compound | | binary | compound |
/// |--------|----------|-|--------|----------|
/// | `+`    | `+=`     | | `<<`   | `<<=`    |
/// | `-`    | `-=`     | | `>>`   | `>>=`    |
/// | `*`    | `*=`     | | `>>>`  | `>>>=`   |
/// | `/`    | `/=`     | | `&`    | `&=`     |
/// | `%`    | `%=`     | | `|`    | `|=`     |
/// | `**`   | `**=`    | | `^`    | `^=`     |
///
/// The relational (`< <= > >=`), equality (`== != === !==`), `in`, and
/// `instanceof` operators have no `OP=` form in JavaScript, so they return
/// `None` and the caller declines the contraction.
fn compound_assignment_operator(op: BinaryOperator) -> Option<(AssignmentOperator, &'static str)> {
    use AssignmentOperator as A;
    use BinaryOperator as B;
    Some(match op {
        B::Add => (A::AddEq, "+"),
        B::Sub => (A::SubEq, "-"),
        B::Mul => (A::MulEq, "*"),
        B::Div => (A::DivEq, "/"),
        B::Mod => (A::ModEq, "%"),
        B::Exp => (A::ExpEq, "**"),
        B::LeftShift => (A::LeftShiftEq, "<<"),
        B::RightShift => (A::RightShiftEq, ">>"),
        B::UnsignedRightShift => (A::UnsignedRightShiftEq, ">>>"),
        B::BitAnd => (A::BitAndEq, "&"),
        B::BitOr => (A::BitOrEq, "|"),
        B::BitXor => (A::BitXorEq, "^"),
        // No compound assignment form exists for these.
        B::Eq
        | B::NotEq
        | B::StrictEq
        | B::StrictNotEq
        | B::Lt
        | B::LtEq
        | B::Gt
        | B::GtEq
        | B::In
        | B::InstanceOf => return None,
    })
}

/// Fold a template literal — recurse into its substitutions, then collapse the
/// whole node to a string literal when every substitution is a stringifiable
/// constant (CLOC12.197). Kept `#[inline(never)]` and out of the big
/// `fold_expression` match so its several `Vec`/`String` locals do NOT inflate
/// the shared recursive frame — that frame is walked once per AST depth level,
/// so bloating it lowers the input-nesting depth the pass survives (the
/// `deeply_nested_*` stack-safety tests pin this). See the DCE crate's identical
/// `#[inline(never)]` helper convention.
#[inline(never)]
fn fold_template_literal(t: &TemplateLiteral, st: &mut FoldState) -> Expression {
    // Recurse first so a substitution like `${1 + 2}` folds to `${3}`, and a
    // nested template `${`b${1}c`}` folds to `${"b1c"}` — by the time we test
    // the substitutions below they are already constants.
    let expressions: Vec<Expression> =
        t.expressions.iter().map(|e| fold_expression(e, st)).collect();

    // When EVERY substitution is a stringifiable constant literal AND every
    // quasi carries a cooked value, the whole template is a compile-time-known
    // string: interleave the quasis' cooked text with each substitution's
    // `ToString` form (`cooked₀ str(e₀) cooked₁ … cookedₙ`).
    //
    // Emitted as a plain string literal; no emitter work is needed because
    // closurec's string emitter already matches the reference compiler's quote
    // choice (single-quote when the value contains a `"`) and escaping (`\n`,
    // `\t`, `\\`) byte-for-byte.
    //
    // A `cooked` of `None` (an escape legal only in a *tagged* template) or a
    // non-const / BigInt / RegExp substitution makes the string not statically
    // known, so we decline and keep the template.
    let sub_strings: Option<Vec<String>> =
        expressions.iter().map(stringify_const_operand).collect();
    let cooked: Option<Vec<&str>> = t.quasis.iter().map(|q| q.cooked.as_deref()).collect();
    if let (Some(subs), Some(cooked)) = (sub_strings, cooked) {
        let mut result = String::new();
        for (i, quasi) in cooked.iter().enumerate() {
            result.push_str(quasi);
            if let Some(s) = subs.get(i) {
                result.push_str(s);
            }
        }
        let parent = t.cv.clone();
        let before = format!("template-literal[{} subs]", subs.len());
        let after = format!("\"{result}\"");
        let new_cv = st.fork_cv(&parent, &before, &after);
        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
    }

    Expression::TemplateLiteral(TemplateLiteral {
        cv: t.cv.clone(),
        quasis: t.quasis.clone(),
        expressions,
    })
}

/// The `ToString` form of a constant-literal template substitution, or `None`
/// if the operand is not a constant we can stringify at compile time.
///
/// Matches JavaScript `String(x)` for the primitives that appear as folded
/// constants: a number takes its shortest round-trip form (`3`, not `3.0`), a
/// string is itself, `true`/`false`/`null`/`undefined` take their keyword text.
/// A `BigInt` (its text would drop the `n`), a `RegExp`, or any non-literal
/// (identifier, call, …) returns `None`, so the enclosing template declines to
/// fold — exactly matching the reference compiler, which leaves
/// `` `a${x}b` `` / `` `a${f()}b` `` intact.
fn stringify_const_operand(e: &Expression) -> Option<String> {
    match e {
        Expression::NumericLiteral(n) => Some(format_js_number(n.value)),
        Expression::StringLiteral(s) => Some(s.value.clone()),
        Expression::BooleanLiteral(b) => Some(if b.value { "true" } else { "false" }.to_string()),
        Expression::NullLiteral(_) => Some("null".to_string()),
        Expression::UndefinedLiteral(_) => Some("undefined".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Method calls
// ---------------------------------------------------------------------

/// Folds the no-argument string-casing methods on a string literal:
///
/// ```text
///   "abc".toUpperCase()   →  "ABC"
///   "ABC".toLowerCase()   →  "abc"
///   "".toUpperCase()      →  ""
/// ```
///
/// **ASCII-only.** We fold only when the literal `is_ascii()`, using Rust's
/// `to_ascii_uppercase`/`to_ascii_lowercase`. ASCII case mapping is
/// locale-independent and byte-for-byte identical between Rust and JavaScript,
/// so the fold is exactly sound. Non-ASCII strings are deliberately left
/// alone: JS `toUpperCase`/`toLowerCase` use full Unicode default case
/// mapping with context-sensitive special cases (final sigma `ς`, German `ß`
/// → `SS`, locale-independent but length-changing) that a conservative
/// fold-set shouldn't try to reproduce here — `"é".toUpperCase()` stays a call.
///
/// Only the dotted, zero-argument form on a string literal folds. An argument
/// (`"x".toUpperCase(1)` — ignored by the runtime but we stay conservative),
/// the computed form `"x"["toUpperCase"]()`, and a method on a non-literal
/// (`s.toUpperCase()`) all pass through unchanged. We still recurse into the
/// callee and arguments so nested constants inside them fold.
/// Coerce a single array-literal element to the string `Array.prototype.join`
/// would produce for it, or `None` if the element is not a compile-time
/// constant we can represent faithfully.
///
/// Truth table (mirrors ECMAScript's per-element `ToString`, with the
/// join-specific rule that `null`/`undefined`/holes stringify to `""`):
///
/// ```text
///   element            join string
///   ----------------   -----------
///   hole (elision)     ""
///   null               ""
///   undefined          ""
///   "abc"              abc
///   42                 42          (String(Number))
///   true / false       true / false
///   anything else      None  → decline the whole fold
/// ```
fn join_element_str(element: &Option<Expression>) -> Option<String> {
    match element {
        // A hole (`[1, , 3]`) and the two nullish literals all join as `""`.
        None => Some(String::new()),
        Some(Expression::NullLiteral(_)) => Some(String::new()),
        Some(Expression::UndefinedLiteral(_)) => Some(String::new()),
        Some(Expression::StringLiteral(s)) => Some(s.value.clone()),
        Some(Expression::NumericLiteral(n)) => Some(format_js_number(n.value)),
        Some(Expression::BooleanLiteral(b)) => {
            Some(if b.value { "true" } else { "false" }.to_string())
        }
        // Nested arrays/objects, identifiers, calls, template literals, … all
        // have runtime-dependent string forms — decline so the call stands.
        _ => None,
    }
}

/// Fold `array.join(sep)` when every element is a join-representable constant.
/// Returns the joined string, or `None` to leave the call intact.
///
/// A length cap (mirroring `fold_string_repeat`'s DoS guard) prevents a crafted
/// array literal from materializing an oversized string at compile time.
fn fold_array_join(arr: &ArrayExpression, sep: &str) -> Option<String> {
    const MAX_JOIN_LEN: usize = 100_000;
    let mut parts: Vec<String> = Vec::with_capacity(arr.elements.len());
    let mut total = 0usize;
    for element in &arr.elements {
        let piece = join_element_str(element)?;
        total = total.saturating_add(piece.len()).saturating_add(sep.len());
        if total > MAX_JOIN_LEN {
            return None;
        }
        parts.push(piece);
    }
    Some(parts.join(sep))
}

fn fold_call(c: &CallExpression, st: &mut FoldState) -> Expression {
    let callee = fold_expression(&c.callee, st);
    let arguments: Vec<Expression> = c.arguments.iter().map(|a| fold_expression(a, st)).collect();

    if let Expression::MemberExpression(m) = &callee {
        if !m.computed {
            if let (Expression::StringLiteral(s), Expression::Identifier(id)) =
                (m.object.as_ref(), m.property.as_ref())
            {
                // ---- slice(start[, end]) → substring ----
                //
                // `"abcd".slice(1, 3)` → `"bc"`, `"abcd".slice(1)` → `"bcd"`,
                // `"abcd".slice(-2)` → `"cd"`, `"abc".slice()` → `"abc"`
                // (ECMAScript §22.1.3.22). Indices are UTF-16 code units;
                // negatives count from the end. Computed by `fold_string_slice`
                // below, which returns `None` (leaving the call) for a
                // non-integer-literal argument, more than two arguments, or a
                // cut that would split a surrogate pair into a lone surrogate.
                if id.name == "slice" {
                    if let Some(result) = fold_string_slice(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".slice({})", s.value, args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- substring(start[, end]) → the clamped, ordered cut ----
                //
                // `"abcd".substring(1, 3)` → `"bc"`, `"abcd".substring(2)` →
                // `"cd"`, `"abcd".substring(3, 1)` → `"bc"` (the endpoints SWAP
                // when start > end), `"abcd".substring(-2)` → `"abcd"` (a
                // negative — or `NaN` — clamps to 0; substring NEVER counts from
                // the end, that is `slice`'s job), `"abc".substring()` → `"abc"`
                // (ECMAScript §22.1.3.24). Indices are UTF-16 code units.
                // `fold_string_substring` returns `None` (leaving the call) for a
                // non-integer-literal argument, more than two arguments, or a cut
                // that would split a surrogate pair into a lone surrogate.
                else if id.name == "substring" {
                    if let Some(result) = fold_string_substring(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".substring({})", s.value, args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- substr(start[, length]) → a length-counted slice ----
                //
                // The legacy `String.prototype.substr` (ECMAScript Annex B
                // §B.2.3.1): the second argument is a *length*, not an end
                // index. `"abcde".substr(1, 2)` → `"bc"`, `"abcde".substr(1)` →
                // `"bcde"` (length defaults to the rest), `"abcde".substr(-2)` →
                // `"de"` (a negative start counts from the end, then clamps to
                // 0), `"abcde".substr(-2, 1)` → `"d"`, `"abcde".substr(10)` →
                // `""` (start past the end), `"abcde".substr(2, 0)` → `""`.
                // Indices are UTF-16 code units. `fold_string_substr` declines
                // (leaving the call) for a non-integer-literal argument, more
                // than two arguments, or a cut that would split a surrogate pair
                // into a lone surrogate.
                else if id.name == "substr" {
                    if let Some(result) = fold_string_substr(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".substr({})", s.value, args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- repeat(count) → the string concatenated `count` times ----
                //
                // `"ab".repeat(3)` → `"ababab"` (ECMAScript §22.1.3.18). The
                // count must be a non-negative integer literal; `fold_string_repeat`
                // declines (leaves the call) for a negative count (JS throws a
                // `RangeError`), a fractional/non-literal count, or a result
                // whose length would exceed a fixed cap — the cap keeps the
                // optimizer from materializing a megabyte string at compile time
                // (an algorithmic-blowup / DoS guard).
                else if id.name == "repeat" {
                    if let Some(result) = fold_string_repeat(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let count_src = match arguments.first() {
                            Some(Expression::NumericLiteral(n)) => format_js_number(n.value),
                            _ => "?".to_string(),
                        };
                        let before = format!("\"{}\".repeat({})", s.value, count_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- concat(...strings) → receiver followed by each argument ----
                //
                // `"a".concat("b", "c")` → `"abc"`, `"".concat("x")` → `"x"`,
                // `"a".concat()` → `"a"` (ECMAScript §22.1.3.4, the variadic
                // form). JS coerces every argument via `ToString`, and we now
                // model that for the primitive constants: `"x".concat(1, 2)` →
                // `"x12"`, `"a".concat(true)` → `"atrue"`, `"a".concat(null)` →
                // `"anull"` (note `ToString(null)` is `"null"`, NOT `""` — that
                // is `Array#join`'s rule). A non-constant / object / array
                // argument makes `fold_string_concat_call` decline and the call
                // is left for the runtime. Concatenating valid strings can only
                // ever yield valid UTF-16 (no surrogate pair is ever split, the
                // hazard `slice`/`charAt` guard against), so the result is always
                // a representable literal. The total length is still bounded by a
                // fixed cap as an algorithmic-blowup / DoS guard, mirroring
                // `repeat` and `padStart`/`padEnd`.
                else if id.name == "concat" {
                    if let Some(result) = fold_string_concat_call(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::StringLiteral(a) => format!("\"{}\"", a.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".concat({})", s.value, args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- split(separator[, limit]) → array of substrings ----
                //
                // `"a,b,c".split(",")` → `["a","b","c"]`,
                // `"axbxc".split("x")` → `["a","b","c"]`,
                // `"abc".split("")` → `["a","b","c"]` (empty separator splits
                // into single UTF-16 code units), `"".split(",")` → `[""]`,
                // `"".split("")` → `[]`, `"abc".split()` (no separator) →
                // `["abc"]` (ECMAScript §22.1.3.23). An optional second argument
                // is a non-negative integer LIMIT that caps the number of
                // pieces (`"a,b,c".split(",", 2)` → `["a","b"]`, limit 0 → `[]`).
                //
                // This is the FIRST fold that produces an *array* rather than a
                // scalar literal: the result is an `ArrayExpression` whose
                // elements are the piece strings, each a `StringLiteral`. The
                // array node and every element carry correlation-vector
                // provenance forked from the original call, so each produced
                // byte traces back to the `split` it came from.
                //
                // `fold_string_split` DECLINES (leaves the call for the
                // runtime, returning `None`) for: a non-string-literal
                // separator (a regular-expression separator needs a regex
                // engine; a numeric/identifier separator would need `ToString`
                // coercion we don't model), a non-integer / negative / non-
                // literal limit, more than two arguments, or — for the
                // empty-separator per-code-unit split — a receiver containing an
                // astral (non-BMP) character, since splitting its surrogate pair
                // would produce a lone surrogate that has no representable Rust
                // `String` (the same hazard `slice`/`charAt` guard against).
                // No output-size cap is needed: unlike `repeat`/`pad`, `split`
                // never amplifies — the pieces' total length never exceeds the
                // receiver's, so there is no algorithmic-blowup vector.
                else if id.name == "split" {
                    if let Some(parts) = fold_string_split(&s.value, &arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::StringLiteral(a) => format!("\"{}\"", a.value),
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".split({})", s.value, args_src);
                        let after = format!(
                            "[{}]",
                            parts
                                .iter()
                                .map(|p| format!("\"{}\"", p))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        let array_cv = st.fork_cv(&parent, &before, &after);
                        let elements: Vec<Option<Expression>> = parts
                            .into_iter()
                            .map(|p| {
                                let elem_after = format!("\"{}\"", p);
                                let elem_cv = st.fork_cv(&array_cv, &before, &elem_after);
                                Some(stamp_literal_cv(FoldedLiteral::String(p), elem_cv))
                            })
                            .collect();
                        return Expression::ArrayExpression(ArrayExpression {
                            cv: array_cv,
                            elements,
                        });
                    }
                }
                // ---- padStart(target[, pad]) / padEnd(target[, pad]) ----
                //
                // `"5".padStart(3, "0")` → `"005"`, `"abc".padEnd(6)` →
                // `"abc   "` (ECMAScript §22.1.3.16/17). Pads the string to a
                // target length (in UTF-16 code units) with a fill string
                // (default a single space), repeated and truncated to fit.
                // `fold_string_pad` declines for a non-integer target, a
                // non-string-literal pad, a target over the size cap, or a
                // truncation that would leave a lone surrogate.
                else if id.name == "padStart" || id.name == "padEnd" {
                    let at_start = id.name == "padStart";
                    if let Some(result) = fold_string_pad(&s.value, &arguments, at_start) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                Expression::StringLiteral(p) => format!("\"{}\"", p.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("\"{}\".{}({})", s.value, id.name, args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- two-argument literal replacement: replace / replaceAll ----
                //
                // `"…".replace(from, to)` substitutes the FIRST literal
                // occurrence of `from`; `replaceAll(from, to)` substitutes
                // EVERY occurrence. We fold only the string-pattern,
                // string-replacement overload — both arguments are string
                // literals. `fold_string_replace` declines the two cases JS
                // handles differently from a plain literal copy: a `to`
                // containing `$` (V8 expands `$$`/`$&`/`` $` ``/`$'`/`$n`
                // substitution patterns) and an empty `from` (V8 inserts at
                // every code-unit boundary). The string overload matches
                // `from` literally — no regex — so `"a.b".replace(".","X")`
                // → `"aXb"` is sound.
                else if (id.name == "replace" || id.name == "replaceAll")
                    && arguments.len() == 2
                {
                    if let (Expression::StringLiteral(from), Expression::StringLiteral(to)) =
                        (&arguments[0], &arguments[1])
                    {
                        if let Some(result) =
                            fold_string_replace(&id.name, &s.value, &from.value, &to.value)
                        {
                            let parent = c.cv.clone();
                            let before = format!(
                                "\"{}\".{}(\"{}\",\"{}\")",
                                s.value, id.name, from.value, to.value
                            );
                            let after = format!("\"{}\"", result);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                        }
                    }
                }
                // ---- zero-argument casing + trimming methods ----
                //
                // `toUpperCase`/`toLowerCase` are ASCII-only (full-Unicode case
                // mapping has length-changing special cases we don't reproduce);
                // `trim`/`trimStart`/`trimEnd` strip the ECMAScript whitespace
                // set (`fold_string_trim` below) from one or both ends.
                else if arguments.is_empty() {
                    let folded = match id.name.as_str() {
                        "toUpperCase" if s.value.is_ascii() => Some(s.value.to_ascii_uppercase()),
                        "toLowerCase" if s.value.is_ascii() => Some(s.value.to_ascii_lowercase()),
                        "trim" => Some(fold_string_trim(&s.value, true, true)),
                        "trimStart" => Some(fold_string_trim(&s.value, true, false)),
                        "trimEnd" => Some(fold_string_trim(&s.value, false, true)),
                        _ => None,
                    };
                    if let Some(result) = folded {
                        let parent = c.cv.clone();
                        let before = format!("\"{}\".{}()", s.value, id.name);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- single-argument string methods ----
                //
                // Two shapes share the one-argument arm, dispatched on the
                // argument's literal kind:
                //
                //   * a NUMERIC index   → `charCodeAt` / `charAt` (below);
                //   * a STRING needle   → `indexOf` (the `else if` further down).
                //
                // JS indexes a string by UTF-16 *code unit*, so we index into
                // `encode_utf16()` (an astral char occupies two units). The
                // index argument must be a non-negative integer literal; a
                // fractional, negative, or non-literal index is left for the
                // runtime (we stay conservative rather than model `ToInteger`
                // coercion and the NaN/"" out-of-range edge cases for those).
                else if arguments.len() == 1 {
                    if let Expression::NumericLiteral(n) = &arguments[0] {
                        if n.value.is_finite() && n.value >= 0.0 && n.value.fract() == 0.0 {
                            let units: Vec<u16> = s.value.encode_utf16().collect();
                            let i = n.value as usize;
                            match id.name.as_str() {
                                // `"abc".charCodeAt(i)` → the code unit at `i`.
                                // Out of range is JS `NaN`, for which there is
                                // no literal — so we simply don't fold it.
                                "charCodeAt" if i < units.len() => {
                                    let value = units[i] as f64;
                                    let parent = c.cv.clone();
                                    let before = format!("\"{}\".charCodeAt({})", s.value, i);
                                    let after = format_js_number(value);
                                    let new_cv = st.fork_cv(&parent, &before, &after);
                                    return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
                                }
                                // `"a💩b".codePointAt(i)` → the Unicode code
                                // POINT whose encoding starts at UTF-16 unit `i`
                                // (ECMAScript §22.1.3.4). Unlike `charCodeAt`
                                // (which returns a single 16-bit code UNIT),
                                // `codePointAt` combines a leading high surrogate
                                // at `i` with the following low surrogate into one
                                // astral code point in U+10000..=U+10FFFF — e.g.
                                // for "💩" (units [0xD83D, 0xDCA9])
                                // `codePointAt(0)` is 128169, whereas
                                // `charCodeAt(0)` is 55357. When `i` does NOT
                                // begin a surrogate pair — an ordinary BMP unit,
                                // a high surrogate with no following low, or a
                                // low surrogate itself — the result is just the
                                // code-unit value at `i`, identical to
                                // `charCodeAt`. Out of range is JS `undefined`,
                                // for which there is no literal, so we decline
                                // (`i < units.len()`). All arithmetic is on
                                // 16-bit values widened to `u32`, so the pair
                                // combination cannot overflow.
                                "codePointAt" if i < units.len() => {
                                    let hi = units[i];
                                    let value = if (0xD800..=0xDBFF).contains(&hi)
                                        && i + 1 < units.len()
                                        && (0xDC00..=0xDFFF).contains(&units[i + 1])
                                    {
                                        let lo = units[i + 1];
                                        (((hi as u32 - 0xD800) << 10)
                                            + (lo as u32 - 0xDC00)
                                            + 0x1_0000) as f64
                                    } else {
                                        hi as f64
                                    };
                                    let parent = c.cv.clone();
                                    let before =
                                        format!("\"{}\".codePointAt({})", s.value, i);
                                    let after = format_js_number(value);
                                    let new_cv = st.fork_cv(&parent, &before, &after);
                                    return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
                                }
                                // `"abc".charAt(i)` → the 1-code-unit string at
                                // `i`, or `""` when out of range. A lone
                                // surrogate (e.g. `"💩".charAt(0)`) is a valid
                                // length-1 JS string but cannot be a Rust
                                // `String`, so `from_utf16` fails and we leave
                                // that call unfolded (conservative, still sound).
                                "charAt" => {
                                    let result = if i < units.len() {
                                        String::from_utf16(&units[i..i + 1]).ok()
                                    } else {
                                        Some(String::new())
                                    };
                                    if let Some(result) = result {
                                        let parent = c.cv.clone();
                                        let before = format!("\"{}\".charAt({})", s.value, i);
                                        let after = format!("\"{}\"", result);
                                        let new_cv = st.fork_cv(&parent, &before, &after);
                                        return stamp_literal_cv(
                                            FoldedLiteral::String(result),
                                            new_cv,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        // ---- at(i): index with negative-from-end support ----
                        //
                        // `"abc".at(0)` → `"a"`, `"abc".at(2)` → `"c"`,
                        // `"abc".at(-1)` → `"c"`, `"abc".at(-3)` → `"a"`
                        // (ECMAScript §22.1.3.1). Unlike `charAt`, a NEGATIVE
                        // index counts from the end (`len + i`), and an
                        // out-of-range index returns `undefined` (NOT `""`) —
                        // for which there is no literal, so we decline and
                        // leave the call. The index must be an integer literal
                        // of any sign; a fractional or non-literal index is
                        // left for the runtime (we don't model `ToIntegerOr
                        // Infinity` coercion). Like `charAt`, a lone surrogate
                        // (`"💩".at(0)`) cannot be a Rust `String`, so
                        // `from_utf16` fails and we leave that call unfolded.
                        //
                        // `saturating_add` keeps the `len + i` computation from
                        // overflowing when `i` is a huge negative literal (the
                        // `as i64` cast already saturates a float past the i64
                        // range); a saturated index lands out of range and
                        // declines, never panicking.
                        if id.name == "at"
                            && n.value.is_finite()
                            && n.value.fract() == 0.0
                        {
                            let units: Vec<u16> = s.value.encode_utf16().collect();
                            let len = units.len() as i64;
                            let raw = n.value as i64;
                            let idx = if raw < 0 { len.saturating_add(raw) } else { raw };
                            if idx >= 0 && idx < len {
                                let u = idx as usize;
                                if let Ok(result) = String::from_utf16(&units[u..u + 1]) {
                                    let parent = c.cv.clone();
                                    let before = format!(
                                        "\"{}\".at({})",
                                        s.value,
                                        format_js_number(n.value)
                                    );
                                    let after = format!("\"{}\"", result);
                                    let new_cv = st.fork_cv(&parent, &before, &after);
                                    return stamp_literal_cv(
                                        FoldedLiteral::String(result),
                                        new_cv,
                                    );
                                }
                            }
                        }
                    }
                    // ---- substring search: indexOf ----
                    //
                    // `"haystack".indexOf("needle")` → the UTF-16 code-unit
                    // index of the first occurrence, or `-1` when absent
                    // (ECMAScript §22.1.3.8, the one-argument form). Both
                    // receiver and needle must be string literals.
                    //
                    // Rust's `str::find` returns a *byte* offset into the UTF-8
                    // haystack; JS reports the index in *UTF-16 code units*, so
                    // we re-measure the matched prefix with `encode_utf16()`
                    // (an astral char before the hit counts as two units). For
                    // ASCII the two indices coincide; the conversion keeps
                    // `"💩x".indexOf("x")` → `2`, matching V8. An empty needle
                    // yields `0` (`"abc".indexOf("")` is `0`), exactly as
                    // `str::find("")` returns `Some(0)`.
                    //
                    // Only the single-argument form folds; the `fromIndex`
                    // overload (`"abc".indexOf("b", 1)`) lands in the 2-arg
                    // arm and passes through unchanged.
                    else if let Expression::StringLiteral(needle) = &arguments[0] {
                        if id.name == "indexOf" {
                            let value = match s.value.find(&needle.value) {
                                Some(byte) => s.value[..byte].encode_utf16().count() as f64,
                                None => -1.0,
                            };
                            let parent = c.cv.clone();
                            let before =
                                format!("\"{}\".indexOf(\"{}\")", s.value, needle.value);
                            let after = format_js_number(value);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
                        }
                        // ---- substring search from the end: lastIndexOf ----
                        //
                        // `"abcabc".lastIndexOf("bc")` → the UTF-16 code-unit
                        // index of the *last* occurrence, or `-1` when absent
                        // (ECMAScript §22.1.3.9, the one-argument form). The
                        // mirror of `indexOf`: Rust's `str::rfind` returns the
                        // *byte* offset of the last match, which we re-measure in
                        // UTF-16 code units via `encode_utf16()` (an astral char
                        // before the hit counts as two units), so
                        // `"💩x💩x".lastIndexOf("x")` → `5`, matching V8.
                        //
                        // An empty needle yields the string *length* (in UTF-16
                        // units): `"abc".lastIndexOf("")` is `3`, and `str::rfind("")`
                        // returns `Some(byte_len)`, whose UTF-16 re-measure is
                        // exactly that length. Only the single-argument form
                        // folds; the `fromIndex` overload
                        // (`"abc".lastIndexOf("b", 0)`) carries a second argument
                        // and passes through to the runtime.
                        else if id.name == "lastIndexOf" {
                            let value = match s.value.rfind(&needle.value) {
                                Some(byte) => s.value[..byte].encode_utf16().count() as f64,
                                None => -1.0,
                            };
                            let parent = c.cv.clone();
                            let before =
                                format!("\"{}\".lastIndexOf(\"{}\")", s.value, needle.value);
                            let after = format_js_number(value);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
                        }
                        // ---- substring predicates: startsWith / endsWith / includes ----
                        //
                        // `"abc".startsWith("a")` → `true`, `"abc".endsWith("c")`
                        // → `true`, `"abc".includes("b")` → `true` (ECMAScript
                        // §22.1.3.{23,7,9}, each in its single-argument form).
                        // Both receiver and search string must be string
                        // literals; the result is a boolean literal, so the
                        // whole call collapses to `true`/`false`.
                        //
                        // JS compares these by UTF-16 code unit while Rust's
                        // `starts_with` / `ends_with` / `contains` compare UTF-8
                        // bytes — but the two ALWAYS agree here, because both
                        // operands are valid Rust `String`s (no lone
                        // surrogates), and prefix / suffix / substring matching
                        // over a sequence of whole Unicode scalar values gives
                        // the same yes/no answer in either encoding (each
                        // encoding is deterministic and self-synchronizing per
                        // scalar, so a match can only land on scalar
                        // boundaries). `"a💩b".includes("💩")` is `true` in both,
                        // and an empty needle is always present (`""` is a
                        // prefix, a suffix, and a substring of everything),
                        // matching `str`'s behavior exactly.
                        //
                        // Only the single-argument form folds; the position
                        // overloads (`"abc".startsWith("b", 1)`, etc.) carry a
                        // second argument and so never reach this one-argument
                        // arm — they pass through to the runtime.
                        else if let Some(value) =
                            fold_string_predicate(&id.name, &s.value, &needle.value)
                        {
                            let parent = c.cv.clone();
                            let before = format!(
                                "\"{}\".{}(\"{}\")",
                                s.value, id.name, needle.value
                            );
                            let after = if value { "true" } else { "false" };
                            let new_cv = st.fork_cv(&parent, &before, after);
                            return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
                        }
                    }
                }
            }

            // ---- `[a, b, c].join(sep)` on an array literal of constants ----
            //
            // `["a","b","c"].join("-")` → `"a-b-c"`, `[1,2,3].join()` →
            // `"1,2,3"` (the separator defaults to `","`), `[].join("-")` →
            // `""` (ECMAScript §23.1.3.16). Each element is coerced to a string
            // the way `Array.prototype.join` does: `null`, `undefined`, and
            // array HOLES all become the empty string `""`; numbers and
            // booleans take their `String(...)` form. We DECLINE (leave the
            // call intact) if any element is something we cannot coerce at
            // compile time without changing semantics — a nested array/object
            // (its own `toString` runs at runtime), an identifier, a call, etc.
            // — or if the separator is anything other than an absent argument or
            // a single STRING literal (a numeric separator like `[1,2].join(0)`
            // coerces to `"0"`, which we leave for the runtime to keep the fold
            // obviously correct). `fold_array_join` also caps the result length
            // as an algorithmic-blowup guard.
            if let (Expression::ArrayExpression(arr), Expression::Identifier(id)) =
                (m.object.as_ref(), m.property.as_ref())
            {
                if id.name == "join" {
                    let sep = match arguments.as_slice() {
                        [] => Some(",".to_string()),
                        [Expression::StringLiteral(s)] => Some(s.value.clone()),
                        _ => None,
                    };
                    if let Some(sep) = sep {
                        if let Some(result) = fold_array_join(arr, &sep) {
                            let parent = c.cv.clone();
                            let sep_src = match arguments.first() {
                                Some(Expression::StringLiteral(s)) => format!("\"{}\"", s.value),
                                _ => String::new(),
                            };
                            let before = format!("[...].join({sep_src})");
                            let after = format!("\"{result}\"");
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                        }
                    }
                }
            }

            // ---- numeric `toString([radix])` on a non-negative integer ----
            //
            // `(255).toString()` → `"255"`, `(255).toString(16)` → `"ff"`,
            // `(255).toString(2)` → `"11111111"` (ECMAScript §21.1.3.6).
            // The receiver must be a NON-NEGATIVE INTEGER literal (a numeric
            // literal is never negative in the AST — `-255` is a unary
            // expression — so this is automatic, but we assert it anyway), and
            // small enough to format with plain digits in every radix
            // (`< 2^53`, the safe-integer ceiling; beyond it JS switches to
            // exponential notation, which a digit loop would not reproduce).
            // The radix is the default 10 or a single integer-literal argument
            // in `2..=36`; anything else (a variable radix, a fractional or
            // out-of-range radix) is left for the runtime.
            if let (Expression::NumericLiteral(n), Expression::Identifier(id)) =
                (m.object.as_ref(), m.property.as_ref())
            {
                if id.name == "toString"
                    && n.value.is_finite()
                    && n.value >= 0.0
                    && n.value.fract() == 0.0
                    && n.value < 9_007_199_254_740_992.0
                {
                    let radix = match arguments.as_slice() {
                        [] => Some(10u32),
                        [Expression::NumericLiteral(r)]
                            if r.value.fract() == 0.0 && (2.0..=36.0).contains(&r.value) =>
                        {
                            Some(r.value as u32)
                        }
                        _ => None,
                    };
                    if let Some(radix) = radix {
                        let result = to_radix_string(n.value as u64, radix);
                        let parent = c.cv.clone();
                        let before = if radix == 10 {
                            format!("({}).toString()", format_js_number(n.value))
                        } else {
                            format!("({}).toString({})", format_js_number(n.value), radix)
                        };
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
            }
            // ---- String.fromCharCode(u0, u1, …) → the built string ----
            //
            // The static `String.fromCharCode` (ECMAScript §22.1.2.1) builds a
            // string from UTF-16 code UNITS: `String.fromCharCode(72, 73)` →
            // `"HI"`, and an adjacent high+low surrogate pair assembles one
            // astral character — `String.fromCharCode(0xD83D, 0xDCA9)` → `"💩"`.
            // No arguments yields `""`.
            //
            // SOUNDNESS — this folds under the same "builtins intact" premise as
            // every fold here, but, like `parseInt`/`parseFloat` below, one notch
            // weaker: `String` is a *free identifier*, so a local binding
            // (`let String = …`) could mask it. We fold anyway — matching Closure
            // Compiler, which treats redefining the global as out of scope — but
            // ONLY when the receiver is the bare identifier `String` (never a
            // member access like `window.String`, which would carry a non-
            // Identifier object). Each argument must be a non-negative integer
            // literal that fits in 16 bits (`0..=0xFFFF`); JS applies `ToUint16`
            // (mod 2^16) to every argument, but we stay conservative and DECLINE
            // for a fractional, negative, out-of-16-bit, or non-literal argument
            // rather than model that coercion. We also DECLINE when the units do
            // not form valid UTF-16 — a LONE surrogate is a valid JS string but
            // cannot be a Rust `String` (the same hazard `slice`/`charAt`/
            // `codePointAt` guard against).
            if let (Expression::Identifier(obj), Expression::Identifier(prop)) =
                (m.object.as_ref(), m.property.as_ref())
            {
                if obj.name == "String" && prop.name == "fromCharCode" {
                    if let Some(result) = fold_string_from_char_code(&arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("String.fromCharCode({})", args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }
                // ---- String.fromCodePoint(cp0, cp1, …) → the built string ----
                //
                // The static `String.fromCodePoint` (ECMAScript §22.1.2.2) builds
                // a string from Unicode CODE POINTS — unlike `fromCharCode`, whose
                // arguments are 16-bit UTF-16 *units*. So a single astral argument
                // suffices: `String.fromCodePoint(128169)` → `"💩"` (U+1F4A9),
                // `String.fromCodePoint(72, 73)` → `"HI"`, no args → `""`. Same
                // bare-global-`String` soundness premise as `fromCharCode`; each
                // argument must be a non-negative integer literal that is a VALID
                // code point (`0..=0x10FFFF`, not a surrogate) — `char::from_u32`
                // returns `None` for exactly the inputs JS would throw on or that
                // can't be a Rust `char`, so we DECLINE rather than mis-fold.
                if obj.name == "String" && prop.name == "fromCodePoint" {
                    if let Some(result) = fold_string_from_code_point(&arguments) {
                        let parent = c.cv.clone();
                        let args_src = arguments
                            .iter()
                            .map(|a| match a {
                                Expression::NumericLiteral(n) => format_js_number(n.value),
                                _ => "?".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let before = format!("String.fromCodePoint({})", args_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }

                // ---- Number.parseInt(string[, radix]) / Number.parseFloat(string) ----
                //
                // The ES2015 static methods (ECMAScript §21.1.2.12/.13) are the
                // SAME function objects as the global `parseInt`/`parseFloat` —
                // `Number.parseInt === parseInt` — so they run the identical
                // algorithm and we reuse `fold_parse_int`/`fold_parse_float`:
                // `Number.parseInt("12px")` → `12`, `Number.parseInt("FF", 16)` →
                // `255`, `Number.parseFloat("3.14abc")` → `3.14`. As with the
                // global forms, we DECLINE (leave the call) when the result is
                // `NaN`/`±Infinity` (no literal to substitute), and `parseInt`
                // only folds with a missing or integer-literal radix.
                //
                // These dispatch HERE (the MemberExpression arm) rather than the
                // free-identifier arm because the callee is `Number.parseX`, a
                // member access — so only the bare global `Number` folds, never a
                // shadowed receiver (`n.parseInt(...)` is left alone), the same
                // premise as the `String.from*` statics.
                if obj.name == "Number"
                    && matches!(prop.name.as_str(), "parseInt" | "parseFloat")
                {
                    if let Some(Expression::StringLiteral(s)) = arguments.first() {
                        let folded = match prop.name.as_str() {
                            "parseInt" if arguments.len() <= 2 => match arguments.get(1) {
                                None => fold_parse_int(&s.value, None),
                                Some(Expression::NumericLiteral(r))
                                    if r.value.is_finite() && r.value.fract() == 0.0 =>
                                {
                                    fold_parse_int(&s.value, Some(r.value))
                                }
                                Some(_) => None,
                            },
                            "parseFloat" if arguments.len() == 1 => {
                                fold_parse_float(&s.value)
                            }
                            _ => None,
                        };
                        if let Some(value) = folded {
                            let parent = c.cv.clone();
                            let before = match arguments.get(1) {
                                Some(Expression::NumericLiteral(r)) => format!(
                                    "Number.{}(\"{}\",{})",
                                    prop.name,
                                    s.value,
                                    format_js_number(r.value)
                                ),
                                _ => format!("Number.{}(\"{}\")", prop.name, s.value),
                            };
                            let after = format_js_number(value);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
                        }
                    }
                }

                // ---- Number.isInteger / isFinite / isNaN / isSafeInteger (x) → boolean ----
                //
                // The ES2015 static predicates (ECMAScript §21.1.2.2/.3/.4/.5).
                // UNLIKE the *global* `isNaN`/`isFinite`, these do **no** coercion:
                // their argument must already BE a Number, otherwise the answer is
                // `false` with no `ToNumber` step (`Number.isNaN("NaN")` → `false`,
                // `Number.isInteger("5")` → `false`). So:
                //
                //   * a NUMBER literal → classify its value directly —
                //     `Number.isNaN(v)` = `v` is `NaN` (never, for a literal),
                //     `Number.isFinite(v)` = `v` is finite,
                //     `Number.isInteger(v)` = `v` is finite AND has no fraction
                //     (so `Number.isInteger(1e21)` → `true`, `…(3.5)` → `false`,
                //     `…(Infinity)` → `false`),
                //     `Number.isSafeInteger(v)` = `v` is an integer whose magnitude
                //     does not exceed 2^53−1 (`Number.MAX_SAFE_INTEGER` =
                //     9007199254740991, the largest integer the f64 mantissa
                //     represents without colliding with a neighbour): so
                //     `Number.isSafeInteger(7)` → `true`,
                //     `…(9007199254740991)` → `true`, `…(9007199254740992)` (2^53)
                //     → `false`, `…(3.5)` / `…(1e21)` / `…(Infinity)` → `false`;
                //   * a STRING / BOOLEAN / NULL literal → `false` for all four
                //     (it is provably not a Number, and there is no coercion).
                //
                // Any other argument (an identifier, an array/object, a missing or
                // extra argument) is left for the runtime — we can't prove its type
                // at compile time. Same bare-global-`Number` soundness premise as
                // the `String.from*` statics (a member access, not a free
                // identifier, so only the literal `Number.isX(...)` callee folds).
                if obj.name == "Number"
                    && matches!(
                        prop.name.as_str(),
                        "isInteger" | "isFinite" | "isNaN" | "isSafeInteger"
                    )
                    && arguments.len() == 1
                {
                    let folded: Option<bool> = match arguments.first() {
                        Some(Expression::NumericLiteral(n)) => Some(match prop.name.as_str() {
                            "isNaN" => n.value.is_nan(),
                            "isFinite" => n.value.is_finite(),
                            // A *safe* integer is a finite integer whose magnitude
                            // is ≤ 2^53−1 (9007199254740991). Beyond that, distinct
                            // mathematical integers share one f64, so `isSafeInteger`
                            // is `false` even though `isInteger` stays `true`.
                            "isSafeInteger" => {
                                n.value.is_finite()
                                    && n.value.fract() == 0.0
                                    && n.value.abs() <= 9_007_199_254_740_991.0
                            }
                            // Integer ⟺ finite with a zero fractional part. Every
                            // f64 magnitude ≥ 2^52 is integer-valued (`fract()==0`),
                            // matching V8 for large literals like `1e21`.
                            _ => n.value.is_finite() && n.value.fract() == 0.0,
                        }),
                        // A non-number literal is provably not a Number — all three
                        // predicates are `false`, with no coercion.
                        Some(Expression::StringLiteral(_))
                        | Some(Expression::BooleanLiteral(_))
                        | Some(Expression::NullLiteral(_)) => Some(false),
                        _ => None,
                    };
                    if let Some(value) = folded {
                        let parent = c.cv.clone();
                        let arg_src = match arguments.first() {
                            Some(Expression::NumericLiteral(n)) => format_js_number(n.value),
                            Some(Expression::StringLiteral(s)) => format!("\"{}\"", s.value),
                            Some(Expression::BooleanLiteral(b)) => b.value.to_string(),
                            Some(Expression::NullLiteral(_)) => "null".to_string(),
                            _ => "?".to_string(),
                        };
                        let before = format!("Number.{}({})", prop.name, arg_src);
                        let after = if value { "!0" } else { "!1" };
                        let new_cv = st.fork_cv(&parent, &before, after);
                        return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
                    }
                }

                // ---- JSON.stringify(x) → string literal (primitive subset) ----
                //
                // `JSON.stringify` (ECMAScript §25.5.2) serialises a value to JSON
                // text. We fold ONLY the primitive literal arguments whose JSON
                // text we can render exactly, and ONLY the single-argument form (a
                // `replacer`/`space` second argument can change the result — a
                // replacer function is invoked even on a primitive value):
                //
                //   * a NUMBER literal → its `ToString` (a JSON number is the same
                //     spelling): `JSON.stringify(42)` → the string `"42"`. We reuse
                //     `fold_string_of_number`, which declines fractional values and
                //     magnitudes ≥ 2^53 (whose shortest-decimal / exponential
                //     spelling could diverge), so those leave the call intact;
                //   * a BOOLEAN literal → `"true"` / `"false"`;
                //   * the NULL literal → `"null"`.
                //
                // We DECLINE a STRING literal (JSON escaping — quotes, backslashes,
                // control characters, and the `U+2028`/`U+2029` edge cases — is
                // subtle enough to leave to the runtime) and any array/object
                // literal (its elements/properties may have side effects, and the
                // serialisation recurses). An identifier or other non-literal is
                // also declined. Same bare-global-`JSON` premise as the other
                // statics — only the literal `JSON.stringify(...)` callee folds,
                // never a shadowed receiver. The folded text is pure ASCII
                // (digits / `true` / `false` / `null`), so it needs no escaping.
                if obj.name == "JSON" && prop.name == "stringify" && arguments.len() == 1 {
                    let folded: Option<String> = match arguments.first() {
                        Some(Expression::NumericLiteral(n)) => fold_string_of_number(n.value),
                        Some(Expression::BooleanLiteral(b)) => {
                            Some(if b.value { "true" } else { "false" }.to_string())
                        }
                        Some(Expression::NullLiteral(_)) => Some("null".to_string()),
                        _ => None,
                    };
                    if let Some(result) = folded {
                        let parent = c.cv.clone();
                        let arg_src = match arguments.first() {
                            Some(Expression::NumericLiteral(n)) => format_js_number(n.value),
                            Some(Expression::BooleanLiteral(b)) => b.value.to_string(),
                            Some(Expression::NullLiteral(_)) => "null".to_string(),
                            _ => "?".to_string(),
                        };
                        let before = format!("JSON.stringify({})", arg_src);
                        let after = format!("\"{}\"", result);
                        let new_cv = st.fork_cv(&parent, &before, &after);
                        return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                    }
                }

                // ---- Array.isArray(x) → boolean ----
                //
                // The static `Array.isArray` (ECMAScript §22.1.2.2) tests whether
                // its single argument is a real Array, with NO coercion. We decide
                // it at compile time only for the literal shapes whose evaluation
                // has NO observable side effect to drop:
                //
                //   * an EMPTY array literal `[]` → `true`;
                //   * an EMPTY object literal `{}` → `false`;
                //   * a primitive literal (string / number / boolean / null) →
                //     `false` (provably not an Array).
                //
                // A NON-empty array/object literal is DECLINED: replacing the call
                // with a boolean would discard the element/property expressions and
                // drop any side effect they evaluate (`Array.isArray([f()])` must
                // still call `f`). An identifier or any other non-literal is also
                // declined (unknown type at compile time). Same bare-global-`Array`
                // premise as the `String.from*` / `Number.isX` statics — only the
                // literal `Array.isArray(...)` callee folds, never a shadowed
                // receiver (`x.isArray(...)` is left alone).
                if obj.name == "Array" && prop.name == "isArray" && arguments.len() == 1 {
                    let folded: Option<bool> = match arguments.first() {
                        Some(Expression::ArrayExpression(a)) if a.elements.is_empty() => {
                            Some(true)
                        }
                        Some(Expression::ObjectExpression(o)) if o.properties.is_empty() => {
                            Some(false)
                        }
                        Some(Expression::StringLiteral(_))
                        | Some(Expression::NumericLiteral(_))
                        | Some(Expression::BooleanLiteral(_))
                        | Some(Expression::NullLiteral(_)) => Some(false),
                        _ => None,
                    };
                    if let Some(value) = folded {
                        let parent = c.cv.clone();
                        let arg_src = match arguments.first() {
                            Some(Expression::ArrayExpression(_)) => "[]".to_string(),
                            Some(Expression::ObjectExpression(_)) => "{}".to_string(),
                            Some(Expression::StringLiteral(s)) => format!("\"{}\"", s.value),
                            Some(Expression::NumericLiteral(n)) => format_js_number(n.value),
                            Some(Expression::BooleanLiteral(b)) => b.value.to_string(),
                            Some(Expression::NullLiteral(_)) => "null".to_string(),
                            _ => "?".to_string(),
                        };
                        let before = format!("Array.isArray({})", arg_src);
                        let after = if value { "!0" } else { "!1" };
                        let new_cv = st.fork_cv(&parent, &before, after);
                        return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
                    }
                }

                // ---- Array.from("…") → array of code-point strings ----
                //
                // `Array.from` (ECMAScript §23.1.2.1) builds an array from an
                // iterable or array-like. For a STRING the iterator yields one
                // element per CODE POINT (not per UTF-16 code unit) — exactly what
                // the spread `[..."…"]` produces — so `Array.from("abc")` →
                // `["a", "b", "c"]` and `Array.from("a💩b")` → `["a", "💩", "b"]`
                // (the astral `💩` is a SINGLE element, never split into its two
                // surrogate halves). Folding a string LITERAL to that array
                // literal is exact and side-effect-free; the empty string → `[]`.
                //
                // We fold ONLY the single-string-literal-argument form. A SECOND
                // argument is a `mapFn` whose return values we cannot compute at
                // compile time, so we decline it. Any non-string-literal first
                // argument (an array-like object, a real iterable, an identifier,
                // a number) is also declined — its iteration result is unknown.
                // Same bare-global-`Array` premise as `Array.isArray` — only the
                // literal `Array.from(...)` callee folds, never a shadowed
                // receiver (`a.from(...)` is left alone).
                if obj.name == "Array" && prop.name == "from" && arguments.len() == 1 {
                    if let Some(Expression::StringLiteral(s)) = arguments.first() {
                        let parent = c.cv.clone();
                        // Rust's `chars()` iterates Unicode scalar values — i.e.
                        // code points — matching the string iterator JS uses, so
                        // an astral char stays a single element.
                        let code_points: Vec<String> =
                            s.value.chars().map(|c| c.to_string()).collect();
                        let before = format!("Array.from(\"{}\")", s.value);
                        let after = format!(
                            "[{}]",
                            code_points
                                .iter()
                                .map(|p| format!("\"{}\"", p))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        let array_cv = st.fork_cv(&parent, &before, &after);
                        let elements: Vec<Option<Expression>> = code_points
                            .into_iter()
                            .map(|p| {
                                let elem_after = format!("\"{}\"", p);
                                let elem_cv = st.fork_cv(&array_cv, &before, &elem_after);
                                Some(stamp_literal_cv(FoldedLiteral::String(p), elem_cv))
                            })
                            .collect();
                        return Expression::ArrayExpression(ArrayExpression {
                            cv: array_cv,
                            elements,
                        });
                    }
                }
                // ---- Object.keys / values / entries ({}) → [] ----
                //
                // The static `Object.keys`/`values`/`entries` (ECMAScript
                // §20.1.2.16/.22/.5) enumerate an object's own enumerable string
                // keys. For an EMPTY object literal `{}` the result is ALWAYS the
                // empty array `[]` — there are no keys, and evaluating `{}` has no
                // observable side effect, so collapsing the call to `[]` is sound.
                //
                // We fold ONLY the empty-object-literal case. A NON-empty object
                // literal is declined: its property *values* (and computed keys /
                // spreads) may have side effects that collapsing to `[]` would
                // drop, and the result is non-empty anyway. An array literal, a
                // primitive (`Object.keys("ab")` → `["0","1"]`), an identifier, or
                // a call with ≠1 argument is also declined (the result isn't a
                // known empty array, or the type is unknown at compile time). Same
                // bare-global-`Object` premise as the other statics — only the
                // literal `Object.keys(...)` callee folds, never a shadowed
                // receiver (`o.keys(...)` is left alone).
                if obj.name == "Object"
                    && matches!(prop.name.as_str(), "keys" | "values" | "entries")
                    && arguments.len() == 1
                {
                    if let Some(Expression::ObjectExpression(o)) = arguments.first() {
                        if o.properties.is_empty() {
                            let parent = c.cv.clone();
                            let before = format!("Object.{}({{}})", prop.name);
                            let new_cv = st.fork_cv(&parent, &before, "[]");
                            return Expression::ArrayExpression(ArrayExpression {
                                cv: new_cv,
                                elements: vec![],
                            });
                        }
                    }
                }

                // ---- Object.entries({k: v, …}) → [["k", v], …] ----
                //
                // `Object.entries` (ECMAScript §20.1.2.5) returns an array of an
                // object's own enumerable string-keyed `[key, value]` pairs — the
                // exact inverse of `Object.fromEntries`. For a fully-static object
                // LITERAL we can build that array at compile time:
                //
                //   Object.entries({a: 1, b: 2})  → [["a", 1], ["b", 2]]
                //   Object.entries({x: "hi"})     → [["x", "hi"]]
                //
                // (The EMPTY-object case `Object.entries({})` → `[]` is already
                // handled by the block above; this block fires only for non-empty
                // literals.) Each entry KEY is always a string, so we emit a string
                // literal; the VALUE expression is copied verbatim.
                //
                // Soundness conditions — EVERY property must satisfy all of these,
                // else we DECLINE and leave the call untouched (declining is safe):
                //   * the property is a plain data property `k: v` — NOT a getter or
                //     setter (`get k() {…}` runs code), NOT a method, NOT a computed
                //     key `[expr]: v` (the key is unknown);
                //   * the VALUE is a primitive literal (string / number / boolean /
                //     null) — a non-literal value (including the implicit identifier
                //     of a shorthand `{x}`) may have side effects or an unknown value;
                //   * the key is NOT `"__proto__"` — a non-computed `{__proto__: v}`
                //     is the §B.3.1 prototype SETTER, which creates NO own property,
                //     so `Object.entries` would not enumerate it (folding it in would
                //     invent an entry that does not exist);
                //   * NO key is a canonical ARRAY INDEX (e.g. `0`, `1`, `42`):
                //     `[[OwnPropertyKeys]]` lists integer-index keys first, in numeric
                //     order, ahead of the source insertion order we emit, so a single
                //     index key could reorder the result.
                //
                // DUPLICATE keys in the source literal collapse to one own property
                // whose value is the LAST occurrence (kept at the FIRST occurrence's
                // position), exactly mirroring the object the literal builds. Same
                // bare-global-`Object` premise as the other statics — only the literal
                // `Object.entries(...)` callee folds, never a shadowed receiver.
                if obj.name == "Object" && prop.name == "entries" && arguments.len() == 1 {
                    if let Some(Expression::ObjectExpression(o)) = arguments.first() {
                        if !o.properties.is_empty() {
                            if let Some(pairs) = fold_object_entries_pairs(&o.properties) {
                                let parent = c.cv.clone();
                                let before =
                                    format!("Object.entries({{{} prop(s)}})", o.properties.len());
                                let after = format!("[{} pair(s)]", pairs.len());
                                let new_cv = st.fork_cv(&parent, &before, &after);
                                return Expression::ArrayExpression(ArrayExpression {
                                    cv: new_cv,
                                    elements: pairs.into_iter().map(Some).collect(),
                                });
                            }
                        }
                    }
                }

                // ---- Object.keys({k: v, …}) → ["k", …] ----
                //
                // `Object.keys` (ECMAScript §20.1.2.16) returns an array of an
                // object's own enumerable string keys, in property-enumeration
                // order. For a fully-static object LITERAL we can build that array
                // of key strings at compile time:
                //
                //   Object.keys({a: 1, b: 2})  → ["a", "b"]
                //   Object.keys({x: "hi"})     → ["x"]
                //
                // (The EMPTY-object case `Object.keys({})` → `[]` is already handled
                // by the combined block above; this block fires only for non-empty
                // literals.) Each key is always a string, so we emit a string
                // literal; the VALUE is DROPPED.
                //
                // Dropping the value is exactly why the soundness conditions are the
                // SAME as `Object.entries`, NOT weaker: even though `Object.keys`
                // never emits the value, the value EXPRESSION is still evaluated when
                // the source object literal is built. `Object.keys({a: foo()})` runs
                // `foo()`; folding to `["a"]` would silently DROP that call, and
                // `Object.keys({a: x})` would no longer throw if `x` is undeclared.
                // So we still require every value to be a side-effect-free primitive
                // literal (string / number / boolean / null) and reject the same
                // shapes entries does — getters/setters/methods (run code), computed
                // keys (unknown key), `__proto__` (the §B.3.1 setter makes no own
                // property), and any canonical array-index key (enumerated first, in
                // numeric order, which would reorder the result). Declining is always
                // safe — the call is simply left untouched. Same bare-global-`Object`
                // premise: only the literal `Object.keys(...)` callee folds, never a
                // shadowed receiver (`o.keys(...)` is left alone).
                if obj.name == "Object" && prop.name == "keys" && arguments.len() == 1 {
                    if let Some(Expression::ObjectExpression(o)) = arguments.first() {
                        if !o.properties.is_empty() {
                            if let Some(names) = fold_object_keys_names(&o.properties) {
                                let parent = c.cv.clone();
                                let before =
                                    format!("Object.keys({{{} prop(s)}})", o.properties.len());
                                let after = format!("[{} key(s)]", names.len());
                                let new_cv = st.fork_cv(&parent, &before, &after);
                                return Expression::ArrayExpression(ArrayExpression {
                                    cv: new_cv,
                                    elements: names.into_iter().map(Some).collect(),
                                });
                            }
                        }
                    }
                }

                // ---- Object.is(a, b) → boolean (SameValue) ----
                //
                // `Object.is` (ECMAScript §20.1.2.13) compares two values with the
                // SameValue algorithm (§7.2.11), which differs from `===` in exactly
                // two cases:
                //
                //   * `Object.is(NaN, NaN)` is `true`   (=== gives `false`);
                //   * `Object.is(+0, -0)`   is `false`  (=== gives `true`);
                //
                // everywhere else SameValue agrees with `===`: same-type primitives
                // are equal iff their values are equal, and operands of different
                // types are never the same. We fold ONLY when BOTH arguments are
                // primitive LITERALS whose values we know exactly:
                //
                //   * two NUMBER literals → SameValue on the f64 values (NaN==NaN,
                //     +0 ≠ -0 via the sign bit, otherwise `==`);
                //   * two STRING literals → byte-equal;
                //   * two BOOLEAN literals → equal;
                //   * two NULL literals → `true`;
                //   * a MISMATCH of literal kinds (number vs string, null vs
                //     boolean, …) → `false` (SameValue requires the same Type).
                //
                // We DECLINE if EITHER argument is a non-literal (identifier, array,
                // object, call — its value is unknown at compile time) or the call
                // does not have exactly two arguments. Same bare-global-`Object`
                // premise as `Object.keys` — only the literal `Object.is(...)`
                // callee folds, never a shadowed receiver (`o.is(...)`).
                if obj.name == "Object" && prop.name == "is" && arguments.len() == 2 {
                    // SameValue on two f64 literals: NaN is the same as NaN; +0 and
                    // −0 are distinguished by their sign; otherwise ordinary `==`.
                    fn same_value_number(x: f64, y: f64) -> bool {
                        if x.is_nan() && y.is_nan() {
                            true
                        } else if x == 0.0 && y == 0.0 {
                            x.is_sign_negative() == y.is_sign_negative()
                        } else {
                            x == y
                        }
                    }
                    let folded: Option<bool> = match (&arguments[0], &arguments[1]) {
                        (Expression::NumericLiteral(a), Expression::NumericLiteral(b)) => {
                            Some(same_value_number(a.value, b.value))
                        }
                        (Expression::StringLiteral(a), Expression::StringLiteral(b)) => {
                            Some(a.value == b.value)
                        }
                        (Expression::BooleanLiteral(a), Expression::BooleanLiteral(b)) => {
                            Some(a.value == b.value)
                        }
                        (Expression::NullLiteral(_), Expression::NullLiteral(_)) => Some(true),
                        // A mismatch of two *known* primitive-literal kinds is
                        // provably a different Type → SameValue is `false`.
                        (
                            Expression::NumericLiteral(_)
                            | Expression::StringLiteral(_)
                            | Expression::BooleanLiteral(_)
                            | Expression::NullLiteral(_),
                            Expression::NumericLiteral(_)
                            | Expression::StringLiteral(_)
                            | Expression::BooleanLiteral(_)
                            | Expression::NullLiteral(_),
                        ) => Some(false),
                        // At least one operand is a non-literal — value unknown.
                        _ => None,
                    };
                    if let Some(value) = folded {
                        let parent = c.cv.clone();
                        let before = "Object.is(a, b)".to_string();
                        let after = if value { "!0" } else { "!1" };
                        let new_cv = st.fork_cv(&parent, &before, after);
                        return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
                    }
                }

                // ---- Array.of(v0, v1, …) → array literal `[v0, v1, …]` ----
                //
                // `Array.of` (ECMAScript §23.1.2.3) ALWAYS builds a fresh array
                // whose elements are EXACTLY its arguments, in order. Crucially it
                // is NOT the `Array(…)` constructor: a single numeric argument to
                // `Array` sets the LENGTH (`Array(7)` is a 7-hole array of length 7),
                // whereas `Array.of(7)` is the one-element array `[7]`. So for ANY
                // argument list — including side-effecting, identifier, or call
                // arguments — `Array.of(a, b, c)` is byte-for-byte the array literal
                // `[a, b, c]`:
                //
                //   Array.of()        → []
                //   Array.of(7)       → [7]        (NOT Array(7)'s length-7 array!)
                //   Array.of(1, 2, 3) → [1, 2, 3]
                //   Array.of(f(), x)  → [f(), x]    (f() still called, order kept)
                //
                // Folding to an array literal preserves every element expression in
                // evaluation order, so no argument is dropped, duplicated, or
                // reordered and all side effects are retained — the fold is sound
                // for every argument list. We would DECLINE only a spread argument
                // (`Array.of(...xs)`), whose element count is unknown at compile
                // time; the AST has no spread variant in call arguments today (only
                // object spread is contemplated, as a future "Phase 2"), so every
                // argument is a plain expression and the fold always applies. The
                // guard below is written against `arguments` directly so that, if a
                // call-argument spread node is ever added, it can be matched and
                // declined here. Same bare-global-`Array` premise as `Array.isArray`
                // — only the literal `Array.of(...)` callee folds, never a shadowed
                // receiver (`a.of(...)` is left alone).
                if obj.name == "Array" && prop.name == "of" {
                    let parent = c.cv.clone();
                    let before = format!("Array.of({} arg(s))", arguments.len());
                    let after = format!("[{} elem(s)]", arguments.len());
                    let new_cv = st.fork_cv(&parent, &before, &after);
                    return Expression::ArrayExpression(ArrayExpression {
                        cv: new_cv,
                        elements: arguments.iter().map(|a| Some(a.clone())).collect(),
                    });
                }

                // ---- Math.max(n0, n1, …) / Math.min(…) → numeric literal ----
                //
                // `Math.max` / `Math.min` (ECMAScript §21.3.2.24 / .25) coerce each
                // argument with ToNumber and return the largest / smallest. When
                // EVERY argument is already a numeric literal we can evaluate the
                // result at compile time:
                //
                //   Math.max(1, 2, 3) → 3        Math.min(1, 2, 3) → 1
                //   Math.max(-5, -1)  → -1       Math.min(-5, -1)  → -5
                //
                // We fold ONLY when there is at least one argument and ALL of them
                // are numeric literals (so no ToNumber side effect, and the result
                // is a definite finite number — `Infinity`/`NaN` are GLOBAL
                // identifiers, never numeric literals, so a non-literal argument is
                // declined). We model the spec's signed-zero rule exactly: `Math.max`
                // prefers `+0` over `-0`, `Math.min` prefers `-0` over `+0` (see
                // `js_math_max` / `js_math_min`) — we do NOT rely on Rust's
                // `f64::max`/`min`, whose zero handling we don't want to depend on.
                // The empty call `Math.max()` (→ `-Infinity`) / `Math.min()` (→
                // `+Infinity`) is declined: emitting an infinite numeric literal is
                // out of scope. Same bare-global premise — only the literal
                // `Math.max(...)` callee folds, never a shadowed `m.max(...)`.
                if obj.name == "Math"
                    && matches!(prop.name.as_str(), "max" | "min")
                    && !arguments.is_empty()
                {
                    let nums: Option<Vec<f64>> = arguments
                        .iter()
                        .map(|a| match a {
                            Expression::NumericLiteral(n) => Some(n.value),
                            _ => None,
                        })
                        .collect();
                    if let Some(nums) = nums {
                        let result = if prop.name == "max" {
                            js_math_max(&nums)
                        } else {
                            js_math_min(&nums)
                        };
                        // Emit only when the result has a faithful numeric-literal
                        // spelling. Two results don't:
                        //   * an infinite result — but all-finite literal inputs
                        //     never produce one (defense-in-depth);
                        //   * NEGATIVE ZERO — `-0` has NO numeric-literal token in
                        //     JS (`-0` is UnaryMinus on `0`, and ToString(-0) is
                        //     "0"), so a bare `NumericLiteral` would print as `0`
                        //     (=== +0). `Math.min(0, -0)` is `-0`, so folding it
                        //     would flip the sign bit (observable via `1/x` or
                        //     `Object.is`). DECLINE — leaving the call intact is safe.
                        if result.is_finite() && !(result == 0.0 && result.is_sign_negative()) {
                            let parent = c.cv.clone();
                            let before =
                                format!("Math.{}({} numeric arg(s))", prop.name, nums.len());
                            let after = format_js_number(result);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::Number(result), new_cv);
                        }
                    }
                }

                // ---- Math.abs/floor/ceil/round(n) → numeric literal ----
                //
                // The single-argument numeric `Math` methods (ECMAScript
                // §21.3.2) each map one finite input to one output:
                //
                //   Math.abs(-5)    → 5        Math.floor(4.7)  → 4
                //   Math.ceil(4.2)  → 5        Math.round(2.5)  → 3
                //
                // We fold ONLY when the sole argument is a numeric literal (so no
                // ToNumber side effect) and the bare-global premise holds (a
                // literal `Math.<m>(...)` callee, never a shadowed `m.abs(...)`).
                // Extra arguments are declined (`arguments.len() == 1`): JS ignores
                // them, but keeping the fold to the exact-arity case keeps the
                // reasoning airtight.
                //
                // **Negative-zero care (mirrors Math.max/min).** `-0` has no
                // numeric-literal token, so any result that is negative zero — or
                // any zero-magnitude result from a *negative* input, where JS
                // yields `-0` (e.g. `Math.ceil(-0.4)` → -0, `Math.round(-0.4)` →
                // -0) — is DECLINED. Leaving the call intact is always safe. All
                // finite literal inputs produce finite outputs, so the
                // `is_finite` check is defense-in-depth.
                if obj.name == "Math"
                    && matches!(prop.name.as_str(), "abs" | "floor" | "ceil" | "round")
                    && arguments.len() == 1
                {
                    if let Expression::NumericLiteral(n) = &arguments[0] {
                        let x = n.value;
                        let result = match prop.name.as_str() {
                            "abs" => x.abs(),
                            "floor" => x.floor(),
                            "ceil" => x.ceil(),
                            "round" => js_math_round(x),
                            _ => unreachable!("matches! guard limits the method set"),
                        };
                        let neg_zero_result = result == 0.0
                            && (result.is_sign_negative() || x.is_sign_negative());
                        if result.is_finite() && !neg_zero_result {
                            let parent = c.cv.clone();
                            let before = format!("Math.{}({})", prop.name, x);
                            let after = format_js_number(result);
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return stamp_literal_cv(FoldedLiteral::Number(result), new_cv);
                        }
                    }
                }

                // ---- Object.fromEntries([[k, v], …]) → object literal ----
                //
                // `Object.fromEntries` (ECMAScript §20.1.2.7) is the inverse of
                // `Object.entries`: it walks an iterable of `[key, value]` pairs and
                // builds a plain object, assigning each value under `ToPropertyKey(key)`
                // via CreateDataPropertyOnObject. We fold the fully-static shape:
                //
                //   Object.fromEntries([["a", 1], ["b", 2]]) → {a: 1, b: 2}
                //   Object.fromEntries([[1, "x"]])           → {"1": "x"}   (key ToString)
                //   Object.fromEntries([["a", 1], ["a", 2]]) → {a: 2}       (last wins)
                //   Object.fromEntries([])                   → {}
                //
                // Soundness conditions — EVERY one must hold or we DECLINE and leave the
                // call untouched (declining is always safe):
                //   * exactly ONE argument, and it is an ARRAY LITERAL;
                //   * NO array holes at the outer level (every element present);
                //   * every element is itself a 2-element ARRAY LITERAL with no holes;
                //   * the pair's KEY is a STRING or NUMERIC literal — a numeric key is
                //     converted to its ECMAScript ToString (so `1` → "1"); boolean,
                //     null, identifier, and computed keys are declined (their property
                //     key is either a different string or not known statically);
                //   * the pair's VALUE is a primitive literal (string / number / boolean
                //     / null) — any non-literal value could carry side effects or an
                //     unknown runtime value, so we decline;
                //   * the key is NOT "__proto__" — `Object.fromEntries` makes an OWN
                //     property named "__proto__", but `{__proto__: …}` in an object
                //     literal is the §B.3.1 prototype setter, so folding it would
                //     change semantics (see `fold_from_entries_pairs`).
                //
                // DUPLICATE keys follow the spec exactly: a repeated key keeps the
                // POSITION of its FIRST occurrence but takes the value of its LAST
                // occurrence (CreateDataPropertyOnObject overwrites an existing key in
                // place). Each key is emitted as a bare identifier when it is a valid
                // identifier name (`{a: 1}`) and as a quoted string otherwise
                // (`{"1": "x"}`) — both encode the same own-property key; the identifier
                // form simply minifies smaller. Same bare-global-`Object` premise as the
                // other statics — only the literal `Object.fromEntries(...)` callee
                // folds, never a shadowed receiver (`o.fromEntries(...)` is left alone).
                if obj.name == "Object" && prop.name == "fromEntries" && arguments.len() == 1 {
                    if let Some(Expression::ArrayExpression(arr)) = arguments.first() {
                        if let Some(props) = fold_from_entries_pairs(&arr.elements) {
                            let parent = c.cv.clone();
                            let before =
                                format!("Object.fromEntries([{} pair(s)])", arr.elements.len());
                            let after = format!("{{{} prop(s)}}", props.len());
                            let new_cv = st.fork_cv(&parent, &before, &after);
                            return Expression::ObjectExpression(ObjectExpression {
                                cv: new_cv,
                                properties: props,
                            });
                        }
                    }
                }
            }
        }
    }

    // ---- global parseInt(string[, radix]) / parseFloat(string) ----
    //
    // `parseInt("12px")` → `12`, `parseInt("FF", 16)` → `255`,
    // `parseInt("0x1F")` → `31`, `parseInt("-7")` → `-7`,
    // `parseFloat("3.14abc")` → `3.14`, `parseFloat("1e3")` → `1000`
    // (ECMAScript §19.2.5 / §19.2.4). Both functions read the *leading* numeric
    // prefix of a string and ignore the trailing garbage, so a string LITERAL
    // argument folds to the exact numeric literal V8 produces at runtime.
    //
    // SOUNDNESS NOTE — these fold under the same "builtins are intact" premise
    // every fold in this pass already relies on, but one notch weaker. A string
    // literal's `.slice`/`.concat` can only be subverted by monkeypatching
    // `String.prototype`; `parseInt`/`parseFloat`/`Number` are *free
    // identifiers*, so a local binding (`let parseInt = …`) can additionally
    // mask them. We fold them anyway — matching Closure Compiler, which treats
    // redefining these globals as out of scope — but ONLY when the callee is the
    // bare identifier `parseInt`/`parseFloat`/`Number`, never a member access
    // (`window.parseInt`, which reaches the MemberExpression arm above and is
    // left untouched).
    //
    // We DECLINE (leave the call for the runtime) whenever the runtime result
    // is `NaN` (`parseInt("")`, an invalid/out-of-range radix) or `±Infinity`
    // (`parseFloat("Infinity")`): JavaScript has no literal token for either —
    // `NaN`/`Infinity` are themselves shadowable global identifiers — so there
    // is nothing sound to substitute. The helpers below return `None` for those
    // cases.
    if let Expression::Identifier(id) = &callee {
        if let Some(Expression::StringLiteral(s)) = arguments.first() {
            let folded = match id.name.as_str() {
                // The optional second argument is an integer-literal radix; a
                // non-literal or fractional radix can't be modelled, so the
                // whole call is left alone (we never guess the radix).
                "parseInt" if arguments.len() <= 2 => match arguments.get(1) {
                    None => fold_parse_int(&s.value, None),
                    Some(Expression::NumericLiteral(r))
                        if r.value.is_finite() && r.value.fract() == 0.0 =>
                    {
                        fold_parse_int(&s.value, Some(r.value))
                    }
                    Some(_) => None,
                },
                "parseFloat" if arguments.len() == 1 => fold_parse_float(&s.value),
                // `Number("…")` runs the FULL string→number coercion (not the
                // longest-prefix scan `parseInt`/`parseFloat` use): the whole
                // trimmed string must be a numeric literal or the result is
                // `NaN`. One string argument only — a second argument is ignored
                // by the runtime but we leave such calls alone to stay obvious.
                "Number" if arguments.len() == 1 => fold_number(&s.value),
                _ => None,
            };
            if let Some(value) = folded {
                let parent = c.cv.clone();
                let before = match arguments.get(1) {
                    Some(Expression::NumericLiteral(r)) => {
                        format!("{}(\"{}\",{})", id.name, s.value, format_js_number(r.value))
                    }
                    _ => format!("{}(\"{}\")", id.name, s.value),
                };
                let after = format_js_number(value);
                let new_cv = st.fork_cv(&parent, &before, &after);
                return stamp_literal_cv(FoldedLiteral::Number(value), new_cv);
            }
        }
    }

    // ---- global encodeURI(string) / decodeURI(string) ----
    //
    // `encodeURI("a b")` → `"a%20b"`, `encodeURI("é")` → `"%C3%A9"`,
    // `decodeURI("a%20b")` → `"a b"` (ECMAScript §19.2.6.4 / §19.2.6.2).
    // `encodeURI` is the *whole-URI* escaper: like `encodeURIComponent` it
    // percent-escapes every UTF-8 byte that is not unreserved, but it
    // ADDITIONALLY leaves the URI reserved/structural delimiters
    // (`; , / ? : @ & = + $` and `#`) intact — escaping them would corrupt an
    // already-assembled URI. So `encodeURI` escapes only the genuinely unsafe
    // bytes (space, non-ASCII, controls, and ``< > " { } | \ ^ [ ] ` ``).
    //
    // `decodeURI` is the matching inverse, but NOT a plain `%XX`→byte decode: it
    // KEEPS a `%XX` escape encoded (verbatim) whenever the byte it would decode
    // to is one of the reserved delimiters `; / ? : @ & = + $ , #`, so that the
    // reserved structure of a URI survives a round trip. That is the *only*
    // difference from `decodeURIComponent`, which decodes every escape.
    //
    // SOUNDNESS — identical to the `encodeURIComponent`/`parseInt` reasoning:
    // these are *free identifiers* a local could shadow (`let encodeURI = …`),
    // but we fold the bare identifier anyway (matching Closure Compiler) and
    // NEVER a member access (`window.encodeURI` reaches the MemberExpression arm
    // above and is left alone). A string literal's value is a Rust `&str` —
    // whole Unicode scalars — so every byte we emit is a real UTF-8 byte V8 would
    // emit too; there is no lone-surrogate input (the only `encodeURI` throw) to
    // hit. `decodeURI` DECLINES (returns the call to the runtime) on exactly the
    // two `URIError` inputs: a malformed `%XX` escape and a `%`-decoded byte run
    // that is not valid UTF-8. Declining a throw is always sound.
    //
    // ---- global encodeURIComponent(string) / decodeURIComponent(string) ----
    //
    // `encodeURIComponent("a b")` → `"a%20b"`, `encodeURIComponent("é")` →
    // `"%C3%A9"`, `decodeURIComponent("a%20b")` → `"a b"` (ECMAScript §19.2.6.5
    // / §19.2.6.3). `encodeURIComponent` percent-escapes every byte of the
    // string's UTF-8 encoding that is *not* an unreserved character
    // (`A-Z a-z 0-9` plus the marks ``-_.!~*'()``); `decodeURIComponent` is its
    // inverse, turning each `%XX` escape back into a byte and re-reading the
    // bytes as UTF-8.
    //
    // SOUNDNESS — same "builtins intact" premise as `parseInt`/`parseFloat`
    // above, and the same *free identifier* caveat: a local binding
    // (`let encodeURIComponent = …`) could mask the global. We fold anyway —
    // matching Closure Compiler, which treats redefining these globals as out of
    // scope — but ONLY for the bare identifier, never a member access
    // (`window.decodeURIComponent`, which reaches the MemberExpression arm above
    // and is left alone). A string LITERAL's value is a Rust `&str`, i.e. a
    // sequence of whole Unicode scalars, so `encodeURIComponent` can never hit
    // the lone-surrogate input on which JS would throw — every byte we emit is a
    // real UTF-8 byte of that scalar sequence, exactly what V8 encodes.
    // `decodeURIComponent` DECLINES (returns the call to the runtime) for the
    // two inputs JS would throw a `URIError` on: a malformed escape (a `%` not
    // followed by two hex digits) and a `%`-decoded byte sequence that is not
    // valid UTF-8. Declining a throw is always sound — we never fold a value in
    // where the runtime would have raised.
    if let Expression::Identifier(id) = &callee {
        if arguments.len() == 1 {
            if let Some(Expression::StringLiteral(s)) = arguments.first() {
                let folded = match id.name.as_str() {
                    "encodeURI" => Some(encode_uri(&s.value)),
                    "decodeURI" => decode_uri(&s.value),
                    "encodeURIComponent" => Some(encode_uri_component(&s.value)),
                    "decodeURIComponent" => decode_uri_component(&s.value),
                    _ => None,
                };
                if let Some(result) = folded {
                    let parent = c.cv.clone();
                    let before = format!("{}(\"{}\")", id.name, s.value);
                    let after = format!("\"{}\"", result);
                    let new_cv = st.fork_cv(&parent, &before, &after);
                    return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                }
            }
        }
    }

    // ---- global Boolean(value) on a string- or number-literal argument ----
    //
    // `Boolean(x)` is the `ToBoolean` coercion (ECMAScript §7.1.2): it maps its
    // argument to `true`/`false` by JS truthiness. For the two literal shapes we
    // can judge at compile time the answer is exact and total — no decline:
    //
    //   * a string literal → `false` only for the EMPTY string, else `true`
    //     (`Boolean("")` → `false`, `Boolean("0")` → `true` — a non-empty string
    //     is truthy even if it looks falsy);
    //   * a number literal → `false` for `0`/`-0`, else `true` (`Boolean(0)` →
    //     `false`, `Boolean(-0)` → `false` since `-0.0 == 0.0`, `Boolean(1)` →
    //     `true`). `NaN` is falsy but can't appear as a numeric LITERAL token.
    //
    // Every other argument (a boolean, `null`, an identifier, a second argument)
    // is left for the runtime. Like `parseInt`/`parseFloat`/`Number`/`String`,
    // `Boolean` is a free identifier, so we fold only the bare `Boolean(...)`
    // callee — never a member access (`window.Boolean(...)`).
    if let Expression::Identifier(id) = &callee {
        if id.name == "Boolean" && arguments.len() == 1 {
            let folded: Option<bool> = match arguments.first() {
                Some(Expression::StringLiteral(s)) => Some(!s.value.is_empty()),
                Some(Expression::NumericLiteral(n)) => Some(n.value != 0.0),
                _ => None,
            };
            if let Some(value) = folded {
                let parent = c.cv.clone();
                let before = match arguments.first() {
                    Some(Expression::NumericLiteral(n)) => {
                        format!("Boolean({})", format_js_number(n.value))
                    }
                    Some(Expression::StringLiteral(s)) => format!("Boolean(\"{}\")", s.value),
                    _ => "Boolean(?)".to_string(),
                };
                let after = if value { "!0" } else { "!1" };
                let new_cv = st.fork_cv(&parent, &before, after);
                return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
            }
        }
    }

    // ---- global String(value) on a string- or number-literal argument ----
    //
    // `String("x")` → `"x"` (identity) and `String(42)` → `"42"` — the ToString
    // coercion (ECMAScript §22.1.3.1 → §7.1.17). We fold only the two literal
    // argument shapes we can render *exactly*:
    //
    //   * a string literal — returned unchanged;
    //   * an INTEGER number literal — rendered by `fold_string_of_number`, which
    //     folds only integers (declining fractional values whose shortest-decimal
    //     tie-break could diverge from V8), so we never substitute a wrong string.
    //
    // Every other argument (a boolean, `null`, an identifier, a second argument)
    // is left for the runtime. Like `parseInt`/`parseFloat`, `String` is a free
    // identifier, so we fold only the bare `String(...)` callee — never a member
    // access (`window.String(...)`, handled by the MemberExpression arm above).
    if let Expression::Identifier(id) = &callee {
        if id.name == "String" && arguments.len() == 1 {
            let folded: Option<String> = match arguments.first() {
                Some(Expression::StringLiteral(s)) => Some(s.value.clone()),
                Some(Expression::NumericLiteral(n)) => fold_string_of_number(n.value),
                _ => None,
            };
            if let Some(result) = folded {
                let parent = c.cv.clone();
                let before = match arguments.first() {
                    Some(Expression::NumericLiteral(n)) => {
                        format!("String({})", format_js_number(n.value))
                    }
                    _ => format!("String(\"{}\")", result),
                };
                let after = format!("\"{}\"", result);
                let new_cv = st.fork_cv(&parent, &before, &after);
                return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
            }
        }
    }

    // ---- global isNaN(value) / isFinite(value) → boolean ----
    //
    // `isNaN(x)` / `isFinite(x)` (ECMAScript §19.2.3 / §19.2.2) coerce their
    // argument with `ToNumber`, then test the result: `isNaN` is `true` exactly
    // when `ToNumber(x)` is `NaN`; `isFinite` is `true` exactly when it is
    // neither `NaN` nor `±Infinity`. For the two literal argument shapes we can
    // run `ToNumber` at compile time and the answer is exact and total:
    //
    //   * a NUMBER literal — its value is already the coerced number, so
    //     `isNaN(0)` → `false`, `isFinite(0)` → `true`, and a literal that
    //     overflows to `Infinity` (`1e400`) → `isFinite` `false`;
    //   * a STRING literal — coerced by `js_to_number` (the FULL ECMAScript
    //     string→number coercion, which — unlike `parseInt`/`parseFloat` — reads
    //     the WHOLE trimmed string): `isNaN("abc")` → `true`, `isNaN("42")` →
    //     `false`, `isNaN(" ")` → `false` (`ToNumber(" ")` is `+0`),
    //     `isFinite("1e3")` → `true`, `isFinite("Infinity")` → `false`.
    //
    // Every other argument (a boolean, `null`, an identifier, a second argument)
    // is left for the runtime. Like `parseInt`/`Number`/`Boolean`, `isNaN` and
    // `isFinite` are free identifiers, so we fold only the bare callee — never a
    // member access (`window.isNaN(...)`, handled by the MemberExpression arm
    // above). Unlike `Number(...)`, no shape DECLINES: `js_to_number` returns a
    // real `f64` (`NaN`/`±Infinity`/finite) for every string, and we only ever
    // read its `is_nan()`/`is_finite()` classification — never emit the number —
    // so a value beyond the exact-integer range is still classified correctly.
    if let Expression::Identifier(id) = &callee {
        if (id.name == "isNaN" || id.name == "isFinite") && arguments.len() == 1 {
            let coerced: Option<f64> = match arguments.first() {
                Some(Expression::NumericLiteral(n)) => Some(n.value),
                Some(Expression::StringLiteral(s)) => Some(js_to_number(&s.value)),
                _ => None,
            };
            if let Some(v) = coerced {
                let value = if id.name == "isNaN" {
                    v.is_nan()
                } else {
                    v.is_finite()
                };
                let parent = c.cv.clone();
                let before = match arguments.first() {
                    Some(Expression::NumericLiteral(n)) => {
                        format!("{}({})", id.name, format_js_number(n.value))
                    }
                    Some(Expression::StringLiteral(s)) => {
                        format!("{}(\"{}\")", id.name, s.value)
                    }
                    _ => format!("{}(?)", id.name),
                };
                let after = if value { "!0" } else { "!1" };
                let new_cv = st.fork_cv(&parent, &before, after);
                return stamp_literal_cv(FoldedLiteral::Boolean(value), new_cv);
            }
        }
    }

    // ---- global escape(string) / unescape(string) ----
    //
    // The legacy Annex B escapers (ECMAScript §B.2.1.1 / §B.2.1.2). `escape`
    // percent-encodes each UTF-16 CODE UNIT that is not in its small unescaped
    // set (`A-Z a-z 0-9` plus the seven marks ``@ * _ + - . /``): a unit below
    // `0x100` becomes `%XX` (two UPPERCASE hex digits), a unit `0x100` and above
    // becomes `%uXXXX` (four). So `escape("a b")` → `"a%20b"`, `escape("é")` →
    // `"%E9"` (U+00E9 is one code unit < 0x100), and `escape("😀")` →
    // `"%uD83D%uDE00"` (one astral scalar is two surrogate code units).
    // `unescape` is the inverse: `%uXXXX` → that code unit, `%XX` → that code
    // unit, and any `%` that does NOT begin a complete escape (a lone `%`, a
    // non-hex digit, a truncated tail) passes through LITERALLY. Neither throws.
    //
    // Both operate on UTF-16 CODE UNITS, NOT UTF-8 bytes — which is why we
    // iterate `.encode_utf16()` rather than `.as_bytes()`. (Escaping the UTF-8
    // bytes of `é`/`😀` is what `encodeURIComponent` does, not `escape`.)
    //
    // SOUNDNESS — same *free identifier* rule as `parseInt`/`String`: a local
    // binding could shadow the global, but we fold the bare identifier only
    // (matching Closure Compiler), never a member access (`window.escape`, which
    // reaches the MemberExpression arm above). A string literal's value is a Rust
    // `&str` (whole Unicode scalars), so `escape` renders exactly what V8 does.
    // `unescape` DECLINES (returns the call to the runtime) only when its result
    // would contain an UNPAIRED surrogate (e.g. `unescape("%uD83D")`): such a
    // value has no Rust-`String` / string-literal representation, so we leave the
    // call rather than substitute a lossy one. `unescape` never throws, so
    // declining is always sound.
    if let Expression::Identifier(id) = &callee {
        if arguments.len() == 1 {
            if let Some(Expression::StringLiteral(s)) = arguments.first() {
                let folded = match id.name.as_str() {
                    "escape" => Some(escape_js(&s.value)),
                    "unescape" => unescape_js(&s.value),
                    _ => None,
                };
                if let Some(result) = folded {
                    let parent = c.cv.clone();
                    let before = format!("{}(\"{}\")", id.name, s.value);
                    let after = format!("\"{}\"", result);
                    let new_cv = st.fork_cv(&parent, &before, &after);
                    return stamp_literal_cv(FoldedLiteral::String(result), new_cv);
                }
            }
        }
    }

    Expression::CallExpression(CallExpression {
        cv: c.cv.clone(),
        callee: Box::new(callee),
        arguments,
    })
}

/// Percent-encode `s` exactly as JavaScript's legacy global `escape`
/// (ECMAScript §B.2.1.1). Iterating over UTF-16 CODE UNITS, each unit is emitted
/// verbatim when it is in the unescaped set — the ASCII alphanumerics plus the
/// seven marks ``@ * _ + - . /`` — and percent-escaped otherwise: a unit below
/// `0x100` as `%XX` (two UPPERCASE hex digits), a unit `0x100` and above as
/// `%uXXXX` (four). `escape` never throws.
///
/// Operating on code units — not the UTF-8 bytes `encodeURIComponent` uses — is
/// the whole distinction of the legacy escaper:
///
/// | input  | code units    | output            |
/// |--------|---------------|-------------------|
/// | `"a b"`| `61 20 62`    | `"a%20b"`         |
/// | `"é"`  | `00E9`        | `"%E9"`           |
/// | `"😀"` | `D83D DE00`   | `"%uD83D%uDE00"`  |
/// | `"~"`  | `7E`          | `"%7E"` (not kept)|
/// | `"/"`  | `2F` (a mark) | `"/"`             |
///
/// A string literal's value is a Rust `&str`, so `s.encode_utf16()` yields
/// exactly the UTF-16 unit sequence V8 escapes — byte-for-byte identical output.
fn escape_js(s: &str) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for u in s.encode_utf16() {
        // The unescaped set is entirely ASCII, so only a unit < 0x80 can match.
        if u < 0x80
            && {
                let b = u as u8;
                b.is_ascii_alphanumeric()
                    || matches!(b, b'@' | b'*' | b'_' | b'+' | b'-' | b'.' | b'/')
            }
        {
            out.push(u as u8 as char);
        } else if u < 0x100 {
            out.push('%');
            out.push(HEX[(u >> 4) as usize] as char);
            out.push(HEX[(u & 0xf) as usize] as char);
        } else {
            out.push('%');
            out.push('u');
            out.push(HEX[((u >> 12) & 0xf) as usize] as char);
            out.push(HEX[((u >> 8) & 0xf) as usize] as char);
            out.push(HEX[((u >> 4) & 0xf) as usize] as char);
            out.push(HEX[(u & 0xf) as usize] as char);
        }
    }
    out
}

/// Percent-encode `s` exactly as JavaScript's global `encodeURI` (ECMAScript
/// §19.2.6.4). Every byte of the string's UTF-8 encoding is emitted verbatim
/// when it is part of the **unescaped** set and as `%XX` — two UPPERCASE hex
/// digits — otherwise. `encodeURI` is the *whole-URI* escaper: its unescaped set
/// is the unreserved characters (ASCII alphanumerics plus the marks
/// ``- _ . ! ~ * ' ( )``) PLUS the URI reserved/structural delimiters
/// `; , / ? : @ & = + $` and `#`. That wider keep-set — leaving the reserved
/// punctuation intact — is exactly what distinguishes it from
/// `encodeURIComponent`, which escapes those delimiters too.
///
/// | input      | bytes (UTF-8) | output       | note                       |
/// |------------|---------------|--------------|----------------------------|
/// | `"a b"`    | `61 20 62`    | `"a%20b"`    | space → `%20`              |
/// | `"a/b?c"`  | …             | `"a/b?c"`    | reserved kept intact       |
/// | `"é"`      | `C3 A9`       | `"%C3%A9"`   | non-ASCII escaped per byte  |
/// | `"a<b>"`   | …             | `"a%3Cb%3E"` | `< >` are unsafe → escaped  |
///
/// This coincides bit-for-bit with V8 for any string literal: a literal's value
/// is a Rust `&str`, i.e. a run of whole Unicode scalars, so `s.as_bytes()` is
/// precisely the UTF-8 byte sequence V8 percent-encodes — there is no lone
/// surrogate (the only `encodeURI` input that throws) to worry about.
fn encode_uri(s: &str) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(
                b,
                // unreserved marks (the `encodeURIComponent` keep-set) …
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
                // … PLUS the URI reserved/structural delimiters `encodeURI` keeps.
                | b';' | b',' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b'#'
            )
        {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
    }
    out
}

/// Percent-encode `s` exactly as JavaScript's global `encodeURIComponent`
/// (ECMAScript §19.2.6.5). Every byte of the string's UTF-8 encoding is emitted
/// verbatim when it is an **unreserved** character and as `%XX` — two UPPERCASE
/// hex digits — otherwise. The unreserved set is the ASCII alphanumerics plus
/// the nine marks ``- _ . ! ~ * ' ( )``; note that the URI *reserved* delimiters
/// (`; , / ? : @ & = + $`), which `encodeURI` leaves intact, ARE escaped here —
/// that asymmetry is the whole point of the `…Component` variant.
///
/// | input  | bytes (UTF-8)   | output     |
/// |--------|-----------------|------------|
/// | `"a b"`| `61 20 62`      | `"a%20b"`  |
/// | `"é"`  | `C3 A9`         | `"%C3%A9"` |
/// | `"/"`  | `2F`            | `"%2F"`    |
/// | `"~"`  | `7E` (mark)     | `"~"`      |
///
/// This coincides bit-for-bit with V8 for any string literal: a literal's value
/// is a Rust `&str`, i.e. a run of whole Unicode scalars, so `s.as_bytes()` is
/// precisely the UTF-8 byte sequence V8 percent-encodes — there is no lone
/// surrogate (the only `encodeURIComponent` input that throws) to worry about.
fn encode_uri_component(s: &str) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
    }
    out
}

/// Decode `s` exactly as JavaScript's legacy global `unescape` (ECMAScript
/// §B.2.1.2), the inverse of [`escape_js`]. Scanning UTF-16 code units: `%uXXXX`
/// (a `%u` followed by four hex digits) yields that one code unit, `%XX` (a `%`
/// followed by two hex digits) yields that code unit, and any `%` that does NOT
/// begin a complete escape — a lone `%`, a non-hex digit, a truncated tail —
/// passes through verbatim. `unescape` itself never throws.
///
/// Returns `None` — DECLINING the fold — only when the decoded units would form
/// an UNPAIRED surrogate (`unescape("%uD83D")`): that value cannot be held in a
/// Rust `String` / a string literal, so we leave the call for the runtime rather
/// than substitute a lossy replacement. Declining a non-throwing call is sound.
///
/// | input            | result            | note                          |
/// |------------------|-------------------|-------------------------------|
/// | `"a%20b"`        | `Some("a b")`     | round-trips an escape         |
/// | `"%E9"`          | `Some("é")`       | one `%XX` code unit           |
/// | `"%uD83D%uDE00"` | `Some("😀")`      | surrogate pair reassembled    |
/// | `"%2F"`          | `Some("/")`       | every escape decodes          |
/// | `"%"`            | `Some("%")`       | lone `%` passes through        |
/// | `"%uD83D"`       | `None`            | unpaired surrogate → decline  |
fn unescape_js(s: &str) -> Option<String> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let n = units.len();
    let mut out: Vec<u16> = Vec::with_capacity(n);
    // A unit is only a hex digit when it is ASCII, so casting through `char` to
    // reuse `to_digit(16)` is exact (and yields `None` for any non-hex unit).
    let hexval = |c: u16| -> Option<u16> { char::from_u32(c as u32)?.to_digit(16).map(|d| d as u16) };
    let mut i = 0;
    while i < n {
        if units[i] == b'%' as u16 {
            // `%uXXXX` — the `u` plus four hex digits (needs five units ahead).
            if i + 5 < n && units[i + 1] == b'u' as u16 {
                if let (Some(a), Some(b), Some(c), Some(d)) = (
                    hexval(units[i + 2]),
                    hexval(units[i + 3]),
                    hexval(units[i + 4]),
                    hexval(units[i + 5]),
                ) {
                    out.push((a << 12) | (b << 8) | (c << 4) | d);
                    i += 6;
                    continue;
                }
            }
            // `%XX` — two hex digits (needs two units ahead).
            if i + 2 < n {
                if let (Some(a), Some(b)) = (hexval(units[i + 1]), hexval(units[i + 2])) {
                    out.push((a << 4) | b);
                    i += 3;
                    continue;
                }
            }
            // A `%` that starts no complete escape is a literal `%`.
            out.push(units[i]);
            i += 1;
        } else {
            out.push(units[i]);
            i += 1;
        }
    }
    String::from_utf16(&out).ok()
}

/// Decode `s` exactly as JavaScript's global `decodeURI` (ECMAScript §19.2.6.2),
/// the inverse of [`encode_uri`]. Each `%XX` escape (two hex digits) decodes to
/// one byte, EXCEPT that an escape whose byte is one of the reserved delimiters
/// `; / ? : @ & = + $ , #` is left **encoded** — its original three characters
/// pass through verbatim — so the reserved structure of a URI survives. Every
/// other character passes through as its own UTF-8 byte(s); the collected bytes
/// are then re-interpreted as UTF-8. Returns `None` — DECLINING the fold — for
/// exactly the inputs on which JS throws a `URIError`: a malformed escape (a `%`
/// not followed by two hex digits) and a decoded byte run that is not valid
/// UTF-8.
///
/// | input      | result          | note                                       |
/// |------------|-----------------|--------------------------------------------|
/// | `"a%20b"`  | `Some("a b")`   | `%20`→space (not reserved) → decoded       |
/// | `"%2F"`    | `Some("%2F")`   | `/` IS reserved → kept encoded (vs Comp.)  |
/// | `"%C3%A9"` | `Some("é")`     | two non-reserved bytes → one scalar        |
/// | `"%41"`    | `Some("A")`     | `A` not reserved → decoded                 |
/// | `"%"`      | `None`          | truncated escape → URIError                |
/// | `"%80"`    | `None`          | lone continuation byte: bad UTF-8          |
///
/// The reserved bytes are all ASCII (`< 0x80`), so they can never coincide with
/// a UTF-8 lead or continuation byte (`>= 0x80`); preserving them therefore
/// never disturbs a multi-byte scalar, which always decodes (its code point is
/// non-ASCII, hence never reserved). Decoding the whole buffer once and
/// validating is equivalent to the spec's per-`%XX`-run reading because UTF-8 is
/// self-synchronizing — JS succeeds and we succeed on the same inputs, and JS
/// throws (we decline) on the same ones.
fn decode_uri(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // A `%` must be followed by two hex digits; otherwise JS throws.
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            let byte = (hi * 16 + lo) as u8;
            // `decodeURI` keeps reserved-delimiter escapes ENCODED: emit the
            // original three characters (`%` + both hex digits) verbatim.
            if matches!(
                byte,
                b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b',' | b'#'
            ) {
                out.push(bytes[i]);
                out.push(bytes[i + 1]);
                out.push(bytes[i + 2]);
            } else {
                out.push(byte);
            }
            i += 3;
        } else {
            // `s` is valid UTF-8, so a non-`%` byte is part of a complete scalar
            // we copy through verbatim.
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Decode `s` exactly as JavaScript's global `decodeURIComponent` (ECMAScript
/// §19.2.6.3), the inverse of [`encode_uri_component`]. Each `%XX` escape (two
/// hex digits) contributes one byte; any other character passes through as its
/// own UTF-8 byte(s); the collected bytes are then re-interpreted as UTF-8.
/// Returns `None` — DECLINING the fold — for exactly the inputs on which JS
/// throws a `URIError`: a malformed escape (a `%` not followed by two hex
/// digits, including a `%` in the final one or two positions) and a decoded
/// byte run that is not valid UTF-8.
///
/// | input        | result        | note                              |
/// |--------------|---------------|-----------------------------------|
/// | `"a%20b"`    | `Some("a b")` | round-trips an encode             |
/// | `"%C3%A9"`   | `Some("é")`   | two bytes reassembled into one é  |
/// | `"%"`        | `None`        | truncated escape → URIError       |
/// | `"%G0"`      | `None`        | non-hex digit → URIError          |
/// | `"%80"`      | `None`        | lone continuation byte: bad UTF-8 |
///
/// Decoding the whole byte buffer once (rather than per `%XX` run, as the spec
/// phrases it) is equivalent because a string literal's pass-through characters
/// are already complete scalars: UTF-8 is self-synchronizing, so concatenating
/// complete-scalar regions with `%`-decoded byte runs and validating the result
/// yields the same string when JS succeeds and an error precisely when JS
/// throws. Declining a throw is always sound.
fn decode_uri_component(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // A `%` must be followed by two hex digits; otherwise JS throws.
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            // `s` is valid UTF-8, so a non-`%` byte is part of a complete scalar
            // we copy through verbatim.
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Render a NUMBER literal the way JavaScript's `String(n)` / `Number.prototype
/// .toString()` would (ECMAScript §6.1.6.1.20 `Number::toString`), or `None`
/// when we can't guarantee a byte-identical result.
///
/// `n` is always finite here (a `NumericLiteral` can never be `NaN`/`Infinity`
/// — those are global identifiers, not literal tokens).
///
/// We fold ONLY integer-valued numbers in the exact-`i64` range. We deliberately
/// do **not** fold fractional values: Rust's `f64::to_string` and V8's
/// `Number::toString` are *both* shortest-round-trip, but on a value that sits
/// exactly halfway between two equally-short decimals they can break the tie in
/// OPPOSITE directions — a silent last-digit-off-by-one (e.g.
/// `String(108868734838530.12)` would mis-fold to `"...530.13"`). Reproducing
/// V8's tie-breaking would mean implementing the full spec `Number::toString`,
/// so instead we decline every fractional argument (the call is left for the
/// runtime — always sound). An integer, by contrast, has a *unique* decimal
/// spelling, so the `i64` path is byte-identical to V8.
///
/// | call             | value    | branch                              |
/// |------------------|----------|-------------------------------------|
/// | `String(0)`      | `"0"`    | zero (covers `-0` too)              |
/// | `String(42)`     | `"42"`   | exact integer via `i64`             |
/// | `String(-3)`     | `"-3"`   | exact integer                       |
/// | `String(0.5)`    | decline  | fractional → tie-break divergence   |
/// | `String(3.14)`   | decline  | fractional → tie-break divergence   |
/// | `String(1e21)`   | decline  | ≥ 2^53 (and V8 exponential anyway)  |
fn fold_string_of_number(n: f64) -> Option<String> {
    // Both `+0` and `-0` stringify to `"0"` (and `-0.0 == 0.0` in Rust).
    if n == 0.0 {
        return Some("0".to_string());
    }
    // Integer-valued and inside `i64`'s exact range: `< 2^53` keeps every integer
    // both exactly f64-representable AND safely inside `i64` (so the `as` cast
    // can't saturate). Fractional values, and integers ≥ 2^53, are declined.
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        return Some(format!("{}", n as i64));
    }
    None
}

/// Evaluate a single-argument `String.prototype` substring **predicate** —
/// `startsWith`, `endsWith`, or `includes` — over two constant strings,
/// returning the boolean answer, or `None` when `method` is not one we model
/// (so the caller leaves the call untouched).
///
/// | JS call                  | result  | Rust intrinsic    |
/// |--------------------------|---------|-------------------|
/// | `"abc".startsWith("a")`  | `true`  | `str::starts_with`|
/// | `"abc".endsWith("c")`    | `true`  | `str::ends_with`  |
/// | `"abc".includes("b")`    | `true`  | `str::contains`   |
/// | `"abc".includes("x")`    | `false` | `str::contains`   |
///
/// These coincide bit-for-bit with V8 for any pair of literals: JS matches by
/// UTF-16 code unit and Rust by UTF-8 byte, but both operands are valid
/// `String`s (whole Unicode scalars, no lone surrogates), and a prefix /
/// suffix / substring relation over a scalar sequence holds identically in
/// every self-synchronizing encoding. The empty needle is always present, just
/// as `str` reports (`"".starts_with("")` and `"abc".contains("")` are both
/// `true`).
fn fold_string_predicate(method: &str, haystack: &str, needle: &str) -> Option<bool> {
    match method {
        "startsWith" => Some(haystack.starts_with(needle)),
        "endsWith" => Some(haystack.ends_with(needle)),
        "includes" => Some(haystack.contains(needle)),
        _ => None,
    }
}

/// Render a non-negative integer `v` in `radix` (2..=36) the way JavaScript's
/// `Number.prototype.toString(radix)` does: lowercase digits `0-9a-z`, no
/// leading zeros, and `"0"` for zero. This is the inverse of parsing a
/// base-`radix` integer literal. `radix` is guaranteed in range by the caller,
/// so the digit lookup never goes out of bounds and the `from_utf8` of an
/// all-ASCII buffer never fails.
fn to_radix_string(mut v: u64, radix: u32) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if v == 0 {
        return "0".to_string();
    }
    let radix = radix as u64;
    let mut out = Vec::new();
    while v > 0 {
        out.push(DIGITS[(v % radix) as usize]);
        v /= radix;
    }
    out.reverse();
    String::from_utf8(out).expect("radix digits are ASCII")
}

/// Compute `value.slice(args…)` at compile time, or `None` when it cannot be
/// folded soundly (ECMAScript §22.1.3.22, `String.prototype.slice`).
///
/// `slice` works in **UTF-16 code units**, so we index into `encode_utf16()`
/// (an astral char is two units). The algorithm, matching the spec:
///
/// 1. `start` (default `0`) and `end` (default the length) are each clamped:
///    a negative index counts from the end (`len + idx`, floored at `0`); a
///    non-negative one is capped at `len`.
/// 2. the result is the half-open range `[start, max(start, end))` — empty when
///    `start >= end`.
///
/// We decline (return `None`, leaving the call for the runtime) when:
/// - there are more than two arguments;
/// - any *given* argument is not a finite integer literal (we don't model
///   `ToInteger` coercion of arbitrary values); or
/// - the cut would split a surrogate pair, yielding a lone surrogate that a
///   Rust `String` cannot hold (`String::from_utf16` fails) — the same
///   conservative guard `charAt` uses.
fn fold_string_slice(value: &str, args: &[Expression]) -> Option<String> {
    if args.len() > 2 {
        return None;
    }
    // A provided argument must be a finite integer literal (any sign).
    let to_int = |e: &Expression| -> Option<i64> {
        match e {
            Expression::NumericLiteral(n)
                if n.value.is_finite()
                    && n.value.fract() == 0.0
                    && n.value.abs() < 9_007_199_254_740_992.0 =>
            {
                Some(n.value as i64)
            }
            _ => None,
        }
    };

    let units: Vec<u16> = value.encode_utf16().collect();
    let len = units.len() as i64;
    let clamp = |idx: i64| -> i64 {
        if idx < 0 {
            (len + idx).max(0)
        } else {
            idx.min(len)
        }
    };

    let start = match args.first() {
        None => 0,
        Some(e) => clamp(to_int(e)?),
    };
    let end = match args.get(1) {
        None => len,
        Some(e) => clamp(to_int(e)?),
    };

    let lo = start as usize;
    let hi = end.max(start) as usize; // empty range when end < start
    if lo >= hi {
        return Some(String::new());
    }
    // A lone surrogate (split pair) can't be a Rust String — decline.
    String::from_utf16(&units[lo..hi]).ok()
}

/// Fold the static `String.fromCharCode(u0, u1, …)` — build a string from
/// UTF-16 code units (ECMAScript §22.1.2.1).
///
/// Each argument is one UTF-16 code unit, so `String.fromCharCode(72, 73)` →
/// `"HI"` and an adjacent high+low surrogate pair assembles an astral scalar
/// (`String.fromCharCode(0xD83D, 0xDCA9)` → `"💩"`). No arguments → `""`.
///
/// Conservative scope: every argument must be a non-negative integer literal
/// that already fits in 16 bits (`0..=0xFFFF`). JS coerces each via `ToUint16`
/// (mod 2^16), but we decline (returning `None`, leaving the call) for a
/// fractional, negative, out-of-range, or non-literal argument rather than
/// model that wrap-around. We also return `None` when the assembled units are
/// not valid UTF-16 — a LONE surrogate is a legal JS string but cannot be a
/// Rust `String` (`String::from_utf16` fails), the same guard `slice`/`charAt`/
/// `codePointAt` use.
fn fold_string_from_char_code(args: &[Expression]) -> Option<String> {
    let mut units: Vec<u16> = Vec::with_capacity(args.len());
    for a in args {
        match a {
            Expression::NumericLiteral(n)
                if n.value.is_finite()
                    && n.value.fract() == 0.0
                    && n.value >= 0.0
                    && n.value <= 0xFFFF as f64 =>
            {
                units.push(n.value as u16);
            }
            _ => return None,
        }
    }
    // A lone surrogate among the units can't be a Rust String — decline.
    String::from_utf16(&units).ok()
}

/// Fold the static `String.fromCodePoint(cp0, cp1, …)` — build a string from
/// Unicode CODE POINTS (ECMAScript §22.1.2.2).
///
/// Unlike `fromCharCode` (whose arguments are 16-bit UTF-16 *units*), each
/// argument here is a whole code point, so one astral argument is enough:
/// `String.fromCodePoint(128169)` → `"💩"` (U+1F4A9),
/// `String.fromCodePoint(72, 73)` → `"HI"`, no arguments → `""`.
///
/// Conservative scope: every argument must be a non-negative integer literal
/// that is a VALID Unicode scalar — in `0..=0x10FFFF` and NOT a surrogate
/// (`0xD800..=0xDFFF`). In JS an out-of-range / fractional argument throws a
/// `RangeError`, and a surrogate code point yields a lone-surrogate string a
/// Rust `String` cannot hold; `char::from_u32` returns `None` for exactly those
/// inputs, so we return `None` (leaving the call for the runtime) rather than
/// emit a wrong literal or one for a call JS would have thrown on. A
/// fractional, negative, `>0x10FFFF`, or non-literal argument also declines.
fn fold_string_from_code_point(args: &[Expression]) -> Option<String> {
    let mut result = String::new();
    for a in args {
        match a {
            Expression::NumericLiteral(n)
                if n.value.is_finite()
                    && n.value.fract() == 0.0
                    && n.value >= 0.0
                    && n.value <= 0x10FFFF as f64 =>
            {
                // `char::from_u32` rejects surrogates (D800..DFFF) and anything
                // past U+10FFFF, exactly the code points that can't be a Rust
                // `char` — decline (`?`) for those.
                result.push(char::from_u32(n.value as u32)?);
            }
            _ => return None,
        }
    }
    Some(result)
}

/// Fold `"…".substring(start[, end])` (ECMAScript §22.1.3.24).
///
/// `substring` differs from `slice` in two ways, both modelled here:
///
///   1. **Clamping.** Each index is clamped into `[0, len]`. A negative (or
///      `NaN`) argument becomes `0` — it never counts from the end the way
///      `slice` does. So `"abcd".substring(-2)` is the whole string, whereas
///      `"abcd".slice(-2)` is `"cd"`.
///   2. **Ordering.** After clamping, the smaller index is the start: when
///      `start > end` the two endpoints SWAP. So `"abcd".substring(3, 1)` and
///      `"abcd".substring(1, 3)` both yield `"bc"`.
///
/// Indices are UTF-16 code units (matching `slice`/`charAt`). Returns `None`
/// (leaving the call for the runtime) for a non-integer-literal argument, more
/// than two arguments, or a cut that would split a surrogate pair into a lone
/// surrogate (a valid JS string but not a Rust `String`).
fn fold_string_substring(value: &str, args: &[Expression]) -> Option<String> {
    if args.len() > 2 {
        return None;
    }
    // A provided argument must be a finite integer literal.
    let to_int = |e: &Expression| -> Option<i64> {
        match e {
            Expression::NumericLiteral(n)
                if n.value.is_finite()
                    && n.value.fract() == 0.0
                    && n.value.abs() < 9_007_199_254_740_992.0 =>
            {
                Some(n.value as i64)
            }
            _ => None,
        }
    };

    let units: Vec<u16> = value.encode_utf16().collect();
    let len = units.len() as i64;
    // Clamp into [0, len]: negatives become 0 (no end-relative indexing).
    let clamp = |idx: i64| -> i64 { idx.clamp(0, len) };

    let start = match args.first() {
        None => 0,
        Some(e) => clamp(to_int(e)?),
    };
    let end = match args.get(1) {
        None => len,
        Some(e) => clamp(to_int(e)?),
    };

    // Swap so the cut is always the range [lo, hi).
    let lo = start.min(end) as usize;
    let hi = start.max(end) as usize;
    if lo >= hi {
        return Some(String::new());
    }
    // A lone surrogate (split pair) can't be a Rust String — decline.
    String::from_utf16(&units[lo..hi]).ok()
}

/// Fold `"…".substr(start[, length])` — the legacy length-counted slice
/// (ECMAScript Annex B §B.2.3.1, `String.prototype.substr`).
///
/// Unlike `slice`/`substring`, `substr`'s second argument is a **length**, not
/// an end index:
///
///   1. **Start.** A negative `start` counts from the end and then clamps to 0
///      (`"abcde".substr(-2)` begins at index 3); a non-negative `start` clamps
///      to `len`.
///   2. **Length.** When omitted, it defaults to "the rest of the string". The
///      requested length is clamped into `[0, len - start]`, so it can never
///      read past the end. A length `<= 0` yields `""`.
///
/// So `"abcde".substr(1, 2)` → `"bc"`, `"abcde".substr(1)` → `"bcde"`,
/// `"abcde".substr(-2, 1)` → `"d"`, `"abcde".substr(10)` → `""`. Indices are
/// UTF-16 code units (matching `slice`/`charAt`). Returns `None` (leaving the
/// call for the runtime) for a non-integer-literal argument, more than two
/// arguments, or a cut that would split a surrogate pair into a lone surrogate.
fn fold_string_substr(value: &str, args: &[Expression]) -> Option<String> {
    if args.len() > 2 {
        return None;
    }
    // A provided argument must be a finite integer literal (any sign).
    let to_int = |e: &Expression| -> Option<i64> {
        match e {
            Expression::NumericLiteral(n)
                if n.value.is_finite()
                    && n.value.fract() == 0.0
                    && n.value.abs() < 9_007_199_254_740_992.0 =>
            {
                Some(n.value as i64)
            }
            _ => None,
        }
    };

    let units: Vec<u16> = value.encode_utf16().collect();
    let len = units.len() as i64;

    // start: a negative counts from the end (then clamps to 0); otherwise it
    // clamps up to `len`.
    let int_start = match args.first() {
        None => 0,
        Some(e) => to_int(e)?,
    };
    let start = if int_start < 0 {
        (len + int_start).max(0)
    } else {
        int_start.min(len)
    };

    // length: defaults to "the rest"; the actual count is clamped into
    // [0, len - start] so it can never read past the end.
    let int_length = match args.get(1) {
        None => len, // any value >= len - start works; len is a safe ceiling
        Some(e) => to_int(e)?,
    };
    let result_len = int_length.clamp(0, len - start);
    if result_len <= 0 {
        return Some(String::new());
    }

    let lo = start as usize;
    let hi = (start + result_len) as usize;
    // A lone surrogate (split pair) can't be a Rust String — decline.
    String::from_utf16(&units[lo..hi]).ok()
}

/// Compute `value.repeat(count)` at compile time, or `None` when it cannot be
/// folded soundly (ECMAScript §22.1.3.18, `String.prototype.repeat`).
///
/// `repeat` concatenates the whole receiver `count` times, so — unlike `slice`
/// — it never splits a surrogate pair: the UTF-8 string is simply duplicated.
/// We require exactly one **non-negative integer literal** argument:
/// `"ab".repeat(3)` → `"ababab"`, `"x".repeat(0)` → `""`.
///
/// We decline (return `None`, leaving the call for the runtime) when:
/// - the argument is missing, fractional, non-finite, or not a numeric literal
///   (we don't model `ToInteger` coercion of arbitrary values);
/// - the count is negative — JS throws a `RangeError`, which we must not erase
///   by folding to a value; or
/// - the materialized result would exceed `MAX_REPEAT_UNITS` UTF-16 code units.
///   `"x".repeat(1e9)` is a valid program, but expanding it at compile time
///   would balloon the output (and the optimizer's memory) — an
///   algorithmic-blowup / DoS guard. `checked_mul` also stops the length
///   computation itself from overflowing.
fn fold_string_repeat(value: &str, args: &[Expression]) -> Option<String> {
    /// Cap on the folded result's length, in UTF-16 code units. Past this we
    /// leave `repeat` for the runtime rather than materialize a huge literal.
    const MAX_REPEAT_UNITS: u64 = 100_000;

    let n = match args {
        [Expression::NumericLiteral(n)] => n,
        _ => return None,
    };
    if !(n.value.is_finite() && n.value.fract() == 0.0 && n.value >= 0.0) {
        return None;
    }
    let count = n.value as u64;
    let unit_len = value.encode_utf16().count() as u64;
    // Decline on overflow (None from checked_mul) or when over the cap.
    match unit_len.checked_mul(count) {
        Some(total) if total <= MAX_REPEAT_UNITS => Some(value.repeat(count as usize)),
        _ => None,
    }
}

/// Compute `value.concat(args…)` at compile time, or `None` when it cannot be
/// folded soundly (ECMAScript §22.1.3.4, `String.prototype.concat`).
///
/// `concat` appends each argument, coerced to a string, to the receiver:
/// `"a".concat("b", "c")` → `"abc"`. We fold only the case where **every**
/// argument is already a string literal, so the result is a pure textual
/// join — no `ToString` coercion to model, and (because each piece is a valid
/// string) the join is always valid UTF-16, so it can never produce a lone
/// surrogate the way a `slice` cut can. A zero-argument call (`"a".concat()`)
/// folds to the receiver unchanged.
///
/// We decline (return `None`, leaving the call for the runtime) when:
/// - any argument is not a string literal (e.g. `"a".concat(x)` or
///   `"a".concat(1)`); or
/// - the joined length would exceed `MAX_CONCAT_UNITS` UTF-16 code units. The
///   pieces all come from the source, so this is a defensive cap (and
///   `checked_add` stops the running length from overflowing) rather than a
///   true blowup vector, but it mirrors the `repeat`/`pad` guards.
///
/// Coerce a single `String.prototype.concat` argument to the string JS would
/// pass to the concatenation, or `None` if it is not a compile-time constant we
/// can coerce faithfully.
///
/// `concat` runs `ToString` on every argument (ECMAScript §22.1.3.4), which is
/// where this differs from `Array.prototype.join`: `join` maps `null`/
/// `undefined` to the empty string, but `ToString(null)` is `"null"` and
/// `ToString(undefined)` is `"undefined"`.
///
/// ```text
///   argument        concat string
///   -------------   -------------
///   "abc"           abc
///   42              42          (String(Number))
///   true / false    true / false
///   null            null
///   undefined       undefined
///   anything else   None  → decline the whole fold
/// ```
fn concat_arg_str(arg: &Expression) -> Option<String> {
    match arg {
        Expression::StringLiteral(s) => Some(s.value.clone()),
        Expression::NumericLiteral(n) => Some(format_js_number(n.value)),
        Expression::BooleanLiteral(b) => Some(if b.value { "true" } else { "false" }.to_string()),
        Expression::NullLiteral(_) => Some("null".to_string()),
        Expression::UndefinedLiteral(_) => Some("undefined".to_string()),
        // Objects, arrays, identifiers, calls, … have runtime-dependent string
        // forms — decline so the call stands.
        _ => None,
    }
}

fn fold_string_concat_call(value: &str, args: &[Expression]) -> Option<String> {
    /// Cap on the folded result's length, in UTF-16 code units.
    const MAX_CONCAT_UNITS: usize = 100_000;

    let mut units = value.encode_utf16().count();
    let mut out = String::from(value);
    for a in args {
        let piece = concat_arg_str(a)?;
        units = units.checked_add(piece.encode_utf16().count())?;
        if units > MAX_CONCAT_UNITS {
            return None;
        }
        out.push_str(&piece);
    }
    Some(out)
}

/// Compute `receiver.split(separator[, limit])` at compile time, returning the
/// pieces, or `None` to decline (leaving the call for the runtime).
///
/// `split` turns one string into an *array* of strings (ECMAScript §22.1.3.23).
/// We model the three shapes the language defines, and only those:
///
/// | call                       | result                | rule                          |
/// |----------------------------|-----------------------|-------------------------------|
/// | `"a,b,c".split(",")`       | `["a","b","c"]`       | split at each occurrence      |
/// | `"axbxc".split("x")`       | `["a","b","c"]`       | of the (non-empty) separator  |
/// | `"abc".split("x")`         | `["abc"]`             | no occurrence → whole string  |
/// | `"".split(",")`            | `[""]`                | empty receiver, found nothing |
/// | `"abc".split("")`          | `["a","b","c"]`       | empty sep → one piece per     |
/// | `"".split("")`             | `[]`                  | UTF-16 code unit              |
/// | `"abc".split()`            | `["abc"]`             | no separator → whole string   |
/// | `"a,b,c".split(",", 2)`    | `["a","b"]`           | LIMIT caps the piece count    |
/// | `"a,b,c".split(",", 0)`    | `[]`                  | limit 0 → empty array         |
///
/// **Why a non-empty separator is always safe.** When the separator is a
/// non-empty string we delegate to Rust's `str::split`, which matches whole
/// code points on UTF-8 boundaries. JS matches UTF-16 code units, but because
/// our separator is itself a valid Rust `String` (it can hold no lone
/// surrogate), every match in both encodings lands on a code-point boundary —
/// so the two agree piece for piece. Each piece is a substring of the receiver
/// and therefore always a representable literal.
///
/// **Why the empty separator needs a guard.** `split("")` cuts *between every
/// UTF-16 code unit*. For a character outside the Basic Multilingual Plane —
/// e.g. `"💩"`, encoded as the surrogate pair `D83D DCA9` — JS produces two
/// *lone surrogates* (`["\uD83D","\uDCA9"]`), which no Rust `String` can hold.
/// So we DECLINE the empty-separator split whenever the receiver contains an
/// astral character (`c as u32 > 0xFFFF`), exactly the hazard `slice`/`charAt`
/// guard against. For an all-BMP receiver each `char` is a single UTF-16 unit,
/// so `chars()` reproduces the per-code-unit pieces JS would.
///
/// **What we decline** (return `None`, leave the call):
/// - a separator that is **not a string literal** — a `RegExp` separator would
///   need a regex engine; a numeric/identifier separator would need the
///   `ToString` coercion we deliberately don't model;
/// - a **limit** that is not a non-negative integer literal (negative,
///   fractional, non-finite, or non-literal);
/// - **more than two arguments**;
/// - the astral-character empty-separator case described above.
///
/// No output-size cap is required. Unlike `repeat`/`pad`, `split` cannot
/// amplify: the pieces' combined length never exceeds the receiver's, so there
/// is no algorithmic-blowup / DoS vector to bound.
fn fold_string_split(receiver: &str, args: &[Expression]) -> Option<Vec<String>> {
    match args.len() {
        // `split()` with no separator → the whole string as the only piece.
        0 => Some(vec![receiver.to_string()]),
        // `split(sep)` or `split(sep, limit)`.
        1 | 2 => {
            // The separator must be a STRING literal. A regex literal, numeric,
            // identifier, `undefined`, etc. → decline.
            let sep = match &args[0] {
                Expression::StringLiteral(s) => s.value.as_str(),
                _ => return None,
            };
            // Optional non-negative-integer limit.
            let limit: Option<usize> = if args.len() == 2 {
                match &args[1] {
                    Expression::NumericLiteral(n) => {
                        let v = n.value;
                        if v.is_finite() && v >= 0.0 && v.fract() == 0.0 {
                            Some(v as usize)
                        } else {
                            return None; // negative / fractional / non-finite
                        }
                    }
                    _ => return None, // non-literal limit
                }
            } else {
                None
            };

            let parts: Vec<String> = if sep.is_empty() {
                // Empty separator → one piece per UTF-16 code unit. Decline if
                // any character is astral (its surrogate pair can't be split
                // into representable Rust strings).
                if receiver.chars().any(|c| c as u32 > 0xFFFF) {
                    return None;
                }
                receiver.chars().map(|c| c.to_string()).collect()
            } else {
                receiver.split(sep).map(|p| p.to_string()).collect()
            };

            Some(match limit {
                Some(n) => parts.into_iter().take(n).collect(),
                None => parts,
            })
        }
        // More than two arguments → decline (be conservative).
        _ => None,
    }
}

/// `true` iff `c` is in the ECMAScript string-trim white-space set — the union
/// of `WhiteSpace` and `LineTerminator` that `String.prototype.trim` removes
/// (ECMAScript §22.1.3.32, via `TrimString`).
///
/// We hard-code the exact set rather than use Rust's `char::is_whitespace`,
/// because the two **disagree**: Rust treats U+0085 (NEL) as whitespace but JS
/// does not, and JS treats U+FEFF (the BOM / ZERO WIDTH NO-BREAK SPACE) as
/// whitespace but Rust does not. Folding with the wrong set would silently
/// miscompile, so the predicate below is the single source of truth.
///
/// | code point(s)        | name                                   |
/// |----------------------|----------------------------------------|
/// | U+0009..=U+000D      | tab, LF, VT, FF, CR                     |
/// | U+0020               | space                                  |
/// | U+00A0               | no-break space                         |
/// | U+1680               | ogham space mark                       |
/// | U+2000..=U+200A      | en quad … hair space                   |
/// | U+2028, U+2029       | line / paragraph separator             |
/// | U+202F               | narrow no-break space                  |
/// | U+205F               | medium mathematical space              |
/// | U+3000               | ideographic space                      |
/// | U+FEFF               | zero-width no-break space (BOM)        |
fn is_js_trim_whitespace(c: char) -> bool {
    matches!(c,
        '\u{0009}'..='\u{000D}'
        | '\u{0020}'
        | '\u{00A0}'
        | '\u{1680}'
        | '\u{2000}'..='\u{200A}'
        | '\u{2028}'
        | '\u{2029}'
        | '\u{202F}'
        | '\u{205F}'
        | '\u{3000}'
        | '\u{FEFF}'
    )
}

/// Compute `value.trim()` / `trimStart()` / `trimEnd()` at compile time
/// (ECMAScript §22.1.3.32/.34/.33). `start`/`end` select which ends to strip;
/// trimming works on whole Unicode scalar values, so — unlike `slice` — it can
/// never split a surrogate pair, and always yields a valid Rust `String`
/// (hence it returns `String`, not `Option`). The stripped set is exactly
/// `is_js_trim_whitespace`.
fn fold_string_trim(value: &str, start: bool, end: bool) -> String {
    let mut s = value;
    if start {
        s = s.trim_start_matches(is_js_trim_whitespace);
    }
    if end {
        s = s.trim_end_matches(is_js_trim_whitespace);
    }
    s.to_string()
}

/// Compute `value.padStart(target[, pad])` (when `at_start`) or
/// `value.padEnd(target[, pad])` at compile time, or `None` when it cannot be
/// folded soundly (ECMAScript §22.1.3.16 / §22.1.3.17).
///
/// JS pads to `target` **UTF-16 code units** with a fill string (default a
/// single space `" "`), formed by repeating `pad` and truncating it to exactly
/// the shortfall. `"5".padStart(3, "0")` → `"005"`, `"abc".padEnd(6)` →
/// `"abc   "`, `"abc".padStart(6, "12")` → `"121abc"` (the `"12"` repeats to
/// `"121"`). If the string is already `>= target`, it is returned unchanged.
///
/// We decline (return `None`, leaving the call for the runtime) when:
/// - there is no argument, or more than two;
/// - the target is not a non-negative integer literal (no `ToLength` coercion);
/// - the pad argument is present but not a string literal;
/// - the target exceeds `MAX_PAD_UNITS` (a denial-of-service guard against
///   materializing a huge literal at compile time); or
/// - truncating the fill would split a surrogate pair, leaving a lone surrogate
///   the result `String` cannot hold (`String::from_utf16` fails) — the same
///   conservative guard `slice`/`charAt` use.
fn fold_string_pad(value: &str, args: &[Expression], at_start: bool) -> Option<String> {
    /// Cap on the padded result's length, in UTF-16 code units.
    const MAX_PAD_UNITS: u64 = 100_000;

    if args.is_empty() || args.len() > 2 {
        return None;
    }
    // arg 0: target length — a non-negative integer literal.
    let target = match &args[0] {
        Expression::NumericLiteral(n)
            if n.value.is_finite() && n.value.fract() == 0.0 && n.value >= 0.0 =>
        {
            n.value as u64
        }
        _ => return None,
    };
    if target > MAX_PAD_UNITS {
        return None;
    }
    // arg 1: pad string — a string literal, defaulting to a single space.
    let pad: &str = match args.get(1) {
        None => " ",
        Some(Expression::StringLiteral(p)) => &p.value,
        Some(_) => return None,
    };

    let s_units: Vec<u16> = value.encode_utf16().collect();
    let s_len = s_units.len() as u64;
    // Already long enough (or an empty pad can't extend it) → unchanged.
    if target <= s_len {
        return Some(value.to_string());
    }
    let pad_units: Vec<u16> = pad.encode_utf16().collect();
    if pad_units.is_empty() {
        return Some(value.to_string());
    }

    // Build the filler by repeating `pad` and truncating to the shortfall.
    let fill_len = (target - s_len) as usize;
    let mut filler: Vec<u16> = Vec::with_capacity(fill_len);
    while filler.len() < fill_len {
        let take = (fill_len - filler.len()).min(pad_units.len());
        filler.extend_from_slice(&pad_units[..take]);
    }

    // Concatenate (filler before or after the string) and reject a result that
    // a Rust `String` cannot hold (a truncation-induced lone surrogate).
    let result_units: Vec<u16> = if at_start {
        filler.iter().chain(s_units.iter()).copied().collect()
    } else {
        s_units.iter().chain(filler.iter()).copied().collect()
    };
    String::from_utf16(&result_units).ok()
}

/// Compute `value.replace(from, to)` (FIRST match) or
/// `value.replaceAll(from, to)` (EVERY match) at compile time, or `None`
/// when it cannot be folded soundly. This handles only the
/// **string-pattern, string-replacement** overload — `from` and `to` are
/// both plain string literals (ECMAScript §22.1.3.19 / §22.1.3.20).
///
/// Two divergences from a naive Rust `str::replace` make us decline:
///
///  1. **`$` substitution patterns in the replacement.** When the
///     replacement is a *string*, V8 still scans it for `$$`, `$&`,
///     `` $` ``, `$'`, and `$n` and substitutes the matched / surrounding
///     text. Rust's `str::replace` copies the replacement verbatim. So we
///     decline whenever `to` contains a `$`: `"a".replace("a","$&!")`
///     yields `"a!"` in JS but `"$&!"` under a literal copy.
///
///  2. **Empty search string.** V8's `replaceAll("", "X")` inserts `X`
///     at *every* code-unit boundary (and at both ends); `replace("",
///     "X")` prepends `X`. A literal find/replace cannot reproduce that
///     boundary semantics, so an empty `from` declines.
///
/// Otherwise `from` is matched **literally** — the string overload does
/// no regex interpretation, so `"a.b".replace(".", "X")` → `"aXb"` is
/// sound. `replace` folds the first match via `replacen(.., .., 1)`;
/// `replaceAll` folds every match via `replace`. Both operands are valid
/// strings, so a literal substitution can only yield valid UTF-16 — no
/// surrogate pair is ever split.
///
/// | call                              | result   |
/// |-----------------------------------|----------|
/// | `"aXbXc".replace("X","-")`         | `"a-bXc"`|
/// | `"a-b-c".replaceAll("-","_")`      | `"a_b_c"`|
/// | `"a.b".replace(".","X")`           | `"aXb"`  |
/// | `"abc".replace("z","Q")`           | `"abc"`  |
/// | `"a".replace("a","$&")` (→ `$`)    | declines |
/// | `"abc".replaceAll("","X")` (empty) | declines |
fn fold_string_replace(method: &str, haystack: &str, from: &str, to: &str) -> Option<String> {
    /// Cap on the folded result's length, in bytes. Mirrors the size guards
    /// on the `repeat` / `pad` folds: `replaceAll`'s output is bounded by
    /// `haystack.len() * to.len()` (a one-byte `from` matched everywhere and
    /// replaced by a long `to`), a quadratic blowup in source size, so we
    /// decline rather than materialize a huge literal at compile time. Unlike
    /// `repeat`, both operands are already in the source, so this is a
    /// defensive cap rather than an amplification vector — but it keeps a
    /// pathological pair of large literals from OOMing the optimizer.
    const MAX_REPLACE_BYTES: usize = 100_000;

    // The replacement's `$` patterns and the empty-search boundary
    // semantics are the two cases JS handles differently from a literal
    // copy; decline both (see the doc comment).
    if to.contains('$') || from.is_empty() {
        return None;
    }
    // Bound the worst-case output length *before* allocating. `replace`
    // touches one match; `replaceAll` touches every (non-overlapping) match.
    // Each match changes the length by `to.len() - from.len()`; only growth
    // (a `to` longer than `from`) can blow up, so use a saturating delta and
    // checked arithmetic against `usize` overflow.
    let matches = match method {
        "replaceAll" => haystack.matches(from).count(),
        "replace" => 1,
        _ => return None,
    };
    let worst_case = haystack
        .len()
        .checked_add(matches.checked_mul(to.len().saturating_sub(from.len()))?)?;
    if worst_case > MAX_REPLACE_BYTES {
        return None;
    }
    match method {
        "replace" => Some(haystack.replacen(from, to, 1)),
        "replaceAll" => Some(haystack.replace(from, to)),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Member access
// ---------------------------------------------------------------------

/// Folds the one member-access form whose value is known at compile time:
/// the `.length` of a string literal.
///
/// ```text
///   "hello".length   →  5
///   "".length        →  0
///   "💩".length      →  2     (one astral char = two UTF-16 code units)
/// ```
///
/// JavaScript's `String#length` is the number of **UTF-16 code units**, NOT
/// Unicode scalar values or bytes (ECMAScript §22.1.3.1 / String exotic
/// objects). Rust's [`str::encode_utf16`] yields exactly those code units —
/// astral-plane characters (U+10000…U+10FFFF) expand to a surrogate pair, so
/// `"💩".length` is `2`, matching V8/SpiderMonkey. `.count()` is total and
/// allocation-free; it cannot panic.
///
/// We fold **only** the dotted, non-computed form `"...".length`:
/// - The object must fold to a `StringLiteral` (so the value is known).
/// - The access must be non-`computed` and the property an `Identifier`
///   named `length`. We deliberately leave the computed form `"..."["length"]`
///   alone — it is vanishingly rare in real code and folding it would mean
///   reasoning about arbitrary computed keys (`s[k]`), which needs the runtime
///   value of `k`. Keeping the surface narrow keeps the fold obviously sound.
/// - Anything else (identifier objects like `s.length`, other properties like
///   `"x".charCodeAt`) falls through unchanged; we still recurse into the
///   object and property so nested constants inside them fold.
///
/// Fold `a?.b` / `a?.[k]`. Recurse into object and property so nested
/// constants fold, but keep the optional-member node itself — we deliberately
/// do NOT apply the `.length` / string-method folds to the `?.` variant, so the
/// short-circuit semantics are preserved verbatim and the transform stays
/// obviously sound.
///
/// `#[inline(never)]` on purpose: this keeps the struct-building locals out of
/// the recursive `fold_expression` frame, which recurses once per AST level. On
/// the deep-binary-chain path this arm is never entered, so its frame cost must
/// not be paid per level (see the deep-chain DoS-guard test).
#[inline(never)]
fn fold_optional_member(m: &OptionalMemberExpression, st: &mut FoldState) -> Expression {
    Expression::OptionalMemberExpression(OptionalMemberExpression {
        cv: m.cv.clone(),
        object: Box::new(fold_expression(&m.object, st)),
        property: Box::new(fold_expression(&m.property, st)),
        computed: m.computed,
    })
}

/// Fold `a?.(args)`. Recurse into callee and arguments to fold nested
/// constants, keeping the optional-call node (the string-method folds are not
/// applied, so the `?.` short-circuit variant is preserved). `#[inline(never)]`
/// for the same frame-size reason as [`fold_optional_member`].
#[inline(never)]
fn fold_optional_call(c: &OptionalCallExpression, st: &mut FoldState) -> Expression {
    Expression::OptionalCallExpression(OptionalCallExpression {
        cv: c.cv.clone(),
        callee: Box::new(fold_expression(&c.callee, st)),
        arguments: c.arguments.iter().map(|a| fold_expression(a, st)).collect(),
    })
}

/// Fold a `ChainExpression` — it transparently wraps an optional-chain spine,
/// so recurse into its inner expression and rewrap. `#[inline(never)]` for the
/// same frame-size reason as [`fold_optional_member`].
#[inline(never)]
fn fold_chain(c: &ChainExpression, st: &mut FoldState) -> Expression {
    Expression::ChainExpression(ChainExpression {
        cv: c.cv.clone(),
        expression: Box::new(fold_expression(&c.expression, st)),
    })
}

/// Is evaluating `e` guaranteed to produce no observable side effect?
///
/// Used by the array-literal `.length` fold: dropping `[a, b, c]` (replacing it
/// with `3`) is only legal when evaluating its elements runs nothing observable.
/// This is deliberately **conservative** — it only ever answers `true` for
/// expressions that are *definitely* pure. Under-answering (saying `false` for a
/// pure expression) merely misses an optimization; over-answering would drop a
/// real side effect, so the fall-through is `false`.
///
/// The classification mirrors what Closure folds for array `.length` (verified
/// against the reference compiler): literals, a plain variable read, a property
/// read (`x.y`), and pure operators over pure operands are free; a call, `new`,
/// assignment, `++`/`--`, `await`/`yield`, a tagged template, a dynamic
/// `import()`, a spread, an object literal (getters/spread), or a class
/// expression (its `static{}` block runs at definition time) are not.
fn is_side_effect_free(e: &Expression) -> bool {
    match e {
        // Inert leaves — no sub-expression, nothing to run.
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::UndefinedLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        // Building a function / arrow *value* runs no code (the body is not
        // executed). A CLASS expression is deliberately excluded — a `static {}`
        // initializer runs at definition time.
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        // A property read is treated as free, matching Closure (which folds
        // `[x.y].length`); a getter could in principle run, but Closure does not
        // model that in this fold.
        | Expression::MemberExpression(_)
        | Expression::OptionalMemberExpression(_) => true,

        // `delete x.y` mutates its target; every other unary operator (`-`, `+`,
        // `!`, `~`, `typeof`, `void`) is pure over a pure operand.
        Expression::UnaryExpression(u) => {
            u.operator != UnaryOperator::Delete && is_side_effect_free(&u.argument)
        }
        Expression::BinaryExpression(b) => {
            is_side_effect_free(&b.left) && is_side_effect_free(&b.right)
        }
        Expression::LogicalExpression(l) => {
            is_side_effect_free(&l.left) && is_side_effect_free(&l.right)
        }
        Expression::ConditionalExpression(c) => {
            is_side_effect_free(&c.test)
                && is_side_effect_free(&c.consequent)
                && is_side_effect_free(&c.alternate)
        }
        Expression::SequenceExpression(s) => s.expressions.iter().all(is_side_effect_free),
        Expression::ChainExpression(c) => is_side_effect_free(&c.expression),
        Expression::TemplateLiteral(t) => t.expressions.iter().all(is_side_effect_free),
        // A hole (`None`) evaluates nothing; a present element must be free.
        Expression::ArrayExpression(a) => a
            .elements
            .iter()
            .all(|el| el.as_ref().is_none_or(is_side_effect_free)),

        // Conservatively unsafe: Call / New / OptionalCall / Assignment / Update
        // (++/--) / Await / Yield / TaggedTemplate / ImportExpression / Spread /
        // ObjectExpression / ClassExpression. Never mark these free.
        _ => false,
    }
}

fn fold_member(m: &MemberExpression, st: &mut FoldState) -> Expression {
    // Recurse first, so e.g. `("a" + "b").length` sees the folded `"ab"`.
    let object = fold_expression(&m.object, st);
    let property = fold_expression(&m.property, st);

    if !m.computed {
        if let (Expression::StringLiteral(s), Expression::Identifier(id)) = (&object, &property) {
            if id.name == "length" {
                let len = s.value.encode_utf16().count() as f64;
                let parent = m.cv.clone();
                let before = format!("\"{}\".length", s.value);
                let after = format_js_number(len);
                let new_cv = st.fork_cv(&parent, &before, &after);
                return stamp_literal_cv(FoldedLiteral::Number(len), new_cv);
            }
        }
        // `[e0, e1, …].length` → the element count (CLOC12.193). Two guards keep
        // this byte-identical to Closure:
        //   1. a spread element (`[...x]`) contributes an unknown number of
        //      elements at runtime, so the length is not statically known —
        //      decline;
        //   2. an element with a side effect must not be dropped, since
        //      evaluating the array literal runs it — decline unless every
        //      present element is side-effect-free.
        // Holes (`[,,]`) evaluate nothing yet still count toward the length
        // (`[,,].length === 2`), so `elements.len()` is the right count.
        if let (Expression::ArrayExpression(a), Expression::Identifier(id)) = (&object, &property) {
            if id.name == "length" {
                let has_spread = a
                    .elements
                    .iter()
                    .any(|el| matches!(el, Some(Expression::SpreadElement(_))));
                let all_free = a
                    .elements
                    .iter()
                    .all(|el| el.as_ref().is_none_or(is_side_effect_free));
                if !has_spread && all_free {
                    let len = a.elements.len() as f64;
                    let parent = m.cv.clone();
                    let before = format!("array-literal[{}].length", a.elements.len());
                    let after = format_js_number(len);
                    let new_cv = st.fork_cv(&parent, &before, &after);
                    return stamp_literal_cv(FoldedLiteral::Number(len), new_cv);
                }
            }
        }
    } else {
        // `[e0, e1, …][K]` → computed integer-index access into an array
        // literal (CLOC12.196 in-bounds element pick + CLOC12.196b the
        // out-of-bounds / hole `void 0` result). The whole truth table lives
        // in `fold_array_index_access`; delegating keeps this hot, recursive
        // `fold_member` frame small so deeply-nested expressions don't blow
        // the stack (a fold body inlined here would bloat every frame on the
        // recursion — see the frame-size lesson).
        if let (Expression::ArrayExpression(a), Expression::NumericLiteral(k)) =
            (&object, &property)
        {
            if let Some(folded) = fold_array_index_access(a, k, &m.cv, st) {
                return folded;
            }
        }
        // `[e0, e1, …]["K"]` — a computed STRING key. Closure coerces the key
        // with JS `ToNumber` and applies the same index fold, so non-canonical
        // spellings (`["01"]`, `["1.0"]`, `[" 1"]`, `["0x1"]`, `[""]`, …) all
        // select their integer value's element; `["length"]` folds to the
        // element count. The full truth table lives in
        // `fold_array_string_key_access` (also `#[inline(never)]` to keep this
        // recursive frame small).
        if let (Expression::ArrayExpression(a), Expression::StringLiteral(s)) =
            (&object, &property)
        {
            if let Some(folded) = fold_array_string_key_access(a, s, &m.cv, st) {
                return folded;
            }
        }
        // `obj["key"]` → `obj.key` — a computed member whose key is a string
        // literal that is a valid, non-reserved ASCII identifier name folds to a
        // dot member (CLOC12.199). The reference Closure Compiler prints
        // `o["foo"]` as `o.foo`, `o["$x"]` as `o.$x`, `o["let"]` as `o.let`
        // (`let` is not an ES3 keyword), etc. A key that is an ES3 **reserved
        // word** (`o["class"]`, `o["static"]`, `o["delete"]`, `o["int"]`, …) or
        // is not an ASCII identifier (`o["1a"]`, `o[""]`, `o["a b"]`, `o["é"]`)
        // stays bracketed — matching Closure, which keeps ES3-unsafe keys
        // quoted so the output parses under an ES3 target.
        if let Expression::StringLiteral(s) = &property {
            if is_identifier_name(&s.value) && !is_es3_reserved_word(&s.value) {
                let before = format!("member[\"{}\"]", s.value);
                let after = format!("member.{}", s.value);
                let new_cv = st.fork_cv(&m.cv, &before, &after);
                return Expression::MemberExpression(MemberExpression {
                    cv: new_cv,
                    object: Box::new(object),
                    property: Box::new(Expression::Identifier(Identifier {
                        cv: s.cv.clone(),
                        name: s.value.clone(),
                    })),
                    computed: false,
                });
            }
        }
    }

    Expression::MemberExpression(MemberExpression {
        cv: m.cv.clone(),
        object: Box::new(object),
        property: Box::new(property),
        computed: m.computed,
    })
}

/// Is `s` an ECMAScript **ES3 reserved word** — a keyword or future-reserved
/// word from the ES3 grammar (the set Closure's Rhino `TokenStream.isKeyword`
/// uses to decide whether a property name must stay quoted / bracketed)?
///
/// Closure keeps a member key or object-property key bracketed when it is one of
/// these, so `o["class"]` / `o["delete"]` / `o["int"]` do NOT become dot access
/// even though ES5+ would permit it — the quoted form parses under an ES3
/// target. Words added AFTER ES3 (`let`, `yield`, `await`, `async`, `static` is
/// ES3-reserved, but `let`/`yield` are not) are deliberately absent, so
/// `o["let"]` → `o.let` — matching Closure byte-for-byte.
fn is_es3_reserved_word(s: &str) -> bool {
    matches!(
        s,
        // ES3 keywords
        "break" | "case" | "catch" | "continue" | "default" | "delete" | "do"
            | "else" | "finally" | "for" | "function" | "if" | "in" | "instanceof"
            | "new" | "return" | "switch" | "this" | "throw" | "try" | "typeof"
            | "var" | "void" | "while" | "with" | "debugger"
        // ES3 future-reserved words (includes the Java-flavoured set)
            | "abstract" | "boolean" | "byte" | "char" | "class" | "const"
            | "double" | "enum" | "export" | "extends" | "final" | "float"
            | "goto" | "implements" | "import" | "int" | "interface" | "long"
            | "native" | "package" | "private" | "protected" | "public"
            | "short" | "static" | "super" | "synchronized" | "throws"
            | "transient" | "volatile"
        // ES3 literals
            | "null" | "true" | "false"
    )
}

/// Fold a computed integer-index access `[e0, e1, …][K]` into an array
/// literal. Returns `Some(replacement)` when the access folds, `None` to
/// decline (leaving the `MemberExpression` intact).
///
/// This is the union of two oracle-verified arcs:
///
/// * **CLOC12.196** — an *in-bounds* index whose element is **present**
///   folds to that element (`[a, b, c][1]` → `b`).
/// * **CLOC12.196b** — an *out-of-bounds* index (`K ≥ len` **or** `K < 0`)
///   or an in-bounds **hole** reads as `undefined`, which Closure spells
///   `void 0` (`[1,2,3][5]` / `[1,,3][1]` / `[1,2,3][-1]` → `void 0`).
///
/// Truth table (all rows require **no spread** and an **integer** `K`; a
/// fractional index such as `[1,2,3][1.5]` is an ordinary absent-property
/// read that Closure leaves intact, so it declines):
///
/// | array          | K    | result   | why                                  |
/// |----------------|------|----------|--------------------------------------|
/// | `[a,b,c]`      | `1`  | `b`      | in-bounds, element present           |
/// | `[1,2,3]`      | `5`  | `void 0` | out of bounds (`K ≥ len`)            |
/// | `[1,2]`        | `2`  | `void 0` | out of bounds (`K == len`)           |
/// | `[]`           | `0`  | `void 0` | out of bounds (empty)                |
/// | `[1,2,3]`      | `-1` | `void 0` | negative index (post-fold `-1`)      |
/// | `[1,,3]`       | `1`  | `void 0` | in-bounds **hole** reads `undefined` |
/// | `[1,2,3]`      | `1.5`| decline  | fractional index — not an element    |
///
/// **Side-effect discipline** differs between the two results, because it
/// governs which elements may be dropped:
///
/// * A **present** in-bounds element is *preserved verbatim* — its own side
///   effect still runs (`[a, b()][1]` → `b()`). Only the **other** elements
///   must be side-effect-free, or dropping them would drop a side effect
///   (`[a, b()][0]` declines).
/// * A **`void 0`** result drops the *whole* array literal, so **every**
///   element must be side-effect-free (`[f(),2][5]` declines even though the
///   index is out of bounds).
///
/// Marked `#[inline(never)]`: `fold_member` is on the recursive
/// `fold_expression` path, so inlining this body would enlarge every stack
/// frame along a deeply-nested expression and shrink the nesting depth we can
/// fold before overflowing (see the frame-size lesson).
#[inline(never)]
fn fold_array_index_access(
    a: &ArrayExpression,
    k: &NumericLiteral,
    parent_cv: &Option<String>,
    st: &mut FoldState,
) -> Option<Expression> {
    // A spread (`[...x]`) makes the runtime indices statically unknown.
    let has_spread = a
        .elements
        .iter()
        .any(|el| matches!(el, Some(Expression::SpreadElement(_))));
    if has_spread {
        return None;
    }
    // Only a finite INTEGER index selects (or misses) an array element. A
    // fractional index is an ordinary property read Closure leaves intact.
    // (Post-fold a negative index is a `NumericLiteral` with a negative
    // `value` — `-1` folds `-1` — so the negative case flows through here.)
    if !(k.value.is_finite() && k.value.fract() == 0.0) {
        return None;
    }

    let len = a.elements.len();
    // `void 0` drops the whole literal, so every element must be pure.
    let all_free = || {
        a.elements
            .iter()
            .all(|el| el.as_ref().is_none_or(is_side_effect_free))
    };

    if k.value >= 0.0 && (k.value as usize) < len {
        // In bounds: `0 ≤ K < len`.
        let idx = k.value as usize;
        match &a.elements[idx] {
            // Present element (CLOC12.196): fold to it, preserving its own
            // side effect; only the OTHER elements must be side-effect-free.
            Some(selected) => {
                let others_free = a
                    .elements
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != idx)
                    .all(|(_, el)| el.as_ref().is_none_or(is_side_effect_free));
                if !others_free {
                    return None;
                }
                let before = format!("array-literal[{len}][{idx}]");
                let after = format!("element[{idx}]");
                // Records the fold (and sets `changed`); the selected element
                // keeps its own cv for span provenance.
                let _new_cv = st.fork_cv(parent_cv, &before, &after);
                Some(selected.clone())
            }
            // In-bounds HOLE (CLOC12.196b): reads as `undefined` → `void 0`.
            None => {
                if !all_free() {
                    return None;
                }
                let before = format!("array-literal[{len}][{idx}]=hole");
                let new_cv = st.fork_cv(parent_cv, &before, "void 0");
                Some(stamp_literal_cv(FoldedLiteral::Undefined, new_cv))
            }
        }
    } else {
        // Out of bounds (CLOC12.196b): `K ≥ len` or `K < 0`. The absent index
        // reads as `undefined` → `void 0`.
        if !all_free() {
            return None;
        }
        let before = format!("array-literal[{len}][{}]", format_js_number(k.value));
        let new_cv = st.fork_cv(parent_cv, &before, "void 0");
        Some(stamp_literal_cv(FoldedLiteral::Undefined, new_cv))
    }
}

/// Fold a computed **string-key** access into an array literal:
/// `[e0, e1, …]["K"]`. Returns `Some(replacement)` when it folds, `None` to
/// decline (leaving the `MemberExpression` intact).
///
/// The reference Closure Compiler coerces the string key with the SAME full
/// JS `ToNumber` used by `Number("…")` (`fold_number`), then applies the
/// integer-index fold — so every spelling that `ToNumber` maps to an integer
/// selects (or misses) the corresponding element, including the non-canonical
/// ones. Verified byte-identical at `SIMPLE`:
///
/// | access                | ToNumber(key) | result   | why                     |
/// |-----------------------|---------------|----------|-------------------------|
/// | `[a,b,c]["0"]`        | `0`           | `a`      | in-bounds               |
/// | `[a,b,c]["01"]`       | `1`           | `b`      | leading zero → `1`      |
/// | `[a,b,c]["1.0"]`      | `1`           | `b`      | trailing `.0` → `1`     |
/// | `[a,b,c][" 1"]`/`["1 "]`| `1`         | `b`      | whitespace trimmed      |
/// | `[a,b,c]["0x1"]`      | `1`           | `b`      | hex literal → `1`       |
/// | `[a,b,c]["1e0"]`      | `1`           | `b`      | exponent → `1`          |
/// | `[a,b,c][""]`         | `0`           | `a`      | `ToNumber("")` is `+0`  |
/// | `[a,b,c]["3"]`        | `3`           | `void 0` | out of bounds           |
/// | `[a,b,c]["-1"]`       | `-1`          | `void 0` | negative                |
/// | `[a,b,c]["1.5"]`      | `1.5`         | decline  | fractional — not index  |
/// | `[a,b,c]["foo"]`      | `NaN`         | decline  | not numeric (see below) |
/// | `[a,b,c]["length"]`   | —             | `3`      | the length property     |
///
/// Two keys need special handling because `fold_number` can't express them:
///
/// * `"length"` isn't an index — it's the length property — so it routes to the
///   same element-count fold as `.length` (subject to the same no-spread /
///   all-elements-pure guard).
/// * A key whose `ToNumber` is `NaN` (`"foo"`) makes `fold_number` return
///   `None`, so this declines. Closure instead rewrites `["foo"]` to `.foo`
///   (a member-access *normalisation*, not an index fold); that transform is a
///   separate slice, so declining here leaves the `["foo"]` access intact.
///
/// Marked `#[inline(never)]` for the same frame-size reason as
/// [`fold_array_index_access`]: `fold_member` is on the recursive
/// `fold_expression` path.
#[inline(never)]
fn fold_array_string_key_access(
    a: &ArrayExpression,
    s: &StringLiteral,
    parent_cv: &Option<String>,
    st: &mut FoldState,
) -> Option<Expression> {
    // `["length"]` — the length property, identical to `.length`. Same guards:
    // a spread makes the count unknown, and every present element must be
    // side-effect-free because folding drops the whole array literal.
    if s.value == "length" {
        let has_spread = a
            .elements
            .iter()
            .any(|el| matches!(el, Some(Expression::SpreadElement(_))));
        let all_free = a
            .elements
            .iter()
            .all(|el| el.as_ref().is_none_or(is_side_effect_free));
        if has_spread || !all_free {
            return None;
        }
        let len = a.elements.len() as f64;
        let before = format!("array-literal[{}][\"length\"]", a.elements.len());
        let after = format_js_number(len);
        let new_cv = st.fork_cv(parent_cv, &before, &after);
        return Some(stamp_literal_cv(FoldedLiteral::Number(len), new_cv));
    }

    // Otherwise coerce the key with JS `ToNumber` (declines on `NaN`/`Infinity`/
    // >2^53) and reuse the integer-index fold, which itself declines on a
    // fractional index such as `ToNumber("1.5") == 1.5`.
    let n = fold_number(&s.value)?;
    let k = NumericLiteral {
        cv: None,
        value: n,
        raw: format_js_number(n),
    };
    fold_array_index_access(a, &k, parent_cv, st)
}

// ---------------------------------------------------------------------
// Binary
// ---------------------------------------------------------------------

/// The `f64` value of an expression that is a numeric literal, else `None`.
/// (Unary-minus on a literal has already been folded to a `NumericLiteral` by
/// the bottom-up walk, so `-1` arrives here as `NumericLiteral(-1.0)`.)
fn numeric_literal_value(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::NumericLiteral(n) => Some(n.value),
        _ => None,
    }
}

fn fold_binary(b: &BinaryExpression, st: &mut FoldState) -> Expression {
    // First recurse into children. By the time we look at left/right
    // they're already folded — that's what gives us `1 + (2 * 3) → 7`
    // in one bottom-up walk.
    let left = fold_expression(&b.left, st);
    let right = fold_expression(&b.right, st);

    // Try to fold. If we can't, return a new BinaryExpression with the
    // (possibly folded) children.
    if let Some(value) = try_fold_binary_op(b.operator, &left, &right) {
        // Division / modulo BY ZERO is never folded — matching the reference
        // Closure Compiler, which keeps the source operation rather than emit
        // the shadowable `Infinity`/`NaN` globals:
        //
        //   1/0  → Infinity   kept as `1/0`
        //   -1/0 → -Infinity  kept as `-1/0`   (`-1` is a folded literal operand)
        //   0/0  → NaN        kept as `0/0`
        //   1%0  → NaN        kept as `1%0`
        //
        // The result of `x / 0` or `x % 0` is always non-finite (`±Infinity` or
        // `NaN`), and folding to those literals is both LONGER than the source
        // and unsound if `Infinity`/`NaN` is shadowed in scope — so Closure
        // declines, and so do we. A NON-zero divisor still folds normally
        // (`5/2`→`2.5`, `1/8`→`.125`, `6/3`→`2`). (A separate divergence —
        // Closure also keeps a NON-terminating quotient like `1/3` rather than
        // the 16-digit `.3333333333333333`, governed by its numeric byte-cost
        // heuristic — is filed as a follow-up, not handled here.)
        if matches!(b.operator, BinaryOperator::Div | BinaryOperator::Mod)
            && numeric_literal_value(&right) == Some(0.0)
        {
            return Expression::BinaryExpression(BinaryExpression {
                cv: b.cv.clone(),
                operator: b.operator,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        let parent = b.cv.clone();
        let before = format!(
            "({}) {} ({})",
            lit_label(&left),
            op_label(b.operator),
            lit_label(&right)
        );
        let after = literal_label(&value);
        let new_cv = st.fork_cv(&parent, &before, &after);
        return stamp_literal_cv(value, new_cv);
    }

    Expression::BinaryExpression(BinaryExpression {
        cv: b.cv.clone(),
        operator: b.operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

/// JS `ToInt32` (ECMAScript §7.1.6): coerce a Number to a signed 32-bit
/// integer. Non-finite values and `±0` map to `0`; otherwise truncate
/// toward zero, reduce modulo 2³², and reinterpret the low 32 bits as a
/// signed integer.
///
/// ```text
///   to_int32(5.9)        = 5
///   to_int32(-1.0)       = -1
///   to_int32(4294967296) = 0          // 2³² wraps to 0
///   to_int32(3000000000) = -1294967296 // ≥ 2³¹ becomes negative
///   to_int32(NaN/±Inf)   = 0
/// ```
fn to_int32(x: f64) -> i32 {
    // `to_uint32` already gives the low-32-bit value as an unsigned int;
    // reinterpreting that bit pattern as `i32` yields the signed result.
    to_uint32(x) as i32
}

/// JS `ToUint32` (ECMAScript §7.1.7): coerce a Number to an unsigned
/// 32-bit integer. Non-finite values and `±0` map to `0`; otherwise
/// truncate toward zero and reduce modulo 2³² into `[0, 2³²)`.
fn to_uint32(x: f64) -> u32 {
    if !x.is_finite() || x == 0.0 {
        return 0;
    }
    // Truncate toward zero (ES `ToIntegerOrInfinity` for a finite value),
    // then take the non-negative residue modulo 2³². `rem_euclid` keeps the
    // result in `[0, 2³²)` for negative inputs too (e.g. -1 → 2³²-1), and an
    // integer-valued f64 in that range casts to `u32` exactly.
    const TWO_POW_32: f64 = 4_294_967_296.0;
    x.trunc().rem_euclid(TWO_POW_32) as u32
}

/// Pure fold logic — given two operand expressions and an operator,
/// decide whether they collapse to a literal. Returns `None` when the
/// operands aren't both literals, when types don't match for the
/// chosen comparison, or when the operator isn't in the fold set.
fn try_fold_binary_op(
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
) -> Option<FoldedLiteral> {
    use BinaryOperator::*;

    let (ln, rn) = match (left, right) {
        (Expression::NumericLiteral(a), Expression::NumericLiteral(b)) => {
            (Some(a.value), Some(b.value))
        }
        _ => (None, None),
    };

    // Arithmetic on two numeric literals.
    if let (Some(a), Some(b)) = (ln, rn) {
        return match op {
            Add => Some(FoldedLiteral::Number(a + b)),
            Sub => Some(FoldedLiteral::Number(a - b)),
            Mul => Some(FoldedLiteral::Number(a * b)),
            Div => Some(FoldedLiteral::Number(a / b)),
            Mod => Some(FoldedLiteral::Number(a % b)),
            Exp => Some(FoldedLiteral::Number(a.powf(b))),
            Eq | StrictEq => Some(FoldedLiteral::Boolean(a == b)),
            NotEq | StrictNotEq => Some(FoldedLiteral::Boolean(a != b)),
            Lt => Some(FoldedLiteral::Boolean(a < b)),
            LtEq => Some(FoldedLiteral::Boolean(a <= b)),
            Gt => Some(FoldedLiteral::Boolean(a > b)),
            GtEq => Some(FoldedLiteral::Boolean(a >= b)),
            // Bitwise / shift on numeric literals (CLOC15.D). JS evaluates
            // these on 32-bit integers: both operands are coerced via
            // `ToInt32` (or `ToUint32` for the left side of `>>>` and the
            // result of `>>>`), the operation runs on those 32-bit values,
            // and the shift COUNT is `ToUint32(rhs) & 31`. The operands here
            // are already numeric literals, so the coercions are exact and
            // deterministic — folding cannot diverge from the runtime value.
            BitAnd => Some(FoldedLiteral::Number((to_int32(a) & to_int32(b)) as f64)),
            BitOr => Some(FoldedLiteral::Number((to_int32(a) | to_int32(b)) as f64)),
            BitXor => Some(FoldedLiteral::Number((to_int32(a) ^ to_int32(b)) as f64)),
            // `<<` / `>>`: left is ToInt32 (signed). `wrapping_shl`/`shr`
            // shift by `rhs % 32`, which equals JS's `ToUint32(rhs) & 31`.
            // `i32 >>` is arithmetic (sign-propagating), matching `>>`.
            LeftShift => Some(FoldedLiteral::Number(
                to_int32(a).wrapping_shl(to_uint32(b)) as f64,
            )),
            RightShift => Some(FoldedLiteral::Number(
                to_int32(a).wrapping_shr(to_uint32(b)) as f64,
            )),
            // `>>>`: left is ToUint32 and the shift is logical (zero-fill);
            // the result is an UNSIGNED 32-bit value, so it can exceed
            // `i32::MAX` and is rendered as a non-negative `Number`.
            UnsignedRightShift => Some(FoldedLiteral::Number(
                to_uint32(a).wrapping_shr(to_uint32(b)) as f64,
            )),
            _ => None,
        };
    }

    // String concatenation and mixed string+number coercion for `+`.
    if let Add = op {
        let lstr = literal_to_string(left);
        let rstr = literal_to_string(right);
        // Per ES: if EITHER operand is a string, `+` is concatenation
        // and the non-string is coerced. We only fold when both can
        // be statically rendered as strings.
        if matches!(left, Expression::StringLiteral(_))
            || matches!(right, Expression::StringLiteral(_))
        {
            if let (Some(a), Some(b)) = (lstr, rstr) {
                return Some(FoldedLiteral::String(format!("{}{}", a, b)));
            }
        }
    }

    // String/string comparisons — strict equality only collapses safely
    // when both sides are the same literal type.
    if let (Expression::StringLiteral(a), Expression::StringLiteral(b)) = (left, right) {
        return match op {
            Eq | StrictEq => Some(FoldedLiteral::Boolean(a.value == b.value)),
            NotEq | StrictNotEq => Some(FoldedLiteral::Boolean(a.value != b.value)),
            Lt => Some(FoldedLiteral::Boolean(a.value < b.value)),
            LtEq => Some(FoldedLiteral::Boolean(a.value <= b.value)),
            Gt => Some(FoldedLiteral::Boolean(a.value > b.value)),
            GtEq => Some(FoldedLiteral::Boolean(a.value >= b.value)),
            _ => None,
        };
    }

    // Boolean comparisons.
    if let (Expression::BooleanLiteral(a), Expression::BooleanLiteral(b)) = (left, right) {
        return match op {
            Eq | StrictEq => Some(FoldedLiteral::Boolean(a.value == b.value)),
            NotEq | StrictNotEq => Some(FoldedLiteral::Boolean(a.value != b.value)),
            _ => None,
        };
    }

    // Null/null comparisons (closes CLOC12 gap-007).
    //
    // JS-spec behavior at compile time when both sides are
    // `null` literals:
    //
    //     null == null        → true     (same primitive value)
    //     null === null       → true     (same type + value)
    //     null != null        → false
    //     null !== null       → false
    //     null <  null        → false    (both coerce to 0, 0 < 0 is false)
    //     null >  null        → false    (0 > 0 is false)
    //     null <= null        → true     (0 <= 0 is true)
    //     null >= null        → true     (0 >= 0 is true)
    //
    // Relational operators on `null` go through the abstract
    // relational comparison algorithm in the ECMAScript spec
    // (§IsLessThan), which calls ToPrimitive then ToNumber. For
    // `null`, ToNumber(null) is `0`. So `null < null` reduces to
    // `0 < 0` which is `false`, and the symmetric cases follow.
    if let (Expression::NullLiteral(_), Expression::NullLiteral(_)) = (left, right) {
        return match op {
            Eq | StrictEq => Some(FoldedLiteral::Boolean(true)),
            NotEq | StrictNotEq => Some(FoldedLiteral::Boolean(false)),
            Lt => Some(FoldedLiteral::Boolean(false)),
            Gt => Some(FoldedLiteral::Boolean(false)),
            LtEq => Some(FoldedLiteral::Boolean(true)),
            GtEq => Some(FoldedLiteral::Boolean(true)),
            _ => None,
        };
    }

    // Cross-type LOOSE equality involving `null` (closes CLOC12 gap-003).
    //
    // The ECMAScript abstract-equality algorithm (§IsLooselyEqual) has
    // exactly two ways for `null == x` (or `x == null`) to be `true`:
    //
    //   1. `x` is also `null` (same primitive value).            ← gap-007 above
    //   2. `x` is `undefined`. The spec hard-codes that
    //      `null == undefined → true` (and symmetrically).
    //
    // Every other case — `null == 0`, `null == ""`, `null == true`,
    // `null == 1n` — falls through the algorithm and produces `false`.
    // None of the type-coercion clauses (Number↔String, Boolean → Number,
    // Object → primitive) apply when one side is `null`: `null` is its
    // own ECMAScript "Null" type, and the coercion clauses are written
    // against Number/String/Boolean/BigInt/Object operands.
    //
    // Truth table once `null` is on one side and a *literal* of known JS
    // type sits on the other:
    //
    //     other side          ==     !=
    //     -------------------+------+------
    //     null               | true | false   ← gap-007 (already returned above)
    //     undefined          | true | false
    //     number             | false| true
    //     string             | false| true
    //     boolean            | false| true
    //     bigint             | false| true
    //
    // We deliberately bail out when the non-null side is *not* a literal
    // we recognise (e.g. an Identifier, a function call): a runtime
    // value could itself be `null` or `undefined`, and folding to a
    // concrete boolean would be unsound.
    //
    // Why this runs after the null/null branch above: that branch
    // already settled `null == null`, so by the time we get here we
    // know at most one side is a NullLiteral. We pick whichever side it
    // is and inspect the *other* side.
    if matches!(op, Eq | NotEq) {
        let other = match (left, right) {
            (Expression::NullLiteral(_), other) if !matches!(other, Expression::NullLiteral(_)) => {
                Some(other)
            }
            (other, Expression::NullLiteral(_)) if !matches!(other, Expression::NullLiteral(_)) => {
                Some(other)
            }
            _ => None,
        };
        if let Some(other) = other {
            if let Some(other_type) = js_literal_type(other) {
                // `other_type` ∈ {"number", "string", "boolean",
                // "bigint", "undefined"} here — "null" is excluded by
                // the `other` filter above, so the only `true` case is
                // when the partner is the undefined literal.
                let eq = other_type == "undefined";
                return match op {
                    Eq => Some(FoldedLiteral::Boolean(eq)),
                    NotEq => Some(FoldedLiteral::Boolean(!eq)),
                    _ => unreachable!(),
                };
            }
        }
    }

    // Cross-type strict equality (closes CLOC12 gap-008).
    //
    // Per ECMAScript §IsStrictlyEqual, `===` between two values
    // of different *types* is always `false` (and `!==` is
    // always `true`) regardless of values:
    //
    //     1 === "1"            → false
    //     1 !== "1"            → true
    //     true === 1           → false
    //     null === undefined   → false       (different types — see also gap-001)
    //
    // We don't *touch* loose `==` here — that goes through the
    // abstract-equality algorithm with coercion, which is
    // gap-003 / gap-004. Strict equality is the easy case: just
    // check whether the two literals are of different JS types.
    //
    // Why this runs after the same-type branches: those branches
    // already handle `1 === 1`, `"a" === "a"`, `true === true`,
    // `null === null`. By the time we get here, we know at least
    // one side is not a literal we recognise *or* the two sides
    // are of different JS literal types. We only want to fire
    // when both are literals of *known but different* JS types.
    if matches!(op, StrictEq | StrictNotEq) {
        if let (Some(lt), Some(rt)) = (js_literal_type(left), js_literal_type(right)) {
            if lt != rt {
                return match op {
                    StrictEq => Some(FoldedLiteral::Boolean(false)),
                    StrictNotEq => Some(FoldedLiteral::Boolean(true)),
                    _ => unreachable!(),
                };
            }
        }
    }

    // Cross-type Number ↔ String comparisons (closes CLOC12 gap-004).
    //
    // The ECMAScript abstract-equality (§IsLooselyEqual) and
    // abstract-relational-comparison (§IsLessThan) algorithms both
    // coerce a String operand against a Number operand by calling
    // §StringToNumber on the string and then doing a Number-vs-Number
    // comparison. So:
    //
    //   1 < '2'   →  1 < 2     →  true
    //   1 == '2'  →  1 == 2    →  false
    //   2 < '1'   →  2 < 1     →  false
    //   1 <= '1'  →  1 <= 1    →  true
    //
    // We deliberately bail when the string can't be losslessly mapped
    // to a number we're sure JS would produce — this keeps the fold
    // sound under unusual whitespace, hex / binary / octal prefixes,
    // or stray non-numeric content. See `js_string_to_number_strict`
    // for the conservative subset of §StringToNumber we implement.
    //
    // Strict equality on Number/String is gap-008: already returns
    // `false` / `true` above, regardless of the string's content. So
    // this branch only fires on loose `==` / `!=` / `< > <= >=`.
    //
    // Why bail rather than fold to NaN: JS's StringToNumber maps
    // unrecognised inputs to NaN, and `1 == NaN` is `false`, `1 < NaN`
    // is `false`, etc. We *could* fold those, but distinguishing
    // "NaN" from "we don't know" at the fold-rule level requires
    // committing to a complete StringToNumber implementation. The
    // conservative `Option<f64>` helper returns `None` for ambiguous
    // input; the fold then bails and runtime handles it.
    if matches!(op, Eq | NotEq | Lt | LtEq | Gt | GtEq) {
        let (num_val, str_lit, num_on_left) = match (left, right) {
            (Expression::NumericLiteral(n), Expression::StringLiteral(s)) => {
                (n.value, &s.value, true)
            }
            (Expression::StringLiteral(s), Expression::NumericLiteral(n)) => {
                (n.value, &s.value, false)
            }
            _ => (0.0, &String::new(), false), // sentinel; flag below
        };
        let applies = matches!(
            (left, right),
            (Expression::NumericLiteral(_), Expression::StringLiteral(_))
                | (Expression::StringLiteral(_), Expression::NumericLiteral(_))
        );
        if applies {
            if let Some(coerced) = js_string_to_number_strict(str_lit) {
                // After coercion both sides are JS Numbers. The
                // operation must be evaluated in the *original* lexical
                // order (`num_on_left = true` means the Number was the
                // left-hand operand pre-coercion), because `<` and `>`
                // are not symmetric.
                let (a, b) = if num_on_left {
                    (num_val, coerced)
                } else {
                    (coerced, num_val)
                };
                return match op {
                    Eq => Some(FoldedLiteral::Boolean(a == b)),
                    NotEq => Some(FoldedLiteral::Boolean(a != b)),
                    Lt => Some(FoldedLiteral::Boolean(a < b)),
                    LtEq => Some(FoldedLiteral::Boolean(a <= b)),
                    Gt => Some(FoldedLiteral::Boolean(a > b)),
                    GtEq => Some(FoldedLiteral::Boolean(a >= b)),
                    _ => unreachable!("op guarded by outer matches!"),
                };
            }
        }
    }

    // typeof-identity fold (closes CLOC12 gap-029).
    //
    // `typeof x === typeof x` → true
    // `typeof x !== typeof x` → false
    //
    // Fires only when:
    // 1. The operator is `===` or `!==`.
    // 2. Both sides are `typeof <Identifier>` with the SAME
    //    identifier name.
    //
    // Why Identifier-only is provably safe: ECMAScript §UnaryTypeofExpression
    // special-cases `typeof <undeclared-identifier>` so it returns
    // the string `"undefined"` *instead* of throwing a
    // ReferenceError. So even if `x` is never declared in the
    // program, evaluating `typeof x` twice produces the same
    // string both times — guaranteeing the equality. No shadowing
    // / no race / no side effect can flip the result between the
    // two evaluations because reading a binding from a scope chain
    // is observably deterministic within a single expression's
    // evaluation.
    //
    // Why we don't bother with NumericLiteral / StringLiteral /
    // BooleanLiteral / NullLiteral / BigIntLiteral on the inside:
    // by the time we get here, the inner `typeof <literal>` has
    // already been folded to a StringLiteral by the unary-fold
    // path (CLOC12.09 + the bigint extension), and the resulting
    // two StringLiterals were then collapsed by the string-string
    // comparison branch above. So the only new behaviour this
    // arm adds is the identifier case.
    if matches!(op, StrictEq | StrictNotEq) {
        if let (Expression::UnaryExpression(lu), Expression::UnaryExpression(ru)) = (left, right) {
            if lu.operator == UnaryOperator::TypeOf && ru.operator == UnaryOperator::TypeOf {
                if let (Expression::Identifier(la), Expression::Identifier(ra)) =
                    (lu.argument.as_ref(), ru.argument.as_ref())
                {
                    if la.name == ra.name {
                        return match op {
                            StrictEq => Some(FoldedLiteral::Boolean(true)),
                            StrictNotEq => Some(FoldedLiteral::Boolean(false)),
                            _ => unreachable!(),
                        };
                    }
                }
            }
        }
    }

    None
}

/// Tag the JS type of a literal expression for the cross-type
/// strict-equality fold (gap-008). Returns `None` for anything
/// that isn't a Phase 1 primitive literal — identifiers, calls,
/// member expressions, etc. — so the caller leaves them alone.
///
/// The string tags here are *internal* to this module; they're
/// not the result of `typeof` (which has its own quirks like
/// `typeof null === "object"`). For the strict-equality fold,
/// what matters is that two different literal kinds get
/// different tags, so the equality of the tags drives the fold.
fn js_literal_type(expr: &Expression) -> Option<&'static str> {
    match expr {
        Expression::NumericLiteral(_) => Some("number"),
        Expression::StringLiteral(_) => Some("string"),
        Expression::BooleanLiteral(_) => Some("boolean"),
        Expression::NullLiteral(_) => Some("null"),
        Expression::BigIntLiteral(_) => Some("bigint"),
        Expression::UndefinedLiteral(_) => Some("undefined"),
        _ => None,
    }
}

/// Conservative subset of ECMAScript §StringToNumber used for cross-type
/// Number/String comparison folding (gap-004).
///
/// Returns `Some(f64)` only when the input is *unambiguously* convertible
/// to a JS Number under the spec rules; returns `None` for anything else.
/// Bailing on the ambiguous cases is sound — the fold simply doesn't fire
/// and the runtime handles the comparison.
///
/// What we recognise (mirroring §StringToNumber's StrNumericLiteral grammar):
///
/// 1. **Empty / whitespace-only** → `0.0`. JS treats `""`, `"   "`, `"\t\n"`
///    as `0` per §StringNumericValue. Our whitespace recogniser matches
///    ASCII whitespace which is a strict subset of JS WhiteSpace and
///    LineTerminator, so we may reject some exotic-whitespace strings
///    JS would accept — but we never accept what JS would reject.
///
/// 2. **`"Infinity"`, `"+Infinity"`, `"-Infinity"`** (case-sensitive,
///    after trim) → ±∞. JS is case-sensitive here; we reject `"inf"`,
///    `"INFINITY"`, etc., even though Rust's `f64::from_str` accepts them.
///
/// 3. **Decimal-style numeric literals** matching the JS grammar:
///    optional sign, then a sequence of digits with optional `.` and
///    optional `[eE][+-]?digits` exponent. Examples: `"1"`, `"1.5"`,
///    `"-2"`, `".5"`, `"1e3"`, `"1.5e-10"`.
///
/// What we **don't** handle (deliberate follow-ups; returning `None` is sound):
///
/// - Hex / binary / octal prefixes (`0x...`, `0b...`, `0o...`). These
///   are valid JS NumericLiteralStrings; we'd just need to parse via
///   the relevant radix.
/// - Non-ASCII JS WhiteSpace (NBSP, ZWNBSP, BOM, various Unicode space
///   separators) and LineTerminator chars (LS U+2028, PS U+2029).
/// - Underscore numeric separators (`"1_000_000"`) — those aren't
///   actually accepted by StringToNumber (JS only allows separators
///   in source-level NumericLiterals, not StringNumericLiteral) so this
///   is intentional.
/// - The empty-or-whitespace `+` `-` sign-only forms (`"+"`, `"-"`) →
///   JS gives `NaN`. We reject (return `None`) rather than fold to NaN.
///
/// Why a separate helper (vs inlining): it's small, but the spec
/// reasoning is dense and benefits from a single named entry point that
/// future gap-fixes can extend (e.g. hex support) without re-deriving
/// the rules.
fn js_string_to_number_strict(s: &str) -> Option<f64> {
    let t = s.trim_matches(|c: char| c.is_ascii_whitespace());

    // Rule 1: empty trimmed input → 0 (spec §StringNumericValue).
    if t.is_empty() {
        return Some(0.0);
    }

    // Rule 2: explicit Infinity (case-sensitive).
    match t {
        "Infinity" | "+Infinity" => return Some(f64::INFINITY),
        "-Infinity" => return Some(f64::NEG_INFINITY),
        _ => {}
    }

    // Rule 3: decimal-style numeric literal.
    //
    // We need to gate Rust's `f64::from_str` on a character set check,
    // because:
    //   - `f64::from_str("inf")` → `Ok(∞)` (Rust accepts case-insensitive
    //     "inf" / "infinity"); JS rejects these → NaN.
    //   - `f64::from_str("nan")` → `Ok(NaN)`; JS rejects these → NaN
    //     (different return path; observable as fold suppression).
    //
    // Quick rejection: if any character outside the JS decimal grammar
    // appears, bail. The grammar is: optional `+`/`-`, then digits / `.` /
    // `e` / `E` / `+` / `-` (where the second sign can only appear after
    // an exponent). We over-approximate by accepting the union character
    // class — Rust's parser will reject malformed orderings.
    let allowed = |c: char| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-');
    if !t.chars().all(allowed) {
        return None;
    }
    // Disallow lone sign / lone dot / lone exponent marker.
    if matches!(
        t,
        "+" | "-" | "." | "e" | "E" | "+e" | "-e" | "+." | "-." | ".e" | ".E"
    ) {
        return None;
    }
    t.parse::<f64>()
        .ok()
        .filter(|f| f.is_finite() || t.contains(['e', 'E']))
}

/// Best-effort static string rendering of a literal expression. Used
/// for `+` concatenation folding. Returns `None` for non-literal or
/// not-statically-renderable inputs.
fn literal_to_string(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.clone()),
        Expression::NumericLiteral(n) => Some(format_js_number(n.value)),
        Expression::BooleanLiteral(b) => Some(if b.value {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        Expression::NullLiteral(_) => Some("null".to_string()),
        _ => None,
    }
}

/// Render a number the way JS's `String(x)` does — `42` not `42.0`,
/// `0.5` not `.5`, `NaN` and `Infinity` literal-cased. Used so
/// `"x" + 1 === "x1"` not `"x1.0"`.
/// Compute JavaScript's global `parseInt(string, radix)` at compile time
/// (ECMAScript §19.2.5), or `None` when the runtime result would be `NaN`.
///
/// The algorithm: strip leading whitespace, read an optional `+`/`-` sign, pick
/// the radix, then consume the longest run of valid base-`radix` digits and
/// stop at the first character that is not one — which is why `parseInt("12px")`
/// is `12` (the `"px"` is ignored). With no valid leading digit the runtime
/// yields `NaN`, which has no literal, so we return `None`.
///
/// `radix_arg` is the already integer-validated second argument: `None` for the
/// one-argument call. A radix of `0` (or absent) means base 10, except that a
/// `"0x"`/`"0X"` prefix selects base 16 — modern JS no longer treats a leading
/// `"0"` as octal, so `parseInt("08")` is `8`. Any radix outside `2..=36` makes
/// the runtime return `NaN`, so we decline.
///
/// Like V8 we accumulate in `f64`, so a magnitude beyond `2^53` rounds exactly
/// the way the engine's own `parseInt` rounds, and a run long enough to overflow
/// to `Infinity` fails the final `is_finite` check and is declined.
fn fold_parse_int(input: &str, radix_arg: Option<f64>) -> Option<f64> {
    let s = input.trim_start_matches(is_js_trim_whitespace);
    let b = s.as_bytes();
    let mut i = 0usize;

    let mut sign = 1.0f64;
    if let Some(&c) = b.first() {
        if c == b'+' || c == b'-' {
            if c == b'-' {
                sign = -1.0;
            }
            i = 1;
        }
    }

    // Resolve the radix, honouring a `0x`/`0X` prefix for base 0 (auto) and 16.
    let mut radix = radix_arg.unwrap_or(0.0) as i64;
    if radix == 0 {
        if i + 1 < b.len() && b[i] == b'0' && (b[i + 1] | 0x20) == b'x' {
            radix = 16;
            i += 2;
        } else {
            radix = 10;
        }
    } else if radix == 16 && i + 1 < b.len() && b[i] == b'0' && (b[i + 1] | 0x20) == b'x' {
        i += 2;
    }
    if !(2..=36).contains(&radix) {
        return None;
    }

    let digits_start = i;
    let mut acc = 0.0f64;
    while i < b.len() {
        let digit = match b[i] {
            c @ b'0'..=b'9' => (c - b'0') as i64,
            c @ b'a'..=b'z' => (c - b'a') as i64 + 10,
            c @ b'A'..=b'Z' => (c - b'A') as i64 + 10,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        acc = acc * radix as f64 + digit as f64;
        i += 1;
    }
    if i == digits_start {
        return None; // no valid leading digit → NaN
    }
    let value = sign * acc;
    value.is_finite().then_some(value)
}

/// Compute JavaScript's global `parseFloat(string)` at compile time
/// (ECMAScript §19.2.4), or `None` when the result is `NaN` or `±Infinity`.
///
/// `parseFloat` reads the longest leading prefix of the whitespace-trimmed
/// string that matches a decimal number — optional sign, integer part,
/// fractional part, exponent — and ignores the rest, so `parseFloat("3.14abc")`
/// is `3.14` and `parseFloat("1e3")` is `1000`. A missing mantissa
/// (`parseFloat("")`, `parseFloat("abc")`) yields `NaN`; the prefix `"Infinity"`
/// yields `Infinity`. Neither `NaN` nor `Infinity` has a numeric literal, so
/// both make us decline.
///
/// We scan the prefix ourselves to accept exactly JavaScript's grammar (a
/// trailing `"5."` and a leading `".5"` are both valid), then hand the matched
/// text to Rust's own correctly-rounded `f64` parser, which yields the same
/// nearest-`f64` value the engine produces. A bare trailing dot (and a dot
/// directly before the exponent, `"5.e3"`) is the one shape Rust rejects, so we
/// splice in a `0` before parsing.
fn fold_parse_float(input: &str) -> Option<f64> {
    let s = input.trim_start_matches(is_js_trim_whitespace);
    let b = s.as_bytes();
    let mut i = 0usize;
    if let Some(&c) = b.first() {
        if c == b'+' || c == b'-' {
            i = 1;
        }
    }
    // `Infinity` (after an optional sign) → runtime `±Infinity` → decline.
    if s[i..].starts_with("Infinity") {
        return None;
    }

    let mut saw_digit = false;
    let mut j = i;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
        saw_digit = true;
    }
    if j < b.len() && b[j] == b'.' {
        j += 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return None; // no mantissa digit → NaN
    }
    // Optional exponent — consumed only when it carries at least one digit, so
    // `"1e"` parses as `1` with the `"e"` left as trailing garbage.
    if j < b.len() && (b[j] | 0x20) == b'e' {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let exp_digits_start = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k > exp_digits_start {
            j = k;
        }
    }

    // `&s[0..j]` keeps the leading sign. Normalise the one shape Rust's parser
    // rejects — a dot with no digit after it (`"5."`, `"5.e3"`) — by inserting
    // a `0`; a leading dot (`".5"`) Rust already accepts.
    let matched = &s[0..j];
    let normalised = matched.replace(".e", ".0e").replace(".E", ".0E");
    let normalised = if normalised.ends_with('.') {
        format!("{normalised}0")
    } else {
        normalised
    };
    let value: f64 = normalised.parse().ok()?;
    value.is_finite().then_some(value)
}

/// Is `body` (already stripped of any leading sign) a JavaScript
/// `StrDecimalLiteral` mantissa — i.e. the part of `Number("…")` that is plain
/// base-10? It must hold at least one digit, with the shape
///
/// ```text
///   DecimalDigits ( '.' DecimalDigits? )? ( [eE] [+-]? DecimalDigits )?
///   '.' DecimalDigits                      ( [eE] [+-]? DecimalDigits )?
/// ```
///
/// and NOTHING left over. Examples that pass: `"5"`, `"5."`, `".5"`, `"5.5"`,
/// `"5e3"`, `"5.e3"`, `"1E-9"`. Examples that fail (→ `NaN`, so we decline):
/// `"1,2"` (stray comma), `"1_000"` (underscore), `"abc"` (no digit), `"1e"`
/// (exponent with no digit), `"0x1F"` (the `x` is leftover — the hex form is
/// handled separately, before this is ever called).
///
/// We validate the shape ourselves rather than trust Rust's `f64` parser
/// because that parser ALSO accepts spellings JavaScript's `Number` rejects
/// (`"inf"`, `"nan"`), and we must never fold one of those into a literal.
fn is_js_decimal_literal(body: &str) -> bool {
    let b = body.as_bytes();
    let mut i = 0usize;
    let mut saw_digit = false;

    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return false; // a bare `"."`, `"e5"`, `""` — no mantissa digit
    }
    // Optional exponent — `[eE]`, optional sign, then ≥1 digit. Unlike
    // `parseFloat` (which would keep `"1"` and drop a digitless `"e"`), `Number`
    // requires the WHOLE string to parse, so `"1e"` is simply invalid here.
    if i < b.len() && (b[i] | 0x20) == b'e' {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return false; // `"1e"`, `"1e+"` — exponent marker without digits
        }
    }
    i == b.len() // reject any trailing garbage (`"1,2"`, `"5px"`, `"1_0"`)
}

/// Compute JavaScript's global `Number(string)` coercion at compile time
/// (ECMAScript §21.1.1.1 → §7.1.4.1.1 `StringToNumber`), or `None` when the
/// result is `NaN` or `±Infinity` (neither has a literal token to substitute).
///
/// Unlike `parseInt`/`parseFloat`, `Number` consumes the *entire* string after
/// trimming — a single stray character anywhere makes the whole thing `NaN`:
///
/// | call                  | value     | note                                  |
/// |-----------------------|-----------|---------------------------------------|
/// | `Number("42")`        | `42`      | plain decimal                         |
/// | `Number("")`          | `0`       | empty / all-whitespace → `+0`         |
/// | `Number("  3.5 ")`    | `3.5`     | surrounding whitespace is trimmed     |
/// | `Number("0x1F")`      | `31`      | hex — **no** sign permitted           |
/// | `Number("0b101")`     | `5`       | binary                                |
/// | `Number("0o17")`      | `15`      | octal                                 |
/// | `Number("017")`       | `17`      | leading zero is decimal, NOT octal    |
/// | `Number("abc")`       | `NaN`     | → decline                             |
/// | `Number("1,2")`       | `NaN`     | → decline                             |
/// | `Number("Infinity")`  | `∞`       | → decline (no literal)                |
///
/// SOUNDNESS: the trimmed set is exactly `is_js_trim_whitespace` (the engine's
/// `StrWhiteSpace`), so a successful trim never diverges from the runtime. For
/// the non-decimal `0x`/`0b`/`0o` forms we decline any value above `2^53`, where
/// an `f64` can no longer represent every integer exactly — so whatever literal
/// we emit is bit-identical to what the engine would compute.
fn fold_number(input: &str) -> Option<f64> {
    let s = input.trim_matches(is_js_trim_whitespace);

    // An empty or all-whitespace string coerces to `+0`, not `NaN`.
    if s.is_empty() {
        return Some(0.0);
    }

    // NonDecimalIntegerLiteral: `0x`/`0b`/`0o` (case-insensitive), no sign, and
    // at least one digit valid for the base. `u128::from_str_radix` would also
    // accept a leading `+`/`-`, so we screen the digits ourselves first — both
    // to reject signs (`Number("0x+1")` is `NaN`) and stray separators.
    let non_decimal = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .map(|d| (16u32, d))
        .or_else(|| {
            s.strip_prefix("0b")
                .or_else(|| s.strip_prefix("0B"))
                .map(|d| (2u32, d))
        })
        .or_else(|| {
            s.strip_prefix("0o")
                .or_else(|| s.strip_prefix("0O"))
                .map(|d| (8u32, d))
        });
    if let Some((radix, digits)) = non_decimal {
        if digits.is_empty() || !digits.bytes().all(|c| (c as char).is_digit(radix)) {
            return None;
        }
        let value = u128::from_str_radix(digits, radix).ok()?;
        if value > (1u128 << 53) {
            return None; // beyond exact f64 integer range — don't risk a mis-fold
        }
        return Some(value as f64);
    }

    // StrDecimalLiteral: an optional sign in front of a base-10 mantissa.
    // `Infinity`/`±Infinity` has no numeric literal, so it makes us decline.
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body == "Infinity" {
        return None;
    }
    if !is_js_decimal_literal(body) {
        return None;
    }
    // The shape is verified; normalise the two spellings Rust's `f64` parser
    // rejects — a bare trailing dot (`"5."`) and a dot right before the exponent
    // (`"5.e3"`) — then hand it to that correctly-rounded parser.
    let normalised = s.replace(".e", ".0e").replace(".E", ".0E");
    let normalised = if normalised.ends_with('.') {
        format!("{normalised}0")
    } else {
        normalised
    };
    let value: f64 = normalised.parse().ok()?;
    value.is_finite().then_some(value)
}

/// Coerce a string the way JavaScript's `ToNumber` does, returning the EXACT
/// `f64` — including `NaN` and `±Infinity` — rather than declining like
/// [`fold_number`]. This is the classifier behind `isNaN`/`isFinite` folding:
/// those predicates only ever read `.is_nan()` / `.is_finite()`, never emit the
/// number, so (unlike `fold_number`, which declines anything it cannot render as
/// an exact literal — `Infinity`, `NaN`, and integers beyond `2^53`) we can and
/// must return the real classification for every string.
///
/// The grammar mirrors `fold_number` exactly (same `is_js_trim_whitespace` trim,
/// same `0x`/`0b`/`0o` and decimal shapes via `is_js_decimal_literal`); the only
/// difference is that the non-finite results are kept rather than declined:
///
/// | input        | result      | isNaN | isFinite |
/// |--------------|-------------|-------|----------|
/// | `"42"`       | `42`        | false | true     |
/// | `""` / `" "` | `+0`        | false | true     |
/// | `"abc"`      | `NaN`       | true  | false    |
/// | `"1e3"`      | `1000`      | false | true     |
/// | `"Infinity"` | `+∞`        | false | false    |
/// | `"-Infinity"`| `-∞`        | false | false    |
/// | `"1e400"`    | `+∞`        | false | false    |
///
/// For the non-decimal forms we fold the digits into an `f64` accumulator (rather
/// than `fold_number`'s exact-`u128` path): we never emit the value, only its
/// finite-vs-infinite class, and the accumulator overflows to `Infinity` exactly
/// when the true number does — so the classification stays correct without
/// `fold_number`'s conservative `2^53` decline.
fn js_to_number(input: &str) -> f64 {
    let s = input.trim_matches(is_js_trim_whitespace);

    // An empty / all-whitespace string coerces to `+0`.
    if s.is_empty() {
        return 0.0;
    }

    // NonDecimalIntegerLiteral: `0x`/`0b`/`0o` (case-insensitive), no sign.
    let non_decimal = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .map(|d| (16u32, d))
        .or_else(|| {
            s.strip_prefix("0b")
                .or_else(|| s.strip_prefix("0B"))
                .map(|d| (2u32, d))
        })
        .or_else(|| {
            s.strip_prefix("0o")
                .or_else(|| s.strip_prefix("0O"))
                .map(|d| (8u32, d))
        });
    if let Some((radix, digits)) = non_decimal {
        if digits.is_empty() || !digits.bytes().all(|c| (c as char).is_digit(radix)) {
            return f64::NAN;
        }
        // Every byte is a validated base-`radix` digit, so `to_digit` is always
        // `Some`; `unwrap_or(0)` is a panic-free guard that is never taken.
        let mut acc = 0.0_f64;
        for c in digits.bytes() {
            acc = acc * radix as f64 + (c as char).to_digit(radix).unwrap_or(0) as f64;
        }
        return acc;
    }

    // StrDecimalLiteral: an optional sign in front of `Infinity` or a base-10
    // mantissa. A non-matching shape is `NaN` (ToNumber reads the WHOLE string).
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body == "Infinity" {
        return if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if !is_js_decimal_literal(body) {
        return f64::NAN;
    }
    // Same normalisation as `fold_number` for the two spellings Rust's parser
    // rejects (`"5."`, `"5.e3"`). Rust's `f64` parser is correctly rounded and
    // returns `Infinity` for an overflowing magnitude (`"1e400"`) — exactly the
    // engine's result — so a parse failure can only mean a shape bug; we fall
    // back to `NaN`, the safe (non-folding-divergent) classification.
    let normalised = s.replace(".e", ".0e").replace(".E", ".0E");
    let normalised = if normalised.ends_with('.') {
        format!("{normalised}0")
    } else {
        normalised
    };
    normalised.parse::<f64>().unwrap_or(f64::NAN)
}

fn format_js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if n == 0.0 {
        return "0".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    n.to_string()
}

// ---------------------------------------------------------------------
// Logical (short-circuit) — &&, ||, ??
// ---------------------------------------------------------------------

fn fold_logical(l: &LogicalExpression, st: &mut FoldState) -> Expression {
    let left = fold_expression(&l.left, st);
    let right = fold_expression(&l.right, st);

    // We only short-circuit when the LEFT side is a literal we can
    // judge for truthiness/nullishness. If left isn't a literal, we
    // can't fold (right might have side effects we can't elide).
    let left_truthy = literal_truthy(&left);
    let left_nullish = literal_nullish(&left);

    let kept = match (l.operator, left_truthy, left_nullish) {
        // `false && X → false`. Left wins, right is dropped.
        // `0 && X → 0`, `"" && X → ""`, `null && X → null`.
        (LogicalOperator::And, Some(false), _) => Some(Side::Left),
        // `true && X → X`. Right wins.
        (LogicalOperator::And, Some(true), _) => Some(Side::Right),
        // `true || X → true`. Left wins.
        (LogicalOperator::Or, Some(true), _) => Some(Side::Left),
        // `false || X → X`. Right wins.
        (LogicalOperator::Or, Some(false), _) => Some(Side::Right),
        // `null ?? X → X`, `undefined ?? X → X`. Right wins.
        (LogicalOperator::NullishCoalescing, _, Some(true)) => Some(Side::Right),
        // `0 ?? X → 0`, `"" ?? X → ""`. Left wins (not nullish).
        (LogicalOperator::NullishCoalescing, _, Some(false)) => Some(Side::Left),
        _ => None,
    };

    if let Some(side) = kept {
        let chosen = match side {
            Side::Left => left,
            Side::Right => right,
        };
        let parent = l.cv.clone();
        let before = format!(
            "({}) {} (...)",
            lit_label(&chosen),
            logical_op_label(l.operator)
        );
        let after = literal_label_for_expr(&chosen);
        // We mark `changed = true` regardless of tracing, but only
        // emit a contribution + derive a new cv when tracing is on.
        let _new_cv = st.fork_cv(&parent, &before, &after);
        // The chosen side keeps its own cv (it's the same node, just
        // promoted); we don't re-stamp it. Contribution records the
        // collapse.
        return chosen;
    }

    // Left-associativity normalization: `a op (b op c)` → `(a op b) op c` for
    // `&&` / `||`. Both operators are fully associative — the two groupings
    // yield the same value, short-circuit at the same point, and evaluate `a`,
    // `b`, `c` left-to-right in the same order — so the rewrite is behaviour
    // preserving. The payoff is byte-identity with the reference compiler: a
    // right-nested same-operator logical must be parenthesised on emit
    // (`a&&(b&&c)`), whereas the left-nested form prints bare (`a&&b&&c`).
    // Applied bottom-up under the pass's fixed-point iteration, a single
    // re-association step per node fully flattens arbitrarily deep right nests
    // (`a&&(b&&(c&&d))` → `a&&b&&c&&d`). `??` is intentionally excluded here (it
    // cannot legally mix with `&&`/`||` without parens, and is a separate case).
    let same_op_right_nest = matches!(l.operator, LogicalOperator::And | LogicalOperator::Or)
        && matches!(&right, Expression::LogicalExpression(r) if r.operator == l.operator);
    if same_op_right_nest {
        let Expression::LogicalExpression(r) = right else { unreachable!() };
        let new_cv = st.fork_cv(&l.cv, "a op (b op c)", "(a op b) op c");
        // `(a op b)` — the new left-nested inner node (synthetic, no cv).
        let inner = Expression::LogicalExpression(LogicalExpression {
            cv: None,
            operator: l.operator,
            left: Box::new(left),
            right: r.left,
        });
        return Expression::LogicalExpression(LogicalExpression {
            cv: new_cv,
            operator: l.operator,
            left: Box::new(inner),
            right: r.right,
        });
    }

    Expression::LogicalExpression(LogicalExpression {
        cv: l.cv.clone(),
        operator: l.operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

#[derive(Copy, Clone)]
enum Side {
    Left,
    Right,
}

// ---------------------------------------------------------------------
// Unary — !, -, +
// ---------------------------------------------------------------------

fn fold_unary(u: &UnaryExpression, st: &mut FoldState) -> Expression {
    let arg = fold_expression(&u.argument, st);

    let folded = match u.operator {
        UnaryOperator::Not => literal_truthy(&arg).map(|t| FoldedLiteral::Boolean(!t)),
        UnaryOperator::Negate => {
            if let Expression::NumericLiteral(n) = &arg {
                Some(FoldedLiteral::Number(-n.value))
            } else {
                None
            }
        }
        UnaryOperator::Plus => {
            // `+x` coerces to number. Fold for literals where coercion
            // is well-defined: NumericLiteral (identity),
            // BooleanLiteral (true → 1, false → 0), NullLiteral (→ 0),
            // StringLiteral that parses as number.
            match &arg {
                Expression::NumericLiteral(n) => Some(FoldedLiteral::Number(n.value)),
                Expression::BooleanLiteral(b) => {
                    Some(FoldedLiteral::Number(if b.value { 1.0 } else { 0.0 }))
                }
                Expression::NullLiteral(_) => Some(FoldedLiteral::Number(0.0)),
                Expression::StringLiteral(s) => {
                    // Empty string coerces to 0; strings that parse as
                    // numbers coerce to the value; otherwise NaN. Only
                    // fold for the unambiguous cases.
                    let t = s.value.trim();
                    if t.is_empty() {
                        Some(FoldedLiteral::Number(0.0))
                    } else if let Ok(n) = t.parse::<f64>() {
                        Some(FoldedLiteral::Number(n))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        UnaryOperator::TypeOf => {
            // `typeof <primitive literal>` → corresponding string,
            // per the ECMAScript §UnaryTypeofExpression table:
            //
            //   typeof <NumericLiteral>   →  "number"
            //   typeof <StringLiteral>    →  "string"
            //   typeof <BooleanLiteral>   →  "boolean"
            //   typeof <NullLiteral>      →  "object"   (the famous JS quirk)
            //   typeof <undefined>        →  "undefined" (gap-001: no
            //                                            UndefinedLiteral
            //                                            variant yet)
            //   typeof <BigIntLiteral>    →  "bigint"   (Phase 1.x;
            //                                            gap-021 closed
            //                                            in CLOC12.15)
            //   typeof <function expr>    →  "function" (Phase 1.x; not
            //                                            in v0.4.0's
            //                                            fold-set)
            //
            // Closes CLOC12 gap-005 for the four primitive-literal cases
            // we can already model. The identifier-typeof identity-fold
            // (`typeof a === typeof a` → `true`) is structurally a
            // *different* operation — it requires equality between two
            // syntactically-identical operands, not a value-substitution
            // fold. That part stays gated under the new gap-029.
            match &arg {
                Expression::NumericLiteral(_) => Some(FoldedLiteral::String("number".to_string())),
                Expression::StringLiteral(_) => Some(FoldedLiteral::String("string".to_string())),
                Expression::BooleanLiteral(_) => Some(FoldedLiteral::String("boolean".to_string())),
                Expression::NullLiteral(_) => Some(FoldedLiteral::String("object".to_string())),
                Expression::BigIntLiteral(_) => Some(FoldedLiteral::String("bigint".to_string())),
                Expression::UndefinedLiteral(_) => {
                    Some(FoldedLiteral::String("undefined".to_string()))
                }
                _ => None,
            }
        }
        UnaryOperator::Void => {
            // `void <pure-literal>` → `undefined`. CLOC12 gap-002.
            //
            // The general rule `void <expr> → undefined` is only sound
            // when `<expr>` has no observable side effects — otherwise
            // we'd silently drop the side effects.  For now we
            // conservatively fold only when the argument is a
            // primitive literal (or another folded literal we
            // already produced).  Identifiers, calls, member-access
            // are deliberately NOT folded — `void f()` must still
            // call `f`.
            //
            // The canonical case `void 0` (a Closure-Compiler-style
            // synonym for `undefined`) is now resolved.
            match &arg {
                Expression::NumericLiteral(_)
                | Expression::StringLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::BigIntLiteral(_)
                | Expression::UndefinedLiteral(_) => Some(FoldedLiteral::Undefined),
                _ => None,
            }
        }
        UnaryOperator::BitNot => {
            // `~<numeric literal>` → bitwise complement under ES `ToInt32`
            // semantics (ECMAScript §13.5.6 `BitwiseNOT`):
            //
            //   ~5      →  -6        (~ToInt32(5)  = ~5  = -6)
            //   ~-1     →   0        (~ToInt32(-1) = ~-1 =  0)
            //   ~5.9    →  -6        (truncate toward zero first → ~5)
            //   ~NaN    →  -1        (ToInt32(NaN) = 0, ~0 = -1)
            //   ~2.5e10 →  fold of the 32-bit-wrapped operand
            //
            // The binary bitwise operators (`&`, `|`, `^`) already fold via
            // [`to_int32`] (see `fold_binary`); the unary `~` was the lone
            // bitwise gap and reuses the very same coercion so the two stay
            // bit-for-bit consistent. Rust's prefix `!` on `i32` *is* the
            // two's-complement bitwise NOT, matching JS exactly. We fold only
            // a `NumericLiteral` argument — `~x` for an identifier or call
            // needs the runtime value, and `~"5"`/`~true` would require
            // string/boolean ToNumber coercion that the conservative fold-set
            // deliberately leaves to a later phase.
            if let Expression::NumericLiteral(n) = &arg {
                Some(FoldedLiteral::Number((!to_int32(n.value)) as f64))
            } else {
                None
            }
        }
        // Skipped: Delete (side effects).
        _ => None,
    };

    if let Some(value) = folded {
        let parent = u.cv.clone();
        let before = format!("{}({})", unary_op_label(u.operator), lit_label(&arg));
        let after = literal_label(&value);
        let new_cv = st.fork_cv(&parent, &before, &after);
        return stamp_literal_cv(value, new_cv);
    }

    // Negation push (upstream Closure's `PeepholeMinimizeConditions`):
    //   !(a == b)   →  a != b          !(a != b)   →  a == b
    //   !(a === b)  →  a !== b         !(a !== b)  →  a === b
    //
    // Sound for the four (in)equality operators ONLY, because `!=`/`!==` are
    // *defined* as the boolean negation of `==`/`===` (ECMAScript §13.10): both
    // `!` and these operators yield booleans, so the rewrite is value-identical
    // in every context. Relational operators (`<`/`<=`/`>`/`>=`) are deliberately
    // NOT inverted — `!(a < b)` is NOT `a >= b` when an operand is `NaN`
    // (`!(NaN < 1)` is `true` but `NaN >= 1` is `false`).
    if u.operator == UnaryOperator::Not {
        if let Expression::BinaryExpression(b) = &arg {
            if let Some(inverted) = invert_equality_operator(b.operator) {
                let parent = u.cv.clone();
                let before = format!("!(x {} y)", op_label(b.operator));
                let after = format!("x {} y", op_label(inverted));
                let new_cv = st.fork_cv(&parent, &before, &after);
                return Expression::BinaryExpression(BinaryExpression {
                    cv: new_cv,
                    operator: inverted,
                    left: b.left.clone(),
                    right: b.right.clone(),
                });
            }
        }

        // Idempotent double-negation collapse (upstream Closure's
        // `PeepholeMinimizeConditions`): `!!!x` → `!x`. A `!` whose operand is
        // itself a `!!y` (i.e. `Not(Not(y))`) drops that inner `!!` pair:
        //
        //   !!!x    →  !x        !!!!x   →  !!x       !!!!!x  →  !x
        //   !!x     →  !!x  (KEPT — a lone `!!` is the canonical boolean coercion)
        //
        // Sound for ANY operand — a getter, a call, `a+b` — with no side-effect
        // gate, because the operand is evaluated EXACTLY ONCE no matter how many
        // `!` wrap it (`!` never re-evaluates its operand, and `ToBoolean`
        // invokes no user coercion — unlike the `ToNumber`/`valueOf` reordering
        // that makes bitwise re-association unsound). `!!!x` computes
        // `¬¬¬ToBoolean(x)` = `¬ToBoolean(x)` = `!x`; three negations of a
        // boolean equal one. Folding is bottom-up, so `!!!!x`'s inner `!!!x`
        // collapses to `!x` first, then the outer `!` yields `!!x` — the
        // even/odd cascade converges in a single pass, matching Closure
        // byte-for-byte. A lone `!!y` is deliberately preserved: it is the
        // minified spelling of `Boolean(y)` and dropping it would change the
        // VALUE (`!!5` is `true`, `5` is `5`).
        if let Expression::UnaryExpression(inner) = &arg {
            if inner.operator == UnaryOperator::Not {
                if let Expression::UnaryExpression(inner2) = &*inner.argument {
                    if inner2.operator == UnaryOperator::Not {
                        // `arg` is `!!y`; the whole `!(!!y)` collapses to `!y`.
                        let parent = u.cv.clone();
                        let new_cv = st.fork_cv(&parent, "!!!x", "!x");
                        return Expression::UnaryExpression(UnaryExpression {
                            cv: new_cv,
                            operator: UnaryOperator::Not,
                            prefix: true,
                            argument: inner2.argument.clone(),
                        });
                    }
                }
            }
        }
    }

    Expression::UnaryExpression(UnaryExpression {
        cv: u.cv.clone(),
        operator: u.operator,
        prefix: u.prefix,
        argument: Box::new(arg),
    })
}

/// The negation of an (in)equality operator, or `None` for any operator whose
/// `!`-negation is NOT a single other operator. Only the four equality forms
/// qualify; relational operators do not (NaN breaks `!(a<b)` ≡ `a>=b`).
fn invert_equality_operator(op: BinaryOperator) -> Option<BinaryOperator> {
    match op {
        BinaryOperator::Eq => Some(BinaryOperator::NotEq),
        BinaryOperator::NotEq => Some(BinaryOperator::Eq),
        BinaryOperator::StrictEq => Some(BinaryOperator::StrictNotEq),
        BinaryOperator::StrictNotEq => Some(BinaryOperator::StrictEq),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Conditional — `test ? a : b`
// ---------------------------------------------------------------------

fn fold_conditional(c: &ConditionalExpression, st: &mut FoldState) -> Expression {
    let test = fold_expression(&c.test, st);
    let consequent = fold_expression(&c.consequent, st);
    let alternate = fold_expression(&c.alternate, st);

    if let Some(truthy) = literal_truthy(&test) {
        let chosen = if truthy { consequent } else { alternate };
        let parent = c.cv.clone();
        let before = format!("({}) ? ... : ...", lit_label(&test));
        let after = literal_label_for_expr(&chosen);
        let _new_cv = st.fork_cv(&parent, &before, &after);
        return chosen;
    }

    // Equal-branch collapse: `t ? X : X` → `X` when `t` is side-effect-free.
    // Both arms are the SAME expression, so the selected value is `X` no matter
    // which way `t` decides — the branch on `t` is dead. The ONE thing the
    // rewrite must not silently drop is `t`'s own evaluation, so we require `t`
    // to be side-effect-free (`is_side_effect_free`, the crate-wide contract:
    // an identifier / literal / member read is free — a getter on `a.p` is not
    // modelled, exactly as in `[a.p].length`; a call / assignment / `++` is
    // NOT). This mirrors the reference Closure Compiler's `PeepholeFoldConstants`
    // equal-branch case byte-for-byte (`a?b:b`→`b`, `a?1:1`→`1`, `a?b.c:b.c`→
    // `b.c`, `a?b():b()`→`b()`).
    //
    // When `t` IS impure, Closure instead rewrites to the comma sequence
    // `(t, X)` to preserve the effect (`f()?b:b`→`(f(),b)`, `(a=1)?b:b`→
    // `(a=1,b)`). That is a DIFFERENT, larger transform (it must build a
    // `SequenceExpression` and reason about the result position); we DECLINE it
    // here — leaving the impure-test ternary intact, which is sound — and file
    // it as a follow-up rather than ship a partial version.
    //
    // Branch equality uses derived structural `==`. In the default pipeline
    // every node carries `cv: None` (the bridge stamps `None`, and folding an
    // identifier/literal mints nothing), so `a?b:b`'s two `b`s compare equal.
    // Under `--correlation_vector` the two arms may carry distinct minted CVs;
    // then `==` is `false` and we conservatively DECLINE — a sound miss, never
    // a miscompile.
    if is_side_effect_free(&test) && consequent == alternate {
        let parent = c.cv.clone();
        let before = "t ? X : X".to_string();
        let after = "X".to_string();
        let _new_cv = st.fork_cv(&parent, &before, &after);
        return consequent;
    }

    Expression::ConditionalExpression(ConditionalExpression {
        cv: c.cv.clone(),
        test: Box::new(test),
        consequent: Box::new(consequent),
        alternate: Box::new(alternate),
    })
}

// =====================================================================
// Helpers — truthiness, nullishness, FoldedLiteral, label formatting
// =====================================================================

/// JS truthiness for the literals we know how to fold. Returns
/// `None` for non-literal expressions; the caller treats this as
/// "can't fold."
fn literal_truthy(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::BooleanLiteral(b) => Some(b.value),
        Expression::NumericLiteral(n) => Some(n.value != 0.0 && !n.value.is_nan()),
        Expression::StringLiteral(s) => Some(!s.value.is_empty()),
        Expression::NullLiteral(_) => Some(false),
        _ => None,
    }
}

/// JS nullishness — `null` and `undefined` only. We don't have an
/// undefined-literal node in Phase 1, so only NullLiteral qualifies.
fn literal_nullish(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::NullLiteral(_) => Some(true),
        Expression::BooleanLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_) => Some(false),
        _ => None,
    }
}

/// Evaluate `Math.round(x)` per ECMAScript §21.3.2.28. JS rounds half **toward
/// +Infinity** (`Math.round(2.5) === 3`, `Math.round(-2.5) === -2`), whereas
/// Rust's [`f64::round`] rounds half **away from zero** (`(-2.5).round() ==
/// -3.0`). The two agree everywhere EXCEPT at an exact `.5` fraction on a
/// negative value, so we special-case that: at a half, take the `+Inf`-ward
/// neighbour (`floor + 1`, i.e. `ceil` of a non-integer). For every other input
/// Rust's round-to-nearest already matches JS — including the fp-pathological
/// `0.49999999999999994`, which rounds to `0.0` in both. The `-0` result that
/// `Math.round(-0.5)` produces is filtered by the caller's negative-zero decline.
fn js_math_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x - x.floor() == 0.5 {
        x.floor() + 1.0
    } else {
        x.round()
    }
}

/// Evaluate `Math.max(values…)` per ECMAScript §21.3.2.24: the largest value,
/// with `+0` preferred over `-0`, and `NaN` if any input is NaN. (Our callers
/// only pass numeric literals, which are never NaN, but the NaN guard keeps the
/// helper a faithful standalone model.) We implement the signed-zero rule by
/// hand rather than using `f64::max`, whose zero handling we don't want to rely
/// on. The starting accumulator is `-Infinity` (the identity for max).
fn js_math_max(values: &[f64]) -> f64 {
    let mut acc = f64::NEG_INFINITY;
    for &v in values {
        if v.is_nan() {
            return f64::NAN;
        }
        // Take v when it is strictly larger, OR when both are zero and v is the
        // `+0` that max must prefer over a `-0` accumulator.
        if v > acc || (v == acc && v == 0.0 && v.is_sign_positive()) {
            acc = v;
        }
    }
    acc
}

/// Evaluate `Math.min(values…)` per ECMAScript §21.3.2.25: the smallest value,
/// with `-0` preferred over `+0`, and `NaN` if any input is NaN. Starting
/// accumulator is `+Infinity` (the identity for min).
fn js_math_min(values: &[f64]) -> f64 {
    let mut acc = f64::INFINITY;
    for &v in values {
        if v.is_nan() {
            return f64::NAN;
        }
        // Take v when it is strictly smaller, OR when both are zero and v is the
        // `-0` that min must prefer over a `+0` accumulator.
        if v < acc || (v == acc && v == 0.0 && v.is_sign_negative()) {
            acc = v;
        }
    }
    acc
}

/// Lower the properties of the object literal passed to `Object.entries` into a
/// list of `[key, value]` pair array-literal expressions, honouring the spec's
/// own-enumerable-string-key semantics. Returns `None` (decline the fold) the
/// instant any property fails the static-shape conditions documented at the call
/// site. On success the pairs are in source order with duplicate keys collapsed
/// (first position, last value).
fn fold_object_entries_pairs(properties: &[ObjectMember]) -> Option<Vec<Expression>> {
    // Parallel vectors in first-occurrence order; `keys` finds a duplicate so its
    // value can be overwritten in place (the object the literal builds keeps only
    // the last value under a repeated key).
    let mut keys: Vec<String> = Vec::new();
    let mut pairs: Vec<Expression> = Vec::new();
    for member in properties {
        // An object spread `...o` injects keys we cannot know statically, so any
        // spread makes the whole `Object.entries` result indeterminate — decline.
        let p = match member {
            ObjectMember::Property(p) => p,
            ObjectMember::Spread(_) => return None,
        };
        // Only plain data properties `k: v`. Getters/setters execute code,
        // methods are functions, and a computed key `[expr]: v` is unknown.
        if p.kind != PropertyKind::Init || p.method || p.computed {
            return None;
        }
        let key_string = match &p.key {
            PropertyKey::Identifier(id) => id.name.clone(),
            PropertyKey::StringLiteral(s) => s.value.clone(),
            PropertyKey::NumericLiteral(n) => format_js_number(n.value),
            PropertyKey::Expression(_) => return None, // computed
            // A private name is not a public property key — an object literal
            // can never hold one, and `Object.keys/entries` would not enumerate
            // it — so decline the fold (same as a computed key).
            PropertyKey::PrivateName(_) => return None,
        };
        // A non-computed `{__proto__: v}` is the §B.3.1 prototype setter, not an
        // own property, so `Object.entries` would not enumerate it — decline.
        if key_string == "__proto__" {
            return None;
        }
        // Integer-index keys enumerate first, in numeric order, ahead of the
        // source insertion order we emit — decline if any key is one.
        if is_array_index(&key_string) {
            return None;
        }
        // VALUE must be a primitive literal. A shorthand `{x}` has the non-literal
        // identifier `x` as its value and is declined here.
        let value: Expression = match &*p.value {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => (*p.value).clone(),
            _ => return None,
        };
        // An entry key is ALWAYS a string; emit a string literal.
        let key_raw = format!(
            "\"{}\"",
            key_string.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let key_expr = Expression::StringLiteral(StringLiteral {
            cv: None,
            value: key_string.clone(),
            raw: key_raw,
        });
        let pair = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(key_expr), Some(value)],
        });
        if let Some(pos) = keys.iter().position(|k| k == &key_string) {
            pairs[pos] = pair;
        } else {
            keys.push(key_string);
            pairs.push(pair);
        }
    }
    Some(pairs)
}

/// Lower the properties of the object literal passed to `Object.keys` into a list
/// of own-enumerable string-key literal expressions, honouring the spec's
/// own-enumerable-string-key semantics. Returns `None` (decline the fold) the
/// instant any property fails the static-shape conditions documented at the call
/// site. On success the keys are in source (enumeration) order with duplicate
/// keys collapsed to a single first-position entry.
///
/// The conditions match [`fold_object_entries_pairs`] exactly — see the call site
/// for why dropping the value does NOT loosen them: the value expression is still
/// evaluated when the source literal is built, so it must be a side-effect-free
/// primitive literal even though `Object.keys` never emits it.
fn fold_object_keys_names(properties: &[ObjectMember]) -> Option<Vec<Expression>> {
    // First-occurrence order; a duplicate key is found and ignored (the object the
    // literal builds keeps a single own property under a repeated key, and its
    // position is the first occurrence).
    let mut keys: Vec<String> = Vec::new();
    let mut names: Vec<Expression> = Vec::new();
    for member in properties {
        // A spread `...o` injects statically-unknown keys — decline the fold.
        let p = match member {
            ObjectMember::Property(p) => p,
            ObjectMember::Spread(_) => return None,
        };
        // Only plain data properties `k: v`. Getters/setters execute code,
        // methods are functions, and a computed key `[expr]: v` is unknown.
        if p.kind != PropertyKind::Init || p.method || p.computed {
            return None;
        }
        let key_string = match &p.key {
            PropertyKey::Identifier(id) => id.name.clone(),
            PropertyKey::StringLiteral(s) => s.value.clone(),
            PropertyKey::NumericLiteral(n) => format_js_number(n.value),
            PropertyKey::Expression(_) => return None, // computed
            // A private name is not a public property key — an object literal
            // can never hold one, and `Object.keys/entries` would not enumerate
            // it — so decline the fold (same as a computed key).
            PropertyKey::PrivateName(_) => return None,
        };
        // NOTE (previously: a `contains('\\')` decline guarded against escapes).
        // A `PropertyKey::StringLiteral`'s `value` now holds the DECODED property
        // name — the bridge runs `unquote_string` on every string key — so the
        // source key `"a\"b"` arrives here as the three characters `a"b`. The
        // re-escape below (`replace('\\', "\\\\").replace('"', "\\\"")`) turns
        // that back into the correct source form, so escaped keys fold soundly and
        // no longer need to be declined. This matches the sibling
        // `fold_object_entries_pairs`, which already had no such guard.
        // A non-computed `{__proto__: v}` is the §B.3.1 prototype setter, not an
        // own property, so `Object.keys` would not enumerate it — decline.
        if key_string == "__proto__" {
            return None;
        }
        // Integer-index keys enumerate first, in numeric order, ahead of the
        // source insertion order we emit — decline if any key is one.
        if is_array_index(&key_string) {
            return None;
        }
        // The VALUE is dropped, but evaluating the source literal still runs it, so
        // it must be a side-effect-free primitive literal (see call site). A
        // shorthand `{x}` has the non-literal identifier `x` as its value and is
        // declined here.
        match &*p.value {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => {}
            _ => return None,
        };
        // An own key is ALWAYS a string; emit a string literal.
        let key_raw = format!(
            "\"{}\"",
            key_string.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let key_expr = Expression::StringLiteral(StringLiteral {
            cv: None,
            value: key_string.clone(),
            raw: key_raw,
        });
        if let Some(pos) = keys.iter().position(|k| k == &key_string) {
            names[pos] = key_expr;
        } else {
            keys.push(key_string);
            names.push(key_expr);
        }
    }
    Some(names)
}

/// True when `s` is a canonical ECMAScript *array index*: a string for which
/// `ToString(ToUint32(s)) === s` and whose numeric value is in `[0, 2^32 − 2]`.
/// In the ASCII subset that means: all digits, no leading zero (except the single
/// character `"0"`), and a value strictly below `2^32 − 1`. Such keys are
/// enumerated ahead of ordinary string keys, so folds containing them are
/// declined to preserve ordering.
fn is_array_index(s: &str) -> bool {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if s.len() > 1 && s.as_bytes()[0] == b'0' {
        return false; // leading zero → not the canonical form
    }
    match s.parse::<u64>() {
        Ok(n) => n < u32::MAX as u64, // strictly below 2^32 − 1
        Err(_) => false,              // larger than u64 → not an index
    }
}

/// A folded value, kept separate from the AST node enum so the fold
/// logic can produce a value first and only later attach a fresh
/// CvId. Converted to an [`Expression`] via [`stamp_literal_cv`].
#[derive(Debug)]
enum FoldedLiteral {
    Number(f64),
    String(String),
    Boolean(bool),
    // Models the JS `null` value for completeness of the folded-literal domain.
    // No current fold produces it (null-valued expressions are left for the
    // runtime), but keeping the variant makes the value enum total.
    #[allow(dead_code)]
    Null,
    /// `undefined`. Produced by:
    /// - `void <any-expression-without-side-effects>` fold (CLOC12.20 / gap-002).
    /// - Future: identifier-reference to `undefined` in scopes that don't shadow it.
    Undefined,
}

fn stamp_literal_cv(v: FoldedLiteral, cv: Option<String>) -> Expression {
    match v {
        FoldedLiteral::Number(n) => Expression::NumericLiteral(NumericLiteral {
            cv,
            value: n,
            raw: format_js_number(n),
        }),
        FoldedLiteral::String(s) => {
            let raw = format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
            Expression::StringLiteral(StringLiteral { cv, value: s, raw })
        }
        FoldedLiteral::Boolean(b) => Expression::BooleanLiteral(BooleanLiteral { cv, value: b }),
        FoldedLiteral::Null => Expression::NullLiteral(NullLiteral { cv }),
        FoldedLiteral::Undefined => Expression::UndefinedLiteral(UndefinedLiteral { cv }),
    }
}

/// Attempt to lower the element list of the array passed to
/// `Object.fromEntries` into a list of object [`Property`] entries. Returns
/// `None` (decline the fold) the instant any element fails the static-shape
/// conditions documented at the call site. On success the returned properties
/// honour the spec's duplicate-key rule: first-occurrence POSITION, last VALUE.
fn fold_from_entries_pairs(elements: &[Option<Expression>]) -> Option<Vec<ObjectMember>> {
    // Parallel vectors in first-occurrence order; `keys` lets us find a
    // duplicate so its value can be overwritten in place (spec behaviour).
    let mut keys: Vec<String> = Vec::new();
    let mut props: Vec<ObjectMember> = Vec::new();
    for element in elements {
        // No outer hole — `[ , ["a", 1]]` is declined.
        let pair = element.as_ref()?;
        let Expression::ArrayExpression(pair) = pair else {
            return None; // an element that is not an array literal
        };
        // Exactly two PRESENT elements — `[k, v]`. A `[k]`, `[k, v, w]`, or a
        // pair with a hole (`[ , v]` / `[k, ]`) is declined.
        if pair.elements.len() != 2 {
            return None;
        }
        let key_expr = pair.elements[0].as_ref()?; // no hole in key slot
        let value_expr = pair.elements[1].as_ref()?; // no hole in value slot
        // KEY must be a string or numeric literal; a numeric key folds to its
        // ECMAScript ToString (e.g. `1` → "1", matching ToPropertyKey).
        let key_string = match key_expr {
            Expression::StringLiteral(s) => s.value.clone(),
            Expression::NumericLiteral(n) => format_js_number(n.value),
            _ => return None,
        };
        // CRITICAL — decline `__proto__`. `Object.fromEntries` calls
        // CreateDataPropertyOnObject, which makes an OWN enumerable property
        // literally named "__proto__" and does NOT touch the prototype. But in
        // an object literal a non-computed `__proto__:` (bare OR quoted) is the
        // §B.3.1 prototype SETTER — it changes `[[Prototype]]` and creates no
        // own property (and `{__proto__: null}` yields a null-prototype object).
        // Folding here would silently change semantics, so we decline; the call
        // is left intact. (A numeric key can never ToString to "__proto__", so
        // only the string-key path can reach this.)
        if key_string == "__proto__" {
            return None;
        }
        // VALUE must be a primitive literal — anything else is declined.
        let value: Expression = match value_expr {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => value_expr.clone(),
            _ => return None,
        };
        let property = ObjectMember::Property(Property {
            cv: None,
            kind: PropertyKind::Init,
            key: property_key_for(&key_string),
            value: Box::new(value),
            computed: false,
            shorthand: false,
            method: false,
        });
        // Duplicate key → keep first POSITION, take last VALUE.
        if let Some(pos) = keys.iter().position(|k| k == &key_string) {
            props[pos] = property;
        } else {
            keys.push(key_string);
            props.push(property);
        }
    }
    Some(props)
}

/// Build the [`PropertyKey`] for a known key string: a bare identifier when the
/// string is a valid ECMAScript identifier name (so `{a: 1}`), otherwise a
/// quoted string literal (so `{"1": "x"}` or `{"a-b": 1}`). Both encode the
/// same own-property key; the identifier form is just shorter.
fn property_key_for(key: &str) -> PropertyKey {
    if is_identifier_name(key) {
        PropertyKey::Identifier(Identifier {
            cv: None,
            name: key.to_string(),
        })
    } else {
        let raw = format!("\"{}\"", key.replace('\\', "\\\\").replace('"', "\\\""));
        PropertyKey::StringLiteral(StringLiteral {
            cv: None,
            value: key.to_string(),
            raw,
        })
    }
}

/// True when `s` is a valid ECMAScript identifier *name* in the ASCII subset:
/// a leading `A–Z a–z _ $` followed by zero or more `A–Z a–z 0–9 _ $`. We stay
/// ASCII-only on purpose — a Unicode identifier key is always sound to emit as a
/// string literal, and reserved words ARE legal as property names (`{if: 1}`),
/// so they are intentionally NOT excluded here.
fn is_identifier_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

// Pretty-printing for Contribution.meta.
fn lit_label(expr: &Expression) -> String {
    match expr {
        Expression::NumericLiteral(n) => format_js_number(n.value),
        Expression::StringLiteral(s) => format!("\"{}\"", s.value),
        Expression::BooleanLiteral(b) => if b.value { "true" } else { "false" }.to_string(),
        Expression::NullLiteral(_) => "null".to_string(),
        Expression::Identifier(i) => i.name.clone(),
        _ => "<expr>".to_string(),
    }
}

fn literal_label(v: &FoldedLiteral) -> String {
    match v {
        FoldedLiteral::Number(n) => format_js_number(*n),
        FoldedLiteral::String(s) => format!("\"{}\"", s),
        FoldedLiteral::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        FoldedLiteral::Null => "null".to_string(),
        FoldedLiteral::Undefined => "undefined".to_string(),
    }
}

fn literal_label_for_expr(expr: &Expression) -> String {
    lit_label(expr)
}

fn op_label(op: BinaryOperator) -> &'static str {
    use BinaryOperator::*;
    match op {
        Eq => "==",
        NotEq => "!=",
        StrictEq => "===",
        StrictNotEq => "!==",
        Lt => "<",
        LtEq => "<=",
        Gt => ">",
        GtEq => ">=",
        LeftShift => "<<",
        RightShift => ">>",
        UnsignedRightShift => ">>>",
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Mod => "%",
        Exp => "**",
        BitOr => "|",
        BitXor => "^",
        BitAnd => "&",
        In => "in",
        InstanceOf => "instanceof",
    }
}

fn logical_op_label(op: LogicalOperator) -> &'static str {
    match op {
        LogicalOperator::And => "&&",
        LogicalOperator::Or => "||",
        LogicalOperator::NullishCoalescing => "??",
    }
}

fn unary_op_label(op: UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Negate => "-",
        UnaryOperator::Plus => "+",
        UnaryOperator::Not => "!",
        UnaryOperator::BitNot => "~",
        UnaryOperator::TypeOf => "typeof ",
        UnaryOperator::Void => "void ",
        UnaryOperator::Delete => "delete ",
    }
}

#[cfg(test)]
mod tests {
    // These tests exercise constant-folding of `parseFloat`/number literals, so
    // literals like `3.14` are deliberate test inputs/expected values, not
    // approximations of std::f64::consts::PI to be replaced.
    #![allow(clippy::approx_constant)]
    use super::*;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_javascript_ast::{statement::TaggedStatement, Identifier, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    fn untraced_program() -> Program {
        Program::new_untraced(EsVersion::Es2025, SourceType::Module)
    }

    fn num(v: f64, cv: Option<&str>) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v,
            raw: v.to_string(),
        })
    }
    fn boolean(v: bool, cv: Option<&str>) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v,
        })
    }
    fn string(v: &str, cv: Option<&str>) -> Expression {
        Expression::StringLiteral(StringLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v.to_string(),
            raw: format!("\"{}\"", v),
        })
    }
    fn null(cv: Option<&str>) -> Expression {
        Expression::NullLiteral(NullLiteral {
            cv: cv.map(|s| s.to_string()),
        })
    }
    fn ident(name: &str) -> Expression {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    }

    fn run_pass(prog: Program) -> (Program, Vec<Contribution>, bool, u32) {
        let pass = ConstantFoldPass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let ctx = PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        let out = pass.run(ctx).expect("pass should succeed");
        (
            out.program,
            out.contributions,
            out.changed,
            out.stats.nodes_touched,
        )
    }

    /// Build a Program whose body is a single ExpressionStatement
    /// wrapping the given expression.
    fn program_with_expr(expr: Expression, traced: bool) -> Program {
        let p = if traced {
            program()
        } else {
            untraced_program()
        };
        p.with_body(vec![ProgramItem::Statement(
            Statement::expression_statement(ExpressionStatement {
                cv: if traced {
                    Some("es.1".to_string())
                } else {
                    None
                },
                expression: expr,
            }),
        )])
    }

    /// CLOC12.191 PR1: a default-parameter's `right` expression must fold. The
    /// pass previously cloned parameter lists verbatim (rest-params carry only a
    /// name); a default carries *live code*, so `function f(a = 2 + 3){}` has to
    /// shrink to `function f(a = 5){}` — the same fold a function body gets.
    #[test]
    fn folds_default_parameter_expression() {
        let default_expr = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0, None)),
            right: Box::new(num(3.0, None)),
        });
        let fd = FunctionDeclaration {
            cv: None,
            id: Identifier { cv: None, name: "f".to_string() },
            params: vec![FunctionParam::AssignmentPattern(AssignmentPattern {
                cv: None,
                left: Identifier { cv: None, name: "a".to_string() },
                right: default_expr,
            })],
            body: BlockStatement { cv: None, body: vec![] },
            generator: false,
            is_async: false,
        };
        let prog = untraced_program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(fd),
        )]);
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed, "folding a default expression should mark the program changed");
        let ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) = &out.body[0] else {
            panic!("expected a function declaration back");
        };
        match f.params[0].default_value() {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 5.0),
            other => panic!("expected the default to fold to 5; got {other:?}"),
        }
    }

    /// Extract the (folded) expression from a Program whose body is a
    /// single ExpressionStatement — for assertion convenience.
    fn extract_expr(prog: &Program) -> &Expression {
        let item = &prog.body[0];
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(es))) =
            item
        else {
            panic!("expected a single expression statement, got {:?}", item);
        };
        &es.expression
    }

    // ------------------- deep-chain DoS regression -------------------

    /// A very deep left-nested operator chain — the shape the bridge builds
    /// for flat source like `1+1+…+1` (tens of thousands of terms) — must fold
    /// without overflowing the native stack. The bottom-up `fold_binary` walk
    /// recurses once per operator; on the caller's ordinary ~2 MiB stack this
    /// used to overflow (an uncatchable abort). The large-stack worker
    /// (`FOLD_STACK_SIZE`) absorbs the recursion and still folds the whole
    /// chain to a single number.
    #[test]
    fn deeply_nested_binary_chain_folds_without_stack_overflow() {
        const N: usize = 20_000;
        let mut expr = num(1.0, None);
        for _ in 0..N {
            expr = Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(expr),
                right: Box::new(num(1.0, None)),
            });
        }
        let prog = program_with_expr(expr, false);
        // `fold_program` runs its recursion on the 64 MiB `FOLD_STACK_SIZE`
        // worker, so this depth folds fine even though the test runs on cargo's
        // ~2 MiB thread. Without the worker, `fold_binary`'s per-operator
        // recursion overflows here — so a regression re-breaks this test. The
        // 20 000-deep *input* AST's own recursive `Drop` would ALSO overflow
        // this small thread (orthogonal), so we run the pass by reference and
        // `forget` the input; the shallow folded output drops fine.
        let pass = ConstantFoldPass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("pass should succeed");
        assert!(out.changed, "the deep chain must fold");
        match extract_expr(&out.program) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, (N + 1) as f64),
            other => panic!("expected a single folded number, got {other:?}"),
        }
        std::mem::forget(prog);
    }

    // ------------------- metadata + identity tests -------------------

    #[test]
    fn name_is_constant_fold() {
        assert_eq!(ConstantFoldPass::new().name(), "constant-fold");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            ConstantFoldPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_two_pass_units() {
        assert_eq!(ConstantFoldPass::new().cost(), 2);
    }

    // --------- standard-constructor `new`-drop (Error / Object / Array) ---------

    fn new_expr(callee: Expression, args: Vec<Expression>) -> Expression {
        // A traced `cv` so the fold records a provenance contribution (matching
        // the bridge, which stamps every node) — `fork_cv` no-ops on `None`.
        Expression::NewExpression(NewExpression {
            cv: Some("new.1".to_string()),
            callee: Box::new(callee),
            arguments: args,
        })
    }
    fn member_expr(object: Expression, prop: &str) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(object),
            property: Box::new(ident(prop)),
            computed: false,
        })
    }

    /// `new Error("x")` → `Error("x")` — the `new` is dropped to a plain call.
    #[test]
    fn new_error_with_arg_drops_new() {
        let expr = new_expr(ident("Error"), vec![string("x", None)]);
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "folded"));
        match extract_expr(&out) {
            Expression::CallExpression(c) => {
                assert!(matches!(c.callee.as_ref(), Expression::Identifier(id) if id.name == "Error"));
                assert_eq!(c.arguments.len(), 1);
            }
            other => panic!("expected a plain call `Error(\"x\")`; got {other:?}"),
        }
    }

    /// `new Error` (no args) → `Error()` — still a call, with an empty arg list.
    #[test]
    fn new_error_no_args_drops_new() {
        let expr = new_expr(ident("Error"), vec![]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::CallExpression(c) => {
                assert!(matches!(c.callee.as_ref(), Expression::Identifier(id) if id.name == "Error"));
                assert!(c.arguments.is_empty());
            }
            other => panic!("expected `Error()`; got {other:?}"),
        }
    }

    /// `new TypeError(x)` is LEFT ALONE — the reference compiler folds only
    /// `Error`, not its subtypes.
    #[test]
    fn new_typeerror_subtype_is_not_dropped() {
        let expr = new_expr(ident("TypeError"), vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "a subtype constructor must not fold");
        assert!(matches!(extract_expr(&out), Expression::NewExpression(_)));
    }

    /// `new obj.Error(x)` is LEFT ALONE — the gate requires a BARE `Error`
    /// identifier callee, not a member access.
    #[test]
    fn new_member_error_callee_is_not_dropped() {
        let expr = new_expr(member_expr(ident("obj"), "Error"), vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "a member `obj.Error` callee must not fold");
        assert!(matches!(extract_expr(&out), Expression::NewExpression(_)));
    }

    /// `new Object(x)` → `Object(x)` — a plain call.
    #[test]
    fn new_object_with_arg_drops_to_call() {
        let expr = new_expr(ident("Object"), vec![ident("x")]);
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "folded"));
        match extract_expr(&out) {
            Expression::CallExpression(c) => {
                assert!(matches!(c.callee.as_ref(), Expression::Identifier(id) if id.name == "Object"));
                assert_eq!(c.arguments.len(), 1);
            }
            other => panic!("expected `Object(x)`; got {other:?}"),
        }
    }

    /// `new Object()` → `{}` — an empty object literal.
    #[test]
    fn new_object_no_args_to_empty_object_literal() {
        let expr = new_expr(ident("Object"), vec![]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => assert!(o.properties.is_empty()),
            other => panic!("expected `{{}}`; got {other:?}"),
        }
    }

    /// `new Array(x)` → `Array(x)` — a lone argument is a length, so the call
    /// form is kept (NOT `[x]`).
    #[test]
    fn new_array_one_arg_keeps_call() {
        let expr = new_expr(ident("Array"), vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::CallExpression(c) => {
                assert!(matches!(c.callee.as_ref(), Expression::Identifier(id) if id.name == "Array"));
                assert_eq!(c.arguments.len(), 1);
            }
            other => panic!("expected `Array(x)`; got {other:?}"),
        }
    }

    /// `new Array()` → `[]` — an empty array literal.
    #[test]
    fn new_array_no_args_to_empty_array_literal() {
        let expr = new_expr(ident("Array"), vec![]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => assert!(a.elements.is_empty()),
            other => panic!("expected `[]`; got {other:?}"),
        }
    }

    /// `new Array(1, 2)` → `[1, 2]` — 2+ args become an array literal.
    #[test]
    fn new_array_multi_args_to_array_literal() {
        let expr = new_expr(ident("Array"), vec![num(1.0, None), num(2.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2);
                assert!(a.elements.iter().all(|e| e.is_some()));
            }
            other => panic!("expected `[1,2]`; got {other:?}"),
        }
    }

    /// `new Array(a, ...xs)` keeps the CALL form `Array(a, ...xs)` — NOT the
    /// array literal `[a, ...xs]`, which would be a miscompile when the spread
    /// expands to zero elements (`new Array(5, ...[])` is a length-5 array, but
    /// `[5, ...[]]` is `[5]`). The call form is always equivalent.
    #[test]
    fn new_array_multi_args_with_spread_keeps_call() {
        let spread = Expression::SpreadElement(SpreadElement {
            cv: None,
            argument: Box::new(ident("xs")),
        });
        let expr = new_expr(ident("Array"), vec![ident("a"), spread]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::CallExpression(c) => {
                assert!(matches!(c.callee.as_ref(), Expression::Identifier(id) if id.name == "Array"));
                assert_eq!(c.arguments.len(), 2);
            }
            other => panic!("expected the call form `Array(a,...xs)`; got {other:?}"),
        }
    }

    /// `new obj.Array(x)` is LEFT ALONE — a member callee is not the global.
    #[test]
    fn new_member_array_callee_not_folded() {
        let expr = new_expr(member_expr(ident("obj"), "Array"), vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "a member `obj.Array` callee must not fold");
        assert!(matches!(extract_expr(&out), Expression::NewExpression(_)));
    }

    /// `new Foo(x)` (a user constructor) is LEFT ALONE.
    #[test]
    fn new_user_ctor_not_folded() {
        let expr = new_expr(ident("Foo"), vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "a user constructor must not fold");
        assert!(matches!(extract_expr(&out), Expression::NewExpression(_)));
    }

    #[test]
    fn no_depends_on_or_invalidates_in_v1() {
        let p = ConstantFoldPass::new();
        assert!(p.depends_on().is_empty());
        assert!(p.invalidates().is_empty());
    }

    #[test]
    fn empty_program_is_identity() {
        let (out, contribs, changed, nodes) = run_pass(program());
        assert_eq!(out.cv, Some("prog.1".to_string()));
        assert!(!changed);
        assert!(contribs.is_empty());
        // Visited the Program root and no body items.
        assert_eq!(nodes, 1);
    }

    // ------------------- numeric arithmetic --------------------------

    #[test]
    fn fold_addition_of_two_numbers() {
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0, Some("n.l"))),
            right: Box::new(num(3.0, Some("n.r"))),
        });
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].source, "constant-fold");
        assert_eq!(contribs[0].tag, "folded");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => {
                assert_eq!(n.value, 5.0);
                assert!(n.cv.is_some(), "traced fold gives Some cv");
            }
            other => panic!("expected NumericLiteral(5); got {:?}", other),
        }
    }

    #[test]
    fn fold_subtraction_multiplication_division_modulo_exponentiation() {
        // (10 - 3) * 2 / 4 % 3 ** 2 = 14 / 4 % 9 = 3.5 % 9 = 3.5
        // We just verify each op folds individually in this test.
        for (op, expected) in [
            (BinaryOperator::Sub, 1.0),
            (BinaryOperator::Mul, 6.0),
            (BinaryOperator::Div, 1.5),
            (BinaryOperator::Mod, 1.0),
            (BinaryOperator::Exp, 9.0),
        ] {
            let (a, b) = match op {
                BinaryOperator::Mod => (3.0, 2.0),
                _ => (3.0, 2.0),
            };
            let _ = a;
            let _ = b;
            // 3 op 2 = expected
            let expr = Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: op,
                left: Box::new(num(3.0, None)),
                right: Box::new(num(2.0, None)),
            });
            let (out, _, _, _) = run_pass(program_with_expr(expr, true));
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expected, "op {:?}", op),
                other => panic!("expected NumericLiteral; got {:?} for op {:?}", other, op),
            }
        }
    }

    #[test]
    fn division_and_modulo_by_zero_are_not_folded() {
        // `x / 0` and `x % 0` produce ±Infinity / NaN — Closure keeps the source
        // op rather than emit the shadowable global, and so do we. The binary
        // node must SURVIVE unfolded.
        for (op, l, r) in [
            (BinaryOperator::Div, 1.0, 0.0),   // 1/0  → Infinity  (kept)
            (BinaryOperator::Div, -1.0, 0.0),  // -1/0 → -Infinity (kept)
            (BinaryOperator::Div, 0.0, 0.0),   // 0/0  → NaN       (kept)
            (BinaryOperator::Mod, 1.0, 0.0),   // 1%0  → NaN       (kept)
        ] {
            let expr = Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: op,
                left: Box::new(num(l, None)),
                right: Box::new(num(0.0, None)),
            });
            let _ = r;
            let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
            assert!(!changed, "{l} {op:?} 0 must NOT fold");
            match extract_expr(&out) {
                Expression::BinaryExpression(b) => {
                    assert_eq!(b.operator, op);
                    assert!(matches!(&*b.right, Expression::NumericLiteral(n) if n.value == 0.0));
                }
                other => panic!("expected the binary op to survive; got {other:?}"),
            }
        }
    }

    #[test]
    fn division_by_nonzero_still_folds() {
        // The guard is scoped to a ZERO divisor: `6/3`→2 and `5/2`→2.5 still fold.
        for (l, r, expected) in [(6.0, 3.0, 2.0), (5.0, 2.0, 2.5), (1.0, 8.0, 0.125)] {
            let expr = Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: BinaryOperator::Div,
                left: Box::new(num(l, None)),
                right: Box::new(num(r, None)),
            });
            let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
            assert!(changed, "{l}/{r} should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expected, "{l}/{r}"),
                other => panic!("expected NumericLiteral({expected}); got {other:?}"),
            }
        }
    }

    #[test]
    fn fold_nested_arithmetic_in_one_pass() {
        // 1 + (2 * 3) → 1 + 6 → 7 — proves the bottom-up walk
        // collapses chains in a single iteration.
        let inner = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.inner".to_string()),
            operator: BinaryOperator::Mul,
            left: Box::new(num(2.0, None)),
            right: Box::new(num(3.0, None)),
        });
        let outer = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.outer".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(num(1.0, None)),
            right: Box::new(inner),
        });
        let (out, contribs, _, _) = run_pass(program_with_expr(outer, true));
        // Two folds: inner (2*3) then outer (1+6).
        assert_eq!(contribs.len(), 2);
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 7.0),
            other => panic!("expected NumericLiteral(7); got {:?}", other),
        }
    }

    // ------------------- comparison ----------------------------------

    #[test]
    fn fold_numeric_comparisons() {
        for (op, a, b, expected) in [
            (BinaryOperator::Lt, 1.0, 2.0, true),
            (BinaryOperator::Gt, 1.0, 2.0, false),
            (BinaryOperator::LtEq, 2.0, 2.0, true),
            (BinaryOperator::GtEq, 2.0, 2.0, true),
            (BinaryOperator::Eq, 1.0, 1.0, true),
            (BinaryOperator::StrictEq, 1.0, 2.0, false),
            (BinaryOperator::NotEq, 1.0, 2.0, true),
            (BinaryOperator::StrictNotEq, 1.0, 1.0, false),
        ] {
            let expr = Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: op,
                left: Box::new(num(a, None)),
                right: Box::new(num(b, None)),
            });
            let (out, _, _, _) = run_pass(program_with_expr(expr, true));
            match extract_expr(&out) {
                Expression::BooleanLiteral(bl) => {
                    assert_eq!(bl.value, expected, "op {:?} a={} b={}", op, a, b)
                }
                other => panic!("expected BooleanLiteral; got {:?}", other),
            }
        }
    }

    // ------------------- Bitwise / shift on numeric literals (CLOC15.D) -----

    #[test]
    fn to_int32_and_to_uint32_match_es_semantics() {
        // ToInt32: truncate, mod 2^32, reinterpret signed.
        assert_eq!(to_int32(5.9), 5);
        assert_eq!(to_int32(-1.0), -1);
        assert_eq!(to_int32(4_294_967_296.0), 0); // 2^32 wraps to 0
        assert_eq!(to_int32(3_000_000_000.0), -1_294_967_296); // >= 2^31 → negative
        assert_eq!(to_int32(f64::NAN), 0);
        assert_eq!(to_int32(f64::INFINITY), 0);
        assert_eq!(to_int32(-0.0), 0);
        // ToUint32: same but unsigned residue.
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
        assert_eq!(to_uint32(3_000_000_000.0), 3_000_000_000);
        assert_eq!(to_uint32(f64::NEG_INFINITY), 0);
    }

    #[test]
    fn fold_bitwise_and_shift_on_numeric_literals() {
        // (op, a, b, expected) — `expected` is the exact JS runtime value.
        for (op, a, b, expected) in [
            (BinaryOperator::BitAnd, 5.0, 3.0, 1.0),
            (BinaryOperator::BitOr, 5.0, 2.0, 7.0),
            (BinaryOperator::BitXor, 5.0, 1.0, 4.0),
            // ToInt32 coercion of a fractional operand: 5 & ToInt32(2.5)=2 → 0.
            (BinaryOperator::BitAnd, 5.0, 2.5, 0.0),
            // ToInt32 wraps a value >= 2^31 to negative: 3e9 | 0 = -1294967296.
            (
                BinaryOperator::BitOr,
                3_000_000_000.0,
                0.0,
                -1_294_967_296.0,
            ),
            (BinaryOperator::LeftShift, 1.0, 4.0, 16.0),
            // Shift count is masked to 5 bits: 1 << 32 == 1 << 0 == 1.
            (BinaryOperator::LeftShift, 1.0, 32.0, 1.0),
            (BinaryOperator::LeftShift, 1.0, 33.0, 2.0),
            // `>>` is arithmetic (sign-propagating): -8 >> 1 = -4.
            (BinaryOperator::RightShift, -8.0, 1.0, -4.0),
            // `>>>` is logical and unsigned: -1 >>> 0 = 2^32 - 1.
            (
                BinaryOperator::UnsignedRightShift,
                -1.0,
                0.0,
                4_294_967_295.0,
            ),
            (BinaryOperator::UnsignedRightShift, 256.0, 4.0, 16.0),
        ] {
            match try_fold_binary_op(op, &num(a, None), &num(b, None)) {
                Some(FoldedLiteral::Number(got)) => {
                    assert_eq!(got, expected, "op {:?} a={} b={}", op, a, b)
                }
                other => panic!(
                    "op {:?} a={} b={}: expected Number({}); got {:?}",
                    op, a, b, expected, other
                ),
            }
        }
    }

    #[test]
    fn fold_bitwise_end_to_end_renders_unsigned_result() {
        // Full pass: `-1 >>> 0` must emit the unsigned value `4294967295`,
        // not `-1`. Confirms the emitter renders a > i32::MAX Number too.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::UnsignedRightShift,
            left: Box::new(num(-1.0, None)),
            right: Box::new(num(0.0, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(nl) => assert_eq!(nl.value, 4_294_967_295.0),
            other => panic!("expected NumericLiteral; got {:?}", other),
        }
    }

    // ------------------- Number/String cross-type (gap-004) ----------
    //
    // These tests pin the gap-004 behaviour: §IsLooselyEqual and
    // §IsLessThan now coerce a string operand against a numeric one
    // for compile-time-known string literals that fall in the strict
    // recognised subset (see `js_string_to_number_strict`'s doc).

    #[test]
    fn gap004_jstr2num_simple_decimal_cases() {
        // Pin the helper's behaviour directly — the doc comment lists
        // these as recognised cases.
        assert_eq!(js_string_to_number_strict(""), Some(0.0));
        assert_eq!(js_string_to_number_strict("   "), Some(0.0));
        assert_eq!(js_string_to_number_strict("\t\n"), Some(0.0));
        assert_eq!(js_string_to_number_strict("0"), Some(0.0));
        assert_eq!(js_string_to_number_strict("1"), Some(1.0));
        assert_eq!(js_string_to_number_strict("-2"), Some(-2.0));
        assert_eq!(js_string_to_number_strict("1.5"), Some(1.5));
        assert_eq!(js_string_to_number_strict(".5"), Some(0.5));
        assert_eq!(js_string_to_number_strict("1e3"), Some(1000.0));
        assert_eq!(js_string_to_number_strict("1.5e-1"), Some(0.15));
        assert_eq!(js_string_to_number_strict("  42  "), Some(42.0)); // trim
    }

    #[test]
    fn gap004_jstr2num_explicit_infinity() {
        assert_eq!(js_string_to_number_strict("Infinity"), Some(f64::INFINITY));
        assert_eq!(js_string_to_number_strict("+Infinity"), Some(f64::INFINITY));
        assert_eq!(
            js_string_to_number_strict("-Infinity"),
            Some(f64::NEG_INFINITY)
        );
        // JS is case-sensitive — these must NOT match Infinity.
        assert_eq!(js_string_to_number_strict("infinity"), None);
        assert_eq!(js_string_to_number_strict("INFINITY"), None);
        assert_eq!(js_string_to_number_strict("inf"), None); // Rust accepts; JS doesn't.
    }

    #[test]
    fn gap004_jstr2num_rejects_ambiguous_or_unsupported() {
        // Hex/binary/octal — JS accepts but we bail (follow-up).
        assert_eq!(js_string_to_number_strict("0x1A"), None);
        assert_eq!(js_string_to_number_strict("0b101"), None);
        assert_eq!(js_string_to_number_strict("0o17"), None);
        // Non-numeric strings.
        assert_eq!(js_string_to_number_strict("hello"), None);
        assert_eq!(js_string_to_number_strict("1abc"), None);
        // Lone signs / dots.
        assert_eq!(js_string_to_number_strict("+"), None);
        assert_eq!(js_string_to_number_strict("-"), None);
        assert_eq!(js_string_to_number_strict("."), None);
        assert_eq!(js_string_to_number_strict("e"), None);
        // Malformed exponent — Rust's parser rejects.
        assert_eq!(js_string_to_number_strict("1e"), None);
        assert_eq!(js_string_to_number_strict("e5"), None);
    }

    #[test]
    fn gap004_number_string_equality_upstream_cases() {
        // Direct pin of upstream's test_number_string_comparison lines.
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::Lt,
                num(1.0, None),
                string("2", None)
            )),
            Some(true),
            "1 < '2' should fold to true"
        );
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::Eq,
                num(1.0, None),
                string("2", None)
            )),
            Some(false),
            "1 == '2' should fold to false"
        );
    }

    #[test]
    fn gap004_number_string_is_symmetric_and_order_preserving() {
        // String-on-left must yield the correct ordering: '2' < 1 is false
        // (2 < 1), not true (the left-side coercion preserves operand order).
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::Lt,
                string("2", None),
                num(1.0, None)
            )),
            Some(false),
            "'2' < 1 must fold to false (NOT swap to 1 < 2)"
        );
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::Gt,
                string("2", None),
                num(1.0, None)
            )),
            Some(true),
            "'2' > 1 must fold to true"
        );
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::Eq,
                string("1", None),
                num(1.0, None)
            )),
            Some(true),
            "'1' == 1 must fold to true (loose equality coerces)"
        );
    }

    #[test]
    fn gap004_number_string_relational_full_truth_table() {
        // For two values that disagree numerically, each of the 6 relations
        // gives a determined result.
        for (op, expected) in [
            (BinaryOperator::Lt, true),
            (BinaryOperator::LtEq, true),
            (BinaryOperator::Gt, false),
            (BinaryOperator::GtEq, false),
            (BinaryOperator::Eq, false),
            (BinaryOperator::NotEq, true),
        ] {
            // 1 OP '2'
            assert_eq!(
                run_and_extract_bool(binary_with(op, num(1.0, None), string("2", None))),
                Some(expected),
                "1 {:?} '2' wrong",
                op
            );
        }
        // Equal-after-coercion: '1.5' vs 1.5 — both 1.5.
        for (op, expected) in [
            (BinaryOperator::Eq, true),
            (BinaryOperator::NotEq, false),
            (BinaryOperator::Lt, false),
            (BinaryOperator::Gt, false),
            (BinaryOperator::LtEq, true),
            (BinaryOperator::GtEq, true),
        ] {
            assert_eq!(
                run_and_extract_bool(binary_with(op, num(1.5, None), string("1.5", None))),
                Some(expected),
                "1.5 {:?} '1.5' wrong",
                op
            );
        }
    }

    #[test]
    fn gap004_strict_equality_path_still_uses_gap008_not_this_branch() {
        // Sanity: `1 === '1'` should fold to false (gap-008), and `1 !== '1'`
        // to true. The new gap-004 branch is gated on Eq/NotEq/Lt/LtEq/Gt/GtEq
        // so it doesn't fire on StrictEq/StrictNotEq.
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::StrictEq,
                num(1.0, None),
                string("1", None)
            )),
            Some(false),
            "gap-008: 1 === '1' must stay false"
        );
        assert_eq!(
            run_and_extract_bool(binary_with(
                BinaryOperator::StrictNotEq,
                num(1.0, None),
                string("1", None)
            )),
            Some(true),
            "gap-008: 1 !== '1' must stay true"
        );
    }

    #[test]
    fn gap004_bails_when_string_is_unrecognised() {
        // `1 == 'hi'` is `false` per spec (ToNumber('hi') = NaN, then
        // 1 == NaN is false). But our conservative helper returns None
        // for unrecognised strings, so the fold doesn't fire — output
        // is the original BinaryExpression unchanged.
        let expr = binary_with(BinaryOperator::Eq, num(1.0, None), string("hi", None));
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        assert!(
            matches!(extract_expr(&out), Expression::BinaryExpression(_)),
            "1 == 'hi' must not fold (conservative bail)"
        );
    }

    // ------------------- null cross-type equality (gap-003) ----------
    //
    // These tests pin the behaviour added when CLOC12 gap-003 was
    // closed: the abstract-equality algorithm's `null`-side branch is
    // implemented for compile-time-known partner literals.

    fn undefined_(cv: Option<&str>) -> Expression {
        Expression::UndefinedLiteral(UndefinedLiteral {
            cv: cv.map(|s| s.to_string()),
        })
    }

    fn binary_with(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
        Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn run_and_extract_bool(expr: Expression) -> Option<bool> {
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        if let Expression::BooleanLiteral(bl) = extract_expr(&out) {
            Some(bl.value)
        } else {
            None
        }
    }

    #[test]
    fn gap003_null_loose_eq_other_primitives_folds_false() {
        // Left-side null vs every non-null/non-undefined literal kind.
        for partner in [
            num(0.0, None),
            num(42.0, None),
            string("hi", None),
            string("", None),
            boolean(true, None),
            boolean(false, None),
        ] {
            let folded =
                run_and_extract_bool(binary_with(BinaryOperator::Eq, null(None), partner.clone()));
            assert_eq!(
                folded,
                Some(false),
                "null == {:?} should fold to false",
                partner
            );
        }
    }

    #[test]
    fn gap003_null_loose_eq_other_primitives_is_symmetric() {
        // Same as above but with `null` on the right — fold must still fire.
        for partner in [num(0.0, None), string("hi", None), boolean(true, None)] {
            let folded =
                run_and_extract_bool(binary_with(BinaryOperator::Eq, partner.clone(), null(None)));
            assert_eq!(
                folded,
                Some(false),
                "{:?} == null should fold to false",
                partner
            );
        }
    }

    #[test]
    fn gap003_null_loose_neq_other_primitives_folds_true() {
        // `!=` is the boolean negation of `==`.
        for partner in [num(0.0, None), string("hi", None), boolean(true, None)] {
            let folded = run_and_extract_bool(binary_with(
                BinaryOperator::NotEq,
                null(None),
                partner.clone(),
            ));
            assert_eq!(
                folded,
                Some(true),
                "null != {:?} should fold to true",
                partner
            );
        }
    }

    #[test]
    fn gap003_null_loose_eq_undefined_folds_true() {
        // The one cross-type partner that's NOT false: `null == undefined`.
        // Per ES §IsLooselyEqual, the spec hard-codes this case.
        let folded = run_and_extract_bool(binary_with(
            BinaryOperator::Eq,
            null(None),
            undefined_(None),
        ));
        assert_eq!(folded, Some(true), "null == undefined must fold to true");

        // Symmetric.
        let folded = run_and_extract_bool(binary_with(
            BinaryOperator::Eq,
            undefined_(None),
            null(None),
        ));
        assert_eq!(folded, Some(true), "undefined == null must fold to true");

        // And the `!=` complement.
        let folded = run_and_extract_bool(binary_with(
            BinaryOperator::NotEq,
            null(None),
            undefined_(None),
        ));
        assert_eq!(folded, Some(false), "null != undefined must fold to false");
    }

    #[test]
    fn gap003_null_loose_eq_identifier_does_not_fold() {
        // Critical unsoundness guard: an Identifier's runtime value
        // could itself be null/undefined. Folding `null == someVar` to
        // a concrete boolean would change observable behaviour.
        let expr = binary_with(BinaryOperator::Eq, null(None), ident("x"));
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        // Output should still be a BinaryExpression — i.e., NOT a
        // BooleanLiteral.
        assert!(
            matches!(extract_expr(&out), Expression::BinaryExpression(_)),
            "null == identifier must NOT fold"
        );
    }

    #[test]
    fn gap003_null_strict_eq_is_handled_by_gap008_not_this_branch() {
        // Sanity check that we didn't accidentally break gap-008's
        // strict-equality cross-type fold by adding the loose branch
        // ahead of it. `null === 0` should still fold to false (via
        // gap-008's StrictEq/StrictNotEq branch).
        let folded = run_and_extract_bool(binary_with(
            BinaryOperator::StrictEq,
            null(None),
            num(0.0, None),
        ));
        assert_eq!(
            folded,
            Some(false),
            "null === 0 (gap-008) must still fold to false"
        );

        let folded = run_and_extract_bool(binary_with(
            BinaryOperator::StrictNotEq,
            null(None),
            num(0.0, None),
        ));
        assert_eq!(
            folded,
            Some(true),
            "null !== 0 (gap-008) must still fold to true"
        );
    }

    // ------------------- string ops ----------------------------------

    #[test]
    fn fold_string_concat() {
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(string("foo", None)),
            right: Box::new(string("bar", None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "foobar"),
            other => panic!("expected StringLiteral; got {:?}", other),
        }
    }

    #[test]
    fn fold_string_plus_number_coerces_to_string() {
        // "x" + 1 = "x1" per ES.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(string("x", None)),
            right: Box::new(num(1.0, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "x1"),
            other => panic!("expected StringLiteral; got {:?}", other),
        }
    }

    #[test]
    fn fold_number_plus_string_coerces_to_string() {
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0, None)),
            right: Box::new(string("x", None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "2x"),
            other => panic!("expected StringLiteral; got {:?}", other),
        }
    }

    // ------------------- unary ---------------------------------------

    #[test]
    fn fold_not_on_booleans_and_numbers_and_strings() {
        // !true, !false, !0, !1, !"", !"x", !null
        for (arg, expected) in [
            (boolean(true, None), false),
            (boolean(false, None), true),
            (num(0.0, None), true),
            (num(1.0, None), false),
            (string("", None), true),
            (string("x", None), false),
            (null(None), true),
        ] {
            let expr = Expression::UnaryExpression(UnaryExpression {
                cv: Some("u.1".to_string()),
                operator: UnaryOperator::Not,
                prefix: true,
                argument: Box::new(arg.clone()),
            });
            let (out, _, _, _) = run_pass(program_with_expr(expr, true));
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => {
                    assert_eq!(b.value, expected, "arg {:?}", arg)
                }
                other => panic!("expected BooleanLiteral; got {:?}", other),
            }
        }
    }

    // ------------- idempotent double-negation collapse (!!!x → !x) -----

    /// Prefix `!expr`.
    fn not(arg: Expression) -> Expression {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(arg),
        })
    }

    /// Count the `!` prefixes wrapping a leaf, returning `(count, leaf_name)`.
    fn count_nots(mut e: &Expression) -> (usize, String) {
        let mut n = 0;
        while let Expression::UnaryExpression(u) = e {
            assert_eq!(u.operator, UnaryOperator::Not, "expected only `!` chain");
            n += 1;
            e = &u.argument;
        }
        let name = match e {
            Expression::Identifier(id) => id.name.clone(),
            other => panic!("expected identifier leaf; got {other:?}"),
        };
        (n, name)
    }

    #[test]
    fn triple_not_collapses_to_single() {
        // !!!a → !a
        let expr = not(not(not(ident("a"))));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "!!!a should collapse");
        assert_eq!(count_nots(extract_expr(&out)), (1, "a".to_string()));
    }

    #[test]
    fn double_not_is_preserved() {
        // !!a is the canonical Boolean(a) coercion — must NOT collapse.
        let expr = not(not(ident("a")));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "!!a must be preserved");
        assert_eq!(count_nots(extract_expr(&out)), (2, "a".to_string()));
    }

    #[test]
    fn quad_not_collapses_to_double() {
        // !!!!a → !!a (even count keeps the boolean coercion)
        let expr = not(not(not(not(ident("a")))));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "!!!!a should collapse to !!a");
        assert_eq!(count_nots(extract_expr(&out)), (2, "a".to_string()));
    }

    #[test]
    fn quint_not_collapses_to_single() {
        // !!!!!a → !a
        let expr = not(not(not(not(not(ident("a"))))));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "!!!!!a should collapse to !a");
        assert_eq!(count_nots(extract_expr(&out)), (1, "a".to_string()));
    }

    #[test]
    fn triple_not_over_impure_operand_still_collapses_once_evaluated() {
        // !!!f() → !f(). Sound: `!` never re-evaluates its operand, so `f` is
        // called exactly once in both forms; no side-effect gate is needed.
        let expr = not(not(not(bare_call_local("f"))));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "!!!f() should collapse to !f()");
        match extract_expr(&out) {
            Expression::UnaryExpression(u) => {
                assert_eq!(u.operator, UnaryOperator::Not);
                assert!(
                    matches!(&*u.argument, Expression::CallExpression(_)),
                    "the single surviving `!` must wrap the call directly: {:?}",
                    u.argument
                );
            }
            other => panic!("expected `!f()`; got {other:?}"),
        }
    }

    /// A bare `f()` call — local helper (the module's other `call0` builds a
    /// method call, and `bare_call` lives in a different test section).
    fn bare_call_local(callee: &str) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident(callee)),
            arguments: vec![],
        })
    }

    #[test]
    fn fold_negate_and_plus() {
        let neg = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.neg".to_string()),
            operator: UnaryOperator::Negate,
            prefix: true,
            argument: Box::new(num(5.0, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(neg, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, -5.0),
            other => panic!("expected -5; got {:?}", other),
        }

        // +"42" → 42
        let plus_str = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.plus".to_string()),
            operator: UnaryOperator::Plus,
            prefix: true,
            argument: Box::new(string("42", None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(plus_str, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 42.0),
            other => panic!("expected 42; got {:?}", other),
        }

        // +true → 1
        let plus_bool = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.plusb".to_string()),
            operator: UnaryOperator::Plus,
            prefix: true,
            argument: Box::new(boolean(true, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(plus_bool, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 1.0),
            other => panic!("expected 1; got {:?}", other),
        }
    }

    #[test]
    fn fold_bitwise_not_on_numeric_literal() {
        // ~5 → -6  (~ToInt32(5) = ~5 = -6)
        let bn = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.bn".to_string()),
            operator: UnaryOperator::BitNot,
            prefix: true,
            argument: Box::new(num(5.0, None)),
        });
        let (out, _, changed, _) = run_pass(program_with_expr(bn, true));
        assert!(changed, "~5 should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, -6.0),
            other => panic!("expected -6; got {:?}", other),
        }

        // ~5.9 → -6  (ToInt32 truncates toward zero first → ~5)
        let bn_frac = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.bnf".to_string()),
            operator: UnaryOperator::BitNot,
            prefix: true,
            argument: Box::new(num(5.9, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(bn_frac, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, -6.0),
            other => panic!("expected -6; got {:?}", other),
        }

        // ~(-1) → 0. The inner `-1` is itself a Negate unary that folds to a
        // NumericLiteral first, then the outer `~` folds bottom-up in one walk.
        let inner_neg = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.bn.inner".to_string()),
            operator: UnaryOperator::Negate,
            prefix: true,
            argument: Box::new(num(1.0, None)),
        });
        let bn_neg = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.bn.neg".to_string()),
            operator: UnaryOperator::BitNot,
            prefix: true,
            argument: Box::new(inner_neg),
        });
        let (out, _, _, _) = run_pass(program_with_expr(bn_neg, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {:?}", other),
        }
    }

    #[test]
    fn bitwise_not_on_identifier_does_not_fold() {
        // `~x` needs the runtime value of `x`, so the unary stays put.
        let bn = Expression::UnaryExpression(UnaryExpression {
            cv: Some("u.bnx".to_string()),
            operator: UnaryOperator::BitNot,
            prefix: true,
            argument: Box::new(ident("x")),
        });
        let (out, _, changed, _) = run_pass(program_with_expr(bn, true));
        assert!(!changed, "~x must not fold");
        assert!(matches!(
            extract_expr(&out),
            Expression::UnaryExpression(_)
        ));
    }

    // ------------------- member access (`.length`) -------------------

    /// Build a non-computed `<object>.<name>` member expression.
    fn member(object: Expression, name: &str) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(object),
            property: Box::new(ident(name)),
            computed: false,
        })
    }

    #[test]
    fn fold_string_literal_length() {
        // "abc".length → 3
        let m = member(string("abc", None), "length");
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(changed, "\"abc\".length should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.0),
            other => panic!("expected 3; got {:?}", other),
        }

        // "".length → 0
        let m0 = member(string("", None), "length");
        let (out, _, _, _) = run_pass(program_with_expr(m0, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {:?}", other),
        }
    }

    /// Build an array literal from a slice of optional elements (`None` = hole).
    fn array_opt(elements: Vec<Option<Expression>>) -> Expression {
        Expression::ArrayExpression(ArrayExpression {
            cv: Some("arr.cv".to_string()),
            elements,
        })
    }

    /// CLOC12.193: `[e0, e1, …].length` folds to the element count when every
    /// present element is side-effect-free and there is no spread. Truth table
    /// verified against the reference Closure Compiler.
    #[test]
    fn fold_array_literal_length_when_pure() {
        // `[1, 2, 3].length` → 3.
        let m = member(
            array_opt(vec![
                Some(num(1.0, None)),
                Some(num(2.0, None)),
                Some(num(3.0, None)),
            ]),
            "length",
        );
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(changed, "[1,2,3].length should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.0),
            other => panic!("expected 3; got {other:?}"),
        }

        // `[].length` → 0.
        let m0 = member(array_opt(vec![]), "length");
        let (out, _, _, _) = run_pass(program_with_expr(m0, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {other:?}"),
        }

        // `[,,].length` → 2 — holes evaluate nothing but DO count toward length.
        let mh = member(array_opt(vec![None, None]), "length");
        let (out, _, _, _) = run_pass(program_with_expr(mh, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2 (holes count); got {other:?}"),
        }

        // `[a, b].length` → 2 — identifier elements are side-effect-free.
        let mid = member(
            array_opt(vec![Some(ident("a")), Some(ident("b"))]),
            "length",
        );
        let (out, _, _, _) = run_pass(program_with_expr(mid, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2 (identifiers are pure); got {other:?}"),
        }
    }

    /// The array-`.length` fold must DECLINE when dropping the array would drop a
    /// side effect (a call / an assignment) or when a spread makes the length
    /// statically unknown — matching Closure, which keeps all three intact.
    #[test]
    fn array_literal_length_declines_on_side_effect_or_spread() {
        use coding_adventures_javascript_ast::{AssignmentOperator, AssignmentTarget};
        // `[g()].length` — the call must not be dropped.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("g")),
            arguments: vec![],
        });
        let m_call = member(array_opt(vec![Some(call)]), "length");
        let (out, _, changed, _) = run_pass(program_with_expr(m_call, true));
        assert!(!changed, "[g()].length must not fold — the call has a side effect");
        assert!(
            matches!(extract_expr(&out), Expression::MemberExpression(_)),
            "expected the member expression to survive"
        );

        // `[a = 1].length` — the assignment mutates `a`, so it must not be dropped.
        let assign = Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: AssignmentOperator::Eq,
            left: AssignmentTarget::Identifier(Identifier { cv: None, name: "a".to_string() }),
            right: Box::new(num(1.0, None)),
        });
        let m_assign = member(array_opt(vec![Some(assign)]), "length");
        let (_out, _, changed, _) = run_pass(program_with_expr(m_assign, true));
        assert!(!changed, "[a=1].length must not fold — the assignment has a side effect");

        // `[1, 2, ...x].length` — a spread makes the length statically unknown.
        let spread = Expression::SpreadElement(SpreadElement {
            cv: None,
            argument: Box::new(ident("x")),
        });
        let m_spread = member(
            array_opt(vec![Some(num(1.0, None)), Some(num(2.0, None)), Some(spread)]),
            "length",
        );
        let (_out, _, changed, _) = run_pass(program_with_expr(m_spread, true));
        assert!(!changed, "[1,2,...x].length must not fold — spread length is unknown");
    }

    // -----------------------------------------------------------------
    // CLOC12.198: compound self-assignment contraction (`x = x OP E` → `x OP= E`)
    // -----------------------------------------------------------------

    /// Build `<target> = <right>` — a plain `=` assignment to a bare identifier.
    fn assign_eq(target: &str, right: Expression) -> Expression {
        Expression::AssignmentExpression(AssignmentExpression {
            cv: Some("as.1".to_string()),
            operator: AssignmentOperator::Eq,
            left: AssignmentTarget::Identifier(Identifier {
                cv: None,
                name: target.to_string(),
            }),
            right: Box::new(right),
        })
    }

    /// Every arithmetic / bitwise operator contracts: `x = x OP y` → `x OP= y`,
    /// keeping only the binary's RIGHT operand. Verified byte-identical against
    /// the reference Closure Compiler (`x=x+1`→`x+=1`, `x=x*2`→`x*=2`, …).
    #[test]
    fn contracts_compound_self_assignment_for_each_operator() {
        use AssignmentOperator as A;
        use BinaryOperator as B;
        let cases = [
            (B::Add, A::AddEq),
            (B::Sub, A::SubEq),
            (B::Mul, A::MulEq),
            (B::Div, A::DivEq),
            (B::Mod, A::ModEq),
            (B::Exp, A::ExpEq),
            (B::LeftShift, A::LeftShiftEq),
            (B::RightShift, A::RightShiftEq),
            (B::UnsignedRightShift, A::UnsignedRightShiftEq),
            (B::BitAnd, A::BitAndEq),
            (B::BitOr, A::BitOrEq),
            (B::BitXor, A::BitXorEq),
        ];
        for (bin_op, want) in cases {
            // `x = x <op> y`
            let rhs = binary_with(bin_op, ident("x"), ident("y"));
            let (out, _c, changed, _) = run_pass(program_with_expr(assign_eq("x", rhs), true));
            assert!(changed, "{bin_op:?}: the contraction should mark the program changed");
            match extract_expr(&out) {
                Expression::AssignmentExpression(a) => {
                    assert_eq!(a.operator, want, "{bin_op:?}: wrong compound operator");
                    match &a.left {
                        AssignmentTarget::Identifier(id) => {
                            assert_eq!(id.name, "x", "{bin_op:?}: target must survive")
                        }
                        other => panic!("{bin_op:?}: expected identifier target; got {other:?}"),
                    }
                    // The compound form keeps only the binary's RIGHT operand.
                    match a.right.as_ref() {
                        Expression::Identifier(id) => {
                            assert_eq!(id.name, "y", "{bin_op:?}: RHS must be the right operand")
                        }
                        other => panic!("{bin_op:?}: expected `y` as the RHS; got {other:?}"),
                    }
                }
                other => panic!("{bin_op:?}: expected an assignment back; got {other:?}"),
            }
        }
    }

    /// The contraction DECLINES whenever the shape is not `target = target OP E`:
    /// a different left identifier, the target on the binary's RIGHT operand
    /// (`x = 1 + x` — unsound for `-`/`/`/… and never done by Closure), an
    /// operator with no compound form (`==`), or an already-compound operator.
    #[test]
    fn compound_self_assignment_declines_off_pattern() {
        // `x = y + 1` — a *different* identifier: not a self-assignment.
        let e1 = assign_eq("x", binary_with(BinaryOperator::Add, ident("y"), num(1.0, None)));
        let (o1, _, c1, _) = run_pass(program_with_expr(e1, true));
        assert!(!c1, "x = y + 1 must not contract");
        assert!(
            matches!(extract_expr(&o1), Expression::AssignmentExpression(a) if a.operator == AssignmentOperator::Eq),
            "x = y + 1 must stay a plain `=` assignment"
        );

        // `x = 1 + x` — the target is the binary's RIGHT operand, not its left.
        let e2 = assign_eq("x", binary_with(BinaryOperator::Add, num(1.0, None), ident("x")));
        let (o2, _, c2, _) = run_pass(program_with_expr(e2, true));
        assert!(!c2, "x = 1 + x must not contract (target on the right)");
        assert!(
            matches!(extract_expr(&o2), Expression::AssignmentExpression(a) if a.operator == AssignmentOperator::Eq),
            "x = 1 + x must stay a plain `=` assignment"
        );

        // `x = x == 1` — the comparison operator has no `OP=` form.
        let e3 = assign_eq("x", binary_with(BinaryOperator::Eq, ident("x"), num(1.0, None)));
        let (_o3, _, c3, _) = run_pass(program_with_expr(e3, true));
        assert!(!c3, "x = x == 1 must not contract (no `==` compound form)");

        // `x += x + 1` — the operator is already compound (not plain `=`).
        let e4 = Expression::AssignmentExpression(AssignmentExpression {
            cv: Some("as.4".to_string()),
            operator: AssignmentOperator::AddEq,
            left: AssignmentTarget::Identifier(Identifier { cv: None, name: "x".to_string() }),
            right: Box::new(binary_with(BinaryOperator::Add, ident("x"), num(1.0, None))),
        });
        let (_o4, _, c4, _) = run_pass(program_with_expr(e4, true));
        assert!(!c4, "x += (x + 1) must not further contract");
    }

    /// A string self-concat contracts as well: `s = s + "b"` → `s += "b"`, and
    /// the rewrite is CV-traced — the contracted node carries a forked CV id and
    /// a `constant-fold` contribution is recorded.
    #[test]
    fn contracts_string_self_concat_and_traces_cv() {
        let expr = assign_eq("s", binary_with(BinaryOperator::Add, ident("s"), string("b", None)));
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "s = s + \"b\" should contract");
        match extract_expr(&out) {
            Expression::AssignmentExpression(a) => {
                assert_eq!(a.operator, AssignmentOperator::AddEq);
                match a.right.as_ref() {
                    Expression::StringLiteral(s) => assert_eq!(s.value, "b"),
                    other => panic!("expected the string \"b\" as RHS; got {other:?}"),
                }
                assert!(a.cv.is_some(), "the contracted node should carry a forked CV id");
            }
            other => panic!("expected an assignment back; got {other:?}"),
        }
        assert!(
            contribs
                .iter()
                .any(|c| c.source == "constant-fold" && c.tag == "folded"),
            "a constant-fold contribution should be recorded for the contraction"
        );
    }

    /// Build a computed index access `object[k]` (an integer-literal index).
    fn index(object: Expression, k: f64) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(object),
            property: Box::new(num(k, None)),
            computed: true,
        })
    }

    /// CLOC12.196: `[e0, e1, …][K]` folds to the element at index `K` when `K` is
    /// an in-bounds non-negative integer, the element is present, no spread
    /// exists, and every *other* element is side-effect-free. Verified against
    /// the reference Closure Compiler.
    #[test]
    fn fold_array_index_when_in_bounds_and_pure() {
        // `[1, 2, 3][0]` → 1, `[1, 2, 3][1]` → 2, `[1, 2, 3][2]` → 3.
        for (k, want) in [(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)] {
            let e = index(
                array_opt(vec![
                    Some(num(1.0, None)),
                    Some(num(2.0, None)),
                    Some(num(3.0, None)),
                ]),
                k,
            );
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(changed, "[1,2,3][{k}] should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, want, "[1,2,3][{k}]"),
                other => panic!("expected {want}; got {other:?}"),
            }
        }

        // `[a, b, c][1]` → `b` — identifier elements are side-effect-free.
        let e = index(
            array_opt(vec![Some(ident("a")), Some(ident("b")), Some(ident("c"))]),
            1.0,
        );
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::Identifier(id) => assert_eq!(id.name, "b"),
            other => panic!("expected ident b; got {other:?}"),
        }

        // `[1, , 3][0]` → 1 — a hole ELSEWHERE (index 1) doesn't block index 0.
        let e = index(
            array_opt(vec![Some(num(1.0, None)), None, Some(num(3.0, None))]),
            0.0,
        );
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 1.0),
            other => panic!("expected 1; got {other:?}"),
        }
    }

    #[test]
    fn fold_array_index_preserves_side_effect_in_selected_element() {
        // `[a, g()][1]` → `g()` — the SELECTED element is preserved verbatim, so
        // its call still runs; the other element `a` is pure and safely dropped.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("g")),
            arguments: vec![],
        });
        let e = index(array_opt(vec![Some(ident("a")), Some(call)]), 1.0);
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "[a,g()][1] should fold to the (preserved) call");
        assert!(
            matches!(extract_expr(&out), Expression::CallExpression(_)),
            "expected the selected call to survive"
        );
    }

    #[test]
    fn array_index_declines_on_side_effect_hole_oob_or_spread() {
        // `[a, g()][0]` — selecting `a` would DROP `g()`, a side effect → decline.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("g")),
            arguments: vec![],
        });
        let e = index(array_opt(vec![Some(ident("a")), Some(call)]), 0.0);
        let (_out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(!changed, "[a,g()][0] must not fold — would drop the call g()");

        // `[1, 2, ...x][0]` — a spread makes runtime indices unknown → decline.
        let spread = Expression::SpreadElement(SpreadElement {
            cv: None,
            argument: Box::new(ident("x")),
        });
        let e = index(
            array_opt(vec![Some(num(1.0, None)), Some(num(2.0, None)), Some(spread)]),
            0.0,
        );
        let (_out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(!changed, "[1,2,...x][0] must not fold — spread indices unknown");

        // `[1, 2, 3][1.5]` — a fractional index is an ordinary absent-property
        // read, not an element pick. Closure leaves it intact → decline.
        let e = index(
            array_opt(vec![
                Some(num(1.0, None)),
                Some(num(2.0, None)),
                Some(num(3.0, None)),
            ]),
            1.5,
        );
        let (_out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(!changed, "[1,2,3][1.5] fractional index — must not fold");

        // `[f(), 2][5]` — out of bounds, BUT the `void 0` result would drop the
        // whole literal including the impure `f()` → decline (CLOC12.196b: every
        // element must be side-effect-free for a `void 0` fold).
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("f")),
            arguments: vec![],
        });
        let e = index(array_opt(vec![Some(call), Some(num(2.0, None))]), 5.0);
        let (_out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(!changed, "[f(),2][5] must not fold — would drop the call f()");
    }

    #[test]
    fn fold_array_index_out_of_bounds_to_void_0() {
        // CLOC12.196b: an out-of-bounds index reads as `undefined`, which the
        // emitter spells `void 0`. Verified against the Closure oracle:
        //   [1,2,3][5]  [1,2][2]  [][0]  → void 0.
        for (elems, k) in [
            (vec![Some(num(1.0, None)), Some(num(2.0, None)), Some(num(3.0, None))], 5.0),
            (vec![Some(num(1.0, None)), Some(num(2.0, None))], 2.0),
            (vec![], 0.0),
        ] {
            let e = index(array_opt(elems), k);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(changed, "out-of-bounds index {k} should fold to void 0");
            assert!(
                matches!(extract_expr(&out), Expression::UndefinedLiteral(_)),
                "expected void 0 (UndefinedLiteral) for out-of-bounds index {k}"
            );
        }
    }

    #[test]
    fn fold_array_index_negative_to_void_0() {
        // CLOC12.196b: a negative index is never a valid array slot → `void 0`.
        // Post-fold, `-1` is a NumericLiteral with value `-1.0`, so the OOB path
        // handles it. Oracle: `[1,2,3][-1]` → `void 0`.
        let e = index(
            array_opt(vec![
                Some(num(1.0, None)),
                Some(num(2.0, None)),
                Some(num(3.0, None)),
            ]),
            -1.0,
        );
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "[1,2,3][-1] should fold to void 0");
        assert!(
            matches!(extract_expr(&out), Expression::UndefinedLiteral(_)),
            "expected void 0 (UndefinedLiteral) for negative index"
        );
    }

    #[test]
    fn fold_array_index_in_bounds_hole_to_void_0() {
        // CLOC12.196b: an in-bounds slot that is a HOLE reads as `undefined`.
        // Oracle: `[1,,3][1]` → `void 0`. The other present elements (1, 3) are
        // side-effect-free, so dropping the literal is sound.
        let e = index(
            array_opt(vec![Some(num(1.0, None)), None, Some(num(3.0, None))]),
            1.0,
        );
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "[1,,3][1] selects a hole — should fold to void 0");
        assert!(
            matches!(extract_expr(&out), Expression::UndefinedLiteral(_)),
            "expected void 0 (UndefinedLiteral) for in-bounds hole"
        );
    }

    /// Build a computed STRING-key access `object["key"]`.
    fn index_str(object: Expression, key: &str) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(object),
            property: Box::new(string(key, None)),
            computed: true,
        })
    }

    /// String-index fold: Closure coerces the string key with JS `ToNumber` and
    /// applies the same index fold, so canonical AND non-canonical spellings
    /// select their integer value's element. Verified against the reference
    /// Closure Compiler at SIMPLE.
    #[test]
    fn fold_array_string_index_canonical_and_non_canonical() {
        let arr = || array_opt(vec![Some(num(10.0, None)), Some(num(20.0, None)), Some(num(30.0, None))]);
        // key → selected element value.
        for (key, want) in [
            ("0", 10.0),
            ("1", 20.0),
            ("2", 30.0),
            ("01", 20.0),   // leading zero → 1
            ("1.0", 20.0),  // trailing .0 → 1
            (" 1", 20.0),   // leading whitespace trimmed
            ("1 ", 20.0),   // trailing whitespace trimmed
            ("0x1", 20.0),  // hex → 1
            ("1e0", 20.0),  // exponent → 1
            ("", 10.0),     // ToNumber("") === +0
        ] {
            let e = index_str(arr(), key);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(changed, "[10,20,30][{key:?}] should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => {
                    assert_eq!(n.value, want, "[10,20,30][{key:?}] selected wrong element")
                }
                other => panic!("[10,20,30][{key:?}]: expected {want}; got {other:?}"),
            }
        }
    }

    #[test]
    fn fold_array_string_index_oob_and_negative_to_void_0() {
        let arr = || array_opt(vec![Some(num(10.0, None)), Some(num(20.0, None)), Some(num(30.0, None))]);
        for key in ["3", "-1"] {
            let e = index_str(arr(), key);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(changed, "[10,20,30][{key:?}] should fold to void 0");
            assert!(
                matches!(extract_expr(&out), Expression::UndefinedLiteral(_)),
                "[10,20,30][{key:?}]: expected void 0"
            );
        }
    }

    #[test]
    fn fold_array_string_index_declines_on_fractional_and_non_numeric() {
        let arr = || array_opt(vec![Some(num(10.0, None)), Some(num(20.0, None)), Some(num(30.0, None))]);
        // "1.5" coerces to 1.5 — a fractional index is an ordinary absent-property
        // read, and "1.5" is not an identifier name, so it stays a bracketed
        // computed member (declines both the index fold and the dot fold).
        // Oracle: `[10,20,30]["1.5"]` → `[10,20,30]["1.5"]`.
        let e = index_str(arr(), "1.5");
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(!changed, "[10,20,30][\"1.5\"] must NOT fold");
        assert!(
            matches!(extract_expr(&out), Expression::MemberExpression(m) if m.computed),
            "[10,20,30][\"1.5\"]: the computed member access must be kept"
        );

        // "foo" is NaN as an index, but it IS a valid identifier name, so it now
        // normalises to a dot member: `[10,20,30]["foo"]` → `[10,20,30].foo`.
        let e = index_str(arr(), "foo");
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "[10,20,30][\"foo\"] normalises to a dot member");
        assert!(
            matches!(extract_expr(&out), Expression::MemberExpression(m) if !m.computed),
            "[10,20,30][\"foo\"]: expected a non-computed (dot) member"
        );
    }

    #[test]
    fn fold_array_string_key_length_to_element_count() {
        // `[10,20,30]["length"]` → 3, the computed twin of the `.length` fold.
        let e = index_str(
            array_opt(vec![Some(num(10.0, None)), Some(num(20.0, None)), Some(num(30.0, None))]),
            "length",
        );
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "[10,20,30][\"length\"] should fold to 3");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.0),
            other => panic!("expected 3; got {other:?}"),
        }
    }

    /// A computed member whose key is a non-reserved ASCII identifier name folds
    /// to a dot member: `o["foo"]` → `o.foo`. `let`/`yield` are NOT ES3 keywords,
    /// so they dot too. Verified byte-identical against the reference compiler.
    #[test]
    fn fold_computed_string_key_to_dot() {
        for key in ["foo", "$x", "_y", "a1", "let", "yield", "undefined", "NaN"] {
            let e = index_str(ident("o"), key);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(changed, "o[{key:?}] should fold to a dot member");
            match extract_expr(&out) {
                Expression::MemberExpression(m) => {
                    assert!(!m.computed, "o[{key:?}] must become a non-computed member");
                    match m.property.as_ref() {
                        Expression::Identifier(id) => assert_eq!(id.name, key),
                        other => panic!("o[{key:?}]: property must be an Identifier; got {other:?}"),
                    }
                }
                other => panic!("o[{key:?}]: expected a MemberExpression; got {other:?}"),
            }
        }
    }

    #[test]
    fn computed_key_declines_on_es3_reserved_word() {
        // ES3 keywords + future-reserved words stay bracketed (Closure keeps them
        // ES3-safe): class / static / delete / int / boolean / enum / …
        for key in ["class", "if", "static", "delete", "int", "boolean", "enum", "super", "true", "null"] {
            let e = index_str(ident("o"), key);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(!changed, "o[{key:?}] (ES3 reserved) must stay bracketed");
            assert!(
                matches!(extract_expr(&out), Expression::MemberExpression(m) if m.computed),
                "o[{key:?}] must remain a computed member"
            );
        }
    }

    #[test]
    fn computed_key_declines_on_non_identifier() {
        for key in ["1a", "", "a b", "a-b", "a.b"] {
            let e = index_str(ident("o"), key);
            let (out, _, changed, _) = run_pass(program_with_expr(e, true));
            assert!(!changed, "o[{key:?}] (not an identifier) must stay bracketed");
            assert!(
                matches!(extract_expr(&out), Expression::MemberExpression(m) if m.computed),
                "o[{key:?}] must remain a computed member"
            );
        }
    }

    /// An assignment TARGET dot-normalises too: `o["foo"] = 1` → `o.foo = 1`.
    #[test]
    fn computed_key_to_dot_in_assignment_target() {
        use coding_adventures_javascript_ast::{AssignmentOperator, AssignmentTarget};
        let target = AssignmentTarget::MemberExpression(Box::new(MemberExpression {
            cv: None,
            object: Box::new(ident("o")),
            property: Box::new(string("foo", None)),
            computed: true,
        }));
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: Some("as.cv".to_string()),
            operator: AssignmentOperator::Eq,
            left: target,
            right: Box::new(num(1.0, None)),
        });
        let (out, _, changed, _) = run_pass(program_with_expr(e, true));
        assert!(changed, "o[\"foo\"]=1 target should dot-normalise");
        match extract_expr(&out) {
            Expression::AssignmentExpression(a) => match &a.left {
                AssignmentTarget::MemberExpression(m) => {
                    assert!(!m.computed, "target must become a non-computed member");
                    assert!(
                        matches!(m.property.as_ref(), Expression::Identifier(id) if id.name == "foo"),
                        "target property must be Identifier(foo)"
                    );
                }
                other => panic!("expected a member target; got {other:?}"),
            },
            other => panic!("expected an assignment; got {other:?}"),
        }
    }

    /// Build a template literal from quasi cooked strings + substitution
    /// expressions (`quasis.len() == exprs.len() + 1`). CLOC12.197 test helper.
    fn template(quasis: &[&str], exprs: Vec<Expression>) -> Expression {
        use coding_adventures_javascript_ast::{TemplateElement, TemplateLiteral};
        let n = exprs.len();
        let quasi_elems = quasis
            .iter()
            .enumerate()
            .map(|(i, q)| TemplateElement {
                cv: None,
                raw: q.to_string(),
                cooked: Some(q.to_string()),
                tail: i == n,
            })
            .collect();
        Expression::TemplateLiteral(TemplateLiteral {
            cv: Some("tpl.cv".to_string()),
            quasis: quasi_elems,
            expressions: exprs,
        })
    }

    /// CLOC12.197: `` `a${1}b` `` → `"a1b"` when every substitution is a
    /// stringifiable constant literal. Truth table verified against the
    /// reference Closure Compiler.
    #[test]
    fn fold_template_all_const_subs() {
        // `\`a${1}b\`` → "a1b".
        let t = template(&["a", "b"], vec![num(1.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(changed, "template with a const sub should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a1b"),
            other => panic!("expected \"a1b\"; got {other:?}"),
        }

        // `\`${1}-${2}-${3}\`` → "1-2-3".
        let t = template(
            &["", "-", "-", ""],
            vec![num(1.0, None), num(2.0, None), num(3.0, None)],
        );
        let (out, _, _, _) = run_pass(program_with_expr(t, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "1-2-3"),
            other => panic!("expected \"1-2-3\"; got {other:?}"),
        }

        // string / bool / null substitutions stringify via `ToString`.
        let t = template(
            &["", "-", "-", ""],
            vec![string("x", None), boolean(true, None), null(None)],
        );
        let (out, _, _, _) = run_pass(program_with_expr(t, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "x-true-null"),
            other => panic!("expected \"x-true-null\"; got {other:?}"),
        }
    }

    #[test]
    fn fold_template_no_substitutions() {
        // `\`hello\`` → "hello".
        let t = template(&["hello"], vec![]);
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(changed);
        assert!(matches!(extract_expr(&out), Expression::StringLiteral(s) if s.value == "hello"));

        // `\`\`` → "".
        let t = template(&[""], vec![]);
        let (out, _, _, _) = run_pass(program_with_expr(t, true));
        assert!(matches!(extract_expr(&out), Expression::StringLiteral(s) if s.value.is_empty()));
    }

    #[test]
    fn fold_template_const_expr_sub_folds_first() {
        // `\`a${1+2}b\`` — `1+2` folds to `3` first (recursion), then the
        // template collapses → "a3b".
        let sum = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(1.0, None)),
            right: Box::new(num(2.0, None)),
        });
        let t = template(&["a", "b"], vec![sum]);
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(changed);
        assert!(matches!(extract_expr(&out), Expression::StringLiteral(s) if s.value == "a3b"));
    }

    #[test]
    fn template_declines_on_nonconst_substitution() {
        // `\`a${x}b\`` — an identifier is not a compile-time constant → keep.
        let t = template(&["a", "b"], vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(!changed, "template with a non-const sub must not fold");
        assert!(matches!(extract_expr(&out), Expression::TemplateLiteral(_)));

        // `\`a${f()}b\`` — a call is not constant → keep.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("f")),
            arguments: vec![],
        });
        let t = template(&["a", "b"], vec![call]);
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(!changed);
        assert!(matches!(extract_expr(&out), Expression::TemplateLiteral(_)));
    }

    #[test]
    fn fold_string_length_counts_utf16_code_units_not_scalars() {
        // "💩" (U+1F4A9, an astral-plane char) is ONE Unicode scalar but TWO
        // UTF-16 code units, so JS `"💩".length` is 2 — not 1. This guards the
        // encode_utf16() choice against a naive `.chars().count()`.
        let m = member(string("💩", None), "length");
        let (out, _, _, _) = run_pass(program_with_expr(m, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2 (UTF-16 units); got {:?}", other),
        }

        // A BMP combining sequence "é" written as e + U+0301 is two code units.
        let m2 = member(string("e\u{0301}", None), "length");
        let (out, _, _, _) = run_pass(program_with_expr(m2, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2; got {:?}", other),
        }
    }

    #[test]
    fn length_on_identifier_does_not_fold() {
        // `s.length` needs the runtime value of `s`, so it stays a member expr.
        let m = member(ident("s"), "length");
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(!changed, "s.length must not fold");
        assert!(matches!(
            extract_expr(&out),
            Expression::MemberExpression(_)
        ));
    }

    #[test]
    fn non_length_property_on_string_does_not_fold() {
        // `"abc".charCodeAt` is not `.length`; it must pass through untouched.
        let m = member(string("abc", None), "charCodeAt");
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(!changed, "\"abc\".charCodeAt must not fold");
        assert!(matches!(
            extract_expr(&out),
            Expression::MemberExpression(_)
        ));
    }

    #[test]
    fn computed_string_length_normalizes_to_dot() {
        // `"abc"["length"]` — the computed `["length"]` normalises to a `.length`
        // dot member in THIS pass. The `.length` → `3` fold then fires on the
        // pipeline's next fixpoint iteration (a bare member has no enclosing node
        // to re-inspect it within a single pass, unlike the casing-in-a-call
        // case above). End-to-end, closurec emits `3` — see the oracle e2e.
        let m = Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(string("abc", None)),
            property: Box::new(string("length", None)),
            computed: true,
        });
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(changed, "\"abc\"[\"length\"] normalises to a dot member");
        match extract_expr(&out) {
            Expression::MemberExpression(mm) => {
                assert!(!mm.computed, "must become a non-computed member");
                match mm.property.as_ref() {
                    Expression::Identifier(id) => assert_eq!(id.name, "length"),
                    other => panic!("property must be Identifier(length); got {other:?}"),
                }
            }
            other => panic!("expected a MemberExpression; got {other:?}"),
        }
    }

    // ------------------- string casing methods -----------------------

    /// Build a zero-argument method call `<object>.<name>()`.
    fn call0(object: Expression, name: &str) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(object, name)),
            arguments: vec![],
        })
    }

    #[test]
    fn fold_ascii_string_to_upper_and_lower_case() {
        // "abc".toUpperCase() → "ABC"
        let up = call0(string("abc", None), "toUpperCase");
        let (out, _, changed, _) = run_pass(program_with_expr(up, true));
        assert!(changed, "\"abc\".toUpperCase() should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "ABC"),
            other => panic!("expected \"ABC\"; got {:?}", other),
        }

        // "ABC".toLowerCase() → "abc"
        let lo = call0(string("ABC", None), "toLowerCase");
        let (out, _, _, _) = run_pass(program_with_expr(lo, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected \"abc\"; got {:?}", other),
        }

        // "".toUpperCase() → ""
        let empty = call0(string("", None), "toUpperCase");
        let (out, _, _, _) = run_pass(program_with_expr(empty, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, ""),
            other => panic!("expected empty string; got {:?}", other),
        }
    }

    #[test]
    fn non_ascii_string_casing_does_not_fold() {
        // JS toUpperCase on non-ASCII uses full Unicode case mapping (e.g.
        // "é" → "É", "ß" → "SS"); we conservatively leave non-ASCII to a later
        // phase, so the call must survive untouched.
        let up = call0(string("é", None), "toUpperCase");
        let (out, _, changed, _) = run_pass(program_with_expr(up, true));
        assert!(!changed, "non-ASCII \"é\".toUpperCase() must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn string_casing_on_identifier_does_not_fold() {
        // `s.toUpperCase()` needs the runtime value of `s`.
        let up = call0(ident("s"), "toUpperCase");
        let (out, _, changed, _) = run_pass(program_with_expr(up, true));
        assert!(!changed, "s.toUpperCase() must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn string_casing_with_argument_does_not_fold() {
        // An argument makes us stay conservative (the runtime ignores it, but
        // we don't model that): `"abc".toUpperCase(1)` survives.
        let up = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abc", None), "toUpperCase")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(up, true));
        assert!(!changed, "\"abc\".toUpperCase(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn computed_string_casing_folds_via_dot_normalization() {
        // `"abc"["toUpperCase"]()` — the computed key `["toUpperCase"]` first
        // normalises to a dot member (`.toUpperCase`), which then lets the
        // string-casing fold fire: `"abc".toUpperCase()` → `"ABC"`. Oracle:
        // `"abc"["toUpperCase"]()` → `"ABC"`.
        let callee = Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(string("abc", None)),
            property: Box::new(string("toUpperCase", None)),
            computed: true,
        });
        let up = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(callee),
            arguments: vec![],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(up, true));
        assert!(changed, "computed casing call now folds via dot normalization");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "ABC"),
            other => panic!("expected \"ABC\"; got {other:?}"),
        }
    }

    #[test]
    fn unknown_string_method_does_not_fold() {
        // `"abc".normalize()` is not a method we model — pass through. (We do
        // fold `trim`/`trimStart`/`trimEnd` now; see the trimming tests below.)
        let t = call0(string("abc", None), "normalize");
        let (out, _, changed, _) = run_pass(program_with_expr(t, true));
        assert!(!changed, "\"abc\".normalize() must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- trimming (trim / trimStart / trimEnd) --------

    #[test]
    fn fold_string_trim_basic() {
        // (recv, method, expect) — oracle values from V8.
        for (recv, method, expect) in [
            ("  abc  ", "trim", "abc"),
            ("  abc  ", "trimStart", "abc  "),
            ("  abc  ", "trimEnd", "  abc"),
            ("\t\n abc \r\n", "trim", "abc"),  // mixed tab/newline/CR
            ("abc", "trim", "abc"),            // nothing to strip
            ("   ", "trim", ""),               // all whitespace → empty
            ("a b", "trim", "a b"),            // interior space kept
        ] {
            let c = call0(string(recv, None), method);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "{:?}.{method}() should fold", recv);
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "{:?}.{method}()", recv)
                }
                other => panic!("expected {:?}; got {:?}", expect, other),
            }
        }
    }

    #[test]
    fn trim_strips_the_full_js_whitespace_set() {
        // A non-ASCII space JS strips: U+00A0 (no-break) and U+3000 (ideographic)
        // and U+FEFF (BOM). All must be removed at the ends.
        let recv = "\u{00A0}\u{3000}\u{FEFF}hi\u{2028}\u{205F}";
        let c = call0(string(recv, None), "trim");
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "hi"),
            other => panic!("expected \"hi\"; got {:?}", other),
        }
    }

    #[test]
    fn trim_does_not_strip_non_js_whitespace() {
        // U+200B (zero-width space), U+2060 (word joiner), and U+180E are NOT in
        // the JS trim set — Rust's `char::is_whitespace` agrees here, but the
        // guard matters: they must survive at the ends.
        let recv = "\u{200B}hi\u{2060}";
        let c = call0(string(recv, None), "trim");
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "trim still folds (to the unchanged value)");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, recv),
            other => panic!("expected the value unchanged; got {:?}", other),
        }
    }

    #[test]
    fn trim_on_identifier_receiver_does_not_fold() {
        // `s.trim()` needs the runtime value of `s`.
        let c = call0(ident("s"), "trim");
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.trim() must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    /// Build a global call `name(args…)` whose callee is the bare identifier
    /// `name` (not a member access) — the shape `parseInt`/`parseFloat` take.
    fn global_call(name: &str, args: Vec<Expression>) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(ident(name)),
            arguments: args,
        })
    }

    #[test]
    fn fold_parse_int_direct_oracle() {
        // (input, radix, expected) — values confirmed against V8.
        for (input, radix, expect) in [
            ("12px", None, Some(12.0)),     // trailing garbage ignored
            ("", None, None),               // empty → NaN → decline
            ("0x1F", None, Some(31.0)),     // auto-detected hex prefix
            ("FF", Some(16.0), Some(255.0)), // explicit radix 16
            ("-7", None, Some(-7.0)),       // negative sign
            ("+9", None, Some(9.0)),        // positive sign
            ("08", None, Some(8.0)),        // NOT octal in modern JS
            ("3.9", None, Some(3.0)),       // stops at the dot
            ("  42  ", None, Some(42.0)),   // leading whitespace skipped
            ("z", Some(36.0), Some(35.0)),  // base-36 digit
            ("10", Some(2.0), Some(2.0)),   // binary
            ("xyz", None, None),            // no leading digit → NaN
            ("99", Some(1.0), None),        // radix < 2 → NaN
            ("99", Some(37.0), None),       // radix > 36 → NaN
            ("0x1F", Some(16.0), Some(31.0)), // 0x prefix honoured at radix 16
            ("12", Some(0.0), Some(12.0)),  // radix 0 → base 10
        ] {
            assert_eq!(
                fold_parse_int(input, radix),
                expect,
                "parseInt({input:?}, {radix:?})"
            );
        }
    }

    #[test]
    fn fold_parse_float_direct_oracle() {
        for (input, expect) in [
            ("3.14abc", Some(3.14)), // trailing garbage ignored
            ("", None),              // empty → NaN → decline
            ("1e3", Some(1000.0)),   // exponent
            ("Infinity", None),      // Infinity → decline (no literal)
            ("-Infinity", None),     // signed Infinity → decline
            (".5", Some(0.5)),       // leading dot
            ("5.", Some(5.0)),       // trailing dot
            ("-7", Some(-7.0)),      // negative
            ("  6.0 ", Some(6.0)),   // leading whitespace
            ("abc", None),           // no mantissa → NaN
            ("1e", Some(1.0)),       // dangling exponent → just the mantissa
            ("2.5e-3", Some(0.0025)), // signed exponent
        ] {
            assert_eq!(fold_parse_float(input), expect, "parseFloat({input:?})");
        }
    }

    #[test]
    fn fold_parse_int_through_pass() {
        // `parseInt("12px")` folds to the numeric literal `12`.
        let c = global_call("parseInt", vec![string("12px", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "parseInt(\"12px\") should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 12.0),
            other => panic!("expected 12; got {:?}", other),
        }
    }

    #[test]
    fn fold_parse_int_with_radix_through_pass() {
        // `parseInt("FF", 16)` → `255`.
        let c = global_call("parseInt", vec![string("FF", None), num(16.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "parseInt(\"FF\", 16) should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 255.0),
            other => panic!("expected 255; got {:?}", other),
        }
    }

    #[test]
    fn fold_parse_float_through_pass() {
        // `parseFloat("3.14abc")` → `3.14`.
        let c = global_call("parseFloat", vec![string("3.14abc", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "parseFloat(\"3.14abc\") should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.14),
            other => panic!("expected 3.14; got {:?}", other),
        }
    }

    #[test]
    fn parse_int_nan_result_does_not_fold() {
        // `parseInt("")` is NaN — no literal, so the call is left intact.
        let c = global_call("parseInt", vec![string("", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "parseInt(\"\") must not fold (NaN)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn parse_float_infinity_does_not_fold() {
        // `parseFloat("Infinity")` is Infinity — no literal, so left intact.
        let c = global_call("parseFloat", vec![string("Infinity", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "parseFloat(\"Infinity\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn parse_int_non_literal_radix_does_not_fold() {
        // A variable radix can't be modelled — leave the call alone.
        let c = global_call("parseInt", vec![string("10", None), ident("r")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "parseInt(\"10\", r) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn parse_int_non_string_argument_does_not_fold() {
        // Only a STRING-literal argument folds; `parseInt(x)` needs runtime `x`.
        let c = global_call("parseInt", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "parseInt(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_parse_int_does_not_fold() {
        // `window.parseInt("12")` is a member call, not the global identifier —
        // it must NOT be folded by the global-call arm.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "parseInt")),
            arguments: vec![string("12", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.parseInt(\"12\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- encodeURIComponent / decodeURIComponent -----

    #[test]
    fn encode_uri_component_direct_oracle() {
        // (input, expected) — every value confirmed against V8's
        // `encodeURIComponent`.
        for (input, expect) in [
            ("a b", "a%20b"),       // space → %20
            ("é", "%C3%A9"),        // two UTF-8 bytes, each escaped, uppercase hex
            ("💩", "%F0%9F%92%A9"), // astral scalar → four bytes
            ("", ""),               // empty stays empty
            ("AZaz09", "AZaz09"),   // alphanumerics pass through
            ("-_.!~*'()", "-_.!~*'()"), // all nine unreserved marks pass through
            (";,/?:@&=+$", "%3B%2C%2F%3F%3A%40%26%3D%2B%24"), // reserved ARE escaped
            ("100%", "100%25"),     // a literal percent is itself escaped
            ("\n\t", "%0A%09"),     // control chars → %0A %09
        ] {
            assert_eq!(
                encode_uri_component(input),
                expect,
                "encodeURIComponent({input:?})"
            );
        }
    }

    #[test]
    fn decode_uri_component_direct_oracle() {
        // (input, expected) — confirmed against V8's `decodeURIComponent`;
        // `None` is a `URIError` (we decline rather than fold a throw).
        for (input, expect) in [
            ("a%20b", Some("a b")),       // %20 → space
            ("%C3%A9", Some("é")),        // two bytes reassembled into one scalar
            ("%F0%9F%92%A9", Some("💩")), // four bytes → astral scalar
            ("plain", Some("plain")),     // no escapes: identity
            ("%41%42", Some("AB")),       // back-to-back escapes
            ("a%2Bb", Some("a+b")),       // lowercase/uppercase hex both decode
            ("a%2bb", Some("a+b")),
            ("100%25", Some("100%")),     // escaped percent round-trips
            ("", Some("")),               // empty
            ("%", None),                  // truncated escape → URIError
            ("%2", None),                 // one-hex escape → URIError
            ("ab%", None),                // trailing lone percent → URIError
            ("%G0", None),                // non-hex digit → URIError
            ("%2G", None),                // second digit non-hex → URIError
            ("%80", None),                // lone UTF-8 continuation byte → bad UTF-8
            ("%C3", None),                // truncated multi-byte scalar → bad UTF-8
            ("%ED%A0%80", None),          // surrogate-range bytes are not valid UTF-8
        ] {
            assert_eq!(
                decode_uri_component(input),
                expect.map(str::to_string),
                "decodeURIComponent({input:?})"
            );
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        // For any literal, decode∘encode is the identity — a strong cross-check
        // that the two helpers agree on the same byte grammar.
        for s in ["a b/c?d=é💩", "", "~*'()-_.!", "100% done\n", "ünîcødé"] {
            assert_eq!(
                decode_uri_component(&encode_uri_component(s)).as_deref(),
                Some(s),
                "round-trip {s:?}"
            );
        }
    }

    #[test]
    fn fold_encode_uri_component_through_pass() {
        // `encodeURIComponent("a b")` folds to the string literal `"a%20b"`.
        let c = global_call("encodeURIComponent", vec![string("a b", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "encodeURIComponent(\"a b\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a%20b"),
            other => panic!("expected \"a%20b\"; got {:?}", other),
        }
    }

    #[test]
    fn fold_decode_uri_component_through_pass() {
        // `decodeURIComponent("%C3%A9")` folds to the string literal `"é"`.
        let c = global_call("decodeURIComponent", vec![string("%C3%A9", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "decodeURIComponent(\"%C3%A9\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "é"),
            other => panic!("expected \"é\"; got {:?}", other),
        }
    }

    #[test]
    fn decode_uri_component_malformed_does_not_fold() {
        // A `URIError` input (`decodeURIComponent("%")`) is left for the runtime.
        let c = global_call("decodeURIComponent", vec![string("%", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "decodeURIComponent(\"%\") must not fold (URIError)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn uri_component_non_string_argument_does_not_fold() {
        // Only a STRING-literal argument folds; `encodeURIComponent(x)` needs
        // the runtime value of `x`.
        let c = global_call("encodeURIComponent", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "encodeURIComponent(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn uri_component_extra_argument_does_not_fold() {
        // These globals take exactly one argument; a second one is unmodelled —
        // stay conservative and leave the call.
        let c = global_call(
            "encodeURIComponent",
            vec![string("a b", None), num(1.0, None)],
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "encodeURIComponent(\"a b\", 1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_uri_component_does_not_fold() {
        // `window.encodeURIComponent("a b")` is a member call, not the global
        // identifier — the global-call arm must NOT fold it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "encodeURIComponent")),
            arguments: vec![string("a b", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.encodeURIComponent(\"a b\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_boolean_through_pass_string_and_number() {
        // V8 oracle: a string is falsy only when EMPTY; a number is falsy only
        // for 0/-0. So Boolean("")→false, Boolean("0")→true (non-empty!),
        // Boolean(0)→false, Boolean(-0)→false, Boolean(1)→true.
        let cases: [(Expression, bool); 6] = [
            (global_call("Boolean", vec![string("", None)]), false),
            (global_call("Boolean", vec![string("x", None)]), true),
            (global_call("Boolean", vec![string("0", None)]), true), // non-empty
            (global_call("Boolean", vec![num(0.0, None)]), false),
            (global_call("Boolean", vec![num(-0.0, None)]), false),
            (global_call("Boolean", vec![num(1.0, None)]), true),
        ];
        for (c, expect) in cases {
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Boolean(...) should fold to {expect}");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => assert_eq!(b.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_string_of_number_direct_oracle() {
        // (input, expected) — folded values confirmed against V8's `String(n)`;
        // fractional and ≥2^53 inputs DECLINE (we never risk a tie-break mis-fold).
        for (input, expect) in [
            (0.0, Some("0".to_string())),       // +0 → "0"
            (-0.0, Some("0".to_string())),      // -0 → "0"
            (42.0, Some("42".to_string())),     // integer
            (-3.0, Some("-3".to_string())),     // negative integer
            (255.0, Some("255".to_string())),   // integer
            (1000000.0, Some("1000000".to_string())), // 1e6 integer
            (0.5, None),    // fractional → decline (Rust/V8 tie-break can diverge)
            (3.14, None),   // fractional → decline
            (-2.5, None),   // fractional → decline
            (108868734838530.12, None), // the reviewer's known off-by-one → decline
            (1e20, None),   // integer but ≥ 2^53 → decline (conservative)
            (1e21, None),   // ≥ 2^53; V8 exponential anyway → decline
        ] {
            assert_eq!(fold_string_of_number(input), expect, "String({input})");
        }
    }

    #[test]
    fn fold_number_direct_oracle() {
        // (input, expected) — every value confirmed against V8's `Number(x)`.
        for (input, expect) in [
            ("42", Some(42.0)),        // plain decimal
            ("", Some(0.0)),           // empty → +0 (NOT NaN, unlike parseFloat)
            ("   ", Some(0.0)),        // all-whitespace → +0
            ("  3.5 ", Some(3.5)),     // surrounding whitespace trimmed
            ("0x1F", Some(31.0)),      // hex
            ("0X1f", Some(31.0)),      // hex, mixed case
            ("0b101", Some(5.0)),      // binary
            ("0o17", Some(15.0)),      // octal
            ("017", Some(17.0)),       // leading zero is DECIMAL, not octal
            (".5", Some(0.5)),         // leading dot
            ("5.", Some(5.0)),         // trailing dot
            ("1e3", Some(1000.0)),     // exponent
            ("2.5e-3", Some(0.0025)),  // signed exponent
            ("-7", Some(-7.0)),        // negative decimal
            ("+9", Some(9.0)),         // explicit positive
            ("abc", None),             // not numeric → NaN → decline
            ("1,2", None),             // stray comma → NaN → decline
            ("12px", None),            // trailing garbage → NaN (Number is total)
            ("1_000", None),           // underscore separators → NaN
            ("1e", None),              // dangling exponent → NaN
            ("Infinity", None),        // ∞ has no literal → decline
            ("-Infinity", None),       // signed ∞ → decline
            ("1e400", None),           // overflows to ∞ → decline
            ("0x", None),              // prefix with no digits → NaN
            ("0x+1", None),            // sign inside hex → NaN (no mis-fold)
            ("-0x1F", None),           // sign before hex → NaN
        ] {
            assert_eq!(fold_number(input), expect, "Number({input:?})");
        }
    }

    #[test]
    fn boolean_non_literal_argument_does_not_fold() {
        // Only string/number LITERAL args fold; `Boolean(x)` needs runtime `x`,
        // and a boolean/null literal we conservatively leave alone.
        for arg in [ident("x"), Expression::NullLiteral(NullLiteral { cv: None })] {
            let c = global_call("Boolean", vec![arg]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Boolean(non-string/number-literal) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn fold_string_number_through_pass() {
        // `String(42)` folds to the string literal `"42"`.
        let c = global_call("String", vec![num(42.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "String(42) should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "42"),
            other => panic!("expected \"42\"; got {:?}", other),
        }
    }

    #[test]
    fn boolean_with_second_argument_does_not_fold() {
        // We model only the single-argument form; `Boolean("x", y)` is left.
        let c = global_call("Boolean", vec![string("x", None), ident("y")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Boolean(\"x\", y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_number_through_pass() {
        // `Number("0x1F")` folds to the numeric literal `31`.
        let c = global_call("Number", vec![string("0x1F", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Number(\"0x1F\") should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 31.0),
            other => panic!("expected 31; got {:?}", other),
        }
    }

    #[test]
    fn fold_string_identity_through_pass() {
        // `String("x")` is the identity on a string literal → `"x"`.
        let c = global_call("String", vec![string("x", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "String(\"x\") should fold to \"x\"");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "x"),
            other => panic!("expected \"x\"; got {:?}", other),
        }
    }

    #[test]
    fn fold_number_empty_string_folds_to_zero() {
        // `Number("")` is `0`, not NaN — the one shape that surprises people.
        let c = global_call("Number", vec![string("", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Number(\"\") should fold to 0");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {:?}", other),
        }
    }

    #[test]
    fn string_exponential_number_does_not_fold() {
        // `String(1e21)` is "1e+21" in V8 — exponential notation Rust won't
        // produce — so we decline and leave the call intact.
        let c = global_call("String", vec![num(1e21, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "String(1e21) must not fold (exponential)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_nan_result_does_not_fold() {
        // `Number("abc")` is NaN — no literal token, so the call survives.
        let c = global_call("Number", vec![string("abc", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Number(\"abc\") must not fold (NaN)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_boolean_does_not_fold() {
        // `window.Boolean("")` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "Boolean")),
            arguments: vec![string("", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.Boolean(\"\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn string_non_literal_argument_does_not_fold() {
        // Only string/number LITERAL args fold; `String(x)` needs runtime `x`.
        let c = global_call("String", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "String(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_with_second_argument_does_not_fold() {
        // We only model the single-argument form; `Number("5", x)` is left alone.
        let c = global_call("Number", vec![string("5", None), ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Number(\"5\", x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn string_with_second_argument_does_not_fold() {
        // We model only the single-argument form; `String(5, x)` is left alone.
        let c = global_call("String", vec![num(5.0, None), ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "String(5, x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_string_does_not_fold() {
        // `window.String(5)` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "String")),
            arguments: vec![num(5.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.String(5) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn escape_js_direct_oracle() {
        // (input, output) — every result confirmed against V8's `escape`.
        for (input, expect) in [
            ("a b", "a%20b"),           // space → %20
            ("", ""),                   // empty
            ("abcABC123", "abcABC123"), // alphanumerics kept
            ("@*_+-./", "@*_+-./"),     // every unescaped mark kept verbatim
            ("~", "%7E"),               // tilde is NOT in escape's set (unlike …Component)
            ("<", "%3C"),               // unsafe ASCII
            ("\n", "%0A"),              // control char
            ("é", "%E9"),               // U+00E9 < 0x100 → %XX of the code unit
            ("中", "%u4E2D"),           // U+4E2D ≥ 0x100 → %uXXXX
            ("😀", "%uD83D%uDE00"),     // one astral scalar → two surrogate units
        ] {
            assert_eq!(escape_js(input), expect, "escape({input:?})");
        }
    }

    #[test]
    fn unescape_js_direct_oracle() {
        // (input, expected) — confirmed against V8's `unescape`; `None` = decline.
        for (input, expect) in [
            ("a%20b", Some("a b".to_string())),
            ("%E9", Some("é".to_string())),
            ("%2F", Some("/".to_string())), // EVERY escape decodes (unlike decodeURI)
            ("%u4E2D", Some("中".to_string())),
            ("%uD83D%uDE00", Some("😀".to_string())),
            ("abc", Some("abc".to_string())), // no escapes
            ("%", Some("%".to_string())),     // lone % passes through literally
            ("%u", Some("%u".to_string())),   // truncated %u → literal
            ("%G0", Some("%G0".to_string())), // non-hex digit → literal
            ("%4", Some("%4".to_string())),   // truncated %X → literal
            ("100%", Some("100%".to_string())), // trailing lone %
            ("%uD83D", None),                 // unpaired surrogate → decline
        ] {
            assert_eq!(unescape_js(input), expect, "unescape({input:?})");
        }
    }

    #[test]
    fn escape_unescape_round_trip() {
        // `unescape(escape(s)) == s` for any string-literal value (whole scalars).
        for s in ["a b/c?<>", "é😀中", "@*_+-./~", "100%25", "plain"] {
            let escaped = escape_js(s);
            assert_eq!(unescape_js(&escaped).as_deref(), Some(s), "round-trip {s:?}");
        }
    }

    #[test]
    fn fold_escape_through_pass() {
        // `escape("a b")` folds to the string literal `"a%20b"`.
        let c = global_call("escape", vec![string("a b", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "escape(\"a b\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a%20b"),
            other => panic!("expected \"a%20b\"; got {:?}", other),
        }
    }

    #[test]
    fn fold_unescape_through_pass() {
        // `unescape("a%20b")` folds to the string literal `"a b"`.
        let c = global_call("unescape", vec![string("a%20b", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "unescape(\"a%20b\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a b"),
            other => panic!("expected \"a b\"; got {:?}", other),
        }
    }

    #[test]
    fn unescape_unpaired_surrogate_does_not_fold() {
        // `unescape("%uD83D")` yields a lone high surrogate — unrepresentable as a
        // Rust string literal — so the call is left for the runtime.
        let c = global_call("unescape", vec![string("%uD83D", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "unescape(\"%uD83D\") must not fold (unpaired surrogate)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn escape_non_string_argument_does_not_fold() {
        // Only a string LITERAL arg folds; `escape(x)` needs the runtime value.
        let c = global_call("escape", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "escape(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn escape_with_second_argument_does_not_fold() {
        // We model only the single-argument form; `escape("x", y)` is left alone.
        let c = global_call("escape", vec![string("x", None), ident("y")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "escape(\"x\", y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_escape_does_not_fold() {
        // `window.escape("x")` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "escape")),
            arguments: vec![string("x", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.escape(\"x\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_number_does_not_fold() {
        // `window.Number("5")` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "Number")),
            arguments: vec![string("5", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.Number(\"5\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn js_to_number_direct_oracle() {
        // (input, is_nan, is_finite) — every classification confirmed against
        // V8's `Number(input)` ToNumber coercion.
        for (input, nan, fin) in [
            ("42", false, true),
            ("", false, true),         // empty → +0
            ("   ", false, true),      // all-whitespace → +0
            ("  3.5 ", false, true),   // surrounding whitespace trimmed
            ("1e3", false, true),      // exponent
            ("0x1F", false, true),     // hex
            ("0b101", false, true),    // binary
            ("0o17", false, true),     // octal
            ("-7", false, true),       // negative
            ("+9", false, true),       // explicit positive
            (".5", false, true),       // leading dot
            ("5.", false, true),       // trailing dot
            ("abc", true, false),      // not numeric → NaN
            ("12px", true, false),     // trailing garbage → NaN (total coercion)
            ("1,2", true, false),      // stray comma → NaN
            ("0x", true, false),       // prefix, no digits → NaN
            ("0x+1", true, false),     // sign inside hex → NaN
            ("Infinity", false, false), // +∞
            ("+Infinity", false, false),
            ("-Infinity", false, false), // -∞
            ("1e400", false, false),   // overflow → +∞
            ("0xfffffffffffffffffffff", false, true), // huge but finite hex
        ] {
            let v = js_to_number(input);
            assert_eq!(v.is_nan(), nan, "is_nan(Number({input:?}))");
            assert_eq!(v.is_finite(), fin, "is_finite(Number({input:?}))");
        }
    }

    #[test]
    fn encode_uri_direct_oracle() {
        // Each output confirmed against V8's `encodeURI`. The reserved URI
        // delimiters pass through untouched; only the genuinely unsafe bytes
        // (space, non-ASCII, and `< > " { } | \ ^ [ ]`) get percent-escaped.
        for (input, expect) in [
            ("a b", "a%20b"),                        // space → %20
            ("abc-_.!~*'()", "abc-_.!~*'()"),        // unreserved marks kept
            ("a;,/?:@&=+$#b", "a;,/?:@&=+$#b"),       // reserved delimiters kept
            ("é", "%C3%A9"),                          // 2-byte scalar, per byte
            ("a<b>", "a%3Cb%3E"),                     // < > escaped
            ("\"q\"", "%22q%22"),                     // double quote escaped
            ("{x}", "%7Bx%7D"),                        // braces escaped
            ("100%", "100%25"),                        // bare % escaped
            ("a[b]", "a%5Bb%5D"),                      // brackets escaped
        ] {
            assert_eq!(encode_uri(input), expect, "encodeURI({input:?})");
        }
    }

    #[test]
    fn js_to_number_exact_finite_values() {
        // A few exact values (the fold only reads the class, but pin the math).
        assert_eq!(js_to_number("42"), 42.0);
        assert_eq!(js_to_number("0x1F"), 31.0);
        assert_eq!(js_to_number("-2.5"), -2.5);
        assert_eq!(js_to_number(""), 0.0);
        assert_eq!(js_to_number("  10  "), 10.0);
    }

    #[test]
    fn fold_isnan_through_pass() {
        // `isNaN("abc")` folds to the boolean `true` (emitted as `!0`).
        let c = global_call("isNaN", vec![string("abc", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "isNaN(\"abc\") should fold");
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(b.value, "isNaN(\"abc\") → true"),
            other => panic!("expected true; got {:?}", other),
        }
    }

    #[test]
    fn decode_uri_direct_oracle() {
        // Each output confirmed against V8's `decodeURI`. Escapes of reserved
        // delimiters stay ENCODED (the whole point vs decodeURIComponent);
        // escapes of non-reserved bytes decode; malformed/invalid → None.
        for (input, expect) in [
            ("a%20b", Some("a b".to_string())),       // %20 not reserved → decoded
            ("%C3%A9", Some("é".to_string())),         // two bytes → one scalar
            ("%41", Some("A".to_string())),            // 'A' not reserved → decoded
            ("%25", Some("%".to_string())),            // '%' not reserved → decoded
            ("%2F", Some("%2F".to_string())),          // '/' reserved → kept encoded
            ("%3B%2C%3F%3A%40%26%3D%2B%24%23", Some("%3B%2C%3F%3A%40%26%3D%2B%24%23".to_string())), // ; , ? : @ & = + $ # all reserved → kept
            ("a/b", Some("a/b".to_string())),          // literal reserved passes through
            ("%", None),                                // truncated → URIError
            ("%2", None),                               // truncated → URIError
            ("%G0", None),                              // non-hex → URIError
            ("%80", None),                              // lone continuation byte → bad UTF-8
            ("%C3%41", None),                           // lead byte + non-continuation → bad UTF-8
        ] {
            assert_eq!(decode_uri(input), expect, "decodeURI({input:?})");
        }
    }

    #[test]
    fn encode_uri_decode_uri_round_trip() {
        // encodeURI never escapes a reserved delimiter, so its output's only
        // `%XX` escapes are of non-reserved bytes — which decodeURI fully
        // decodes — so encode∘decode is the identity on every literal.
        for s in ["a b/c?d=e&f", "héllo wörld", "100% sure!", "a,b;c#frag", "[x]<y>{z}"] {
            assert_eq!(decode_uri(&encode_uri(s)).as_deref(), Some(s), "round-trip {s:?}");
        }
    }

    #[test]
    fn fold_encode_uri_through_pass() {
        // `encodeURI("a b")` folds to the string literal `"a%20b"`.
        let c = global_call("encodeURI", vec![string("a b", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "encodeURI(\"a b\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a%20b"),
            other => panic!("expected \"a%20b\"; got {:?}", other),
        }
    }

    #[test]
    fn fold_isfinite_through_pass() {
        // `isFinite("1e3")` folds to the boolean `true`.
        let c = global_call("isFinite", vec![string("1e3", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "isFinite(\"1e3\") should fold");
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(b.value, "isFinite(\"1e3\") → true"),
            other => panic!("expected true; got {:?}", other),
        }
    }

    #[test]
    fn fold_decode_uri_through_pass() {
        // `decodeURI("a%20b")` folds to the string literal `"a b"`.
        let c = global_call("decodeURI", vec![string("a%20b", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "decodeURI(\"a%20b\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "a b"),
            other => panic!("expected \"a b\"; got {:?}", other),
        }
    }

    #[test]
    fn fold_isnan_isfinite_number_literals() {
        // A NUMBER literal coerces to itself: isNaN(0) → false, isFinite(0) → true.
        for (name, expect) in [("isNaN", false), ("isFinite", true)] {
            let c = global_call(name, vec![num(0.0, None)]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "{name}(0) should fold");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => assert_eq!(b.value, expect, "{name}(0)"),
                other => panic!("expected bool; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_isnan_isfinite_infinity_string() {
        // `"Infinity"` is a NUMBER (not NaN): isNaN → false, isFinite → false.
        for (name, expect) in [("isNaN", false), ("isFinite", false)] {
            let c = global_call(name, vec![string("Infinity", None)]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "{name}(\"Infinity\") should fold");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => {
                    assert_eq!(b.value, expect, "{name}(\"Infinity\")")
                }
                other => panic!("expected bool; got {:?}", other),
            }
        }
    }

    #[test]
    fn isnan_non_literal_argument_does_not_fold() {
        // Only string/number LITERAL args fold; `isNaN(x)` needs the runtime `x`.
        let c = global_call("isNaN", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "isNaN(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn decode_uri_malformed_does_not_fold() {
        // `decodeURI("%E0")` is a truncated 3-byte lead → invalid UTF-8 → the
        // runtime throws URIError, so we DECLINE and leave the call intact.
        let c = global_call("decodeURI", vec![string("%E0", None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "decodeURI(\"%E0\") must not fold (URIError)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn isfinite_with_second_argument_does_not_fold() {
        // We model only the single-argument form; `isFinite("1", y)` is left.
        let c = global_call("isFinite", vec![string("1", None), ident("y")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "isFinite(\"1\", y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn uri_non_string_argument_does_not_fold() {
        // Only a string LITERAL folds; `encodeURI(x)` needs the runtime value.
        let c = global_call("encodeURI", vec![ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "encodeURI(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_isnan_does_not_fold() {
        // `window.isNaN("abc")` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "isNaN")),
            arguments: vec![string("abc", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.isNaN(\"abc\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn uri_extra_argument_does_not_fold() {
        // We model only the single-argument form; `encodeURI("a", x)` is left.
        let c = global_call("encodeURI", vec![string("a", None), ident("x")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "encodeURI(\"a\", x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn member_uri_does_not_fold() {
        // `window.encodeURI("a b")` is a member call, not the bare global — leave it.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("window"), "encodeURI")),
            arguments: vec![string("a b", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "window.encodeURI(\"a b\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn trim_with_argument_does_not_fold() {
        // An argument keeps us conservative (trim ignores it, but we don't model
        // that): `"  x  ".trim(1)` survives.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("  x  ", None), "trim")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "\"  x  \".trim(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- string indexing (charCodeAt / charAt) -------

    /// Build a one-argument method call `<object>.<name>(<arg>)`.
    fn call1(object: Expression, name: &str, arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(object, name)),
            arguments: vec![arg],
        })
    }

    fn call2(object: Expression, name: &str, a: Expression, b: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(object, name)),
            arguments: vec![a, b],
        })
    }

    /// Run the pass and assert the result is an `ArrayExpression` of string
    /// literals, returning their values for comparison against the V8 oracle.
    fn split_parts(expr: Expression) -> Vec<String> {
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "split should have folded");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => a
                .elements
                .iter()
                .map(|e| match e {
                    Some(Expression::StringLiteral(s)) => s.value.clone(),
                    other => panic!("expected string element; got {:?}", other),
                })
                .collect(),
            other => panic!("expected ArrayExpression; got {:?}", other),
        }
    }

    #[test]
    fn fold_split_non_empty_separator() {
        // V8: "a,b,c".split(",") → ["a","b","c"]
        assert_eq!(
            split_parts(call1(string("a,b,c", None), "split", string(",", None))),
            vec!["a", "b", "c"]
        );
        // V8: "axbxc".split("x") → ["a","b","c"]
        assert_eq!(
            split_parts(call1(string("axbxc", None), "split", string("x", None))),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn fold_split_empty_separator_is_per_code_unit() {
        // V8: "abc".split("") → ["a","b","c"]
        assert_eq!(
            split_parts(call1(string("abc", None), "split", string("", None))),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn fold_split_empty_receiver() {
        // V8: "".split(",") → [""]  (one empty piece — nothing was found)
        assert_eq!(
            split_parts(call1(string("", None), "split", string(",", None))),
            vec![""]
        );
        // V8: "".split("") → []  (zero pieces)
        assert_eq!(
            split_parts(call1(string("", None), "split", string("", None))),
            Vec::<String>::new()
        );
    }

    #[test]
    fn fold_split_no_separator_is_whole_string() {
        // V8: "abc".split() → ["abc"]
        assert_eq!(
            split_parts(call0(string("abc", None), "split")),
            vec!["abc"]
        );
    }

    #[test]
    fn fold_split_separator_absent_is_whole_string() {
        // V8: "abc".split("x") → ["abc"]  (separator never occurs)
        assert_eq!(
            split_parts(call1(string("abc", None), "split", string("x", None))),
            vec!["abc"]
        );
    }

    #[test]
    fn fold_split_with_limit() {
        // V8: "a,b,c".split(",", 2) → ["a","b"]
        assert_eq!(
            split_parts(call2(
                string("a,b,c", None),
                "split",
                string(",", None),
                num(2.0, None)
            )),
            vec!["a", "b"]
        );
        // V8: "a,b,c".split(",", 0) → []
        assert_eq!(
            split_parts(call2(
                string("a,b,c", None),
                "split",
                string(",", None),
                num(0.0, None)
            )),
            Vec::<String>::new()
        );
    }

    #[test]
    fn split_astral_empty_separator_does_not_fold() {
        // "💩a".split("") in V8 → ["\uD83D","\uDCA9","a"]: the pile-of-poo is a
        // surrogate pair that splits into two LONE surrogates, which no Rust
        // String can hold. We must decline (leave the call for the runtime).
        let c = call1(string("💩a", None), "split", string("", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "empty-separator split of an astral string must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn split_astral_non_empty_separator_still_folds() {
        // A non-empty separator never cuts inside a surrogate pair, so an astral
        // receiver is fine: "a💩b".split("💩") → ["a","b"].
        assert_eq!(
            split_parts(call1(string("a💩b", None), "split", string("💩", None))),
            vec!["a", "b"]
        );
    }

    #[test]
    fn split_non_string_separator_does_not_fold() {
        // A numeric separator would need ToString coercion we don't model.
        let c = call1(string("a1b", None), "split", num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "non-string-literal separator must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn split_bad_limit_does_not_fold() {
        // Negative, fractional, and non-literal limits all decline.
        for bad in [num(-1.0, None), num(1.5, None)] {
            let c = call2(string("a,b,c", None), "split", string(",", None), bad);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "bad limit must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn split_helper_unit_oracle() {
        // Direct unit checks of the pure helper against the V8 oracle.
        let sl = |v: &str| string(v, None);
        assert_eq!(
            fold_string_split("a,b,c", &[sl(",")]),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
        assert_eq!(fold_string_split("", &[sl("")]), Some(vec![]));
        assert_eq!(fold_string_split("", &[sl(",")]), Some(vec!["".to_string()]));
        assert_eq!(fold_string_split("abc", &[]), Some(vec!["abc".to_string()]));
        // astral + empty separator declines; three+ args declines.
        assert_eq!(fold_string_split("💩", &[sl("")]), None);
        assert_eq!(fold_string_split("a", &[sl(","), num(1.0, None), sl("x")]), None);
    }

    #[test]
    fn fold_char_code_at_in_range() {
        // "abc".charCodeAt(0) → 97, .charCodeAt(2) → 99
        for (idx, expect) in [(0.0, 97.0), (2.0, 99.0)] {
            let c = call1(string("abc", None), "charCodeAt", num(idx, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"abc\".charCodeAt({idx}) should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn char_code_at_out_of_range_does_not_fold() {
        // JS `"abc".charCodeAt(5)` is NaN — no literal, so don't fold.
        let c = call1(string("abc", None), "charCodeAt", num(5.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "out-of-range charCodeAt must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_char_code_at_counts_utf16_units() {
        // "💩" is the surrogate pair [0xD83D, 0xDCA9]; charCodeAt(0) is the
        // high surrogate 55357, proving we index UTF-16 code units (not scalars).
        let c = call1(string("💩", None), "charCodeAt", num(0.0, None));
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 55357.0),
            other => panic!("expected 55357 (high surrogate); got {:?}", other),
        }
    }

    #[test]
    fn fold_code_point_at_bmp_matches_char_code_at() {
        // For a BMP-only string, codePointAt and charCodeAt agree:
        // "abc".codePointAt(0) → 97, .codePointAt(2) → 99 (V8 oracle).
        for (idx, expect) in [(0.0, 97.0), (2.0, 99.0)] {
            let c = call1(string("abc", None), "codePointAt", num(idx, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"abc\".codePointAt({idx}) should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_code_point_at_combines_surrogate_pair() {
        // "a💩b" units = [0x61, 0xD83D, 0xDCA9, 0x62]. codePointAt(1) starts on
        // the high surrogate and combines the pair into U+1F4A9 = 128169 — the
        // defining difference from charCodeAt(1), which would give 55357 (V8).
        let c = call1(string("a💩b", None), "codePointAt", num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"a💩b\".codePointAt(1) should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 128169.0),
            other => panic!("expected 128169 (astral code point); got {:?}", other),
        }
    }

    #[test]
    fn fold_code_point_at_lone_low_surrogate_is_unit_value() {
        // codePointAt landing on the SECOND half of a pair (the low surrogate,
        // not preceded here at this index by a high one) returns the bare code
        // unit — "💩".codePointAt(1) → 0xDCA9 = 56489, same as charCodeAt(1) (V8).
        let c = call1(string("💩", None), "codePointAt", num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"💩\".codePointAt(1) should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 56489.0),
            other => panic!("expected 56489 (lone low surrogate); got {:?}", other),
        }
    }

    #[test]
    fn code_point_at_out_of_range_does_not_fold() {
        // JS `"abc".codePointAt(5)` is `undefined` — no literal, so don't fold.
        let c = call1(string("abc", None), "codePointAt", num(5.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "out-of-range codePointAt must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_char_at_in_and_out_of_range() {
        // "abc".charAt(1) → "b"
        let c = call1(string("abc", None), "charAt", num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"abc\".charAt(1) should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "b"),
            other => panic!("expected \"b\"; got {:?}", other),
        }

        // out of range → "" (JS semantics)
        let c = call1(string("abc", None), "charAt", num(9.0, None));
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, ""),
            other => panic!("expected empty string; got {:?}", other),
        }
    }

    #[test]
    fn char_at_on_lone_surrogate_does_not_fold() {
        // "💩".charAt(0) is a length-1 JS string holding a lone high surrogate,
        // which a Rust `String` can't represent — so we leave the call alone.
        let c = call1(string("💩", None), "charAt", num(0.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "charAt yielding a lone surrogate must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn non_integer_or_negative_index_does_not_fold() {
        // Fractional index: leave for the runtime (ToInteger coercion).
        let frac = call1(string("abc", None), "charCodeAt", num(0.5, None));
        let (out, _, changed, _) = run_pass(program_with_expr(frac, true));
        assert!(!changed, "fractional index must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));

        // Negative index: charCodeAt(-1) is NaN, charAt(-1) is "" — stay
        // conservative and don't fold either.
        let neg = call1(string("abc", None), "charCodeAt", num(-1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(neg, true));
        assert!(!changed, "negative index must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn char_index_on_identifier_does_not_fold() {
        // `s.charCodeAt(0)` needs the runtime value of `s`.
        let c = call1(ident("s"), "charCodeAt", num(0.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.charCodeAt(0) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- string .at(i) (negative-from-end) -----------

    #[test]
    fn fold_string_at_positive_and_negative() {
        // V8 oracle: "abc".at(0)="a", at(2)="c", at(-1)="c", at(-3)="a".
        for (idx, expect) in [(0.0, "a"), (2.0, "c"), (-1.0, "c"), (-3.0, "a")] {
            let c = call1(string("abc", None), "at", num(idx, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"abc\".at({idx}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(s.value, expect, "\"abc\".at({idx})"),
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn at_out_of_range_does_not_fold() {
        // JS `"abc".at(5)` and `"abc".at(-5)` are `undefined` — no literal,
        // so we decline rather than invent `""` (which is `charAt`'s behavior,
        // not `at`'s).
        for idx in [5.0, -5.0] {
            let c = call1(string("abc", None), "at", num(idx, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "out-of-range \"abc\".at({idx}) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn at_counts_utf16_units() {
        // "a💩b" is [a, D83D, DCA9, b] in UTF-16; index 3 is the trailing "b",
        // and at(-1) is also "b" — proving we index code units, not scalars.
        for idx in [3.0, -1.0] {
            let c = call1(string("a💩b", None), "at", num(idx, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"a💩b\".at({idx}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(s.value, "b", "\"a💩b\".at({idx})"),
                other => panic!("expected \"b\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn at_on_lone_surrogate_does_not_fold() {
        // "💩".at(0) is a length-1 JS string holding a lone high surrogate,
        // which a Rust `String` can't hold — leave the call alone (like charAt).
        let c = call1(string("💩", None), "at", num(0.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "at yielding a lone surrogate must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn at_fractional_index_does_not_fold() {
        // JS `"abc".at(1.5)` is `"b"` via ToIntegerOrInfinity, but we don't
        // model that coercion — leave it for the runtime.
        let c = call1(string("abc", None), "at", num(1.5, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "fractional at index must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn at_huge_index_does_not_overflow_or_fold() {
        // A huge negative literal — `as i64` saturates to i64::MIN and
        // `saturating_add` keeps `len + i` from overflowing; the index lands
        // out of range and declines (no panic).
        let c = call1(string("abc", None), "at", num(-1e18, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "huge negative at index must not fold (no overflow)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn at_on_identifier_does_not_fold() {
        // `s.at(0)` needs the runtime value of `s`.
        let c = call1(ident("s"), "at", num(0.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.at(0) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- numeric toString([radix]) -------------------

    /// Fold `call` and return the resulting string literal's value, or `None`
    /// if the pass left the call unchanged.
    fn folded_string(call: Expression) -> Option<String> {
        let (out, _, changed, _) = run_pass(program_with_expr(call, true));
        if !changed {
            return None;
        }
        match extract_expr(&out) {
            Expression::StringLiteral(s) => Some(s.value.clone()),
            other => panic!("expected a string literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_number_to_string_default_radix() {
        // `(255).toString()` → "255" (radix 10).
        let c = call0(num(255.0, None), "toString");
        assert_eq!(folded_string(c).as_deref(), Some("255"));
        // Zero is a valid receiver.
        let z = call0(num(0.0, None), "toString");
        assert_eq!(folded_string(z).as_deref(), Some("0"));
    }

    #[test]
    fn fold_number_to_string_with_radix() {
        // Hex, binary, and base-36, matching V8.
        for (value, radix, expect) in [
            (255.0, 16.0, "ff"),
            (255.0, 2.0, "11111111"),
            (35.0, 36.0, "z"),
            (10.0, 2.0, "1010"),
        ] {
            let c = call1(num(value, None), "toString", num(radix, None));
            assert_eq!(
                folded_string(c).as_deref(),
                Some(expect),
                "({value}).toString({radix})",
            );
        }
    }

    #[test]
    fn number_to_string_out_of_range_radix_does_not_fold() {
        // Radix must be 2..=36; 37 and 1 are RangeErrors at runtime, so we
        // leave the call alone rather than invent a result.
        for bad in [1.0, 37.0, 0.0] {
            let c = call1(num(255.0, None), "toString", num(bad, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "radix {bad} is out of range and must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn number_to_string_non_integer_receiver_does_not_fold() {
        // `(3.5).toString(2)` is a binary *fraction* ("11.1"); we don't model
        // that, so a fractional receiver passes through.
        let c = call1(num(3.5, None), "toString", num(2.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "fractional receiver must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_to_string_variable_radix_does_not_fold() {
        // A non-literal radix (`(255).toString(r)`) is unknown at compile time.
        let c = call1(num(255.0, None), "toString", ident("r"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "variable radix must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn to_radix_string_matches_known_values() {
        // Direct unit coverage of the digit-loop helper.
        assert_eq!(to_radix_string(0, 16), "0");
        assert_eq!(to_radix_string(255, 16), "ff");
        assert_eq!(to_radix_string(255, 10), "255");
        assert_eq!(to_radix_string(255, 2), "11111111");
        assert_eq!(to_radix_string(35, 36), "z");
    }

    // ------------------- indexOf (substring search) ------------------

    #[test]
    fn fold_index_of_found_and_not_found() {
        // `"abcabc".indexOf("b")` → 1 (first occurrence); absent needle → -1.
        for (hay, needle, expect) in [("abcabc", "b", 1.0), ("abc", "z", -1.0)] {
            let c = call1(string(hay, None), "indexOf", string(needle, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{hay}\".indexOf(\"{needle}\") should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_index_of_empty_needle_is_zero() {
        // JS `"abc".indexOf("")` is 0, matching Rust `str::find("")` → Some(0).
        let c = call1(string("abc", None), "indexOf", string("", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "indexOf of the empty string should fold to 0");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {:?}", other),
        }
    }

    #[test]
    fn fold_index_of_counts_utf16_units_not_bytes() {
        // "💩" is one astral char = two UTF-16 code units (and four UTF-8
        // bytes). `"💩x".indexOf("x")` must be 2 (UTF-16 index), NOT 1 (char
        // index) or 4 (byte index) — proving we re-measure the prefix in
        // UTF-16, exactly like V8.
        let c = call1(string("💩x", None), "indexOf", string("x", None));
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2 (UTF-16 index); got {:?}", other),
        }
    }

    #[test]
    fn index_of_with_from_index_arg_does_not_fold() {
        // The two-argument `fromIndex` overload lands in the 2-arg arm and is
        // left for the runtime (we only fold the single-argument form).
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abcabc", None), "indexOf")),
            arguments: vec![string("b", None), num(2.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "two-arg indexOf(needle, fromIndex) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn index_of_on_identifier_receiver_does_not_fold() {
        // `s.indexOf("x")` needs the runtime value of `s`.
        let c = call1(ident("s"), "indexOf", string("x", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.indexOf(\"x\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- lastIndexOf (search from the end) -----------

    #[test]
    fn fold_last_index_of_found_and_not_found() {
        // V8 oracle (node):
        //   "abcabc".lastIndexOf("bc") === 4 (the *last* "bc")
        //   "abcabc".lastIndexOf("b")  === 4
        //   "abc".lastIndexOf("z")     === -1
        for (hay, needle, expect) in [
            ("abcabc", "bc", 4.0),
            ("abcabc", "b", 4.0),
            ("abc", "z", -1.0),
        ] {
            let c = call1(string(hay, None), "lastIndexOf", string(needle, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{hay}\".lastIndexOf(\"{needle}\") should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_last_index_of_empty_needle_is_length() {
        // JS `"abc".lastIndexOf("")` is the string *length* (3), not 0 — the
        // empty string matches at every position and lastIndexOf takes the
        // highest. Rust `str::rfind("")` returns `Some(byte_len)`, whose UTF-16
        // re-measure is exactly that length.
        let c = call1(string("abc", None), "lastIndexOf", string("", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "lastIndexOf of the empty string should fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.0),
            other => panic!("expected 3 (length); got {:?}", other),
        }
    }

    #[test]
    fn fold_last_index_of_counts_utf16_units_not_bytes() {
        // "💩" is one astral char = two UTF-16 code units (four UTF-8 bytes).
        // `"💩x💩x".lastIndexOf("x")` must be 5 (UTF-16 index of the last "x"),
        // NOT 3 (char index) or 9 (byte index) — proving we re-measure the
        // prefix in UTF-16, exactly like V8.
        let c = call1(string("💩x💩x", None), "lastIndexOf", string("x", None));
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 5.0),
            other => panic!("expected 5 (UTF-16 index); got {:?}", other),
        }
    }

    #[test]
    fn last_index_of_with_from_index_arg_does_not_fold() {
        // The two-argument `fromIndex` overload lands in the 2-arg arm and is
        // left for the runtime (we only fold the single-argument form).
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abcabc", None), "lastIndexOf")),
            arguments: vec![string("b", None), num(0.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "two-arg lastIndexOf(needle, fromIndex) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn last_index_of_on_identifier_receiver_does_not_fold() {
        // `s.lastIndexOf("x")` needs the runtime value of `s`.
        let c = call1(ident("s"), "lastIndexOf", string("x", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.lastIndexOf(\"x\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------- substring predicates (startsWith/endsWith/includes) -------

    /// Drive `call`, asserting it folds to the boolean `expect`.
    fn assert_folds_to_bool(c: Expression, expect: bool, label: &str) {
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "{label} should fold");
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert_eq!(b.value, expect, "{label}"),
            other => panic!("{label}: expected {expect}; got {:?}", other),
        }
    }

    #[test]
    fn fold_starts_with_true_and_false() {
        // V8 oracle: "abc".startsWith("a")=true, "abc".startsWith("b")=false.
        assert_folds_to_bool(
            call1(string("abc", None), "startsWith", string("a", None)),
            true,
            "\"abc\".startsWith(\"a\")",
        );
        assert_folds_to_bool(
            call1(string("abc", None), "startsWith", string("b", None)),
            false,
            "\"abc\".startsWith(\"b\")",
        );
    }

    #[test]
    fn fold_ends_with_true_and_false() {
        // V8 oracle: "abc".endsWith("c")=true, "abc".endsWith("b")=false.
        assert_folds_to_bool(
            call1(string("abc", None), "endsWith", string("c", None)),
            true,
            "\"abc\".endsWith(\"c\")",
        );
        assert_folds_to_bool(
            call1(string("abc", None), "endsWith", string("b", None)),
            false,
            "\"abc\".endsWith(\"b\")",
        );
    }

    #[test]
    fn fold_includes_true_and_false() {
        // V8 oracle: "abc".includes("b")=true, "abc".includes("x")=false.
        assert_folds_to_bool(
            call1(string("abc", None), "includes", string("b", None)),
            true,
            "\"abc\".includes(\"b\")",
        );
        assert_folds_to_bool(
            call1(string("abc", None), "includes", string("x", None)),
            false,
            "\"abc\".includes(\"x\")",
        );
    }

    #[test]
    fn predicates_empty_needle_is_always_true() {
        // The empty string is a prefix, a suffix, and a substring of every
        // string — `"abc".startsWith("")`, `.endsWith("")`, `.includes("")`
        // are all `true` in V8, matching `str`.
        for method in ["startsWith", "endsWith", "includes"] {
            assert_folds_to_bool(
                call1(string("abc", None), method, string("", None)),
                true,
                method,
            );
        }
    }

    #[test]
    fn predicates_match_across_astral_chars() {
        // "a💩b" holds an astral char (a surrogate pair in UTF-16, four UTF-8
        // bytes). Matching whole scalars agrees in both encodings, so V8 and
        // Rust both say yes here — proving we don't false-split the pair.
        assert_folds_to_bool(
            call1(string("a💩b", None), "startsWith", string("a💩", None)),
            true,
            "\"a💩b\".startsWith(\"a💩\")",
        );
        assert_folds_to_bool(
            call1(string("a💩b", None), "endsWith", string("💩b", None)),
            true,
            "\"a💩b\".endsWith(\"💩b\")",
        );
        assert_folds_to_bool(
            call1(string("a💩b", None), "includes", string("💩", None)),
            true,
            "\"a💩b\".includes(\"💩\")",
        );
    }

    #[test]
    fn predicate_with_position_arg_does_not_fold() {
        // The two-argument position overloads (`startsWith(s, pos)` etc.) land
        // in the 2-arg arm and are left for the runtime.
        for method in ["startsWith", "endsWith", "includes"] {
            let c = Expression::CallExpression(CallExpression {
                cv: Some("c.cv".to_string()),
                callee: Box::new(member(string("abc", None), method)),
                arguments: vec![string("b", None), num(1.0, None)],
            });
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "two-arg {method}(needle, pos) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn predicate_on_identifier_receiver_does_not_fold() {
        // `s.includes("x")` needs the runtime value of `s`.
        for method in ["startsWith", "endsWith", "includes"] {
            let c = call1(ident("s"), method, string("x", None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "s.{method}(\"x\") must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn predicate_with_non_string_needle_does_not_fold() {
        // A numeric search argument (`"abc".includes(1)`) is not a string
        // literal, so it isn't our case — leave it for the runtime.
        let c = call1(string("abc", None), "includes", num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "includes with a numeric arg must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- String.fromCharCode (static) ----------------

    /// Build `String.fromCharCode(<args…>)` from numeric literal arguments.
    fn from_char_code_call(args: &[f64]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("String"), "fromCharCode")),
            arguments: args.iter().map(|&a| num(a, None)).collect(),
        })
    }

    #[test]
    fn fold_from_char_code_basic_and_empty() {
        // V8 oracle: String.fromCharCode(72,73) === "HI"; fromCharCode() === "".
        for (args, expect) in [(vec![72.0, 73.0], "HI"), (vec![], ""), (vec![65.0], "A")] {
            let c = from_char_code_call(&args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "String.fromCharCode({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(s.value, expect),
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_from_char_code_assembles_surrogate_pair() {
        // Adjacent high+low surrogate units assemble the astral scalar U+1F4A9:
        // String.fromCharCode(0xD83D, 0xDCA9) === "💩" (V8).
        let c = from_char_code_call(&[0xD83D as f64, 0xDCA9 as f64]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "surrogate-pair fromCharCode should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "💩"),
            other => panic!("expected \"💩\"; got {:?}", other),
        }
    }

    #[test]
    fn from_char_code_lone_surrogate_does_not_fold() {
        // A lone high surrogate is a valid JS string but not a Rust String —
        // decline (String.fromCharCode(0xD83D) has no literal we can emit).
        let c = from_char_code_call(&[0xD83D as f64]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "lone-surrogate fromCharCode must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn from_char_code_out_of_range_or_fractional_does_not_fold() {
        // We conservatively decline rather than model ToUint16 wrap-around for
        // a fractional, negative, or >0xFFFF argument.
        for args in [vec![65.5], vec![-1.0], vec![70000.0], vec![65.0, 0.5]] {
            let c = from_char_code_call(&args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "String.fromCharCode({args:?}) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_char_code_on_non_string_receiver_does_not_fold() {
        // Only the bare global `String` folds; `s.fromCharCode(72)` (some other
        // receiver) is left untouched.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "fromCharCode")),
            arguments: vec![num(72.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "non-String receiver fromCharCode must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- String.fromCodePoint (static) ---------------

    /// Build `String.fromCodePoint(<args…>)` from numeric literal arguments.
    fn from_code_point_call(args: &[f64]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("String"), "fromCodePoint")),
            arguments: args.iter().map(|&a| num(a, None)).collect(),
        })
    }

    #[test]
    fn fold_from_code_point_bmp_astral_and_empty() {
        // V8 oracle: fromCodePoint(72,73)==="HI"; (128169)==="💩" (a SINGLE
        // astral arg, unlike fromCharCode which needs the surrogate pair);
        // ()==="".
        for (args, expect) in [
            (vec![72.0, 73.0], "HI"),
            (vec![128169.0], "💩"),
            (vec![128169.0, 65.0], "💩A"),
            (vec![], ""),
        ] {
            let c = from_code_point_call(&args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "String.fromCodePoint({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(s.value, expect),
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn from_code_point_surrogate_or_out_of_range_does_not_fold() {
        // A surrogate code point (D800..DFFF) is a valid JS arg but yields a
        // lone-surrogate string no Rust String can hold; >0x10FFFF, negative,
        // and fractional all throw RangeError in JS — decline in every case.
        for args in [
            vec![0xD83D as f64],   // lone high surrogate
            vec![0x110000 as f64], // past U+10FFFF
            vec![-1.0],
            vec![65.5],
            vec![65.0, 0.5],
        ] {
            let c = from_code_point_call(&args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "String.fromCodePoint({args:?}) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_code_point_on_non_string_receiver_does_not_fold() {
        // Only the bare global `String` folds; `s.fromCodePoint(65)` is left.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "fromCodePoint")),
            arguments: vec![num(65.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "non-String receiver fromCodePoint must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Number.parseInt/parseFloat (static) ---------

    /// Build `Number.<method>("<recv>"[, radix])`.
    fn number_parse_call(method: &str, recv: &str, radix: Option<f64>) -> Expression {
        let mut arguments = vec![string(recv, None)];
        if let Some(r) = radix {
            arguments.push(num(r, None));
        }
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Number"), method)),
            arguments,
        })
    }

    // ------------------- Number.isInteger/isFinite/isNaN (static) ----

    /// Build `Number.<method>(<arg>)`.
    fn number_static_call(method: &str, arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Number"), method)),
            arguments: vec![arg],
        })
    }

    #[test]
    fn fold_number_parse_int_through_pass() {
        // `Number.parseInt` is the same function as the global `parseInt`.
        for (recv, radix, expect) in [
            ("12px", None, 12.0),
            ("FF", Some(16.0), 255.0),
            ("0x1F", None, 31.0),
            ("-7", None, -7.0),
            ("101", Some(2.0), 5.0),
        ] {
            let c = number_parse_call("parseInt", recv, radix);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Number.parseInt({recv:?},{radix:?}) should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_number_static_on_number_literals() {
        // (method, value, expected) — every classification confirmed against V8's
        // `Number.isInteger` / `Number.isFinite` / `Number.isNaN` (NO coercion).
        for (method, v, expect) in [
            ("isInteger", 42.0, true),
            ("isInteger", -7.0, true),
            ("isInteger", 0.0, true),
            ("isInteger", 3.5, false),
            ("isInteger", 1e21, true), // huge but integer-valued f64 (≥ 2^52)
            ("isInteger", f64::INFINITY, false),
            ("isInteger", f64::NAN, false),
            ("isFinite", 42.0, true),
            ("isFinite", 3.5, true),
            ("isFinite", f64::INFINITY, false),
            ("isFinite", f64::NAN, false),
            ("isNaN", f64::NAN, true),
            ("isNaN", 42.0, false),
            ("isNaN", f64::INFINITY, false), // Infinity is a number, not NaN
        ] {
            let c = number_static_call(method, num(v, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Number.{method}({v}) should fold");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => assert_eq!(b.value, expect, "Number.{method}({v})"),
                other => panic!("expected bool; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_number_is_safe_integer() {
        // Number.isSafeInteger — true iff the value is an integer with magnitude
        // ≤ 2^53−1 (MAX_SAFE_INTEGER = 9007199254740991), with NO coercion.
        // Every classification confirmed against V8.
        for (v, expect) in [
            (7.0, true),
            (0.0, true),
            (-7.0, true),
            (9_007_199_254_740_991.0, true), // MAX_SAFE_INTEGER boundary
            (-9_007_199_254_740_991.0, true), // −MAX_SAFE_INTEGER boundary
            (9_007_199_254_740_992.0, false), // 2^53 — just past the safe range
            (3.5, false),                     // not an integer
            (1e21, false),                    // integer-valued but far past 2^53
            (f64::INFINITY, false),
            (f64::NAN, false),
        ] {
            let c = number_static_call("isSafeInteger", num(v, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Number.isSafeInteger({v}) should fold");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => {
                    assert_eq!(b.value, expect, "Number.isSafeInteger({v})")
                }
                other => panic!("expected bool; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_number_is_safe_integer_on_non_number_literals_is_false() {
        // Like the sibling predicates, a non-number literal is provably not a
        // Number, so `isSafeInteger` is `false` with no `ToNumber` coercion.
        let args = [
            string("7", None),
            Expression::BooleanLiteral(BooleanLiteral {
                cv: None,
                value: true,
            }),
            Expression::NullLiteral(NullLiteral { cv: None }),
        ];
        for arg in args {
            let c = number_static_call("isSafeInteger", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Number.isSafeInteger(non-number) should fold to false");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => assert!(!b.value, "→ false"),
                other => panic!("expected false; got {:?}", other),
            }
        }
    }

    #[test]
    fn number_is_safe_integer_identifier_does_not_fold() {
        // An identifier's type is unknown at compile time — declined.
        let c = number_static_call("isSafeInteger", ident("x"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Number.isSafeInteger(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_number_parse_float_through_pass() {
        // `Number.parseFloat` is the same function as the global `parseFloat`.
        for (recv, expect) in [("3.14abc", 3.14), ("1e3", 1000.0), ("-2.5", -2.5)] {
            let c = number_parse_call("parseFloat", recv, None);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Number.parseFloat({recv:?}) should fold");
            match extract_expr(&out) {
                Expression::NumericLiteral(n) => assert_eq!(n.value, expect),
                other => panic!("expected {expect}; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_number_static_on_non_number_literals_is_false() {
        // A non-number literal is provably NOT a Number — all three predicates
        // are `false`, with no `ToNumber` coercion (Number.isNaN("NaN") === false).
        for method in ["isInteger", "isFinite", "isNaN"] {
            let args = [
                string("42", None),
                Expression::BooleanLiteral(BooleanLiteral {
                    cv: None,
                    value: true,
                }),
                Expression::NullLiteral(NullLiteral { cv: None }),
            ];
            for arg in args {
                let c = number_static_call(method, arg);
                let (out, _, changed, _) = run_pass(program_with_expr(c, true));
                assert!(changed, "Number.{method}(non-number-literal) should fold to false");
                match extract_expr(&out) {
                    Expression::BooleanLiteral(b) => {
                        assert!(!b.value, "Number.{method}(non-number) → false")
                    }
                    other => panic!("expected false; got {:?}", other),
                }
            }
        }
    }

    #[test]
    fn number_parse_nan_result_does_not_fold() {
        // A `NaN`/`Infinity` result has no literal token — decline, like the
        // global forms (`parseInt("")`, `parseFloat("Infinity")`).
        for (method, recv) in [("parseInt", ""), ("parseFloat", "Infinity"), ("parseInt", "xyz")] {
            let c = number_parse_call(method, recv, None);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Number.{method}({recv:?}) must not fold (NaN/Infinity)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn number_parse_non_literal_or_bad_radix_does_not_fold() {
        // A non-string arg, or a non-integer-literal radix, is left alone.
        let cases = [
            number_parse_call("parseInt", "10", Some(2.5)), // fractional radix
            Expression::CallExpression(CallExpression {
                cv: Some("c.cv".to_string()),
                callee: Box::new(member(ident("Number"), "parseInt")),
                arguments: vec![ident("x")], // non-string arg
            }),
        ];
        for c in cases {
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Number.parseInt(bad) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn number_parse_on_non_number_receiver_does_not_fold() {
        // Only the bare global `Number` folds; `n.parseInt("5")` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("n"), "parseInt")),
            arguments: vec![string("5", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "n.parseInt(\"5\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- JSON.stringify (static) ---------------------

    /// Build `JSON.stringify(<arg>)`.
    fn json_stringify_call(arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("JSON"), "stringify")),
            arguments: vec![arg],
        })
    }

    #[test]
    fn fold_json_stringify_primitives() {
        // (arg, expected JSON text) — every result confirmed against V8's
        // `JSON.stringify`; the folded value is that text AS a string literal.
        let cases: [(Expression, &str); 6] = [
            (num(42.0, None), "42"),
            (num(-7.0, None), "-7"),
            (num(0.0, None), "0"),
            (
                Expression::BooleanLiteral(BooleanLiteral {
                    cv: None,
                    value: true,
                }),
                "true",
            ),
            (
                Expression::BooleanLiteral(BooleanLiteral {
                    cv: None,
                    value: false,
                }),
                "false",
            ),
            (Expression::NullLiteral(NullLiteral { cv: None }), "null"),
        ];
        for (arg, expect) in cases {
            let c = json_stringify_call(arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "JSON.stringify(...) should fold to {expect:?}");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(s.value, expect),
                other => panic!("expected {expect:?}; got {:?}", other),
            }
        }
    }

    #[test]
    fn json_stringify_string_literal_does_not_fold() {
        // JSON escaping (quotes/backslash/controls) is declined — left to runtime.
        let c = json_stringify_call(string("x", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "JSON.stringify(\"x\") must not fold (escaping)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn json_stringify_fractional_and_large_number_does_not_fold() {
        // `fold_string_of_number` declines fractional / ≥2^53 — so does this fold
        // (V8 renders 1e21 as "1e+21", a spelling we don't reproduce).
        for v in [3.5, 1e21] {
            let c = json_stringify_call(num(v, None));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "JSON.stringify({v}) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn json_stringify_array_object_or_identifier_does_not_fold() {
        // Arrays/objects may have side effects and recurse; an identifier's type
        // is unknown — all declined.
        let args = [
            Expression::ArrayExpression(ArrayExpression {
                cv: None,
                elements: vec![],
            }),
            Expression::ObjectExpression(ObjectExpression {
                cv: None,
                properties: vec![],
            }),
            ident("x"),
        ];
        for arg in args {
            let c = json_stringify_call(arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "JSON.stringify(array/object/ident) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn json_stringify_second_argument_does_not_fold() {
        // A `replacer`/`space` second argument can change the result (a replacer
        // function is invoked even on a primitive) — decline.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("JSON"), "stringify")),
            arguments: vec![num(42.0, None), ident("replacer")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "JSON.stringify(42, replacer) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn json_stringify_on_non_json_receiver_does_not_fold() {
        // Only the bare global `JSON` folds; `j.stringify(42)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("j"), "stringify")),
            arguments: vec![num(42.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "j.stringify(42) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_static_non_literal_argument_does_not_fold() {
        // An identifier arg has unknown runtime type — we cannot prove the class.
        let c = number_static_call("isInteger", ident("x"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Number.isInteger(x) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_static_second_argument_does_not_fold() {
        // We model only the single-argument form; `Number.isInteger(5, y)` is left.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Number"), "isInteger")),
            arguments: vec![num(5.0, None), ident("y")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Number.isInteger(5, y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn number_static_on_non_number_receiver_does_not_fold() {
        // Only the bare global `Number` folds; `n.isInteger(5)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("n"), "isInteger")),
            arguments: vec![num(5.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "n.isInteger(5) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Array.isArray (static) ----------------------

    /// Build `Array.isArray(<arg>)`.
    fn array_isarray_call(arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Array"), "isArray")),
            arguments: vec![arg],
        })
    }

    // ------------------- Object.keys/values/entries (static) ---------

    /// Build `Object.<method>(<arg>)`.
    fn object_static_call(method: &str, arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), method)),
            arguments: vec![arg],
        })
    }

    fn empty_array() -> Expression {
        Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![],
        })
    }

    fn empty_object() -> Expression {
        Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![],
        })
    }

    #[test]
    fn fold_array_isarray_empty_array_is_true() {
        // `Array.isArray([])` → `true` — the only literal that IS an Array.
        let c = array_isarray_call(empty_array());
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.isArray([]) should fold to true");
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(b.value, "Array.isArray([]) → true"),
            other => panic!("expected true; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_isarray_non_array_literals_are_false() {
        // An empty object and every primitive literal are provably not Arrays.
        let args = [
            empty_object(),
            string("x", None),
            num(42.0, None),
            Expression::BooleanLiteral(BooleanLiteral {
                cv: None,
                value: true,
            }),
            Expression::NullLiteral(NullLiteral { cv: None }),
        ];
        for arg in args {
            let c = array_isarray_call(arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Array.isArray(non-array literal) should fold to false");
            match extract_expr(&out) {
                Expression::BooleanLiteral(b) => assert!(!b.value, "→ false"),
                other => panic!("expected false; got {:?}", other),
            }
        }
    }

    #[test]
    fn array_isarray_non_empty_array_does_not_fold() {
        // A non-empty array literal is DECLINED — folding to a boolean would
        // discard the element expressions and drop any side effect they evaluate.
        let c = array_isarray_call(Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(num(1.0, None))],
        }));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Array.isArray([1]) must not fold (element side effects)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn array_isarray_identifier_does_not_fold() {
        // An identifier's runtime type is unknown — decline.
        let c = array_isarray_call(ident("x"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Array.isArray(x) must not fold (unknown type)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn array_isarray_second_argument_does_not_fold() {
        // We model only the single-argument form; `Array.isArray([], y)` is left.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Array"), "isArray")),
            arguments: vec![empty_array(), ident("y")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Array.isArray([], y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn array_isarray_on_non_array_receiver_does_not_fold() {
        // Only the bare global `Array` folds; `a.isArray([])` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("a"), "isArray")),
            arguments: vec![empty_array()],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "a.isArray([]) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Array.from (static, string) -----------------

    /// Build `Array.from(<arg>)` (single argument).
    fn array_from_call(arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Array"), "from")),
            arguments: vec![arg],
        })
    }

    /// Extract the element string values of a folded `ArrayExpression`.
    fn array_string_elements(expr: &Expression) -> Vec<String> {
        match expr {
            Expression::ArrayExpression(a) => a
                .elements
                .iter()
                .map(|e| match e {
                    Some(Expression::StringLiteral(s)) => s.value.clone(),
                    other => panic!("expected string element; got {:?}", other),
                })
                .collect(),
            other => panic!("expected array literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_from_string_to_code_point_strings() {
        // `Array.from("abc")` → `["a", "b", "c"]` — one element per code point.
        let c = array_from_call(string("abc", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.from(\"abc\") should fold");
        assert_eq!(array_string_elements(extract_expr(&out)), vec!["a", "b", "c"]);
    }

    #[test]
    fn fold_array_from_empty_string_to_empty_array() {
        // `Array.from("")` → `[]`.
        let c = array_from_call(string("", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.from(\"\") should fold to []");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => assert!(a.elements.is_empty(), "→ []"),
            other => panic!("expected []; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_from_astral_char_is_one_element() {
        // `Array.from("a💩b")` → `["a", "💩", "b"]` — the astral code point is a
        // SINGLE element (NOT split into its two UTF-16 surrogate halves), which
        // is exactly how the string iterator / spread behaves.
        let c = array_from_call(string("a💩b", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.from(\"a💩b\") should fold");
        let els = array_string_elements(extract_expr(&out));
        assert_eq!(els.len(), 3, "three elements (astral char not split)");
        assert_eq!(els, vec!["a", "💩", "b"]);
        // The middle element is the full astral scalar (one Unicode char).
        assert_eq!(els[1].chars().count(), 1, "the astral element is one code point");
    }

    #[test]
    fn array_from_with_map_function_does_not_fold() {
        // A second `mapFn` argument changes every element — decline.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Array"), "from")),
            arguments: vec![string("abc", None), ident("fn")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Array.from(\"abc\", fn) must not fold (mapFn)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn array_from_non_string_argument_does_not_fold() {
        // An identifier / non-string-literal argument's iteration is unknown.
        let c = array_from_call(ident("x"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Array.from(x) must not fold (unknown iterable)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn array_from_on_non_array_receiver_does_not_fold() {
        // Only the bare global `Array` folds; `a.from("x")` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("a"), "from")),
            arguments: vec![string("x", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "a.from(\"x\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn fold_object_keys_values_entries_empty_object_to_empty_array() {
        // `Object.keys/values/entries({})` → `[]` for all three methods.
        for method in ["keys", "values", "entries"] {
            let c = object_static_call(method, empty_object());
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Object.{method}({{}}) should fold to []");
            match extract_expr(&out) {
                Expression::ArrayExpression(a) => {
                    assert!(a.elements.is_empty(), "Object.{method}({{}}) → []")
                }
                other => panic!("expected []; got {:?}", other),
            }
        }
    }

    #[test]
    fn object_static_non_empty_object_does_not_fold() {
        // `Object.values` of a non-empty object literal is declined — it has no
        // non-empty fold yet. (`keys`/`entries` DO fold non-empty literals; their
        // dedicated tests cover that.)
        let obj = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![ObjectMember::Property(Property {
                cv: None,
                kind: coding_adventures_javascript_ast::PropertyKind::Init,
                key: PropertyKey::Identifier(coding_adventures_javascript_ast::Identifier {
                    cv: None,
                    name: "a".to_string(),
                }),
                value: Box::new(num(1.0, None)),
                shorthand: false,
                computed: false,
                method: false,
            })],
        });
        let c = object_static_call("values", obj);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Object.values({{a:1}}) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn object_static_array_primitive_or_identifier_does_not_fold() {
        // An array, a primitive (Object.keys("ab")→["0","1"]), and an identifier
        // are all declined.
        let args = [
            Expression::ArrayExpression(ArrayExpression {
                cv: None,
                elements: vec![],
            }),
            string("ab", None),
            ident("x"),
        ];
        for arg in args {
            let c = object_static_call("keys", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Object.keys(array/primitive/ident) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_static_second_argument_does_not_fold() {
        // We model only the single-argument form.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), "keys")),
            arguments: vec![empty_object(), ident("y")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Object.keys({{}}, y) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn object_static_on_non_object_receiver_does_not_fold() {
        // Only the bare global `Object` folds; `o.keys({})` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("o"), "keys")),
            arguments: vec![empty_object()],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "o.keys({{}}) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Object.entries (static, non-empty) ----------

    /// A `{<name>: <value>}` data property with an identifier key.
    fn entries_prop(name: &str, value: Expression) -> Property {
        Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: name.to_string(),
            }),
            value: Box::new(value),
            shorthand: false,
            computed: false,
            method: false,
        }
    }

    /// An object literal from a property list.
    fn object_lit(props: Vec<Property>) -> Expression {
        Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: props.into_iter().map(ObjectMember::Property).collect(),
        })
    }

    /// Extract the `&Property` from an `&ObjectMember` in test assertions.
    /// The fold outputs asserted on here never synthesize object spreads, so
    /// a `Spread` arm here indicates a broken fold, not a missing case.
    fn prop_of(member: &ObjectMember) -> &Property {
        match member {
            ObjectMember::Property(p) => p,
            ObjectMember::Spread(_) => unreachable!("folded object has no spreads"),
        }
    }

    /// Assert that `pair` is `["<key>", <expected-value-matcher>]`.
    fn assert_pair_key(pair: &Expression, key: &str) -> Expression {
        match pair {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "each entry is a 2-element array");
                match &a.elements[0] {
                    Some(Expression::StringLiteral(s)) => {
                        assert_eq!(s.value, key, "entry key string")
                    }
                    other => panic!("expected string key {key:?}; got {:?}", other),
                }
                a.elements[1].clone().expect("entry value present")
            }
            other => panic!("expected a [key, value] array; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_entries_to_pairs() {
        // `Object.entries({a: 1, b: 2})` → `[["a", 1], ["b", 2]]`.
        let c = object_static_call(
            "entries",
            object_lit(vec![
                entries_prop("a", num(1.0, None)),
                entries_prop("b", num(2.0, None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Object.entries({{a:1,b:2}}) should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "two entries");
                let v0 = assert_pair_key(a.elements[0].as_ref().unwrap(), "a");
                assert!(matches!(v0, Expression::NumericLiteral(n) if n.value == 1.0));
                let v1 = assert_pair_key(a.elements[1].as_ref().unwrap(), "b");
                assert!(matches!(v1, Expression::NumericLiteral(n) if n.value == 2.0));
            }
            other => panic!("expected array of pairs; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_entries_all_primitive_value_kinds() {
        // Values may be string / number / boolean / null literals.
        let c = object_static_call(
            "entries",
            object_lit(vec![
                entries_prop("s", string("hi", None)),
                entries_prop("n", num(42.0, None)),
                entries_prop("b", boolean(true, None)),
                entries_prop("z", null(None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "all-primitive-value entries should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 4);
                assert!(matches!(
                    assert_pair_key(a.elements[0].as_ref().unwrap(), "s"),
                    Expression::StringLiteral(_)
                ));
                assert!(matches!(
                    assert_pair_key(a.elements[1].as_ref().unwrap(), "n"),
                    Expression::NumericLiteral(_)
                ));
                assert!(matches!(
                    assert_pair_key(a.elements[2].as_ref().unwrap(), "b"),
                    Expression::BooleanLiteral(_)
                ));
                assert!(matches!(
                    assert_pair_key(a.elements[3].as_ref().unwrap(), "z"),
                    Expression::NullLiteral(_)
                ));
            }
            other => panic!("expected array of pairs; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_entries_string_and_noninteger_numeric_keys() {
        // A string key and a non-integer-index numeric key (1.5 → "1.5", which is
        // NOT an array index) both fold to string entry keys in source order.
        let c = object_static_call(
            "entries",
            object_lit(vec![
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::StringLiteral(StringLiteral {
                        cv: None,
                        value: "a-b".to_string(),
                        raw: "\"a-b\"".to_string(),
                    }),
                    value: Box::new(num(1.0, None)),
                    shorthand: false,
                    computed: false,
                    method: false,
                },
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::NumericLiteral(NumericLiteral {
                        cv: None,
                        value: 1.5,
                        raw: "1.5".to_string(),
                    }),
                    value: Box::new(num(2.0, None)),
                    shorthand: false,
                    computed: false,
                    method: false,
                },
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "string + non-index numeric keys should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2);
                assert_pair_key(a.elements[0].as_ref().unwrap(), "a-b");
                assert_pair_key(a.elements[1].as_ref().unwrap(), "1.5");
            }
            other => panic!("expected array of pairs; got {:?}", other),
        }
    }

    #[test]
    fn noninteger_object_key_folds_to_quoted_string_key() {
        // `{0.5: 1, 2: 3}` — the non-integer key `0.5` has property name
        // ToString(0.5) = "0.5", so it becomes a QUOTED string key
        // (`{"0.5": 1}`); the integer key `2` stays numeric (`{2: 3}`).
        let o = object_lit(vec![
            Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 0.5,
                    raw: "0.5".to_string(),
                }),
                value: Box::new(num(1.0, None)),
                shorthand: false,
                computed: false,
                method: false,
            },
            Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 2.0,
                    raw: "2".to_string(),
                }),
                value: Box::new(num(3.0, None)),
                shorthand: false,
                computed: false,
                method: false,
            },
        ]);
        let (out, _, changed, _) = run_pass(program_with_expr(o, true));
        assert!(changed, "a non-integer numeric key should fold to a string key");
        match extract_expr(&out) {
            Expression::ObjectExpression(obj) => {
                assert_eq!(obj.properties.len(), 2);
                match &obj.properties[0] {
                    ObjectMember::Property(p) => match &p.key {
                        PropertyKey::StringLiteral(s) => assert_eq!(s.value, "0.5"),
                        other => panic!("expected a StringLiteral key for 0.5, got {:?}", other),
                    },
                    other => panic!("expected a Property, got {:?}", other),
                }
                match &obj.properties[1] {
                    ObjectMember::Property(p) => assert!(
                        matches!(&p.key, PropertyKey::NumericLiteral(n) if n.value == 2.0),
                        "the integer key 2 must stay numeric; got {:?}",
                        p.key
                    ),
                    other => panic!("expected a Property, got {:?}", other),
                }
            }
            other => panic!("expected an ObjectExpression; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_entries_duplicate_key_last_value_first_position() {
        // `{a: 1, b: 2, a: 3}` builds `{a: 3, b: 2}`, so entries are
        // `[["a", 3], ["b", 2]]` — key `a` keeps first position, takes last value.
        let c = object_static_call(
            "entries",
            object_lit(vec![
                entries_prop("a", num(1.0, None)),
                entries_prop("b", num(2.0, None)),
                entries_prop("a", num(3.0, None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "duplicate-key entries should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "deduped to two entries");
                let v0 = assert_pair_key(a.elements[0].as_ref().unwrap(), "a");
                assert!(
                    matches!(v0, Expression::NumericLiteral(n) if n.value == 3.0),
                    "a takes the LAST value (3)"
                );
                assert_pair_key(a.elements[1].as_ref().unwrap(), "b");
            }
            other => panic!("expected array of pairs; got {:?}", other),
        }
    }

    #[test]
    fn object_entries_integer_index_key_does_not_fold() {
        // Integer-index keys (0, 1, 42) enumerate before string keys, which would
        // reorder the result — decline. Covers numeric and string index forms.
        let cases = [
            // {1: "x"} — numeric literal key "1" is an array index
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
                value: Box::new(string("x", None)),
                shorthand: false,
                computed: false,
                method: false,
            }]),
            // {"0": "x"} — string key "0" is also an array index
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::StringLiteral(StringLiteral {
                    cv: None,
                    value: "0".to_string(),
                    raw: "\"0\"".to_string(),
                }),
                value: Box::new(string("x", None)),
                shorthand: false,
                computed: false,
                method: false,
            }]),
        ];
        for arg in cases {
            let c = object_static_call("entries", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "integer-index key must not fold (ordering)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_entries_proto_key_does_not_fold() {
        // `{__proto__: v}` is the §B.3.1 prototype setter, not an own property, so
        // Object.entries would not enumerate it — decline rather than invent one.
        let c = object_static_call("entries", object_lit(vec![entries_prop("__proto__", num(1.0, None))]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "__proto__ key must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn object_entries_non_literal_value_does_not_fold() {
        // A non-literal value (identifier / call / nested object / array) is
        // declined, as is a shorthand `{x}` (whose value is the identifier `x`).
        let cases = [
            object_lit(vec![entries_prop("a", ident("v"))]),
            object_lit(vec![entries_prop("a", empty_object())]),
            object_lit(vec![entries_prop(
                "a",
                Expression::ArrayExpression(ArrayExpression {
                    cv: None,
                    elements: vec![Some(num(1.0, None))],
                }),
            )]),
            // shorthand { x }
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
                value: Box::new(ident("x")),
                shorthand: true,
                computed: false,
                method: false,
            }]),
        ];
        for arg in cases {
            let c = object_static_call("entries", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-literal value must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_entries_getter_method_computed_do_not_fold() {
        // Getters/setters run code, methods are functions, computed keys are
        // unknown — each declines the whole fold.
        let getter = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Get,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: false,
            method: false,
        }]);
        let method = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: false,
            method: true,
        }]);
        let computed = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: true,
            method: false,
        }]);
        for arg in [getter, method, computed] {
            let c = object_static_call("entries", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "getter/method/computed must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_values_nonempty_still_declines() {
        // `entries` and `keys` fold non-empty objects; `values` is left for a
        // future PR and must still decline.
        let c = object_static_call("values", object_lit(vec![entries_prop("a", num(1.0, None))]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Object.values({{a:1}}) must not fold yet");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Object.keys (static, non-empty) -------------

    /// Assert that `elem` is a string literal with value `key`.
    fn assert_key_string(elem: &Expression, key: &str) {
        match elem {
            Expression::StringLiteral(s) => assert_eq!(s.value, key, "key string"),
            other => panic!("expected string key {key:?}; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_keys_to_names() {
        // `Object.keys({a: 1, b: 2})` → `["a", "b"]`.
        let c = object_static_call(
            "keys",
            object_lit(vec![
                entries_prop("a", num(1.0, None)),
                entries_prop("b", num(2.0, None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Object.keys({{a:1,b:2}}) should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "two keys");
                assert_key_string(a.elements[0].as_ref().unwrap(), "a");
                assert_key_string(a.elements[1].as_ref().unwrap(), "b");
            }
            other => panic!("expected array of key strings; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_keys_all_primitive_value_kinds() {
        // The value is dropped; any primitive value kind (string/number/boolean/
        // null) is accepted and only the key survives, in source order.
        let c = object_static_call(
            "keys",
            object_lit(vec![
                entries_prop("s", string("hi", None)),
                entries_prop("n", num(42.0, None)),
                entries_prop("b", boolean(true, None)),
                entries_prop("z", null(None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "all-primitive-value keys should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 4);
                assert_key_string(a.elements[0].as_ref().unwrap(), "s");
                assert_key_string(a.elements[1].as_ref().unwrap(), "n");
                assert_key_string(a.elements[2].as_ref().unwrap(), "b");
                assert_key_string(a.elements[3].as_ref().unwrap(), "z");
            }
            other => panic!("expected array of key strings; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_keys_string_and_noninteger_numeric_keys() {
        // A string key and a non-integer-index numeric key (1.5 → "1.5", NOT an
        // array index) both fold to string keys in source order.
        let c = object_static_call(
            "keys",
            object_lit(vec![
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::StringLiteral(StringLiteral {
                        cv: None,
                        value: "a-b".to_string(),
                        raw: "\"a-b\"".to_string(),
                    }),
                    value: Box::new(num(1.0, None)),
                    shorthand: false,
                    computed: false,
                    method: false,
                },
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::NumericLiteral(NumericLiteral {
                        cv: None,
                        value: 1.5,
                        raw: "1.5".to_string(),
                    }),
                    value: Box::new(num(2.0, None)),
                    shorthand: false,
                    computed: false,
                    method: false,
                },
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "string + non-index numeric keys should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2);
                assert_key_string(a.elements[0].as_ref().unwrap(), "a-b");
                assert_key_string(a.elements[1].as_ref().unwrap(), "1.5");
            }
            other => panic!("expected array of key strings; got {:?}", other),
        }
    }

    #[test]
    fn fold_object_keys_duplicate_key_first_position() {
        // `{a: 1, b: 2, a: 3}` builds `{a: 3, b: 2}`, so keys are `["a", "b"]` —
        // key `a` keeps its first position and appears once.
        let c = object_static_call(
            "keys",
            object_lit(vec![
                entries_prop("a", num(1.0, None)),
                entries_prop("b", num(2.0, None)),
                entries_prop("a", num(3.0, None)),
            ]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "duplicate-key object should fold");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "deduped to two keys");
                assert_key_string(a.elements[0].as_ref().unwrap(), "a");
                assert_key_string(a.elements[1].as_ref().unwrap(), "b");
            }
            other => panic!("expected array of key strings; got {:?}", other),
        }
    }

    #[test]
    fn object_keys_integer_index_key_does_not_fold() {
        // Integer-index keys enumerate before string keys, reordering the result —
        // decline. Covers numeric and string index forms.
        let cases = [
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
                value: Box::new(string("x", None)),
                shorthand: false,
                computed: false,
                method: false,
            }]),
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::StringLiteral(StringLiteral {
                    cv: None,
                    value: "0".to_string(),
                    raw: "\"0\"".to_string(),
                }),
                value: Box::new(string("x", None)),
                shorthand: false,
                computed: false,
                method: false,
            }]),
        ];
        for arg in cases {
            let c = object_static_call("keys", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "integer-index key must not fold (ordering)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_keys_escaped_string_key_folds_with_decoded_value() {
        // Property-key string values are now DECODED at the bridge, so the source
        // key `"a\"b"` arrives here as the three characters `a"b` (the real
        // property name). The fold copies that decoded value into a fresh string
        // literal whose emission re-escapes it correctly, so escaped keys fold
        // soundly — they no longer need the old `contains('\\')` decline. (Before
        // the bridge decode, the value held the raw `a\"b` and re-escaping it
        // produced the WRONG name, which is why this case used to be declined.)
        let c = object_static_call(
            "keys",
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::StringLiteral(StringLiteral {
                    cv: None,
                    value: "a\"b".to_string(), // decoded property name: a " b
                    raw: "\"a\\\"b\"".to_string(),
                }),
                value: Box::new(num(1.0, None)),
                shorthand: false,
                computed: false,
                method: false,
            }]),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "escaped string key folds with its decoded value");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 1);
                assert_key_string(a.elements[0].as_ref().unwrap(), "a\"b");
            }
            other => panic!("expected array of key strings; got {:?}", other),
        }
    }

    #[test]
    fn object_keys_proto_key_does_not_fold() {
        // `{__proto__: v}` is the §B.3.1 prototype setter, not an own property —
        // decline rather than invent a key that does not exist.
        let c = object_static_call("keys", object_lit(vec![entries_prop("__proto__", num(1.0, None))]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "__proto__ key must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn object_keys_non_literal_value_does_not_fold() {
        // The value is dropped, but evaluating the source literal still runs it —
        // a non-literal value (identifier / call / nested object / array) may have
        // side effects or throw, so decline. A shorthand `{x}` is also declined.
        let cases = [
            object_lit(vec![entries_prop("a", ident("v"))]),
            object_lit(vec![entries_prop("a", empty_object())]),
            object_lit(vec![entries_prop(
                "a",
                Expression::ArrayExpression(ArrayExpression {
                    cv: None,
                    elements: vec![Some(num(1.0, None))],
                }),
            )]),
            // shorthand { x }
            object_lit(vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
                value: Box::new(ident("x")),
                shorthand: true,
                computed: false,
                method: false,
            }]),
        ];
        for arg in cases {
            let c = object_static_call("keys", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-literal value must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_keys_getter_method_computed_do_not_fold() {
        // Getters/setters run code, methods are functions, computed keys are
        // unknown — each declines the whole fold.
        let getter = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Get,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: false,
            method: false,
        }]);
        let method = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: false,
            method: true,
        }]);
        let computed = object_lit(vec![Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            }),
            value: Box::new(num(1.0, None)),
            shorthand: false,
            computed: true,
            method: false,
        }]);
        for arg in [getter, method, computed] {
            let c = object_static_call("keys", arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "getter/method/computed must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_entries_on_non_object_receiver_does_not_fold() {
        // Only the bare global `Object` folds; `o.entries({a:1})` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("o"), "entries")),
            arguments: vec![object_lit(vec![entries_prop("a", num(1.0, None))])],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "o.entries({{a:1}}) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Object.is (static, SameValue) ---------------

    /// Build `Object.is(<a>, <b>)`.
    fn object_is_call(a: Expression, b: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), "is")),
            arguments: vec![a, b],
        })
    }

    fn boolean_lit(value: bool) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral { cv: None, value })
    }

    #[test]
    fn fold_object_is_same_value_literals() {
        // (a, b, expected) — every result confirmed against V8's Object.is.
        // The two cases where SameValue differs from === are the headline ones:
        // Object.is(NaN, NaN) === true, Object.is(0, -0) === false.
        let neg_zero = -0.0_f64;
        let cases: Vec<(Expression, Expression, bool)> = vec![
            // numbers
            (num(1.0, None), num(1.0, None), true),
            (num(1.0, None), num(2.0, None), false),
            (num(f64::NAN, None), num(f64::NAN, None), true), // NaN IS NaN
            (num(0.0, None), num(neg_zero, None), false),     // +0 is NOT -0
            (num(neg_zero, None), num(neg_zero, None), true), // -0 IS -0
            (num(0.0, None), num(0.0, None), true),
            // strings
            (string("a", None), string("a", None), true),
            (string("a", None), string("b", None), false),
            // booleans
            (boolean_lit(true), boolean_lit(true), true),
            (boolean_lit(true), boolean_lit(false), false),
            // null
            (
                Expression::NullLiteral(NullLiteral { cv: None }),
                Expression::NullLiteral(NullLiteral { cv: None }),
                true,
            ),
            // type mismatches → false (different Type can never be SameValue)
            (num(1.0, None), string("1", None), false),
            (boolean_lit(true), num(1.0, None), false),
            (
                Expression::NullLiteral(NullLiteral { cv: None }),
                num(0.0, None),
                false,
            ),
        ];
        for (a, b, expect) in cases {
            let c = object_is_call(a, b);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "Object.is(.., ..) should fold");
            match extract_expr(&out) {
                Expression::BooleanLiteral(bl) => assert_eq!(bl.value, expect, "expected {expect}"),
                other => panic!("expected bool; got {:?}", other),
            }
        }
    }

    #[test]
    fn object_is_non_literal_argument_does_not_fold() {
        // If EITHER operand is a non-literal, the value is unknown — decline.
        let cases = [
            object_is_call(ident("x"), num(1.0, None)),
            object_is_call(num(1.0, None), ident("y")),
            object_is_call(ident("x"), ident("y")),
        ];
        for c in cases {
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Object.is with a non-literal must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn object_is_wrong_arity_does_not_fold() {
        // We model only the two-argument form.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), "is")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Object.is(1) must not fold (needs two args)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn object_is_on_non_object_receiver_does_not_fold() {
        // Only the bare global `Object` folds; `o.is(1, 1)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("o"), "is")),
            arguments: vec![num(1.0, None), num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "o.is(1, 1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Array.of (static) ---------------------------

    /// Build `Array.of(<args…>)`.
    fn array_of_call(args: Vec<Expression>) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Array"), "of")),
            arguments: args,
        })
    }

    #[test]
    fn fold_array_of_multiple_args_to_array_literal() {
        // `Array.of(1, 2, 3)` → `[1, 2, 3]` — elements preserved in order.
        let c = array_of_call(vec![num(1.0, None), num(2.0, None), num(3.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.of(1,2,3) should fold to [1,2,3]");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 3, "three elements");
                let vals: Vec<f64> = a
                    .elements
                    .iter()
                    .map(|e| match e {
                        Some(Expression::NumericLiteral(n)) => n.value,
                        other => panic!("expected numeric element; got {:?}", other),
                    })
                    .collect();
                assert_eq!(vals, vec![1.0, 2.0, 3.0], "elements in order");
            }
            other => panic!("expected array literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_of_empty_to_empty_array() {
        // `Array.of()` → `[]`.
        let c = array_of_call(vec![]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.of() should fold to []");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => assert!(a.elements.is_empty(), "Array.of() → []"),
            other => panic!("expected []; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_of_single_numeric_is_one_element_not_length() {
        // The defining difference from the `Array(n)` constructor:
        // `Array.of(7)` is the ONE-element array `[7]`, NOT `Array(7)`'s
        // length-7 hole array. We must emit exactly one element whose value is 7.
        let c = array_of_call(vec![num(7.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.of(7) should fold to [7]");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 1, "exactly one element (not length 7)");
                match &a.elements[0] {
                    Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 7.0, "the value 7"),
                    other => panic!("expected the literal 7; got {:?}", other),
                }
            }
            other => panic!("expected [7]; got {:?}", other),
        }
    }

    #[test]
    fn fold_array_of_preserves_identifier_arguments() {
        // Identifier (and any other) arguments are preserved as elements in
        // order — folding never drops or evaluates them, so side effects survive.
        let c = array_of_call(vec![ident("x"), ident("y")]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Array.of(x, y) should fold to [x, y]");
        match extract_expr(&out) {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 2, "two elements");
                let names: Vec<&str> = a
                    .elements
                    .iter()
                    .map(|e| match e {
                        Some(Expression::Identifier(id)) => id.name.as_str(),
                        other => panic!("expected identifier element; got {:?}", other),
                    })
                    .collect();
                assert_eq!(names, vec!["x", "y"], "identifiers preserved in order");
            }
            other => panic!("expected [x, y]; got {:?}", other),
        }
    }

    #[test]
    fn array_of_on_non_array_receiver_does_not_fold() {
        // Only the bare global `Array` folds; `a.of(1)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("a"), "of")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "a.of(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Math.max / Math.min (static) ----------------

    /// Build `Math.<method>(<args…>)`.
    fn math_call(method: &str, args: Vec<Expression>) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Math"), method)),
            arguments: args,
        })
    }

    /// Run the pass and return the single folded numeric value, asserting it
    /// folded to a numeric literal.
    fn folded_number(c: Expression) -> f64 {
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "expected the Math call to fold");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => n.value,
            other => panic!("expected numeric literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_math_max_min_basic() {
        assert_eq!(
            folded_number(math_call(
                "max",
                vec![num(1.0, None), num(2.0, None), num(3.0, None)]
            )),
            3.0
        );
        assert_eq!(
            folded_number(math_call(
                "min",
                vec![num(1.0, None), num(2.0, None), num(3.0, None)]
            )),
            1.0
        );
    }

    #[test]
    fn fold_math_max_min_negatives_and_single() {
        assert_eq!(
            folded_number(math_call("max", vec![num(-5.0, None), num(-1.0, None)])),
            -1.0
        );
        assert_eq!(
            folded_number(math_call("min", vec![num(-5.0, None), num(-1.0, None)])),
            -5.0
        );
        // Single argument returns that argument.
        assert_eq!(folded_number(math_call("max", vec![num(7.0, None)])), 7.0);
        assert_eq!(folded_number(math_call("min", vec![num(7.0, None)])), 7.0);
    }

    #[test]
    fn fold_math_max_prefers_positive_zero() {
        // Math.max(-0, +0) === +0 and Math.max(+0, -0) === +0.
        for args in [
            vec![num(-0.0, None), num(0.0, None)],
            vec![num(0.0, None), num(-0.0, None)],
        ] {
            let r = folded_number(math_call("max", args));
            assert_eq!(r, 0.0);
            assert!(r.is_sign_positive(), "Math.max(±0) must be +0");
        }
    }

    #[test]
    fn math_negative_zero_result_does_not_fold() {
        // A result of -0 has NO faithful numeric-literal spelling (`-0` is
        // UnaryMinus on `0`, ToString(-0) === "0"), so emitting it would print
        // `0` (=== +0) — a sign-bit miscompile. We DECLINE these:
        //   Math.min(-0, +0) === -0, Math.min(+0, -0) === -0, Math.max(-0, -0) === -0,
        //   Math.min(-0)     === -0.
        let cases = [
            math_call("min", vec![num(-0.0, None), num(0.0, None)]),
            math_call("min", vec![num(0.0, None), num(-0.0, None)]),
            math_call("max", vec![num(-0.0, None), num(-0.0, None)]),
            math_call("min", vec![num(-0.0, None)]),
        ];
        for c in cases {
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "a -0 result must not fold (no -0 literal spelling)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn fold_math_positive_zero_result_still_folds() {
        // A +0 result IS representable, so it still folds.
        // Math.max(-0, +0) === +0; Math.min(+0, +0) === +0; Math.max(0) === +0.
        for c in [
            math_call("max", vec![num(-0.0, None), num(0.0, None)]),
            math_call("min", vec![num(0.0, None), num(0.0, None)]),
            math_call("max", vec![num(0.0, None)]),
        ] {
            let r = folded_number(c);
            assert_eq!(r, 0.0);
            assert!(r.is_sign_positive(), "a +0 result folds to +0");
        }
    }

    #[test]
    fn math_max_min_non_literal_argument_does_not_fold() {
        // A non-literal argument (identifier — could be NaN/Infinity at runtime)
        // declines the whole fold.
        for method in ["max", "min"] {
            let c = math_call(method, vec![num(1.0, None), ident("x")]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Math.{method}(1, x) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn math_max_min_zero_args_does_not_fold() {
        // Math.max() → -Infinity, Math.min() → +Infinity; we decline emitting an
        // infinite numeric literal.
        for method in ["max", "min"] {
            let c = math_call(method, vec![]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Math.{method}() must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn math_max_min_on_non_global_receiver_does_not_fold() {
        // Only the bare global `Math` folds; `m.max(1, 2)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("m"), "max")),
            arguments: vec![num(1.0, None), num(2.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "m.max(1, 2) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn math_other_methods_do_not_fold() {
        // pow is not among the modelled methods (max/min/abs/floor/ceil/round);
        // e.g. Math.pow(2, 3) is left alone.
        let c = math_call("pow", vec![num(2.0, None), num(3.0, None)]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "Math.pow(2, 3) is not modelled and must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- Math.abs/floor/ceil/round (unary) -----------

    #[test]
    fn fold_math_unary_basic() {
        assert_eq!(folded_number(math_call("abs", vec![num(-5.0, None)])), 5.0);
        assert_eq!(folded_number(math_call("abs", vec![num(5.0, None)])), 5.0);
        assert_eq!(folded_number(math_call("floor", vec![num(4.7, None)])), 4.0);
        assert_eq!(folded_number(math_call("floor", vec![num(-4.2, None)])), -5.0);
        assert_eq!(folded_number(math_call("ceil", vec![num(4.2, None)])), 5.0);
        assert_eq!(folded_number(math_call("ceil", vec![num(-4.7, None)])), -4.0);
    }

    #[test]
    fn fold_math_round_half_toward_positive_infinity() {
        // JS Math.round rounds a half toward +Infinity (NOT away from zero).
        assert_eq!(folded_number(math_call("round", vec![num(2.5, None)])), 3.0);
        assert_eq!(folded_number(math_call("round", vec![num(-2.5, None)])), -2.0);
        assert_eq!(folded_number(math_call("round", vec![num(2.4, None)])), 2.0);
        assert_eq!(folded_number(math_call("round", vec![num(2.6, None)])), 3.0);
        // The fp-pathological input rounds to 0 in both Rust and JS.
        assert_eq!(
            folded_number(math_call("round", vec![num(0.499_999_999_999_999_94, None)])),
            0.0
        );
    }

    #[test]
    fn math_unary_negative_zero_result_does_not_fold() {
        // Results that are (or, from a negative input, would be) -0 have no
        // faithful numeric-literal spelling, so they DECLINE:
        //   Math.ceil(-0.4)  === -0   Math.round(-0.4) === -0
        //   Math.round(-0.5) === -0   Math.floor(-0.0) === -0   Math.abs(-0) === +0*
        // (*abs(-0) is +0 and would be representable, but the conservative
        // negative-input-zero guard declines it too — always safe.)
        let cases = [
            math_call("ceil", vec![num(-0.4, None)]),
            math_call("round", vec![num(-0.4, None)]),
            math_call("round", vec![num(-0.5, None)]),
            math_call("floor", vec![num(-0.0, None)]),
        ];
        for c in cases {
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "a -0 result must not fold (no -0 literal spelling)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn math_unary_non_literal_or_wrong_arity_does_not_fold() {
        for method in ["abs", "floor", "ceil", "round"] {
            // Non-literal argument.
            let c = math_call(method, vec![ident("x")]);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "Math.{method}(x) must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));

            // Wrong arity (zero or two args) declines — we fold only arity 1.
            for args in [vec![], vec![num(1.0, None), num(2.0, None)]] {
                let c = math_call(method, args);
                let (out, _, changed, _) = run_pass(program_with_expr(c, true));
                assert!(!changed, "Math.{method} with != 1 arg must not fold");
                assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
            }
        }
    }

    #[test]
    fn math_unary_on_non_global_receiver_does_not_fold() {
        // Only the bare global `Math` folds; `m.abs(-1)` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("m"), "abs")),
            arguments: vec![num(-1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "m.abs(-1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn js_math_round_matches_ecmascript_semantics() {
        // Direct helper checks: half toward +Inf, and Rust-vs-JS agreement.
        assert_eq!(js_math_round(2.5), 3.0);
        assert_eq!(js_math_round(-2.5), -2.0);
        assert_eq!(js_math_round(0.5), 1.0);
        assert_eq!(js_math_round(1.4), 1.0);
        assert_eq!(js_math_round(1.6), 2.0);
        assert_eq!(js_math_round(-1.6), -2.0);
    }

    #[test]
    fn js_math_max_min_helpers_signed_zero_and_nan() {
        // Direct unit coverage of the spec-faithful reducers.
        assert_eq!(js_math_max(&[1.0, 5.0, 3.0]), 5.0);
        assert_eq!(js_math_min(&[1.0, 5.0, 3.0]), 1.0);
        assert!(js_math_max(&[-0.0, 0.0]).is_sign_positive());
        assert!(js_math_min(&[0.0, -0.0]).is_sign_negative());
        // NaN propagation (callers never hit this, but the model is faithful).
        assert!(js_math_max(&[1.0, f64::NAN]).is_nan());
        assert!(js_math_min(&[f64::NAN, 1.0]).is_nan());
        // Empty → identities.
        assert_eq!(js_math_max(&[]), f64::NEG_INFINITY);
        assert_eq!(js_math_min(&[]), f64::INFINITY);
    }

    // ------------------- Object.fromEntries (static) -----------------

    /// Build `Object.fromEntries(<arg>)`.
    fn object_from_entries_call(arg: Expression) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), "fromEntries")),
            arguments: vec![arg],
        })
    }

    /// Build an array literal from a list of present elements (no holes).
    fn array_lit(elements: Vec<Expression>) -> Expression {
        Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: elements.into_iter().map(Some).collect(),
        })
    }

    /// Build a `[key, value]` pair array literal.
    fn pair(key: Expression, value: Expression) -> Expression {
        array_lit(vec![key, value])
    }

    #[test]
    fn fold_from_entries_identifier_keys_to_object() {
        // `Object.fromEntries([["a", 1], ["b", 2]])` → `{a: 1, b: 2}`.
        // Both keys are valid identifier names → bare-identifier keys.
        let c = object_from_entries_call(array_lit(vec![
            pair(string("a", None), num(1.0, None)),
            pair(string("b", None), num(2.0, None)),
        ]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Object.fromEntries([[a,1],[b,2]]) should fold");
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => {
                assert_eq!(o.properties.len(), 2, "two properties");
                let p0 = prop_of(&o.properties[0]);
                match &p0.key {
                    PropertyKey::Identifier(id) => assert_eq!(id.name, "a"),
                    other => panic!("expected identifier key `a`; got {:?}", other),
                }
                assert!(matches!(&*p0.value, Expression::NumericLiteral(n) if n.value == 1.0));
                let p1 = prop_of(&o.properties[1]);
                match &p1.key {
                    PropertyKey::Identifier(id) => assert_eq!(id.name, "b"),
                    other => panic!("expected identifier key `b`; got {:?}", other),
                }
                assert!(matches!(&*p1.value, Expression::NumericLiteral(n) if n.value == 2.0));
            }
            other => panic!("expected object literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_from_entries_numeric_key_uses_tostring_string_key() {
        // `Object.fromEntries([[1, "x"]])` → `{"1": "x"}` — the numeric key
        // folds to its ToString "1", which is NOT a valid identifier, so the
        // key is emitted as a quoted string literal.
        let c = object_from_entries_call(array_lit(vec![pair(num(1.0, None), string("x", None))]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Object.fromEntries([[1,\"x\"]]) should fold");
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => {
                assert_eq!(o.properties.len(), 1, "one property");
                match &prop_of(&o.properties[0]).key {
                    PropertyKey::StringLiteral(s) => assert_eq!(s.value, "1", "key ToString → \"1\""),
                    other => panic!("expected string key \"1\"; got {:?}", other),
                }
                assert!(matches!(&*prop_of(&o.properties[0]).value, Expression::StringLiteral(s) if s.value == "x"));
            }
            other => panic!("expected object literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_from_entries_duplicate_key_last_value_wins_first_position() {
        // `Object.fromEntries([["a", 1], ["b", 2], ["a", 3]])` → `{a: 3, b: 2}`.
        // The repeated key `a` keeps its FIRST position but takes the LAST value.
        let c = object_from_entries_call(array_lit(vec![
            pair(string("a", None), num(1.0, None)),
            pair(string("b", None), num(2.0, None)),
            pair(string("a", None), num(3.0, None)),
        ]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "duplicate-key fromEntries should fold");
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => {
                assert_eq!(o.properties.len(), 2, "deduped to two properties");
                // position 0 is still `a`, but its value is the LAST one (3).
                match &prop_of(&o.properties[0]).key {
                    PropertyKey::Identifier(id) => assert_eq!(id.name, "a", "a keeps first position"),
                    other => panic!("expected identifier key `a`; got {:?}", other),
                }
                assert!(
                    matches!(&*prop_of(&o.properties[0]).value, Expression::NumericLiteral(n) if n.value == 3.0),
                    "a takes the LAST value (3)"
                );
                match &prop_of(&o.properties[1]).key {
                    PropertyKey::Identifier(id) => assert_eq!(id.name, "b"),
                    other => panic!("expected identifier key `b`; got {:?}", other),
                }
            }
            other => panic!("expected object literal; got {:?}", other),
        }
    }

    #[test]
    fn fold_from_entries_empty_array_to_empty_object() {
        // `Object.fromEntries([])` → `{}`.
        let c = object_from_entries_call(empty_array());
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "Object.fromEntries([]) should fold to {{}}");
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => assert!(o.properties.is_empty(), "→ {{}}"),
            other => panic!("expected empty object; got {:?}", other),
        }
    }

    #[test]
    fn fold_from_entries_all_primitive_value_kinds() {
        // Values may be string / number / boolean / null literals.
        let c = object_from_entries_call(array_lit(vec![
            pair(string("s", None), string("hi", None)),
            pair(string("n", None), num(42.0, None)),
            pair(string("b", None), boolean(true, None)),
            pair(string("z", None), null(None)),
        ]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "all-primitive-value fromEntries should fold");
        match extract_expr(&out) {
            Expression::ObjectExpression(o) => {
                assert_eq!(o.properties.len(), 4);
                assert!(matches!(&*prop_of(&o.properties[0]).value, Expression::StringLiteral(_)));
                assert!(matches!(&*prop_of(&o.properties[1]).value, Expression::NumericLiteral(_)));
                assert!(matches!(&*prop_of(&o.properties[2]).value, Expression::BooleanLiteral(_)));
                assert!(matches!(&*prop_of(&o.properties[3]).value, Expression::NullLiteral(_)));
            }
            other => panic!("expected object literal; got {:?}", other),
        }
    }

    #[test]
    fn from_entries_non_pair_element_does_not_fold() {
        // An element that is not a 2-element array (here a 1- and a 3-element
        // array, and a non-array) declines the whole fold.
        let cases = [
            array_lit(vec![array_lit(vec![string("a", None)])]), // [["a"]]
            array_lit(vec![array_lit(vec![
                string("a", None),
                num(1.0, None),
                num(2.0, None),
            ])]), // [["a",1,2]]
            array_lit(vec![string("a", None)]),                  // [ "a" ] — element not an array
        ];
        for arg in cases {
            let c = object_from_entries_call(arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-pair element must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_entries_non_literal_key_does_not_fold() {
        // A boolean / null / identifier key is declined (ToPropertyKey unknown
        // or a different string than we model).
        let cases = [
            pair(boolean(true, None), num(1.0, None)),
            pair(null(None), num(1.0, None)),
            pair(ident("k"), num(1.0, None)),
        ];
        for p in cases {
            let c = object_from_entries_call(array_lit(vec![p]));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-string/numeric key must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_entries_proto_key_does_not_fold() {
        // `Object.fromEntries([["__proto__", v]])` makes an OWN "__proto__"
        // property, whereas the object literal `{__proto__: v}` is the §B.3.1
        // prototype setter — so we must DECLINE rather than miscompile. Covers a
        // primitive value (1) and the null-prototype trap (null).
        for value in [num(1.0, None), null(None), string("p", None)] {
            let c = object_from_entries_call(array_lit(vec![pair(string("__proto__", None), value)]));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "__proto__ key must not fold (prototype-setter trap)");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_entries_non_literal_value_does_not_fold() {
        // A non-literal value (identifier / call / array / object) is declined.
        let cases = [
            pair(string("a", None), ident("v")),
            pair(string("a", None), array_lit(vec![num(1.0, None)])),
            pair(string("a", None), empty_object()),
        ];
        for p in cases {
            let c = object_from_entries_call(array_lit(vec![p]));
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-literal value must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_entries_outer_hole_does_not_fold() {
        // A hole at the outer level (`[ , ["a", 1]]`) is declined.
        let arr = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![None, Some(pair(string("a", None), num(1.0, None)))],
        });
        let c = object_from_entries_call(arr);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "outer hole must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn from_entries_pair_hole_does_not_fold() {
        // A hole inside a pair (`[["a", ]]`) is declined — len() counts it but
        // the slot is None.
        let pair_with_hole = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(string("a", None)), None],
        });
        let c = object_from_entries_call(array_lit(vec![pair_with_hole]));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "hole inside a pair must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn from_entries_non_array_argument_does_not_fold() {
        // The single argument must be an array literal; an identifier / object
        // literal argument is declined.
        for arg in [ident("pairs"), empty_object()] {
            let c = object_from_entries_call(arg);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "non-array argument must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn from_entries_wrong_arity_does_not_fold() {
        // We model only the single-argument form.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("Object"), "fromEntries")),
            arguments: vec![empty_array(), empty_array()],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "two-arg fromEntries must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn from_entries_on_non_object_receiver_does_not_fold() {
        // Only the bare global `Object` folds; `o.fromEntries([])` is left alone.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("o"), "fromEntries")),
            arguments: vec![empty_array()],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "o.fromEntries([]) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- slice (substring) ---------------------------

    /// Build `"<recv>".slice(<args…>)` from numeric literal arguments.
    fn slice_call(recv: &str, args: &[f64]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "slice")),
            arguments: args.iter().map(|&a| num(a, None)).collect(),
        })
    }

    #[test]
    fn fold_string_slice_two_args() {
        // Positive, negative, and end-before-start (→ empty).
        for (recv, args, expect) in [
            ("abcd", vec![1.0, 3.0], "bc"),
            ("abcd", vec![0.0, -1.0], "abc"),
            ("abcd", vec![-2.0], "cd"),
            ("abcd", vec![1.0], "bcd"),
            ("abcd", vec![2.0, 1.0], ""),
            ("abcd", vec![10.0], ""),
        ] {
            let c = slice_call(recv, &args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".slice({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "\"{recv}\".slice({args:?})")
                }
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_string_slice_no_args_is_identity() {
        // `"abc".slice()` → "abc" (whole string).
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abc", None), "slice")),
            arguments: vec![],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"abc\".slice() should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected \"abc\"; got {:?}", other),
        }
    }

    #[test]
    fn slice_counts_utf16_units() {
        // "💩" is two UTF-16 units; "💩ab".slice(2) drops the astral char and
        // keeps "ab" — proving UTF-16 (not scalar) indexing.
        let c = slice_call("💩ab", &[2.0]);
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "ab"),
            other => panic!("expected \"ab\"; got {:?}", other),
        }
    }

    #[test]
    fn slice_splitting_a_surrogate_pair_does_not_fold() {
        // "💩".slice(0, 1) would be a lone high surrogate — a valid JS string
        // but not a Rust `String`, so we decline (conservative, like charAt).
        let c = slice_call("💩", &[0.0, 1.0]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "slice splitting a surrogate pair must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn slice_non_integer_or_too_many_args_does_not_fold() {
        // Fractional argument: don't model ToInteger coercion.
        let frac = slice_call("abcd", &[1.5]);
        let (out, _, changed, _) = run_pass(program_with_expr(frac, true));
        assert!(!changed, "fractional slice index must not fold");

        // Three arguments: not the slice signature we model.
        let three = slice_call("abcd", &[0.0, 1.0, 2.0]);
        let (out2, _, changed2, _) = run_pass(program_with_expr(three, true));
        assert!(!changed2, "three-arg slice must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        assert!(matches!(extract_expr(&out2), Expression::CallExpression(_)));
    }

    #[test]
    fn slice_on_identifier_receiver_does_not_fold() {
        // `s.slice(1)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "slice")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.slice(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- substr (length-counted slice) ----------------

    /// Build `"<recv>".substr(<args…>)`.
    fn substr_call(recv: &str, args: &[f64]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "substr")),
            arguments: args.iter().map(|&a| num(a, None)).collect(),
        })
    }

    #[test]
    fn fold_string_substr_start_and_length() {
        // V8 oracle (node): each row is the exact runtime result.
        //   "abcde".substr(1,2)  === "bc"
        //   "abcde".substr(1)    === "bcde"  (length defaults to the rest)
        //   "abcde".substr(-2)   === "de"    (negative start counts from end)
        //   "abcde".substr(-2,1) === "d"
        //   "abcde".substr(0,100)=== "abcde" (length clamps to remaining)
        //   "abcde".substr(2,0)  === ""      (zero length)
        //   "abcde".substr(10)   === ""      (start past the end)
        //   "abcde".substr(-100) === "abcde" (negative start clamps to 0)
        //   "abcde".substr(1,-1) === ""      (negative length clamps to 0)
        for (recv, args, expect) in [
            ("abcde", vec![1.0, 2.0], "bc"),
            ("abcde", vec![1.0], "bcde"),
            ("abcde", vec![-2.0], "de"),
            ("abcde", vec![-2.0, 1.0], "d"),
            ("abcde", vec![0.0, 100.0], "abcde"),
            ("abcde", vec![2.0, 0.0], ""),
            ("abcde", vec![10.0], ""),
            ("abcde", vec![-100.0], "abcde"),
            ("abcde", vec![1.0, -1.0], ""),
        ] {
            let c = substr_call(recv, &args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".substr({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "\"{recv}\".substr({args:?})")
                }
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_string_substr_no_args_is_identity() {
        // `"abc".substr()` → "abc" (start 0, length defaults to the rest).
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abc", None), "substr")),
            arguments: vec![],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"abc\".substr() should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected \"abc\"; got {:?}", other),
        }
    }

    #[test]
    fn substr_counts_utf16_units() {
        // "💩" is two UTF-16 units; "💩ab".substr(2) drops the astral char and
        // keeps "ab" — proving UTF-16 (not scalar) indexing.
        let c = substr_call("💩ab", &[2.0]);
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "ab"),
            other => panic!("expected \"ab\"; got {:?}", other),
        }
    }

    #[test]
    fn substr_splitting_a_surrogate_pair_does_not_fold() {
        // "💩".substr(0, 1) would be a lone high surrogate — a valid JS string
        // but not a Rust `String`, so we decline (conservative).
        let c = substr_call("💩", &[0.0, 1.0]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "substr splitting a surrogate pair must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn substr_non_integer_or_too_many_args_does_not_fold() {
        // Fractional argument: don't model ToInteger coercion.
        let frac = substr_call("abcde", &[1.5]);
        let (out, _, changed, _) = run_pass(program_with_expr(frac, true));
        assert!(!changed, "fractional substr index must not fold");

        // Three arguments: not the substr signature we model.
        let three = substr_call("abcde", &[0.0, 1.0, 2.0]);
        let (out2, _, changed2, _) = run_pass(program_with_expr(three, true));
        assert!(!changed2, "three-arg substr must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        assert!(matches!(extract_expr(&out2), Expression::CallExpression(_)));
    }

    #[test]
    fn substr_on_identifier_receiver_does_not_fold() {
        // `s.substr(1)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "substr")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.substr(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- substring (clamped, ordered cut) -------------

    /// Build `"<recv>".substring(<args…>)`.
    fn substring_call(recv: &str, args: &[f64]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "substring")),
            arguments: args.iter().map(|&a| num(a, None)).collect(),
        })
    }

    #[test]
    fn fold_string_substring_clamps_and_swaps() {
        // V8 oracle (node): each row is the exact runtime result.
        //   "abcd".substring(1,3) === "bc"
        //   "abcd".substring(2)   === "cd"
        //   "abcd".substring(3,1) === "bc"   (endpoints swap)
        //   "abcd".substring(-2)  === "abcd" (negative clamps to 0)
        //   "abcd".substring(1,-1)=== "a"    (-1 clamps to 0, then swap → [0,1))
        //   "abcd".substring(2,2) === ""     (empty range)
        //   "abcd".substring(10)  === ""     (start clamps to len)
        for (recv, args, expect) in [
            ("abcd", vec![1.0, 3.0], "bc"),
            ("abcd", vec![2.0], "cd"),
            ("abcd", vec![3.0, 1.0], "bc"),
            ("abcd", vec![-2.0], "abcd"),
            ("abcd", vec![1.0, -1.0], "a"),
            ("abcd", vec![2.0, 2.0], ""),
            ("abcd", vec![10.0], ""),
        ] {
            let c = substring_call(recv, &args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".substring({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "\"{recv}\".substring({args:?})")
                }
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn fold_string_substring_no_args_is_identity() {
        // `"abc".substring()` → "abc" (whole string).
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abc", None), "substring")),
            arguments: vec![],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"abc\".substring() should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected \"abc\"; got {:?}", other),
        }
    }

    #[test]
    fn substring_counts_utf16_units() {
        // "💩" is two UTF-16 units; "💩ab".substring(2) drops the astral char
        // and keeps "ab" — proving UTF-16 (not scalar) indexing.
        let c = substring_call("💩ab", &[2.0]);
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "ab"),
            other => panic!("expected \"ab\"; got {:?}", other),
        }
    }

    #[test]
    fn substring_splitting_a_surrogate_pair_does_not_fold() {
        // "💩".substring(0, 1) would be a lone high surrogate — a valid JS
        // string but not a Rust `String`, so we decline (conservative).
        let c = substring_call("💩", &[0.0, 1.0]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "substring splitting a surrogate pair must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn substring_non_integer_or_too_many_args_does_not_fold() {
        // Fractional argument: don't model ToInteger coercion.
        let frac = substring_call("abcd", &[1.5]);
        let (out, _, changed, _) = run_pass(program_with_expr(frac, true));
        assert!(!changed, "fractional substring index must not fold");

        // Three arguments: not the substring signature we model.
        let three = substring_call("abcd", &[0.0, 1.0, 2.0]);
        let (out2, _, changed2, _) = run_pass(program_with_expr(three, true));
        assert!(!changed2, "three-arg substring must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        assert!(matches!(extract_expr(&out2), Expression::CallExpression(_)));
    }

    #[test]
    fn substring_on_identifier_receiver_does_not_fold() {
        // `s.substring(1)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "substring")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.substring(1) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- repeat (concatenation) ----------------------

    /// Build `"<recv>".repeat(<count>)`.
    fn repeat_call(recv: &str, count: f64) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "repeat")),
            arguments: vec![num(count, None)],
        })
    }

    #[test]
    fn fold_string_repeat_basic() {
        for (recv, count, expect) in [
            ("ab", 3.0, "ababab"),
            ("x", 5.0, "xxxxx"),
            ("ab", 0.0, ""),  // count 0 → empty
            ("", 9.0, ""),    // empty receiver → empty
            ("é", 2.0, "éé"), // multibyte char duplicated whole, never split
        ] {
            let c = repeat_call(recv, count);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".repeat({count}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "\"{recv}\".repeat({count})")
                }
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn repeat_negative_count_does_not_fold() {
        // JS `"ab".repeat(-1)` throws RangeError — folding would erase the throw.
        let c = repeat_call("ab", -1.0);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "negative repeat count must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn repeat_fractional_count_does_not_fold() {
        // We don't model ToInteger coercion (`"ab".repeat(2.5)` → 2 in JS).
        let c = repeat_call("ab", 2.5);
        let (_out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "fractional repeat count must not fold");
    }

    #[test]
    fn repeat_over_size_cap_does_not_fold() {
        // 10-unit string * 50_000 = 500_000 units > 100_000 cap — DoS guard
        // declines rather than materialize a half-megabyte literal at compile
        // time. Just under the cap folds.
        let over = repeat_call("0123456789", 50_000.0);
        let (out, _, changed, _) = run_pass(program_with_expr(over, true));
        assert!(!changed, "repeat over the size cap must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));

        let under = repeat_call("0123456789", 10_000.0); // 100_000 units, == cap
        let (out2, _, changed2, _) = run_pass(program_with_expr(under, true));
        assert!(changed2, "repeat exactly at the cap should fold");
        match extract_expr(&out2) {
            Expression::StringLiteral(s) => assert_eq!(s.value.len(), 100_000),
            other => panic!("expected a 100_000-byte literal; got {:?}", other),
        }
    }

    #[test]
    fn repeat_huge_count_does_not_overflow_or_fold() {
        // `"x".repeat(1e18)` — checked_mul keeps the length math from
        // overflowing; the call is left for the runtime.
        let c = repeat_call("x", 1e18);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "huge repeat count must not fold (no overflow)");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn repeat_on_identifier_receiver_does_not_fold() {
        // `s.repeat(3)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "repeat")),
            arguments: vec![num(3.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.repeat(3) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- concat (variadic join) ----------------------

    /// Build `"<recv>".concat("<a>", "<b>", …)` from string-literal arguments.
    fn concat_call(recv: &str, args: &[&str]) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "concat")),
            arguments: args.iter().map(|a| string(a, None)).collect(),
        })
    }

    #[test]
    fn fold_string_concat_method_basic() {
        for (recv, args, expect) in [
            ("a", &["b", "c"][..], "abc"),
            ("", &["x"][..], "x"),        // empty receiver
            ("a", &[""][..], "a"),        // empty argument
            ("foo", &["bar"][..], "foobar"),
            ("💩", &["x"][..], "💩x"),  // astral char preserved whole
        ] {
            let c = concat_call(recv, args);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".concat({args:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => {
                    assert_eq!(s.value, expect, "\"{recv}\".concat({args:?})")
                }
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn concat_no_args_folds_to_receiver() {
        // `"abc".concat()` → `"abc"` (identity), still removing the call.
        let c = concat_call("abc", &[]);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"abc\".concat() should fold to the receiver");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected \"abc\"; got {:?}", other),
        }
    }

    /// Build `"<recv>".concat(<args…>)` from arbitrary argument expressions.
    fn concat_call_exprs(recv: &str, args: Vec<Expression>) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), "concat")),
            arguments: args,
        })
    }

    fn folded_concat(recv: &str, args: Vec<Expression>) -> Option<String> {
        let (out, _, changed, _) = run_pass(program_with_expr(concat_call_exprs(recv, args), true));
        if !changed {
            return None;
        }
        match extract_expr(&out) {
            Expression::StringLiteral(s) => Some(s.value.clone()),
            other => panic!("expected a string literal; got {:?}", other),
        }
    }

    #[test]
    fn concat_coerces_numeric_arguments() {
        // `"x".concat(1, 2)` → `"x12"` (ToString on each argument) — gap-143.
        assert_eq!(
            folded_concat("x", vec![num(1.0, None), num(2.0, None)]).as_deref(),
            Some("x12")
        );
        // Mixed with a string argument.
        assert_eq!(
            folded_concat("a", vec![string("b", None), num(3.0, None)]).as_deref(),
            Some("ab3")
        );
    }

    #[test]
    fn concat_coerces_boolean_and_nullish_arguments() {
        // ToString(true)="true", ToString(false)="false".
        assert_eq!(
            folded_concat("a", vec![boolean(true, None), boolean(false, None)]).as_deref(),
            Some("atruefalse")
        );
        // ToString(null)="null", ToString(undefined)="undefined" — note this is
        // DIFFERENT from Array#join, where nullish coerces to "".
        assert_eq!(
            folded_concat(
                "x",
                vec![
                    Expression::NullLiteral(NullLiteral { cv: None }),
                    Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
                ],
            )
            .as_deref(),
            Some("xnullundefined")
        );
    }

    #[test]
    fn concat_object_argument_does_not_fold() {
        // An object argument has a runtime-dependent `toString` → decline.
        let obj = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![],
        });
        assert_eq!(folded_concat("a", vec![obj]), None);
    }

    #[test]
    fn concat_identifier_argument_does_not_fold() {
        // `"a".concat(s)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("a", None), "concat")),
            arguments: vec![ident("s")],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "concat with an identifier argument must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn concat_on_identifier_receiver_does_not_fold() {
        // `s.concat("x")` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "concat")),
            arguments: vec![string("x", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.concat(\"x\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn concat_over_size_cap_does_not_fold() {
        // Receiver (50_001 units) + one 50_000-unit argument = 100_001 > cap,
        // so the defensive DoS guard declines. Just at the cap folds.
        let big = "x".repeat(50_000);
        let over = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(&"x".repeat(50_001), None), "concat")),
            arguments: vec![string(&big, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(over, true));
        assert!(!changed, "concat over the size cap must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));

        let under = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(&"x".repeat(50_000), None), "concat")),
            arguments: vec![string(&big, None)],
        });
        let (out2, _, changed2, _) = run_pass(program_with_expr(under, true));
        assert!(changed2, "concat exactly at the cap should fold");
        match extract_expr(&out2) {
            Expression::StringLiteral(s) => assert_eq!(s.value.len(), 100_000),
            other => panic!("expected a 100_000-byte literal; got {:?}", other),
        }
    }

    // ------------------- padStart / padEnd ---------------------------

    /// Build `"<recv>".<method>(<target>[, "<pad>"])`.
    fn pad_call(recv: &str, method: &str, target: f64, pad: Option<&str>) -> Expression {
        let mut arguments = vec![num(target, None)];
        if let Some(p) = pad {
            arguments.push(string(p, None));
        }
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), method)),
            arguments,
        })
    }

    #[test]
    fn fold_string_pad_basic() {
        // (recv, method, target, pad, expect) — oracle values from V8.
        for (recv, method, target, pad, expect) in [
            ("abc", "padStart", 6.0, None, "   abc"),     // default space pad
            ("abc", "padStart", 6.0, Some("*"), "***abc"),
            ("abc", "padEnd", 6.0, Some("*"), "abc***"),
            ("abc", "padStart", 2.0, Some("*"), "abc"),   // already long enough
            ("abc", "padStart", 6.0, Some("12"), "121abc"), // repeats + truncates
            ("abc", "padEnd", 8.0, Some("xy"), "abcxyxyx"),
            ("abc", "padStart", 6.0, Some(""), "abc"),    // empty pad → unchanged
            ("5", "padStart", 3.0, Some("0"), "005"),     // zero-pad
        ] {
            let c = pad_call(recv, method, target, pad);
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(changed, "\"{recv}\".{method}({target}, {pad:?}) should fold");
            match extract_expr(&out) {
                Expression::StringLiteral(s) => assert_eq!(
                    s.value, expect,
                    "\"{recv}\".{method}({target}, {pad:?})"
                ),
                other => panic!("expected \"{expect}\"; got {:?}", other),
            }
        }
    }

    #[test]
    fn pad_counts_utf16_units() {
        // "💩" is two UTF-16 units, so "💩".padEnd(4, "x") adds two → "💩xx".
        let c = pad_call("💩", "padEnd", 4.0, Some("x"));
        let (out, _, _, _) = run_pass(program_with_expr(c, true));
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(s.value, "💩xx"),
            other => panic!("expected \"💩xx\"; got {:?}", other),
        }
    }

    #[test]
    fn pad_truncating_a_surrogate_pair_does_not_fold() {
        // pad "💩" (2 units) truncated to a 1-unit shortfall is a lone high
        // surrogate — a valid JS string but not a Rust `String`, so decline.
        let c = pad_call("abc", "padStart", 4.0, Some("💩"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "pad truncating a surrogate pair must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn pad_over_size_cap_does_not_fold() {
        // target 200_000 > 100_000 cap — DoS guard declines.
        let c = pad_call("abc", "padStart", 200_000.0, Some("*"));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "pad over the size cap must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn pad_non_integer_target_or_non_literal_pad_does_not_fold() {
        // Fractional target: don't model ToLength coercion.
        let frac = pad_call("abc", "padStart", 5.5, Some("*"));
        let (out, _, changed, _) = run_pass(program_with_expr(frac, true));
        assert!(!changed, "fractional pad target must not fold");

        // Non-literal pad (a numeric pad arg) — only string-literal pads fold.
        let numpad = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("abc", None), "padStart")),
            arguments: vec![num(6.0, None), num(0.0, None)],
        });
        let (out2, _, changed2, _) = run_pass(program_with_expr(numpad, true));
        assert!(!changed2, "non-string-literal pad must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        assert!(matches!(extract_expr(&out2), Expression::CallExpression(_)));
    }

    #[test]
    fn pad_on_identifier_receiver_does_not_fold() {
        // `s.padStart(5)` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "padStart")),
            arguments: vec![num(5.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.padStart(5) must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    // ------------------- replace / replaceAll ------------------------

    /// Build `"recv".method("from", "to")` — the 2-string-arg form.
    fn replace_call(recv: &str, method: &str, from: &str, to: &str) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string(recv, None), method)),
            arguments: vec![string(from, None), string(to, None)],
        })
    }

    /// Assert `"recv".method("from","to")` folds to the string `expect`.
    fn assert_replace(recv: &str, method: &str, from: &str, to: &str, expect: &str) {
        let c = replace_call(recv, method, from, to);
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(changed, "\"{recv}\".{method}(\"{from}\",\"{to}\") should fold");
        match extract_expr(&out) {
            Expression::StringLiteral(s) => assert_eq!(
                s.value, expect,
                "\"{recv}\".{method}(\"{from}\",\"{to}\")"
            ),
            other => panic!("expected \"{expect}\"; got {:?}", other),
        }
    }

    #[test]
    fn replace_first_match_only() {
        // `replace` substitutes only the FIRST occurrence (V8 oracle).
        assert_replace("aXbXc", "replace", "X", "-", "a-bXc");
        assert_replace("a-b-c", "replace", "-", "_", "a_b-c");
    }

    #[test]
    fn replace_all_matches() {
        // `replaceAll` substitutes EVERY occurrence (V8 oracle).
        assert_replace("a-b-c", "replaceAll", "-", "_", "a_b_c");
        assert_replace("aXbXc", "replaceAll", "X", "-", "a-b-c");
    }

    #[test]
    fn replace_matches_from_literally_not_as_regex() {
        // The string overload treats `from` literally — `.` is not "any
        // char". `"a.b".replace(".","X")` → `"aXb"`, not `"XXX"`.
        assert_replace("a.b", "replace", ".", "X", "aXb");
        assert_replace("a.b.c", "replaceAll", ".", "X", "aXbXc");
    }

    #[test]
    fn replace_no_match_is_identity() {
        // No occurrence of `from` → the receiver unchanged (but still folds
        // the call away).
        assert_replace("abc", "replace", "z", "Q", "abc");
        assert_replace("abc", "replaceAll", "z", "Q", "abc");
    }

    #[test]
    fn replace_declines_dollar_in_replacement() {
        // A `$` in `to` triggers V8's substitution patterns ($$, $&, …),
        // which a literal copy would not reproduce — decline.
        for method in ["replace", "replaceAll"] {
            let c = replace_call("abc", method, "b", "$&");
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "{method} with `$` in replacement must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn replace_declines_empty_search() {
        // An empty `from` has V8 boundary-insertion semantics a literal
        // find/replace can't reproduce — decline.
        for method in ["replace", "replaceAll"] {
            let c = replace_call("abc", method, "", "X");
            let (out, _, changed, _) = run_pass(program_with_expr(c, true));
            assert!(!changed, "{method} with empty search must not fold");
            assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
        }
    }

    #[test]
    fn replace_non_string_argument_does_not_fold() {
        // A numeric `from` (or `to`) is not the string-overload we model.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("a1b", None), "replace")),
            arguments: vec![num(1.0, None), string("X", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "replace with a non-string argument must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn replace_on_identifier_receiver_does_not_fold() {
        // `s.replace("a","b")` needs the runtime value of `s`.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(ident("s"), "replace")),
            arguments: vec![string("a", None), string("b", None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "s.replace(\"a\",\"b\") must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn replace_wrong_arity_does_not_fold() {
        // One-arg `"abc".replace("a")` is not the 2-arg form we fold.
        let c = call1(string("abc", None), "replace", string("a", None));
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "one-arg replace must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
    }

    #[test]
    fn replace_over_size_cap_does_not_fold() {
        // A 100k-byte all-`a` string, `replaceAll("a","bb")` → 200k bytes,
        // over the cap — decline rather than materialize a huge literal at
        // compile time (DoS guard, mirrors repeat/pad).
        let big = "a".repeat(100_000);
        let c = replace_call(&big, "replaceAll", "a", "bb");
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "over-cap replaceAll must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));

        // Just under the cap still folds.
        let small = "a".repeat(100);
        let c2 = replace_call(&small, "replaceAll", "a", "b");
        let (out2, _, changed2, _) = run_pass(program_with_expr(c2, true));
        assert!(changed2, "under-cap replaceAll should fold");
        assert!(matches!(extract_expr(&out2), Expression::StringLiteral(_)));
    }

    // ------------------- logical (short-circuit) ---------------------

    fn logical(op: LogicalOperator, l: Expression, r: Expression) -> Expression {
        Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: op,
            left: Box::new(l),
            right: Box::new(r),
        })
    }

    /// `a && (b && c)` re-associates left to `(a && b) && c` — the outer node's
    /// left becomes a `&&` LogicalExpression and its right becomes the bare `c`.
    #[test]
    fn logical_and_right_nest_reassociates_left() {
        let inner = logical(LogicalOperator::And, ident("b"), ident("c"));
        let expr = logical(LogicalOperator::And, ident("a"), inner);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::LogicalExpression(top) => {
                assert_eq!(top.operator, LogicalOperator::And);
                assert!(
                    matches!(top.left.as_ref(), Expression::LogicalExpression(_)),
                    "left should be the (a && b) node"
                );
                assert!(
                    matches!(top.right.as_ref(), Expression::Identifier(id) if id.name == "c"),
                    "right should be the bare `c`"
                );
            }
            other => panic!("expected a LogicalExpression; got {other:?}"),
        }
    }

    /// `a || (b || c)` re-associates the same way.
    #[test]
    fn logical_or_right_nest_reassociates_left() {
        let inner = logical(LogicalOperator::Or, ident("b"), ident("c"));
        let expr = logical(LogicalOperator::Or, ident("a"), inner);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::LogicalExpression(top) => {
                assert!(matches!(top.left.as_ref(), Expression::LogicalExpression(_)));
                assert!(matches!(top.right.as_ref(), Expression::Identifier(id) if id.name == "c"));
            }
            other => panic!("expected a LogicalExpression; got {other:?}"),
        }
    }

    /// `a && (b || c)` is LEFT ALONE — a mixed-operator right nest cannot
    /// re-associate (different precedence/grouping), so the parens stay.
    #[test]
    fn logical_mixed_operator_right_nest_not_reassociated() {
        let inner = logical(LogicalOperator::Or, ident("b"), ident("c"));
        let expr = logical(LogicalOperator::And, ident("a"), inner);
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::LogicalExpression(top) => {
                assert_eq!(top.operator, LogicalOperator::And);
                // Right stays the `||` node (not flattened into the `&&`).
                match top.right.as_ref() {
                    Expression::LogicalExpression(r) => {
                        assert_eq!(r.operator, LogicalOperator::Or)
                    }
                    other => panic!("expected the `||` node preserved on the right; got {other:?}"),
                }
            }
            other => panic!("expected a LogicalExpression; got {other:?}"),
        }
    }

    /// `a && (b && (c && d))` fully flattens to `((a && b) && c) && d` under the
    /// pass's fixed-point iteration (one re-association step per node).
    #[test]
    fn logical_deep_right_nest_fully_flattens() {
        let cd = logical(LogicalOperator::And, ident("c"), ident("d"));
        let bcd = logical(LogicalOperator::And, ident("b"), cd);
        let expr = logical(LogicalOperator::And, ident("a"), bcd);
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        // The top node's right must be the bare `d` (fully left-nested).
        match extract_expr(&out) {
            Expression::LogicalExpression(top) => {
                assert!(
                    matches!(top.right.as_ref(), Expression::Identifier(id) if id.name == "d"),
                    "fully-flattened top-right should be the bare `d`; got {:?}",
                    top.right
                );
                assert!(matches!(top.left.as_ref(), Expression::LogicalExpression(_)));
            }
            other => panic!("expected a LogicalExpression; got {other:?}"),
        }
    }

    #[test]
    fn fold_logical_and_left_falsy_returns_left() {
        // `false && x → false`. The right operand (an unfoldable
        // expression) is dropped.
        let expr = Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: LogicalOperator::And,
            left: Box::new(boolean(false, None)),
            right: Box::new(ident("x")),
        });
        let (out, contribs, _, _) = run_pass(program_with_expr(expr, true));
        assert_eq!(contribs.len(), 1);
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(!b.value),
            other => panic!("expected false; got {:?}", other),
        }
    }

    #[test]
    fn fold_logical_and_left_truthy_returns_right() {
        // `true && x → x`. The right operand wins.
        let expr = Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: LogicalOperator::And,
            left: Box::new(boolean(true, None)),
            right: Box::new(ident("x")),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::Identifier(i) => assert_eq!(i.name, "x"),
            other => panic!("expected identifier 'x'; got {:?}", other),
        }
    }

    #[test]
    fn fold_logical_or_left_truthy_returns_left() {
        let expr = Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: LogicalOperator::Or,
            left: Box::new(boolean(true, None)),
            right: Box::new(ident("x")),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(b.value),
            other => panic!("expected true; got {:?}", other),
        }
    }

    #[test]
    fn fold_logical_nullish_left_null_returns_right() {
        // `null ?? x → x`.
        let expr = Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: LogicalOperator::NullishCoalescing,
            left: Box::new(null(None)),
            right: Box::new(ident("x")),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::Identifier(i) => assert_eq!(i.name, "x"),
            other => panic!("expected identifier 'x'; got {:?}", other),
        }
    }

    #[test]
    fn fold_logical_nullish_left_zero_returns_left() {
        // `0 ?? x → 0` — zero is not nullish.
        let expr = Expression::LogicalExpression(LogicalExpression {
            cv: Some("l.1".to_string()),
            operator: LogicalOperator::NullishCoalescing,
            left: Box::new(num(0.0, None)),
            right: Box::new(ident("x")),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 0.0),
            other => panic!("expected 0; got {:?}", other),
        }
    }

    // ------------------- conditional ---------------------------------

    #[test]
    fn fold_conditional_truthy_test_keeps_consequent() {
        let expr = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("c.1".to_string()),
            test: Box::new(boolean(true, None)),
            consequent: Box::new(num(1.0, None)),
            alternate: Box::new(num(2.0, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 1.0),
            other => panic!("expected 1; got {:?}", other),
        }
    }

    #[test]
    fn fold_conditional_falsy_test_keeps_alternate() {
        let expr = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("c.1".to_string()),
            test: Box::new(num(0.0, None)),
            consequent: Box::new(num(1.0, None)),
            alternate: Box::new(num(2.0, None)),
        });
        let (out, _, _, _) = run_pass(program_with_expr(expr, true));
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 2.0),
            other => panic!("expected 2; got {:?}", other),
        }
    }

    // ------------- ternary equal-branch collapse (t?X:X → X) ----------

    /// `cond ? then : else` with the given CV, boxing each arm.
    fn conditional(test: Expression, then: Expression, els: Expression) -> Expression {
        Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("cond.cv".to_string()),
            test: Box::new(test),
            consequent: Box::new(then),
            alternate: Box::new(els),
        })
    }

    /// A side-effecting `f()` call — the canonical impure test operand.
    fn bare_call(callee: &str) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident(callee)),
            arguments: vec![],
        })
    }

    #[test]
    fn ternary_equal_identifier_branches_collapse_to_branch() {
        // `a ? b : b` → `b` (test `a` is a side-effect-free identifier).
        let expr = conditional(ident("a"), ident("b"), ident("b"));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "a?b:b should collapse to b");
        match extract_expr(&out) {
            Expression::Identifier(id) => assert_eq!(id.name, "b"),
            other => panic!("expected identifier b; got {other:?}"),
        }
    }

    #[test]
    fn ternary_equal_literal_branches_collapse() {
        // `a ? 1 : 1` → `1`.
        let expr = conditional(ident("a"), num(1.0, None), num(1.0, None));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "a?1:1 should collapse to 1");
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 1.0),
            other => panic!("expected 1; got {other:?}"),
        }
    }

    #[test]
    fn ternary_equal_member_branches_collapse_under_free_test() {
        // `a ? b.c : b.c` → `b.c`. The test `a` is free; the (equal) member
        // arms are evaluated exactly once either way.
        let expr = conditional(
            ident("a"),
            member(ident("b"), "c"),
            member(ident("b"), "c"),
        );
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "a?b.c:b.c should collapse to b.c");
        match extract_expr(&out) {
            Expression::MemberExpression(_) => {}
            other => panic!("expected member b.c; got {other:?}"),
        }
    }

    #[test]
    fn ternary_member_test_still_collapses_free_by_contract() {
        // `a.p ? b : b` → `b`. A member READ is side-effect-free under the
        // crate-wide contract (matching Closure; a getter is not modelled), so
        // the equal-branch collapse fires.
        let expr = conditional(member(ident("a"), "p"), ident("b"), ident("b"));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed, "a.p?b:b should collapse (member test is free)");
        assert!(matches!(extract_expr(&out), Expression::Identifier(id) if id.name == "b"));
    }

    #[test]
    fn ternary_impure_test_declines_collapse() {
        // `f() ? b : b` is LEFT INTACT — collapsing would drop the `f()` call.
        // (Closure rewrites this to `(f(),b)`; that sequence build is a
        // deliberately-declined follow-up, not a miscompile.)
        let expr = conditional(bare_call("f"), ident("b"), ident("b"));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "f()?b:b must NOT collapse (impure test)");
        match extract_expr(&out) {
            Expression::ConditionalExpression(c) => {
                assert!(matches!(&*c.test, Expression::CallExpression(_)));
            }
            other => panic!("expected the ternary to survive; got {other:?}"),
        }
    }

    #[test]
    fn ternary_unequal_branches_not_collapsed() {
        // `a ? b : c` (b ≠ c) is a real branch — never collapse.
        let expr = conditional(ident("a"), ident("b"), ident("c"));
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "a?b:c must not collapse");
        assert!(matches!(
            extract_expr(&out),
            Expression::ConditionalExpression(_)
        ));
    }

    // ------------------- doesn't over-fold ---------------------------

    #[test]
    fn unfoldable_expressions_pass_through() {
        // 1 + x — x isn't a literal, so the BinaryExpression stays.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(num(1.0, None)),
            right: Box::new(ident("x")),
        });
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed, "no fold should happen");
        assert!(contribs.is_empty());
        assert!(matches!(
            extract_expr(&out),
            Expression::BinaryExpression(_)
        ));
    }

    #[test]
    fn mixed_type_loose_equality_with_unrecognised_string_not_folded() {
        // **Pre-gap-004**: this test asserted that `1 == "1"` was NOT
        // folded (sound default). **Post-gap-004** (CLOC12.22), `1 == "1"`
        // *does* fold to `true` because `"1"` is in the strict recognised
        // §StringToNumber subset.
        //
        // The original intent — "don't fold mixed-type comparisons we
        // can't soundly evaluate" — is still pinned, but now the
        // canonical example is a string that the conservative helper
        // bails on. `"hi"` triggers `js_string_to_number_strict` to
        // return `None`, so the fold doesn't fire and the BinaryExpression
        // survives.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Eq,
            left: Box::new(num(1.0, None)),
            right: Box::new(string("hi", None)),
        });
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed);
        assert!(matches!(
            extract_expr(&out),
            Expression::BinaryExpression(_)
        ));
    }

    // ------------------- untraced (cv = None) mode -------------------

    #[test]
    fn fold_in_untraced_mode_skips_cv_and_contributions() {
        // Same 2 + 3 fold but with cv: None everywhere. The pass
        // should still fold, mark changed=true, but emit no
        // contributions and produce a NumericLiteral with cv: None.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0, None)),
            right: Box::new(num(3.0, None)),
        });
        let (out, contribs, changed, _) = run_pass(program_with_expr(expr, false));
        assert!(changed, "untraced fold still marks changed=true");
        assert!(
            contribs.is_empty(),
            "untraced fold emits no CV contributions; got {:?}",
            contribs
        );
        match extract_expr(&out) {
            Expression::NumericLiteral(n) => {
                assert_eq!(n.value, 5.0);
                assert!(
                    n.cv.is_none(),
                    "untraced fold produces cv: None; got {:?}",
                    n.cv
                );
            }
            other => panic!("expected NumericLiteral(5); got {:?}", other),
        }
    }

    // ------------------- recursion through declarations / statements -

    #[test]
    fn recurses_through_variable_declarators() {
        // const x = 1 + 2;  →  const x = 3;
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::VariableDeclaration(VariableDeclaration {
                cv: Some("vd.1".to_string()),
                kind: coding_adventures_javascript_ast::VarKind::Const,
                declarations: vec![VariableDeclarator {
                    cv: Some("vdr.1".to_string()),
                    id: coding_adventures_javascript_ast::BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    }),
                    init: Some(Expression::BinaryExpression(BinaryExpression {
                        cv: Some("bin.1".to_string()),
                        operator: BinaryOperator::Add,
                        left: Box::new(num(1.0, None)),
                        right: Box::new(num(2.0, None)),
                    })),
                }],
            }),
        )]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        let ProgramItem::Declaration(Declaration::VariableDeclaration(v)) = &out.body[0] else {
            panic!("expected VariableDeclaration");
        };
        match &v.declarations[0].init {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 3.0),
            other => panic!("expected init=3; got {:?}", other),
        }
    }

    #[test]
    fn recurses_through_if_test_and_branches() {
        // if (1 < 2) { 3 + 4 } else { 5 * 6 } — test folds to true,
        // both branches' expressions fold independently.
        let prog = program().with_body(vec![ProgramItem::Statement(Statement::if_statement(
            IfStatement {
                cv: Some("if.1".to_string()),
                test: Expression::BinaryExpression(BinaryExpression {
                    cv: Some("bin.t".to_string()),
                    operator: BinaryOperator::Lt,
                    left: Box::new(num(1.0, None)),
                    right: Box::new(num(2.0, None)),
                }),
                consequent: Box::new(Statement::expression_statement(ExpressionStatement {
                    cv: Some("es.c".to_string()),
                    expression: Expression::BinaryExpression(BinaryExpression {
                        cv: Some("bin.c".to_string()),
                        operator: BinaryOperator::Add,
                        left: Box::new(num(3.0, None)),
                        right: Box::new(num(4.0, None)),
                    }),
                })),
                alternate: Some(Box::new(Statement::expression_statement(
                    ExpressionStatement {
                        cv: Some("es.a".to_string()),
                        expression: Expression::BinaryExpression(BinaryExpression {
                            cv: Some("bin.a".to_string()),
                            operator: BinaryOperator::Mul,
                            left: Box::new(num(5.0, None)),
                            right: Box::new(num(6.0, None)),
                        }),
                    },
                ))),
            },
        ))]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        // 3 folds: 1<2, 3+4, 5*6. (The if test stays an if; we don't
        // fold the if itself — fold-control-flow's job in a later pass.)
        assert_eq!(contribs.len(), 3);
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::IfStatement(if_s))) =
            &out.body[0]
        else {
            panic!("expected IfStatement");
        };
        // Test should be BooleanLiteral(true) after fold.
        match &if_s.test {
            Expression::BooleanLiteral(b) => assert!(b.value),
            other => panic!("expected BooleanLiteral(true); got {:?}", other),
        }
    }

    // ------------------- pipeline integration ------------------------

    // ------------------- negation push (`!(a == b)` → `a != b`) ----------

    /// `!( left <op> right )` over two identifiers, traced.
    fn not_of_binary(op: BinaryOperator) -> Expression {
        Expression::UnaryExpression(UnaryExpression {
            cv: Some("not.1".to_string()),
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: op,
                left: Box::new(ident("a")),
                right: Box::new(ident("b")),
            })),
        })
    }

    fn assert_negation_pushes(input_op: BinaryOperator, expected_op: BinaryOperator) {
        let (out, contribs, changed, _) = run_pass(program_with_expr(not_of_binary(input_op), true));
        assert!(changed, "negation push should report a change for {input_op:?}");
        match extract_expr(&out) {
            Expression::BinaryExpression(b) => {
                assert_eq!(
                    b.operator, expected_op,
                    "{input_op:?} should invert to {expected_op:?}, got {:?}",
                    b.operator
                );
                // Operands are preserved, and the `!` wrapper is gone.
                assert!(matches!(b.left.as_ref(), Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(b.right.as_ref(), Expression::Identifier(i) if i.name == "b"));
            }
            other => panic!("expected BinaryExpression, got {other:?}"),
        }
        // The rewrite must leave a correlation-vector contribution.
        assert!(
            contribs.iter().any(|c| c.source == "constant-fold"),
            "negation push must emit a CV contribution"
        );
    }

    #[test]
    fn negation_pushes_through_loose_equality() {
        assert_negation_pushes(BinaryOperator::Eq, BinaryOperator::NotEq);
    }

    #[test]
    fn negation_pushes_through_loose_inequality() {
        assert_negation_pushes(BinaryOperator::NotEq, BinaryOperator::Eq);
    }

    #[test]
    fn negation_pushes_through_strict_equality() {
        assert_negation_pushes(BinaryOperator::StrictEq, BinaryOperator::StrictNotEq);
    }

    #[test]
    fn negation_pushes_through_strict_inequality() {
        assert_negation_pushes(BinaryOperator::StrictNotEq, BinaryOperator::StrictEq);
    }

    #[test]
    fn negation_does_not_push_through_relational_operators() {
        // `!(a < b)` must NOT become `a >= b` — they differ when an operand
        // is NaN (`!(NaN < 1)` is `true`, `NaN >= 1` is `false`). The `!`
        // wrapper must survive unchanged.
        for op in [
            BinaryOperator::Lt,
            BinaryOperator::LtEq,
            BinaryOperator::Gt,
            BinaryOperator::GtEq,
        ] {
            let (out, _contribs, _changed, _) =
                run_pass(program_with_expr(not_of_binary(op), true));
            match extract_expr(&out) {
                Expression::UnaryExpression(u) => {
                    assert_eq!(u.operator, UnaryOperator::Not, "outer `!` must survive for {op:?}");
                    assert!(
                        matches!(u.argument.as_ref(), Expression::BinaryExpression(b) if b.operator == op),
                        "inner relational op {op:?} must be unchanged"
                    );
                }
                other => panic!("expected the `!` to survive for {op:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn negation_of_equality_with_foldable_operands_still_folds_to_literal() {
        // `!(1 == 1)` should fold to the boolean `false` (the literal path),
        // not push to `1 != 1`. The literal fold runs first.
        let expr = Expression::UnaryExpression(UnaryExpression {
            cv: Some("not.1".to_string()),
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(Expression::BinaryExpression(BinaryExpression {
                cv: Some("bin.1".to_string()),
                operator: BinaryOperator::Eq,
                left: Box::new(num(1.0, None)),
                right: Box::new(num(1.0, None)),
            })),
        });
        let (out, _contribs, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(changed);
        match extract_expr(&out) {
            Expression::BooleanLiteral(b) => assert!(!b.value, "!(1 == 1) is false"),
            other => panic!("expected BooleanLiteral(false), got {other:?}"),
        }
    }

    #[test]
    fn integrates_with_pass_pipeline_as_solo_pass() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(out.execution_order, vec!["constant-fold".to_string()]);
        assert!(out.stats.contains_key("constant-fold"));
        // The pipeline now iterates FixedPoint passes to a fixed point;
        // a non-changing solo pass converges in one sweep, so the old
        // "not-yet-iterated" limitation note is gone.
        assert!(!out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"));
    }

    // =================================================================
    // Array.prototype.join folding (gap-142 / CLOC12.141)
    // =================================================================

    /// Build `[<elements>].join(<sep_arg?>)` as a CallExpression.
    fn join_call(elements: Vec<Option<Expression>>, sep_arg: Option<Expression>) -> Expression {
        let arr = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements,
        });
        Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(arr, "join")),
            arguments: sep_arg.into_iter().collect(),
        })
    }

    #[test]
    fn fold_array_join_strings_with_separator() {
        let c = join_call(
            vec![
                Some(string("a", None)),
                Some(string("b", None)),
                Some(string("c", None)),
            ],
            Some(string("-", None)),
        );
        assert_eq!(folded_string(c).as_deref(), Some("a-b-c"));
    }

    #[test]
    fn fold_array_join_default_separator_is_comma() {
        // No separator argument → `,` (ECMAScript §23.1.3.16).
        let c = join_call(
            vec![
                Some(num(1.0, None)),
                Some(num(2.0, None)),
                Some(num(3.0, None)),
            ],
            None,
        );
        assert_eq!(folded_string(c).as_deref(), Some("1,2,3"));
    }

    #[test]
    fn fold_array_join_coerces_mixed_constants() {
        // Numbers, booleans, null, undefined, and holes each coerce the way
        // `join` does: nullish + holes → "", number/boolean → String(...).
        let c = join_call(
            vec![
                Some(num(1.0, None)),
                Some(boolean(true, None)),
                Some(Expression::NullLiteral(NullLiteral { cv: None })),
                None, // a hole
                Some(string("x", None)),
            ],
            Some(string(",", None)),
        );
        // 1 , "true" , "" , "" , "x"  →  "1,true,,,x"
        assert_eq!(folded_string(c).as_deref(), Some("1,true,,,x"));
    }

    #[test]
    fn fold_array_join_empty_array_is_empty_string() {
        let c = join_call(vec![], Some(string("-", None)));
        assert_eq!(folded_string(c).as_deref(), Some(""));
    }

    #[test]
    fn array_join_non_constant_element_does_not_fold() {
        // An identifier element has a runtime-dependent string form → decline.
        let c = join_call(
            vec![Some(string("a", None)), Some(ident("x"))],
            Some(string("-", None)),
        );
        assert_eq!(folded_string(c), None);
    }

    #[test]
    fn array_join_non_string_separator_does_not_fold() {
        // A numeric separator (`[1,2].join(0)` → "102") coerces to "0"; we
        // leave it for the runtime to keep the fold obviously correct.
        let c = join_call(
            vec![Some(num(1.0, None)), Some(num(2.0, None))],
            Some(num(0.0, None)),
        );
        assert_eq!(folded_string(c), None);
    }

    #[test]
    fn array_join_nested_array_element_does_not_fold() {
        // A nested array element runs its own `toString` at runtime → decline.
        let nested = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(num(1.0, None))],
        });
        let c = join_call(
            vec![Some(string("a", None)), Some(nested)],
            Some(string("-", None)),
        );
        assert_eq!(folded_string(c), None);
    }
}
