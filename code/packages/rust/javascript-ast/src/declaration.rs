//! ESTree-compatible declaration nodes (CLOC09 Phase 1).
//!
//! Two variants:
//!
//! - [`VariableDeclaration`] — `var` / `let` / `const`.
//! - [`FunctionDeclaration`] — `function name(params) { body }`.
//!
//! Phase 3 adds `ClassDeclaration`; Phase 4 adds the module-level
//! declarations (`ImportDeclaration`, `ExportNamedDeclaration`,
//! `ExportDefaultDeclaration`, `ExportAllDeclaration`).
//!
//! Declarations live in their own enum so passes that care about them
//! specifically (`closure-pass-rename`, `closure-pass-treeshake`,
//! `closure-pass-remove-unused-vars`) traverse `Vec<Declaration>`
//! directly. The `Statement::Declaration(Declaration)` untagged wrap
//! in [`crate::statement`] lets a declaration appear anywhere a
//! statement does — matching ESTree's flatter shape on the JSON wire.

use crate::expression::{ClassMember, Expression, Identifier};
use crate::statement::BlockStatement;
use crate::CvId;
use serde::{Deserialize, Serialize};

/// Tagged union of every declaration variant. JSON wire format is
/// `{"type": "<Variant>", ...}` per ESTree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Declaration {
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),
}

/// `var x = 1, y = 2;`, `let i = 0;`, `const PI = 3.14;`. The `kind`
/// distinguishes the three forms — semantics differ (function scope vs.
/// block scope; const is immutable binding).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub kind: VarKind,
    pub declarations: Vec<VariableDeclarator>,
}

/// Which declaration keyword introduced this binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarKind {
    Var,
    Let,
    Const,
}

/// One `name = expr` pair in a [`VariableDeclaration`].
///
/// `init` is optional (`let x;` has no initializer). When `id` is a
/// destructuring pattern (Phase 3), `init` is required at parse time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "VariableDeclarator", rename_all = "camelCase")]
pub struct VariableDeclarator {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub id: BindingTarget,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub init: Option<Expression>,
}

/// The bound side of a declaration — `let X = init`. Identifier in
/// Phase 1; destructuring patterns (`ArrayPattern` / `ObjectPattern`)
/// land in Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BindingTarget {
    Identifier(Identifier),
}

/// `function name(p1, p2) { body }`. `generator` and `is_async` are the
/// two flags that distinguish kinds of function (generator / async /
/// async generator). Arrow functions are Phase 3 (`ArrowFunctionExpression`).
///
/// `is_async` serializes as JSON `"async"` to match ESTree exactly —
/// Rust's `async` is a reserved keyword and `r#async` would be awkward
/// at call sites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub id: Identifier,
    pub params: Vec<FunctionParam>,
    pub body: BlockStatement,
    pub generator: bool,
    #[serde(rename = "async")]
    pub is_async: bool,
}

/// One parameter of a [`FunctionDeclaration`]. Identifier in Phase 1;
/// `AssignmentPattern` (default value), `RestElement`, and
/// destructuring patterns are Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionParam {
    Identifier(Identifier),
}

/// `class C { … }`, `class C extends B { … }` — a class written in
/// **statement** position. This is the *declaration* form: it binds the name
/// `C` in the enclosing scope, exactly as [`FunctionDeclaration`] binds a
/// function name.
///
/// # How it differs from [`crate::expression::ClassExpression`]
///
/// The class *expression* (`x = class {}`, `f(class C {})`) and the class
/// *declaration* (`class C {}` as a statement) share their entire body shape —
/// the `extends` heritage and the `{ … }` member list — and so reuse the same
/// [`ClassMember`] sub-AST. The **one** structural difference is the name:
///
/// | node                | `id`                    | why                                    |
/// |---------------------|-------------------------|----------------------------------------|
/// | `ClassExpression`   | `Option<Identifier>`    | may be anonymous (`x = class {}`)      |
/// | `ClassDeclaration`  | `Identifier` (required) | `class {}` in statement position is a  |
/// |                     |                         | syntax error — a declaration must name |
/// |                     |                         | its binding                            |
///
/// This mirrors exactly the `FunctionExpression` (optional `id`) vs
/// `FunctionDeclaration` (required `id`) split — a value may be anonymous, a
/// statement that introduces a binding cannot be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The bound class name. Required — see the type-level doc for why a
    /// declaration's `id` is not optional.
    pub id: Identifier,
    /// The `extends <expr>` operand, if any — same shape as
    /// [`crate::expression::ClassExpression::super_class`] (a boxed
    /// [`Expression`], so an identifier / member / call heritage is all legal).
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "superClass")]
    pub super_class: Option<Box<Expression>>,
    /// The class body — the ordered member list between the braces. May be
    /// empty (`class C {}`). Reuses [`ClassMember`] from the expression form.
    pub body: Vec<ClassMember>,
}

#[cfg(test)]
mod tests {
    //! Round-trip tests for every Phase 1 declaration variant.
    use super::*;
    use crate::expression::{Expression, NumericLiteral};
    use crate::statement::{BlockStatement, ReturnStatement, Statement};

