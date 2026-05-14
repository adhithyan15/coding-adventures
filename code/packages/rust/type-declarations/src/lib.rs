//! # `type-declarations`
//!
//! Language-agnostic type declaration format — analogous to TypeScript's
//! `.d.ts` files.  Any parser can emit a `TypeDeclarations` alongside its
//! language-specific AST; the [`grammar-type-checker`] crate consumes it to
//! drive generic type inference over raw `GrammarASTNode` trees.
//!
//! ## Design philosophy
//!
//! Types are not ornamental.  A [`KindDecl::Int`] annotation on an expression
//! must propagate all the way into `type_hint = "i64"` on the IIR instruction
//! that computes it.  The JIT/AOT specialisers in this codebase prioritise
//! `type_hint` over runtime profiling; fully-typed IIR reaches AOT-quality
//! code generation with zero warmup cost.
//!
//! [`KindDecl::to_iir_hint`] provides the mapping from semantic kinds to the
//! IIR string constants understood by `jit-core` and `aot-core`.
//!
//! ## The `.d.ts` analogy
//!
//! TypeScript `.d.ts` files declare the *shape* of types without carrying
//! implementation code.  `TypeDeclarations` is the same idea:
//!
//! - **Source** (e.g., `foo.twig`) is parsed into a `GrammarASTNode` tree.
//! - **Declaration side-output** (`TypeDeclarations`) carries everything the
//!   checker needs to know: named types (records, unions, aliases), global
//!   binding kinds, and the module's typed-mode enforcement setting.
//! - The **generic checker** reads declarations + raw tree → `AnnotatedNode`
//!   (every node tagged with its inferred `KindDecl`).
//!
//! ## Crate dependency
//!
//! Zero dependencies — this crate is pure data.  No parser types, no runtime
//! types.  Any crate in the stack can depend on it without pulling in the
//! whole grammar machinery.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// TypeDeclarations — the top-level container
// ---------------------------------------------------------------------------

/// Language-agnostic collection of type declarations emitted by a parser.
///
/// Call [`TypeDeclarations::new`] to create an empty instance, populate it
/// during a declaration-collection pass over the source AST, then hand it to
/// the `grammar-type-checker`.
///
/// # Example (Twig)
///
/// A `twig-parser` call to `emit_type_declarations(&program)` produces:
///
/// ```text
/// TypeDeclarations {
///     language: "twig",
///     named_types: {
///         "Point" => Record { fields: [FieldDecl { name: "x", kind: Int },
///                                      FieldDecl { name: "y", kind: Int }] }
///     },
///     globals: {
///         "distance" => Function { arity: 2 },
///         "origin"   => Named("Point"),
///     },
///     typed_mode: Some(TypedModeDecl::Strict),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TypeDeclarations {
    /// Language identifier (e.g., `"twig"`, `"ruby"`, `"lua"`).
    pub language: String,

    /// Named type definitions indexed by name.
    ///
    /// Keys match the names used in [`KindDecl::Named`] so that alias chains
    /// and structural types can be resolved via [`TypeDeclarations::resolve`].
    pub named_types: HashMap<String, NamedTypeDecl>,

    /// Top-level global binding kinds indexed by name.
    ///
    /// The generic checker seeds its scope stack from this map before
    /// walking any expressions, so all top-level defines are visible
    /// throughout the program body (forward references work).
    pub globals: HashMap<String, KindDecl>,

    /// Typed-mode enforcement, parsed from the source module declaration.
    ///
    /// `None` is equivalent to `TypedModeDecl::Off` — the checker still
    /// builds the [`AnnotatedNode`] tree but emits no type errors.
    pub typed_mode: Option<TypedModeDecl>,
}

