//! Graph-runner helper for image-gpu-core.
//!
//! Each public op builds a `matrix_ir::Graph` describing its
//! computation, then hands it to this module to plan + dispatch.
//!
//! ## V1 design: runtime inputs as constants
//!
//! The matrix-runtime planner assigns its own BufferIds when it lowers
//! a Graph to a ComputeGraph.  matrix-cpu's executor uses sequential
//! ids when servicing AllocBuffer requests.  These two id-spaces don't
//! coordinate without a protocol extension.
//!
//! For V1 we sidestep the mismatch by embedding **runtime inputs as
//! constants** in the matrix-ir Graph: at dispatch time, the executor
//! pre-uploads each constant's bytes to its declared `residency.buffer`
//! (the planner-assigned id).  The graph then proceeds with that data
//! already in place, runs the ops, and we download the output by
//! looking up its end-of-graph residency.
//!
//! Both `matrix-cpu` and `matrix-metal` implement this pre-upload
//! protocol identically — their `Dispatch` handlers walk
//! `graph.constants` and write the bytes into freshly-allocated buffers
//! whose ids match `c.residency.buffer`.  That's what makes the same
//! image-gpu-core graph runnable on either backend.
//!
//! ## V1 design: single-executor dispatch
//!
//! The matrix-runtime planner can place different ops on different
//! executors (CPU vs Metal) inside a single graph and insert
//! `Transfer` ops between them.  The runtime *crate* doesn't yet ship a
//! "multi-executor coordinator" that drives such a graph end-to-end —
//! that's V2 work.  Today, `LocalTransport` is one transport per
//! executor.
//!
//! What we do instead:
//!
//! 1. Plan with **both** CPU and Metal registered (when Metal is
//!    available).  The planner picks per op based on cost.
//! 2. If every `Compute` op (and every constant) landed on the same
//!    executor, dispatch via that executor's transport.  The whole
//!    graph runs there end-to-end.
//! 3. If the placement is mixed (some Compute on CPU, some on Metal)
//!    we re-plan on a CPU-only runtime and dispatch on CPU.  This
//!    keeps the V1 coordinator simple at the cost of occasionally
//!    forgoing GPU speedup.
//!
//! The image-filter graphs in this crate tend to be a single chain of
//! the same kind of op (a few `Mul`s for sepia, one `Pow` for gamma,
//! etc.), and the planner's cost model splits cleanly by total work —
//! tiny graphs land entirely on CPU, big graphs land entirely on
//! Metal.  So in practice the mixed case is rare.
//!
//! When V2 lands a real multi-executor coordinator, `dispatch_placed`
//! grows a third arm that walks `placed.ops`, routes each Compute to
//! the right transport, and handles `Transfer` itself.

use crate::GpuError;
use compute_ir::{ComputeGraph, PlacedConstant, PlacedOp};
#[cfg(feature = "metal-backend")]
use compute_ir::{ExecutorId, CPU_EXECUTOR};
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport};
use matrix_ir::{Graph, Op, TensorId};
use matrix_runtime::{DefaultPolicy, Profiler, Runtime, SpecCache, SpecRouter};
use std::cell::Cell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
// matrix_profile re-exports SpecialisedKernel via matrix_runtime; we
// take it directly so the auto-installer can pattern-match on its
// `key` field.
use matrix_runtime::SpecialisedKernel;

// ─────────────────────────── Last-executor reporting ───────────────────────────

thread_local! {
    /// The name of the executor that handled the most recent dispatch
    /// on this thread.  Set by [`run_graph_with_constant_inputs`]
    /// before it returns success.  Read by callers (the
    /// `instagram-filters` CLI uses this to print which backend ran).
    ///
    /// Default `None` until the first successful dispatch.  Cleared to
    /// `None` if a dispatch fails partway, so callers don't read stale
    /// values.
    static LAST_EXECUTOR: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The name of the executor that handled the most recent dispatch on
/// this thread.  Returns `"cpu"`, `"metal"`, or `None` if no dispatch
/// has succeeded yet on this thread.
///
/// Useful for CLI demos that want to surface which backend actually
/// ran without changing the public function signatures.
pub fn last_executor() -> Option<&'static str> {
    LAST_EXECUTOR.with(|c| c.get())
}

fn set_last_executor(name: Option<&'static str>) {
    LAST_EXECUTOR.with(|c| c.set(name));
}

// ─────────────────────────── Metal singleton ───────────────────────────
//
// The Metal kernel library takes ~50–100 ms to compile (one MSL source
// → many `MetalComputePipelineState`s).  Doing that on every filter
// invocation would make a "filter a folder of 100 photos" workflow
// pay the cost 100×.  Compile once at first use, then cache.
//
// `OnceLock` is fine here because `LocalTransport` is `Send + Sync`.
// On non-Apple targets the cell holds `None` permanently and we always
// fall through to CPU.

#[cfg(feature = "metal-backend")]
struct MetalBackend {
    /// **MX05 Phase 4.3.**  Direct reference to the metal executor so
    /// the auto-installer ([`try_auto_install_specialised`]) can call
    /// [`MetalExecutor::install_specialised_from_emitted`] when the
    /// `SpecRouter` produces a kernel.  Previously this module only
    /// kept the `LocalTransport`, which hides the executor behind a
    /// boxed `Fn` and offers no direct install API.
    executor: Arc<matrix_metal::MetalExecutor>,
    transport: LocalTransport,
    profile: executor_protocol::BackendProfile,
}

#[cfg(feature = "metal-backend")]
fn metal_backend() -> Option<&'static MetalBackend> {
    static SLOT: OnceLock<Option<MetalBackend>> = OnceLock::new();
    SLOT.get_or_init(|| match matrix_metal::MetalExecutor::new() {
        Ok(exec) => {
            // Pattern mirrors `matrix_metal::local_transport`, but we
            // keep the `Arc<MetalExecutor>` alongside the transport so
            // image-gpu-core can call install methods on it directly.
            let exec = Arc::new(exec);
            let exec2 = exec.clone();
            let transport = LocalTransport::new(move |req| exec2.handle(req));
            Some(MetalBackend {
                executor: exec,
                transport,
                profile: matrix_metal::profile(),
            })
        }
        Err(_) => None,
    })
    .as_ref()
}

// ─────────────────────────── CPU backend singleton (Phase 4.9) ───────────────────────────
//
// Up to Phase 4.8 the CPU dispatch path called `matrix_cpu::local_transport()`
// per invocation, which created a fresh `CpuExecutor` each time.
// That was fine when specialised kernels only ever lived on the
// metal executor, but Phase 4.9 wants the auto-installer to install
// closures on the CPU executor too — and those installs need to
// persist across invocations.  So we promote the CPU executor to
// a process-wide singleton, exactly parallel to `MetalBackend`.

struct CpuBackend {
    /// Direct reference to the CPU executor so the auto-installer
    /// can call `install_specialised(handle, kernel)` on it.
    executor: Arc<matrix_cpu::CpuExecutor>,
    /// The standard wire-format transport used by the dispatcher.
    transport: LocalTransport,
}

// **Per-thread** so concurrent unit tests don't contaminate each
// other's BufferStore.  Within one thread (real workloads, or one
// test's sequence of dispatches) the backend is persistent — so
// specialised kernels installed by the auto-installer survive
// across the 1000+ invocations that real workflows need.  Each
// production dispatch path lives on its own thread (or runs
// serially within image-gpu-core's caller), so per-thread
// persistence matches the actual sharing model.
thread_local! {
    static CPU_BACKEND: CpuBackend = {
        let exec = Arc::new(matrix_cpu::CpuExecutor::new());
        let exec2 = exec.clone();
        let transport = LocalTransport::new(move |req| exec2.handle(req));
        CpuBackend { executor: exec, transport }
    };
}

/// Run a closure with access to the thread-local `CpuBackend`.
/// Used by every caller that needs to interact with the CPU
/// executor — the dispatcher, the auto-installer, and the public
/// `specialised_install_count`.
fn with_cpu_backend<R>(f: impl FnOnce(&CpuBackend) -> R) -> R {
    CPU_BACKEND.with(f)
}

// ─────────────────────────── MX05 specialisation singletons ───────────────────────────
//
// MX05 Phase 1 / 2a / 3 V1 / V2 / V3 shipped the Profiler /
// SpecialisationPolicy / SpecCache / SpecRouter machinery in
// matrix-runtime (now matrix-profile).  Phase 3 V4 wired them up
// here so every `run_graph_with_constant_inputs` call records an
// invocation observation and asks the router whether to specialise
// each op.  Phase 4 minimum-viable installed `matrix_cpu::CpuSpecialiser`
// in matrix-cpu (the first real `Specialiser` impl).  This file
// (Phase 4 wiring) replaces the `NoopSpecialiser` here with
// `matrix_cpu::specialiser()` so the cache visibly fills under the
// instagram-filters demo once enough invocations accumulate.
//
// Both singletons are lazy via `OnceLock`.  Constructing the SpecRouter
// allocates a small SpecCache plus the policy + specialiser trait
// objects; cheap, but doing it once instead of per-call keeps the
// hot path tight.

fn profiler() -> &'static Profiler {
    static SLOT: OnceLock<Profiler> = OnceLock::new();
    SLOT.get_or_init(Profiler::new)
}

fn spec_router() -> &'static SpecRouter {
    static SLOT: OnceLock<SpecRouter> = OnceLock::new();
    SLOT.get_or_init(|| {
        // Phase 4.2: drive_specialisation now samples tensor bytes
        // from `placed.constants[*]` so `DefaultPolicy`'s constant-
        // input check has real observations to act on.  The threshold
        // returns to spec MX05's default (1000 invocations).
        SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::default_capacity(),
            matrix_cpu::specialiser(),
        )
    })
}

/// Snapshot the profile observation accumulated by this process so
/// far.  Phase 3 V4's CLI demos and tests use this to confirm the
/// specialisation pipeline is live (invocation counts climb across
/// repeat calls; cache stays empty under `NoopSpecialiser`).
///
/// Returns the same data shape as
/// [`matrix_runtime::Profiler::observations`] for callers that want
/// to inspect specific ops.
pub fn profiler_observations() -> Vec<matrix_runtime::ProfileObservation> {
    profiler().observations()
}

/// How many specialised kernels the process-wide cache currently
/// holds.  Phase 4 wired `matrix_cpu::CpuSpecialiser` in front of
/// this cache and Phase 4.2 (this module) added per-tensor sampling
/// so `DefaultPolicy` fires properly.  After a graph crosses 1000
/// invocations and at least one of its op-input constants meets the
/// 95%-stability bar, this number rises by one cache entry per
/// distinct `SpecKey`.
pub fn spec_cache_len() -> usize {
    spec_router().cache_len()
}

/// **MX05 Phase 4.3.**  Number of specialised kernels that have been
/// auto-installed onto a backing executor via the runtime auto-installer.
///
/// Distinct from [`spec_cache_len`] — the cache tracks emitted handles,
/// while this counter tracks how many of those handles have actually
/// been turned into compiled pipelines and registered with an executor.
/// The two converge under normal operation but diverge briefly while
/// installs are in flight or when an install fails (e.g. compilation
/// rejects the emitted MSL).
///
/// **MX05 Phase 4.9.**  Now counts installs on **both** the CPU and
/// metal executors.  Before Phase 4.9 the CPU auto-install path
/// didn't exist (matrix-cpu's `CpuSpecialiser` emitted opaque handles
/// without closure sources), so this counter was metal-only and
/// returned 0 on non-Apple builds.  The new
/// `matrix_cpu::build_specialised_kernel` closes that gap, so the
/// counter now reflects the **total** kernels registered across
/// every backing executor in this process.
pub fn specialised_install_count() -> usize {
    let cpu_count = with_cpu_backend(|b| b.executor.specialised_count());
    #[cfg(feature = "metal-backend")]
    let metal_count = match metal_backend() {
        Some(b) => b.executor.specialised_count(),
        None => 0,
    };
    #[cfg(not(feature = "metal-backend"))]
    let metal_count = 0usize;
    cpu_count + metal_count
}

