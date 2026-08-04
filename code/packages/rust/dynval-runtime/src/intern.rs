//! # Process-global symbol intern table.
//!
//! Lisp / Scheme / Twig / Clojure all rely on **interned symbols**:
//! every distinct symbol name maps to a unique [`SymbolId`] so
//! `(eq? 'foo 'foo)` is true everywhere in the program in O(1).
//! The table is process-global (LANG20 §"Cross-language value
//! representation" — `lang-runtime-core` owns the intern table) so
//! Lispy code in different threads / fibers / language frontends
//! sees the same id for the same name.
//!
//! ## Implementation strategy (PR 2)
//!
//! Two `Mutex`-protected sides:
//!
//! - `by_name: HashMap<String, SymbolId>` — fast lookup during
//!   `intern("foo")`.
//! - `by_id: Vec<String>` — fast reverse lookup for printing /
//!   debugging.
//!
//! The mutex is taken once per call.  At interpreter speeds (each
//! intern is one mutex acquire + one hash) this is fine; if it
//! becomes a hot path under JIT, the right answer is a sharded
//! lock or a lock-free table.  PR 2 doesn't optimise.
//!
//! ## Reserved ids
//!
//! - [`SymbolId::EMPTY`] (== `SymbolId(0)`) is the empty string
//!   `""`, eagerly interned at startup so its id is stable.
//! - [`SymbolId::NONE`] (== `SymbolId(u32::MAX)`) is the absent-
//!   symbol sentinel; never returned by [`intern`].
//!
//! `intern("")` always returns `SymbolId::EMPTY`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use lang_runtime_core::SymbolId;

// ---------------------------------------------------------------------------
// SymbolTable
// ---------------------------------------------------------------------------

struct SymbolTable {
    /// Lookup by string for the interning fast path.
    by_name: HashMap<String, SymbolId>,
    /// Reverse lookup for `name_of(id)`.  Element index = `SymbolId(idx as u32).0`.
    by_id: Vec<String>,
}

impl SymbolTable {
    fn new() -> SymbolTable {
        let mut t = SymbolTable {
            by_name: HashMap::new(),
            by_id: Vec::new(),
        };
        // Eagerly intern the empty string so SymbolId::EMPTY is
        // always the empty string and SymbolId(0) is never invented
        // for a non-empty name.
        t.by_id.push(String::new());
        t.by_name.insert(String::new(), SymbolId::EMPTY);
        t
    }
}

static TABLE: OnceLock<Mutex<SymbolTable>> = OnceLock::new();

fn table() -> &'static Mutex<SymbolTable> {
    TABLE.get_or_init(|| Mutex::new(SymbolTable::new()))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Maximum number of distinct symbols the table will hand out.
///
/// Bounded at `u32::MAX - 1` because [`SymbolId::NONE`] is reserved
/// (`SymbolId(u32::MAX)`) and must never collide with a real
/// interned id.  Hitting this cap returns [`SymbolId::NONE`] —
/// callers should treat that as "intern failed" (untrusted input
/// flooded the table) and refuse to proceed.
///
/// 4 billion symbols is far above any plausible legitimate
/// program; an adversarial input that approaches this bound is
/// performing a memory-exhaustion attack and the cap stops it
/// before the table consumes the rest of the address space.
pub const MAX_SYMBOLS: u32 = u32::MAX - 1;

/// Intern `name` and return its [`SymbolId`].  Idempotent: calling
/// twice with the same string returns the same id.
///
/// `intern("")` always returns [`SymbolId::EMPTY`] — the empty
/// string is eagerly interned at table construction so its id is
/// stable across runs.
///
/// # Capacity limit
///
/// The table is bounded at [`MAX_SYMBOLS`] (`u32::MAX - 1`) entries
/// to (a) keep [`SymbolId::NONE`] reserved and (b) cap memory under
/// adversarial input.  Once full, [`intern`] returns
/// [`SymbolId::NONE`] for any **new** name; previously-interned
/// names continue to return their existing id.  Callers should
/// check `id.is_none()` and treat it as a runtime error.
pub fn intern(name: &str) -> SymbolId {
    // Fast path: avoid `name.to_string()` allocation if the name
    // is already interned.  HashMap's get accepts &str via Borrow.
    let mut t = table().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&id) = t.by_name.get(name) {
        return id;
    }
    // Capacity check — refuse to hand out the NONE sentinel.
    // `by_id.len()` is the next id we'd allocate; reject when
    // that would equal MAX_SYMBOLS (so the assigned id is at
    // most MAX_SYMBOLS - 1, leaving u32::MAX free as NONE).
    let next_idx = t.by_id.len();
    if next_idx >= MAX_SYMBOLS as usize {
        return SymbolId::NONE;
    }
    // Slow path: allocate the owned string once and store it in
    // both maps.
    let id = SymbolId(next_idx as u32);
    let owned = name.to_string();
    t.by_id.push(owned.clone());
    t.by_name.insert(owned, id);
    id
}

/// Return the interned name for `id`, or `None` if the id has
/// never been interned (or is [`SymbolId::NONE`]).
///
/// The returned string is a clone — the table holds the canonical
/// copy under a mutex, and exposing a borrow would require holding
/// the mutex for the lifetime of the borrow.
pub fn name_of(id: SymbolId) -> Option<String> {
    if id == SymbolId::NONE {
        return None;
    }
    let t = table().lock().unwrap_or_else(|e| e.into_inner());
    t.by_id.get(id.0 as usize).cloned()
}

/// Total number of interned symbols (including the empty-string at id 0).
///
/// Test/observability hook.  Production code should not depend on
/// this value.
pub fn len() -> usize {
    table().lock().unwrap_or_else(|e| e.into_inner()).by_id.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// These tests share global state with every other test that touches
// the intern table.  They use deterministic name prefixes
// (`__intern_test_*`) so they don't collide with names used in
// other tests, and they always intern fresh names rather than
// asserting on absolute ids.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_is_symbol_id_zero() {
        assert_eq!(intern(""), SymbolId::EMPTY);
        assert_eq!(intern(""), SymbolId(0));
    }

    #[test]
    fn name_of_empty_returns_empty_string() {
        assert_eq!(name_of(SymbolId::EMPTY).as_deref(), Some(""));
    }

    #[test]
    fn intern_is_idempotent() {
        let a = intern("__intern_test_idempotent");
        let b = intern("__intern_test_idempotent");
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_names_get_distinct_ids() {
        let a = intern("__intern_test_distinct_a");
        let b = intern("__intern_test_distinct_b");
        assert_ne!(a, b);
    }

    #[test]
    fn name_of_round_trips() {
        let id = intern("__intern_test_roundtrip");
        assert_eq!(name_of(id).as_deref(), Some("__intern_test_roundtrip"));
    }

    #[test]
    fn name_of_none_returns_none() {
        assert!(name_of(SymbolId::NONE).is_none());
    }

    #[test]
    fn len_reflects_at_least_the_empty_string() {
        // Touch the table so it's initialised.
        let _ = intern("");
        assert!(len() >= 1, "empty string should always be interned");
    }
}