impl TypeDeclarations {
    /// Create an empty `TypeDeclarations` for the given language.
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            named_types: HashMap::new(),
            globals: HashMap::new(),
            typed_mode: None,
        }
    }

    /// Resolve a [`KindDecl`] through alias chains, depth-limited to 32.
    ///
    /// If `kind` is [`KindDecl::Named`] and the name maps to a
    /// [`NamedTypeDecl::Alias`], this function follows the chain until it
    /// reaches a non-alias or the depth limit.  Returns [`KindDecl::Any`]
    /// on cycles to prevent infinite loops.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_declarations::{TypeDeclarations, NamedTypeDecl, KindDecl};
    ///
    /// let mut d = TypeDeclarations::new("twig");
    /// d.named_types.insert(
    ///     "Nat".to_owned(),
    ///     NamedTypeDecl::Alias { target: KindDecl::Int },
    /// );
    /// assert_eq!(d.resolve(&KindDecl::Named("Nat".to_owned())), KindDecl::Int);
    /// assert_eq!(d.resolve(&KindDecl::Int), KindDecl::Int);
    /// ```
    pub fn resolve(&self, kind: &KindDecl) -> KindDecl {
        self.resolve_depth(kind, 0)
    }

    fn resolve_depth(&self, kind: &KindDecl, depth: usize) -> KindDecl {
        // Guard against alias cycles (e.g., type A = A).
        if depth > 32 {
            return KindDecl::Any;
        }
        match kind {
            KindDecl::Named(name) => match self.named_types.get(name) {
                Some(NamedTypeDecl::Alias { target }) => {
                    self.resolve_depth(target, depth + 1)
                }
                _ => kind.clone(),
            },
            other => other.clone(),
        }
    }

    /// Return the variant names of a named union type, or `None` if the
    /// name does not refer to a union.
    ///
    /// Used by the exhaustiveness checker in `grammar-type-checker`.
    ///
    /// # Example
    ///
    /// ```
    /// use type_declarations::{TypeDeclarations, NamedTypeDecl, VariantDecl, KindDecl};
    ///
    /// let mut d = TypeDeclarations::new("twig");
    /// d.named_types.insert("Shape".to_owned(), NamedTypeDecl::Union {
    ///     variants: vec![
    ///         VariantDecl { name: "Circle".to_owned(), fields: vec![] },
    ///         VariantDecl { name: "Rect".to_owned(),   fields: vec![] },
    ///     ],
    /// });
    /// assert_eq!(
    ///     d.union_variants("Shape"),
    ///     Some(vec!["Circle".to_owned(), "Rect".to_owned()])
    /// );
    /// assert!(d.union_variants("Nat").is_none());
    /// ```
    pub fn union_variants(&self, name: &str) -> Option<Vec<String>> {
        match self.named_types.get(name)? {
            NamedTypeDecl::Union { variants } => {
                Some(variants.iter().map(|v| v.name.clone()).collect())
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// TypedModeDecl
// ---------------------------------------------------------------------------

/// Enforcement mode, parsed from the source module's `(typed …)` clause.
///
/// | Clause | Behaviour |
/// |--------|-----------|
/// | `Off` or absent | Build annotated tree; emit no type errors |
/// | `Lenient` | Emit errors; `ok: true` regardless |
/// | `Strict` | Emit errors; `ok: errors.is_empty()` |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedModeDecl {
    /// Type-check results are advisory only; compilation always proceeds.
    Off,
    /// Type errors are warnings — `TypeCheckResult::ok` is always `true`.
    Lenient,
    /// Type errors block compilation — `TypeCheckResult::ok` is `false`.
    Strict,
}

// ---------------------------------------------------------------------------
// NamedTypeDecl
// ---------------------------------------------------------------------------

/// A named type declaration — record, union, or alias.
#[derive(Debug, Clone)]
pub enum NamedTypeDecl {
    /// Product type: all fields are present simultaneously.
    ///
    /// ```text
    /// (record Point (x : int) (y : int))
    /// ```
    Record {
        /// Ordered list of fields (order determines constructor arity).
        fields: Vec<FieldDecl>,
    },

    /// Sum type: exactly one variant is active at runtime.
    ///
    /// ```text
    /// (union Shape (Circle (r : int)) (Rect (w : int) (h : int)))
    /// ```
    Union {
        /// Variants in declaration order (index = runtime integer tag).
        variants: Vec<VariantDecl>,
    },

    /// Compile-time alias that resolves to another kind.
    ///
    /// ```text
    /// (type Nat int)   ;; Alias { target: KindDecl::Int }
    /// ```
    Alias {
        /// The kind this alias expands to.
        target: KindDecl,
    },
}

// ---------------------------------------------------------------------------
// FieldDecl / VariantDecl
// ---------------------------------------------------------------------------

/// A single field in a record or union variant.
#[derive(Debug, Clone)]
pub struct FieldDecl {
    /// Field name as written in the source.
    pub name: String,
    /// Base kind of the field's value.
    pub kind: KindDecl,
}

/// One variant of a tagged union.
#[derive(Debug, Clone)]
pub struct VariantDecl {
    /// Variant constructor name (e.g., `"Circle"`).
    pub name: String,
    /// Fields of this variant, in declaration order.
    pub fields: Vec<FieldDecl>,
}

// ---------------------------------------------------------------------------
// KindDecl — the base type level value
// ---------------------------------------------------------------------------

/// The base kind inferred for every expression during type checking.
///
/// `KindDecl` is intentionally coarse — it's the *language-of-types* level,
/// not a full refinement system.  Refinements (e.g., `0 ≤ x < 128`) are
/// stored separately in the `lang-refined-types` crate and propagated by the
/// `iir-refinement-pass` in TW05-C.
///
/// ## IIR integration
///
/// Every `KindDecl` maps to an IIR `type_hint` string via
/// [`KindDecl::to_iir_hint`].  The JIT and AOT specialisers read
/// `type_hint` *before* consulting runtime profiles:
///
/// ```text
/// KindDecl::Int    → "i64"     (native 64-bit int ops, no boxing)
/// KindDecl::Bool   → "bool"    (direct branch, no type guard)
/// KindDecl::Str    → "str"     (string fast-path)
/// Function{n}      → "closure" (direct closure ops)
/// _                → "any"     (generic, falls back to profiling)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindDecl {
    /// 64-bit signed integer (Twig's only numeric type in V1).
    Int,
    /// Boolean (`#t` / `#f`).
    Bool,
    /// The empty list / null value.
    Nil,
    /// An interned symbol (quoted identifier).
    Symbol,
    /// A heap-allocated string.
    Str,
    /// A cons-cell linked list.
    List,
    /// A named type — look up in [`TypeDeclarations::named_types`].
    Named(String),
    /// A closure value of known arity.
    Function {
        /// Number of parameters the function accepts.
        arity: usize,
    },
    /// Widened / unknown kind — compatible with any other kind.
    ///
    /// Used when type information is absent or cannot be inferred.
    /// Maps to `type_hint = "any"` in IIR (falls back to runtime profiling).
    Any,
}

impl KindDecl {
    /// Return the IIR `type_hint` string for this kind.
    ///
    /// The returned string is understood by `jit-core::specialise` and
    /// `aot-core::specialise`.  The mapping is:
    ///
    /// | `KindDecl` | `type_hint` | Effect in JIT/AOT |
    /// |------------|-------------|-------------------|
    /// | `Int` | `"i64"` | 64-bit int ops, no boxing |
    /// | `Bool` | `"bool"` | Branch directly, no type guard |
    /// | `Str` | `"str"` | String fast-path |
    /// | `Function{..}` | `"closure"` | Direct closure dispatch |
    /// | everything else | `"any"` | Generic path, runtime profiling |
    ///
    /// # Example
    ///
    /// ```
    /// use type_declarations::KindDecl;
    ///
    /// assert_eq!(KindDecl::Int.to_iir_hint(), "i64");
    /// assert_eq!(KindDecl::Bool.to_iir_hint(), "bool");
    /// assert_eq!(KindDecl::Function { arity: 2 }.to_iir_hint(), "closure");
    /// assert_eq!(KindDecl::Any.to_iir_hint(), "any");
    /// assert_eq!(KindDecl::Nil.to_iir_hint(), "any");
    /// ```
    pub fn to_iir_hint(&self) -> &'static str {
        match self {
            KindDecl::Int => "i64",
            KindDecl::Bool => "bool",
            KindDecl::Str => "str",
            KindDecl::Function { .. } => "closure",
            // Nil, Symbol, List, Named, Any — all fall back to the generic path.
            _ => "any",
        }
    }

    /// True if this kind maps to a concrete (non-"any") IIR type hint.
    ///
    /// Used by `twig-ir-compiler` to determine [`FunctionTypeStatus`].
    pub fn is_concrete_hint(&self) -> bool {
        self.to_iir_hint() != "any"
    }
}