/// **MX05 Phase 4.4.**  Number of dispatches the runtime has routed
/// through `ExecutorRequest::DispatchSpecialised` so far — i.e. how
/// often the dispatch path actually invoked an installed specialised
/// kernel rather than the generic op-by-op pipeline.
///
/// Process-wide counter, monotonic, never resets.  Distinct from:
/// - [`spec_cache_len`] — kernels emitted by the specialiser.
/// - [`specialised_install_count`] — kernels compiled and registered
///   with an executor.
/// - This counter — kernels actually *invoked* at dispatch time.
///
/// All three should track together under steady-state load: every
/// installed kernel sees at least one dispatch eventually.  The
/// invariant `dispatch ≥ install ≥ cache` holds in V0.8.0 (each step
/// gates the next).  Always `0` on non-Apple builds — see the cfg
/// equivalent for `specialised_install_count`.
pub fn specialised_dispatch_count() -> usize {
    SPECIALISED_DISPATCH_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Backing storage for [`specialised_dispatch_count`].  Atomic so the
/// hot path doesn't need a mutex; relaxed ordering is sufficient
/// because the counter is observational, not a synchronisation
/// primitive.
static SPECIALISED_DISPATCH_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

// ─────────────────────── MX05 Phase 4.3 — auto-installer ───────────────────────
//
// When `SpecRouter::route` returns `Some(SpecialisedKernel)`, this
// module attempts to *install* the kernel onto the backing executor
// so that future invocations can dispatch through the protocol-level
// `DispatchSpecialised` path.
//
// V0.3.0 ships the **install side only**:
//
//   * Auto-install on metal: cache hit → `msl_emitter::emit_specialised_kernel`
//     → `MetalExecutor::install_specialised_from_emitted`.  Backed by an
//     `INSTALLED_HANDLES: HashSet<u64>` so re-installs are skipped.
//
//   * matrix-cpu auto-install: deferred to a later phase.  `CpuSpecialiser`
//     only emits opaque handles today — it doesn't carry a closure
//     source, so there's no auto-install translation available yet.
//     The matrix-cpu Phase 4.1 install API works (and tests cover it
//     in matrix-cpu's integration suite), it just isn't invoked
//     automatically from here yet.
//
//   * **Dispatch-routing side:** the next dispatch still goes through
//     generic `Dispatch { graph }`.  Phase 4.4 will land
//     `dispatch_specialised_via` that walks the placed graph,
//     fires per-op `DispatchSpecialised` requests, and proves the
//     end-to-end DispatchDone → speedup loop.  Splitting that work
//     into its own PR keeps Phase 4.3 reviewable.

/// Process-wide set of handles already installed on **any** backend
/// (CPU or metal).  Phase 4.9: shared between the two backends so
/// the auto-installer can short-circuit on already-installed handles
/// regardless of which executor they ended up on.
fn installed_handles() -> &'static Mutex<HashSet<u64>> {
    static SLOT: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(HashSet::new()))
}

/// **MX05 Phase 4.4 / extended in 4.6 / 4.9.**  Per-handle metadata
/// recorded at install time so the dispatch-routing path can
/// validate buffer counts and pick the correct unfolded slot without
/// re-emitting the kernel.  Shared between CPU and metal install
/// paths (Phase 4.9): both backends record the same
/// `(n_in, n_out, folded_slot)` triple, keyed by the SpecKey's
/// 64-bit handle.
///
/// `folded_slot` is `Some(s)` for binary ops where the policy
/// observed slot `s` as a constant, and `None` otherwise (e.g.
/// commutative ops where slot doesn't matter, or future
/// `RangeClass::FloatBits`/`Integer` shapes).
#[derive(Copy, Clone, Debug)]
pub(crate) struct KernelMetadata {
    pub n_in: usize,
    pub n_out: usize,
    /// Which IR input slot was folded.  Dispatcher uses this to
    /// pick which `ir_op.inputs()` entries to actually pass to the
    /// specialised kernel: it passes the **unfolded** slots.
    /// `None` for commutative ops where the kernel accepts whichever
    /// slot the dispatcher happens to pick first.
    pub folded_slot: Option<u8>,
}

fn installed_kernel_metadata() -> &'static Mutex<HashMap<u64, KernelMetadata>> {
    static SLOT: OnceLock<Mutex<HashMap<u64, KernelMetadata>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(HashMap::new()))
}

/// **Test-only**.  Manually record kernel metadata for a handle.
/// Used by integration tests that install a kernel directly via
/// `MetalExecutor::install_specialised_from_emitted` rather than
/// going through the auto-installer.  Available on all platforms
/// since Phase 4.9 because the CPU install path also exercises
/// this metadata table.
#[cfg(test)]
pub(crate) fn record_test_kernel_metadata(
    handle: u64,
    n_in: usize,
    n_out: usize,
    folded_slot: Option<u8>,
) {
    let mut md = match installed_kernel_metadata().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    md.insert(
        handle,
        KernelMetadata {
            n_in,
            n_out,
            folded_slot,
        },
    );
}

/// **MX05 Phase 4.3.**  Try to auto-install a `SpecialisedKernel` onto
/// the metal executor, returning `true` if a fresh install happened.
///
/// Idempotent across calls with the same handle — `INSTALLED_HANDLES`
/// tracks which handles we've already installed so repeat hits in the
/// `SpecRouter` cache don't pay MSL compilation cost more than once.
///
/// Returns `false` when:
/// - metal isn't available (non-Apple build, or `MetalExecutor::new`
///   failed at startup);
/// - `msl_emitter::emit_specialised_kernel` returns `None`
///   (Phase 4.2's emitter only supports a small set of SpecKey shapes);
/// - the handle was already installed in a prior call;
/// - compilation failed (rare — MSL emitter is bug-free for the
///   shapes it claims to support, but the install API returns Err
///   here just in case).  In the compile-fail case we **don't** mark
///   the handle as installed so a future emitter fix can retry.
///
/// Hot-path cost: one mutex acquire on `INSTALLED_HANDLES`, plus
/// (on miss) one MSL compile (~few ms on Apple Silicon) and one
/// mutex acquire inside `MetalExecutor::install_specialised_from_emitted`.
/// Subsequent calls with the same handle are a single mutex acquire
/// and a `HashSet::contains` — sub-microsecond.
fn try_auto_install_specialised(specialised: &SpecialisedKernel) -> bool {
    // Fast path: skip if already installed (on any backend).  The
    // handle hash includes `backend_id`, so CPU and metal versions
    // of the same SpecKey get distinct handles — no risk of
    // confusing one for the other.
    {
        let installed = match installed_handles().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if installed.contains(&specialised.handle) {
            return false;
        }
    }

    // Dispatch on `backend_id`:
    //   0 → CPU (matrix_cpu::build_specialised_kernel + CpuExecutor::install_specialised)
    //   1 → metal (matrix_metal::emit_specialised_kernel + install_specialised_from_emitted)
    // Other values → ignore (no backend registered).
    match specialised.key.backend_id {
        0 => try_install_cpu(specialised),
        #[cfg(feature = "metal-backend")]
        1 => try_install_metal(specialised),
        _ => false,
    }
}

/// **MX05 Phase 4.9.**  Install a specialised kernel on the CPU
/// executor singleton.  Returns `true` on a fresh install.
fn try_install_cpu(specialised: &SpecialisedKernel) -> bool {
    let Some(kernel) =
        matrix_cpu::build_specialised_kernel(&specialised.key, specialised.handle)
    else {
        return false;
    };
    // matrix_cpu::CpuExecutor::install_specialised is infallible
    // (no compile step on CPU).  No error path here — record and
    // mark as installed.
    with_cpu_backend(|b| {
        b.executor.install_specialised(specialised.handle, kernel);
    });
    // Record metadata.  CPU kernels' (n_in, n_out) is determined by
    // the SpecKey shape: unary memset → (0, 1); binary-with-folded
    // → (1, 1).
    let (n_in, n_out) = derive_buffer_counts(&specialised.key);
    {
        let mut installed = match installed_handles().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        installed.insert(specialised.handle);
    }
    {
        let mut md = match installed_kernel_metadata().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        md.insert(
            specialised.handle,
            KernelMetadata {
                n_in,
                n_out,
                folded_slot: specialised.key.folded_slot,
            },
        );
    }
    true
}

/// **MX05 Phase 4.9 helper.**  Reverse-engineer `(n_in, n_out)`
/// from a SpecKey, mirroring the logic in
/// `matrix_cpu::build_specialised_kernel`.  Used to populate
/// `KernelMetadata` for CPU installs, where the kernel closure
/// doesn't carry a separate descriptor.
///
/// Returns `(0, 1)` for unary-folded-input kernels (memset shape)
/// and `(1, 1)` for binary-with-folded-constant kernels.
fn derive_buffer_counts(key: &matrix_runtime::SpecKey) -> (usize, usize) {
    // Unary ops with folded input → 0 inputs.
    let is_unary_memset = matches!(key.op_kind, 0x00..=0x06)
        && matches!(key.folded_slot, Some(0));
    if is_unary_memset {
        (0, 1)
    } else {
        // Binary-with-folded-constant → 1 input + 1 output.
        (1, 1)
    }
}

#[cfg(feature = "metal-backend")]
fn try_install_metal(specialised: &SpecialisedKernel) -> bool {
    let Some(backend) = metal_backend() else {
        return false;
    };
    // Slow path: emit MSL, compile, install.
    let Some(emitted) = matrix_metal::emit_specialised_kernel(&specialised.key, specialised.handle)
    else {
        return false;
    };
    let n_in = emitted.input_buffer_count;
    let n_out = emitted.output_buffer_count;
    let folded_slot = specialised.key.folded_slot;
    match backend
        .executor
        .install_specialised_from_emitted(specialised.handle, emitted)
    {
        Ok(()) => {
            {
                let mut installed = match installed_handles().lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                installed.insert(specialised.handle);
            }
            let mut md = match installed_kernel_metadata().lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            md.insert(
                specialised.handle,
                KernelMetadata {
                    n_in,
                    n_out,
                    folded_slot,
                },
            );
            true
        }
        Err(_) => {
            // Don't mark as installed — a future emitter/compiler fix
            // may want to retry this handle.
            false
        }
    }
}

