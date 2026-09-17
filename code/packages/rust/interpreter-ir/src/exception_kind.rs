//! Exception **kinds** — the names `throw`/`catch` match on (AOT00 T2).
//!
//! # Why strings, not an enum
//!
//! `interpreter-ir` already makes this choice for *types* (`opcodes::DYNAMIC_TYPE`
//! doc comment: "IIR uses string type hints rather than an enum so that language
//! frontends can introduce domain-specific types without modifying this crate").
//! Exception kinds face the identical problem one level up: ALGOL's `alarm`,
//! Lisp's `condition` system, and a plain user-defined exception type in some
//! future frontend all need their *own* kind names without a PR against this
//! crate. So a kind is a plain `&str`, exactly like a type hint.
//!
//! The [`AOT00-T2-exceptions.md`](../../../../specs/AOT00-T2-exceptions.md) §4.1
//! design calls this a `kind_mask : u32` — a bitset over a `KindRegistry`
//! (the same registry idea `gc-core`'s `HeapKind`s use, per that spec's note
//! "one registry mechanism, two uses"). That bitset is a **backend-emission**
//! detail (how a compiled `UnwindRecord` packs many kinds into 32 bits); at
//! the source-level IIR carried by `catch`'s `Operand::Str` operand, the
//! human-readable *name* is what a frontend emits, and [`kind_matches`] is
//! the reference semantics a later backend's bitset must agree with bit-for-bit.
//!
//! # Hierarchical kind names
//!
//! A kind name is a dot-separated path, most specific last, e.g. `"Trap.Bounds"`.
//! A `catch` on a *prefix* of a thrown kind catches it — `catch "Trap"` catches
//! `throw Trap.Bounds`, mirroring how Java's `catch (IOException e)` also
//! catches a thrown `FileNotFoundException` subclass. There is no subtyping
//! *hierarchy declaration* here (no registry of "which kinds exist") — just
//! string-prefix matching on the dotted path, which is enough to express both
//! "catch this exact kind" and "catch this whole family" without inventing a
//! class system.
//!
//! ```text
//! thrown = "Trap.Bounds"
//!
//! catch "Trap.Bounds"  → match   (exact)
//! catch "Trap"         → match   (thrown is a "Trap.*" specialisation)
//! catch "*"            → match   (catch-anything sentinel)
//! catch "Trap.Null"    → no match (sibling kind, not an ancestor)
//! catch "Bounds"       → no match ("Bounds" is not a prefix component of "Trap.Bounds")
//! ```
//!
//! # Built-in `Trap` kinds (the trap migration, spec §6)
//!
//! Every trap site that aborts the program today (`RunResult::Trapped`) will,
//! in a later slice, `throw` one of these instead of aborting outright — a
//! program with no handler still unwinds to the top and aborts with today's
//! exit signal (T7 trap-agreement proves this); a program that installs
//! `catch "Trap"` gains the new ability to recover. **This module only names
//! the kinds** — no trap site is migrated yet (see AOT00-T2-exceptions.md §12
//! for the slice that does).
//!
//! | Constant | Kind name | Today's trap site |
//! |----------|-----------|--------------------|
//! | [`TRAP_BOUNDS`] | `"Trap.Bounds"` | `array_get`/`array_set` out-of-range index |
//! | [`TRAP_NULL`] | `"Trap.Null"` | `unbox` of a null `ref<T>` |
//! | [`TRAP_DIV_ZERO`] | `"Trap.DivZero"` | integer `div`/`mod` by zero |
//! | [`TRAP_CONV_RANGE`] | `"Trap.ConvRange"` | `real_to_int_trunc`/`real_to_int_floor` on a non-finite or out-of-range value |
//!
//! [`TRAP_KIND`] (`"Trap"`) is every built-in trap's common ancestor —
//! `catch "Trap"` is the "catch any trap" idiom; [`CATCH_ANY`] (`"*"`) is the
//! stronger "catch literally anything, including frontend-defined kinds"
//! sentinel the spec calls "an all-ones mask" (§4.1).

// ---------------------------------------------------------------------------
// Built-in kind constants
// ---------------------------------------------------------------------------

/// Common ancestor of every built-in trap kind. `catch "Trap"` catches all four.
pub const TRAP_KIND: &str = "Trap";

/// Out-of-range `array_get`/`array_set` index.
pub const TRAP_BOUNDS: &str = "Trap.Bounds";

/// `unbox` of a null `ref<T>`.
pub const TRAP_NULL: &str = "Trap.Null";

/// Integer `div`/`mod` by zero.
pub const TRAP_DIV_ZERO: &str = "Trap.DivZero";

/// `real_to_int_trunc`/`real_to_int_floor` given a non-finite or
/// out-of-range operand (LANG-FULL E8).
pub const TRAP_CONV_RANGE: &str = "Trap.ConvRange";

/// The four built-in trap kinds, in the order §6 of the T2 spec lists them.
pub const ALL_TRAP_KINDS: &[&str] = &[TRAP_BOUNDS, TRAP_NULL, TRAP_DIV_ZERO, TRAP_CONV_RANGE];

/// Catch-anything sentinel — the spec's "all-ones mask" (§4.1). Matches every
/// thrown kind, built-in or frontend-defined.
pub const CATCH_ANY: &str = "*";

