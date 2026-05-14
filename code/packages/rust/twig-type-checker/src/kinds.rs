//! `TwigKind` — the base type system for Twig (TW05-B).
//!
//! ## What is a "kind"?
//!
//! In type theory, a *kind* is the type of a type.  Here we use the word
//! informally to mean "the coarse-grained shape of a value at runtime":
//! is it an integer?  A boolean?  A heap-allocated record?  An unknown `Any`?
//!
//! TW05-B only checks these **base kinds** — it doesn't verify that the
//! integer is in a specific range (that's TW05-C's job with the constraint
//! solver).  Think of it as the difference between C's `int` and a
//! refinement like `{x : int | 0 ≤ x < 256}`.
//!
//! ## The `Any` escape hatch
//!
//! `Any` is the top type: every value is an `Any`.  The checker produces
//! `Any` whenever it can't determine a more specific kind:
//!
//! - An unannotated value binding (`(define x some-expr)` with no annotation).
//! - A `VarRef` that doesn't resolve to anything in scope (in lenient mode —
//!   strict mode makes this an error).
//! - The return type of a function call (without a declared return type).
//!
//! `Any` never causes an error by itself.  It only becomes an issue in
//! `Strict` mode when it appears in an annotated position that requires a
//! more specific kind.
//!
//! ## Mapping from `TypeAnnotation`
//!
//! The parser produces `TypeAnnotation` values (LANG23 subset + opaque
//! TW05-A expressions).  The helpers below map them to `TwigKind`:
//!
//! | `TypeAnnotation` variant | `TwigKind` |
//! |--------------------------|------------|
//! | `UnrefinedInt`           | `Int`      |
//! | `RangeInt { lo, hi }`    | `Int`      |
//! | `MembershipInt { … }`    | `Int`      |
//! | `UnrefinedBool`          | `Bool`     |
//! | `Any`                    | `Any`      |
//! | `Opaque(TypeExpr)`       | resolved via `type_expr_to_kind` |

use crate::env::TypeEnv;
use twig_parser::{TypeAnnotation, TypeExpr};

// ---------------------------------------------------------------------------
// TwigKind
// ---------------------------------------------------------------------------

/// The base kind of a Twig value, as inferred by TW05-B.
///
/// # Ordering principle
///
/// Kinds are ordered from most specific to least specific.  Any operation
/// that combines two kinds (e.g. `if`-branch unification) widens to `Any`
/// if the two kinds don't match:
///
/// ```text
/// Int ──┐
/// Bool ─┤
/// Nil  ─┤
/// …    ─┼──► Any   (when two branches disagree)
/// …    ─┤
/// Any  ─┘
/// ```
///
/// This mirrors the Hindley-Milner style of "unify or widen", but without
/// a solver — we just report the mismatch and use `Any`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwigKind {
    /// An integer value.
    ///
    /// Produced by `IntLit`, `UnrefinedInt`, `RangeInt`, or `MembershipInt`
    /// annotations.  TW05-C will further refine this with bounds predicates.
    Int,

    /// A boolean value: `#t` or `#f`.
    Bool,

    /// The nil / empty-list sentinel.
    Nil,

    /// A quoted symbol (interned string atom): `'foo`, `'bar`.
    Symbol,

    /// A heap-allocated string object (LANG47 `String` type).
    Str,

    /// A homogeneous list of values.
    ///
    /// In TW05-B we don't track the element kind (that would require
    /// generics / parametric types, which are TW05-D territory).
    List,

    /// A named record type (product type).
    ///
    /// The `String` payload is the record name as declared by
    /// `(record Name …)`.  Two `Record`s are only equal if their names match.
    Record(String),

    /// A named union type (sum type).
    ///
    /// The `String` payload is the union name as declared by
    /// `(union Name …)`.  Two `Union`s are only equal if their names match.
    Union(String),

    /// A callable value with a statically-known parameter count.
    ///
    /// Produced by `Lambda` nodes and top-level `(define (f …) …)` forms.
    /// The arity is the number of *user-visible* parameters — capture
    /// variables from closures are not counted.
    Function {
        /// Number of formal parameters.
        arity: usize,
    },

    /// The widened top type — any value is an `Any`.
    ///
    /// Used when the kind cannot be statically determined, or when the
    /// source code explicitly annotates with `any`.
    Any,
}

impl TwigKind {
    /// A stable lowercase string representation, analogous to LSP token-type
    /// mnemonics.
    ///
    /// These strings are intended to be stable across versions — don't change
    /// them once published.
    pub fn mnemonic(&self) -> &'static str {
        match self {
            TwigKind::Int => "int",
            TwigKind::Bool => "bool",
            TwigKind::Nil => "nil",
            TwigKind::Symbol => "symbol",
            TwigKind::Str => "string",
            TwigKind::List => "list",
            TwigKind::Record(_) => "record",
            TwigKind::Union(_) => "union",
            TwigKind::Function { .. } => "function",
            TwigKind::Any => "any",
        }
    }

    /// Unify two kinds: return the shared kind if equal, `Any` if not.
    ///
    /// This is used to find the result kind of an `if` expression:
    ///
    /// ```text
    /// (if cond 1 2)    → then:Int, else:Int  → unified:Int
    /// (if cond 1 #t)   → then:Int, else:Bool → unified:Any
    /// (if cond x nil)  → then:Any, else:Nil  → unified:Any
    /// ```
    pub fn unify(a: TwigKind, b: TwigKind) -> TwigKind {
        if a == b {
            a
        } else {
            TwigKind::Any
        }
    }
}