/// Drive the MX05 specialisation pipeline for one placed graph:
///
/// 1. Bump per-op invocation counters via `Profiler::record_dispatch`.
/// 2. Sample bytes from every constant input the graph has — this
///    populates `ProfileObservation::tensor_observations` so the
///    `DefaultPolicy`'s constant-input / narrowing checks have data
///    to act on.  Without this step, V1 had to fall back to a
///    `HotPolicy` that fired on raw invocation count alone.
/// 3. Build a `ProfileObservation` per Compute op and pass it to
///    `SpecRouter::route` along with op metadata.  Discard the
///    return — V1 still doesn't dispatch via specialised kernels
///    (that's Phase 4.1 + an executor-protocol extension); the
///    return only populates the cache.
///
/// Pure observation work; no behavioural change to the dispatch.
/// Cost: O(constants × bytes-per-constant) on each call, capped by
/// the 16 MiB per-tensor limit (so ≤ a few MB scanned per dispatch).
///
/// **Returns** a map `op_index → installed_handle` covering every
/// Compute op whose `SpecKey` produced a kernel and where the kernel
/// is currently installed on the metal executor.  An op's index is
/// **absent** from the map when (a) the policy didn't fire, or
/// (b) the policy fired but the emitter doesn't support the SpecKey,
/// or (c) installation failed.  Callers use this map to decide
/// whether to route through [`dispatch_specialised_via`] (when all
/// non-Const Compute ops have entries) or fall back to the generic
/// `Dispatch` path.
fn drive_specialisation(placed: &ComputeGraph) -> HashMap<u32, u64> {
    let p = profiler();
    p.record_dispatch(placed);

    // **MX05 Phase 4.4.**  Per-op installed-handle map populated
    // during the route-and-install loop below.
    let mut installed_per_op: HashMap<u32, u64> = HashMap::new();

    let subhash = Profiler::subhash(placed);

    // ── Tensor-byte sampling ──
    //
    // Build a TensorId → &PlacedConstant map for tensors that are
    // Const ops' outputs.  Any subsequent Compute op reading one of
    // those tensors as an input is reading a constant value; we
    // sample the bytes and attribute the observation to the
    // consuming op's input slot.
    let mut const_outputs: HashMap<TensorId, &PlacedConstant> = HashMap::new();
    for op in placed.ops.iter() {
        if let PlacedOp::Compute {
            op: Op::Const { constant, output },
            ..
        } = op
        {
            if let Some(c) = placed.constants.get(*constant as usize) {
                const_outputs.insert(*output, c);
            }
        }
    }
    for (op_idx, pop) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute { op: ir_op, .. } = pop {
            // Skip Const ops themselves; they're the *source* of the
            // observations, not consumers.  Sampling their inputs is
            // a no-op (Const has no inputs).
            if matches!(ir_op, Op::Const { .. }) {
                continue;
            }
            for (slot, in_id) in ir_op.inputs().iter().enumerate() {
                if let Some(c) = const_outputs.get(in_id) {
                    if let Some(t) = placed.tensor(*in_id) {
                        p.sample_tensor(
                            subhash,
                            op_idx as u32,
                            slot as u32,
                            true, // is_input
                            t.dtype,
                            &c.bytes,
                        );
                    }
                }
            }
        }
    }

    // ── Route per Compute op ──
    let r = spec_router();
    // Observations() returns every cached observation; index by
    // op_index so the per-op router calls below are O(n) total
    // rather than O(n²).
    let obs = p.observations();
    let mut by_op: HashMap<u32, &matrix_runtime::ProfileObservation> = HashMap::new();
    for o in &obs {
        if o.graph_subhash == subhash {
            by_op.insert(o.op_index, o);
        }
    }
    for (op_idx, pop) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute {
            op: ir_op,
            executor,
            ..
        } = pop
        {
            let key = op_idx as u32;
            let observation = match by_op.get(&key) {
                Some(o) => *o,
                None => continue,
            };
            let out_id = ir_op.output();
            let out_dtype = match placed.tensor(out_id) {
                Some(t) => t.dtype,
                None => continue,
            };
            // **MX05 Phase 4.3.**  When the router returns a fresh
            // (or cached) specialised kernel, ask the auto-installer
            // to register it with the metal executor.  Idempotent;
            // see `try_auto_install_specialised` for the install
            // semantics.  Returning `None` from `route` (the policy
            // declined, or no specialiser matched) is the common
            // fast path and incurs zero cost here.
            //
            // **MX05 Phase 4.4.**  Whether install was a fresh hit
            // or a no-op, if the handle is *currently installed*
            // we record `(op_index, handle)` so the dispatcher can
            // route this op through `DispatchSpecialised`.  See
            // `handle_is_installed` for what "installed" means.
            if let Some(spec) = r.route(observation, ir_op.wire_tag(), out_dtype, executor.0) {
                let _ = try_auto_install_specialised(&spec);
                if handle_is_installed(spec.handle) {
                    installed_per_op.insert(op_idx as u32, spec.handle);
                }
            }
        }
    }

    installed_per_op
}

/// **MX05 Phase 4.4.**  True iff the given handle is recorded in
/// `INSTALLED_HANDLES`.  Used by [`drive_specialisation`] to populate
/// the per-op installed-handle map even on cache hits where
/// `try_auto_install_specialised` would have short-circuited and
/// returned `false`.
fn handle_is_installed(handle: u64) -> bool {
    let installed = match installed_handles().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    installed.contains(&handle)
}

// ─────────────────────────── Public entry point ───────────────────────────

/// Plan and run a graph that has all its inputs embedded as constants
/// (no runtime [`matrix_ir::Graph::inputs`]) and one declared output.
/// Returns the output's bytes, downloaded from whichever executor ran.
pub fn run_graph_with_constant_inputs(
    graph: &Graph,
    output_id: TensorId,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    set_last_executor(None);

    // ── Step 1: try the dual-backend path (CPU + Metal). ──
    #[cfg(feature = "metal-backend")]
    if let Some(metal) = metal_backend() {
        let mut runtime = Runtime::new(matrix_cpu::profile());
        // Order matters: CPU is registered by `Runtime::new` as
        // executor 0, so Metal becomes executor 1.  The planner uses
        // the `BackendProfile` cost numbers — not the executor id — to
        // decide placement, so this ordering is purely informational.
        let metal_id = runtime.register("metal", metal.profile.clone());

        let placed: ComputeGraph = runtime
            .plan(graph)
            .map_err(|e| GpuError::Other(format!("plan: {:?}", e)))?;

        if let Some(only) = single_executor(&placed) {
            // The whole graph routes to one executor — we can use a
            // single transport.  Pick the right one.
            return if only == metal_id {
                dispatch_via(&metal.transport, placed, output_id, output_byte_count, "metal")
            } else if only == CPU_EXECUTOR {
                // **MX05 Phase 4.9.**  Run inside the thread-local
                // CPU backend so the installed specialised kernels
                // (registered in this same thread by earlier
                // dispatches) are visible.  Per-thread isolation
                // means parallel unit tests don't contaminate each
                // other's `BufferStore`; persistence within a thread
                // means real workflows with thousands of repeat
                // dispatches see the auto-installer's installs.
                with_cpu_backend(|b| {
                    dispatch_via(&b.transport, placed, output_id, output_byte_count, "cpu")
                })
            } else {
                // The planner chose an executor we didn't register.
                // Shouldn't happen with the registry above, but be
                // defensive: fall through to CPU re-plan.
                dispatch_cpu_only(graph, output_id, output_byte_count)
            };
        }
        // Mixed placement.  V1 falls back to CPU-only.
    }

    // ── Step 2: CPU-only fallback. ──
    dispatch_cpu_only(graph, output_id, output_byte_count)
}

// ─────────────────────────── Dispatch helpers ───────────────────────────

/// Dispatch a placed graph through a specific transport, then download
/// the output and record the executor name as last-used.
///
/// Before forwarding to the transport, this function drives the MX05
/// specialisation pipeline ([`drive_specialisation`]): per-op
/// invocation counters climb, and the [`SpecRouter`] is asked
/// whether each Compute op should specialise.  In V1 the answer is
/// always None (the noop specialiser is installed) — Phase 4 will
/// install a real specialiser and the same call site will start
/// emitting kernels.
fn dispatch_via(
    transport: &LocalTransport,
    placed: ComputeGraph,
    output_id: TensorId,
    output_byte_count: usize,
    executor_name: &'static str,
) -> Result<Vec<u8>, GpuError> {
    let installed_per_op = drive_specialisation(&placed);

    let output_residency = placed
        .outputs
        .iter()
        .find(|t| t.id == output_id)
        .map(|t| t.residency)
        .or_else(|| placed.tensors.get(output_id.0 as usize).map(|t| t.residency))
        .ok_or_else(|| {
            GpuError::Other(format!("output tensor {} not in placed graph", output_id.0))
        })?;

    // **MX05 Phase 4.4.**  If this graph has exactly one non-Const
    // Compute op and that op's specialised kernel is installed,
    // route through `dispatch_specialised_via` — a setup `Dispatch`
    // for the prep ops followed by a `DispatchSpecialised` for the
    // single non-Const Compute op.  Otherwise use the existing
    // generic `Dispatch { graph }` path.
    //
    // We restrict to "exactly one non-Const Compute op" for V0.8.0
    // because multi-op routing requires interleaving setup +
    // specialised dispatches, which is more buffer-management code
    // than fits cleanly here.  Phase 4.5 (or later) will extend the
    // routing to multi-op graphs.
    // **MX05 Phase 4.8.**  Try the multi-op specialised route
    // first.  When **every** non-Const Compute op has an installed
    // handle we fire one prep Dispatch + N DispatchSpecialised
    // requests + one DownloadBuffer.  When the multi-op coverage
    // is incomplete (or the single-op gate fires), we still try
    // the Phase 4.4 single-op fast path before falling back to
    // generic Dispatch.
    //
    // Specialised routing remains an *optimisation*, not a
    // correctness guarantee — any Err from the specialised path
    // silently falls through to the generic Dispatch below so the
    // graph still produces correct output.
    if executor_name == "metal" {
        if let Some(computes) =
            all_non_const_computes_with_handles(&placed, &installed_per_op)
        {
            if let Ok(data) = dispatch_specialised_via_multi(
                transport,
                &placed,
                &computes,
                output_residency,
                output_byte_count,
            ) {
                set_last_executor(Some(executor_name));
                return Ok(data);
            }
        }
        // Phase 4.4/4.6/4.7 single-op fast path still works for
        // graphs where only one Compute op is specialised but the
        // multi-op gate didn't fire (e.g. mixed-coverage graphs).
        // Kept for back-compat; the multi-op path covers everything
        // single-op did when N == 1.
        if let Some((compute_idx, handle)) =
            single_non_const_compute_with_handle(&placed, &installed_per_op)
        {
            if let Ok(data) = dispatch_specialised_via(
                transport,
                &placed,
                compute_idx,
                handle,
                output_residency,
                output_byte_count,
            ) {
                set_last_executor(Some(executor_name));
                return Ok(data);
            }
        }
    }
    // Suppress unused-variable warning when not on metal-backend.
    let _ = installed_per_op;

    let resp = block_on(transport.request(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: placed,
    }))
    .map_err(|e| GpuError::Other(format!("dispatch: {:?}", e)))?;

    match resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to Dispatch: {:?}",
                other
            )));
        }
    }

    let download = block_on(transport.request(ExecutorRequest::DownloadBuffer {
        buffer: output_residency.buffer,
        offset: 0,
        len: output_byte_count as u64,
    }))
    .map_err(|e| GpuError::Other(format!("download: {:?}", e)))?;

    let data = match download {
        ExecutorResponse::BufferData { data, .. } => data,
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "download error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to DownloadBuffer: {:?}",
                other
            )));
        }
    };

    // Only record the executor name on full success — failures leave
    // `last_executor()` at whatever it was before, so callers don't
    // see a stale "we ran on metal" message after a mid-dispatch error.
    set_last_executor(Some(executor_name));
    Ok(data)
}

/// **MX05 Phase 4.4.**  Inspect `placed.ops` and return
/// `Some((compute_op_index, installed_handle))` iff the graph has
/// exactly one non-Const Compute op and that op's specialised kernel
/// is installed.  Otherwise return `None` to fall back to the generic
/// dispatch path.
///
/// Restricted to single-op graphs in V0.8.0 — multi-op routing is
/// the next phase's scope.  See `dispatch_specialised_via` for what
/// "single-op routing" actually does at the protocol level.
fn single_non_const_compute_with_handle(
    placed: &ComputeGraph,
    installed_per_op: &HashMap<u32, u64>,
) -> Option<(usize, u64)> {
    let mut found: Option<(usize, u64)> = None;
    for (idx, op) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute { op: ir_op, .. } = op {
            if matches!(ir_op, Op::Const { .. }) {
                continue; // Const ops are setup, not compute proper.
            }
            // Found a non-Const Compute op.  If we'd already found
            // one, this is a multi-op graph — abort.
            if found.is_some() {
                return None;
            }
            let handle = installed_per_op.get(&(idx as u32)).copied()?;
            found = Some((idx, handle));
        }
    }
    found
}

