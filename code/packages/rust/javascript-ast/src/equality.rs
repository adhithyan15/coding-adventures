//! Structural AST equality without provenance identity (CLOC31).
//!
//! CVs identify histories, not JavaScript behavior. This borrowed comparison
//! excludes only `cv`, preserves every other field (including raw spellings),
//! and distinguishes signed zero. It does not serialize or clone whole trees.
//! The explicit field lists/destructuring and exhaustive variant checks force
//! new AST shapes to update this contract instead of silently ignoring fields.

use crate::declaration::*;
use crate::expression::*;
use crate::statement::*;
use crate::{Program, ProgramItem, SourceType};
use coding_adventures_javascript_tokens::EsVersion;

/// Compare structure/representation while ignoring CV identity at every node.
/// This is conservative structural equality, not arbitrary JS equivalence.
/// NaN is unequal to itself; +0 and -0 are unequal. Traversal borrows the AST
/// without allocating. Callers must enforce their AST depth/resource bounds.
pub trait EqIgnoringCv {
    /// Compare all represented fields except provenance identity recursively.
    fn eq_ignoring_cv(&self, other: &Self) -> bool;
}

macro_rules! plain_eq {
    ($($ty:ty),+ $(,)?) => {$(
        impl EqIgnoringCv for $ty {
            fn eq_ignoring_cv(&self, other: &Self) -> bool { self == other }
        }
    )+};
}

impl EqIgnoringCv for f64 {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        self == other && self.to_bits() == other.to_bits()
    }
}

impl<T: EqIgnoringCv> EqIgnoringCv for Box<T> {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        self.as_ref().eq_ignoring_cv(other.as_ref())
    }
}
impl<T: EqIgnoringCv> EqIgnoringCv for Option<T> {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.eq_ignoring_cv(b),
            (None, None) => true,
            _ => false,
        }
    }
}
impl<T: EqIgnoringCv> EqIgnoringCv for Vec<T> {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.eq_ignoring_cv(b))
    }
}

macro_rules! node_eq {
    ($ty:ident { $($field:ident),* $(,)? }) => {
        impl EqIgnoringCv for $ty {
            fn eq_ignoring_cv(&self, other: &Self) -> bool {
                // No `..`: a newly added field must be handled deliberately.
                let Self { cv: _, $($field: _,)* } = self;
                let Self { cv: _, $($field: _,)* } = other;
                true $(&& self.$field.eq_ignoring_cv(&other.$field))*
            }
        }
    };
}
macro_rules! enum_eq {
    ($ty:ident { $variant:ident $(,)? }) => {
        impl EqIgnoringCv for $ty {
            fn eq_ignoring_cv(&self, other: &Self) -> bool {
                match (self, other) {
                    (Self::$variant(a), Self::$variant(b)) => a.eq_ignoring_cv(b),
                }
            }
        }
    };
    ($ty:ident { $($variant:ident),+ $(,)? }) => {
        impl EqIgnoringCv for $ty {
            fn eq_ignoring_cv(&self, other: &Self) -> bool {
                // Separate exhaustive match makes adding a variant an error,
                // even though mismatched variants below necessarily need `_`.
                match self { $(Self::$variant(_) => {},)+ }
                match (self, other) {
                    $((Self::$variant(a), Self::$variant(b)) => a.eq_ignoring_cv(b),)+
                    _ => false,
                }
            }
        }
    };
}

