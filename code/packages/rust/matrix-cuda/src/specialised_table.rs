//! `SpecialisedTable` — per-`CudaExecutor` table of installed
//! specialised kernel closures, keyed by the opaque `u64` handle a
//! backend `Specialiser` emits.
//!
//! Mirrors `matrix_metal::SpecialisedTable` and (further upstream)
//! `matrix_cpu::SpecialisedTable`.  The structural shape is identical
//! (a `HashMap<u64, Box<dyn Fn>>`); the closure signature differs
//! because each backend's closure captures backend-specific state:
//!
//! - `matrix-cpu` closures take `&mut BufferStore` (host-resident
//!   slices).
//! - `matrix-metal` closures take a `DispatchCtx<'_>` that bundles
//!   the device, command queue, and pipeline cache.
//! - `matrix-cuda` closures take `(&CudaDevice, &mut BufferStore,
//!   &[BufferId], &[BufferId])`.  The simpler signature works because
//!   a CUDA closure typically captures its own `CudaFunction` and
//!   launches it directly through the device handle.
//!
//! # Cross-platform
//!
//! The table is callable on every platform.  `CudaBuffer` is `Send`
//! (cuda-compute v0.1.1) and `CudaFunction` is `Send + Sync`
//! (cuda-compute v0.1.2), so a `Box<dyn Fn(...) + Send>` closure can
//! capture either freely.  Non-NVIDIA hosts simply never install
//! anything — the table stays empty.

use compute_ir::BufferId;
use cuda_compute::CudaDevice;
use executor_protocol::OpTiming;
use std::collections::HashMap;

use crate::BufferStore;

/// Closure signature for a CUDA-side specialised kernel.
///
/// Takes:
/// - `&CudaDevice` — for launching kernels and synchronising.
/// - `&mut BufferStore` — for resolving `BufferId`s to `CudaBuffer`s
///   and (less commonly) allocating scratch space.
/// - `inputs` / `outputs` — the runtime-supplied `BufferId`s from
///   the `DispatchSpecialised` request, in slot order.
///
/// Returns either per-op timings (Phase 5+) or a human-readable
/// error.  Closures should be **pure with respect to captured
/// state** — the observer model behind specialisation breaks if the
/// kernel mutates closure-captured values across calls.  All
/// mutation goes through the explicit `&mut BufferStore` argument.
///
/// ## Why `Send` but not `Sync`
///
/// The closure runs only while the executor holds `Mutex<State>`,
/// so `Sync` would force callers to add a wrapper lock with no
/// real safety benefit.  This matches `matrix-metal`'s policy.
pub type CudaSpecialisedKernelFn = dyn for<'a> Fn(
        &'a CudaDevice,
        &'a mut BufferStore,
        &[BufferId],
        &[BufferId],
    ) -> Result<Vec<OpTiming>, String>
    + Send;

/// Per-executor table of installed specialised kernel closures.
///
/// Wraps the `HashMap` so we have a natural place to:
/// - document invariants (install replaces, allowing
///   Phase 5 deoptimisation to swap a closure without an explicit
///   evict-then-install dance);
/// - hang a `Debug` impl that hides closure pointers (only the
///   installed handles, in decimal, are printed);
/// - choke-point future changes (LRU caps, telemetry) without
///   touching every call site.
#[derive(Default)]
pub struct SpecialisedTable {
    /// `handle → kernel closure`.  Boxed so the table is sized.
    kernels: HashMap<u64, Box<CudaSpecialisedKernelFn>>,
}

impl SpecialisedTable {
    /// Construct an empty table.
    pub fn new() -> Self {
        SpecialisedTable {
            kernels: HashMap::new(),
        }
    }

    /// Install a kernel under `handle`.  Overwrites any prior
    /// closure at the same handle.  Overwriting is intentional so
    /// MX05 Phase 5 deoptimisation can swap a closure for a fresh
    /// one without a separate `evict-then-install` step — same
    /// contract as `matrix-metal`'s table.
    pub fn install(&mut self, handle: u64, kernel: Box<CudaSpecialisedKernelFn>) {
        self.kernels.insert(handle, kernel);
    }

