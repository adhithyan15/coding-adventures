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

use crate::expression::{ClassMember, Expression, Identifier, StringLiteral};
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
    /// `import x from "y"` — an ES-module import declaration (CLOC12.188).
    /// A *module-level* declaration: legal only at the top of a module, but
    /// modelled here as a `Declaration` variant (the codebase's Phase-4 plan)
    /// so it flows through the existing `ProgramItem::Declaration` plumbing.
    ImportDeclaration(ImportDeclaration),
    /// `export { a, b as c }` / `export { a } from "y"` / `export const x = 1`
    /// / `export function f(){}` / `export class C {}` — a named or
    /// declaration export (CLOC12.189). See [`ExportNamedDeclaration`].
    ExportNamedDeclaration(ExportNamedDeclaration),
    /// `export default <expr | function | class>` (CLOC12.189). See
    /// [`ExportDefaultDeclaration`].
    ExportDefaultDeclaration(ExportDefaultDeclaration),
    /// `export * from "y"` (and, once the grammar allows it, `export * as ns
    /// from "y"`) — a re-export of an entire module (CLOC12.189). See
    /// [`ExportAllDeclaration`].
    ExportAllDeclaration(ExportAllDeclaration),
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

/// A **rest parameter** — the trailing `...name` in a parameter list that
/// gathers every remaining call argument into a fresh array bound to `name`:
///
/// ```text
///   function f(a, ...rest) {}   // f(1,2,3) → a=1, rest=[2,3]
///   function g(...all) {}       // g(1,2)   → all=[1,2]
/// ```
///
/// ESTree `RestElement`. A rest parameter is always **last** in the list and
/// never carries a default (`...x = []` is a syntax error), so there is no
/// `AssignmentPattern` wrapping here. Only a **simple identifier** target is
/// modelled: a destructuring rest target (`...[a, b]`, `...{x}`) reuses the
/// Phase-3 [`BindingTarget`] machinery and is declined by the bridge for now.
///
/// The `argument` field name (vs [`Identifier`]'s `name`) is what lets the
/// `#[serde(untagged)]` [`FunctionParam`] tell a rest element apart from a
/// plain identifier parameter by shape alone — no `type` tag needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestElement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The identifier the gathered arguments are bound to (the `rest` in
    /// `...rest`). A simple name in this slice; destructuring targets are
    /// Phase 3.
    pub argument: Identifier,
}

/// A **default parameter** — `name = expr` in a parameter list. When the
/// caller omits the argument (or passes `undefined`), the binding takes the
/// value of the default expression instead:
///
/// ```text
///   function f(a = 1) {}        // f()    → a=1 ;  f(5) → a=5
///   function g(a, b = a + 1) {} // g(2)   → a=2, b=3
/// ```
///
/// ESTree `AssignmentPattern`: `{ left, right }`. `left` is the bound name and
/// `right` is the default-value [`Expression`]. Only a **simple identifier**
/// target is modelled here — a destructuring target with a default
/// (`{x} = {}`, `[a] = []`) reuses the Phase-3 pattern machinery and is
/// declined by the bridge for now.
///
/// The distinctive `left` + `right` field shape is what lets the
/// `#[serde(untagged)]` [`FunctionParam`] tell a default parameter apart from a
/// plain [`Identifier`] (`name`) or a [`RestElement`] (`argument`) — no `type`
/// tag needed.
///
/// **Why `right` must be a full [`Expression`], not just a literal:** the
/// default is live code that the optimizer walks. `function f(a = 1 + 2)` folds
/// to `function f(a = 3)`; a name referenced in the default (`b = SOME_GLOBAL`)
/// participates in renaming and inlining exactly as any other expression does.
/// Every pass that visits a parameter list therefore recurses into `right`
/// through the same expression-visit path it uses for a function body — unlike
/// [`RestElement`], whose only payload is a bound name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentPattern {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The bound name (`a` in `a = 1`). A simple identifier in this slice.
    pub left: Identifier,
    /// The default-value expression (`1` in `a = 1`). Full live code: folded,
    /// renamed, and inlined by the passes like any other expression.
    pub right: Expression,
}

/// One parameter of a [`FunctionDeclaration`]. A plain [`Identifier`], a
/// trailing [`RestElement`] (`...name`), or a default-valued
/// [`AssignmentPattern`] (`name = expr`) in this slice; destructuring patterns
/// are Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionParam {
    Identifier(Identifier),
    RestElement(RestElement),
    AssignmentPattern(AssignmentPattern),
}

