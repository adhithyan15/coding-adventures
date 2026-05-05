//! Debug-mode hooks for the dispatch loop.
//!
//! When a [`DebugHooks`] implementation is supplied to
//! [`crate::dispatch::run_with_debug`], the dispatcher calls
//! [`DebugHooks::before_instruction`] **between every instruction** at
//! every recursion depth.  The hook owns whatever blocking-on-TCP logic
//! is needed to support pause / step / breakpoint semantics — the
//! dispatcher itself stays a tight loop over IIR instructions.
//!
//! ## Why a trait, not a concrete type
//!
//! The trait keeps `twig-vm` free of TCP / DAP concerns: those live in
//! [`crate::debug_server`] (the production hook) and in tests (mock
//! hooks that just count calls or assert on locations).  Three callers,
//! one signature.
//!
//! ## Call-stack reconstruction
//!
//! The hook receives `(fn_name, depth, pc, frame)` on every call and can
//! reconstruct the live call stack by tracking depth deltas:
//!
//! ```text
//! before_instruction(main, depth=0, pc=0)  → stack = [main:0]
//! before_instruction(foo,  depth=1, pc=0)  → push foo  → [main:?, foo:0]
//! before_instruction(foo,  depth=1, pc=3)  → update    → [main:?, foo:3]
//! before_instruction(main, depth=0, pc=4)  → pop foo   → [main:4]
//! ```
//!
//! `main:?` between the push and the pop is filled in by the most-
//! recently-reported pc for that depth.  Holding one entry per depth in
//! a `Vec<(String, usize)>` is a single 16-byte update per safepoint —
//! cheap.

use crate::dispatch::Frame;

// ---------------------------------------------------------------------------
// FrameView — read-only snapshot of one Frame
// ---------------------------------------------------------------------------

/// Read-only view into a [`Frame`] for the debug hook.
///
/// The underlying `Frame` is private to `dispatch`.  `FrameView` exposes
/// just the bits a debugger needs (variable lookup by name, name
/// enumeration) without leaking `Frame` itself or the
/// register-storage details.
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

    /// Return a printable representation of `name`'s current value, or
    /// `None` if the register is not bound in this frame.
    ///
    /// We return `String` rather than `LispyValue` to keep the
    /// debug-server / DAP surface free of `lispy-runtime` types.
    pub fn read_register(&self, name: &str) -> Option<String> {
        self.inner.debug_print(name)
    }
}

// ---------------------------------------------------------------------------
// DebugHooks trait
// ---------------------------------------------------------------------------

/// Sole interface between [`crate::dispatch`] and any debug agent.
///
/// `dispatch` calls [`Self::before_instruction`] right before executing
/// each IIR instruction.  Implementations are free to:
///
/// 1. Process incoming wire-protocol commands (set/clear breakpoint).
/// 2. Detect that the current `(fn_name, pc)` is a breakpoint and emit
///    a `stopped` event.
/// 3. **Block** on the wire (e.g. waiting for `continue` after a stop)
///    — the dispatcher resumes only when the hook returns.
/// 4. Implement single-step by remembering "stop on the *next*
///    instruction" between calls.
///
/// All blocking happens inside the hook; the dispatcher loop itself
/// stays linear and tight.
pub trait DebugHooks {
    /// Called before executing instruction `pc` in `fn_name`.
    ///
    /// `depth` is the number of *parent* frames currently on the stack
    /// (top-level main = 0, a function called from main = 1, …).
    ///
    /// `frame` exposes the local register state for variable
    /// inspection.  Implementations that don't need locals may
    /// ignore it.
    fn before_instruction(
        &mut self,
        fn_name: &str,
        depth: usize,
        pc: usize,
        frame: &FrameView<'_>,
    );

    /// Called once when `dispatch` is about to return from `fn_name`
    /// (whether via `ret`, falling off the end, or propagated error).
    ///
    /// The default impl is a no-op; servers that track call-stack
    /// state via depth deltas only generally don't override this.
    fn on_function_exit(&mut self, _fn_name: &str, _depth: usize) {}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Recording hook used by tests to assert `before_instruction` was
    /// called with the expected sequence of `(fn, pc, depth)` tuples.
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
            _frame: &FrameView<'_>,
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
