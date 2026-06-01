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
//! - Bitwise (`&`, `|`, `^`, `<<`, `>>`, `>>>`) — requires int32 coercion
//!   semantics; safe-but-non-trivial; queued for Phase 1.x once we have
//!   real test fixtures driving demand.
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
    IfStatement, LogicalExpression, LogicalOperator, MemberExpression, NullLiteral,
    NumericLiteral, ObjectExpression, Program, ProgramItem, Property, PropertyKey,
    ReturnStatement, Statement, StringLiteral, UnaryExpression, UnaryOperator,
    VariableDeclaration, VariableDeclarator, WhileStatement,
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
    fn fork_cv(
        &mut self,
        parent: &Option<String>,
        before: &str,
        after: &str,
    ) -> Option<String> {
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
        TaggedStatement::BlockStatement(s) => {
            TaggedStatement::BlockStatement(BlockStatement {
                cv: s.cv.clone(),
                body: s.body.iter().map(|x| fold_statement(x, st)).collect(),
            })
        }
        TaggedStatement::IfStatement(s) => TaggedStatement::IfStatement(IfStatement {
            cv: s.cv.clone(),
            test: fold_expression(&s.test, st),
            consequent: Box::new(fold_statement(&s.consequent, st)),
            alternate: s.alternate.as_ref().map(|a| Box::new(fold_statement(a, st))),
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
        TaggedStatement::ReturnStatement(s) => {
            TaggedStatement::ReturnStatement(ReturnStatement {
                cv: s.cv.clone(),
                argument: s.argument.as_ref().map(|e| fold_expression(e, st)),
            })
        }
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
                    body: f
                        .body
                        .body
                        .iter()
                        .map(|s| fold_statement(s, st))
                        .collect(),
                },
                generator: f.generator,
                is_async: f.is_async,
            })
        }
    }
}

fn fold_variable_declaration(
    v: &VariableDeclaration,
    st: &mut FoldState,
) -> VariableDeclaration {
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
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => expr.clone(),

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
                        PropertyKey::StringLiteral(s) => {
                            PropertyKey::StringLiteral(s.clone())
                        }
                        PropertyKey::NumericLiteral(n) => {
                            PropertyKey::NumericLiteral(n.clone())
                        }
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
        let before = format!("({}) {} ({})", lit_label(&left), op_label(b.operator), lit_label(&right));
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
        (Expression::NumericLiteral(a), Expression::NumericLiteral(b)) => (Some(a.value), Some(b.value)),
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
        if matches!(left, Expression::StringLiteral(_)) || matches!(right, Expression::StringLiteral(_)) {
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
        _ => None,
    }
}

/// Best-effort static string rendering of a literal expression. Used
/// for `+` concatenation folding. Returns `None` for non-literal or
/// not-statically-renderable inputs.
fn literal_to_string(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.clone()),
        Expression::NumericLiteral(n) => Some(format_js_number(n.value)),
        Expression::BooleanLiteral(b) => Some(if b.value { "true".to_string() } else { "false".to_string() }),
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
        return if n > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
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
        let before = format!("({}) {} (...)", lit_label(&chosen), logical_op_label(l.operator));
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
            //   typeof <BigIntLiteral>    →  "bigint"   (gap-021: no
            //                                            BigIntLiteral
            //                                            variant yet)
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
                _ => None,
            }
        }
        // Skipped: BitNot (int32 coercion), Void (need undefined),
        // Delete (side effects).
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
enum FoldedLiteral {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
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
    }
}

fn literal_label_for_expr(expr: &Expression) -> String {
    lit_label(expr)
}

fn op_label(op: BinaryOperator) -> &'static str {
    use BinaryOperator::*;
    match op {
        Eq => "==", NotEq => "!=", StrictEq => "===", StrictNotEq => "!==",
        Lt => "<", LtEq => "<=", Gt => ">", GtEq => ">=",
        LeftShift => "<<", RightShift => ">>", UnsignedRightShift => ">>>",
        Add => "+", Sub => "-", Mul => "*", Div => "/", Mod => "%", Exp => "**",
        BitOr => "|", BitXor => "^", BitAnd => "&",
        In => "in", InstanceOf => "instanceof",
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
    use coding_adventures_javascript_ast::{
        statement::TaggedStatement, Identifier, SourceType,
    };
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
        (out.program, out.contributions, out.changed, out.stats.nodes_touched)
    }

    /// Build a Program whose body is a single ExpressionStatement
    /// wrapping the given expression.
    fn program_with_expr(expr: Expression, traced: bool) -> Program {
        let p = if traced {
            program()
        } else {
            untraced_program()
        };
        p.with_body(vec![ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: if traced { Some("es.1".to_string()) } else { None },
                expression: expr,
            },
        ))])
    }

    /// Extract the (folded) expression from a Program whose body is a
    /// single ExpressionStatement — for assertion convenience.
    fn extract_expr(prog: &Program) -> &Expression {
        let item = &prog.body[0];
        let ProgramItem::Statement(Statement::Tagged(
            TaggedStatement::ExpressionStatement(es),
        )) = item
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
            let _ = a; let _ = b;
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
        assert!(matches!(extract_expr(&out), Expression::BinaryExpression(_)));
    }

    #[test]
    fn mixed_type_loose_equality_not_folded() {
        // 1 == "1" is true in JS but we don't fold mixed-type
        // comparisons — sound default.
        let expr = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Eq,
            left: Box::new(num(1.0, None)),
            right: Box::new(string("1", None)),
        });
        let (out, _, changed, _) = run_pass(program_with_expr(expr, true));
        assert!(!changed);
        assert!(matches!(extract_expr(&out), Expression::BinaryExpression(_)));
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
                assert!(n.cv.is_none(), "untraced fold produces cv: None; got {:?}", n.cv);
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
                    id: coding_adventures_javascript_ast::BindingTarget::Identifier(
                        Identifier {
                            cv: None,
                            name: "x".to_string(),
                        },
                    ),
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
        let ProgramItem::Declaration(Declaration::VariableDeclaration(v)) = &out.body[0]
        else {
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
        // FixedPoint policy → v0.1.0 pipeline emits the "not yet
        // iterated" note diagnostic. Holds even for the real body.
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"));
    }
}
