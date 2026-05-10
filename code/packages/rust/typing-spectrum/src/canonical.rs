//! # Canonical type-name constants and per-language mapping tables.
//!
//! LANG22 §"Type ascription syntax (per language)" defines a universal
//! vocabulary of type-name strings that every LANG pipeline backend agrees on.
//! Language frontends lower their local type annotations to these strings
//! before storing them in `IIRInstr::type_hint`.
//!
//! ## Vocabulary
//!
//! ```text
//! i8 i16 i32 i64          — signed integers
//! u8 u16 u32 u64          — unsigned integers
//! f32 f64                 — IEEE 754 floats
//! bool                    — boolean
//! nil                     — the null / absent value
//! symbol                  — interned identifier (Lisp atom, Ruby Symbol)
//! str                     — UTF-8 string
//! closure                 — first-class callable
//! cons                    — Lisp-style pair / cons cell
//! any                     — dynamic / unknown type (sentinel)
//! <binding>::<class-name> — language-specific class (e.g. "lispy::cons")
//! ```
//!
//! ## Mapping table (per language)
//!
//! [`map_frontend_type`] takes a language-specific type annotation
//! string and returns the canonical IIR type string.  The mapping table
//! is derived from LANG22 §"Type ascription syntax (per language)".
//!
//! ```
//! use typing_spectrum::canonical::map_frontend_type;
//!
//! // Twig / Tetrad
//! assert_eq!(map_frontend_type("int", "twig"), Some("i64"));
//! assert_eq!(map_frontend_type("bool", "twig"), Some("bool"));
//!
//! // TypeScript
//! assert_eq!(map_frontend_type("number", "typescript"), Some("f64"));
//! assert_eq!(map_frontend_type("string", "typescript"), Some("str"));
//!
//! // Unknown type stays None — caller emits "any"
//! assert_eq!(map_frontend_type("unknown_type_xyz", "twig"), None);
//! ```

// ---------------------------------------------------------------------------
// Canonical type constants
// ---------------------------------------------------------------------------

/// Signed 8-bit integer.
pub const TYPE_I8: &str = "i8";
/// Signed 16-bit integer.
pub const TYPE_I16: &str = "i16";
/// Signed 32-bit integer.
pub const TYPE_I32: &str = "i32";
/// Signed 64-bit integer (the default integer type for most languages).
pub const TYPE_I64: &str = "i64";

/// Unsigned 8-bit integer.
pub const TYPE_U8: &str = "u8";
/// Unsigned 16-bit integer.
pub const TYPE_U16: &str = "u16";
/// Unsigned 32-bit integer.
pub const TYPE_U32: &str = "u32";
/// Unsigned 64-bit integer.
pub const TYPE_U64: &str = "u64";

/// 32-bit IEEE 754 float.
pub const TYPE_F32: &str = "f32";
/// 64-bit IEEE 754 double (TypeScript's `number`, Java's `double`).
pub const TYPE_F64: &str = "f64";

/// Boolean (`true` / `false`).
pub const TYPE_BOOL: &str = "bool";

/// The null / nil / absent value (Lisp's `nil`, Ruby's `nil`, etc.).
pub const TYPE_NIL: &str = "nil";

/// An interned symbol / identifier (Lisp atom, Ruby `Symbol`, etc.).
pub const TYPE_SYMBOL: &str = "symbol";

/// A UTF-8 string.
pub const TYPE_STR: &str = "str";

/// A first-class callable (lambda, closure, Proc, etc.).
pub const TYPE_CLOSURE: &str = "closure";

/// A Lisp-style pair / cons cell.
pub const TYPE_CONS: &str = "cons";

/// The dynamic / unknown type.  Instructions carrying this `type_hint`
/// are profiled at runtime and specialised by the JIT.
pub const TYPE_ANY: &str = "any";

/// All canonical numeric types in declaration order.  Useful for
/// generating conversion tables or validator allow-lists.
pub const NUMERIC_TYPES: &[&str] = &[
    TYPE_I8, TYPE_I16, TYPE_I32, TYPE_I64,
    TYPE_U8, TYPE_U16, TYPE_U32, TYPE_U64,
    TYPE_F32, TYPE_F64,
];

