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
//! | [`TRAP_UNBOUND`] | `"Trap.Unbound"` | reading a name that is not bound yet, or at all (JavaScript's `ReferenceError`, LANG84 6e) |
//!
//! # The closed trap-kind table (LANG83)
//!
//! The `trap kind` operation (LANG83) may only name a kind from a **closed**
//! table: exactly the seven built-in `Trap.*` leaves above. Each kind has a
//! **stable integer id** ([`trap_kind_id`]) — what a native backend passes to
//! its runtime (`mov edi, id; call __twig_trap`) and what a runtime maps back
//! to a name for the stderr report. Ids are append-only and never renumbered:
//! a kind's id is its position in [`ALL_TRAP_KINDS`] plus one.
//!
//! # Every kind is language-neutral (LANG83 PR7d)
//!
//! A trap kind names a *machine-level condition* — a divisor is zero, an
//! index is out of range, a name is unbound — never one source language's
//! exception class. Each language's IIR compiler maps its errors onto the
//! kind for their condition:
//!
//! ```text
//! JavaScript  ReferenceError  ─┐
//! Python      NameError        ├─▶  trap "Trap.Unbound"   (id 7)
//! a Lisp      unbound variable ─┘
//!
//! Python      ZeroDivisionError ─┐
//! Ruby        ZeroDivisionError  ├─▶  trap "Trap.DivZero"  (id 3)
//! ```
//!
//! Two consequences shaped this table:
//!
//! - id 7 was first named `Trap.ReferenceError` (LANG84 6e, the first
//!   frontend to need it). PR7d renamed it `Trap.Unbound` and **kept id 7**:
//!   native code only ever passes the id, so a renamed row cannot strand a
//!   compiled binary, it only changes the text a trap prints.
//! - LANG83 PR1b once reserved ids from 1000 for a `LANGUAGE_TRAP_KINDS`
//!   table of language-namespaced kinds (`Ruby.ZeroDivisionError`). Nothing
//!   ever registered one, and PR7d retired the table: a language exception a
//!   program can *catch* will be a value the frontend builds once AOT00 T2
//!   exceptions land, never an IIR trap kind. So [`is_trap_kind`] is the
//!   whole validation, and a `trap "Ruby.ZeroDivisionError"` is malformed.
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

/// A read of a name that is not bound yet, or at all, with *defined*
/// trapping semantics (LANG84 phase 6e).
///
/// The condition is neutral: the name has no value here. JavaScript calls it
/// a `ReferenceError` for both the temporal dead zone
/// (`console.log(a); let a = 1;`) and an undeclared name, Python a
/// `NameError`, a Lisp an unbound variable. `javascript-iir-compiler` lowers
/// both JavaScript cases to `trap "Trap.Unbound"` after the output written
/// so far, and keeps V8's own wording (`ReferenceError: z is not defined`)
/// on its side of the boundary, since that sentence is JavaScript knowledge.
///
/// Stable id 7. Named `Trap.ReferenceError` until LANG83 PR7d renamed it for
/// language neutrality; the id did not change.
pub const TRAP_UNBOUND: &str = "Trap.Unbound";

/// The built-in trap kinds — the whole closed table. The first four are in
/// the order §6 of the T2 spec lists them; LANG83 appended `Trap.Overflow`
/// and `Trap.User`, and LANG84 phase 6e `Trap.Unbound` (then called
/// `Trap.ReferenceError`). **Append only** — an entry's position (plus one)
/// is its stable [`trap_kind_id`].
pub const ALL_TRAP_KINDS: &[&str] = &[
    TRAP_BOUNDS,
    TRAP_NULL,
    TRAP_DIV_ZERO,
    TRAP_CONV_RANGE,
    TRAP_OVERFLOW,
    TRAP_USER,
    TRAP_UNBOUND,
];

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

