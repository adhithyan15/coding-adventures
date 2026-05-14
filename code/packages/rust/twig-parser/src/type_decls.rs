//! # `emit_type_declarations` — parser-side type declaration emission
//!
//! This module converts a parsed Twig [`Program`] into a [`TypeDeclarations`]
//! struct, analogous to TypeScript's `.d.ts` emit step.  The result is
//! consumed by the generic `grammar-type-checker` crate to drive
//! language-agnostic type inference over raw `GrammarASTNode` trees.
//!
//! ## What gets emitted
//!
//! | Twig source form        | Emitted to                                       |
//! |-------------------------|--------------------------------------------------|
//! | `(type Name expr)`      | `named_types["Name"] = Alias { target: kind }`   |
//! | `(record Name …)`       | `named_types["Name"] = Record { fields }`        |
//! | `(union Name …)`        | `named_types["Name"] = Union { variants }`       |
//! | `(define name expr)`    | `globals["name"] = kind_of(expr)`               |
//! | `(module … (typed …))`  | `typed_mode = Some(…)`                           |
//!
//! ## Kind mapping
//!
//! Twig type names and [`TypeAnnotation`] variants map to [`KindDecl`] as
//! follows:
//!
//! | Twig type / annotation     | KindDecl              |
//! |----------------------------|-----------------------|
//! | `int` / `UnrefinedInt`     | `Int`                 |
//! | `bool` / `UnrefinedBool`   | `Bool`                |
//! | `str`                      | `Str`                 |
//! | `nil`                      | `Nil`                 |
//! | `any` / `Any`              | `Any`                 |
//! | `RangeInt{..}`             | `Int`                 |
//! | `MembershipInt{..}`        | `Int`                 |
//! | `Opaque(Name(n))`          | `Named(n)` if unknown name; else resolved |
//! | `(lambda (x y) body)`      | `Function { arity }`  |
//! | anything else              | `Any`                 |
//!
//! Forward references work because `define` globals are seeded into the
//! checker scope *before* the walk — the emitted `TypeDeclarations` captures
//! the final global kind, not an intermediate inference state.

use type_declarations::{
    FieldDecl, KindDecl, NamedTypeDecl, TypeDeclarations, TypedModeDecl, VariantDecl,
};

use crate::ast_nodes::{
    Expr, Form, Program, TypeAnnotation, TypeExpr, TypedMode,
};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Convert a parsed Twig `Program` into a [`TypeDeclarations`] struct.
///
/// This is the Twig implementation of the generic "emit declarations from
/// parsed AST" step.  Call this immediately after [`crate::parse`] (or
/// [`crate::extract_program`]) to produce the declarations the generic
/// `grammar-type-checker` needs.
///
/// ## Example
///
/// ```no_run
/// use twig_parser::{parse, emit_type_declarations};
///
/// let program = parse("(module mylib (typed strict)) (define x : int 42)").unwrap();
/// let decls = emit_type_declarations(&program);
/// assert_eq!(decls.globals["x"], type_declarations::KindDecl::Int);
/// ```
pub fn emit_type_declarations(program: &Program) -> TypeDeclarations {
    // ── 1. Start with an empty TypeDeclarations for "twig". ───────────────
    let mut decls = TypeDeclarations::new("twig");

    // ── 2. Capture the typed_mode from the optional module declaration. ───
    if let Some(mi) = &program.module_info {
        decls.typed_mode = mi.typed_mode.as_ref().map(twig_mode_to_decl);
    }

    // ── 3. Collect named type declarations (alias / record / union). ───────
    for form in &program.forms {
        match form {
            Form::TypeAlias(ta) => {
                let target = type_expr_to_kind(&ta.expr);
                decls
                    .named_types
                    .insert(ta.name.clone(), NamedTypeDecl::Alias { target });
            }
            Form::RecordDef(r) => {
                let fields = r
                    .fields
                    .iter()
                    .map(|f| FieldDecl {
                        name: f.name.clone(),
                        kind: annotation_to_kind(&f.type_annotation),
                    })
                    .collect();
                decls
                    .named_types
                    .insert(r.name.clone(), NamedTypeDecl::Record { fields });
            }
            Form::UnionDef(u) => {
                let variants = u
                    .variants
                    .iter()
                    .map(|v| VariantDecl {
                        name: v.name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .map(|f| FieldDecl {
                                name: f.name.clone(),
                                kind: annotation_to_kind(&f.type_annotation),
                            })
                            .collect(),
                    })
                    .collect();
                decls
                    .named_types
                    .insert(u.name.clone(), NamedTypeDecl::Union { variants });
            }
            // Define globals are collected below; expressions are skipped here.
            Form::Define(_) | Form::Expr(_) => {}
        }
    }

    // ── 4. Collect global defines. ────────────────────────────────────────
    //
    // Two-pass behaviour: named types are registered above so that a
    // `define` referring to a user-defined record/union type (via its
    // type_annotation) resolves correctly via KindDecl::Named.
    for form in &program.forms {
        if let Form::Define(d) = form {
            // Prefer an explicit type_annotation when present.
            let kind = if let Some(ann) = &d.type_annotation {
                annotation_to_kind(ann)
            } else {
                // Fall back to inferring kind from the expression itself.
                expr_to_kind(&d.expr)
            };
            decls.globals.insert(d.name.clone(), kind);
        }
    }

    decls
}