/// All non-numeric primitive types.
pub const PRIMITIVE_TYPES: &[&str] = &[TYPE_BOOL, TYPE_NIL, TYPE_SYMBOL, TYPE_STR];

/// All canonical types (numeric + primitive + structural).
pub const ALL_CANONICAL_TYPES: &[&str] = &[
    TYPE_I8, TYPE_I16, TYPE_I32, TYPE_I64,
    TYPE_U8, TYPE_U16, TYPE_U32, TYPE_U64,
    TYPE_F32, TYPE_F64,
    TYPE_BOOL, TYPE_NIL, TYPE_SYMBOL, TYPE_STR,
    TYPE_CLOSURE, TYPE_CONS,
    // TYPE_ANY is deliberately excluded: it is the *absence* of a type.
];

// ---------------------------------------------------------------------------
// Per-language mapping table
// ---------------------------------------------------------------------------

/// Map a language-specific type annotation to the canonical IIR type string.
///
/// Returns `None` if the frontend type is not recognised — the caller should
/// then emit `"any"` and let the profiler fill it in at runtime.
///
/// The mapping follows LANG22 §"Type ascription syntax (per language)":
///
/// | Language | Frontend string | Canonical |
/// |----------|----------------|-----------|
/// | Twig / Tetrad | `"int"` | `"i64"` |
/// | Twig / Tetrad | `"float"` | `"f64"` |
/// | Twig / Tetrad | `"bool"` | `"bool"` |
/// | Twig / Tetrad | `"string"` / `"str"` | `"str"` |
/// | TypeScript | `"number"` | `"f64"` |
/// | TypeScript | `"boolean"` | `"bool"` |
/// | TypeScript | `"string"` | `"str"` |
/// | TypeScript | `"null"` / `"undefined"` | `"nil"` |
/// | Ruby / Sorbet | `"Integer"` | `"i64"` |
/// | Ruby / Sorbet | `"Float"` | `"f64"` |
/// | Ruby / Sorbet | `"TrueClass"` / `"FalseClass"` | `"bool"` |
/// | Ruby / Sorbet | `"String"` | `"str"` |
/// | Ruby / Sorbet | `"Symbol"` | `"symbol"` |
/// | Ruby / Sorbet | `"NilClass"` | `"nil"` |
/// | Hack | `"int"` | `"i64"` |
/// | Hack | `"float"` | `"f64"` |
/// | Hack | `"bool"` | `"bool"` |
/// | Hack | `"string"` | `"str"` |
/// | Mypy / Python | `"int"` | `"i64"` |
/// | Mypy / Python | `"float"` | `"f64"` |
/// | Mypy / Python | `"bool"` | `"bool"` |
/// | Mypy / Python | `"str"` | `"str"` |
/// | Mypy / Python | `"None"` | `"nil"` |
/// | Rust / C+ | `"i8"` … `"u64"`, `"f32"`, `"f64"`, `"bool"` | pass-through |
///
/// All canonical type-name strings (those already in `ALL_CANONICAL_TYPES`)
/// are accepted as-is regardless of the `language` parameter — they need
/// no translation.
///
/// ```
/// use typing_spectrum::canonical::map_frontend_type;
///
/// // Canonical pass-through
/// assert_eq!(map_frontend_type("i64", "any"), Some("i64"));
/// assert_eq!(map_frontend_type("bool", "any"), Some("bool"));
///
/// // Twig
/// assert_eq!(map_frontend_type("int", "twig"), Some("i64"));
/// assert_eq!(map_frontend_type("float", "twig"), Some("f64"));
///
/// // TypeScript
/// assert_eq!(map_frontend_type("number", "typescript"), Some("f64"));
/// assert_eq!(map_frontend_type("null", "typescript"), Some("nil"));
///
/// // Ruby / Sorbet
/// assert_eq!(map_frontend_type("Integer", "ruby"), Some("i64"));
/// assert_eq!(map_frontend_type("NilClass", "ruby"), Some("nil"));
///
/// // Unknown → None
/// assert_eq!(map_frontend_type("Widget", "twig"), None);
/// ```
pub fn map_frontend_type(frontend_type: &str, language: &str) -> Option<&'static str> {
    // 1. Pass-through: already canonical.
    if is_canonical(frontend_type) {
        // Safety: the string matches a &'static str in ALL_CANONICAL_TYPES —
        // return the matching static ref.
        return ALL_CANONICAL_TYPES
            .iter()
            .copied()
            .find(|&s| s == frontend_type);
    }

    // 2. Language-specific mappings.
    match language.to_lowercase().as_str() {
        "twig" | "lispy" | "tetrad" | "algol" | "oct" => map_twig_like(frontend_type),
        "typescript" | "ts" => map_typescript(frontend_type),
        "ruby" | "sorbet" | "ruby-sorbet" => map_ruby(frontend_type),
        "hack" | "php" => map_hack(frontend_type),
        "python" | "mypy" => map_python(frontend_type),
        "rust" | "c" | "c+" | "cpp" => map_rust_like(frontend_type),
        _ => None,
    }
}

