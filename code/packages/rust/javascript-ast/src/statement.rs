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
//! - [`EmptyStatement`]
//! - `Statement::Declaration(Declaration)` — untagged wrap so JSON
//!   collapses to the inner `{"type": "VariableDeclaration", ...}`
//!   shape directly.
//!
//! Phase 2 will add `SwitchStatement`, `TryStatement`, `ThrowStatement`,
//! `LabeledStatement`, `DoWhileStatement`, `ForInStatement`,
//! `ForOfStatement`, `DebuggerStatement`, and `WithStatement`.

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
    ForStatement(ForStatement),
    ReturnStatement(ReturnStatement),
    BreakStatement(BreakStatement),
    ContinueStatement(ContinueStatement),
    EmptyStatement(EmptyStatement),
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
    pub fn for_statement(s: ForStatement) -> Self {
        Self::Tagged(TaggedStatement::ForStatement(s))
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
    pub fn empty_statement(s: EmptyStatement) -> Self {
        Self::Tagged(TaggedStatement::EmptyStatement(s))
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

/// A lone semicolon `;`. Rare in user code but legal everywhere a
/// statement can appear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyStatement {
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
    fn empty_statement_roundtrips() {
        let s = Statement::empty_statement(EmptyStatement {
            cv: Some("empty.1".to_string()),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "EmptyStatement");
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