// ---------------------------------------------------------------------------
// Conversion helpers — TypedMode → TypedModeDecl
// ---------------------------------------------------------------------------

/// Map a Twig [`TypedMode`] (from `ast_nodes`) to the language-agnostic
/// [`TypedModeDecl`] (from `type-declarations`).
fn twig_mode_to_decl(mode: &TypedMode) -> TypedModeDecl {
    // The two enums are structurally identical; we keep them separate so
    // neither crate depends on the other's AST.
    match mode {
        TypedMode::Off => TypedModeDecl::Off,
        TypedMode::Lenient => TypedModeDecl::Lenient,
        TypedMode::Strict => TypedModeDecl::Strict,
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers — TypeAnnotation → KindDecl
// ---------------------------------------------------------------------------

/// Convert a Twig [`TypeAnnotation`] to a [`KindDecl`].
///
/// The LANG23 annotation vocabulary maps cleanly to `KindDecl`; opaque
/// TW05-A expressions are lowered via [`type_expr_to_kind`].
pub(crate) fn annotation_to_kind(ann: &TypeAnnotation) -> KindDecl {
    match ann {
        // LANG23 base kinds.
        TypeAnnotation::UnrefinedInt => KindDecl::Int,
        TypeAnnotation::UnrefinedBool => KindDecl::Bool,
        TypeAnnotation::Any => KindDecl::Any,
        // Range / membership annotations are still integer-kinded.
        TypeAnnotation::RangeInt { .. } => KindDecl::Int,
        TypeAnnotation::MembershipInt { .. } => KindDecl::Int,
        // TW05-A opaque type expression — delegate to the TypeExpr converter.
        TypeAnnotation::Opaque(te) => type_expr_to_kind(te),
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers — TypeExpr → KindDecl
// ---------------------------------------------------------------------------

/// Convert a Twig [`TypeExpr`] to a [`KindDecl`].
///
/// This is a best-effort conversion: complex type expressions (dependent
/// types, higher-kinded types) fall back to `KindDecl::Any`.  Refinements
/// are intentionally erased here — the IIR refinement pass handles them.
///
/// ## Mapping table
///
/// | TypeExpr                             | KindDecl                     |
/// |--------------------------------------|------------------------------|
/// | `Name("int")` / `Name("Int")`        | `Int`                        |
/// | `Name("bool")` / `Name("Bool")`      | `Bool`                       |
/// | `Name("str")` / `Name("Str")`        | `Str`                        |
/// | `Name("nil")`                        | `Nil`                        |
/// | `Name("any")` / `Name("Any")`        | `Any`                        |
/// | `Name("sym")` / `Name("Symbol")`     | `Symbol`                     |
/// | `Name(other)`                        | `Named(other)`               |
/// | `Int(_)`                             | `Int`                        |
/// | `List([Name("fn"), params, ...])`    | `Function { arity }`         |
/// | `List([Name("Int"), _, _])`          | `Int` (range form)           |
/// | `List([Name("Member"), _, _])`       | `Int` (membership form)      |
/// | anything else                        | `Any`                        |
fn type_expr_to_kind(te: &TypeExpr) -> KindDecl {
    match te {
        // ── Bare name → map standard Twig kind names. ─────────────────────
        TypeExpr::Name(n) => match n.as_str() {
            "int" | "Int" => KindDecl::Int,
            "bool" | "Bool" => KindDecl::Bool,
            "str" | "Str" | "string" | "String" => KindDecl::Str,
            "nil" | "Nil" => KindDecl::Nil,
            "any" | "Any" | "_" => KindDecl::Any,
            "sym" | "Symbol" | "symbol" => KindDecl::Symbol,
            // User-defined named type (record, union, or alias resolved later
            // by TypeDeclarations::resolve).
            other => KindDecl::Named(other.to_owned()),
        },

        // ── Integer literal in type position → Int.  ──────────────────────
        // (Rare, but valid in dependent types like `(Index 0)`.)
        TypeExpr::Int(_) => KindDecl::Int,

        // ── Parenthesised list: dispatch on the head element. ─────────────
        TypeExpr::List(parts) => match parts.as_slice() {
            // `(fn (params…) body)` — function type.  Arity is the number
            // of items in the parameter list (second element of the list).
            [TypeExpr::Name(kw), TypeExpr::List(params), _rest @ ..]
                if kw == "fn" =>
            {
                KindDecl::Function {
                    arity: params.len(),
                }
            }

            // `(Int lo hi)` — integer range annotation.
            [TypeExpr::Name(kw), _, _] if kw == "Int" => KindDecl::Int,

            // `(Member int (v0 v1 …))` — integer membership.
            [TypeExpr::Name(kw), TypeExpr::Name(base), _]
                if kw == "Member" && (base == "int" || base == "Int") =>
            {
                KindDecl::Int
            }

            // Head is a known bare name — rare, but forward-compatible.
            [TypeExpr::Name(n)] => type_expr_to_kind(&TypeExpr::Name(n.clone())),

            // Anything else (dependent types, parametric types, …) is
            // too complex to resolve statically here; fall back to Any.
            _ => KindDecl::Any,
        },
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers — Expr → KindDecl
// ---------------------------------------------------------------------------

/// Infer the [`KindDecl`] of a Twig [`Expr`] without a scope.
///
/// This is a shallow, structural inference used only to populate the
/// `globals` map.  It does not walk into subexpressions — the generic
/// `grammar-type-checker` handles that during the full type-check pass.
///
/// Lambdas contribute `Function { arity }` because the arity is always
/// statically known.  Everything else that can't be determined structurally
/// falls back to `Any` — the checker will refine it.
fn expr_to_kind(expr: &Expr) -> KindDecl {
    match expr {
        // ── Literals: directly known. ─────────────────────────────────────
        Expr::IntLit(_) => KindDecl::Int,
        Expr::BoolLit(_) => KindDecl::Bool,
        Expr::NilLit(_) => KindDecl::Nil,
        Expr::SymLit(_) => KindDecl::Symbol,

        // ── Lambda: arity known from parameter list. ─────────────────────
        //
        // Example: `(define (f x y) (+ x y))` desugars to
        // `(define f (lambda (x y) …))` — arity = 2.
        Expr::Lambda(l) => KindDecl::Function {
            arity: l.params.len(),
        },

        // ── Compound expressions: fall back to Any. ──────────────────────
        //
        // VarRef: the referenced name's kind is resolved during the check
        // pass, not here.  Apply/If/Let/Begin/Match: the return kind
        // depends on runtime values; grammar-type-checker infers it.
        Expr::VarRef(_)
        | Expr::Apply(_)
        | Expr::If(_)
        | Expr::Let(_)
        | Expr::Begin(_)
        | Expr::Match(_) => KindDecl::Any,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use type_declarations::{KindDecl, TypedModeDecl};

    // ── Basic: empty program ─────────────────────────────────────────────

    #[test]
    fn empty_program_empty_decls() {
        let p = parse("").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.language, "twig");
        assert!(d.named_types.is_empty());
        assert!(d.globals.is_empty());
        assert!(d.typed_mode.is_none());
    }

    // ── TypedMode from module declaration ────────────────────────────────

    #[test]
    fn module_strict_sets_typed_mode() {
        let p = parse("(module mylib (typed strict))").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.typed_mode, Some(TypedModeDecl::Strict));
    }

    #[test]
    fn module_lenient_sets_typed_mode() {
        let p = parse("(module mylib (typed lenient))").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.typed_mode, Some(TypedModeDecl::Lenient));
    }

    #[test]
    fn module_off_sets_typed_mode() {
        let p = parse("(module mylib (typed off))").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.typed_mode, Some(TypedModeDecl::Off));
    }

    #[test]
    fn no_module_typed_mode_is_none() {
        let p = parse("(define x 1)").unwrap();
        let d = emit_type_declarations(&p);
        assert!(d.typed_mode.is_none());
    }

    // ── type alias ───────────────────────────────────────────────────────

    #[test]
    fn type_alias_int() {
        let p = parse("(type Nat int)").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Nat").expect("Nat should be registered") {
            NamedTypeDecl::Alias { target } => assert_eq!(*target, KindDecl::Int),
            other => panic!("expected Alias, got {other:?}"),
        }
    }

    #[test]
    fn type_alias_bool() {
        let p = parse("(type Flag bool)").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Flag").expect("Flag") {
            NamedTypeDecl::Alias { target } => assert_eq!(*target, KindDecl::Bool),
            other => panic!("expected Alias, got {other:?}"),
        }
    }

    #[test]
    fn type_alias_range_int_is_int() {
        let p = parse("(type Byte (Int 0 256))").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Byte").expect("Byte") {
            NamedTypeDecl::Alias { target } => assert_eq!(*target, KindDecl::Int),
            other => panic!("expected Alias, got {other:?}"),
        }
    }

    #[test]
    fn type_alias_opaque_becomes_named() {
        let p = parse("(type MyToken Token)").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("MyToken").expect("MyToken") {
            NamedTypeDecl::Alias { target } => {
                assert_eq!(*target, KindDecl::Named("Token".to_owned()))
            }
            other => panic!("expected Alias, got {other:?}"),
        }
    }

    // ── record def ───────────────────────────────────────────────────────

    #[test]
    fn record_def_emits_record_type() {
        let p = parse("(record Point (x : int) (y : int))").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Point").expect("Point") {
            NamedTypeDecl::Record { fields } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "x");
                assert_eq!(fields[0].kind, KindDecl::Int);
                assert_eq!(fields[1].name, "y");
                assert_eq!(fields[1].kind, KindDecl::Int);
            }
            other => panic!("expected Record, got {other:?}"),
        }
    }

    #[test]
    fn record_def_zero_fields() {
        let p = parse("(record Unit)").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Unit").expect("Unit") {
            NamedTypeDecl::Record { fields } => assert!(fields.is_empty()),
            other => panic!("expected Record, got {other:?}"),
        }
    }

    // ── union def ────────────────────────────────────────────────────────

    #[test]
    fn union_def_emits_union_type() {
        let p = parse("(union Expr (IntLit (value : int)) (NameRef (name : any)))").unwrap();
        let d = emit_type_declarations(&p);
        match d.named_types.get("Expr").expect("Expr") {
            NamedTypeDecl::Union { variants } => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "IntLit");
                assert_eq!(variants[0].fields.len(), 1);
                assert_eq!(variants[0].fields[0].kind, KindDecl::Int);
                assert_eq!(variants[1].name, "NameRef");
                assert_eq!(variants[1].fields[0].kind, KindDecl::Any);
            }
            other => panic!("expected Union, got {other:?}"),
        }
    }

    #[test]
    fn union_variants_accessible_via_decls_helper() {
        let p = parse("(union Bool (True) (False))").unwrap();
        let d = emit_type_declarations(&p);
        let vs = d.union_variants("Bool").expect("Bool variants");
        assert_eq!(vs, vec!["True".to_owned(), "False".to_owned()]);
    }

    // ── define globals ───────────────────────────────────────────────────

    #[test]
    fn define_int_literal_kind_int() {
        let p = parse("(define x 42)").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["x"], KindDecl::Int);
    }

    #[test]
    fn define_bool_literal_kind_bool() {
        let p = parse("(define flag #t)").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["flag"], KindDecl::Bool);
    }

    #[test]
    fn define_lambda_kind_function() {
        let p = parse("(define (add x y) (+ x y))").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["add"], KindDecl::Function { arity: 2 });
    }

    #[test]
    fn define_with_int_annotation_overrides_expr() {
        // Even though the expr would be Any (it's a var ref), the annotation wins.
        let p = parse("(define x : int some-var)").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["x"], KindDecl::Int);
    }

    #[test]
    fn define_with_range_annotation_is_int() {
        let p = parse("(define n : (Int 0 128) 42)").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["n"], KindDecl::Int);
    }

    #[test]
    fn define_apply_expr_kind_any() {
        // (f 1 2) — return type unknown without full inference.
        let p = parse("(define z (f 1 2))").unwrap();
        let d = emit_type_declarations(&p);
        assert_eq!(d.globals["z"], KindDecl::Any);
    }

    // ── full program round-trip ───────────────────────────────────────────

    #[test]
    fn full_program_round_trip() {
        let src = r#"
          (module mylib (typed strict))
          (type Nat int)
          (record Point (x : int) (y : int))
          (union Shape (Circle (r : int)) (Rect (w : int) (h : int)))
          (define (distance p1 p2) 0)
          (define origin : Point nil)
        "#;
        let p = parse(src).unwrap();
        let d = emit_type_declarations(&p);

        assert_eq!(d.typed_mode, Some(TypedModeDecl::Strict));
        assert!(d.named_types.contains_key("Nat"));
        assert!(d.named_types.contains_key("Point"));
        assert!(d.named_types.contains_key("Shape"));
        assert_eq!(d.globals["distance"], KindDecl::Function { arity: 2 });
        // "origin" has annotation "Point" → Opaque(Name("Point")) → Named("Point")
        assert_eq!(d.globals["origin"], KindDecl::Named("Point".to_owned()));
    }
}