enum_eq!(Declaration {
    VariableDeclaration,
    FunctionDeclaration,
    ClassDeclaration,
    ImportDeclaration,
    ExportNamedDeclaration,
    ExportDefaultDeclaration,
    ExportAllDeclaration
});
enum_eq!(BindingTarget { Identifier });
enum_eq!(FunctionParam {
    Identifier,
    RestElement,
    AssignmentPattern
});
enum_eq!(ExportDefaultKind {
    Expression,
    FunctionDeclaration,
    ClassDeclaration
});
enum_eq!(Expression {
    Identifier,
    NumericLiteral,
    StringLiteral,
    BooleanLiteral,
    NullLiteral,
    BigIntLiteral,
    UndefinedLiteral,
    RegExpLiteral,
    BinaryExpression,
    LogicalExpression,
    UnaryExpression,
    AssignmentExpression,
    ConditionalExpression,
    CallExpression,
    MemberExpression,
    OptionalMemberExpression,
    OptionalCallExpression,
    ChainExpression,
    ArrayExpression,
    ObjectExpression,
    FunctionExpression,
    ArrowFunctionExpression,
    ClassExpression,
    TemplateLiteral,
    UpdateExpression,
    NewExpression,
    SequenceExpression,
    TaggedTemplateExpression,
    SpreadElement,
    YieldExpression,
    AwaitExpression,
    ThisExpression,
    Super,
    NewTarget,
    ImportMeta,
    ImportExpression
});
enum_eq!(AssignmentTarget {
    Identifier,
    MemberExpression
});
enum_eq!(ObjectMember { Property, Spread });
enum_eq!(PropertyKey {
    Identifier,
    PrivateName,
    StringLiteral,
    NumericLiteral,
    Expression
});
enum_eq!(ClassMember {
    Method,
    Field,
    StaticBlock
});
enum_eq!(ArrowBody { Expression, Block });
enum_eq!(ProgramItem {
    Statement,
    Declaration
});
enum_eq!(Statement {
    Tagged,
    Declaration
});
enum_eq!(TaggedStatement {
    ExpressionStatement,
    BlockStatement,
    IfStatement,
    WhileStatement,
    DoWhileStatement,
    ForStatement,
    ForInStatement,
    ForOfStatement,
    ReturnStatement,
    BreakStatement,
    ContinueStatement,
    LabeledStatement,
    ThrowStatement,
    SwitchStatement,
    TryStatement,
    EmptyStatement,
    DebuggerStatement,
    WithStatement
});
enum_eq!(ForInit {
    VariableDeclaration,
    Expression
});
plain_eq!(
    bool,
    String,
    EsVersion,
    VarKind,
    BinaryOperator,
    LogicalOperator,
    UnaryOperator,
    UpdateOperator,
    AssignmentOperator,
    PropertyKind,
    MethodKind,
    SourceType
);

node_eq!(VariableDeclaration { kind, declarations });
node_eq!(VariableDeclarator { id, init });
node_eq!(FunctionDeclaration {
    id,
    params,
    body,
    generator,
    is_async
});
node_eq!(RestElement { argument });
node_eq!(AssignmentPattern { left, right });
node_eq!(ClassDeclaration {
    id,
    super_class,
    body
});
node_eq!(ImportDeclaration { specifiers, source });
node_eq!(ExportNamedDeclaration {
    declaration,
    specifiers,
    source
});
node_eq!(ExportDefaultDeclaration { declaration });
node_eq!(ExportAllDeclaration { exported, source });
node_eq!(Identifier { name });
node_eq!(NumericLiteral { value, raw });
node_eq!(StringLiteral { value, raw });
node_eq!(RegExpLiteral { pattern, flags });
node_eq!(BooleanLiteral { value });
node_eq!(NullLiteral {});
node_eq!(UndefinedLiteral {});
node_eq!(BigIntLiteral { value, raw });
node_eq!(BinaryExpression {
    operator,
    left,
    right
});
node_eq!(LogicalExpression {
    operator,
    left,
    right
});
node_eq!(UnaryExpression {
    operator,
    prefix,
    argument
});
node_eq!(UpdateExpression {
    operator,
    prefix,
    argument
});
node_eq!(AssignmentExpression {
    operator,
    left,
    right
});
node_eq!(ConditionalExpression {
    test,
    consequent,
    alternate
});
node_eq!(CallExpression { callee, arguments });
node_eq!(NewExpression { callee, arguments });
node_eq!(SequenceExpression { expressions });
node_eq!(SpreadElement { argument });
node_eq!(YieldExpression { delegate, argument });
node_eq!(AwaitExpression { argument });
node_eq!(ImportExpression { source });
node_eq!(ThisExpression {});
node_eq!(Super {});
node_eq!(NewTarget {});
node_eq!(ImportMeta {});
node_eq!(MemberExpression {
    object,
    property,
    computed
});
node_eq!(OptionalMemberExpression {
    object,
    property,
    computed
});
node_eq!(OptionalCallExpression { callee, arguments });
node_eq!(ChainExpression { expression });
node_eq!(ArrayExpression { elements });
node_eq!(ObjectExpression { properties });
node_eq!(Property {
    kind,
    key,
    value,
    computed,
    shorthand,
    method
});
node_eq!(PrivateName { name });
node_eq!(FunctionExpression {
    id,
    params,
    body,
    generator,
    is_async
});
node_eq!(ClassExpression {
    id,
    super_class,
    body
});
node_eq!(MethodDefinition {
    key,
    kind,
    value,
    computed,
    is_static
});
node_eq!(PropertyDefinition {
    key,
    value,
    computed,
    is_static
});
node_eq!(ArrowFunctionExpression {
    params,
    body,
    is_async
});
node_eq!(TemplateLiteral {
    quasis,
    expressions
});
node_eq!(TemplateElement { raw, cooked, tail });
node_eq!(TaggedTemplateExpression { tag, quasi });
node_eq!(Program {
    version,
    source_type,
    body
});
node_eq!(ExpressionStatement { expression });
node_eq!(BlockStatement { body });
node_eq!(IfStatement {
    test,
    consequent,
    alternate
});
node_eq!(WhileStatement { test, body });
node_eq!(WithStatement { object, body });
node_eq!(DoWhileStatement { body, test });
node_eq!(ForStatement {
    init,
    test,
    update,
    body
});
node_eq!(ForInStatement { left, right, body });
node_eq!(ForOfStatement { left, right, body });
node_eq!(ReturnStatement { argument });
node_eq!(BreakStatement { label });
node_eq!(ContinueStatement { label });
node_eq!(LabeledStatement { label, body });
node_eq!(ThrowStatement { argument });
node_eq!(SwitchStatement {
    discriminant,
    cases
});
node_eq!(SwitchCase { test, consequent });
node_eq!(TryStatement {
    block,
    handler,
    finalizer
});
node_eq!(CatchClause { param, body });
node_eq!(EmptyStatement {});
node_eq!(DebuggerStatement {});