impl FunctionParam {
    /// The single [`Identifier`] this parameter binds — the name itself for a
    /// plain identifier param, or the gathered-array name for a `...rest`
    /// param. Both bind exactly one simple name in this slice, so passes that
    /// count, look up, or reason about parameter bindings can treat the two
    /// uniformly through this accessor instead of matching every call site.
    pub fn binding_identifier(&self) -> &Identifier {
        match self {
            FunctionParam::Identifier(id) => id,
            FunctionParam::RestElement(re) => &re.argument,
            FunctionParam::AssignmentPattern(ap) => &ap.left,
        }
    }

    /// Mutable twin of [`binding_identifier`](Self::binding_identifier) — used
    /// by the renaming passes, which rewrite `id.name` in place. A `...rest`
    /// param's gathered name is an ordinary renameable local, so it is renamed
    /// through the same path as a plain parameter.
    pub fn binding_identifier_mut(&mut self) -> &mut Identifier {
        match self {
            FunctionParam::Identifier(id) => id,
            FunctionParam::RestElement(re) => &mut re.argument,
            FunctionParam::AssignmentPattern(ap) => &mut ap.left,
        }
    }

    /// The default-value expression when this parameter is an
    /// [`AssignmentPattern`] (`name = expr`), else `None`. Passes that walk
    /// parameter defaults — constant-fold, the renamers, the inliners — reach
    /// the live `right` expression through this accessor, then visit it with the
    /// same machinery they use for a function body.
    pub fn default_value(&self) -> Option<&Expression> {
        match self {
            FunctionParam::AssignmentPattern(ap) => Some(&ap.right),
            _ => None,
        }
    }

    /// Mutable twin of [`default_value`](Self::default_value) — the folding and
    /// renaming passes rewrite the default expression in place (`a = 1 + 2` →
    /// `a = 3`), so they need `&mut` access to `right`.
    pub fn default_value_mut(&mut self) -> Option<&mut Expression> {
        match self {
            FunctionParam::AssignmentPattern(ap) => Some(&mut ap.right),
            _ => None,
        }
    }
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

/// `import x from "y"` / `import {a, b as c} from "y"` / `import * as ns from
/// "y"` / `import "y"` — an ES-module **import declaration** (CLOC12.188).
///
/// # Anatomy
///
/// ```text
///   import  x, { a, b as c }  from  "y" ;
///   └────┘  └──────────────┘  └──┘  └─┘
///   keyword     specifiers     from  source
/// ```
///
/// - `specifiers` — the bound names the import introduces, in source order.
///   A **side-effect** import (`import "y";`) has an EMPTY `specifiers` list —
///   it runs the module for effects and binds nothing.
/// - `source` — the module specifier string literal (`"y"`). Preserved as a
///   [`StringLiteral`] so the emitter re-quotes it faithfully.
///
/// # Renaming note
///
/// The *local* names an import introduces are ordinary module-scoped bindings.
/// But the `imported` half of a named specifier (and a `default`/`namespace`
/// binding's link to the foreign module) references an **export of another
/// module** — renaming it would break the cross-module contract. The renaming
/// passes therefore treat import-introduced names conservatively; that gate
/// lands with the bridge PR that makes this node reachable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The imported bindings, in source order. Empty for a side-effect import.
    pub specifiers: Vec<ImportSpecifier>,
    /// The module specifier — `"y"` in `import … from "y"`.
    pub source: StringLiteral,
}

/// One binding introduced by an [`ImportDeclaration`]. ESTree splits these into
/// three node types; we model them as one `#[serde(tag = "type")]` enum so the
/// JSON wire stays ESTree-shaped (`ImportDefaultSpecifier` /
/// `ImportNamespaceSpecifier` / `ImportSpecifier`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImportSpecifier {
    /// `import x from "y"` — the default export bound to local `x`.
    #[serde(rename = "ImportDefaultSpecifier")]
    Default(Identifier),
    /// `import * as ns from "y"` — the whole module namespace bound to `ns`.
    #[serde(rename = "ImportNamespaceSpecifier")]
    Namespace(Identifier),
    /// `import { a } from "y"` (→ `imported = local = a`) or
    /// `import { a as c } from "y"` (→ `imported = a`, `local = c`). `imported`
    /// is the foreign export's name; `local` is the name bound in this module.
    #[serde(rename = "ImportSpecifier")]
    Named {
        imported: Identifier,
        local: Identifier,
    },
}

