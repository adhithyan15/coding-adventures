//! ESTree-compatible expression nodes (CLOC09 Phase 1).
//!
//! 15 variants:
//! - Leaves: [`Identifier`], [`NumericLiteral`], [`StringLiteral`],
//!   [`BooleanLiteral`], [`NullLiteral`], [`BigIntLiteral`] (Phase 1.x —
//!   added in CLOC12.15), [`UndefinedLiteral`] (Phase 1.x — added in
//!   CLOC12.16).
//! - Operators: [`BinaryExpression`], [`LogicalExpression`],
//!   [`UnaryExpression`], [`AssignmentExpression`],
//!   [`ConditionalExpression`].
//! - Application: [`CallExpression`], [`MemberExpression`].
//! - Composites: [`ArrayExpression`], [`ObjectExpression`].
//! - Callables: [`FunctionExpression`] (Phase 1.x — added in CLOC12.149;
//!   the expression sibling of [`crate::FunctionDeclaration`], e.g.
//!   `var f = function () {}`, IIFEs, function-valued properties).
//!
//! Every struct carries `cv: Option<CvId>` first per the CLOC09
//! amendment. Operator enums serialize to ESTree-canonical operator
//! strings (`"=="`, `"==="`, `"&&"`, etc.) via per-variant
//! `#[serde(rename = "...")]`.

use crate::declaration::FunctionParam;
use crate::statement::BlockStatement;
use crate::CvId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// Expression — the tagged union
// ---------------------------------------------------------------------

/// Tagged union of every expression variant. JSON wire format is
/// `{"type": "<Variant>", ...}` per ESTree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Expression {
    Identifier(Identifier),
    NumericLiteral(NumericLiteral),
    StringLiteral(StringLiteral),
    BooleanLiteral(BooleanLiteral),
    NullLiteral(NullLiteral),
    BigIntLiteral(BigIntLiteral),
    UndefinedLiteral(UndefinedLiteral),
    BinaryExpression(BinaryExpression),
    LogicalExpression(LogicalExpression),
    UnaryExpression(UnaryExpression),
    AssignmentExpression(AssignmentExpression),
    ConditionalExpression(ConditionalExpression),
    CallExpression(CallExpression),
    MemberExpression(MemberExpression),
    ArrayExpression(ArrayExpression),
    ObjectExpression(ObjectExpression),
    FunctionExpression(FunctionExpression),
}

// ---------------------------------------------------------------------
// Leaves
// ---------------------------------------------------------------------

/// A name reference: variable, parameter, function name, property key
/// (when not computed). ESTree `Identifier`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub name: String,
}

/// A numeric literal — `42`, `3.14`, `0xff`. ESTree splits its single
/// `Literal` node into typed variants for us; the underlying numeric
/// value is always an IEEE 754 double.
///
/// `value` is the parsed numeric value; `raw` preserves the original
/// source text (so `0xFF` round-trips as `0xFF` rather than `255`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumericLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: f64,
    pub raw: String,
}

/// A string literal — `"hello"`, `'world'`. `value` is the unescaped
/// string contents; `raw` preserves the original source representation
/// including quotes and escape sequences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: String,
    pub raw: String,
}

/// A boolean literal — `true` / `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: bool,
}

/// The `null` literal. Carries no payload beyond its `cv`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NullLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// The `undefined` literal.  Carries no payload beyond its `cv`.
///
/// **Note: `undefined` is technically an identifier in ECMAScript,
/// not a reserved word — `var undefined = 1;` is legal in
/// non-strict mode and shadows the global.** ESTree historically
/// modelled it as `Identifier { name: "undefined" }`.  We follow
/// the modern *typed* variant approach: a dedicated leaf node so
/// passes can pattern-match on it without first checking the
/// identifier name.  The emitter renders it as `void 0` (a fixed
/// expression that always evaluates to the genuine undefined value
/// regardless of any shadow binding in scope — upstream Closure's
/// safe rendering).
///
/// Added in Phase 1.x (CLOC12.16) to close gap-001.  Closes the
/// final hole in CLOC12.09's typeof-literal fold table — passes
/// can now fold `typeof <UndefinedLiteral>` to `"undefined"`
/// without needing the surrounding context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndefinedLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// A BigInt literal — `123n`, `0n`, `0x1fn`. ESTree models this as a
/// separate node from [`NumericLiteral`] because bigints can exceed
/// the `f64` range — the `value` field is therefore a `String` (per
/// ESTree's JSON-safety convention) holding the decimal expansion of
/// the bigint, while `raw` keeps the original source representation
/// including the trailing `n` suffix.
///
/// Added in Phase 1.x (CLOC12.15) to model the bigint primitive
/// (gap-021). No optimisation rides on this yet — passes treat a
/// `BigIntLiteral` as already-folded the same way `NumericLiteral`
/// is treated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigIntLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// Decimal expansion of the bigint as a string. E.g. `"123"`,
    /// `"0"`, `"31"` (for `0x1fn`). Always non-negative — the `-` in
    /// `-123n` is a [`UnaryExpression`] over a `BigIntLiteral`, never
    /// part of the literal itself.
    pub value: String,
    /// Original source representation including the trailing `n`,
    /// e.g. `"123n"`, `"0x1fn"`. Preserved so that hex-typed bigints
    /// round-trip through the emitter without losing their radix.
    pub raw: String,
}