/// **MX05 Phase 4.8.**  Multi-op version of
/// [`single_non_const_compute_with_handle`].  Returns
/// `Some(Vec<(op_index, handle)>)` iff **every** non-Const Compute
/// op in the placed graph has an installed specialised kernel for
/// the metal executor.  Returns `None` if even one Compute op is
/// missing a handle — in that case the dispatcher falls back to the
/// generic `Dispatch { graph }` path, since interleaving generic
/// and specialised dispatches across the same buffer-id space
/// requires more bookkeeping than the prep-then-specialise pattern
/// supports today.
///
/// The returned vector is ordered by `op_index` so the dispatcher
/// can fire `DispatchSpecialised` requests in placement order,
/// honouring the data-dependency chain encoded by the planner.
fn all_non_const_computes_with_handles(
    placed: &ComputeGraph,
    installed_per_op: &HashMap<u32, u64>,
) -> Option<Vec<(usize, u64)>> {
    let mut out: Vec<(usize, u64)> = Vec::new();
    for (idx, op) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute { op: ir_op, .. } = op {
            if matches!(ir_op, Op::Const { .. }) {
                continue;
            }
            let handle = installed_per_op.get(&(idx as u32)).copied()?;
            out.push((idx, handle));
        }
    }
    if out.is_empty() {
        // No non-Const Compute ops at all — nothing to specialise.
        // Returning None here lets the caller fall back to generic
        // dispatch (which will execute the Const + Alloc + Free
        // bookkeeping); attempting a "specialised" dispatch of
        // zero kernels would be wasted protocol round-trips.
        return None;
    }
    Some(out)
}

/// **MX05 Phase 4.8.**  Run a multi-op graph through the
/// `DispatchSpecialised` path, firing one specialised dispatch per
/// non-Const Compute op in placement order.
///
/// Strategy:
///   1. Build a **prep graph** containing every `PlacedOp` *except*
///      the non-Const Compute ops listed in `computes`.  This keeps
///      every `Op::Const`, `Alloc`, `Free`, and `Transfer` op — so
///      matrix-metal's existing dispatch handler allocates the
///      planner-assigned BufferIds, uploads all constants, and
///      tears down any temporary buffers, exactly as it would for
///      generic dispatch.
///   2. Fire one `Dispatch { prep_graph }` request.  After it
///      returns, the executor's BufferStore holds every buffer the
///      pending Compute ops will read or write.
///   3. For each `(op_idx, handle)` in `computes` (in placement
///      order — `Vec` is already sorted by `op_idx`), fire one
///      `DispatchSpecialised { handle, inputs, outputs }`.  The
///      inputs/outputs trimming and folded-slot logic is identical
///      to the single-op path in [`dispatch_specialised_via`], so
///      we factor that into [`build_specialised_inputs_outputs`].
///   4. Fire one `DownloadBuffer` for the final output.
///   5. Increment `SPECIALISED_DISPATCH_COUNT` by `computes.len()`.
///
/// Why not interleave generic and specialised dispatches per op
/// (i.e. fall back per-op to generic Dispatch when a kernel isn't
/// installed): the prep dispatch already runs the Const + Alloc
/// for buffers the unspecialised generic op would need, but the
/// generic dispatcher walks `placed.ops` from the top, expecting
/// to encounter all those Const/Alloc ops itself.  Splitting the
/// graph between two dispatchers would either duplicate
/// allocation (with planner-id collisions in the buffer store)
/// or require a "skip ops" hint we'd have to add to the protocol.
/// Both are out-of-scope for Phase 4.8; the all-or-nothing gate
/// keeps the protocol surface stable.
///
/// Returns `Err` if any protocol step fails, so the caller can
/// fall through to generic dispatch.
fn dispatch_specialised_via_multi(
    transport: &LocalTransport,
    placed: &ComputeGraph,
    computes: &[(usize, u64)],
    output_residency: compute_ir::Residency,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    // Step 1: build the prep graph — original ops minus the
    // non-Const Compute ops we'll dispatch as specialised.
    let specialised_indices: std::collections::HashSet<usize> =
        computes.iter().map(|(idx, _)| *idx).collect();
    let prep_graph = ComputeGraph {
        format_version: placed.format_version,
        tensors: placed.tensors.clone(),
        inputs: placed.inputs.clone(),
        outputs: placed.outputs.clone(),
        constants: placed.constants.clone(),
        ops: placed
            .ops
            .iter()
            .enumerate()
            .filter(|(i, _)| !specialised_indices.contains(i))
            .map(|(_, op)| op.clone())
            .collect(),
    };

    // Step 2: prep Dispatch — allocates buffers, uploads constants.
    let prep_resp = block_on(transport.request(ExecutorRequest::Dispatch {
        job_id: 200,
        graph: prep_graph,
    }))
    .map_err(|e| GpuError::Other(format!("prep dispatch: {:?}", e)))?;
    match prep_resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "prep dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to prep Dispatch: {:?}",
                other
            )));
        }
    }

    // Step 3: fire DispatchSpecialised per Compute op, in placement
    // order.  Each request reuses the same planner-assigned buffer
    // ids — the executor's BufferStore is unchanged across calls
    // because we hold no Free ops between them.  Job ids are
    // sequential for observability in any timing/profiling capture.
    let mut next_job_id: u64 = 201;
    for &(compute_op_idx, handle) in computes {
        let (input_bufs, output_buf) =
            build_specialised_inputs_outputs(placed, compute_op_idx, handle)?;
        let spec_resp = block_on(transport.request(ExecutorRequest::DispatchSpecialised {
            job_id: next_job_id,
            handle,
            inputs: input_bufs,
            outputs: vec![output_buf],
        }))
        .map_err(|e| {
            GpuError::Other(format!(
                "specialised dispatch (op {}): {:?}",
                compute_op_idx, e
            ))
        })?;
        match spec_resp {
            ExecutorResponse::DispatchDone { .. } => {}
            ExecutorResponse::Error { code, message, .. } => {
                return Err(GpuError::Other(format!(
                    "specialised dispatch error 0x{:04X} (op {}): {}",
                    code.0, compute_op_idx, message
                )));
            }
            other => {
                return Err(GpuError::Other(format!(
                    "unexpected response to DispatchSpecialised (op {}): {:?}",
                    compute_op_idx, other
                )));
            }
        }
        next_job_id += 1;
    }

    // Step 4: download the final graph output.
    let download = block_on(transport.request(ExecutorRequest::DownloadBuffer {
        buffer: output_residency.buffer,
        offset: 0,
        len: output_byte_count as u64,
    }))
    .map_err(|e| GpuError::Other(format!("download: {:?}", e)))?;
    let data = match download {
        ExecutorResponse::BufferData { data, .. } => data,
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "download error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to DownloadBuffer: {:?}",
                other
            )));
        }
    };

    // Step 5: bump the counter by the number of specialised
    // dispatches we actually fired.  Atomic increment is `Relaxed`
    // because the counter is observational.
    SPECIALISED_DISPATCH_COUNT
        .fetch_add(computes.len(), std::sync::atomic::Ordering::Relaxed);

    Ok(data)
}

/// **MX05 Phase 4.8.**  Shared input/output buffer-id extraction
/// used by both the single-op ([`dispatch_specialised_via`]) and
/// the multi-op ([`dispatch_specialised_via_multi`]) paths.
///
/// Reads the kernel's `KernelMetadata` (n_in, n_out, folded_slot)
/// from `INSTALLED_KERNEL_METADATA`, then resolves which IR input
/// slots to pass to the kernel:
///   - `folded_slot = Some(s)` on a 2-input op with `n_in == 1` →
///     pass `ir_inputs[1 - s]` (the unfolded slot).
///   - Otherwise → pass the first `n_in` ir_inputs in declared order.
///
/// Returns `(input_buffers, output_buffer)` ready for a
/// `DispatchSpecialised` request.
fn build_specialised_inputs_outputs(
    placed: &ComputeGraph,
    compute_op_idx: usize,
    handle: u64,
) -> Result<(Vec<compute_ir::BufferId>, compute_ir::BufferId), GpuError> {
    let md = {
        let g = match installed_kernel_metadata().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        g.get(&handle).copied().ok_or_else(|| {
            GpuError::Other(format!(
                "no kernel metadata for handle 0x{:016X} — did the install side run?",
                handle
            ))
        })?
    };
    let n_in = md.n_in;
    let pop = &placed.ops[compute_op_idx];
    let PlacedOp::Compute { op: ir_op, .. } = pop else {
        return Err(GpuError::Other("not a Compute op".to_string()));
    };
    let ir_inputs = ir_op.inputs();
    if n_in > ir_inputs.len() {
        return Err(GpuError::Other(format!(
            "specialised kernel expects {} inputs but op has only {}",
            n_in,
            ir_inputs.len()
        )));
    }
    let selected_input_ids: Vec<matrix_ir::TensorId> = match md.folded_slot {
        Some(s) if ir_inputs.len() == 2 && n_in == 1 => {
            let unfolded = if s == 0 { 1usize } else { 0usize };
            vec![ir_inputs[unfolded]]
        }
        _ => ir_inputs.iter().take(n_in).copied().collect(),
    };
    let input_bufs: Vec<compute_ir::BufferId> = selected_input_ids
        .iter()
        .map(|in_id| {
            placed
                .tensor(*in_id)
                .map(|t| t.residency.buffer)
                .ok_or_else(|| {
                    GpuError::Other(format!("tensor {} not in placed graph", in_id.0))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let out_id = ir_op.output();
    let out_buf = placed
        .tensor(out_id)
        .map(|t| t.residency.buffer)
        .ok_or_else(|| {
            GpuError::Other(format!("output tensor {} not in placed graph", out_id.0))
        })?;
    Ok((input_bufs, out_buf))
}

/// **MX05 Phase 4.4.**  Run a graph that has exactly one non-Const
/// Compute op through the `DispatchSpecialised` path.
///
/// Strategy: send a "prep dispatch" with the *original* placed graph
/// minus the single non-Const Compute op — this lets matrix-metal's
/// existing dispatch handler do all the buffer management
/// (allocate, upload constants, free) under the planner-assigned
/// BufferIds.  Then fire one `DispatchSpecialised` for the Compute
/// op using those same planner-assigned BufferIds in `inputs` /
/// `outputs`.  Finally download the output.
///
/// Why not "manage buffers in image-gpu-core ourselves": the
/// `AllocBuffer` protocol message returns *server-assigned*
/// BufferIds that don't match the planner-assigned IDs the placed
/// graph references.  Going through `Dispatch { prep_graph }` lets
/// the executor's internal allocator use planner IDs (which is its
/// existing graph-walking pattern), keeping the BufferId space
/// consistent across the prep + specialised + download dance.
///
/// The output buffer is in the placed graph's outputs at
/// `output_residency.buffer` and survives between dispatches because
/// the prep graph doesn't issue `Free` for it (Free ops in the
/// placed graph all run during prep; the Compute op's output is
/// allocated by its `Alloc` op which *is* in the prep graph).
///
/// Returns Err if any protocol step fails so the caller can fall
/// back to generic dispatch.
fn dispatch_specialised_via(
    transport: &LocalTransport,
    placed: &ComputeGraph,
    compute_op_idx: usize,
    handle: u64,
    output_residency: compute_ir::Residency,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    // Step 1: extract the Compute op's input / output BufferIds,
    // trimmed to the count the installed kernel actually expects,
    // and (for binary ops with a known `folded_slot`) picking the
    // **unfolded** slot rather than blindly taking the first N.
    //
    // Phase 4.4 took the first `n_in` IR inputs.  That was fine
    // for the commutative-Add case but breaks Sub/Div/Pow when the
    // policy folds the LHS — we'd pass the LHS buffer (which is
    // the constant!) to the kernel and skip the RHS (the real
    // variable input).
    //
    // Phase 4.6 fix: when `folded_slot = Some(s)`, the unfolded
    // input is at slot `1 - s` (binary ops have exactly 2 IR
    // inputs).  When `folded_slot = None`, fall back to the
    // Phase 4.4 behaviour for commutative kernels.
    let md = {
        let g = match installed_kernel_metadata().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        g.get(&handle).copied().ok_or_else(|| {
            GpuError::Other(format!(
                "no kernel metadata for handle 0x{:016X} — did the install side run?",
                handle
            ))
        })?
    };
    let n_in = md.n_in;
    let (input_bufs, output_buf) = {
        let pop = &placed.ops[compute_op_idx];
        let PlacedOp::Compute { op: ir_op, .. } = pop else {
            return Err(GpuError::Other("not a Compute op".to_string()));
        };
        let ir_inputs = ir_op.inputs();
        if n_in > ir_inputs.len() {
            return Err(GpuError::Other(format!(
                "specialised kernel expects {} inputs but op has only {}",
                n_in,
                ir_inputs.len()
            )));
        }
        // Resolve which IR inputs to actually pass.
        //   - `folded_slot = Some(s)` and the op has exactly 2
        //     inputs and the kernel takes exactly 1 → pass the
        //     unfolded slot `1 - s`.
        //   - Otherwise → first `n_in` inputs in declared order.
        let selected_input_ids: Vec<matrix_ir::TensorId> = match md.folded_slot {
            Some(s) if ir_inputs.len() == 2 && n_in == 1 => {
                let unfolded = if s == 0 { 1usize } else { 0usize };
                vec![ir_inputs[unfolded]]
            }
            _ => ir_inputs.iter().take(n_in).copied().collect(),
        };
        let inputs: Vec<compute_ir::BufferId> = selected_input_ids
            .iter()
            .map(|in_id| {
                placed
                    .tensor(*in_id)
                    .map(|t| t.residency.buffer)
                    .ok_or_else(|| {
                        GpuError::Other(format!("tensor {} not in placed graph", in_id.0))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let out_id = ir_op.output();
        let out_buf = placed
            .tensor(out_id)
            .map(|t| t.residency.buffer)
            .ok_or_else(|| {
                GpuError::Other(format!("output tensor {} not in placed graph", out_id.0))
            })?;
        (inputs, out_buf)
    };

    // Step 2: build a "prep graph" = placed without the single
    // non-Const Compute op.  All Const + Alloc + Free + Transfer
    // ops stay, so matrix-metal allocates buffers and uploads
    // constants under planner-assigned IDs.
    let prep_graph = ComputeGraph {
        format_version: placed.format_version,
        tensors: placed.tensors.clone(),
        inputs: placed.inputs.clone(),
        outputs: placed.outputs.clone(),
        constants: placed.constants.clone(),
        ops: placed
            .ops
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != compute_op_idx)
            .map(|(_, op)| op.clone())
            .collect(),
    };

    // Step 3: fire prep Dispatch.
    let prep_resp = block_on(transport.request(ExecutorRequest::Dispatch {
        job_id: 100,
        graph: prep_graph,
    }))
    .map_err(|e| GpuError::Other(format!("prep dispatch: {:?}", e)))?;
    match prep_resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "prep dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to prep Dispatch: {:?}",
                other
            )));
        }
    }

    // Step 4: fire DispatchSpecialised for the single Compute op.
    let spec_resp = block_on(transport.request(ExecutorRequest::DispatchSpecialised {
        job_id: 101,
        handle,
        inputs: input_bufs,
        outputs: vec![output_buf],
    }))
    .map_err(|e| GpuError::Other(format!("specialised dispatch: {:?}", e)))?;
    match spec_resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "specialised dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to DispatchSpecialised: {:?}",
                other
            )));
        }
    }

    // Step 5: download the output.
    let download = block_on(transport.request(ExecutorRequest::DownloadBuffer {
        buffer: output_residency.buffer,
        offset: 0,
        len: output_byte_count as u64,
    }))
    .map_err(|e| GpuError::Other(format!("download: {:?}", e)))?;
    let data = match download {
        ExecutorResponse::BufferData { data, .. } => data,
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "download error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to DownloadBuffer: {:?}",
                other
            )));
        }
    };

    // Successful specialised dispatch → bump the counter.  Atomic
    // increment is `Relaxed` because the counter is observational.
    SPECIALISED_DISPATCH_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    Ok(data)
}