/// Return `true` if `type_str` is already a canonical IIR type name.
///
/// ```
/// use typing_spectrum::canonical::is_canonical;
///
/// assert!(is_canonical("i64"));
/// assert!(is_canonical("bool"));
/// assert!(!is_canonical("any"));  // "any" is not in ALL_CANONICAL_TYPES
/// assert!(!is_canonical("Integer"));
/// ```
pub fn is_canonical(type_str: &str) -> bool {
    ALL_CANONICAL_TYPES.contains(&type_str)
}

// ---------------------------------------------------------------------------
// Private per-language helpers
// ---------------------------------------------------------------------------

fn map_twig_like(t: &str) -> Option<&'static str> {
    // Twig, Lispy, Tetrad, Algol — all share a simple "int/float/bool/str/nil"
    // vocabulary.  Some also accept the canonical Rust-style names directly.
    match t {
        "int" | "integer" | "Integer" => Some(TYPE_I64),
        "float" | "double" | "Float"  => Some(TYPE_F64),
        "bool" | "boolean" | "Bool"   => Some(TYPE_BOOL),
        "string" | "String"           => Some(TYPE_STR),
        "nil" | "null" | "Nil"        => Some(TYPE_NIL),
        "symbol" | "Symbol"           => Some(TYPE_SYMBOL),
        "closure" | "fn" | "lambda"   => Some(TYPE_CLOSURE),
        "cons" | "pair"               => Some(TYPE_CONS),
        _ => None,
    }
}

fn map_typescript(t: &str) -> Option<&'static str> {
    // TypeScript: `number` is always f64 (IEEE 754 double); `bigint` is
    // intentionally not mapped because it exceeds i64 semantics.
    match t {
        "number"              => Some(TYPE_F64),
        "boolean"             => Some(TYPE_BOOL),
        "string"              => Some(TYPE_STR),
        "null" | "undefined"  => Some(TYPE_NIL),
        "symbol"              => Some(TYPE_SYMBOL),
        // TypeScript function types are all "closure" in IIR.
        "Function" | "(...args: any[]) => any" => Some(TYPE_CLOSURE),
        _ => None,
    }
}

fn map_ruby(t: &str) -> Option<&'static str> {
    // Ruby / Sorbet: class names are PascalCase.
    match t {
        "Integer"                        => Some(TYPE_I64),
        "Float"                          => Some(TYPE_F64),
        "TrueClass" | "FalseClass" | "T::Boolean" => Some(TYPE_BOOL),
        "String"                         => Some(TYPE_STR),
        "Symbol"                         => Some(TYPE_SYMBOL),
        "NilClass"                       => Some(TYPE_NIL),
        "Proc" | "Method" | "UnboundMethod" => Some(TYPE_CLOSURE),
        // Typed-list types could map to cons; conservative: only for Lisp-ish Ruby.
        _ => None,
    }
}