// ---------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------

/// `a op b` where `op` is non-short-circuiting. Note that ESTree splits
/// short-circuiting operators (`&&`, `||`, `??`) into [`LogicalExpression`]
/// — the semantic distinction matters for evaluation order and for
/// passes that reason about side effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

/// Binary operators per ECMAScript. Serializes as the ESTree-canonical
/// source-text string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    #[serde(rename = "==")] Eq,
    #[serde(rename = "!=")] NotEq,
    #[serde(rename = "===")] StrictEq,
    #[serde(rename = "!==")] StrictNotEq,
    #[serde(rename = "<")] Lt,
    #[serde(rename = "<=")] LtEq,
    #[serde(rename = ">")] Gt,
    #[serde(rename = ">=")] GtEq,
    #[serde(rename = "<<")] LeftShift,
    #[serde(rename = ">>")] RightShift,
    #[serde(rename = ">>>")] UnsignedRightShift,
    #[serde(rename = "+")] Add,
    #[serde(rename = "-")] Sub,
    #[serde(rename = "*")] Mul,
    #[serde(rename = "/")] Div,
    #[serde(rename = "%")] Mod,
    #[serde(rename = "**")] Exp,
    #[serde(rename = "|")] BitOr,
    #[serde(rename = "^")] BitXor,
    #[serde(rename = "&")] BitAnd,
    #[serde(rename = "in")] In,
    #[serde(rename = "instanceof")] InstanceOf,
}

/// `a && b`, `a || b`, `a ?? b`. Split from [`BinaryExpression`] because
/// these short-circuit — `right` is only evaluated when the truthiness
/// of `left` requires it. Optimization passes that reason about side
/// effects need this distinction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: LogicalOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "&&")] And,
    #[serde(rename = "||")] Or,
    #[serde(rename = "??")] NullishCoalescing,
}

/// Prefix unary: `-x`, `+x`, `!x`, `~x`, `typeof x`, `void x`, `delete x`.
/// `prefix: true` in v1; ESTree's `UpdateExpression` (`++` / `--`) is
/// deferred to Phase 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnaryExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: UnaryOperator,
    pub prefix: bool,
    pub argument: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    #[serde(rename = "-")] Negate,
    #[serde(rename = "+")] Plus,
    #[serde(rename = "!")] Not,
    #[serde(rename = "~")] BitNot,
    #[serde(rename = "typeof")] TypeOf,
    #[serde(rename = "void")] Void,
    #[serde(rename = "delete")] Delete,
}

/// `x = y`, `x += y`, etc. The target is an [`AssignmentTarget`]
/// (identifier or member expression in Phase 1; destructuring patterns
/// land in Phase 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: AssignmentOperator,
    pub left: AssignmentTarget,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentOperator {
    #[serde(rename = "=")] Eq,
    #[serde(rename = "+=")] AddEq,
    #[serde(rename = "-=")] SubEq,
    #[serde(rename = "*=")] MulEq,
    #[serde(rename = "/=")] DivEq,
    #[serde(rename = "%=")] ModEq,
    #[serde(rename = "**=")] ExpEq,
    #[serde(rename = "<<=")] LeftShiftEq,
    #[serde(rename = ">>=")] RightShiftEq,
    #[serde(rename = ">>>=")] UnsignedRightShiftEq,
    #[serde(rename = "|=")] BitOrEq,
    #[serde(rename = "^=")] BitXorEq,
    #[serde(rename = "&=")] BitAndEq,
    // Phase 5: LogicalAndEq (&&=), LogicalOrEq (||=),
    // NullishCoalescingEq (??=) once we have ES2021 in scope.
}

