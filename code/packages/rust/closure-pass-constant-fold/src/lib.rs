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
//! - Bitwise (`&`, `|`, `^`, `<<`, `>>`, `>>>`) — NOW FOLDED on two numeric
//!   literals (CLOC15.D) via ES `ToInt32`/`ToUint32` 32-bit semantics. See
//!   [`to_int32`] / [`to_uint32`]; `>>>` yields an unsigned result that can
//!   exceed `i32::MAX`.
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
    Declaration, Expression, ExpressionStatement, ForInit, ForStatement, FunctionDeclaration,
    IfStatement, LogicalExpression, LogicalOperator, MemberExpression, NullLiteral, NumericLiteral,
    ObjectExpression, Program, ProgramItem, Property, PropertyKey, ReturnStatement, Statement,
    StringLiteral, UnaryExpression, UnaryOperator, UndefinedLiteral, VariableDeclaration,
    VariableDeclarator, WhileStatement,
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
        | TaggedStatement::EmptyStatement(_) => stmt.clone(),
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
        Expression::CallExpression(c) => Expression::CallExpression(CallExpression {
            cv: c.cv.clone(),
            callee: Box::new(fold_expression(&c.callee, st)),
            arguments: c.arguments.iter().map(|a| fold_expression(a, st)).collect(),
        }),
        Expression::MemberExpression(m) => Expression::MemberExpression(MemberExpression {
            cv: m.cv.clone(),
            object: Box::new(fold_expression(&m.object, st)),
            property: Box::new(fold_expression(&m.property, st)),
            computed: m.computed,
        }),
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
        // Skipped: BitNot (int32 coercion), Delete (side effects).
        _ => None,
    };

    if let Some(value) = folded {
        let parent = u.cv.clone();
        let before = format!("{}({})", unary_op_label(u.operator), lit_label(&arg));
        let after = literal_label(&value);
        let new_cv = st.fork_cv(&parent, &before, &after);
        return stamp_literal_cv(value, new_cv);
    }

    Expression::UnaryExpression(UnaryExpression {
        cv: u.cv.clone(),
        operator: u.operator,
        prefix: u.prefix,
        argument: Box::new(arg),
    })
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