    fn roundtrip(d: Declaration) -> Declaration {
        let json = serde_json::to_string(&d).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    fn type_tag(d: &Declaration) -> String {
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(d).unwrap()).unwrap();
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
    fn variable_declaration_let_roundtrips() {
        let d = Declaration::VariableDeclaration(VariableDeclaration {
            cv: Some("vd.1".to_string()),
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: Some("vdr.1".to_string()),
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: "i".to_string(),
                }),
                init: Some(lit(0.0)),
            }],
        });
        assert_eq!(d.clone(), roundtrip(d.clone()));
        assert_eq!(type_tag(&d), "VariableDeclaration");
    }

    #[test]
    fn every_var_kind_roundtrips_with_correct_string() {
        for (k, expected) in [
            (VarKind::Var, "var"),
            (VarKind::Let, "let"),
            (VarKind::Const, "const"),
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
            let back: VarKind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn variable_declarator_serializes_with_type_tag() {
        // ESTree gives VariableDeclarator its own "type": "VariableDeclarator"
        // (rather than being untagged inside the parent). Lock this in.
        let v = VariableDeclarator {
            cv: None,
            id: BindingTarget::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            }),
            init: None,
        };
        let json = serde_json::to_string(&v).expect("serialize");
        assert!(
            json.contains("\"type\":\"VariableDeclarator\""),
            "got {}",
            json
        );
    }

    #[test]
    fn variable_declarator_without_init_omits_field() {
        // `let x;` — no init.
        let v = VariableDeclarator {
            cv: None,
            id: BindingTarget::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            }),
            init: None,
        };
        let json = serde_json::to_string(&v).expect("serialize");
        assert!(!json.contains("\"init\""), "got {}", json);
    }

    #[test]
    fn function_declaration_roundtrips() {
        // function f(x) { return x; }
        let d = Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: Some("fd.1".to_string()),
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![FunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body: BlockStatement {
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
        });
        assert_eq!(d.clone(), roundtrip(d.clone()));
        assert_eq!(type_tag(&d), "FunctionDeclaration");
    }

    #[test]
    fn function_declaration_is_async_serializes_as_async_in_json() {
        // ESTree calls the field "async"; we use is_async in Rust
        // because async is a reserved keyword. Round-trip both ways.
        let d = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: false,
            is_async: true,
        };
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(
            json.contains("\"async\":true"),
            "expected JSON 'async' field; got {}",
            json
        );
        assert!(
            !json.contains("\"isAsync\""),
            "must not serialize as isAsync; got {}",
            json
        );
        let back: FunctionDeclaration =
            serde_json::from_str(&json).expect("deserialize");
        assert!(back.is_async);
    }

    #[test]
    fn function_declaration_generator_flag() {
        let d = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "gen".to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: true,
            is_async: false,
        };
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(json.contains("\"generator\":true"));
        let back: FunctionDeclaration =
            serde_json::from_str(&json).expect("deserialize");
        assert!(back.generator);
    }

    #[test]
    fn class_declaration_roundtrips_with_type_tag() {
        // `class C extends B { m() {} }` — a named declaration with heritage and
        // one member, exercising the required `id`, the `superClass` operand,
        // and the reused `ClassMember` body.
        use crate::expression::{
            ClassMember, FunctionExpression, MethodDefinition, MethodKind, PropertyKey,
        };
        let d = Declaration::ClassDeclaration(ClassDeclaration {
            cv: Some("cd.1".to_string()),
            id: Identifier {
                cv: None,
                name: "C".to_string(),
            },
            super_class: Some(Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "B".to_string(),
            }))),
            body: vec![ClassMember::Method(MethodDefinition {
                cv: None,
                key: PropertyKey::Identifier(Identifier {
                    cv: None,
                    name: "m".to_string(),
                }),
                kind: MethodKind::Method,
                value: FunctionExpression {
                    cv: None,
                    id: None,
                    params: vec![],
                    body: BlockStatement {
                        cv: None,
                        body: vec![],
                    },
                    generator: false,
                    is_async: false,
                },
                computed: false,
                is_static: false,
            })],
        });
        assert_eq!(type_tag(&d), "ClassDeclaration");
        assert_eq!(roundtrip(d.clone()), d);
    }

    #[test]
    fn class_declaration_serializes_superclass_camelcase() {
        // The heritage field must serialize as ESTree `superClass`, not
        // `super_class`, and be omitted entirely when there is no heritage.
        let with = Declaration::ClassDeclaration(ClassDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "C".to_string(),
            },
            super_class: Some(Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "B".to_string(),
            }))),
            body: vec![],
        });
        let json = serde_json::to_string(&with).expect("serialize");
        assert!(json.contains("\"superClass\""), "got {}", json);
        assert!(!json.contains("super_class"), "got {}", json);

        let without = Declaration::ClassDeclaration(ClassDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "C".to_string(),
            },
            super_class: None,
            body: vec![],
        });
        let json = serde_json::to_string(&without).expect("serialize");
        assert!(!json.contains("superClass"), "heritage omitted; got {}", json);
    }
}