/// Return `true` if `kind` is one of the built-in [`ALL_TRAP_KINDS`] — the
/// closed table, and so exactly the kinds a `trap` operation may name.
///
/// This is the only trap-kind validation: every backend's `trap` lowering,
/// vm-core's `trap` handler and the LANG83 registry's `traps` lists all ask
/// it. A language-namespaced kind is refused (LANG83 PR7d).
///
/// ```
/// use interpreter_ir::exception_kind::{is_trap_kind, TRAP_DIV_ZERO, TRAP_UNBOUND};
/// assert!(is_trap_kind(TRAP_DIV_ZERO));
/// assert!(is_trap_kind(TRAP_UNBOUND));
/// assert!(!is_trap_kind("app.UserDefinedError"));
/// // The family ancestor "Trap" itself is not one of the leaf kinds.
/// assert!(!is_trap_kind("Trap"));
/// // A language's exception class is never a trap kind.
/// assert!(!is_trap_kind("Ruby.ZeroDivisionError"));
/// assert!(!is_trap_kind("Trap.ReferenceError")); // id 7's name before PR7d
/// ```
pub fn is_trap_kind(kind: &str) -> bool {
    ALL_TRAP_KINDS.contains(&kind)
}

/// The stable integer id of a trap kind, or `None`.
///
/// Kinds are numbered from 1 in [`ALL_TRAP_KINDS`] order (0 is never an id,
/// so a zeroed register can never masquerade as a valid kind).
///
/// ```
/// use interpreter_ir::exception_kind::{trap_kind_id, trap_kind_name, TRAP_DIV_ZERO};
/// assert_eq!(trap_kind_id(TRAP_DIV_ZERO), Some(3));
/// assert_eq!(trap_kind_name(3), Some(TRAP_DIV_ZERO));
/// assert_eq!(trap_kind_id("nope"), None);
/// ```
pub fn trap_kind_id(kind: &str) -> Option<u32> {
    ALL_TRAP_KINDS.iter().position(|k| *k == kind).map(|i| i as u32 + 1)
}

/// The kind name for a stable id — the inverse of [`trap_kind_id`].
///
/// ```
/// use interpreter_ir::exception_kind::{trap_kind_name, TRAP_UNBOUND};
/// assert_eq!(trap_kind_name(7), Some(TRAP_UNBOUND));
/// assert_eq!(trap_kind_name(0), None);
/// assert_eq!(trap_kind_name(1000), None); // the retired language-kind range
/// ```
pub fn trap_kind_name(id: u32) -> Option<&'static str> {
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
        assert!(is_trap_kind(TRAP_UNBOUND));
        assert!(kind_matches(TRAP_KIND, TRAP_UNBOUND));
        assert_eq!(ALL_TRAP_KINDS.len(), 7);
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
            (TRAP_UNBOUND, 7),
        ];
        for (kind, id) in pinned {
            assert_eq!(trap_kind_id(kind), Some(id), "{kind}");
            assert_eq!(trap_kind_name(id), Some(kind));
        }
        assert_eq!(trap_kind_name(0), None);
        assert_eq!(trap_kind_name(8), None);
        assert_eq!(trap_kind_name(1000), None);
        assert_eq!(trap_kind_id(TRAP_KIND), None);
        assert_eq!(trap_kind_id(CATCH_ANY), None);
    }

    /// LANG83 PR7d: id 7 was renamed, not renumbered, and its old name is gone.
    #[test]
    fn unbound_is_id_seven_and_its_old_name_is_not_a_kind() {
        assert_eq!(TRAP_UNBOUND, "Trap.Unbound");
        assert_eq!(trap_kind_id(TRAP_UNBOUND), Some(7));
        assert!(!is_trap_kind("Trap.ReferenceError"));
        assert_eq!(trap_kind_id("Trap.ReferenceError"), None);
    }

    /// LANG83 PR7d retired the language-kind table: a language's exception
    /// class is never a trap kind, and no id outside `1..=7` names one.
    #[test]
    fn language_namespaced_kinds_are_refused() {
        for kind in ["Ruby.ZeroDivisionError", "JavaScript.ReferenceError", "Python.NameError"] {
            assert!(!is_trap_kind(kind), "{kind}");
            assert_eq!(trap_kind_id(kind), None, "{kind}");
        }
        for id in [1000, 1001, u32::MAX] {
            assert_eq!(trap_kind_name(id), None, "id {id}");
        }
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