// ---------------------------------------------------------------------------
// AnnotatedNode — the central compilation artifact
// ---------------------------------------------------------------------------

/// A `GrammarASTNode`-shaped tree where every node carries its inferred
/// [`KindDecl`].
///
/// This is the central compilation artifact produced by the
/// `grammar-type-checker`.  It flows from type checking into IIR emission:
///
/// ```text
/// GrammarASTNode
///     │  grammar_type_checker::check()
///     ▼
/// AnnotatedNode   (kind on every node)
///     │  twig_ir_compiler::compile_annotated()
///     ▼
/// IIRModule   (type_hint = annotated_node.iir_hint() on each instruction)
///     │  jit-core / aot-core
///     ▼
/// typed machine code  (zero profiling wait for fully-typed functions)
/// ```
///
/// ## Relation to `GrammarASTNode`
///
/// `AnnotatedNode` mirrors the structure of `parser::GrammarASTNode` but is
/// a separate type so that:
/// - The `type-declarations` crate doesn't depend on `parser`.
/// - Callers can store annotations without keeping the raw tree alive.
///
/// Children that were raw `Token`s in the grammar tree are represented as
/// [`AnnotatedChild::Token`] (text + position only; we never need to type a
/// raw token in isolation).
#[derive(Debug, Clone)]
pub struct AnnotatedNode {
    /// Grammar rule name of this node (e.g., `"apply"`, `"atom"`, `"if_form"`).
    pub rule_name: String,