/// `export { a, b as c }` / `export { a } from "y"` / `export const x = 1` /
/// `export function f(){}` / `export class C {}` / `export var v = 1` — an
/// ES-module **named or declaration export** (CLOC12.189, ESTree
/// `ExportNamedDeclaration`).
///
/// # Three shapes, one node
///
/// ```text
///   export const x = 1;              declaration = Some(…),  specifiers = [],       source = None
///   export { a, b as c };            declaration = None,     specifiers = [a, b→c], source = None
///   export { a } from "y";           declaration = None,     specifiers = [a],      source = Some("y")
/// ```
///
/// ESTree collapses all three into one node with three optional parts:
/// - `declaration` — an inner [`Declaration`] the `export` prefixes
///   (`export const x = 1`, `export function f(){}`, `export class C {}`). Boxed
///   because a `Declaration` may contain an `ExportNamedDeclaration` in turn
///   (the enum is self-referential). `None` for the `export { … }` forms.
/// - `specifiers` — the `{ a, b as c }` list (empty for a declaration export).
/// - `source` — the `from "y"` re-export module, if present. Only a
///   `{ … } from "y"` re-export carries one; a plain `export { … }` does not.
///
/// # Renaming note
///
/// A name in `specifiers` (or a binding introduced by an inner `declaration`)
/// is part of this module's *public surface*: another module may import it by
/// exactly that name. Renaming it would break the cross-module contract, just
/// as with [`ImportDeclaration`]. The renaming passes gate on the presence of
/// an export; that gate lands with the bridge PR that makes this node reachable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNamedDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The inner declaration (`export const x = 1`), or `None` for a
    /// `export { … }` specifier list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub declaration: Option<Box<Declaration>>,
    /// The `{ a, b as c }` export specifiers, in source order. Empty for a
    /// declaration export.
    pub specifiers: Vec<ExportSpecifier>,
    /// The `from "y"` re-export source, if present.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<StringLiteral>,
}

/// One `{ … }` entry of an [`ExportNamedDeclaration`] (ESTree
/// `ExportSpecifier`). `export { a }` → `local == exported == a`;
/// `export { a as c }` → `local = a` (the in-module binding), `exported = c`
/// (the name other modules see). Note the local/exported order is the mirror
/// image of [`ImportSpecifier::Named`]'s imported/local.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "ExportSpecifier")]
pub struct ExportSpecifier {
    /// The name bound inside this module.
    pub local: Identifier,
    /// The name exposed to importers (`as c`), equal to `local` when no `as`.
    pub exported: Identifier,
}

/// `export default <expr | function | class>` — an ES-module **default export**
/// (CLOC12.189, ESTree `ExportDefaultDeclaration`).
///
/// A module has at most one default export. The operand is either an
/// *expression* (`export default 1`, `export default foo()`) or a
/// *function/class declaration* (`export default function f(){}`,
/// `export default class C {}`). ESTree keeps these under one `declaration`
/// field of union type; we model that union as [`ExportDefaultKind`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDefaultDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The exported value or declaration.
    pub declaration: ExportDefaultKind,
}

/// The operand of an [`ExportDefaultDeclaration`]: an expression, or a
/// function / class declaration. `#[serde(untagged)]` so the JSON wire is the
/// inner node's own `{"type": …}` object, matching ESTree's union-typed
/// `declaration` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExportDefaultKind {
    /// `export default <expr>;` — e.g. `export default 1`, `export default a+b`.
    Expression(Box<Expression>),
    /// `export default function f(){}` — a named or anonymous function.
    FunctionDeclaration(FunctionDeclaration),
    /// `export default class C {}` — a named or anonymous class.
    ClassDeclaration(ClassDeclaration),
}

/// `export * from "y"` (and, once the grammar allows it, `export * as ns from
/// "y"`) — an ES-module **re-export of an entire module** (CLOC12.189, ESTree
/// `ExportAllDeclaration`).
///
/// `exported` names the namespace binding for the `export * as ns` form and is
/// `None` for the bare `export *`. The current grammar rejects `export * as ns`
/// at the parse layer, so the bridgeable subset always has `exported = None`;
/// the field is kept so the node models the full ESTree shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAllDeclaration {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The `as ns` namespace name, or `None` for a bare `export *`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exported: Option<Identifier>,
    /// The module being re-exported — `"y"` in `export * from "y"`.
    pub source: StringLiteral,
}