impl EqIgnoringCv for ExportSpecifier {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        let Self {
            local: _,
            exported: _,
        } = self;
        self.local.eq_ignoring_cv(&other.local) && self.exported.eq_ignoring_cv(&other.exported)
    }
}
impl EqIgnoringCv for ImportSpecifier {
    fn eq_ignoring_cv(&self, other: &Self) -> bool {
        match self {
            Self::Default(_)
            | Self::Namespace(_)
            | Self::Named {
                imported: _,
                local: _,
            } => {}
        }
        match (self, other) {
            (Self::Default(a), Self::Default(b)) | (Self::Namespace(a), Self::Namespace(b)) => {
                a.eq_ignoring_cv(b)
            }
            (
                Self::Named {
                    imported: a,
                    local: b,
                },
                Self::Named {
                    imported: c,
                    local: d,
                },
            ) => a.eq_ignoring_cv(c) && b.eq_ignoring_cv(d),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64, raw: &str, cv: &str) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            value,
            raw: raw.into(),
            cv: Some(cv.into()),
        })
    }

    fn array(value: Expression, cv: &str) -> Expression {
        Expression::ArrayExpression(ArrayExpression {
            elements: vec![Some(value)],
            cv: Some(cv.into()),
        })
    }

    #[test]
    fn nested_values_ignore_only_provenance() {
        let left = array(number(2.0, "2", "left-leaf"), "left-root");
        let right = array(number(2.0, "2", "right-leaf"), "right-root");
        assert_ne!(left, right, "ordinary equality still includes CVs");
        assert!(left.eq_ignoring_cv(&right));
        assert!(!left.eq_ignoring_cv(&array(number(3.0, "3", "other"), "other")));
        assert!(!left.eq_ignoring_cv(&array(number(2.0, "2.0", "other"), "other")));
        assert!(!left.eq_ignoring_cv(&number(2.0, "2", "other")));
    }

    #[test]
    fn numeric_comparison_preserves_javascript_distinctions() {
        // Folded zero literals can share the same raw spelling.
        assert!(!number(-0.0, "0", "a").eq_ignoring_cv(&number(0.0, "0", "b")));
        assert!(
            !array(number(-0.0, "0", "a"), "a").eq_ignoring_cv(&array(number(0.0, "0", "b"), "b"))
        );
        assert!(!number(f64::NAN, "NaN", "a").eq_ignoring_cv(&number(f64::NAN, "NaN", "b")));
        assert!(
            number(f64::INFINITY, "Infinity", "a").eq_ignoring_cv(&number(
                f64::INFINITY,
                "Infinity",
                "b"
            ))
        );
        assert!(
            !number(f64::INFINITY, "Infinity", "a").eq_ignoring_cv(&number(
                f64::NEG_INFINITY,
                "Infinity",
                "b"
            ))
        );
    }

    #[test]
    fn composite_flags_and_optional_fields_still_matter() {
        let mut left = FunctionExpression {
            cv: Some("left".into()),
            id: None,
            params: vec![],
            body: BlockStatement {
                cv: Some("body".into()),
                body: vec![],
            },
            generator: false,
            is_async: false,
        };
        let mut right = left.clone();
        right.cv = Some("right".into());
        right.body.cv = Some("other-body".into());
        assert!(left.eq_ignoring_cv(&right));
        right.is_async = true;
        assert!(!left.eq_ignoring_cv(&right));
        left.is_async = true;
        right.id = Some(Identifier {
            cv: None,
            name: "named".into(),
        });
        assert!(!left.eq_ignoring_cv(&right));
    }
}
