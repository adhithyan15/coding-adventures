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
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, BinaryExpression,
    BinaryOperator, BlockStatement, BooleanLiteral, CallExpression, ConditionalExpression,
    Declaration, Expression, ExpressionStatement, ForInStatement, ForInit, ForOfStatement,
    ForStatement,
    FunctionDeclaration,
    IfStatement, LogicalExpression, LogicalOperator, MemberExpression, NullLiteral, NumericLiteral,
    ObjectExpression, Program, ProgramItem, Property, PropertyKey, ReturnStatement, Statement,
    StringLiteral, UnaryExpression, UnaryOperator, UndefinedLiteral, VariableDeclaration,
    DoWhileStatement, VariableDeclarator, WhileStatement,
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
        let mut state = FoldState {
            cv: ctx.cv,
            contributions: Vec::new(),
            changed: false,
            nodes_touched: 0,
        };

        let new_program = fold_program(ctx.program, &mut state);

        Ok(PassOutput {
            program: new_program,
            contributions: state.contributions,
            changed: state.changed,
            diagnostics: Vec::new(),
            stats: PassStats {
                nodes_touched: state.nodes_touched,
            },
        })
    }
}

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
                params: f.params.clone(),
                body: BlockStatement {
                    cv: f.body.cv.clone(),
                    body: f.body.body.iter().map(|s| fold_statement(s, st)).collect(),
                },
                generator: f.generator,
                is_async: f.is_async,
            })
        }
    }
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
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => expr.clone(),

        Expression::BinaryExpression(b) => fold_binary(b, st),
        Expression::LogicalExpression(l) => fold_logical(l, st),
        Expression::UnaryExpression(u) => fold_unary(u, st),
        Expression::ConditionalExpression(c) => fold_conditional(c, st),

        // Recurse but don't collapse — these have runtime semantics.
        Expression::AssignmentExpression(a) => {
            Expression::AssignmentExpression(AssignmentExpression {
                cv: a.cv.clone(),
                operator: a.operator,
                left: a.left.clone(),
                right: Box::new(fold_expression(&a.right, st)),
            })
        }
        Expression::CallExpression(c) => fold_call(c, st),
        Expression::MemberExpression(m) => fold_member(m, st),
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
                .map(|p| Property {
                    cv: p.cv.clone(),
                    kind: p.kind,
                    key: match &p.key {
                        // Identifier / literal keys: pass through.
                        PropertyKey::Identifier(i) => PropertyKey::Identifier(i.clone()),
                        PropertyKey::StringLiteral(s) => PropertyKey::StringLiteral(s.clone()),
                        PropertyKey::NumericLiteral(n) => PropertyKey::NumericLiteral(n.clone()),
                        PropertyKey::Expression(e) => {
                            PropertyKey::Expression(Box::new(fold_expression(e, st)))
                        }
                    },
                    value: Box::new(fold_expression(&p.value, st)),
                    computed: p.computed,
                    shorthand: p.shorthand,
                    method: p.method,
                })
                .collect(),
        }),
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
                // form). Every argument must itself be a STRING literal — JS
                // coerces non-strings via `ToString` (`"a".concat(1)` → `"a1"`),
                // but we don't model that coercion, so a numeric/identifier
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
    // `String.prototype`; `parseInt`/`parseFloat` are *free identifiers*, so a
    // local binding (`let parseInt = …`) can additionally mask them. We fold
    // them anyway — matching Closure Compiler, which treats redefining these
    // globals as out of scope — but ONLY when the callee is the bare identifier
    // `parseInt`/`parseFloat`, never a member access (`window.parseInt`, which
    // reaches the MemberExpression arm above and is left untouched).
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

    Expression::CallExpression(CallExpression {
        cv: c.cv.clone(),
        callee: Box::new(callee),
        arguments,
    })
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
fn fold_string_concat_call(value: &str, args: &[Expression]) -> Option<String> {
    /// Cap on the folded result's length, in UTF-16 code units.
    const MAX_CONCAT_UNITS: usize = 100_000;

    let mut units = value.encode_utf16().count();
    let mut out = String::from(value);
    for a in args {
        let piece = match a {
            Expression::StringLiteral(s) => &s.value,
            _ => return None,
        };
        units = units.checked_add(piece.encode_utf16().count())?;
        if units > MAX_CONCAT_UNITS {
            return None;
        }
        out.push_str(piece);
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
    }

    Expression::MemberExpression(MemberExpression {
        cv: m.cv.clone(),
        object: Box::new(object),
        property: Box::new(property),
        computed: m.computed,
    })
}

// ---------------------------------------------------------------------
// Binary
// ---------------------------------------------------------------------

fn fold_binary(b: &BinaryExpression, st: &mut FoldState) -> Expression {
    // First recurse into children. By the time we look at left/right
    // they're already folded — that's what gives us `1 + (2 * 3) → 7`
    // in one bottom-up walk.
    let left = fold_expression(&b.left, st);
    let right = fold_expression(&b.right, st);

    // Try to fold. If we can't, return a new BinaryExpression with the
    // (possibly folded) children.
    if let Some(value) = try_fold_binary_op(b.operator, &left, &right) {
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

/// A folded value, kept separate from the AST node enum so the fold
/// logic can produce a value first and only later attach a fresh
/// CvId. Converted to an [`Expression`] via [`stamp_literal_cv`].
#[derive(Debug)]
enum FoldedLiteral {
    Number(f64),
    String(String),
    Boolean(bool),
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
    fn computed_string_length_does_not_fold() {
        // `"abc"["length"]` is the computed form — deliberately left alone.
        let m = Expression::MemberExpression(MemberExpression {
            cv: Some("m.cv".to_string()),
            object: Box::new(string("abc", None)),
            property: Box::new(string("length", None)),
            computed: true,
        });
        let (out, _, changed, _) = run_pass(program_with_expr(m, true));
        assert!(!changed, "computed \"abc\"[\"length\"] must not fold");
        assert!(matches!(
            extract_expr(&out),
            Expression::MemberExpression(_)
        ));
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
    fn computed_string_casing_does_not_fold() {
        // `"abc"["toUpperCase"]()` is the computed form — left alone.
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
        assert!(!changed, "computed casing call must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
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
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
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

    #[test]
    fn concat_non_string_argument_does_not_fold() {
        // `"a".concat(1)` is `"a1"` in JS via ToString, but we don't model that
        // coercion — leave it for the runtime rather than guess.
        let c = Expression::CallExpression(CallExpression {
            cv: Some("c.cv".to_string()),
            callee: Box::new(member(string("a", None), "concat")),
            arguments: vec![num(1.0, None)],
        });
        let (out, _, changed, _) = run_pass(program_with_expr(c, true));
        assert!(!changed, "concat with a numeric argument must not fold");
        assert!(matches!(extract_expr(&out), Expression::CallExpression(_)));
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
}