impl Declaration {
    /// Convenience constructor for an [`ImportDeclaration`].
    pub fn import_declaration(d: ImportDeclaration) -> Self {
        Declaration::ImportDeclaration(d)
    }

    /// Convenience constructor for an [`ExportNamedDeclaration`].
    pub fn export_named_declaration(d: ExportNamedDeclaration) -> Self {
        Declaration::ExportNamedDeclaration(d)
    }

    /// Convenience constructor for an [`ExportDefaultDeclaration`].
    pub fn export_default_declaration(d: ExportDefaultDeclaration) -> Self {
        Declaration::ExportDefaultDeclaration(d)
    }

    /// Convenience constructor for an [`ExportAllDeclaration`].
    pub fn export_all_declaration(d: ExportAllDeclaration) -> Self {
        Declaration::ExportAllDeclaration(d)
    }
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

    // ---------------------------------------------------------------
    // CLOC12.188 — ImportDeclaration.
    // ---------------------------------------------------------------

    fn id(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
    }

    fn src(v: &str) -> StringLiteral {
        StringLiteral {
            cv: None,
            value: v.to_string(),
            raw: format!("\"{v}\""),
        }
    }

    #[test]
    fn import_declaration_all_specifier_kinds_roundtrip() {
        // `import def, * as ns, { a, b as c } from "y"` — one of every
        // specifier kind in a single declaration exercises all serde arms.
        // (Real JS can't combine namespace + named, but the AST models each
        // independently, so a mixed list is the strongest round-trip probe.)
        let d = Declaration::import_declaration(ImportDeclaration {
            cv: None,
            specifiers: vec![
                ImportSpecifier::Default(id("def")),
                ImportSpecifier::Namespace(id("ns")),
                ImportSpecifier::Named {
                    imported: id("a"),
                    local: id("a"),
                },
                ImportSpecifier::Named {
                    imported: id("b"),
                    local: id("c"),
                },
            ],
            source: src("y"),
        });
        assert_eq!(roundtrip(d.clone()), d);
        assert_eq!(type_tag(&d), "ImportDeclaration");

        // ESTree-shaped specifier type tags.
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(json.contains("\"ImportDefaultSpecifier\""), "got {json}");
        assert!(json.contains("\"ImportNamespaceSpecifier\""), "got {json}");
        assert!(json.contains("\"ImportSpecifier\""), "got {json}");
    }

    #[test]
    fn side_effect_import_has_empty_specifiers() {
        // `import "y";` binds nothing — an empty specifier list round-trips.
        let d = Declaration::import_declaration(ImportDeclaration {
            cv: None,
            specifiers: vec![],
            source: src("y"),
        });
        assert_eq!(roundtrip(d.clone()), d);
        match &d {
            Declaration::ImportDeclaration(i) => {
                assert!(i.specifiers.is_empty());
                assert_eq!(i.source.value, "y");
            }
            other => panic!("expected ImportDeclaration, got {other:?}"),
        }
    }

    // ---------------------------------------------------------------
    // CLOC12.189 — Export declarations.
    // ---------------------------------------------------------------

    fn spec(local: &str, exported: &str) -> ExportSpecifier {
        ExportSpecifier {
            local: id(local),
            exported: id(exported),
        }
    }

    #[test]
    fn export_named_specifiers_reexport_roundtrip() {
        // `export { a, b as c } from "y"` — specifiers + a re-export source, no
        // inner declaration. Exercises the plain/aliased specifier arms and the
        // `source` field together.
        let d = Declaration::export_named_declaration(ExportNamedDeclaration {
            cv: None,
            declaration: None,
            specifiers: vec![spec("a", "a"), spec("b", "c")],
            source: Some(src("y")),
        });
        assert_eq!(roundtrip(d.clone()), d);
        assert_eq!(type_tag(&d), "ExportNamedDeclaration");
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(json.contains("\"ExportSpecifier\""), "got {json}");
    }

