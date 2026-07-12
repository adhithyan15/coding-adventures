//! ESTree-compatible statement nodes (CLOC09 Phase 1).
//!
//! 10 variants — 9 leaf statements plus a `Declaration` wrapping arm
//! that lets a declaration appear anywhere a statement does (matching
//! ESTree's lift of `VariableDeclaration` etc. into `Statement`):
//!
//! - [`ExpressionStatement`]
//! - [`BlockStatement`]
//! - [`IfStatement`]
//! - [`WhileStatement`]
//! - [`ForStatement`]
//! - [`ReturnStatement`]
//! - [`BreakStatement`]
//! - [`ContinueStatement`]
//! - [`LabeledStatement`] (Phase 1.x — added in CLOC12.13 to unblock
//!   the DCE port's `testRemoveNoOpLabelledStatement` case)
//! - [`ThrowStatement`] (Phase 1.x — added in CLOC12.14 to unblock
//!   the fold-control-flow port's `testMinimizeIfWithThrow` case)
//! - [`SwitchStatement`] + [`SwitchCase`] (Phase 1.x — added in
//!   CLOC12.33 to unblock the DCE / fold-control-flow ports'
//!   switch-related cases, gap-014)
//! - [`EmptyStatement`]
//! - `Statement::Declaration(Declaration)` — untagged wrap so JSON
//!   collapses to the inner `{"type": "VariableDeclaration", ...}`
//!   shape directly.
//!
//! - [`TryStatement`] + [`CatchClause`] (CLOC19 — `try`/`catch`/`finally`,
//!   added to unblock the whitespace-only fallback on any program using
//!   exception handling)
//! - [`DoWhileStatement`] (CLOC20 — `do body while (test)`, the
//!   test-after-body loop; unblocks the whitespace-only fallback on any
//!   program using a do-while loop)
//!
//! - [`DebuggerStatement`] (CLOC21 — `debugger;`, the breakpoint hook;
//!   unblocks the whitespace-only fallback on any program using it)
//!
//! - [`ForInStatement`] (CLOC22 — `for (left in right) body`, the
//!   property-enumerating loop)
//!
//! - [`ForOfStatement`] (CLOC23 — `for (left of right) body`, the iterator
//!   loop)
//!
//! Phase 2 will add `WithStatement`.

use crate::declaration::{Declaration, VariableDeclaration};
use crate::expression::{Expression, Identifier};
use crate::CvId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// Statement — the tagged union
// ---------------------------------------------------------------------

/// Tagged union of every statement variant. JSON wire format is
/// `{"type": "<Variant>", ...}` per ESTree.
///
/// The `Declaration` variant is **untagged**: a JSON object like
/// `{"type": "VariableDeclaration", ...}` deserializes into
/// `Statement::Declaration(Declaration::VariableDeclaration(...))` so
/// downstream consumers that expect ESTree's flatter shape (where
/// declarations appear as statements) still work.
// The `Declaration` variant is intentionally large; boxing it would ripple
// through the public AST API and every consumer that pattern-matches these
// variants, so we accept the size difference here.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Statement {
    Tagged(TaggedStatement),
    Declaration(Declaration),
}

/// The non-declaration statement variants. Split into a separate inner
/// enum so we can apply `#[serde(tag = "type")]` to it while keeping
/// the outer [`Statement`] `untagged` (which is what lets the
/// [`Statement::Declaration`] wrapping flatten on serialize).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaggedStatement {
    ExpressionStatement(ExpressionStatement),
    BlockStatement(BlockStatement),
    IfStatement(IfStatement),
    WhileStatement(WhileStatement),
    DoWhileStatement(DoWhileStatement),
    ForStatement(ForStatement),
    ForInStatement(ForInStatement),
    ForOfStatement(ForOfStatement),
    ReturnStatement(ReturnStatement),
    BreakStatement(BreakStatement),
    ContinueStatement(ContinueStatement),
    LabeledStatement(LabeledStatement),
    ThrowStatement(ThrowStatement),
    SwitchStatement(SwitchStatement),
    TryStatement(TryStatement),
    EmptyStatement(EmptyStatement),
    DebuggerStatement(DebuggerStatement),
    WithStatement(WithStatement),
}