    /// Look up a kernel by handle.  Returns `None` if the handle
    /// was never installed (or has since been evicted).
    pub fn get(&self, handle: u64) -> Option<&CudaSpecialisedKernelFn> {
        self.kernels.get(&handle).map(|b| b.as_ref())
    }

    /// True iff the handle is installed.
    pub fn contains(&self, handle: u64) -> bool {
        self.kernels.contains_key(&handle)
    }

    /// Number of installed kernels.
    pub fn len(&self) -> usize {
        self.kernels.len()
    }

    /// Whether the table has no installed kernels.
    pub fn is_empty(&self) -> bool {
        self.kernels.is_empty()
    }

    /// **MX05 Phase 5 deoptimisation hook.**  Evict the kernel
    /// under `handle`.  Returns `true` if an entry was removed.
    ///
    /// Used when an observation reveals that a previously-folded
    /// constant has changed — the compiled `CudaModule` in the
    /// closure encodes the *old* constant and is now wrong, so the
    /// runtime drops it.  The driver releases the underlying PTX
    /// module when the boxed closure drops (it owns the
    /// `CudaModule` Arc).
    pub fn evict(&mut self, handle: u64) -> bool {
        self.kernels.remove(&handle).is_some()
    }
}

impl std::fmt::Debug for SpecialisedTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut handles: Vec<u64> = self.kernels.keys().copied().collect();
        handles.sort();
        f.debug_struct("SpecialisedTable")
            .field("installed_handles", &handles)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A no-op kernel closure for tests.  Returns an empty timings
    /// vec.  Closures capture nothing, so the test doesn't depend
    /// on `CudaDevice` being constructible (and therefore runs on
    /// every platform).
    fn dummy_kernel() -> Box<CudaSpecialisedKernelFn> {
        Box::new(|_dev, _buf, _ins, _outs| Ok(Vec::new()))
    }

    #[test]
    fn new_table_is_empty() {
        let t = SpecialisedTable::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn default_matches_new() {
        let a = SpecialisedTable::new();
        let b = SpecialisedTable::default();
        assert_eq!(a.is_empty(), b.is_empty());
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn install_then_lookup_finds_kernel() {
        let mut t = SpecialisedTable::new();
        assert!(!t.contains(0xABCD));
        t.install(0xABCD, dummy_kernel());
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
        t.install(42, dummy_kernel());
        t.install(42, Box::new(|_, _, _, _| Err("v2".to_string())));
        // Re-install replaces; len stays at 1.
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn evict_removes_kernel_and_returns_true() {
        let mut t = SpecialisedTable::new();
        t.install(7, dummy_kernel());
        assert!(t.evict(7));
        assert!(!t.contains(7));
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn evict_missing_handle_returns_false() {
        let mut t = SpecialisedTable::new();
        assert!(!t.evict(0xBAD));
    }

    /// `SpecialisedTable` must be `Send` so it can live inside the
    /// executor's `Mutex<State>` (and the executor stays `Sync`
    /// because `Mutex<T>: Sync where T: Send`).
    #[test]
    fn specialised_table_is_send() {
        fn require_send<T: Send>() {}
        require_send::<SpecialisedTable>();
    }

    #[test]
    fn debug_impl_shows_handles_in_sorted_order() {
        let mut t = SpecialisedTable::new();
        t.install(0xCAFE, dummy_kernel());
        t.install(0x42, dummy_kernel());
        let printed = format!("{:?}", t);
        // 0x42 (66) appears before 0xCAFE (51966) when sorted.
        let idx_66 = printed.find("66").unwrap();
        let idx_caf = printed.find("51966").unwrap();
        assert!(idx_66 < idx_caf, "got: {}", printed);
    }
}