fn map_hack(t: &str) -> Option<&'static str> {
    // Hack / HHVM: PHP-style lowercase names.
    match t {
        "int"    => Some(TYPE_I64),
        "float"  => Some(TYPE_F64),
        "bool"   => Some(TYPE_BOOL),
        "string" => Some(TYPE_STR),
        "null"   => Some(TYPE_NIL),
        _ => None,
    }
}

fn map_python(t: &str) -> Option<&'static str> {
    // Mypy / Python: class names are PascalCase; built-ins are lowercase.
    match t {
        "int"   | "Int"   => Some(TYPE_I64),
        "float" | "Float" => Some(TYPE_F64),
        "bool"  | "Bool"  => Some(TYPE_BOOL),
        "str"   | "Str"   => Some(TYPE_STR),
        "None"  | "NoneType" => Some(TYPE_NIL),
        _ => None,
    }
}

fn map_rust_like(t: &str) -> Option<&'static str> {
    // Rust / C / C+: use the canonical IIR names directly *plus* some aliases.
    match t {
        "i8"   => Some(TYPE_I8),
        "i16"  => Some(TYPE_I16),
        "i32"  => Some(TYPE_I32),
        "i64"  => Some(TYPE_I64),
        "u8"   => Some(TYPE_U8),
        "u16"  => Some(TYPE_U16),
        "u32"  => Some(TYPE_U32),
        "u64"  => Some(TYPE_U64),
        "f32"  => Some(TYPE_F32),
        "f64"  => Some(TYPE_F64),
        "bool" => Some(TYPE_BOOL),
        // C aliases.
        "char"           => Some(TYPE_U8),
        "short"          => Some(TYPE_I16),
        "int" | "long"   => Some(TYPE_I32),
        "long long"      => Some(TYPE_I64),
        "unsigned char"  => Some(TYPE_U8),
        "unsigned short" => Some(TYPE_U16),
        "unsigned int" | "unsigned long" => Some(TYPE_U32),
        "unsigned long long" => Some(TYPE_U64),
        "float"          => Some(TYPE_F32),
        "double"         => Some(TYPE_F64),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Canonical pass-through ───────────────────────────────────────
    #[test]
    fn canonical_type_passes_through() {
        for &ty in ALL_CANONICAL_TYPES {
            assert_eq!(
                map_frontend_type(ty, "any"),
                Some(ty),
                "canonical type {ty} not passed through"
            );
        }
    }

    #[test]
    fn any_is_not_canonical() {
        assert!(!is_canonical("any"));
    }

    #[test]
    fn all_canonical_types_are_canonical() {
        for &ty in ALL_CANONICAL_TYPES {
            assert!(is_canonical(ty), "{ty} not recognised as canonical");
        }
    }

    // ── Twig / Lispy ────────────────────────────────────────────────
    #[test]
    fn twig_int_maps_to_i64() {
        assert_eq!(map_frontend_type("int", "twig"), Some("i64"));
        assert_eq!(map_frontend_type("integer", "twig"), Some("i64"));
    }

    #[test]
    fn twig_float_maps_to_f64() {
        assert_eq!(map_frontend_type("float", "twig"), Some("f64"));
        assert_eq!(map_frontend_type("double", "twig"), Some("f64"));
    }

    #[test]
    fn twig_bool_maps_to_bool() {
        assert_eq!(map_frontend_type("bool", "twig"), Some("bool"));
        assert_eq!(map_frontend_type("boolean", "twig"), Some("bool"));
    }

    #[test]
    fn twig_string_maps_to_str() {
        assert_eq!(map_frontend_type("string", "twig"), Some("str"));
        assert_eq!(map_frontend_type("String", "twig"), Some("str"));
    }

    #[test]
    fn twig_nil_maps_to_nil() {
        assert_eq!(map_frontend_type("nil", "twig"), Some("nil"));
        assert_eq!(map_frontend_type("null", "twig"), Some("nil"));
    }

    #[test]
    fn twig_symbol() {
        assert_eq!(map_frontend_type("symbol", "twig"), Some("symbol"));
    }

    #[test]
    fn twig_closure() {
        assert_eq!(map_frontend_type("closure", "twig"), Some("closure"));
        assert_eq!(map_frontend_type("lambda", "twig"), Some("closure"));
    }

    // ── TypeScript ───────────────────────────────────────────────────
    #[test]
    fn typescript_number_is_f64() {
        assert_eq!(map_frontend_type("number", "typescript"), Some("f64"));
    }

    #[test]
    fn typescript_boolean_is_bool() {
        assert_eq!(map_frontend_type("boolean", "typescript"), Some("bool"));
    }

    #[test]
    fn typescript_string_is_str() {
        assert_eq!(map_frontend_type("string", "typescript"), Some("str"));
    }

    #[test]
    fn typescript_null_is_nil() {
        assert_eq!(map_frontend_type("null", "typescript"), Some("nil"));
        assert_eq!(map_frontend_type("undefined", "typescript"), Some("nil"));
    }

    // ── Ruby / Sorbet ────────────────────────────────────────────────
    #[test]
    fn ruby_integer_is_i64() {
        assert_eq!(map_frontend_type("Integer", "ruby"), Some("i64"));
    }

    #[test]
    fn ruby_float_is_f64() {
        assert_eq!(map_frontend_type("Float", "ruby"), Some("f64"));
    }

    #[test]
    fn ruby_trueclass_is_bool() {
        assert_eq!(map_frontend_type("TrueClass", "ruby"), Some("bool"));
        assert_eq!(map_frontend_type("FalseClass", "ruby"), Some("bool"));
    }

    #[test]
    fn ruby_nilclass_is_nil() {
        assert_eq!(map_frontend_type("NilClass", "ruby"), Some("nil"));
    }

    #[test]
    fn ruby_symbol_is_symbol() {
        assert_eq!(map_frontend_type("Symbol", "ruby"), Some("symbol"));
    }

    #[test]
    fn ruby_proc_is_closure() {
        assert_eq!(map_frontend_type("Proc", "ruby"), Some("closure"));
    }

    // ── Hack ─────────────────────────────────────────────────────────
    #[test]
    fn hack_int_is_i64() {
        assert_eq!(map_frontend_type("int", "hack"), Some("i64"));
    }

    #[test]
    fn hack_float_is_f64() {
        assert_eq!(map_frontend_type("float", "hack"), Some("f64"));
    }

    #[test]
    fn hack_bool_is_bool() {
        assert_eq!(map_frontend_type("bool", "hack"), Some("bool"));
    }

    #[test]
    fn hack_null_is_nil() {
        assert_eq!(map_frontend_type("null", "hack"), Some("nil"));
    }

    // ── Python / mypy ────────────────────────────────────────────────
    #[test]
    fn python_int_is_i64() {
        assert_eq!(map_frontend_type("int", "python"), Some("i64"));
    }

    #[test]
    fn python_none_is_nil() {
        assert_eq!(map_frontend_type("None", "python"), Some("nil"));
        assert_eq!(map_frontend_type("NoneType", "python"), Some("nil"));
    }

    #[test]
    fn python_str_is_str() {
        assert_eq!(map_frontend_type("str", "python"), Some("str"));
    }

    // ── Rust / C ────────────────────────────────────────────────────
    #[test]
    fn rust_primitives_pass_through() {
        for &ty in NUMERIC_TYPES {
            assert_eq!(map_frontend_type(ty, "rust"), Some(ty));
        }
        assert_eq!(map_frontend_type("bool", "rust"), Some("bool"));
    }

    #[test]
    fn c_char_maps_to_u8() {
        assert_eq!(map_frontend_type("char", "c"), Some("u8"));
    }

    #[test]
    fn c_long_long_maps_to_i64() {
        assert_eq!(map_frontend_type("long long", "c"), Some("i64"));
    }

    // ── Unknown types ────────────────────────────────────────────────
    #[test]
    fn unknown_type_returns_none() {
        assert_eq!(map_frontend_type("Widget", "twig"), None);
        assert_eq!(map_frontend_type("AnyRandomClass", "typescript"), None);
    }

    #[test]
    fn unknown_language_returns_none_for_non_canonical() {
        assert_eq!(map_frontend_type("Integer", "brainfuck"), None);
    }
}
