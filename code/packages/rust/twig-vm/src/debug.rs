//! Debug-mode glue between the twig-vm dispatcher and the generic
//! [`vm_debug`] substrate.
//!
//! ## What lives where (post-VMDEBUG01)
//!
//! - **`vm_debug` crate**: the generic [`DebugHooks`] trait, the
//!   [`DebugFrame`] trait, the [`StopReason`] enum, and the
//!   production TCP-backed [`DebugServer`].  Shared across every
//!   LANG-VM-based interpreter.
//! - **This module**: the concrete [`FrameView`] (a borrow of
//!   `dispatch::Frame`) plus its `impl vm_debug::DebugFrame`.  Keeps
//!   the link between twig-vm's private `Frame` type and the public
//!   debug surface in one place.
//!
//! For back-compat, this module also re-exports
//! [`vm_debug::DebugHooks`] so callers that wrote
//! `twig_vm::debug::DebugHooks` keep working.
//!
//! ## Why a trait, not a concrete type
//!
//! The trait keeps `twig-vm` free of TCP / DAP concerns: those live
//! in [`vm_debug::DebugServer`] (the production hook) and in tests
//! (mock hooks that just count calls or assert on locations).  Three
//! callers, one signature.
//!
//! ## Call-stack reconstruction
//!
//! The hook receives `(fn_name, depth, pc, frame)` on every call and
//! can reconstruct the live call stack by tracking depth deltas:
//!
//! ```text
//! before_instruction(main, depth=0, pc=0)  → stack = [main:0]
//! before_instruction(foo,  depth=1, pc=0)  → push foo  → [main:?, foo:0]
//! before_instruction(foo,  depth=1, pc=3)  → update    → [main:?, foo:3]
//! before_instruction(main, depth=0, pc=4)  → pop foo   → [main:4]
//! ```
//!
//! `main:?` between the push and the pop is filled in by the most-
//! recently-reported pc for that depth.  Holding one entry per depth
//! in a `Vec<(String, usize)>` is a single 16-byte update per
//! safepoint — cheap.

use crate::dispatch::Frame;

// Re-export so existing callers (`use twig_vm::debug::DebugHooks;`)
// keep working without changes.
pub use vm_debug::{DebugFrame, DebugHooks};

// ---------------------------------------------------------------------------
// FrameView — read-only snapshot of one Frame
// ---------------------------------------------------------------------------

/// Read-only view into a [`Frame`] for the debug hook.
///
/// The underlying `Frame` is private to `dispatch`.  `FrameView`
/// exposes just the bits a debugger needs (variable lookup by name,
/// name enumeration) without leaking `Frame` itself or the
/// register-storage details.
///
/// Implements [`vm_debug::DebugFrame`] so it can be passed to any
/// hook the generic [`DebugServer`](vm_debug::DebugServer) plugs
/// into.
pub struct FrameView<'a> {
    inner: &'a Frame,
}

impl<'a> FrameView<'a> {
    /// Construct from a borrow of `Frame`.
    ///
    /// `pub(crate)` so only `dispatch` can build one.
    pub(crate) fn new(inner: &'a Frame) -> Self {
        FrameView { inner }
    }

    /// All register names live in the frame.
    ///
    /// Names appear in HashMap iteration order — consumers that need
    /// stable order should sort.
    pub fn register_names(&self) -> Vec<String> {
        self.inner.register_names()
    }

    /// Return a printable representation of `name`'s current value,
    /// or `None` if the register is not bound in this frame.
    ///
    /// We return `String` rather than `LispyValue` to keep the
    /// debug-server / DAP surface free of `lispy-runtime` types.
    pub fn read_register(&self, name: &str) -> Option<String> {
        self.inner.debug_print(name)
    }
}

impl<'a> vm_debug::DebugFrame for FrameView<'a> {
    fn register_names(&self) -> Vec<String> {
        FrameView::register_names(self)
    }
    fn read_register(&self, name: &str) -> Option<String> {
        FrameView::read_register(self, name)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Recording hook used by tests to assert `before_instruction`
    /// was called with the expected sequence of `(fn, pc, depth)`
    /// tuples.
    #[derive(Default)]
    struct RecordingHook {
        events: Arc<Mutex<Vec<(String, usize, usize)>>>,
    }

    impl DebugHooks for RecordingHook {
        fn before_instruction(
            &mut self,
            fn_name: &str,
            depth: usize,
            pc: usize,
            _frame: &dyn DebugFrame,
        ) {
            self.events.lock().unwrap().push((fn_name.to_string(), depth, pc));
        }
    }

    #[test]
    fn recording_hook_default_compiles() {
        // Only here so the trait's required signature is type-checked.
        let _h = RecordingHook::default();
    }
}