// Convenience constructors so call sites don't have to write
// `Statement::Tagged(TaggedStatement::IfStatement(...))` every time.
impl Statement {
    pub fn expression_statement(s: ExpressionStatement) -> Self {
        Self::Tagged(TaggedStatement::ExpressionStatement(s))
    }
    pub fn block_statement(s: BlockStatement) -> Self {
        Self::Tagged(TaggedStatement::BlockStatement(s))
    }
    pub fn if_statement(s: IfStatement) -> Self {
        Self::Tagged(TaggedStatement::IfStatement(s))
    }
    pub fn while_statement(s: WhileStatement) -> Self {
        Self::Tagged(TaggedStatement::WhileStatement(s))
    }
    pub fn do_while_statement(s: DoWhileStatement) -> Self {
        Self::Tagged(TaggedStatement::DoWhileStatement(s))
    }
    pub fn for_statement(s: ForStatement) -> Self {
        Self::Tagged(TaggedStatement::ForStatement(s))
    }
    pub fn for_in_statement(s: ForInStatement) -> Self {
        Self::Tagged(TaggedStatement::ForInStatement(s))
    }
    pub fn for_of_statement(s: ForOfStatement) -> Self {
        Self::Tagged(TaggedStatement::ForOfStatement(s))
    }
    pub fn return_statement(s: ReturnStatement) -> Self {
        Self::Tagged(TaggedStatement::ReturnStatement(s))
    }
    pub fn break_statement(s: BreakStatement) -> Self {
        Self::Tagged(TaggedStatement::BreakStatement(s))
    }
    pub fn continue_statement(s: ContinueStatement) -> Self {
        Self::Tagged(TaggedStatement::ContinueStatement(s))
    }
    pub fn labeled_statement(s: LabeledStatement) -> Self {
        Self::Tagged(TaggedStatement::LabeledStatement(s))
    }
    pub fn throw_statement(s: ThrowStatement) -> Self {
        Self::Tagged(TaggedStatement::ThrowStatement(s))
    }
    pub fn switch_statement(s: SwitchStatement) -> Self {
        Self::Tagged(TaggedStatement::SwitchStatement(s))
    }
    pub fn try_statement(s: TryStatement) -> Self {
        Self::Tagged(TaggedStatement::TryStatement(s))
    }
    pub fn empty_statement(s: EmptyStatement) -> Self {
        Self::Tagged(TaggedStatement::EmptyStatement(s))
    }
    pub fn debugger_statement(s: DebuggerStatement) -> Self {
        Self::Tagged(TaggedStatement::DebuggerStatement(s))
    }
    pub fn with_statement(s: WithStatement) -> Self {
        Self::Tagged(TaggedStatement::WithStatement(s))
    }
}

// ---------------------------------------------------------------------
// Variants
// ---------------------------------------------------------------------

/// `expr;` — an expression evaluated for side effects (assignment,
/// function call, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub expression: Expression,
}

/// `{ stmt; stmt; ... }`. Introduces a new block scope for `let` /
/// `const` declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub body: Vec<Statement>,
}

/// `if (test) consequent` or `if (test) consequent else alternate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub test: Expression,
    pub consequent: Box<Statement>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alternate: Option<Box<Statement>>,
}

/// `while (test) body`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhileStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub test: Expression,
    pub body: Box<Statement>,
}

/// `with (object) body` — the legacy scope-injection statement (CLOC12.187).
/// It evaluates `object` and runs `body` with that object pushed onto the scope
/// chain, so a bare name inside `body` may resolve to one of the object's
/// properties rather than a lexical binding.
///
/// **Minifier implication.** That ambiguity is exactly why `with` is forbidden
/// in strict mode and ES modules — and why renaming a local inside a `with`
/// body is unsound (a name you think is a local might be a property of the
/// `with` object at runtime). The optimization passes therefore treat a `with`
/// body **conservatively**: they descend into `object` (an ordinary expression)
/// but leave `body` untouched, which is always correct (it only forgoes
/// optimisations, never changes behaviour). Structurally the node mirrors
/// [`WhileStatement`] — an expression plus a single-statement body — and matches
/// ESTree's `WithStatement` (`object` + `body`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub object: Expression,
    pub body: Box<Statement>,
}