/// The left-hand-side of an `AssignmentExpression`. Identifier or member
/// expression in Phase 1; destructuring patterns are Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssignmentTarget {
    Identifier(Identifier),
    MemberExpression(Box<MemberExpression>),
}

/// `test ? consequent : alternate`. The ternary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub test: Box<Expression>,
    pub consequent: Box<Expression>,
    pub alternate: Box<Expression>,
}

// ---------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------

/// `f(x, y, z)`. `callee` is an expression — typically an identifier or
/// a member expression but in principle any expression that evaluates
/// to a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

/// `obj.prop` or `obj[key]`. `computed = false` ↔ `obj.prop` (the
/// `property` is conventionally an [`Expression::Identifier`]);
/// `computed = true` ↔ `obj[key]` (the `property` is any expression).
///
/// The distinction between dot- and bracket-access matters: with
/// `Symbol` keys, `obj[sym]` and `obj.sym` reach different properties.
/// Per the ESTree spec the property is always typed `Expression` (the
/// JSON wire format makes them indistinguishable by shape — both write
/// `{"object": ..., "property": ..., "computed": bool}`). When
/// `computed = false`, the parser is required to emit an `Identifier`
/// as the `property`; tools that walk the tree can assert this if
/// they care.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub object: Box<Expression>,
    pub property: Box<Expression>,
    pub computed: bool,
}

/// Deprecated convenience alias kept for the brief window between
/// spec write-up and implementation; downstream code should use
/// `Box<Expression>` directly. Will be removed in Phase 1.1.
#[deprecated = "Use Box<Expression> directly on MemberExpression.property"]
pub type MemberProperty = Box<Expression>;

// ---------------------------------------------------------------------
// Composites
// ---------------------------------------------------------------------

/// `[a, b, c]`. `None` in the elements vector represents an *elision*
/// (a hole) — `[1, , 3]` produces `Vec[Some(1), None, Some(3)]`.
/// Elisions are distinct from `undefined` (you can check via `in`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrayExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub elements: Vec<Option<Expression>>,
}

/// `{ key1: value1, key2: value2 }`. Property insertion order is
/// preserved per ES2015+ — `Object.keys` is observable. Phase 1
/// supports `Init` properties (`{k: v}`), getters, setters, shorthand
/// `{k}`, and methods `{k() {}}`. Spread (`{...x}`) is Phase 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub properties: Vec<Property>,
}

/// One entry of an [`ObjectExpression`]. ESTree calls this `Property`;
/// when serialized inside an object expression it carries a
/// `"type": "Property"` tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "Property", rename_all = "camelCase")]
pub struct Property {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub kind: PropertyKind,
    pub key: PropertyKey,
    pub value: Box<Expression>,
    pub computed: bool,
    pub shorthand: bool,
    pub method: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}

/// The key side of a [`Property`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyKey {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
    NumericLiteral(NumericLiteral),
    /// When the property is `[expr]: value` (computed = true).
    Expression(Box<Expression>),
}

// ---------------------------------------------------------------------
// FunctionExpression — a function used in value position
// ---------------------------------------------------------------------

/// `function (p1, p2) { body }` or the *named* form
/// `function f(p1, p2) { body }`, appearing where an **expression** is
/// expected — the right side of an assignment (`var g = function () {}`),
/// an argument (`arr.map(function (x) { return x; })`), a property value
/// (`{ run: function () {} }`), or the callee of an IIFE
/// (`(function () {})()`).
///
/// # How it differs from [`crate::FunctionDeclaration`]
///
/// A declaration *binds a name in the enclosing scope* and is a
/// statement; an expression *produces a function value* and its name
/// (if any) is visible **only inside its own body** (so a named
/// function expression can recurse by its own name without leaking that
/// name outward). That single semantic difference is exactly why `id`
/// here is `Option<Identifier>` — anonymous function expressions are the
/// common case — whereas a declaration's `id` is mandatory.
///
/// ```text
///   declaration:  function f(){}      f is bound in the outer scope
///   expression:   (function f(){})    f is bound ONLY inside the body
///   expression:   (function (){})     anonymous — no name at all
/// ```
///
/// `params`, `body`, `generator`, and `is_async` carry the identical
/// meaning as on [`crate::FunctionDeclaration`], and the two share the
/// [`FunctionParam`] and [`BlockStatement`] types so passes that walk a
/// function body do not care which form produced it.
///
/// Added in Phase 1.x (CLOC09 §"future Phase 1.x FunctionExpression").
/// Arrow functions, methods, getters/setters, and class expressions
/// remain Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// `None` for an anonymous `function () {}`; `Some` for a named
    /// `function f() {}` expression (the name is body-local — see the
    /// type-level docs).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<Identifier>,
    pub params: Vec<FunctionParam>,
    pub body: BlockStatement,
    pub generator: bool,
    #[serde(rename = "async")]
    pub is_async: bool,
}