    #[test]
    fn export_declaration_wraps_inner_declaration_roundtrip() {
        // `export const x = 1;` — the inner declaration is present, specifiers
        // empty, no source. Proves the boxed self-referential `declaration`
        // field round-trips.
        let inner = Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Const,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(id("x")),
                init: Some(Expression::Identifier(id("y"))),
            }],
        });
        let d = Declaration::export_named_declaration(ExportNamedDeclaration {
            cv: None,
            declaration: Some(Box::new(inner)),
            specifiers: vec![],
            source: None,
        });
        assert_eq!(roundtrip(d.clone()), d);
        match &d {
            Declaration::ExportNamedDeclaration(e) => {
                assert!(e.specifiers.is_empty());
                assert!(e.source.is_none());
                assert!(matches!(
                    e.declaration.as_deref(),
                    Some(Declaration::VariableDeclaration(_))
                ));
            }
            other => panic!("expected ExportNamedDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn export_default_expression_roundtrip() {
        // `export default x;` — a default export whose operand is an expression.
        let d = Declaration::export_default_declaration(ExportDefaultDeclaration {
            cv: None,
            declaration: ExportDefaultKind::Expression(Box::new(Expression::Identifier(id("x")))),
        });
        assert_eq!(roundtrip(d.clone()), d);
        assert_eq!(type_tag(&d), "ExportDefaultDeclaration");
    }

    #[test]
    fn export_all_declaration_roundtrip() {
        // `export * from "y";` — bare re-export-all, no namespace binding.
        let d = Declaration::export_all_declaration(ExportAllDeclaration {
            cv: None,
            exported: None,
            source: src("y"),
        });
        assert_eq!(roundtrip(d.clone()), d);
        assert_eq!(type_tag(&d), "ExportAllDeclaration");
        match &d {
            Declaration::ExportAllDeclaration(e) => {
                assert!(e.exported.is_none());
                assert_eq!(e.source.value, "y");
            }
            other => panic!("expected ExportAllDeclaration, got {other:?}"),
        }
    }

    // ---- CLOC12.191: default parameters (AssignmentPattern) ----

    /// Build the numeric-literal default `= n` used by the default-param tests.
    fn num_default(value: f64, raw: &str) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value,
            raw: raw.to_string(),
        })
    }

    #[test]
    fn default_param_roundtrips_and_untagged_discriminates() {
        // `function f(a, b = 1) {}` — a fixed parameter followed by a default.
        // The untagged `FunctionParam` must tell the three shapes apart:
        // Identifier (`name`), RestElement (`argument`), AssignmentPattern
        // (`left`+`right`). Serialize then deserialize and confirm each param
        // deserializes back to the *same* variant — the whole point of the
        // shape-based discrimination.
        let d = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![
                FunctionParam::Identifier(Identifier {
                    cv: None,
                    name: "a".to_string(),
                }),
                FunctionParam::AssignmentPattern(AssignmentPattern {
                    cv: None,
                    left: Identifier {
                        cv: None,
                        name: "b".to_string(),
                    },
                    right: num_default(1.0, "1"),
                }),
            ],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: false,
            is_async: false,
        };

        let json = serde_json::to_string(&d).expect("serialize");
        // The default carries ESTree's `left`/`right`, not a `type` tag.
        assert!(json.contains("\"left\""), "expected `left` field; got {json}");
        assert!(
            json.contains("\"right\""),
            "expected `right` field; got {json}"
        );

        let back: FunctionDeclaration = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, d, "default param did not round-trip");
        assert!(
            matches!(back.params[0], FunctionParam::Identifier(_)),
            "first param should stay a plain Identifier"
        );
        assert!(
            matches!(back.params[1], FunctionParam::AssignmentPattern(_)),
            "second param should stay an AssignmentPattern (not mis-read as Identifier)"
        );
    }

    #[test]
    fn default_param_accessors_reach_name_and_expr() {
        // The `binding_identifier*` accessors return the LEFT name; the
        // `default_value*` accessors expose the RIGHT expression that the
        // folding/renaming passes rewrite in place.
        let mut p = FunctionParam::AssignmentPattern(AssignmentPattern {
            cv: None,
            left: Identifier {
                cv: None,
                name: "a".to_string(),
            },
            right: num_default(2.0, "2"),
        });

        assert_eq!(p.binding_identifier().name, "a");
        assert!(
            p.default_value().is_some(),
            "a default param must expose its default expression"
        );

        // A plain identifier has no default.
        let plain = FunctionParam::Identifier(Identifier {
            cv: None,
            name: "x".to_string(),
        });
        assert!(plain.default_value().is_none());

        // Mutating through the accessors reaches both sides.
        p.binding_identifier_mut().name = "renamed".to_string();
        *p.default_value_mut().expect("default expr") = num_default(3.0, "3");
        assert_eq!(p.binding_identifier().name, "renamed");
        match p.default_value().expect("default expr") {
            Expression::NumericLiteral(n) => assert_eq!(n.value, 3.0),
            other => panic!("expected numeric default, got {other:?}"),
        }
    }
}