/// `do body while (test)` (CLOC20). The mirror of [`WhileStatement`] with the
/// test moved to *after* the body, which changes the control flow in one
/// important way: the body always runs **at least once** before the test is
/// first evaluated. Field order here follows that execution order (`body`
/// before `test`) and matches ESTree's `DoWhileStatement`.
///
/// The shape is otherwise identical to `while`, so every pass treats it the
/// same: recurse into `test` (an expression) and `body` (a statement). Like
/// `while`, a `do`-`while` is **not** a terminator — control can fall out of
/// the loop, so statements after it stay reachable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoWhileStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub body: Box<Statement>,
    pub test: Expression,
}

/// `for (init; test; update) body`. All three head clauses are
/// optional — `for (;;) {}` is legal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub init: Option<ForInit>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub test: Option<Expression>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub update: Option<Expression>,
    pub body: Box<Statement>,
}

/// The `init` clause of a [`ForStatement`] — either a fresh
/// declaration (`for (let i = 0; ...)` ) or an expression
/// (`for (i = 0; ...)`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ForInit {
    VariableDeclaration(VariableDeclaration),
    Expression(Expression),
}

/// `for (left in right) body` (CLOC22). The enumerating loop: `right` is
/// evaluated once, and `body` runs with `left` bound to each enumerable
/// property key in turn.
///
/// `left` reuses [`ForInit`]:
/// - `ForInit::VariableDeclaration` for `for (var k in o)` / `for (let k in o)`
///   / `for (const k in o)` — a single-declarator binding with no initializer.
/// - `ForInit::Expression` for `for (k in o)` / `for (o.p in src)` — an
///   existing assignment target.
///
/// (Destructuring left-hand sides are not represented; the bridge declines
/// them, falling back to whitespace-only, which is sound.)
///
/// Like the other loops, a `for`-`in` is NOT a terminator — the loop body may
/// run zero times (an object with no enumerable keys), so control can fall
/// through and statements after it stay reachable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForInStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub left: ForInit,
    pub right: Expression,
    pub body: Box<Statement>,
}

/// `for (left of right) body` (CLOC23). The iterator loop: `right` is evaluated
/// to an iterable, and `body` runs with `left` bound to each yielded value in
/// turn. Structurally identical to [`ForInStatement`] — only the keyword (`of`
/// vs `in`) and the iteration protocol differ — so it reuses [`ForInit`] for the
/// left in exactly the same way:
/// - `ForInit::VariableDeclaration` for `for (var/let/const v of it)`.
/// - `ForInit::Expression` for `for (v of it)` / `for (o.p of it)`.
///
/// Destructuring left-hand sides and `using` bindings are not represented; the
/// bridge declines them (sound whitespace-only fallback). `for await (… of …)`
/// is a distinct grammar production and is likewise not handled here.
///
/// Like the other loops, a `for`-`of` is NOT a terminator — the iterable may be
/// empty, so the body can run zero times and control can fall through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForOfStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub left: ForInit,
    pub right: Expression,
    pub body: Box<Statement>,
}

/// `return;` or `return expr;`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub argument: Option<Expression>,
}

/// `break;` or `break label;`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<Identifier>,
}

/// `continue;` or `continue label;`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<Identifier>,
}

/// `label: stmt` — attaches a name to any statement so that
/// `break label;` and `continue label;` can target it. Almost always
/// wraps a loop or block; the ECMAScript grammar allows it on any
/// statement.
///
/// Added in Phase 1.x (CLOC12.13) to unblock the DCE port's
/// `testRemoveNoOpLabelledStatement` case (gap-009). The actual
/// "collapse `a: break a;` to empty" optimisation is a follow-up;
/// modelling the node first is the structural prerequisite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub label: Identifier,
    pub body: Box<Statement>,
}

/// `throw expr;` — raises a runtime exception. Per ECMAScript
/// §13.14, `throw;` (with no argument) is a SyntaxError, so the
/// `argument` field is non-optional: a `ThrowStatement` always
/// carries a value to throw.
///
/// Added in Phase 1.x (CLOC12.14) to unblock the
/// fold-control-flow port's `testMinimizeIfWithThrow` case (gap-020).
/// The optimisation that triggers off this node — rewriting
/// `if (x) foo(); else throw e;` into `if (!x) throw e; foo();` —
/// is a separate follow-up; modelling the node first is the
/// structural prerequisite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThrowStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub argument: Expression,
}

