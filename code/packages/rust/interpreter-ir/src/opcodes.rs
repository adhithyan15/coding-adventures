//! Opcode category helpers and type-string constants for InterpreterIR.
//!
//! These functions let vm-core, jit-core, and IR passes classify instructions
//! without string-matching against long mnemonic lists.  Every set is a simple
//! `match` — no heap allocation, no `HashSet`, no lazy initialisation.  The
//! compiler inlines and optimises them to a jump table.
//!
//! # Type constants
//!
//! IIR uses *string* type hints rather than an enum so that language frontends
//! can introduce domain-specific types without modifying this crate.  The
//! constants below are the universally recognised ones:
//!
//! ```
//! use interpreter_ir::opcodes::{DYNAMIC_TYPE, POLYMORPHIC_TYPE};
//! assert_eq!(DYNAMIC_TYPE, "any");
//! assert_eq!(POLYMORPHIC_TYPE, "polymorphic");
//! ```
//!
//! # Opcode categories
//!
//! ```
//! use interpreter_ir::opcodes::is_arithmetic;
//! assert!(is_arithmetic("add"));
//! assert!(is_arithmetic("neg"));
//! assert!(!is_arithmetic("jmp"));
//! ```
//!
//! # Reference types (LANG16)
//!
//! Heap pointers are encoded as the string `"ref<T>"` where `T` is the
//! pointee type.  Examples:
//!
//! ```
//! use interpreter_ir::opcodes::{is_ref_type, unwrap_ref_type, make_ref_type};
//! assert!(is_ref_type("ref<u8>"));
//! assert_eq!(unwrap_ref_type("ref<u8>"), Some("u8".to_string()));
//! assert_eq!(make_ref_type("any"), "ref<any>");
//! ```

// ---------------------------------------------------------------------------
// Type-string constants
// ---------------------------------------------------------------------------

/// The dynamic (unknown) type used by untyped languages before profiling.
///
/// An instruction whose `type_hint == DYNAMIC_TYPE` will be observed by the
/// profiler; instructions with concrete types are skipped (zero overhead).
pub const DYNAMIC_TYPE: &str = "any";

/// Sentinel written by the profiler when a slot has seen multiple types.
///
/// A JIT that reads `observed_type == POLYMORPHIC_TYPE` should NOT specialise
/// — the value is too variable to fix at compile time.
pub const POLYMORPHIC_TYPE: &str = "polymorphic";

/// The concrete types recognised by every LANG-pipeline backend.
///
/// Language frontends may use additional type strings; these are the ones
/// that all backends (WASM, JVM, Intel 4004, …) agree on.
pub const CONCRETE_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64",
    "i8", "i16", "i32", "i64",
    "f32", "f64",
    "bool", "str",
];

// ---------------------------------------------------------------------------
// Reference-type helpers (LANG16)
// ---------------------------------------------------------------------------
//
// Heap pointers use the "ref<T>" encoding so the rest of the type system stays
// unchanged.  A Lisp nil-terminated list might look like `ref<ref<any>>`;
// a boxed integer is `ref<u8>`.

const REF_PREFIX: &str = "ref<";
const REF_SUFFIX: &str = ">";

/// Return `true` if `type_hint` is a heap-reference type `"ref<T>"`.
pub fn is_ref_type(type_hint: &str) -> bool {
    type_hint.starts_with(REF_PREFIX) && type_hint.ends_with(REF_SUFFIX)
}

/// Return `Some(T)` for `"ref<T>"`, or `None` for non-reference types.
///
/// ```
/// use interpreter_ir::opcodes::unwrap_ref_type;
/// assert_eq!(unwrap_ref_type("ref<u8>"),       Some("u8".to_string()));
/// assert_eq!(unwrap_ref_type("ref<ref<any>>"), Some("ref<any>".to_string()));
/// assert_eq!(unwrap_ref_type("u8"),            None);
/// ```
pub fn unwrap_ref_type(type_hint: &str) -> Option<String> {
    if !is_ref_type(type_hint) {
        return None;
    }
    let inner = &type_hint[REF_PREFIX.len()..type_hint.len() - REF_SUFFIX.len()];
    Some(inner.to_string())
}

/// Wrap `inner` as a reference type string.  Inverse of [`unwrap_ref_type`].
///
/// ```
/// use interpreter_ir::opcodes::make_ref_type;
/// assert_eq!(make_ref_type("u8"),  "ref<u8>");
/// assert_eq!(make_ref_type("any"), "ref<any>");
/// ```
pub fn make_ref_type(inner: &str) -> String {
    format!("{REF_PREFIX}{inner}{REF_SUFFIX}")
}

// ---------------------------------------------------------------------------
// Opcode category predicates
// ---------------------------------------------------------------------------
//
// Using plain `match` rather than `HashSet` means:
//   • Zero heap allocation
//   • LLVM can turn the match into a lookup table at -O2
//   • Every new opcode must be added to exactly one category (no silent gaps)

/// Integer and floating-point arithmetic.
pub fn is_arithmetic(op: &str) -> bool {
    matches!(op, "add" | "sub" | "mul" | "div" | "mod" | "neg")
}

/// Bitwise operations.
pub fn is_bitwise(op: &str) -> bool {
    matches!(op, "and" | "or" | "xor" | "not" | "shl" | "shr")
}

