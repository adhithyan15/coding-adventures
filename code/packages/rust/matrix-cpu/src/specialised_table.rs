//! `SpecialisedTable` — per-`CpuExecutor` table of installed specialised
//! kernel closures, keyed by the opaque `u64` handle that
//! [`CpuSpecialiser`] emits.
//!
//! # MX05 Phase 4.1 — what this module unlocks
//!
//! Up to Phase 4 the pipeline looked like this:
//!
//! ```text
//!   sampler  →  policy  →  SpecRouter  →  CpuSpecialiser
//!                                              │
//!                                              ▼
//!                                          SpecCache
//!                                          (handle stored,
//!                                           never invoked)
//! ```
//!
//! The cache filled up but no one *did* anything with the handles —
//! the dispatch path still walked the generic `ComputeGraph` for
//! every op.  Phase 4.1 closes that loop:
//!
//! ```text
//!   runtime  ──DispatchSpecialised(handle, in, out)──►  CpuExecutor
//!                                                        │
//!                                          handle ──►  SpecialisedTable
//!                                                        │
//!                                                        ▼
//!                                              Box<dyn SpecialisedKernelFn>
//!                                                        │
//!                                                        ▼
//!                                                  reads inputs,
//!                                                  writes outputs,
//!                                                  returns timings
//! ```
//!
//! Phase 4.1 is intentionally minimum-viable on the *kernel* side —
//! the closures we install in tests are simple byte-copies and
//! single-op replays.  What this lands is the **plumbing**: the
//! protocol-level `DispatchSpecialised` request, which already exists
//! at the wire layer, now has an executor that answers `DispatchDone`
//! instead of `NOT_IMPLEMENTED`.
//!
//! Later phases extend the closures themselves:
//!
//! - Phase 4.2: matrix-metal emits MSL strings keyed by handle and
//!   compiles to `MetalComputePipelineState`.
//! - Phase 5:   deoptimisation — closures self-invalidate when an
//!   observed assumption proves wrong, and the table evicts them.
//!
//! # Closure signature
//!
//! ```ignore
//! fn(&mut BufferStore, &[BufferId], &[BufferId]) -> Result<Vec<OpTiming>, String>
//! ```
//!
//! The closure gets `&mut BufferStore` so it can read/write tensor
//! bytes directly — no copies, no extra protocol round-trips.  It
//! gets the runtime-supplied input/output `BufferId` lists from the
//! `DispatchSpecialised` request payload.  It returns per-op timings
//! using the same `OpTiming` shape as the generic `Dispatch` path,
//! so the runtime's downstream profiler doesn't need a separate code
//! path for specialised dispatches.
//!
//! # Why a HashMap
//!
//! Handles are 64-bit FNV-1a hashes of `SpecKey` — sparse and
//! deterministic but not contiguous.  `HashMap<u64, _>` is the right
//! shape; the table is read-mostly (one install per cache miss, many
//! dispatches per install) so hashing cost on lookup is negligible
//! relative to the kernel itself.
//!
//! # Send + Sync
//!
//! The boxed closure must be `Send + Sync` because `CpuExecutor`
//! lives behind an `Arc<Mutex<_>>` and may be invoked from any
//! thread.  Bounded by `dyn Fn(...) + Send + Sync`.

use crate::buffers::BufferStore;
use compute_ir::BufferId;
use executor_protocol::OpTiming;
use std::collections::HashMap;

/// Boxed signature for a specialised CPU kernel.  See the module docs
/// for what each argument means.
///
/// ## Why `Fn`, not `FnMut`
///
/// A specialised kernel is logically pure with respect to its
/// closure environment: the same `(handle, inputs, outputs)` call
/// must produce the same effect every invocation, otherwise the
/// observer model behind specialisation breaks.  `Fn` enforces that
/// statically — the kernel cannot mutate captured state across calls.
/// All mutation goes through the `&mut BufferStore` argument, which
/// is per-call and explicit.
pub type SpecialisedKernelFn =
    dyn Fn(&mut BufferStore, &[BufferId], &[BufferId]) -> Result<Vec<OpTiming>, String>
        + Send
        + Sync;

/// Per-executor table of installed specialised kernel closures.
///
/// Wrapping the underlying map in a named type (rather than a bare
/// `HashMap`) gives us:
///
/// - A natural place to hang invariants (e.g. "installation is
///   monotonic in V1 — no eviction") and document them.
/// - A choke point where Phase 5 deoptimisation can add an `evict()`
///   method without touching every call site.
/// - A `Debug` impl that hides the closures' pointer values, which
///   are noisy and unstable across builds.
#[derive(Default)]
pub struct SpecialisedTable {
    /// `handle → kernel closure`.  Boxed so the table is sized.
    kernels: HashMap<u64, Box<SpecialisedKernelFn>>,
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
    /// future deoptimisation can swap a closure for a fresh one
    /// without a separate `evict-then-install` dance.
    pub fn install(&mut self, handle: u64, kernel: Box<SpecialisedKernelFn>) {
        self.kernels.insert(handle, kernel);
    }