/// CPU-only path: re-plan with no Metal in the registry, then dispatch
/// on `matrix-cpu`.  This is both the no-Metal fallback and the
/// mixed-placement fallback.
fn dispatch_cpu_only(
    graph: &Graph,
    output_id: TensorId,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    let runtime = Runtime::new(matrix_cpu::profile());
    let placed: ComputeGraph = runtime
        .plan(graph)
        .map_err(|e| GpuError::Other(format!("plan: {:?}", e)))?;
    // **MX05 Phase 4.9.**  Run inside the thread-local CPU backend
    // so installed specialised kernels are visible.  Same pattern as
    // the dual-backend path above.
    with_cpu_backend(|b| {
        dispatch_via(&b.transport, placed, output_id, output_byte_count, "cpu")
    })
}

/// Returns `Some(id)` iff every `Compute` op and every constant in
/// `placed` references the same executor.  Returns `None` for the
/// mixed-placement case.
///
/// Empty graphs (no Compute ops, no constants) trivially count as
/// single-executor; we report `CPU_EXECUTOR` since there's nothing to
/// dispatch — the caller will hit a cheap empty CPU dispatch.
#[cfg(feature = "metal-backend")]
fn single_executor(placed: &ComputeGraph) -> Option<ExecutorId> {
    let mut ids: HashSet<ExecutorId> = HashSet::new();
    for op in &placed.ops {
        if let PlacedOp::Compute { executor, .. } = op {
            ids.insert(*executor);
        }
    }
    for c in &placed.constants {
        ids.insert(c.residency.executor);
    }
    match ids.len() {
        0 => Some(CPU_EXECUTOR),
        1 => ids.into_iter().next(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "metal-backend")]
    mod placement {
        use super::super::*;
        use compute_ir::{
            BufferId, OpTiming, PlacedConstant, PlacedOp, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{Op, TensorId};

        fn t(id: u32) -> TensorId {
            TensorId(id)
        }

        fn r(executor: u32, buffer: u64) -> Residency {
            Residency {
                executor: ExecutorId(executor),
                buffer: BufferId(buffer),
            }
        }

        fn empty_graph() -> ComputeGraph {
            ComputeGraph {
                format_version: WIRE_FORMAT_VERSION,
                inputs: vec![],
                outputs: vec![],
                constants: vec![],
                ops: vec![],
                tensors: vec![],
            }
        }

        #[test]
        fn single_executor_empty_is_cpu() {
            let g = empty_graph();
            assert_eq!(single_executor(&g), Some(CPU_EXECUTOR));
        }

        #[test]
        fn single_executor_all_metal() {
            let mut g = empty_graph();
            g.ops.push(PlacedOp::Compute {
                op: Op::Neg {
                    input: t(0),
                    output: t(1),
                },
                executor: ExecutorId(1),
                timing: OpTiming { estimated_ns: 0 },
            });
            g.constants.push(PlacedConstant {
                tensor: t(0),
                bytes: vec![0; 4],
                residency: r(1, 0),
            });
            assert_eq!(single_executor(&g), Some(ExecutorId(1)));
        }

        #[test]
        fn single_executor_mixed_returns_none() {
            let mut g = empty_graph();
            g.ops.push(PlacedOp::Compute {
                op: Op::Neg {
                    input: t(0),
                    output: t(1),
                },
                executor: CPU_EXECUTOR,
                timing: OpTiming { estimated_ns: 0 },
            });
            g.ops.push(PlacedOp::Compute {
                op: Op::Abs {
                    input: t(1),
                    output: t(2),
                },
                executor: ExecutorId(1),
                timing: OpTiming { estimated_ns: 0 },
            });
            assert_eq!(single_executor(&g), None);
        }
    }

    #[test]
    fn last_executor_starts_unset_per_thread() {
        // Run in a fresh thread to avoid bleed from earlier tests.
        std::thread::spawn(|| {
            assert_eq!(last_executor(), None);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn invert_records_an_executor_name() {
        // Build the smallest possible graph through the public API
        // and verify last_executor() reports something after dispatch.
        use crate::gpu_invert;
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(50, 100, 150, 255);

        let _ = gpu_invert(&img).unwrap();
        let exec = last_executor().expect("an executor name should be recorded");
        // CPU is always available; Metal may or may not be present.
        assert!(exec == "cpu" || exec == "metal", "unexpected: {}", exec);
    }

    /// MX05 Phase 3 V4 wiring smoke test.  Confirms that calling a
    /// public op (gpu_invert) drives the SpecRouter pipeline:
    /// observations accumulate.
    #[test]
    fn dispatch_drives_spec_router_pipeline() {
        use crate::{gpu_invert, profiler_observations};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        // Take a baseline.  The profiler is process-global so other
        // tests in this file may have already populated it; capture
        // the *delta* across this dispatch instead of asserting an
        // absolute count.
        let before: u64 = profiler_observations()
            .iter()
            .map(|o| o.invocation_count)
            .sum();

        let _ = gpu_invert(&img).unwrap();

        let after: u64 = profiler_observations()
            .iter()
            .map(|o| o.invocation_count)
            .sum();
        assert!(
            after > before,
            "gpu_invert should bump at least one observation counter \
             (before = {}, after = {})",
            before,
            after
        );
    }

    /// MX05 Phase 4.2 end-to-end visibility test.  Drives `gpu_invert`
    /// past the spec MX05 default 1000-invocation threshold and
    /// asserts that [`spec_cache_len`] rises above zero.
    ///
    /// Up to V4.1 (PR #2165) image-gpu-core used a custom HotPolicy
    /// at threshold 100 because `drive_specialisation` didn't yet
    /// sample tensor bytes — `DefaultPolicy`'s constant-input check
    /// requires `observed_min == observed_max` on at least one input
    /// tensor, which means somebody has to call `Profiler::sample_tensor`.
    /// Phase 4.2 adds that sampling, so we can switch back to
    /// `DefaultPolicy::new()` (threshold 1000, stability 0.95) and
    /// the cache still fills — now backed by real constant-input
    /// observations rather than just hotness.
    #[test]
    fn default_policy_populates_cache_via_constant_input_sampling() {
        use crate::{gpu_invert, spec_cache_len};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        let before = spec_cache_len();

        // Drive past the default threshold of 1000 invocations.  Each
        // gpu_invert call is fast (small graph, CPU dispatch);
        // 1100 iterations runs in <100 ms on commodity hardware.
        for _ in 0..1100 {
            let _ = gpu_invert(&img).unwrap();
        }

        let after = spec_cache_len();
        assert!(
            after > before,
            "expected spec cache to grow after 1100 gpu_invert calls under DefaultPolicy; \
             before = {}, after = {}",
            before,
            after
        );
    }

    /// Phase 4.2 also makes `tensor_observations` actually populated
    /// (was always empty in earlier phases).  This test confirms that
    /// driving a single dispatch records at least one input
    /// observation with a stable min/max (which is the signal
    /// `DefaultPolicy` reads).
    #[test]
    fn drive_specialisation_populates_tensor_observations() {
        use crate::{gpu_invert, profiler_observations};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        let _ = gpu_invert(&img).unwrap();

        let obs = profiler_observations();
        let total_tensor_observations: usize =
            obs.iter().map(|o| o.tensor_observations.len()).sum();
        assert!(
            total_tensor_observations > 0,
            "expected at least one TensorObservation populated after gpu_invert; got 0"
        );
    }

    /// **MX05 Phase 4.3 end-to-end auto-installer test.**  Builds a
    /// hot graph whose Add op has a stable f32 constant on at least one
    /// input, drives it past the `DefaultPolicy` 1000-invocation
    /// threshold, and asserts that `specialised_install_count` rises
    /// above zero — proving the chain:
    ///
    /// ```text
    ///   route() returns Some(SpecialisedKernel)
    ///     → try_auto_install_specialised()
    ///       → msl_emitter::emit_specialised_kernel(...)
    ///         → MetalExecutor::install_specialised_from_emitted(...)
    ///           → SpecialisedTable grows
    /// ```
    ///
    /// Apple-only — `metal-backend` is feature-gated *at compile
    /// time* and is enabled by default on every platform, but the
    /// actual Metal device is only available at runtime on Apple
    /// targets (`MetalExecutor::new` returns `Err` elsewhere).  So
    /// we gate on `target_vendor = "apple"` rather than the feature
    /// flag — on Linux/Windows CI, `specialised_install_count`
    /// stays at 0 by definition and there's nothing to assert.
    ///
    /// Test scope intentionally limited:
    /// - We only assert that the **install** side fires.  Phase 4.4
    ///   will land the dispatch-routing side (replacing the generic
    ///   `Dispatch { graph }` request with per-op `DispatchSpecialised`
    ///   requests) and add its own end-to-end speedup test.
    /// - We use raw `matrix_ir::GraphBuilder` rather than a public
    ///   filter because v0.6.0's `msl_emitter` only knows
    ///   `Op::Add + F32 + Constant`, and none of the existing public
    ///   filters happen to hit that exact shape.  Future emitter
    ///   extensions (Sub/Mul/Div) will broaden the surface so that
    ///   real filters drive the install path automatically.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn auto_installer_registers_kernel_after_threshold() {
        use crate::specialised_install_count;
        use matrix_ir::{DType, GraphBuilder, Shape};

        // Snapshot the install count *before* this test runs — other
        // tests in this process may have already populated the table,
        // so we look for a delta rather than an absolute value.
        let before = specialised_install_count();

        // Run the same Add-of-constants graph 1100 times.  Both
        // operands are stable f32 constants, so the DefaultPolicy's
        // constant-input check will fire once `tensor_observations`
        // accumulates enough samples (~1000 invocations).
        for _ in 0..1100 {
            let mut g = GraphBuilder::new();
            // Use 4-element f32 vectors so the emitter's
            // `add_f32 + folded constant` kernel has a meaningful
            // workload (4 elements × 1 thread per element).
            let a_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let b_bytes: Vec<u8> = [7.0_f32, 7.0, 7.0, 7.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let a = g.constant(DType::F32, Shape::from(&[4u32]), a_bytes);
            let b = g.constant(DType::F32, Shape::from(&[4u32]), b_bytes);
            let c = g.add(&a, &b);
            g.output(&c);
            let graph = g.build().expect("graph builds");
            let _ = crate::pipeline::run_graph_with_constant_inputs(&graph, c.id, 16);
        }

        let after = specialised_install_count();
        // The install count must rise — under DefaultPolicy + the
        // matrix-cpu specialiser + the metal-side msl_emitter
        // auto-install, at least one Add-with-constant kernel must
        // have been emitted, compiled, and registered with the metal
        // executor.  If this assertion fails it means *some* link in
        // the chain (sampling, policy, router, emitter, compile,
        // install) regressed.
        assert!(
            after > before,
            "expected auto-installer to fire after 1100 invocations of an \
             Add-with-constant graph; before = {}, after = {}",
            before,
            after
        );
    }

    /// **MX05 Phase 4.4 end-to-end dispatch-routing test.**  Builds a
    /// placed graph manually pinned to metal, installs an
    /// `Add-with-constant` specialised kernel on the metal executor,
    /// then drives `dispatch_specialised_via` directly and asserts:
    ///
    /// 1. `specialised_dispatch_count` rises above zero — proves the
    ///    `DispatchSpecialised` request actually fired and the metal
    ///    executor returned `DispatchDone`.
    /// 2. The downloaded output bytes match what the generic kernel
    ///    would compute — the specialised path is functionally
    ///    equivalent.  Inputs are `[1, 2, 3, 4]` and a folded constant
    ///    of `7.0`, so the output must be `[8, 9, 10, 11]`.
    ///
    /// # Why manually-built, not planner-driven?
    ///
    /// The cost model in matrix-runtime currently prefers CPU over
    /// metal for the `Op::Add(f32, f32) -> f32` shape regardless of
    /// `N` — the per-element host→device transfer cost
    /// (`bytes / host_to_device_bw = N*4 / 50` ns) exceeds the CPU
    /// per-element compute cost (`N / 40` ns), and `Op::Add` is only
    /// 1 flop/element so even very large graphs don't tip the
    /// balance.  To drive a real planner-picks-metal scenario we'd
    /// need either (a) a heavier op like `Op::MatMul` (Phase 4.5
    /// emitter work), (b) more realistic profile numbers (Apple
    /// Silicon's unified memory is closer to 200 GB/s effective
    /// bandwidth), or (c) constants persistent across invocations
    /// (a future protocol extension).  This test side-steps the
    /// planner concern and verifies the *protocol-level* dispatch
    /// routing in isolation — the planner-side decision logic is
    /// covered by existing tests in matrix-runtime/src/planner.rs.
    ///
    /// Apple-only: without a real Metal device the dispatch-specialised
    /// path can't fire.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn dispatch_specialised_via_produces_correct_output() {
        use crate::specialised_dispatch_count;
        use compute_ir::{
            BufferId, ComputeGraph, ExecutorId as ComputeExecutorId, OpTiming as PlanOpTiming,
            PlacedConstant, PlacedOp, PlacedTensor, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{DType, Op, Shape, TensorId};

        // Step 1: ensure metal is available; otherwise we'd be testing
        // matrix-metal's non-Apple stubs which short-circuit.
        let backend = match metal_backend() {
            Some(b) => b,
            None => return, // Skip on environments without Metal.
        };

        // Step 2: emit + compile + install an Add-with-constant
        // kernel under a known handle.  Bypass the auto-installer so
        // the test is hermetic.
        let constant: f32 = 7.0;
        let spec_key = matrix_runtime::SpecKey {
            op_kind: 0x07, // Op::Add
            dtype: DType::F32,
            shape_class: matrix_runtime::ShapeClass::Dynamic,
            range_class: matrix_runtime::RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1, // metal convention
            folded_slot: Some(1),
        };
        const TEST_HANDLE: u64 = 0xC0DE_C0DE_C0DE_C0DE;
        let emitted = matrix_metal::emit_specialised_kernel(&spec_key, TEST_HANDLE)
            .expect("emitter must support the canonical Add+const f32 SpecKey");
        // Capture metadata before `emitted` is consumed by install.
        let n_in = emitted.input_buffer_count;
        let n_out = emitted.output_buffer_count;
        backend
            .executor
            .install_specialised_from_emitted(TEST_HANDLE, emitted)
            .expect("install must succeed on a real Metal device");
        // Replicate what `try_auto_install_specialised` records so
        // `dispatch_specialised_via` can find the metadata.
        record_test_kernel_metadata(TEST_HANDLE, n_in, n_out, Some(1));

        // Step 3: build a placed ComputeGraph pinned to metal that:
        //   Op::Const → tensor A (the variable operand bytes)
        //   Op::Const → tensor B (the constant 7.0 × 4)
        //   PlacedOp::Alloc → output buffer
        //   Op::Add(A, B) → tensor C
        // The B tensor / constant is what the specialised kernel
        // folds in.  After `dispatch_specialised_via` removes the
        // Add op, the prep-graph just allocates buffers and uploads
        // the two constants.  Then DispatchSpecialised invokes the
        // installed Add+7.0 kernel reading from A and writing to C.
        let metal_exec_id = ComputeExecutorId(1);
        let a_residency = Residency {
            executor: metal_exec_id,
            buffer: BufferId(10),
        };
        let b_residency = Residency {
            executor: metal_exec_id,
            buffer: BufferId(11),
        };
        let c_residency = Residency {
            executor: metal_exec_id,
            buffer: BufferId(12),
        };
        let a_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let b_bytes: Vec<u8> = [constant; 4]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let f32_shape4 = Shape::from(&[4u32]);

        let tensor_a = PlacedTensor {
            id: TensorId(0),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: a_residency,
        };
        let tensor_b = PlacedTensor {
            id: TensorId(1),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: b_residency,
        };
        let tensor_c = PlacedTensor {
            id: TensorId(2),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: c_residency,
        };

        let const_a = PlacedConstant {
            tensor: TensorId(0),
            residency: a_residency,
            bytes: a_bytes,
        };
        let const_b = PlacedConstant {
            tensor: TensorId(1),
            residency: b_residency,
            bytes: b_bytes,
        };

        // Const ops materialise the constants from the table into
        // their declared buffers.  The Add op consumes them.
        let placed = ComputeGraph {
            format_version: WIRE_FORMAT_VERSION,
            inputs: vec![],
            outputs: vec![tensor_c.clone()],
            constants: vec![const_a, const_b],
            ops: vec![
                PlacedOp::Compute {
                    op: Op::Const {
                        constant: 0,
                        output: TensorId(0),
                    },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Compute {
                    op: Op::Const {
                        constant: 1,
                        output: TensorId(1),
                    },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Alloc {
                    residency: c_residency,
                    bytes: 16,
                },
                PlacedOp::Compute {
                    op: Op::Add {
                        lhs: TensorId(0),
                        rhs: TensorId(1),
                        output: TensorId(2),
                    },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
            ],
            tensors: vec![tensor_a, tensor_b, tensor_c.clone()],
        };

        // Step 4: call dispatch_specialised_via.  The Add op is at
        // index 3.
        let before = specialised_dispatch_count();
        let bytes = dispatch_specialised_via(
            &backend.transport,
            &placed,
            3, // index of the Add op in `placed.ops`
            TEST_HANDLE,
            c_residency,
            16,
        )
        .expect("specialised dispatch must succeed");
        let after = specialised_dispatch_count();

        // Step 5: assertions.
        assert!(
            after > before,
            "specialised_dispatch_count must rise after a successful \
             DispatchSpecialised; before = {}, after = {}",
            before,
            after
        );

        // Expected output: A + constant = [1, 2, 3, 4] + 7.0 = [8, 9, 10, 11].
        let result: Vec<f32> = bytes
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(
            result,
            vec![8.0, 9.0, 10.0, 11.0],
            "specialised Add+7.0 kernel output must match generic Add"
        );
    }

    /// **MX05 Phase 4.6 end-to-end test.**  Asserts the dispatcher
    /// correctly picks the **unfolded** input when the policy folded
    /// the LHS of a non-commutative op.
    ///
    /// Graph: `Op::Sub(A = [10, 10, 10, 10], B = [1, 2, 3, 4]) → C`,
    /// where A (LHS) is observed as the constant `10.0`.  The
    /// specialised kernel is the `sub_lhs_const_f32` variant — it
    /// computes `K - b[gid]` where `K = 10.0`.  Expected output:
    /// `[10 - 1, 10 - 2, 10 - 3, 10 - 4] = [9, 8, 7, 6]`.
    ///
    /// If the dispatcher mistakenly passed A's buffer (the LHS, the
    /// constant) instead of B's buffer (the RHS, the variable), the
    /// kernel would compute `K - a[gid] = 10 - 10 = 0` for every
    /// element — the test would fail with `[0,0,0,0]`.  That's the
    /// Phase 4.4 → Phase 4.6 regression this test pins.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn dispatch_specialised_via_routes_lhs_folded_correctly() {
        use crate::specialised_dispatch_count;
        use compute_ir::{
            BufferId, ComputeGraph, ExecutorId as ComputeExecutorId, OpTiming as PlanOpTiming,
            PlacedConstant, PlacedOp, PlacedTensor, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{DType, Op, Shape, TensorId};

        let backend = match metal_backend() {
            Some(b) => b,
            None => return,
        };

        // Install a Sub-with-LHS-folded-constant kernel: K = 10.0,
        // folded_slot = 0 (LHS).  Kernel computes `10.0 - a[gid]`.
        let constant: f32 = 10.0;
        let spec_key = matrix_runtime::SpecKey {
            op_kind: 0x08, // Op::Sub
            dtype: DType::F32,
            shape_class: matrix_runtime::ShapeClass::Dynamic,
            range_class: matrix_runtime::RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(0), // LHS is the constant
        };
        const TEST_HANDLE: u64 = 0xABC0_ABC0_ABC0_ABC0;
        let emitted = matrix_metal::emit_specialised_kernel(&spec_key, TEST_HANDLE)
            .expect("Phase 4.6 emitter must support Sub with LHS-folded constant");
        let n_in = emitted.input_buffer_count;
        let n_out = emitted.output_buffer_count;
        backend
            .executor
            .install_specialised_from_emitted(TEST_HANDLE, emitted)
            .expect("install must succeed on a real Metal device");
        record_test_kernel_metadata(TEST_HANDLE, n_in, n_out, Some(0));

        // Build the placed graph: A = [10, 10, 10, 10] (LHS const),
        // B = [1, 2, 3, 4] (RHS const, the variable from the
        // dispatcher's POV), C = A - B.  Because both inputs are
        // graph-level constants, both go through Op::Const into
        // metal buffers; the dispatcher then routes
        // DispatchSpecialised with the unfolded slot (slot 1 = B).
        let metal_exec_id = ComputeExecutorId(1);
        let a_residency = Residency { executor: metal_exec_id, buffer: BufferId(20) };
        let b_residency = Residency { executor: metal_exec_id, buffer: BufferId(21) };
        let c_residency = Residency { executor: metal_exec_id, buffer: BufferId(22) };
        let a_bytes: Vec<u8> = [constant; 4].iter().flat_map(|v| v.to_le_bytes()).collect();
        let b_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let f32_shape4 = Shape::from(&[4u32]);

        let tensor_a = PlacedTensor {
            id: TensorId(0),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: a_residency,
        };
        let tensor_b = PlacedTensor {
            id: TensorId(1),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: b_residency,
        };
        let tensor_c = PlacedTensor {
            id: TensorId(2),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: c_residency,
        };

        let placed = ComputeGraph {
            format_version: WIRE_FORMAT_VERSION,
            inputs: vec![],
            outputs: vec![tensor_c.clone()],
            constants: vec![
                PlacedConstant {
                    tensor: TensorId(0),
                    residency: a_residency,
                    bytes: a_bytes,
                },
                PlacedConstant {
                    tensor: TensorId(1),
                    residency: b_residency,
                    bytes: b_bytes,
                },
            ],
            ops: vec![
                PlacedOp::Compute {
                    op: Op::Const { constant: 0, output: TensorId(0) },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Compute {
                    op: Op::Const { constant: 1, output: TensorId(1) },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Alloc {
                    residency: c_residency,
                    bytes: 16,
                },
                PlacedOp::Compute {
                    op: Op::Sub {
                        lhs: TensorId(0),
                        rhs: TensorId(1),
                        output: TensorId(2),
                    },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
            ],
            tensors: vec![tensor_a, tensor_b, tensor_c.clone()],
        };

        let before = specialised_dispatch_count();
        let bytes = dispatch_specialised_via(
            &backend.transport,
            &placed,
            3, // Sub op is at index 3
            TEST_HANDLE,
            c_residency,
            16,
        )
        .expect("specialised dispatch must succeed");
        let after = specialised_dispatch_count();

        assert!(after > before, "specialised_dispatch_count must rise");

        // Expected: 10 - B = [10-1, 10-2, 10-3, 10-4] = [9, 8, 7, 6].
        let result: Vec<f32> = bytes
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(
            result,
            vec![9.0, 8.0, 7.0, 6.0],
            "Phase 4.6 LHS-folded Sub: dispatcher must pass the unfolded \
             RHS slot to the kernel, not the LHS.  Got {:?} — if this is \
             [0, 0, 0, 0] then the dispatcher passed the constant LHS \
             instead of the variable RHS.",
            result
        );
    }

    /// **MX05 Phase 4.7 end-to-end test.**  Unary op with folded
    /// input constant — the kernel takes **zero** input buffers and
    /// writes the precomputed `f(K)` to every output element.
    ///
    /// Graph: `Op::Sqrt(input = [16, 16, 16, 16]) → C`.  The input
    /// is observed as the constant `K = 16.0`, so the specialised
    /// kernel is `sqrt_input_const_f32` which writes `√16 = 4.0`
    /// everywhere.  Expected output: `[4.0, 4.0, 4.0, 4.0]`.
    ///
    /// This exercises the `n_in == 0` path in
    /// `dispatch_specialised_via`: the `DispatchSpecialised` request
    /// carries an empty `inputs: vec![]`, and the metal kernel only
    /// binds the output buffer at `buffer(0)`.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn dispatch_specialised_via_routes_unary_folded_input() {
        use crate::specialised_dispatch_count;
        use compute_ir::{
            BufferId, ComputeGraph, ExecutorId as ComputeExecutorId, OpTiming as PlanOpTiming,
            PlacedConstant, PlacedOp, PlacedTensor, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{DType, Op, Shape, TensorId};

        let backend = match metal_backend() {
            Some(b) => b,
            None => return,
        };

        // Install a Sqrt-with-input=16.0 kernel.  folded_slot = 0
        // (unary ops have only one input slot).
        let constant: f32 = 16.0;
        let spec_key = matrix_runtime::SpecKey {
            op_kind: 0x02, // Op::Sqrt
            dtype: DType::F32,
            shape_class: matrix_runtime::ShapeClass::Dynamic,
            range_class: matrix_runtime::RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(0),
        };
        const TEST_HANDLE: u64 = 0x1717_1717_1717_1717;
        let emitted = matrix_metal::emit_specialised_kernel(&spec_key, TEST_HANDLE)
            .expect("Phase 4.7 emitter must support Sqrt with folded input");
        assert_eq!(emitted.input_buffer_count, 0, "unary kernel must take 0 inputs");
        let n_in = emitted.input_buffer_count;
        let n_out = emitted.output_buffer_count;
        backend
            .executor
            .install_specialised_from_emitted(TEST_HANDLE, emitted)
            .expect("install must succeed on a real Metal device");
        record_test_kernel_metadata(TEST_HANDLE, n_in, n_out, Some(0));

        // Build the placed graph:
        //   Op::Const → tensor A ([16, 16, 16, 16])
        //   PlacedOp::Alloc → output buffer
        //   Op::Sqrt(A) → tensor B
        //
        // The Sqrt op's input is A; the prep dispatch will still
        // run the Const op (writes 16.0 four times into a buffer
        // that the specialised kernel never reads).  The specialised
        // kernel writes 4.0 to every element of B.  Wasted work for
        // the Const but correct.
        let metal_exec_id = ComputeExecutorId(1);
        let a_residency = Residency { executor: metal_exec_id, buffer: BufferId(30) };
        let b_residency = Residency { executor: metal_exec_id, buffer: BufferId(31) };
        let a_bytes: Vec<u8> = [constant; 4].iter().flat_map(|v| v.to_le_bytes()).collect();
        let f32_shape4 = Shape::from(&[4u32]);

        let tensor_a = PlacedTensor {
            id: TensorId(0),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: a_residency,
        };
        let tensor_b = PlacedTensor {
            id: TensorId(1),
            dtype: DType::F32,
            shape: f32_shape4.clone(),
            residency: b_residency,
        };

        let placed = ComputeGraph {
            format_version: WIRE_FORMAT_VERSION,
            inputs: vec![],
            outputs: vec![tensor_b.clone()],
            constants: vec![PlacedConstant {
                tensor: TensorId(0),
                residency: a_residency,
                bytes: a_bytes,
            }],
            ops: vec![
                PlacedOp::Compute {
                    op: Op::Const { constant: 0, output: TensorId(0) },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Alloc {
                    residency: b_residency,
                    bytes: 16,
                },
                PlacedOp::Compute {
                    op: Op::Sqrt { input: TensorId(0), output: TensorId(1) },
                    executor: metal_exec_id,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
            ],
            tensors: vec![tensor_a, tensor_b.clone()],
        };

        let before = specialised_dispatch_count();
        let bytes = dispatch_specialised_via(
            &backend.transport,
            &placed,
            2, // Sqrt op is at index 2
            TEST_HANDLE,
            b_residency,
            16,
        )
        .expect("specialised unary dispatch must succeed");
        let after = specialised_dispatch_count();

        assert!(after > before, "specialised_dispatch_count must rise");

        // Expected: [√16, √16, √16, √16] = [4.0, 4.0, 4.0, 4.0].
        let result: Vec<f32> = bytes
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(
            result,
            vec![4.0, 4.0, 4.0, 4.0],
            "Phase 4.7 Sqrt with folded input=16: kernel must memset √16=4.0"
        );
    }

    /// **MX05 Phase 4.8 end-to-end test.**  Routes a graph with
    /// **two chained specialised Compute ops** through the
    /// `dispatch_specialised_via_multi` path.
    ///
    /// Graph (data flow):
    /// ```
    ///   x = [1, 2, 3, 4]
    ///   const_3 = [3, 3, 3, 3]   (folded as the RHS of Add)
    ///   const_2 = [2, 2, 2, 2]   (folded as the RHS of Mul)
    ///   y = Add(x, const_3)      → [4, 5, 6, 7]
    ///   z = Mul(y, const_2)      → [8, 10, 12, 14]    ← graph output
    /// ```
    /// Both Add and Mul are commutative — we use the canonical
    /// `add_const_f32` and `mul_const_f32` kernels with the RHS
    /// folded (`folded_slot = Some(1)`).  Each gets its own handle;
    /// both are installed before dispatch; `dispatch_via` then
    /// recognises both Compute ops have installed kernels and
    /// routes through `dispatch_specialised_via_multi`.
    ///
    /// Assertions:
    ///   1. `specialised_dispatch_count` rises by **2** (one per
    ///      Compute op).
    ///   2. The final output bytes are `[8, 10, 12, 14]`.  If the
    ///      multi-op dispatcher had skipped one of the ops, or had
    ///      reordered them, the output would differ.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn dispatch_multi_op_specialised_chain_produces_correct_output() {
        use crate::specialised_dispatch_count;
        use compute_ir::{
            BufferId, ComputeGraph, ExecutorId as ComputeExecutorId, OpTiming as PlanOpTiming,
            PlacedConstant, PlacedOp, PlacedTensor, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{DType, Op, Shape, TensorId};

        let backend = match metal_backend() {
            Some(b) => b,
            None => return,
        };

        // ── Install kernel for the Add(_, 3) specialisation ──
        let add_const: f32 = 3.0;
        let add_key = matrix_runtime::SpecKey {
            op_kind: 0x07, // Op::Add
            dtype: DType::F32,
            shape_class: matrix_runtime::ShapeClass::Dynamic,
            range_class: matrix_runtime::RangeClass::Constant {
                bytes: add_const.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(1), // RHS folded (Add is commutative; convention)
        };
        const ADD_HANDLE: u64 = 0x0808_0808_0808_0808;
        let add_emitted = matrix_metal::emit_specialised_kernel(&add_key, ADD_HANDLE).unwrap();
        let add_n_in = add_emitted.input_buffer_count;
        let add_n_out = add_emitted.output_buffer_count;
        backend
            .executor
            .install_specialised_from_emitted(ADD_HANDLE, add_emitted)
            .unwrap();
        record_test_kernel_metadata(ADD_HANDLE, add_n_in, add_n_out, Some(1));

        // ── Install kernel for the Mul(_, 2) specialisation ──
        let mul_const: f32 = 2.0;
        let mul_key = matrix_runtime::SpecKey {
            op_kind: 0x09, // Op::Mul
            dtype: DType::F32,
            shape_class: matrix_runtime::ShapeClass::Dynamic,
            range_class: matrix_runtime::RangeClass::Constant {
                bytes: mul_const.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(1),
        };
        const MUL_HANDLE: u64 = 0x0909_0909_0909_0909;
        let mul_emitted = matrix_metal::emit_specialised_kernel(&mul_key, MUL_HANDLE).unwrap();
        let mul_n_in = mul_emitted.input_buffer_count;
        let mul_n_out = mul_emitted.output_buffer_count;
        backend
            .executor
            .install_specialised_from_emitted(MUL_HANDLE, mul_emitted)
            .unwrap();
        record_test_kernel_metadata(MUL_HANDLE, mul_n_in, mul_n_out, Some(1));

        // ── Build the placed graph ──
        // Tensors:
        //   0 = x
        //   1 = const_3
        //   2 = y       (Add(x, const_3))
        //   3 = const_2
        //   4 = z       (Mul(y, const_2))   ← output
        let metal = ComputeExecutorId(1);
        let r = |b| Residency { executor: metal, buffer: BufferId(b) };
        let r_x = r(40);
        let r_c3 = r(41);
        let r_y = r(42);
        let r_c2 = r(43);
        let r_z = r(44);
        let shape4 = Shape::from(&[4u32]);
        let mk_tensor = |id, res| PlacedTensor {
            id: TensorId(id),
            dtype: DType::F32,
            shape: shape4.clone(),
            residency: res,
        };
        let x_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let c3_bytes: Vec<u8> = [add_const; 4].iter().flat_map(|v| v.to_le_bytes()).collect();
        let c2_bytes: Vec<u8> = [mul_const; 4].iter().flat_map(|v| v.to_le_bytes()).collect();

        let placed = ComputeGraph {
            format_version: WIRE_FORMAT_VERSION,
            inputs: vec![],
            outputs: vec![mk_tensor(4, r_z)],
            constants: vec![
                PlacedConstant {
                    tensor: TensorId(0),
                    residency: r_x,
                    bytes: x_bytes,
                },
                PlacedConstant {
                    tensor: TensorId(1),
                    residency: r_c3,
                    bytes: c3_bytes,
                },
                PlacedConstant {
                    tensor: TensorId(3),
                    residency: r_c2,
                    bytes: c2_bytes,
                },
            ],
            ops: vec![
                PlacedOp::Compute {
                    op: Op::Const { constant: 0, output: TensorId(0) },
                    executor: metal,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Compute {
                    op: Op::Const { constant: 1, output: TensorId(1) },
                    executor: metal,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Alloc {
                    residency: r_y,
                    bytes: 16,
                },
                PlacedOp::Compute {
                    op: Op::Add {
                        lhs: TensorId(0),
                        rhs: TensorId(1),
                        output: TensorId(2),
                    },
                    executor: metal,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Compute {
                    op: Op::Const { constant: 2, output: TensorId(3) },
                    executor: metal,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
                PlacedOp::Alloc {
                    residency: r_z,
                    bytes: 16,
                },
                PlacedOp::Compute {
                    op: Op::Mul {
                        lhs: TensorId(2),
                        rhs: TensorId(3),
                        output: TensorId(4),
                    },
                    executor: metal,
                    timing: PlanOpTiming { estimated_ns: 0 },
                },
            ],
            tensors: vec![
                mk_tensor(0, r_x),
                mk_tensor(1, r_c3),
                mk_tensor(2, r_y),
                mk_tensor(3, r_c2),
                mk_tensor(4, r_z),
            ],
        };

        // Non-Const Compute ops are at indices 3 (Add) and 6 (Mul).
        let computes = vec![(3usize, ADD_HANDLE), (6usize, MUL_HANDLE)];

        let before = specialised_dispatch_count();
        let bytes = dispatch_specialised_via_multi(
            &backend.transport,
            &placed,
            &computes,
            r_z,
            16,
        )
        .expect("multi-op specialised dispatch must succeed");
        let after = specialised_dispatch_count();

        // Counter must rise by **at least** the number of
        // specialised ops we fired.  `>=` instead of `==` because
        // cargo runs `#[test]` functions in parallel and concurrent
        // tests increment the same process-wide counter.  This
        // dispatch contributed exactly 2 increments; concurrent
        // tests may have added more.
        assert!(
            after >= before + 2,
            "specialised_dispatch_count must rise by at least 2 \
             (one per Compute op); before = {}, after = {}",
            before,
            after
        );

        // Expected output: (x + 3) * 2 = ([1,2,3,4] + 3) * 2 = [8, 10, 12, 14].
        let result: Vec<f32> = bytes
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(
            result,
            vec![8.0, 10.0, 12.0, 14.0],
            "Phase 4.8 multi-op chain Add+Mul must produce (x + 3) * 2"
        );
    }

    /// **MX05 Phase 4.9 end-to-end test.**  Auto-installer + dispatch
    /// routing on the **CPU** executor.  Phase 4.3 introduced the
    /// auto-installer for metal but left the CPU side as a "Phase 4.9
    /// or later" TODO.  Phase 4.9 closes that gap.
    ///
    /// This test drives a small Add-with-constant graph through
    /// `run_graph_with_constant_inputs` enough times to fire
    /// `DefaultPolicy` (1100 iterations).  After the policy fires:
    ///   - `r.route(...)` returns `Some(SpecialisedKernel)` with
    ///     `backend_id = 0` (CPU)
    ///   - `try_auto_install_specialised` matches the CPU branch
    ///     and calls `matrix_cpu::build_specialised_kernel` to
    ///     produce a Rust closure
    ///   - The closure is installed on the thread-local
    ///     `cpu_backend().executor` via `CpuExecutor::install_specialised`
    ///   - `specialised_install_count` rises above zero
    ///
    /// This is the CPU counterpart of Phase 4.3's
    /// `auto_installer_registers_kernel_after_threshold` (which
    /// covered the metal path).  Runs on **every platform** —
    /// unlike the metal tests, this doesn't need an Apple device.
    /// On non-Apple targets, the planner picks CPU anyway because
    /// there's no metal executor registered.
    #[test]
    fn cpu_auto_installer_registers_kernel_after_threshold() {
        use crate::specialised_install_count;
        use matrix_ir::{DType, GraphBuilder, Shape};

        let before = specialised_install_count();

        // Tiny f32 Add with a stable constant operand.  Use an
        // **8-element** shape (rather than the 4-element shape that
        // `auto_installer_registers_kernel_after_threshold` uses)
        // so the resulting graph subhash is distinct.  The Profiler
        // keys observations by `(subhash, op_index, slot)`, and the
        // subhash hashes structural fields (op kinds, Alloc bytes,
        // tensor ids) but **not** constant payload bytes — so two
        // tests using the same structure with different K values
        // would accumulate observations into the same bucket, mixing
        // their min/max ranges and breaking the constant detection
        // for both.  Distinct shapes → distinct subhashes →
        // independent observations.
        for _ in 0..1100 {
            let mut g = GraphBuilder::new();
            let a_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let b_bytes: Vec<u8> = [11.0_f32; 8]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let a = g.constant(DType::F32, Shape::from(&[8u32]), a_bytes);
            let b = g.constant(DType::F32, Shape::from(&[8u32]), b_bytes);
            let c = g.add(&a, &b);
            g.output(&c);
            let graph = g.build().expect("graph builds");
            let _ = crate::pipeline::run_graph_with_constant_inputs(&graph, c.id, 32);
        }

        let after = specialised_install_count();
        assert!(
            after > before,
            "Phase 4.9: CPU auto-installer must fire after 1100 invocations \
             of an Add-with-constant graph; before = {}, after = {}.  If 0 \
             here, either (a) the policy didn't fire (check tensor sampling \
             in drive_specialisation), or (b) build_specialised_kernel \
             returned None for the SpecKey (check matrix-cpu coverage).",
            before,
            after
        );
    }

    /// **MX05 Phase 4.10 end-to-end test.**  CPU specialised dispatch
    /// of `Op::MatMul` with a folded 2×2 RHS matrix.
    ///
    /// Builds the closure directly via `matrix_cpu::build_specialised_kernel`,
    /// installs it on the thread-local CPU executor, then invokes it
    /// via the BufferStore to verify the matrix multiplication is
    /// correct end-to-end.
    ///
    /// Graph (semantically):
    ///   `A = [[1, 2], [3, 4]]`   ← runtime input (`[2, 2]`)
    ///   `B = [[5, 6], [7, 8]]`   ← folded constant (`[2, 2]`)
    ///   `C = A × B = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]`
    ///                ` = [[19, 22], [43, 50]]`
    ///
    /// We don't go through the planner here — the test is hermetic.
    /// The Phase 4.9 auto-installer + Phase 4.8 multi-op dispatcher
    /// would normally fire this for graphs that reach the
    /// 1000-invocation threshold; this test validates the closure
    /// + buffer plumbing in isolation.
    #[test]
    fn cpu_matmul_folded_rhs_2x2_produces_correct_output() {
        use matrix_runtime::{RangeClass, ShapeClass, SpecKey};

        // Folded RHS matrix B = [[5, 6], [7, 8]] (row-major).
        let b_bytes: Vec<u8> = [5.0_f32, 6.0, 7.0, 8.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let key = SpecKey {
            op_kind: 0x15, // Op::MatMul
            dtype: matrix_ir::DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes: b_bytes },
            backend_id: 0, // CPU
            folded_slot: Some(1), // RHS folded
        };
        let kernel = matrix_cpu::build_specialised_kernel(&key, 0xCAFE)
            .expect("matrix-cpu must build a 2x2 MatMul kernel");

        // Install on the thread-local CPU executor and seed the
        // input buffer.
        with_cpu_backend(|backend| {
            const TEST_HANDLE: u64 = 0xCAFE_BABE;
            backend.executor.install_specialised(TEST_HANDLE, kernel);

            // A = [[1, 2], [3, 4]] flattened.  4 elements × 4 bytes
            // = 16 bytes.
            let a_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();

            // Use the executor directly (not the protocol) so the
            // test stays focused on the kernel's correctness.
            let input_buf = compute_ir::BufferId(100);
            let output_buf = compute_ir::BufferId(101);
            // Pre-allocate buffers via protocol calls so the
            // executor's BufferStore is in the right state.
            let alloc_resp = backend.executor.handle(
                executor_protocol::ExecutorRequest::AllocBuffer { bytes: 16 },
            );
            let _ = alloc_resp; // server assigns its own id; we sidestep that here

            // Actually use the executor's `handle()` for end-to-end
            // protocol-level testing.  Allocate two server-assigned
            // buffer ids.
            let _ = (input_buf, output_buf);
            let in_buf = match backend.executor.handle(
                executor_protocol::ExecutorRequest::AllocBuffer { bytes: 16 },
            ) {
                executor_protocol::ExecutorResponse::BufferAllocated { buffer } => buffer,
                other => panic!("expected BufferAllocated, got {:?}", other),
            };
            let out_buf = match backend.executor.handle(
                executor_protocol::ExecutorRequest::AllocBuffer { bytes: 16 },
            ) {
                executor_protocol::ExecutorResponse::BufferAllocated { buffer } => buffer,
                other => panic!("expected BufferAllocated, got {:?}", other),
            };
            // Upload A.
            let upload_resp = backend.executor.handle(
                executor_protocol::ExecutorRequest::UploadBuffer {
                    buffer: in_buf,
                    offset: 0,
                    data: a_bytes,
                },
            );
            assert!(
                matches!(
                    upload_resp,
                    executor_protocol::ExecutorResponse::BufferUploaded { .. }
                ),
                "upload failed: {:?}",
                upload_resp
            );

            // Fire the DispatchSpecialised request.
            let dispatch_resp = backend.executor.handle(
                executor_protocol::ExecutorRequest::DispatchSpecialised {
                    job_id: 1,
                    handle: TEST_HANDLE,
                    inputs: vec![in_buf],
                    outputs: vec![out_buf],
                },
            );
            assert!(
                matches!(
                    dispatch_resp,
                    executor_protocol::ExecutorResponse::DispatchDone { .. }
                ),
                "dispatch failed: {:?}",
                dispatch_resp
            );

            // Download the result.
            let download_resp = backend.executor.handle(
                executor_protocol::ExecutorRequest::DownloadBuffer {
                    buffer: out_buf,
                    offset: 0,
                    len: 16,
                },
            );
            let bytes = match download_resp {
                executor_protocol::ExecutorResponse::BufferData { data, .. } => data,
                other => panic!("expected BufferData, got {:?}", other),
            };
            let result: Vec<f32> = bytes
                .chunks(4)
                .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                .collect();
            // Expected: A × B = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
            //                 = [[19, 22], [43, 50]]
            assert_eq!(
                result,
                vec![19.0, 22.0, 43.0, 50.0],
                "Phase 4.10 CPU 2x2 MatMul: expected [[19, 22], [43, 50]]"
            );
        });
    }
}

