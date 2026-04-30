//! # `lispy-runtime` — `LangBinding` for Lisp/Scheme/Twig/Clojure.
//!
//! Implementation of LANG20 PR 2 from the
//! [migration path](../../specs/LANG20-multilang-runtime.md).  Ships
//! the **first concrete `LangBinding` implementation** plus the
//! supporting heap object types, intern table, builtins, and
//! `extern "C"` ABI surface that any Lispy frontend (Twig, Lisp,
//! Scheme, Clojure) consumes unchanged.
//!
//! ## Pipeline (Lispy frontends today)
//!
//! ```text
//! Twig / Lisp / Scheme / Clojure source
//!         │
//!         ▼  per-frontend lexer / parser / IR-emitter
//! IIRModule (LANG01)
//!         │
//!         ▼  vm-core (LANG02) — wired via LispyBinding in PR 4
//! execution
//! ```
//!
//! Each frontend reuses **everything in this crate** — `LispyValue`,
//! `LispyBinding`, all builtins, the `lispy_*` C ABI — and only
//! writes its own AST → IIR step.
//!
//! ## What this PR ships
//!
//! - [`LispyValue`] — the tagged-i64 value representation
//!   (immediate ints / nil / true / false / symbols + heap-tagged
//!   cons / closure pointers).  Exactly 8 bytes, asserted at
//!   compile time.
//! - [`LispyBinding`] — full `LangBinding` impl: `type_tag`,
//!   `class_of`, `is_truthy`, `equal`, `identical`, `hash`,
//!   `trace_object`, `trace_value`, `materialize_value`,
//!   `box_value`, plus the per-language method/property/builtin
//!   methods (Lispy doesn't have method dispatch, so `send` /
//!   `load_property` / `store_property` return `RuntimeError`).
//! - [`heap`] — `ConsCell` and `Closure` `#[repr(C)]` types with
//!   the LANG20 16-byte header; `alloc_cons` / `alloc_closure`
//!   factory functions (PR 2: `Box::leak`; PR 4+: real GC).
//! - [`intern`] — process-global symbol intern table.
//! - [`builtins`] — TW00 builtin handlers (`+`, `-`, `*`, `/`,
//!   `=`, `<`, `>`, `cons`, `car`, `cdr`, `null?`, `pair?`,
//!   `number?`, `symbol?`, `print`).
//! - [`abi`] — `extern "C"` surface (`lispy_cons`, `lispy_car`,
//!   `lispy_cdr`, `lispy_make_symbol`, `lispy_make_closure`,
//!   `lispy_apply_closure`).
//!
//! ## What this PR does NOT ship
//!
//! - Live GC integration — the allocator leaks; `gc-core` (LANG16)
//!   wires the real collector.
//! - Closure dispatch — `apply_callable` returns a placeholder
//!   error.  PR 4 (vm-core wiring) makes it functional.
//! - `send` / `load_property` / `store_property` opcode handlers —
//!   Lispy doesn't use these, so the binding correctly returns
//!   `NoSuchMethod` / `NoSuchProperty`.  IIR opcode handlers in
//!   vm-core land in PR 5 once Ruby/JS frontends need them.
//! - Real-runtime backends (JVM/CLR/BEAM/WASM) — those remain on
//!   the host-runtime path per LANG20 §"Compilation paths" and
//!   are unchanged by this PR.
//!
//! ## Where this crate sits
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │ Lispy frontends: twig-frontend, lisp-frontend,         │
//! │                  scheme-frontend, clojure-frontend     │
//! └─────────────────────────────┬──────────────────────────┘
//!                               │ uses
//!                               ▼
//! ┌────────────────────────────────────────────────────────┐
//! │ lispy-runtime (this crate)                             │
//! │   - LispyValue (tagged i64)                            │
//! │   - LispyBinding (impl LangBinding)                    │
//! │   - ConsCell / Closure heap types                      │
//! │   - Intern table                                       │
//! │   - Builtins                                           │
//! │   - extern "C" ABI                                     │
//! └─────────────────────────────┬──────────────────────────┘
//!                               │ implements trait from
//!                               ▼
//! ┌────────────────────────────────────────────────────────┐
//! │ lang-runtime-core (LANG20 PR 1)                        │
//! │   - LangBinding trait                                  │
//! │   - ObjectHeader, SymbolId, BoxedReprToken, …          │
//! └────────────────────────────────────────────────────────┘
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod abi;
pub mod binding;
pub mod builtins;
pub mod heap;
pub mod intern;
pub mod value;

// Public re-exports — keep the headline surface easy to discover.
pub use binding::{LispyBinding, LispyClass, LispyICEntry};
pub use heap::{
    alloc_closure, alloc_cons, as_closure, car, cdr, is_closure, is_cons, Closure, ConsCell,
    CLASS_CLOSURE, CLASS_CONS,
};
pub use intern::{intern, name_of};
pub use value::{LispyValue, TAG_BITS, TAG_FALSE, TAG_HEAP, TAG_INT, TAG_NIL, TAG_SYMBOL, TAG_TRUE};

// ---------------------------------------------------------------------------
// Crate-level integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;
    use lang_runtime_core::LangBinding;

    /// LANG20 ABI commitment: every binding's Value must be
    /// exactly 8 bytes.  Re-asserted at the crate level to catch
    /// future refactors that move types between modules.
    #[test]
    fn lispy_binding_value_is_8_bytes() {
        assert_eq!(std::mem::size_of::<<LispyBinding as LangBinding>::Value>(), 8);
    }

    /// LANG20 invariant: `LANGUAGE_NAME` is lower-snake-case ASCII.
    #[test]
    fn lispy_language_name_is_correct() {
        assert_eq!(LispyBinding::LANGUAGE_NAME, "lispy");
    }

    /// End-to-end check: build a small data structure through the
    /// public API, then verify the binding's introspection methods
    /// agree.  Catches re-export bugs and module-coupling issues.
    #[test]
    fn build_and_introspect_data_structure() {
        // Build (foo . (1 2)) — a cons of a symbol with a list.
        let foo = LispyValue::symbol(intern("foo"));
        let two = alloc_cons(LispyValue::int(2), LispyValue::NIL);
        let one_two = alloc_cons(LispyValue::int(1), two);
        let pair = alloc_cons(foo, one_two);

        // Introspection through the binding.
        assert_eq!(LispyBinding::class_of(pair), Some(LispyClass::Cons));
        assert_eq!(LispyBinding::class_of(foo), Some(LispyClass::Symbol));

        // Walk the structure.  SAFETY: every value here came from
        // the crate's allocators (alloc_cons / int / symbol).
        unsafe {
            assert_eq!(car(pair), Some(foo));
            let tail = cdr(pair).unwrap();
            assert_eq!(car(tail), Some(LispyValue::int(1)));
            assert_eq!(car(cdr(tail).unwrap()), Some(LispyValue::int(2)));
        }
    }

    /// End-to-end: resolve a builtin through the binding and call
    /// it.  Proves the builtin module composes with the binding.
    #[test]
    fn resolve_and_call_builtin_through_binding() {
        let cons_fn = LispyBinding::resolve_builtin("cons").unwrap();
        let pair = cons_fn(&[LispyValue::int(1), LispyValue::int(2)]).unwrap();
        // SAFETY: `pair` came from the cons builtin (alloc_cons).
        unsafe {
            assert_eq!(car(pair), Some(LispyValue::int(1)));
            assert_eq!(cdr(pair), Some(LispyValue::int(2)));
        }
    }
}
