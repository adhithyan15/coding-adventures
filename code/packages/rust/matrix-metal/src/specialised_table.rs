//! `SpecialisedTable` — per-`MetalExecutor` table of installed
//! specialised kernel closures, keyed by the opaque `u64` handle that
//! a backend [`Specialiser`] emits.
//!
//! This is the matrix-metal analog of `matrix_cpu::SpecialisedTable`,
//! which landed in MX05 Phase 4.1.  The structure is identical (a
//! `HashMap<u64, Box<dyn Fn>>`) but the closure signature differs:
//! matrix-cpu kernels take a `&mut BufferStore`; matrix-metal kernels
//! take the full [`DispatchCtx`] so they can encode a Metal command
//! buffer through the same path the generic dispatcher uses.
//!
//! # Apple-only
//!
//! Compiled MSL pipelines exist only on Apple targets, so this whole
//! module is gated on `cfg(target_vendor = "apple")`.  The non-Apple
//! build replaces it with a stub at the `lib.rs` level.
//!
//! [`Specialiser`]: matrix_profile::Specialiser
//! [`DispatchCtx`]: crate::dispatch::DispatchCtx

#![cfg(target_vendor = "apple")]

use crate::dispatch::DispatchCtx;
use compute_ir::BufferId;
use executor_protocol::OpTiming;
use std::collections::HashMap;

/// Closure signature for a metal-side specialised kernel.
///
/// Takes the `DispatchCtx` (so the closure has access to the
/// device, queue, buffers, and base pipeline cache), the
/// runtime-supplied `inputs` and `outputs` buffer ids from the
/// `DispatchSpecialised` request, and returns either per-op timings
/// or a human-readable error string.
///
/// ## Why `Fn`, not `FnMut`
///
/// A specialised kernel is logically pure w.r.t. its captured
/// environment: the same `(handle, inputs, outputs)` call must
/// produce the same effect every invocation, otherwise the observer
/// model behind specialisation breaks.  `Fn` enforces that
/// statically — kernels cannot mutate captured state across calls.
/// All mutation routes through the `&mut DispatchCtx` argument,
/// which is per-call and explicit.
///
/// ## Why `Send` but **not** `Sync`
///
/// The matrix-cpu analog requires `Send + Sync` because every
/// captured type (`Vec<u8>`, `HashMap`) is naturally `Sync`.  On
/// metal the closure typically captures a `MetalComputePipeline`,
/// which wraps a raw `*mut objc_bridge::Object` and is `Send` but
/// **not** `Sync` (the Objective-C runtime serialises access to a
/// pipeline state object internally).  That's fine for our use
/// case: the closure only ever runs while holding the executor's
/// `Mutex<State>`, and `Mutex<T>: Sync where T: Send`.  Requiring
/// `Sync` on the closure would force callers to wrap the pipeline
/// in a `Mutex` just to satisfy the bound, which would cost an
/// extra lock per dispatch with no real safety benefit.
///
/// ## Why `for<'a>` instead of a concrete lifetime
///
/// The dispatcher constructs a fresh `DispatchCtx<'_>` per request
/// borrowing from `MetalExecutor::state`'s fields.  A higher-ranked
/// bound lets a single closure be invoked with whatever lifetime
/// the executor's mutex guard happens to hand it.
pub type MetalSpecialisedKernelFn = dyn for<'a> Fn(
        &mut DispatchCtx<'a>,
        &[BufferId],
        &[BufferId],
    ) -> Result<Vec<OpTiming>, String>
    + Send;

/// Per-executor table of installed specialised kernel closures.
///
/// Wraps the underlying map so we have a natural place to:
/// - document invariants (install is monotonic in v0.6.0 — no
///   eviction; Phase 5 deoptimisation will add eviction);
/// - hang a `Debug` impl that hides the closure pointers;
/// - choke-point future changes (LRU caps, telemetry) without
///   touching every call site.
#[derive(Default)]
pub struct SpecialisedTable {
    /// `handle → kernel closure`.  Boxed so the table is sized.
    kernels: HashMap<u64, Box<MetalSpecialisedKernelFn>>,
}