#[cfg(test)]
mod tests {
    //! Round-trip tests for every Phase 1 expression variant. Each
    //! variant gets at least one test for each tracing mode (traced /
    //! untraced) — proving the wire format works both ways per the
    //! CLOC09 amendment.
    //!
    //! The structural property we assert is: `from_str(to_string(x)) == x`.
    //! The `"type"` tag is asserted in a separate test per variant when
    //! the variant adds a new tag name.
    use super::*;

    fn roundtrip(expr: Expression) -> Expression {
        let json = serde_json::to_string(&expr).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    fn type_tag(expr: &Expression) -> String {
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(expr).unwrap()).unwrap();
        v["type"].as_str().unwrap().to_string()
    }

    #[test]
    fn identifier_roundtrips_traced_and_untraced() {
        let traced = Expression::Identifier(Identifier {
            cv: Some("id.1".to_string()),
            name: "x".to_string(),
        });
        let untraced = Expression::Identifier(Identifier {
            cv: None,
            name: "x".to_string(),
        });
        assert_eq!(traced.clone(), roundtrip(traced.clone()));
        assert_eq!(untraced.clone(), roundtrip(untraced.clone()));
        assert_eq!(type_tag(&traced), "Identifier");
    }

    #[test]
    fn numeric_literal_roundtrips() {
        let n = Expression::NumericLiteral(NumericLiteral {
            cv: Some("n.1".to_string()),
            value: 42.0,
            raw: "42".to_string(),
        });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "NumericLiteral");
    }

    #[test]
    fn numeric_literal_untraced_omits_cv_in_json() {
        let n = Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: 1.5,
            raw: "1.5".to_string(),
        });
        let json = serde_json::to_string(&n).expect("serialize");
        assert!(!json.contains("\"cv\""), "untraced should omit cv; got {}", json);
        assert_eq!(n.clone(), roundtrip(n));
    }

    #[test]
    fn string_literal_roundtrips() {
        let s = Expression::StringLiteral(StringLiteral {
            cv: Some("s.1".to_string()),
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "StringLiteral");
    }

    #[test]
    fn boolean_literal_roundtrips() {
        let b = Expression::BooleanLiteral(BooleanLiteral {
            cv: Some("b.1".to_string()),
            value: true,
        });
        assert_eq!(b.clone(), roundtrip(b.clone()));
        assert_eq!(type_tag(&b), "BooleanLiteral");
    }

    #[test]
    fn null_literal_roundtrips() {
        let n = Expression::NullLiteral(NullLiteral {
            cv: Some("null.1".to_string()),
        });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "NullLiteral");
    }

    #[test]
    fn undefined_literal_roundtrips_traced() {
        let u = Expression::UndefinedLiteral(UndefinedLiteral {
            cv: Some("u.1".to_string()),
        });
        assert_eq!(u.clone(), roundtrip(u.clone()));
        assert_eq!(type_tag(&u), "UndefinedLiteral");
    }

    #[test]
    fn undefined_literal_untraced_omits_cv() {
        let u = Expression::UndefinedLiteral(UndefinedLiteral { cv: None });
        let json = serde_json::to_string(&u).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(u.clone(), roundtrip(u));
    }

    #[test]
    fn bigint_literal_decimal_roundtrips() {
        // 123n  — decimal-source bigint
        let b = Expression::BigIntLiteral(BigIntLiteral {
            cv: Some("bi.1".to_string()),
            value: "123".to_string(),
            raw: "123n".to_string(),
        });
        assert_eq!(b.clone(), roundtrip(b.clone()));
        assert_eq!(type_tag(&b), "BigIntLiteral");
    }

    #[test]
    fn bigint_literal_hex_preserves_raw() {
        // 0x1fn → value "31", raw "0x1fn"
        let b = Expression::BigIntLiteral(BigIntLiteral {
            cv: None,
            value: "31".to_string(),
            raw: "0x1fn".to_string(),
        });
        let json = serde_json::to_string(&b).expect("serialize");
        // cv omitted; value + raw preserved as strings.
        assert!(!json.contains("\"cv\""), "cv key should be absent; got {}", json);
        assert!(json.contains("\"value\":\"31\""), "value mismatch; got {}", json);
        assert!(json.contains("\"raw\":\"0x1fn\""), "raw mismatch; got {}", json);
        assert_eq!(b.clone(), roundtrip(b));
    }

    #[test]
    fn binary_expression_with_plus_serializes_operator() {
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: Some("e.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"+\""), "got {}", json);
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "BinaryExpression");
    }

    #[test]
    fn every_binary_operator_round_trips() {
        // Each operator value: construct → serialize → deserialize →
        // structural equal.
        let ops = [
            (BinaryOperator::Eq, "=="),
            (BinaryOperator::NotEq, "!="),
            (BinaryOperator::StrictEq, "==="),
            (BinaryOperator::StrictNotEq, "!=="),
            (BinaryOperator::Lt, "<"),
            (BinaryOperator::LtEq, "<="),
            (BinaryOperator::Gt, ">"),
            (BinaryOperator::GtEq, ">="),
            (BinaryOperator::LeftShift, "<<"),
            (BinaryOperator::RightShift, ">>"),
            (BinaryOperator::UnsignedRightShift, ">>>"),
            (BinaryOperator::Add, "+"),
            (BinaryOperator::Sub, "-"),
            (BinaryOperator::Mul, "*"),
            (BinaryOperator::Div, "/"),
            (BinaryOperator::Mod, "%"),
            (BinaryOperator::Exp, "**"),
            (BinaryOperator::BitOr, "|"),
            (BinaryOperator::BitXor, "^"),
            (BinaryOperator::BitAnd, "&"),
            (BinaryOperator::In, "in"),
            (BinaryOperator::InstanceOf, "instanceof"),
        ];
        for (op, expected) in ops {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected), "op {:?}", op);
            let back: BinaryOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn logical_expression_distinct_from_binary() {
        // `a && b` is LogicalExpression, not BinaryExpression — the
        // short-circuit semantics are observable.
        let e = Expression::LogicalExpression(LogicalExpression {
            cv: None,
            operator: LogicalOperator::And,
            left: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            })),
            right: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "b".to_string(),
            })),
        });
        assert_eq!(type_tag(&e), "LogicalExpression");
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"&&\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn every_logical_operator_round_trips() {
        for (op, expected) in [
            (LogicalOperator::And, "&&"),
            (LogicalOperator::Or, "||"),
            (LogicalOperator::NullishCoalescing, "??"),
        ] {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
            let back: LogicalOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn unary_expression_prefix_true() {
        let e = Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"prefix\":true"));
        assert!(json.contains("\"operator\":\"!\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn every_unary_operator_round_trips() {
        for (op, expected) in [
            (UnaryOperator::Negate, "-"),
            (UnaryOperator::Plus, "+"),
            (UnaryOperator::Not, "!"),
            (UnaryOperator::BitNot, "~"),
            (UnaryOperator::TypeOf, "typeof"),
            (UnaryOperator::Void, "void"),
            (UnaryOperator::Delete, "delete"),
        ] {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
            let back: UnaryOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn assignment_expression_roundtrips() {
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: Some("ae.1".to_string()),
            operator: AssignmentOperator::Eq,
            left: AssignmentTarget::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            }),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 42.0,
                raw: "42".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"=\""));
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "AssignmentExpression");
    }

    #[test]
    fn assignment_to_member_target() {
        // `obj.prop = value`.
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: AssignmentOperator::AddEq,
            left: AssignmentTarget::MemberExpression(Box::new(MemberExpression {
                cv: None,
                object: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "obj".to_string(),
                })),
                property: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "prop".to_string(),
                })),
                computed: false,
            })),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"+=\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn conditional_expression_roundtrips() {
        let e = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("c.1".to_string()),
            test: Box::new(Expression::BooleanLiteral(BooleanLiteral {
                cv: None,
                value: true,
            })),
            consequent: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
            alternate: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            })),
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ConditionalExpression");
    }

    #[test]
    fn call_expression_roundtrips() {
        let e = Expression::CallExpression(CallExpression {
            cv: Some("call.1".to_string()),
            callee: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "f".to_string(),
            })),
            arguments: vec![
                Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
                Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 2.0,
                    raw: "2".to_string(),
                }),
            ],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "CallExpression");
    }

    #[test]
    fn member_expression_dot_vs_bracket() {
        // `obj.foo` — computed = false. ESTree convention: property
        // is an Identifier when not computed.
        let dot = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            })),
            property: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "foo".to_string(),
            })),
            computed: false,
        });
        // `obj[expr]` — computed = true; here property is a NumericLiteral
        // to make the difference structurally observable on round-trip
        // (untagged enums can otherwise ambiguously resolve identical
        // shapes).
        let bracket = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            })),
            property: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 0.0,
                raw: "0".to_string(),
            })),
            computed: true,
        });
        assert_ne!(dot, bracket);
        assert_eq!(dot.clone(), roundtrip(dot.clone()));
        assert_eq!(bracket.clone(), roundtrip(bracket.clone()));
        assert_eq!(type_tag(&dot), "MemberExpression");
    }

    #[test]
    fn array_expression_preserves_elisions() {
        // `[1, , 3]` — middle element is a hole.
        let e = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![
                Some(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                None, // elision
                Some(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 3.0,
                    raw: "3".to_string(),
                })),
            ],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ArrayExpression");
    }

    #[test]
    fn object_expression_with_property() {
        // `{ foo: 1 }`.
        let e = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(Identifier {
                    cv: None,
                    name: "foo".to_string(),
                }),
                value: Box::new(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                computed: false,
                shorthand: false,
                method: false,
            }],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ObjectExpression");
    }

    #[test]
    fn property_kind_serializes_lowercase() {
        // ESTree uses lowercase "init" / "get" / "set".
        for (k, expected) in [
            (PropertyKind::Init, "init"),
            (PropertyKind::Get, "get"),
            (PropertyKind::Set, "set"),
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    // -----------------------------------------------------------------
    // FunctionExpression (Phase 1.x, CLOC09) — the expression sibling of
    // FunctionDeclaration. Round-trips + tag + the `id`-is-optional and
    // `async`-renames-to-JSON contracts.
    // -----------------------------------------------------------------
    use crate::declaration::FunctionParam as TestFunctionParam;
    use crate::statement::{BlockStatement as TestBlock, ReturnStatement, Statement};

    /// `function f(x) { return x; }` used in value position.
    fn named_fn_expr() -> Expression {
        Expression::FunctionExpression(FunctionExpression {
            cv: Some("fe.1".to_string()),
            id: Some(Identifier { cv: None, name: "f".to_string() }),
            params: vec![TestFunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body: TestBlock {
                cv: None,
                body: vec![Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(Expression::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    })),
                })],
            },
            generator: false,
            is_async: false,
        })
    }

    /// Anonymous `function () {}`.
    fn anon_fn_expr() -> Expression {
        Expression::FunctionExpression(FunctionExpression {
            cv: None,
            id: None,
            params: vec![],
            body: TestBlock { cv: None, body: vec![] },
            generator: false,
            is_async: false,
        })
    }

    #[test]
    fn function_expression_named_roundtrips_and_tags() {
        let e = named_fn_expr();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "FunctionExpression");
    }

    #[test]
    fn function_expression_anonymous_roundtrips() {
        let e = anon_fn_expr();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "FunctionExpression");
    }

    #[test]
    fn function_expression_anonymous_omits_id_in_json() {
        // `id: None` must NOT appear in the wire format — an anonymous
        // function expression has no `id` key at all (ESTree uses
        // `"id": null`, but our `skip_serializing_if` omits it; both
        // deserialize back to `None`, which is what round-trip asserts).
        let json = serde_json::to_string(&anon_fn_expr()).expect("serialize");
        assert!(!json.contains("\"id\""), "anonymous fn-expr should omit id; got {}", json);
    }

    #[test]
    fn function_expression_async_key_renames() {
        // The `is_async` field serializes as JSON `"async"` (ESTree),
        // exactly as on FunctionDeclaration.
        let mut e = anon_fn_expr();
        if let Expression::FunctionExpression(f) = &mut e {
            f.is_async = true;
        }
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"async\":true"), "expected async key; got {}", json);
        assert!(!json.contains("isAsync"), "must not leak Rust field name; got {}", json);
        assert_eq!(e.clone(), roundtrip(e));
    }
}