/// Comparison operations — all produce a `bool`.
pub fn is_cmp(op: &str) -> bool {
    matches!(
        op,
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// Conditional and unconditional branches.
pub fn is_branch(op: &str) -> bool {
    matches!(op, "jmp" | "jmp_if_true" | "jmp_if_false")
}

/// Control-flow terminators (labels count here because they delimit blocks).
pub fn is_control(op: &str) -> bool {
    matches!(op, "label" | "ret" | "ret_void")
}

/// Register and memory loads/stores.
pub fn is_memory(op: &str) -> bool {
    matches!(op, "load_reg" | "store_reg" | "load_mem" | "store_mem")
}

/// Function calls.
pub fn is_call(op: &str) -> bool {
    matches!(op, "call" | "call_builtin")
}

/// I/O operations.
pub fn is_io(op: &str) -> bool {
    matches!(op, "io_in" | "io_out")
}

/// Type coercions and assertions.
pub fn is_coercion(op: &str) -> bool {
    matches!(op, "cast" | "type_assert")
}

/// Heap / GC operations (LANG16).
///
/// Programs that never allocate never emit these — GC overhead is zero.
/// Programs that do allocate use these seven opcodes to communicate
/// allocation intent and write-barrier points to vm-core's GC layer.
pub fn is_heap(op: &str) -> bool {
    matches!(
        op,
        "alloc"       // heap-allocate N bytes of kind K → ref<K>
        | "box"       // heap-allocate and store a value → ref<T>
        | "unbox"     // load from ref<T>; trap on null
        | "field_load"  // *(ref + offset)
        | "field_store" // *(ref + offset) = value; may emit write barrier
        | "is_null"   // (ref == NULL) → bool
        | "safepoint" // yield to GC if collection pending; may_alloc
    )
}

/// Return `true` if `op` produces a result value (has a non-`None` dest).
pub fn is_value_producing(op: &str) -> bool {
    is_arithmetic(op)
        || is_bitwise(op)
        || is_cmp(op)
        || matches!(
            op,
            "const"
                | "load_reg"
                | "load_mem"
                | "call"
                | "call_builtin"
                | "io_in"
                | "cast"
                | "alloc"
                | "box"
                | "unbox"
                | "field_load"
                | "is_null"
        )
}

/// Return `true` if `op` has side effects beyond producing a value.
pub fn has_side_effects(op: &str) -> bool {
    is_branch(op)
        || is_control(op)
        || matches!(
            op,
            "store_reg" | "store_mem" | "io_out" | "type_assert" | "field_store" | "safepoint"
        )
}

/// Return `true` if `op` may trigger a GC cycle.
///
/// Language frontends set `IIRInstr::may_alloc = true` for these opcodes
/// plus any `call` whose callee transitively allocates.
pub fn is_allocating(op: &str) -> bool {
    matches!(op, "alloc" | "box" | "safepoint")
}

/// Return `true` if `op` is a recognised IIR mnemonic.
///
/// Unknown mnemonics are rejected by the module validator.
pub fn is_known_op(op: &str) -> bool {
    op == "const"
        || is_arithmetic(op)
        || is_bitwise(op)
        || is_cmp(op)
        || is_branch(op)
        || is_control(op)
        || is_memory(op)
        || is_call(op)
        || is_io(op)
        || is_coercion(op)
        || is_heap(op)
}

/// Return `true` if `type_hint` is a concrete (non-dynamic) type.
///
/// Concrete-type instructions are skipped by the profiler — their type is
/// already known at compile time.
pub fn is_concrete_type(type_hint: &str) -> bool {
    CONCRETE_TYPES.contains(&type_hint) || is_ref_type(type_hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_ops_recognised() {
        for op in &["add", "sub", "mul", "div", "mod", "neg"] {
            assert!(is_arithmetic(op), "{op}");
        }
        assert!(!is_arithmetic("jmp"));
    }

    #[test]
    fn cmp_ops_recognised() {
        for op in &["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"] {
            assert!(is_cmp(op), "{op}");
        }
    }

    #[test]
    fn ref_type_round_trip() {
        assert!(is_ref_type("ref<u8>"));
        assert!(is_ref_type("ref<ref<any>>"));
        assert!(!is_ref_type("u8"));
        assert_eq!(unwrap_ref_type("ref<u8>"), Some("u8".to_string()));
        assert_eq!(unwrap_ref_type("ref<ref<any>>"), Some("ref<any>".to_string()));
        assert_eq!(unwrap_ref_type("u8"), None);
        assert_eq!(make_ref_type("u8"), "ref<u8>");
    }

    #[test]
    fn concrete_type_check() {
        for t in CONCRETE_TYPES {
            assert!(is_concrete_type(t), "{t}");
        }
        assert!(!is_concrete_type("any"));
        assert!(!is_concrete_type("polymorphic"));
        assert!(is_concrete_type("ref<u8>"));
    }

    #[test]
    fn is_known_op_covers_all_categories() {
        for op in &[
            "const", "add", "sub", "and", "cmp_eq", "jmp", "label", "ret",
            "load_reg", "call", "io_in", "cast", "alloc",
        ] {
            assert!(is_known_op(op), "{op}");
        }
        assert!(!is_known_op("tetrad.move"));
    }
}