/// The path separator for hierarchical kind names (`"Trap.Bounds"`).
const KIND_SEPARATOR: char = '.';

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// Return `true` if a `catch` declaring `catch_kind` covers a `throw` of
/// `thrown_kind`.
///
/// Three ways a match happens, checked in this order:
/// 1. `catch_kind == "*"` — [`CATCH_ANY`] matches everything.
/// 2. `catch_kind == thrown_kind` — an exact match.
/// 3. `thrown_kind` starts with `catch_kind` **followed by a `.`** — `catch_kind`
///    is a strict dotted-path ancestor of `thrown_kind` (so `"Trap"` matches
///    `"Trap.Bounds"`, but `"Bounds"` does not, and `"Trap.Bounds"` does not
///    match a *different* leaf like `"Trap.Null"`).
///
/// ```
/// use interpreter_ir::exception_kind::{kind_matches, CATCH_ANY, TRAP_KIND, TRAP_BOUNDS, TRAP_NULL};
///
/// // Exact match.
/// assert!(kind_matches(TRAP_BOUNDS, TRAP_BOUNDS));
/// // Ancestor match — catching the family catches the specific trap.
/// assert!(kind_matches(TRAP_KIND, TRAP_BOUNDS));
/// // Catch-anything.
/// assert!(kind_matches(CATCH_ANY, TRAP_BOUNDS));
/// assert!(kind_matches(CATCH_ANY, "app.UserDefinedError"));
/// // Siblings never match each other.
/// assert!(!kind_matches(TRAP_BOUNDS, TRAP_NULL));
/// // A bare leaf name is not a prefix *component* match.
/// assert!(!kind_matches("Bounds", TRAP_BOUNDS));
/// ```
pub fn kind_matches(catch_kind: &str, thrown_kind: &str) -> bool {
    if catch_kind == CATCH_ANY {
        return true;
    }
    if catch_kind == thrown_kind {
        return true;
    }
    thrown_kind
        .strip_prefix(catch_kind)
        .is_some_and(|rest| rest.starts_with(KIND_SEPARATOR))
}

/// Return `true` if `kind` is one of the four built-in [`ALL_TRAP_KINDS`].
///
/// ```
/// use interpreter_ir::exception_kind::{is_trap_kind, TRAP_DIV_ZERO};
/// assert!(is_trap_kind(TRAP_DIV_ZERO));
/// assert!(!is_trap_kind("app.UserDefinedError"));
/// // The family ancestor "Trap" itself is not one of the four leaf kinds.
/// assert!(!is_trap_kind("Trap"));
/// ```
pub fn is_trap_kind(kind: &str) -> bool {
    ALL_TRAP_KINDS.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        for k in ALL_TRAP_KINDS {
            assert!(kind_matches(k, k), "{k} should match itself");
        }
    }

    #[test]
    fn ancestor_match() {
        for k in ALL_TRAP_KINDS {
            assert!(kind_matches(TRAP_KIND, k), "\"Trap\" should catch {k}");
        }
    }

    #[test]
    fn catch_any_matches_everything() {
        assert!(kind_matches(CATCH_ANY, TRAP_BOUNDS));
        assert!(kind_matches(CATCH_ANY, TRAP_KIND));
        assert!(kind_matches(CATCH_ANY, "totally.unregistered.kind"));
        assert!(kind_matches(CATCH_ANY, CATCH_ANY));
    }

    #[test]
    fn siblings_do_not_match() {
        assert!(!kind_matches(TRAP_BOUNDS, TRAP_NULL));
        assert!(!kind_matches(TRAP_NULL, TRAP_DIV_ZERO));
        assert!(!kind_matches(TRAP_CONV_RANGE, TRAP_BOUNDS));
    }

    #[test]
    fn descendant_does_not_catch_ancestor() {
        // catch "Trap.Bounds" does NOT catch a bare "throw Trap" — matching
        // is ancestor-catches-descendant, never the reverse.
        assert!(!kind_matches(TRAP_BOUNDS, TRAP_KIND));
    }

    #[test]
    fn bare_prefix_without_separator_does_not_match() {
        // "Bounds" is a substring of "Trap.Bounds" but not a dotted-path
        // ancestor of it — the separator check must reject this.
        assert!(!kind_matches("Bounds", TRAP_BOUNDS));
        assert!(!kind_matches("Trap.Boun", TRAP_BOUNDS));
    }

    #[test]
    fn frontend_defined_kinds_use_the_same_rules() {
        // A frontend (ALGOL/Lisp/etc.) can register its own dotted namespace
        // with zero changes to this crate.
        assert!(kind_matches("lisp.condition", "lisp.condition.arithmetic-error"));
        assert!(!kind_matches("lisp.condition.arithmetic-error", "lisp.condition"));
        assert!(kind_matches(CATCH_ANY, "lisp.condition.arithmetic-error"));
    }

    #[test]
    fn is_trap_kind_covers_exactly_the_four_leaves() {
        for k in ALL_TRAP_KINDS {
            assert!(is_trap_kind(k));
        }
        assert!(!is_trap_kind(TRAP_KIND));
        assert!(!is_trap_kind(CATCH_ANY));
        assert!(!is_trap_kind("app.UserDefinedError"));
    }

    #[test]
    fn all_trap_kinds_are_under_the_trap_namespace() {
        for k in ALL_TRAP_KINDS {
            assert!(k.starts_with("Trap."), "{k} should be under the Trap. namespace");
        }
    }
}