impl std::fmt::Display for TwigKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigKind::Record(name) => write!(f, "record:{name}"),
            TwigKind::Union(name) => write!(f, "union:{name}"),
            TwigKind::Function { arity } => write!(f, "function/{arity}"),
            other => f.write_str(other.mnemonic()),
        }
    }
}

// ---------------------------------------------------------------------------
// TypeAnnotation → TwigKind
// ---------------------------------------------------------------------------

/// Map a parsed [`TypeAnnotation`] to a [`TwigKind`].
///
/// This is a one-directional, lossy conversion: range and membership
/// predicates are collapsed to the base `Int` kind.  TW05-C recovers the
/// predicate from the original `TypeAnnotation` to feed into the constraint
/// solver.
///
/// # Examples
///
/// ```text
/// UnrefinedInt              → Int
/// RangeInt { lo: 0, hi: 256 } → Int   (predicate stripped — TW05-C's job)
/// UnrefinedBool             → Bool
/// Any                       → Any
/// Opaque(Name("Symbol"))    → Symbol  (via type_expr_to_kind)
/// ```
pub fn type_annotation_to_kind(ann: &TypeAnnotation, env: &TypeEnv) -> TwigKind {
    match ann {
        TypeAnnotation::UnrefinedInt
        | TypeAnnotation::RangeInt { .. }
        | TypeAnnotation::MembershipInt { .. } => TwigKind::Int,
        TypeAnnotation::UnrefinedBool => TwigKind::Bool,
        TypeAnnotation::Any => TwigKind::Any,
        TypeAnnotation::Opaque(te) => type_expr_to_kind(te, env),
    }
}

/// Map a [`TypeExpr`] tree to a [`TwigKind`].
///
/// ## Resolution order for `TypeExpr::Name(s)`
///
/// 1. Well-known primitive names (`"Int"`, `"Bool"`, etc.) — see table below.
/// 2. Names declared as `RecordDef` in `env.records`.
/// 3. Names declared as `UnionDef` in `env.unions`.
/// 4. Names declared as `TypeAlias` in `env.aliases` — resolved one level.
/// 5. Unknown name → `Any`.
///
/// ## Well-known names (case-sensitive)
///
/// | Name | Kind |
/// |------|------|
/// | `Int` | `Int` |
/// | `int` | `Int` |
/// | `Bool` | `Bool` |
/// | `bool` | `Bool` |
/// | `Nil` | `Nil` |
/// | `nil` | `Nil` |
/// | `Symbol` | `Symbol` |
/// | `symbol` | `Symbol` |
/// | `String` | `Str` |
/// | `string` | `Str` |
/// | `List` | `List` |
/// | `list` | `List` |
/// | `Any` | `Any` |
/// | `any` | `Any` |
///
/// ## `TypeExpr::List` head-dispatch
///
/// A parenthesised list `(Head ...)` uses the head name to determine the kind:
///
/// | Head | Kind |
/// |------|------|
/// | `Int` / `int` | `Int` |
/// | `Bool` / `bool` | `Bool` |
/// | `List` / `list` | `List` |
/// | `fn` | `Function { arity: 0 }` (arity unknown at type level in TW05-B) |
/// | `Member` | `Int` (membership integer annotation) |
/// | anything else | `Any` |
///
/// ## Depth guard
///
/// Recursive type aliases like `(type Foo Foo)` would loop forever.  The
/// `depth` counter stops recursion after 32 levels and returns `Any`.
pub fn type_expr_to_kind(te: &TypeExpr, env: &TypeEnv) -> TwigKind {
    type_expr_to_kind_depth(te, env, 0)
}

fn type_expr_to_kind_depth(te: &TypeExpr, env: &TypeEnv, depth: usize) -> TwigKind {
    // Depth guard: prevent infinite recursion on cyclic alias definitions.
    if depth > 32 {
        return TwigKind::Any;
    }

    match te {
        TypeExpr::Int(_) => TwigKind::Int,

        TypeExpr::Name(s) => resolve_name_kind(s, env, depth),

        TypeExpr::List(items) => {
            // The head of the list names the type constructor.
            match items.first() {
                Some(TypeExpr::Name(head)) => match head.as_str() {
                    "Int" | "int" => TwigKind::Int,
                    "Bool" | "bool" => TwigKind::Bool,
                    "List" | "list" => TwigKind::List,
                    "fn" => TwigKind::Function { arity: 0 },
                    "Member" | "member" => TwigKind::Int,
                    _ => TwigKind::Any,
                },
                _ => TwigKind::Any,
            }
        }
    }
}

/// Resolve a bare `Name` string to a `TwigKind`, consulting the type env.
fn resolve_name_kind(s: &str, env: &TypeEnv, depth: usize) -> TwigKind {
    match s {
        "Int" | "int" => TwigKind::Int,
        "Bool" | "bool" => TwigKind::Bool,
        "Nil" | "nil" => TwigKind::Nil,
        "Symbol" | "symbol" => TwigKind::Symbol,
        "String" | "string" => TwigKind::Str,
        "List" | "list" => TwigKind::List,
        "Any" | "any" => TwigKind::Any,
        _ => {
            // Is it a record type?
            if env.records.contains_key(s) {
                return TwigKind::Record(s.to_owned());
            }
            // Is it a union type?
            if env.unions.contains_key(s) {
                return TwigKind::Union(s.to_owned());
            }
            // Is it a type alias?
            if let Some(alias_expr) = env.aliases.get(s) {
                // Clone to avoid holding the borrow across the recursive call.
                let alias_clone = alias_expr.clone();
                return type_expr_to_kind_depth(&alias_clone, env, depth + 1);
            }
            // Unknown name — widen to Any.
            TwigKind::Any
        }
    }
}