impl SpecialisedTable {
    /// Construct an empty table.
    pub fn new() -> Self {
        SpecialisedTable {
            kernels: HashMap::new(),
        }
    }

    /// Install a kernel under `handle`.  Overwrites any prior closure
    /// at the same handle.  Overwriting is intentionally allowed so
    /// Phase 5 deoptimisation can swap a closure for a fresh one
    /// without a separate `evict-then-install` dance.
    pub fn install(&mut self, handle: u64, kernel: Box<MetalSpecialisedKernelFn>) {
        self.kernels.insert(handle, kernel);
    }

    /// Look up a kernel by handle.  Returns `None` if the handle was
    /// never installed.
    pub fn get(&self, handle: u64) -> Option<&MetalSpecialisedKernelFn> {
        self.kernels.get(&handle).map(|b| b.as_ref())
    }

    /// True iff the handle is installed.
    pub fn contains(&self, handle: u64) -> bool {
        self.kernels.contains_key(&handle)
    }

    /// Number of installed kernels.  Useful for tests and metrics.
    pub fn len(&self) -> usize {
        self.kernels.len()
    }

    /// **MX05 Phase 5.**  Evict the kernel under `handle`.  Returns
    /// `true` if an entry was removed.  Used by the deoptimisation
    /// path: when an observation reveals that a previously-folded
    /// constant has changed, the compiled `MetalComputePipeline` in
    /// the closure encodes the *old* constant and is now wrong, so
    /// the runtime drops it.  The Metal driver releases the pipeline
    /// when the boxed closure drops.
    pub fn evict(&mut self, handle: u64) -> bool {
        self.kernels.remove(&handle).is_some()
    }

    /// Whether the table has no installed kernels.
    pub fn is_empty(&self) -> bool {
        self.kernels.is_empty()
    }
}

impl std::fmt::Debug for SpecialisedTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let handles: Vec<u64> = self.kernels.keys().copied().collect();
        f.debug_struct("SpecialisedTable")
            .field("installed_handles", &handles)
            .finish()
    }
}

// ────────────────────────── tests ──────────────────────────
//
// Tests here are Apple-only because the underlying `DispatchCtx`
// references a Metal device.  Non-Apple test coverage of the
// install/lookup semantics lives in matrix-cpu, which has the same
// shape minus the Metal types.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_then_lookup_finds_kernel() {
        let mut t = SpecialisedTable::new();
        assert!(!t.contains(0xABCD));
        t.install(0xABCD, Box::new(|_, _, _| Ok(vec![])));
        assert!(t.contains(0xABCD));
        assert_eq!(t.len(), 1);
        assert!(t.get(0xABCD).is_some());
    }

    #[test]
    fn lookup_of_missing_handle_returns_none() {
        let t = SpecialisedTable::new();
        assert!(t.get(0xFEED).is_none());
        assert!(!t.contains(0xFEED));
    }

    #[test]
    fn install_overwrites_prior_kernel() {
        let mut t = SpecialisedTable::new();
        t.install(42, Box::new(|_, _, _| Ok(vec![])));
        t.install(42, Box::new(|_, _, _| Err("v2".to_string())));
        // Re-install replaces — len stays 1.
        assert_eq!(t.len(), 1);
    }

    /// `SpecialisedTable` must be `Send` so it can live inside the
    /// executor's `Mutex<State>` (and the executor stays `Sync`
    /// because `Mutex<T>: Sync where T: Send`).  We don't require
    /// `Sync` on the table itself — see the rustdoc on
    /// `MetalSpecialisedKernelFn` for why metal kernels are `Send`-only.
    #[test]
    fn specialised_table_is_send() {
        fn require_send<T: Send>() {}
        require_send::<SpecialisedTable>();
    }

    #[test]
    fn debug_impl_shows_handles_not_pointers() {
        let mut t = SpecialisedTable::new();
        t.install(0xCAFE, Box::new(|_, _, _| Ok(vec![])));
        let printed = format!("{:?}", t);
        assert!(printed.contains("51966"), "should contain decimal 0xCAFE: {}", printed);
    }
}
