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
//! | [`TRAP_OVERFLOW`] | `"Trap.Overflow"` | `i64_div_floor` of `MIN / -1` (LANG83 PR1b); the `i64_*_ovf` family (PR4) |
//! | [`TRAP_USER`] | `"Trap.User"` | a frontend's explicit abort (`die`, a bare `raise`, …) via `trap` |
//!
//! # The closed trap-kind table (LANG83)
//!
//! The `trap kind` operation (LANG83) may only name a kind from a **closed**
//! table: the six built-in `Trap.*` leaves above plus [`LANGUAGE_TRAP_KINDS`],
//! where a language frontend registers its own kinds (for example a future
//! `Ruby.ZeroDivisionError`) in the PR that first emits them. Each kind has a
//! **stable integer id** ([`trap_kind_id`]) — what a native backend passes to
//! its runtime (`mov edi, id; call __twig_trap`) and what a runtime maps back
//! to a name for the stderr report. Ids are append-only and never renumbered:
//!
//! | Range | Owner |
//! |-------|-------|
//! | 1–999 | built-in `Trap.*` kinds, in [`ALL_TRAP_KINDS`] order |
//! | 1000+ | [`LANGUAGE_TRAP_KINDS`], in table order |
//!
//! [`TRAP_KIND`] (`"Trap"`) is every built-in trap's common ancestor —
//! `catch "Trap"` is the "catch any trap" idiom; [`CATCH_ANY`] (`"*"`) is the
//! stronger "catch literally anything, including frontend-defined kinds"
//! sentinel the spec calls "an all-ones mask" (§4.1).

// ---------------------------------------------------------------------------
// Built-in kind constants
// ---------------------------------------------------------------------------

/// Common ancestor of every built-in trap kind. `catch "Trap"` catches all of them.
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

/// Integer overflow with *defined* trapping semantics (LANG83): `i64_div_floor`
/// of `i64::MIN / -1` in PR1b, and the `i64_*_ovf` family in PR4.
pub const TRAP_OVERFLOW: &str = "Trap.Overflow";

/// A frontend's explicit, language-level abort that has no more specific kind
/// (Perl `die`, Ruby `raise "msg"`), emitted as `trap "Trap.User"` (LANG83).
pub const TRAP_USER: &str = "Trap.User";

/// The built-in trap kinds. The first four are in the order §6 of the T2 spec
/// lists them; LANG83 appended `Trap.Overflow` and `Trap.User`. **Append
/// only** — an entry's position (plus one) is its stable [`trap_kind_id`].
pub const ALL_TRAP_KINDS: &[&str] = &[
    TRAP_BOUNDS,
    TRAP_NULL,
    TRAP_DIV_ZERO,
    TRAP_CONV_RANGE,
    TRAP_OVERFLOW,
    TRAP_USER,
];

/// Language-specific trap kinds a frontend may name in `trap` (LANG83).
///
/// Empty today: the first language kind (e.g. `Ruby.ZeroDivisionError`) is
/// added by the PR whose frontend first emits it. **Append only** — entry `i`
/// has the stable id `LANGUAGE_TRAP_KIND_ID_BASE + i`. A kind here must not
/// start with `"Trap."` (that namespace is the built-in table's).
pub const LANGUAGE_TRAP_KINDS: &[&str] = &[];

/// The first stable id handed to a [`LANGUAGE_TRAP_KINDS`] entry. Built-in kinds
/// use `1..LANGUAGE_TRAP_KIND_ID_BASE`, so the two ranges can never collide.
pub const LANGUAGE_TRAP_KIND_ID_BASE: u32 = 1000;

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

/// Return `true` if `kind` is one of the built-in [`ALL_TRAP_KINDS`].
///
/// ```
/// use interpreter_ir::exception_kind::{is_trap_kind, TRAP_DIV_ZERO};
/// assert!(is_trap_kind(TRAP_DIV_ZERO));
/// assert!(!is_trap_kind("app.UserDefinedError"));
/// // The family ancestor "Trap" itself is not one of the leaf kinds.
/// assert!(!is_trap_kind("Trap"));
/// ```
pub fn is_trap_kind(kind: &str) -> bool {
    ALL_TRAP_KINDS.contains(&kind)
}

