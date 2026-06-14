//! # `lang-runtime-core` — language-agnostic runtime substrate.
//!
//! Implementation of [LANG20 — Multi-Language Runtime
//! Architecture](../../specs/LANG20-multilang-runtime.md).
//!
//! This crate is the substrate every language frontend (Lisp,
//! Ruby, JS, Smalltalk, Perl, Tetrad, Twig, …) plugs into via the
//! [`LangBinding`] trait.  It owns:
//!
//! - The [`LangBinding`] trait — single contract every language
//!   implements (15 methods + 3 associated types + 1 const).
//! - The cross-language value-representation primitives
//!   ([`SymbolId`], [`BoxedReprToken`], [`ObjectHeader`]).
//! - The inline-cache machinery ([`InlineCache`], [`ICState`])
//!   generic over per-language entry shape.
//! - The deopt protocol types ([`FrameDescriptor`],
//!   [`RegisterEntry`], [`NativeLocation`]).
//! - The visitor traits ([`ValueVisitor`], [`RootVisitor`]) the
//!   collector and root scanner use.
//! - The shared error type ([`RuntimeError`]).
//!
//! ## What this PR ships (PR 1 of LANG20 §"Migration path")
//!
//! Trait skeleton + supporting types only.  No live GC, no
//! interpreter wiring, no real C ABI exports.  Subsequent PRs
//! progressively activate the substrate:
//!
//! | PR | Adds |
//! |----|------|
//! | **1 (this PR)** | `LangBinding` trait skeleton + supporting types |
//! | 2 | `LispyBinding` impl in new `lispy-runtime` crate |
//! | 3 | `twig-frontend` / `twig-vm` split |
//! | 4 | `vm-core` calls `LangBinding` for `call_indirect`, `cmp_eq`, `is_truthy` |
//! | 5 | `send` / `load_property` / `store_property` IIR opcodes + handlers |
//! | 6 | IC machinery wired into the dispatch path |
//! | 7+ | Deopt protocol, AOT-PGO, second binding (Ruby) |
//!
//! ## Usage (PR 1 surface)
//!
//! ```
//! use lang_runtime_core::{LangBinding, SymbolId, BoxedReprToken,
//!                        InlineCache, RuntimeError, BuiltinFn};
//!
//! // A language frontend implements LangBinding for its own type.
//! // PR 2 will ship `LispyBinding` as the first real impl.
//! ```
//!
//! ## Design references
//!
//! Every public item links back to the section in
//! [LANG20](../../specs/LANG20-multilang-runtime.md) that
//! specifies it, so readers can trace any decision back to the
//! design doc without scanning the codebase.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// ─── Module declarations ──────────────────────────────────────────────

pub mod binding;
pub mod deopt;
pub mod error;
pub mod ic;
pub mod object;
pub mod value;
pub mod visitor;

// ─── Public re-exports ────────────────────────────────────────────────
//
// Most callers want a single `use lang_runtime_core::*;` to reach
// the entire surface.  Re-exporting the most-used items at the
// crate root keeps that ergonomic.

pub use binding::{BuiltinFn, DispatchCx, LangBinding};
pub use deopt::{DeoptAnchor, FrameDescriptor, InlinedDeoptDescriptor, NativeLocation, RegisterEntry};
pub use error::RuntimeError;
pub use ic::{ClassId, ICId, ICInvalidator, ICState, InlineCache, MAX_PIC_ENTRIES};
pub use object::{header_flags, ObjectHeader};
pub use value::{BoxedReprToken, SymbolId};
pub use visitor::{RootVisitor, ValueVisitor};

// ─── Crate-level integration tests ────────────────────────────────────
//
// Module-level tests live next to their definitions; the tests
// here exercise the *combination* of types — an end-to-end check
// that the public surface composes cleanly.

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Assert the public surface compiles without dragging anything
    /// into the call site that PR 1 doesn't ship yet.  This test
    /// exists so PR 2 (which adds LispyBinding) discovers any
    /// missing re-exports immediately.
    #[test]
    fn public_surface_re_exports_are_complete() {
        // Compile-only: build a struct whose fields cover every
        // re-exported type.  If any disappears the test stops
        // compiling.
        struct SurfaceCheck {
            _symbol: SymbolId,
            _repr: BoxedReprToken,
            _header_size: usize,
            _ic_state: ICState,
            _ic_id: ICId,
            _class_id: ClassId,
            _frame: FrameDescriptor,
            _anchor: DeoptAnchor,
            _location: NativeLocation,
            _entry: RegisterEntry,
            _err: RuntimeError,
        }
        let _ = SurfaceCheck {
            _symbol: SymbolId::EMPTY,
            _repr: BoxedReprToken::BoxedRef,
            _header_size: std::mem::size_of::<ObjectHeader>(),
            _ic_state: ICState::Uninit,
            _ic_id: ICId(0),
            _class_id: ClassId(0),
            _frame: FrameDescriptor::new(0),
            _anchor: DeoptAnchor(0),
            _location: NativeLocation::Register(0),
            _entry: RegisterEntry {
                ir_name: "x".into(),
                location: NativeLocation::Register(0),
                repr: BoxedReprToken::I64Unboxed,
                speculated_type_tag: None,
            },
            _err: RuntimeError::NotCallable,
        };
    }

    /// LANG20 ABI commitment: the heap header is exactly 16 bytes.
    /// Re-asserted at the crate level so future refactors that
    /// move types between modules can't silently break the
    /// header layout.
    #[test]
    fn lang20_abi_invariant_object_header_is_16_bytes() {
        assert_eq!(std::mem::size_of::<ObjectHeader>(), 16);
    }

    /// LANG20 ABI commitment: SymbolId / ICId / ClassId are all
    /// 4-byte transparent newtypes around u32 so they pass through
    /// the C ABI without ceremony.
    #[test]
    fn lang20_abi_invariant_id_types_are_4_bytes() {
        assert_eq!(std::mem::size_of::<SymbolId>(), 4);
        assert_eq!(std::mem::size_of::<ICId>(), 4);
        assert_eq!(std::mem::size_of::<ClassId>(), 4);
    }

    /// LANG20 design choice: the IC's PIC width default is 4.
    /// Catches accidental retuning that would break per-language
    /// codegen assumptions about cache entry layout.
    #[test]
    fn lang20_default_pic_width_is_four() {
        assert_eq!(MAX_PIC_ENTRIES, 4);
    }
}