/// `switch (discriminant) { case test1: ...; case test2: ...; default: ...; }`.
///
/// Per ECMAScript §13.12:
///
/// - The `discriminant` is evaluated once.
/// - Each `case` is compared against it with strict equality (`===`).
/// - Execution falls through from one case to the next unless an
///   explicit `break` (or `return`/`throw`) ends the case body.
/// - At most one `default` clause is allowed. Its position in the
///   `cases` array matters for fallthrough.
///
/// Added in Phase 1.x (CLOC12.33) to unblock the DCE port's
/// `testRemoveSwitch*` cases and the fold-control-flow port's
/// `testRemoveEmptySwitch` case (gap-014). The optimisation passes
/// that consume this node — eliminating empty switches, folding
/// switches with a constant discriminant down to the matching
/// case body — are a separate follow-up; modelling the node first
/// is the structural prerequisite.
///
/// # ESTree compatibility
///
/// ESTree shape `{ type: "SwitchStatement", discriminant, cases }`,
/// where each `cases[i]` is `{ type: "SwitchCase", test, consequent }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub discriminant: Expression,
    pub cases: Vec<SwitchCase>,
}

/// One arm of a [`SwitchStatement`].
///
/// - `test: Some(expr)` — the `case expr:` form. Matched against
///   the discriminant with strict equality (`===`).
/// - `test: None` — the `default:` clause. Per the spec, a switch
///   may have at most one `default` (semantically; we don't
///   enforce this at the AST level — it's the parser's job).
/// - `consequent` — the list of statements making up the body of
///   the case. Fallthrough to the next case happens implicitly if
///   none of these terminate with a `break`/`return`/`throw`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "SwitchCase", rename_all = "camelCase")]
pub struct SwitchCase {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// `None` means the `default:` clause. `Some(expr)` is the
    /// `case expr:` form.
    pub test: Option<Expression>,
    pub consequent: Vec<Statement>,
}

/// `try { … } catch (e) { … } finally { … }` (CLOC19). ESTree shape:
/// `block` is the protected block, `handler` is an optional [`CatchClause`],
/// and `finalizer` is the optional `finally` block. The grammar guarantees at
/// least one of `handler` / `finalizer` is present (a bare `try { }` is a
/// SyntaxError), but the AST does not enforce that.
///
/// Control flow: the `block` runs; if it throws and a `handler` is present, the
/// thrown value binds to the handler's `param` and the handler `body` runs; the
/// `finalizer` (if any) always runs last, on every exit path. A `try` is
/// therefore NOT an unconditional terminator — it can catch and continue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// NB: no self `#[serde(tag = "type")]` here. `TryStatement` is a variant of the
// internally-tagged `TaggedStatement` enum, which injects `"type":
// "TryStatement"` from the variant name. Adding a second `type` tag on the
// struct itself double-tags it and breaks deserialization back into
// `Statement` (the untagged outer enum) — every sibling statement struct
// (IfStatement, SwitchStatement, …) likewise carries only `rename_all`.
#[serde(rename_all = "camelCase")]
pub struct TryStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub block: BlockStatement,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub handler: Option<CatchClause>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub finalizer: Option<BlockStatement>,
}

/// The `catch (param) { body }` arm of a [`TryStatement`].
///
/// - `param: Some(id)` — the bound identifier the thrown value is assigned to.
///   (Destructuring catch params are not represented yet — the bridge declines
///   them, falling back to whitespace-only, which is sound.)
/// - `param: None` — the optional-catch-binding form `catch { … }` (ES2019).
///
/// The `param` is a binding scoped to `body` (and nowhere else). Passes that
/// rename or remove bindings MUST treat it as such — see the per-pass handling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "CatchClause", rename_all = "camelCase")]
pub struct CatchClause {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub param: Option<Identifier>,
    pub body: BlockStatement,
}

/// A lone semicolon `;`. Rare in user code but legal everywhere a
/// statement can appear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// `debugger;` (CLOC21). A breakpoint hook: it pauses execution if a debugger
/// is attached and is otherwise a no-op. Like [`EmptyStatement`] it carries no
/// children. Making it representable lets the typed pipeline optimize the rest
/// of a program that contains a `debugger` statement (previously any such
/// program fell back to WHITESPACE_ONLY). The node itself just preserves the
/// statement; the `closure-pass-dce` pass strips `debugger` statements from
/// statement lists at SIMPLE/ADVANCED (CLOC24, matching the upstream Closure
/// Compiler), while WHITESPACE_ONLY — which never runs that pass — keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