/// Return `true` if `kind` may be named by a `trap` operation: a built-in
/// [`ALL_TRAP_KINDS`] leaf or a registered [`LANGUAGE_TRAP_KINDS`] entry.
///
/// ```
/// use interpreter_ir::exception_kind::{is_registered_trap_kind, TRAP_OVERFLOW};
/// assert!(is_registered_trap_kind(TRAP_OVERFLOW));
/// assert!(!is_registered_trap_kind("Trap"));            // a family, not a leaf
/// assert!(!is_registered_trap_kind("Ruby.Unregistered")); // not in the table
/// ```
pub fn is_registered_trap_kind(kind: &str) -> bool {
    is_trap_kind(kind) || LANGUAGE_TRAP_KINDS.contains(&kind)
}

/// The stable integer id of a registered trap kind, or `None`.
///
/// Built-in kinds are numbered from 1 in [`ALL_TRAP_KINDS`] order (0 is never
/// an id, so a zeroed register can never masquerade as a valid kind);
/// language kinds from [`LANGUAGE_TRAP_KIND_ID_BASE`].
///
/// ```
/// use interpreter_ir::exception_kind::{trap_kind_id, trap_kind_name, TRAP_DIV_ZERO};
/// assert_eq!(trap_kind_id(TRAP_DIV_ZERO), Some(3));
/// assert_eq!(trap_kind_name(3), Some(TRAP_DIV_ZERO));
/// assert_eq!(trap_kind_id("nope"), None);
/// ```
pub fn trap_kind_id(kind: &str) -> Option<u32> {
    if let Some(i) = ALL_TRAP_KINDS.iter().position(|k| *k == kind) {
        return Some(i as u32 + 1);
    }
    LANGUAGE_TRAP_KINDS
        .iter()
        .position(|k| *k == kind)
        .map(|i| LANGUAGE_TRAP_KIND_ID_BASE + i as u32)
}

/// The kind name for a stable id — the inverse of [`trap_kind_id`].
pub fn trap_kind_name(id: u32) -> Option<&'static str> {
    if id >= LANGUAGE_TRAP_KIND_ID_BASE {
        return LANGUAGE_TRAP_KINDS.get((id - LANGUAGE_TRAP_KIND_ID_BASE) as usize).copied();
    }
    id.checked_sub(1).and_then(|i| ALL_TRAP_KINDS.get(i as usize)).copied()
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
    fn lang83_kinds_are_built_in_leaves() {
        assert!(is_trap_kind(TRAP_OVERFLOW));
        assert!(is_trap_kind(TRAP_USER));
        assert!(kind_matches(TRAP_KIND, TRAP_OVERFLOW));
        assert!(!kind_matches(TRAP_OVERFLOW, TRAP_DIV_ZERO));
        assert_eq!(ALL_TRAP_KINDS.len(), 6);
    }

    #[test]
    fn trap_kind_ids_are_stable_and_round_trip() {
        // Pinned: these ids reach native runtimes; renumbering breaks them.
        let pinned = [
            (TRAP_BOUNDS, 1),
            (TRAP_NULL, 2),
            (TRAP_DIV_ZERO, 3),
            (TRAP_CONV_RANGE, 4),
            (TRAP_OVERFLOW, 5),
            (TRAP_USER, 6),
        ];
        for (kind, id) in pinned {
            assert_eq!(trap_kind_id(kind), Some(id), "{kind}");
            assert_eq!(trap_kind_name(id), Some(kind));
        }
        assert_eq!(trap_kind_name(0), None);
        assert_eq!(trap_kind_name(7), None);
        assert_eq!(trap_kind_name(LANGUAGE_TRAP_KIND_ID_BASE), LANGUAGE_TRAP_KINDS.first().copied());
        assert_eq!(trap_kind_id(TRAP_KIND), None);
        assert_eq!(trap_kind_id(CATCH_ANY), None);
    }

    #[test]
    fn language_kinds_stay_out_of_the_trap_namespace_and_id_range() {
        assert!((ALL_TRAP_KINDS.len() as u32) < LANGUAGE_TRAP_KIND_ID_BASE);
        for (i, kind) in LANGUAGE_TRAP_KINDS.iter().enumerate() {
            assert!(!kind.starts_with("Trap."), "{kind} must not use the built-in namespace");
            assert!(kind.contains('.'), "{kind} should be a dotted Language.Kind path");
            assert!(is_registered_trap_kind(kind));
            assert_eq!(trap_kind_id(kind), Some(LANGUAGE_TRAP_KIND_ID_BASE + i as u32));
        }
        assert!(!is_registered_trap_kind("Ruby.ZeroDivisionError"));
    }

    #[test]
    fn is_trap_kind_covers_exactly_the_built_in_leaves() {
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