    /// Look up a kernel by handle.  Returns `None` if the handle was
    /// never installed.
    pub fn get(&self, handle: u64) -> Option<&SpecialisedKernelFn> {
        self.kernels.get(&handle).map(|boxed| boxed.as_ref())
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
    /// path when a previously-stable observation reveals a new
    /// distinct value — the cached closure encodes the *old*
    /// constant and is now wrong, so we drop it and let subsequent
    /// dispatches fall back to the generic path until/unless the
    /// policy re-stabilises.
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
        // Show just the handle set; closure addresses are noise.
        let handles: Vec<u64> = self.kernels.keys().copied().collect();
        f.debug_struct("SpecialisedTable")
            .field("installed_handles", &handles)
            .finish()
    }
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Install a no-op kernel and verify it can be looked up by handle.
    #[test]
    fn install_then_lookup_finds_kernel() {
        let mut t = SpecialisedTable::new();
        assert!(!t.contains(0xABCD));
        t.install(0xABCD, Box::new(|_, _, _| Ok(vec![])));
        assert!(t.contains(0xABCD));
        assert_eq!(t.len(), 1);
        assert!(t.get(0xABCD).is_some());
    }

    /// Lookup of a never-installed handle returns `None` cleanly —
    /// the executor will use this to fall back to `NOT_IMPLEMENTED`.
    #[test]
    fn lookup_of_missing_handle_returns_none() {
        let t = SpecialisedTable::new();
        assert!(t.get(0xFEED).is_none());
        assert!(!t.contains(0xFEED));
    }

    /// Re-installing the same handle is allowed and replaces the closure.
    /// This is the path Phase 5 deoptimisation will use.
    #[test]
    fn install_overwrites_prior_kernel() {
        let mut t = SpecialisedTable::new();
        let calls_v1 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_v2 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        {
            let c = calls_v1.clone();
            t.install(
                42,
                Box::new(move |_, _, _| {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(vec![])
                }),
            );
        }
        // First invocation runs v1.
        let mut buffers = BufferStore::new();
        let _ = t.get(42).unwrap()(&mut buffers, &[], &[]);
        assert_eq!(calls_v1.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Overwrite with v2; subsequent calls hit v2 only.
        {
            let c = calls_v2.clone();
            t.install(
                42,
                Box::new(move |_, _, _| {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(vec![])
                }),
            );
        }
        let _ = t.get(42).unwrap()(&mut buffers, &[], &[]);
        assert_eq!(calls_v1.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(calls_v2.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// A kernel can read input buffers and write output buffers via
    /// the `&mut BufferStore` argument.  This is the path real
    /// specialised kernels will use.
    #[test]
    fn kernel_can_read_inputs_and_write_outputs() {
        let mut buffers = BufferStore::new();
        buffers.alloc(BufferId(1), 4);
        buffers.alloc(BufferId(2), 4);
        buffers.write(BufferId(1), 0, &[7, 8, 9, 10]).unwrap();

        let mut t = SpecialisedTable::new();
        // Identity-copy kernel: read input[0], write to output[0].
        t.install(
            99,
            Box::new(|bufs, inputs, outputs| {
                let src = bufs.read(inputs[0], 0, 4)?;
                bufs.write(outputs[0], 0, &src)?;
                Ok(vec![OpTiming { op_index: 0, ns: 0 }])
            }),
        );

        let timings = t.get(99).unwrap()(&mut buffers, &[BufferId(1)], &[BufferId(2)]).unwrap();
        assert_eq!(timings.len(), 1);
        let out = buffers.read(BufferId(2), 0, 4).unwrap();
        assert_eq!(out, vec![7, 8, 9, 10]);
    }

    /// A kernel that fails returns its error to the caller — the
    /// executor will translate this into an `Error` response with
    /// `RUNTIME_ERROR`, matching the generic `Dispatch` failure path.
    #[test]
    fn kernel_error_propagates() {
        let mut t = SpecialisedTable::new();
        t.install(
            0,
            Box::new(|_, _, _| Err("intentional failure".to_string())),
        );
        let mut buffers = BufferStore::new();
        let r = t.get(0).unwrap()(&mut buffers, &[], &[]);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "intentional failure");
    }

    /// `SpecialisedTable` must be `Send + Sync` so `CpuExecutor`'s
    /// `Mutex<State>` stays `Send + Sync`.
    #[test]
    fn specialised_table_is_send_sync() {
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<SpecialisedTable>();
    }

    /// `Debug` impl prints handles, not closure pointers.
    #[test]
    fn debug_impl_shows_handles_not_pointers() {
        let mut t = SpecialisedTable::new();
        t.install(0xCAFE, Box::new(|_, _, _| Ok(vec![])));
        let printed = format!("{:?}", t);
        assert!(printed.contains("51966"), "should contain decimal 0xCAFE: {}", printed);
    }
}