#[cfg(test)]
mod tests {
    //! Round-trip tests for every Phase 1 statement variant.
    use super::*;
    use crate::declaration::{VarKind, VariableDeclarator};
    use crate::expression::{BinaryExpression, BinaryOperator, Identifier, NumericLiteral};
    use crate::declaration::BindingTarget;

    fn roundtrip(s: Statement) -> Statement {
        let json = serde_json::to_string(&s).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    fn type_tag(s: &Statement) -> String {
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(s).unwrap()).unwrap();
        v["type"].as_str().unwrap().to_string()
    }

    fn lit(n: f64) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: n,
            raw: n.to_string(),
        })
    }

    #[test]
    fn expression_statement_roundtrips() {
        let s = Statement::expression_statement(ExpressionStatement {
            cv: Some("es.1".to_string()),
            expression: lit(1.0),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ExpressionStatement");
    }

    #[test]
    fn block_statement_traced_and_untraced() {
        let traced = Statement::block_statement(BlockStatement {
            cv: Some("b.1".to_string()),
            body: vec![Statement::empty_statement(EmptyStatement {
                cv: Some("e.1".to_string()),
            })],
        });
        let untraced = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::empty_statement(EmptyStatement { cv: None })],
        });
        assert_eq!(traced.clone(), roundtrip(traced.clone()));
        assert_eq!(untraced.clone(), roundtrip(untraced.clone()));
        assert_eq!(type_tag(&traced), "BlockStatement");
    }

    #[test]
    fn if_statement_with_alternate() {
        let s = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Lt,
                left: Box::new(lit(1.0)),
                right: Box::new(lit(2.0)),
            }),
            consequent: Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            })),
            alternate: Some(Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            }))),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "IfStatement");
    }

    #[test]
    fn if_statement_no_alternate_omits_field() {
        let s = Statement::if_statement(IfStatement {
            cv: None,
            test: lit(0.0),
            consequent: Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            })),
            alternate: None,
        });
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(!json.contains("\"alternate\""), "got {}", json);
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn while_statement_roundtrips() {
        let s = Statement::while_statement(WhileStatement {
            cv: None,
            test: lit(1.0),
            body: Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "WhileStatement");
    }

    #[test]
    fn with_statement_roundtrips() {
        // with (o) ; — an object expression plus a single-statement body
        // (CLOC12.187). Serialises as ESTree `WithStatement` (`object` + `body`).
        let s = Statement::with_statement(WithStatement {
            cv: None,
            object: Expression::Identifier(Identifier { cv: None, name: "o".to_string() }),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "WithStatement");
    }

    #[test]
    fn do_while_statement_roundtrips() {
        // do {} while (1) — body before test, mirroring execution order.
        let s = Statement::do_while_statement(DoWhileStatement {
            cv: Some("dw.1".to_string()),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
            test: lit(1.0),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "DoWhileStatement");
    }

    #[test]
    fn for_statement_with_declaration_init() {
        // for (let i = 0; i < 10; i = i + 1) {}
        let s = Statement::for_statement(ForStatement {
            cv: None,
            init: Some(ForInit::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind: VarKind::Let,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "i".to_string(),
                    }),
                    init: Some(lit(0.0)),
                }],
            })),
            test: Some(Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Lt,
                left: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "i".to_string(),
                })),
                right: Box::new(lit(10.0)),
            })),
            update: None,
            body: Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ForStatement");
    }

    #[test]
    fn for_in_statement_with_declaration_left_roundtrips() {
        // for (var k in obj) {}
        let s = Statement::for_in_statement(ForInStatement {
            cv: Some("forin.1".to_string()),
            left: ForInit::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind: VarKind::Var,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "k".to_string(),
                    }),
                    init: None,
                }],
            }),
            right: Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            }),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ForInStatement");
    }

    #[test]
    fn for_in_statement_with_expression_left_roundtrips() {
        // for (k in obj) {}  — an existing assignment target as the left.
        let s = Statement::for_in_statement(ForInStatement {
            cv: None,
            left: ForInit::Expression(Expression::Identifier(Identifier {
                cv: None,
                name: "k".to_string(),
            })),
            right: Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            }),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ForInStatement");
    }

    #[test]
    fn for_of_statement_with_declaration_left_roundtrips() {
        // for (const v of it) {}
        let s = Statement::for_of_statement(ForOfStatement {
            cv: Some("forof.1".to_string()),
            left: ForInit::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind: VarKind::Const,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "v".to_string(),
                    }),
                    init: None,
                }],
            }),
            right: Expression::Identifier(Identifier {
                cv: None,
                name: "it".to_string(),
            }),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ForOfStatement");
    }

    #[test]
    fn for_of_statement_with_expression_left_roundtrips() {
        // for (v of it) {}  — an existing assignment target as the left.
        let s = Statement::for_of_statement(ForOfStatement {
            cv: None,
            left: ForInit::Expression(Expression::Identifier(Identifier {
                cv: None,
                name: "v".to_string(),
            })),
            right: Expression::Identifier(Identifier {
                cv: None,
                name: "it".to_string(),
            }),
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ForOfStatement");
    }

    #[test]
    fn for_statement_all_clauses_empty() {
        // for (;;) {}
        let s = Statement::for_statement(ForStatement {
            cv: None,
            init: None,
            test: None,
            update: None,
            body: Box::new(Statement::empty_statement(EmptyStatement {
                cv: None,
            })),
        });
        let json = serde_json::to_string(&s).expect("serialize");
        // Empty init/test/update are omitted from the JSON entirely.
        assert!(!json.contains("\"init\""));
        assert!(!json.contains("\"test\""));
        assert!(!json.contains("\"update\""));
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn return_statement_with_argument() {
        let s = Statement::return_statement(ReturnStatement {
            cv: Some("r.1".to_string()),
            argument: Some(lit(42.0)),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ReturnStatement");
    }

    #[test]
    fn return_statement_no_argument() {
        let s = Statement::return_statement(ReturnStatement {
            cv: None,
            argument: None,
        });
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(!json.contains("\"argument\""), "got {}", json);
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn break_statement_labeled_vs_bare() {
        let bare = Statement::break_statement(BreakStatement {
            cv: None,
            label: None,
        });
        let labeled = Statement::break_statement(BreakStatement {
            cv: None,
            label: Some(Identifier {
                cv: None,
                name: "outer".to_string(),
            }),
        });
        assert_ne!(bare, labeled);
        assert_eq!(bare.clone(), roundtrip(bare.clone()));
        assert_eq!(labeled.clone(), roundtrip(labeled.clone()));
        assert_eq!(type_tag(&bare), "BreakStatement");
    }

    #[test]
    fn continue_statement_labeled_vs_bare() {
        let bare = Statement::continue_statement(ContinueStatement {
            cv: None,
            label: None,
        });
        let labeled = Statement::continue_statement(ContinueStatement {
            cv: None,
            label: Some(Identifier {
                cv: None,
                name: "inner".to_string(),
            }),
        });
        assert_eq!(bare.clone(), roundtrip(bare.clone()));
        assert_eq!(labeled.clone(), roundtrip(labeled.clone()));
        assert_eq!(type_tag(&bare), "ContinueStatement");
    }

    #[test]
    fn labeled_statement_roundtrips() {
        // a: break a;
        let s = Statement::labeled_statement(LabeledStatement {
            cv: Some("lbl.1".to_string()),
            label: Identifier {
                cv: None,
                name: "a".to_string(),
            },
            body: Box::new(Statement::break_statement(BreakStatement {
                cv: None,
                label: Some(Identifier {
                    cv: None,
                    name: "a".to_string(),
                }),
            })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "LabeledStatement");
    }

    #[test]
    fn labeled_statement_with_block_body() {
        // outer: { ; }
        let s = Statement::labeled_statement(LabeledStatement {
            cv: None,
            label: Identifier {
                cv: None,
                name: "outer".to_string(),
            },
            body: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![Statement::empty_statement(EmptyStatement { cv: None })],
            })),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "LabeledStatement");
    }

    #[test]
    fn throw_statement_with_numeric_literal() {
        // throw 1;
        let s = Statement::throw_statement(ThrowStatement {
            cv: Some("th.1".to_string()),
            argument: lit(1.0),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "ThrowStatement");
    }

    #[test]
    fn throw_statement_with_identifier_no_cv() {
        // throw e;  (untraced)
        let s = Statement::throw_statement(ThrowStatement {
            cv: None,
            argument: Expression::Identifier(Identifier {
                cv: None,
                name: "e".to_string(),
            }),
        });
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn empty_statement_roundtrips() {
        let s = Statement::empty_statement(EmptyStatement {
            cv: Some("empty.1".to_string()),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "EmptyStatement");
    }

    #[test]
    fn debugger_statement_roundtrips() {
        let s = Statement::debugger_statement(DebuggerStatement {
            cv: Some("dbg.1".to_string()),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "DebuggerStatement");
    }

    #[test]
    fn switch_statement_empty_cases_roundtrips() {
        let s = Statement::switch_statement(SwitchStatement {
            cv: Some("sw.1".to_string()),
            discriminant: lit(1.0),
            cases: vec![],
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "SwitchStatement");
    }

    #[test]
    fn switch_statement_with_case_and_default_roundtrips() {
        // switch (1) { case 2: ; default: ; }
        let s = Statement::switch_statement(SwitchStatement {
            cv: Some("sw.2".to_string()),
            discriminant: lit(1.0),
            cases: vec![
                SwitchCase {
                    cv: Some("sc.case.1".to_string()),
                    test: Some(lit(2.0)),
                    consequent: vec![Statement::empty_statement(EmptyStatement {
                        cv: None,
                    })],
                },
                SwitchCase {
                    cv: Some("sc.default".to_string()),
                    test: None,
                    consequent: vec![Statement::empty_statement(EmptyStatement {
                        cv: None,
                    })],
                },
            ],
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "SwitchStatement");
        // Verify inner case type tags too — they should be
        // "SwitchCase" per the ESTree wire format.
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let cases = json["cases"].as_array().unwrap();
        assert_eq!(cases[0]["type"], "SwitchCase");
        assert_eq!(cases[1]["type"], "SwitchCase");
        // The default case has `test: null` (None serialises that way).
        assert!(cases[1]["test"].is_null());
    }

    #[test]
    fn switch_statement_untraced_omits_cv() {
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: lit(0.0),
            cases: vec![],
        });
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(s.clone(), roundtrip(s));
    }

    // ---- try / catch / finally (CLOC19) -----------------------

    /// Build `try { ; } catch (e) { ; } finally { ; }` with the catch
    /// param `e` and a trivial body in each block.
    fn full_try() -> TryStatement {
        let one = || BlockStatement {
            cv: None,
            body: vec![Statement::empty_statement(EmptyStatement { cv: None })],
        };
        TryStatement {
            cv: Some("try.1".to_string()),
            block: one(),
            handler: Some(CatchClause {
                cv: Some("catch.1".to_string()),
                param: Some(Identifier {
                    cv: None,
                    name: "e".to_string(),
                }),
                body: one(),
            }),
            finalizer: Some(one()),
        }
    }

    #[test]
    fn try_statement_full_roundtrips() {
        let s = Statement::try_statement(full_try());
        assert_eq!(type_tag(&s), "TryStatement");
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn try_statement_optional_catch_binding_roundtrips() {
        // `catch { … }` — handler present, param None.
        let s = Statement::try_statement(TryStatement {
            cv: None,
            block: BlockStatement {
                cv: None,
                body: vec![],
            },
            handler: Some(CatchClause {
                cv: None,
                param: None,
                body: BlockStatement {
                    cv: None,
                    body: vec![],
                },
            }),
            finalizer: None,
        });
        let json = serde_json::to_string(&s).unwrap();
        // An absent param must not serialize a `param` key (skip-if-none).
        assert!(
            !json.contains("\"param\""),
            "optional-catch-binding should omit the param key; got {json}",
        );
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn try_finally_without_catch_roundtrips() {
        // `try { } finally { }` — handler omitted entirely.
        let s = Statement::try_statement(TryStatement {
            cv: None,
            block: BlockStatement {
                cv: None,
                body: vec![],
            },
            handler: None,
            finalizer: Some(BlockStatement {
                cv: None,
                body: vec![],
            }),
        });
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            !json.contains("\"handler\""),
            "absent handler should omit the key; got {json}",
        );
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn declaration_wrapping_collapses_to_inner_type_tag() {
        // Statement::Declaration(Declaration::VariableDeclaration(...))
        // should serialize as {"type": "VariableDeclaration", ...} —
        // NOT {"type": "Declaration", "..." } — because Statement's
        // Declaration arm is untagged.
        let s = Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Const,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    }),
                    init: Some(lit(1.0)),
                }],
            },
        ));
        assert_eq!(type_tag(&s), "VariableDeclaration");
        assert_eq!(s.clone(), roundtrip(s));
    }
}