    /// The inferred kind for the *value* this node produces at runtime.
    ///
    /// After `grammar-type-checker::check()` the root's kind is generally
    /// `Any` (a program is a sequence, not a single value).  Leaf expressions
    /// carry the interesting kinds: `Int`, `Bool`, `Function{n}`, etc.
    pub kind: KindDecl,

    /// Annotated children.
    pub children: Vec<AnnotatedChild>,

    /// Source position — populated from the grammar AST.
    pub start_line: Option<usize>,
    /// Source position — populated from the grammar AST.
    pub start_column: Option<usize>,
    /// Source position — populated from the grammar AST.
    pub end_line: Option<usize>,
    /// Source position — populated from the grammar AST.
    pub end_column: Option<usize>,
}

impl AnnotatedNode {
    /// Return the IIR `type_hint` string for the value this node produces.
    ///
    /// Shorthand for `self.kind.to_iir_hint()`.
    pub fn iir_hint(&self) -> &'static str {
        self.kind.to_iir_hint()
    }

    /// Find the first child [`AnnotatedNode`] with the given rule name.
    ///
    /// Useful for the IIR compiler to locate specific sub-trees without
    /// fully re-implementing the grammar's structure.
    pub fn child_node(&self, rule: &str) -> Option<&AnnotatedNode> {
        self.children.iter().find_map(|c| match c {
            AnnotatedChild::Node(n) if n.rule_name == rule => Some(n),
            _ => None,
        })
    }

    /// Collect all immediate annotated child nodes (excluding token leaves).
    pub fn node_children(&self) -> Vec<&AnnotatedNode> {
        self.children
            .iter()
            .filter_map(|c| match c {
                AnnotatedChild::Node(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Source position as `(line, column)`, falling back to `(0, 0)`.
    pub fn position(&self) -> (usize, usize) {
        (
            self.start_line.unwrap_or(0),
            self.start_column.unwrap_or(0),
        )
    }
}

/// One child of an [`AnnotatedNode`] — either a nested annotated sub-tree or
/// a raw token leaf.
#[derive(Debug, Clone)]
pub enum AnnotatedChild {
    /// A nested grammar rule that was inferred.
    Node(AnnotatedNode),
    /// A raw token — punctuation, keywords, or literals that carry no
    /// sub-structure.
    Token {
        /// The token's text (e.g., `"42"`, `"define"`, `"("`).
        text: String,
        /// Source line (1-indexed).
        line: usize,
        /// Source column (1-indexed).
        column: usize,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── KindDecl::to_iir_hint ────────────────────────────────────────────

    #[test]
    fn kind_to_iir_hint_int() {
        assert_eq!(KindDecl::Int.to_iir_hint(), "i64");
    }

    #[test]
    fn kind_to_iir_hint_bool() {
        assert_eq!(KindDecl::Bool.to_iir_hint(), "bool");
    }

    #[test]
    fn kind_to_iir_hint_str() {
        assert_eq!(KindDecl::Str.to_iir_hint(), "str");
    }

    #[test]
    fn kind_to_iir_hint_closure() {
        assert_eq!(KindDecl::Function { arity: 1 }.to_iir_hint(), "closure");
        assert_eq!(KindDecl::Function { arity: 0 }.to_iir_hint(), "closure");
    }

    #[test]
    fn kind_to_iir_hint_any_variants() {
        // All non-concrete kinds map to "any"
        assert_eq!(KindDecl::Any.to_iir_hint(), "any");
        assert_eq!(KindDecl::Nil.to_iir_hint(), "any");
        assert_eq!(KindDecl::Symbol.to_iir_hint(), "any");
        assert_eq!(KindDecl::List.to_iir_hint(), "any");
        assert_eq!(KindDecl::Named("Foo".to_owned()).to_iir_hint(), "any");
    }

    #[test]
    fn kind_is_concrete_hint() {
        assert!(KindDecl::Int.is_concrete_hint());
        assert!(KindDecl::Bool.is_concrete_hint());
        assert!(KindDecl::Function { arity: 2 }.is_concrete_hint());
        assert!(!KindDecl::Any.is_concrete_hint());
        assert!(!KindDecl::Nil.is_concrete_hint());
    }

    // ── AnnotatedNode::iir_hint ──────────────────────────────────────────

    #[test]
    fn annotated_node_iir_hint() {
        let node = AnnotatedNode {
            rule_name: "atom".to_owned(),
            kind: KindDecl::Int,
            children: vec![],
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(2),
        };
        assert_eq!(node.iir_hint(), "i64");
    }

    // ── TypeDeclarations::resolve ────────────────────────────────────────

    #[test]
    fn resolve_alias_chain() {
        let mut d = TypeDeclarations::new("twig");
        // Nat → Int
        d.named_types.insert(
            "Nat".to_owned(),
            NamedTypeDecl::Alias {
                target: KindDecl::Int,
            },
        );
        assert_eq!(d.resolve(&KindDecl::Named("Nat".to_owned())), KindDecl::Int);
        // Non-Named kinds pass through unchanged
        assert_eq!(d.resolve(&KindDecl::Bool), KindDecl::Bool);
    }

    #[test]
    fn resolve_alias_cycle_returns_any() {
        let mut d = TypeDeclarations::new("twig");
        // A → Named("A") — direct cycle
        d.named_types.insert(
            "A".to_owned(),
            NamedTypeDecl::Alias {
                target: KindDecl::Named("A".to_owned()),
            },
        );
        // Should not loop forever; depth guard kicks in and returns Any
        assert_eq!(d.resolve(&KindDecl::Named("A".to_owned())), KindDecl::Any);
    }

    #[test]
    fn resolve_named_record_stays_named() {
        let mut d = TypeDeclarations::new("twig");
        d.named_types.insert(
            "Point".to_owned(),
            NamedTypeDecl::Record { fields: vec![] },
        );
        // Records are not aliases — Named("Point") stays as-is
        assert_eq!(
            d.resolve(&KindDecl::Named("Point".to_owned())),
            KindDecl::Named("Point".to_owned())
        );
    }

    // ── TypeDeclarations::union_variants ─────────────────────────────────

    #[test]
    fn union_variants_lookup() {
        let mut d = TypeDeclarations::new("twig");
        d.named_types.insert(
            "Shape".to_owned(),
            NamedTypeDecl::Union {
                variants: vec![
                    VariantDecl {
                        name: "Circle".to_owned(),
                        fields: vec![],
                    },
                    VariantDecl {
                        name: "Rect".to_owned(),
                        fields: vec![],
                    },
                ],
            },
        );
        let vs = d.union_variants("Shape").unwrap();
        assert_eq!(vs, vec!["Circle", "Rect"]);
        assert!(d.union_variants("Unknown").is_none());
    }

    // ── TypeDeclarations baseline ─────────────────────────────────────────

    #[test]
    fn type_declarations_new_is_empty() {
        let d = TypeDeclarations::new("twig");
        assert_eq!(d.language, "twig");
        assert!(d.named_types.is_empty());
        assert!(d.globals.is_empty());
        assert!(d.typed_mode.is_none());
    }

    // ── AnnotatedNode helpers ─────────────────────────────────────────────

    #[test]
    fn annotated_node_child_node_lookup() {
        let child = AnnotatedNode {
            rule_name: "atom".to_owned(),
            kind: KindDecl::Int,
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        let root = AnnotatedNode {
            rule_name: "expr".to_owned(),
            kind: KindDecl::Int,
            children: vec![AnnotatedChild::Node(child)],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        assert!(root.child_node("atom").is_some());
        assert!(root.child_node("compound").is_none());
    }

    #[test]
    fn annotated_node_position_fallback() {
        let n = AnnotatedNode {
            rule_name: "x".to_owned(),
            kind: KindDecl::Any,
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        assert_eq!(n.position(), (0, 0));
    }
}
