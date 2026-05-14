# Changelog — image-gpu-core

## 0.14.0 — 2026-05-14

### Added — MX05 Phase 5 (deoptimisation when observed assumptions fail)

Phases 4.x committed to specialised kernels under the assumption
that the observed constant remains the same.  Phase 5 closes the
feedback loop: when an observation contradicts a kernel's folded
constant, the runtime evicts the stale kernel and falls back to
generic dispatch.

#### Mechanism

- `try_auto_install_specialised_with_origin(spec, Some((subhash, op_idx)))`
  records each Constant-folded install in `INSTALLED_DEOPT_TRACKING`
  with origin `(subhash, op_idx, slot)`.
- `scan_and_deoptimise()` runs at the end of every
  `drive_specialisation` call.  For each tracked handle, it
  re-reads the origin observation's tensor for the folded slot.
  If `observed_min != observed_max`, the constant has changed:
    - `SpecRouter::cache_invalidate(handle)` drops the cache entry
    - `CpuExecutor::evict_specialised(handle)` drops the closure
    - `MetalExecutor::evict_specialised(handle)` drops the
      compiled pipeline (Apple targets)
    - `INSTALLED_HANDLES`, `INSTALLED_KERNEL_METADATA`, and
      `INSTALLED_DEOPT_TRACKING` are all cleaned up.

#### New public counter

- `deoptimisation_count() -> usize` — total deopts in this process.
  Useful for monitoring kernel stability under shifting workloads.

#### Test (36 → 36, +1 new replacing previous slot)

`deoptimises_when_observed_constant_changes`:

1. 1100 invocations with K=42.0 → install fires.
2. 1 invocation with K=99.0 (same graph shape) → the Profiler's
   observation sees observed_min=42, observed_max=99, slot 1 is
   no longer constant.  The deopt scan evicts the K=42.0 kernel.
3. Assert `deoptimisation_count` rose.

Test count: 35 → 36.

## 0.13.0 — 2026-05-13

### Added — MX05 Phase 4.10 end-to-end (MatMul with folded matrix)

End-to-end exercise of the matrix-metal / matrix-cpu Phase 4.10
MatMul-with-folded-RHS-matrix kernels.  The CPU side runs on
every platform; the metal side activates on Apple targets when
the planner picks metal (which it sometimes does for MatMul
because matmul's flop count overcomes the transfer cost — the
first emitter shape where that's possible).

#### Test (34 → 35)

`cpu_matmul_folded_rhs_2x2_produces_correct_output`:

Direct exercise of the matrix-cpu closure builder for MatMul.
Installs a 2×2 specialised kernel with `B = [[5, 6], [7, 8]]`
folded, allocates input/output buffers via the protocol,
uploads `A = [[1, 2], [3, 4]]`, fires `DispatchSpecialised`,
and asserts the downloaded output is `[[19, 22], [43, 50]]`.

Runs on every platform — no Apple gate.  Exercises the full
`install_specialised → DispatchSpecialised → DownloadBuffer`
path through the executor's protocol surface.

## 0.12.0 — 2026-05-13

### Added — MX05 Phase 4.9 (matrix-cpu auto-install + dispatch routing)

Closes the gap that's been open since Phase 4.3: the auto-installer
now installs on the **CPU executor** too, not just metal.  Combined
with matrix-cpu v0.5.0's new `build_specialised_kernel`, this
finally exercises specialised dispatch on every platform — Linux
and Windows CI included, where metal is unavailable.

#### Per-thread `CpuBackend`

Promotes the CPU executor from a per-dispatch construction
(`matrix_cpu::local_transport()` per call) to a **thread-local**
singleton.  Persistence within a thread means a real workload's
1000+ repeat dispatches see the auto-installer's installs.
Per-thread isolation means cargo's parallel unit-test runner
doesn't cross-contaminate `BufferStore` state.

```rust
thread_local! {
    static CPU_BACKEND: CpuBackend = { /* Arc<CpuExecutor> + LocalTransport */ };
}
```

Callers use `with_cpu_backend(|b| ...)` to access the
thread-local instance.

#### `try_auto_install_specialised` dispatches on `backend_id`

The auto-installer now branches on `specialised.key.backend_id`:

  - `0` → CPU: `matrix_cpu::build_specialised_kernel` →
    `CpuExecutor::install_specialised`.  Always available.
  - `1` → metal: `matrix_metal::emit_specialised_kernel` →
    `MetalExecutor::install_specialised_from_emitted`.
    Apple-only.

Both paths share `INSTALLED_HANDLES` (handles are unique because
the SpecKey hash feeds on `backend_id` since Phase 4.6) and
`INSTALLED_KERNEL_METADATA`.

#### Updated `specialised_install_count`

Was Apple-only via `#[cfg(feature = "metal-backend")]` returning
the metal executor's count.  Now sums **both** backends' counts —
matrix-cpu's `specialised_count()` plus matrix-metal's (when
available).  Returns the total installs in this thread, across
backends.

#### Test (33 → 34)

`cpu_auto_installer_registers_kernel_after_threshold` — the CPU
counterpart of Phase 4.3's metal-only `auto_installer_registers_kernel_after_threshold`.

Drives an Add-with-constant graph 1100 times, asserts
`specialised_install_count` rises.  **Runs on every platform**
because the planner picks CPU for tiny graphs unconditionally
(no metal-vs-CPU cost-model gymnastics required).

#### Side-table cfg cleanup

`installed_handles`, `installed_kernel_metadata`,
`record_test_kernel_metadata`, `KernelMetadata`, and
`handle_is_installed` lost their `#[cfg(feature = "metal-backend")]`
gates — they're now always-on because the CPU install path also
exercises them.

## 0.11.0 — 2026-05-13

### Added — MX05 Phase 4.8 (multi-op specialised dispatch)

Lifts the Phase 4.4 restriction that limited specialised dispatch
routing to graphs with **exactly one** non-Const Compute op.  A
graph with multiple chained ops — e.g. `Add(x, 3) → Mul(_, 2)` —
where every op has an installed specialised kernel now routes
through `DispatchSpecialised` for every Compute op instead of
falling back to generic `Dispatch`.

#### Strategy

```
runtime ─Dispatch{prep_graph}─►        metal  (single dispatch:
                                              allocates all
                                              buffers, uploads all
                                              constants)

for each (op_idx, handle) in computes (ordered by op_idx):
    runtime ─DispatchSpecialised{handle, inputs, outputs}─► metal
                                                              ↓
                                                           DispatchDone

runtime ─DownloadBuffer─►              metal  (final output)
```

The prep graph is `placed.ops` minus the non-Const Compute ops we
intend to dispatch as specialised.  All `Op::Const`, `Alloc`,
`Free`, and `Transfer` ops stay, so matrix-metal's existing
handler does all buffer management under planner-assigned IDs.

#### New helpers

- `all_non_const_computes_with_handles(placed, installed_per_op)`
  — returns `Some(Vec<(usize, u64)>)` iff **every** non-Const
  Compute op has an installed handle.  `None` triggers fallback
  to the Phase 4.4 single-op gate, then to generic dispatch.
- `dispatch_specialised_via_multi(transport, placed, computes, ...)`
  — the multi-op equivalent of `dispatch_specialised_via`.
  Increments `SPECIALISED_DISPATCH_COUNT` by `computes.len()` on
  success.
- `build_specialised_inputs_outputs(placed, compute_op_idx, handle)`
  — refactored from `dispatch_specialised_via` so both single-op
  and multi-op paths share the IR-input trimming + folded-slot
  shuffle logic.

#### Routing precedence in `dispatch_via`

  1. Try multi-op route (when every Compute op is specialised).
  2. Try single-op route (Phase 4.4 path).
  3. Fall back to generic `Dispatch { graph }`.

#### Test (32 → 33)

`dispatch_multi_op_specialised_chain_produces_correct_output`
(Apple-only):

Builds the chain
`x = [1,2,3,4]` → `Add(x, 3) = y` → `Mul(y, 2) = z`
with `Add` and `Mul` each having their own
`*_const_f32` specialised kernel installed.  Drives through
`dispatch_specialised_via_multi` and asserts:

- `specialised_dispatch_count` rises by **at least** 2 (one per
  Compute op).  We use `>=` because cargo runs tests in parallel
  and other tests bump the same counter concurrently.
- Final output is `[8, 10, 12, 14]` — i.e. `(x + 3) * 2` computed
  via two chained DispatchSpecialised requests.

## 0.10.0 — 2026-05-13

### Added — MX05 Phase 4.7 end-to-end (unary memset path through DispatchSpecialised)

matrix-metal v0.9.0 added an emitter shape where a unary op's
input is folded into the kernel source — the kernel becomes a
memset of `f(K)` with zero input buffers.  This release verifies
the full chain end-to-end: image-gpu-core's `dispatch_specialised_via`
handles `n_in == 0` correctly (passes an empty `inputs: vec![]`
to `DispatchSpecialised`), and the metal executor's install
closure binds only the output buffer.

#### Test

`dispatch_specialised_via_routes_unary_folded_input` (Apple-only):

Builds `Op::Sqrt(input = [16, 16, 16, 16]) → C`, installs the
`sqrt_input_const_f32` specialised kernel (K=16), and asserts
the output is `[4.0, 4.0, 4.0, 4.0]` — `√16 = 4` written by the
memset kernel.

Test count: 31 → 32.

#### Dispatcher change

No source change needed — the Phase 4.6 trimming logic
`ir_inputs.iter().take(n_in)` naturally yields an empty Vec when
`n_in == 0`, and the slot-shuffle guard only fires when
`ir_inputs.len() == 2 && n_in == 1`.  The unary-memset path
"just works" through existing code.

## 0.9.0 — 2026-05-13

### Added — MX05 Phase 4.6 (dispatch routes the unfolded slot)

Phase 4.4 routed `Op::Add` through `DispatchSpecialised` by trimming
`ir_op.inputs()` to the first `n_in` IR inputs.  That worked for
commutative ops where slot doesn't matter, but would have produced
wrong output on `Op::Sub` if the policy folded the LHS (the
dispatcher would pass the LHS buffer — the constant! — and skip
the RHS variable input).

Phase 4.6 fixes this by consulting `SpecKey::folded_slot`:

- For a binary op with `folded_slot = Some(s)`, the dispatcher
  passes `ir_op.inputs()[1 - s]` — the **unfolded** slot.
- For commutative ops or `folded_slot = None`, falls back to the
  Phase 4.4 behaviour (first `n_in` inputs in declared order).

#### Internal changes

- `INSTALLED_KERNEL_METADATA` value changed from
  `(usize, usize)` to a named `KernelMetadata { n_in, n_out, folded_slot }`
  struct.  `try_auto_install_specialised` records
  `specialised.key.folded_slot` alongside the buffer counts.
- `dispatch_specialised_via` consults the slot when building the
  `inputs` list for the `DispatchSpecialised` request.
- `record_test_kernel_metadata(handle, n_in, n_out, folded_slot)`
  signature extended; existing call sites updated.

#### Test (31 → 32)

`dispatch_specialised_via_routes_lhs_folded_correctly` (Apple-only):

Builds an `Op::Sub(A = [10, 10, 10, 10], B = [1, 2, 3, 4]) → C`
graph, installs the `sub_lhs_const_f32` kernel (`folded_slot =
Some(0)`, constant `K = 10.0`), and asserts the output is
`[9, 8, 7, 6]` — i.e. `K - B = 10 - [1,2,3,4]`.  If the
dispatcher had passed A (the LHS, the constant) instead of B
(the RHS, the variable), the output would be `[0, 0, 0, 0]` and
the test would catch the regression.

## 0.8.0 — 2026-05-12

### Added — MX05 Phase 4.4 (dispatch-routing half of the loop)

Closes the **dispatch** half of the specialised-kernel routing loop.
When a placed graph has exactly one non-Const Compute op and that
op's specialised kernel is installed on the metal executor,
image-gpu-core now routes the dispatch through the
`ExecutorRequest::DispatchSpecialised` protocol message instead of
the generic `Dispatch { graph }` path.

```
runtime ─Dispatch{prep_graph}─►  metal-executor  (allocate buffers, upload constants)
runtime ─DispatchSpecialised{handle, inputs, outputs}─►  metal-executor.SpecialisedTable[handle]
                                                  ─►  installed_closure(buffers...)
                                                  ─►  DispatchDone
runtime ─DownloadBuffer─►  metal-executor  (returns output bytes)
```

#### What's new

- **`pub fn specialised_dispatch_count() -> usize`** — new public
  counter (process-wide, monotonic, atomic).  Distinct from
  `specialised_install_count` — that counts how many kernels are
  installed; this counts how many *invocations* went through
  `DispatchSpecialised`.  Always `0` on non-Apple builds (no Metal
  executor to dispatch on).
- **`drive_specialisation` now returns `HashMap<u32, u64>`** —
  `(op_index → installed_handle)` for every Compute op whose
  specialised kernel is currently installed on the metal executor.
  The dispatcher uses this map to decide whether to route through
  `dispatch_specialised_via` (when the map covers all non-Const
  Compute ops in the graph).
- **`single_non_const_compute_with_handle(placed, installed_per_op)`**
  — helper that returns `Some((op_index, handle))` iff the placed
  graph contains exactly one non-Const Compute op and that op has
  an installed specialised kernel.  Multi-op routing is later phase
  work — that's `None` for V0.8.0 and the dispatcher falls back to
  the generic path.
- **`dispatch_specialised_via(transport, placed, compute_op_idx,
  handle, output_residency, output_byte_count)`** — the routing
  function itself.  Strategy:
    1. Strip the single non-Const Compute op from `placed.ops`.
    2. Fire `Dispatch { prep_graph }` so matrix-metal's existing
       handler allocates buffers and uploads constants under the
       planner-assigned BufferIds.  This sidesteps the
       protocol-`AllocBuffer`-uses-server-IDs mismatch.
    3. Fire `DispatchSpecialised { handle, inputs, outputs }` where
       `inputs` is `ir_op.inputs()` trimmed to the installed
       kernel's `input_buffer_count` (so a kernel with a folded RHS
       constant only sees the LHS buffer, not both).
    4. Download the output via `DownloadBuffer`.
    5. Bump `SPECIALISED_DISPATCH_COUNT` on success.
- **`INSTALLED_KERNEL_METADATA: Mutex<HashMap<u64, (usize, usize)>>`**
  side-table — records `(input_buffer_count, output_buffer_count)`
  for each installed handle.  Populated by
  `try_auto_install_specialised`; read by `dispatch_specialised_via`
  when trimming `ir_op.inputs()` to the right count.

#### Test

`dispatch_specialised_via_produces_correct_output` (Apple-only):

Builds a manually-placed `Op::Add(A, B) → C` graph pinned to metal
where `A = [1, 2, 3, 4]` and `B = [7, 7, 7, 7]` (the constant to
fold).  Installs the Add+constant-7.0 kernel directly via
`MetalExecutor::install_specialised_from_emitted`, then calls
`dispatch_specialised_via` and asserts:

1. `specialised_dispatch_count` rose by exactly one.
2. The downloaded output is `[8.0, 9.0, 10.0, 11.0]` — same as the
   generic Add would produce.

Total test count: 29 → 30.

#### Why not a planner-driven end-to-end test?

The cost model in matrix-runtime currently prefers CPU over metal
for the `Op::Add(f32) → f32` shape regardless of `N`: the per-element
host→device transfer cost (`bytes / host_to_device_bw = 4N/50` ns)
exceeds the CPU per-element compute cost (`N/40` ns), so the planner
never picks metal for a graph that does Add of two constants no
matter how large.  A real planner-picks-metal scenario needs either
(a) a heavier op like `Op::MatMul` (Phase 4.5 emitter work),
(b) more realistic profile numbers (Apple Silicon's unified memory
is ~200 GB/s effective bandwidth, not the conservative 50 GB/s
we currently advertise), or (c) constants persistent across
invocations (a future protocol extension).

V0.8.0 ships the protocol-level routing in isolation; the
planner-side decision logic is covered by existing tests in
matrix-runtime/src/planner.rs.  The two compose naturally once a
heavier emitter shape lands.

#### What this still doesn't do

- **Multi-op specialised graphs** — V0.8.0 only handles
  single-Compute-op graphs.  A graph with `Add(x, y) → Mul(_, z) → out`
  falls back to generic dispatch even when both Add and Mul have
  installed kernels.  Multi-op routing requires interleaving
  per-op DispatchSpecialised calls with intermediate buffer
  management, which is the next phase's scope.
- **matrix-cpu auto-install + dispatch routing** — Phase 4.3
  noted that matrix-cpu's `CpuSpecialiser` emits opaque handles
  without closure sources, so there's nothing for image-gpu-core
  to auto-install on CPU.  Same constraint here for dispatch
  routing: the CPU path always goes generic.  Future phase work.

## 0.7.0 — 2026-05-12

### Added — MX05 Phase 4.3 (runtime-side auto-installer; **install** half of the loop)

Closes the install half of the spec-routing loop opened by matrix-metal
Phase 4.2.  When `SpecRouter::route` returns a `SpecialisedKernel`,
image-gpu-core now auto-compiles the kernel and installs it onto the
metal executor — proving the chain:

```
sampler → policy → router → cache → msl_emitter → MetalExecutor::install_specialised_from_emitted
```

Phase 4.4 will land the **dispatch** half: replacing the generic
`Dispatch { graph }` request with per-op `DispatchSpecialised` requests
that actually invoke the installed kernels.  Split into a separate PR
to keep this one reviewable.

#### What's new

- **`MetalBackend`** now holds an `Arc<matrix_metal::MetalExecutor>`
  alongside the `LocalTransport`.  Previous code only kept the
  transport (boxed `Fn`); the executor reference is needed so
  image-gpu-core can call `install_specialised_from_emitted` directly.
- **`try_auto_install_specialised(&SpecialisedKernel)`** — new
  internal function.  When the router emits a specialised kernel,
  this:
    1. Checks a process-wide `INSTALLED_HANDLES: Mutex<HashSet<u64>>`
       for idempotency.
    2. Calls `matrix_metal::emit_specialised_kernel` to convert the
       `SpecKey` + handle into an `EmittedKernel`.
    3. Calls `MetalExecutor::install_specialised_from_emitted` to
       compile the MSL and register the dispatching closure under
       the handle.
    4. Records the handle as installed (so repeat hits in the
       `SpecRouter` cache don't pay MSL compilation cost more than
       once).
  Returns `false` on any short-circuit (already installed, emitter
  doesn't support the SpecKey shape, compile failure, no metal
  backend).  Compile failures don't mark the handle as installed so
  a future emitter fix can retry.
- **`drive_specialisation`** now consumes the router's return value
  and feeds it to `try_auto_install_specialised`.  Previously
  `r.route(...)` was called for its side effect on the cache and the
  return discarded.
- **`pub fn specialised_install_count() -> usize`** — new public
  hook.  Distinct from `spec_cache_len`: the cache tracks emitted
  handles; this counter tracks how many of those handles have
  actually been compiled and registered with an executor.  Always
  `0` on non-Apple builds (the matrix-cpu auto-install path will
  land in a later phase — `CpuSpecialiser` emits opaque handles
  today, not closure sources).

#### Integration test

`auto_installer_registers_kernel_after_threshold` (Apple-only):

- Builds a 4-element f32 Add-with-constant graph using
  `matrix_ir::GraphBuilder` directly.
- Runs it 1100 times — enough for `DefaultPolicy`'s 1000-invocation
  threshold to fire on the constant input.
- Asserts that `specialised_install_count()` rises above zero.

This is the first test in image-gpu-core where a kernel actually
gets compiled and installed onto an executor through the entire
specialisation pipeline.  Test count: 28 → 29.

#### What this still doesn't do (Phase 4.4 territory)

- The next dispatch of the same graph still goes through generic
  `Dispatch { graph }` — the installed specialised kernel is
  registered but not yet invoked.
- matrix-cpu auto-install — `CpuSpecialiser` produces handles
  without closure sources, so there's nothing to install.  Phase
  4.5 will either extend `CpuSpecialiser` to emit closure-producing
  metadata or land a separate cpu-emit pipeline.

#### Stub additions in matrix-metal

So image-gpu-core compiles cleanly on non-Apple CI:

- `MetalExecutor::install_specialised(handle, kernel)` — no-op
  stub on non-Apple.
- `MetalExecutor::install_specialised_from_emitted(handle, emitted)`
  — returns `Err("unavailable on non-Apple targets")` on non-Apple.
- `MetalExecutor::specialised_count()` — always `0` on non-Apple.

## 0.6.0 — 2026-05-05

### Added — MX05 Phase 4.2 (tensor-byte sampling)

- `drive_specialisation` now samples bytes from every constant
  input the graph carries via `Profiler::sample_tensor`.  Algorithm:
    1. Walk `placed.ops` to build a `TensorId → &PlacedConstant` map
       for tensors that are `Op::Const` outputs.
    2. For each non-Const Compute op, walk its inputs.  If an input
       traces back to a constant, sample its bytes against the
       consuming op's input slot.
- This populates `ProfileObservation::tensor_observations` (which
  was always empty under the previous wiring), giving
  `DefaultPolicy` real constant-input observations to act on.

### Changed

- The custom `HotPolicy` workaround is gone.  `spec_router()` now
  installs `DefaultPolicy::new()` — spec MX05's 1000-invocation
  threshold + 95% stability — which finally fires on actual
  constant-input observations rather than just hotness.
- `HOTNESS_THRESHOLD` const removed (was V4-only crutch).

### Tests

- `default_policy_populates_cache_via_constant_input_sampling` —
  drives gpu_invert 1100 times and asserts `spec_cache_len()` rises
  under the production-realistic threshold.  Replaces the V0.5
  `cpu_specialiser_populates_cache_after_hotness_threshold` test.
- `drive_specialisation_populates_tensor_observations` — confirms
  that a single dispatch records at least one TensorObservation
  with a populated min/max range.  Was 0 before this PR.

Total tests: 28 unit + 1 doc = 29 (was 27 + 1 = 28).

### Cost

- Per-dispatch overhead: O(constants × bytes-per-constant).  The
  16 MiB per-tensor cap inherited from matrix-runtime keeps this
  bounded at a few MB of byte-scanning per dispatch.  In practice
  most image-filter graphs have a handful of small constants
  (3×3 colour matrix, scalar bias), so the work is tiny.

### Regression check

`instagram-filters` routing on macOS unchanged after this PR:

```
  invert        → cpu     (small graph, planner picks CPU)
  greyscale     → metal   (sRGB + matmul, ships to GPU)
  sepia         → metal   (matmul-heavy, ships to GPU)
```

The dispatch path still doesn't consume the specialised kernel
handle (Phase 4.1 + executor-protocol extension required), so
output bytes and `last_executor()` are identical.  What changed:
`profiler_observations()`'s `tensor_observations` vector now
contains real entries, and `spec_cache_len()` rises above 0 once
1000+ invocations of any one graph have happened.

## 0.5.0 — 2026-05-05

### Added — MX05 Phase 4 visibility (CpuSpecialiser wired)

- The process-wide `SpecRouter` now uses **`matrix_cpu::specialiser()`**
  instead of `NoopSpecialiser`, hooking up the first real backend
  `Specialiser` (landed in `matrix-cpu` v0.3.0).
- A small custom `HotPolicy` replaces `DefaultPolicy` while the
  per-tensor sampling pipeline matures: it fires the Specialiser
  on raw invocation count alone (threshold 100), without requiring
  the constant-input or narrow-range observations that
  `DefaultPolicy` checks.  This is enough to demonstrate the cache
  rising above zero in CLI demos and tests.
- `HOTNESS_THRESHOLD` is `100` — much lower than spec MX05's 1000
  default — because Phase 4's specialisation is still
  observation-only (the dispatch path doesn't yet consume the
  kernel handle; that's Phase 4.1 + an executor-protocol extension).
  The threshold will return to 1000 once specialised dispatch
  actually saves cycles.

### Tests

- New `cpu_specialiser_populates_cache_after_hotness_threshold` test
  drives `gpu_invert` 150 times and asserts that `spec_cache_len()`
  rises.  This is the first place in `image-gpu-core`'s test suite
  where the SpecCache is observably non-empty after a real dispatch.
- The earlier `dispatch_drives_spec_router_pipeline` test no longer
  asserts `cache_len == 0` (which used to be the NoopSpecialiser
  invariant) — it only checks that invocation counters climb.

Total tests: 27 unit + 1 doc = 28 (was 26 + 1 = 27).

### Regression check

`instagram-filters` routing on macOS unchanged — the dispatch path
itself doesn't consume the specialised kernel handle yet, so output
bytes and the `last_executor()` value are identical to V0.4.0:

```
  invert        → cpu     (small graph, planner picks CPU)
  greyscale     → metal   (sRGB + matmul, ships to GPU)
  sepia         → metal   (matmul-heavy, ships to GPU)
```

The only new observable is `spec_cache_len()` — call it after a few
hundred filter invocations and watch the number rise.

## 0.4.0 — 2026-05-05

### Added — MX05 Phase 3 V4 wiring

- Each call to `pipeline::run_graph_with_constant_inputs` now drives
  the MX05 specialisation pipeline end-to-end:
    1. `Profiler::record_dispatch` bumps per-(graph, op) invocation
       counters.
    2. `SpecRouter::route` is consulted for every Compute op, with the
       op's wire tag, output dtype, and target executor id.
    3. The router's return is **discarded in V1** — `NoopSpecialiser`
       declines every key.  The wiring is foundation for Phase 4
       when a real specialiser arrives.
- New public observation hooks:
    - `image_gpu_core::profiler_observations()` — snapshot the
      accumulated `ProfileObservation` set.  Useful for telemetry,
      tests, and future phase 4 caller-side logic.
    - `image_gpu_core::spec_cache_len()` — how many specialised
      kernels are cached process-wide.  Always `0` while
      `NoopSpecialiser` is installed.
- Per-process `Profiler` and `SpecRouter` singletons via `OnceLock`
  so the routing pipeline is set up once and amortised across all
  filter invocations.

### Tests (1 new)

- `dispatch_drives_spec_router_pipeline` — gpu_invert produces a
  visible bump in `profiler_observations`'s aggregate invocation
  count.  Cache stays empty (NoopSpecialiser).

### Notes

- No behavioural change in the dispatch path itself.  Routing,
  output bytes, and the `last_executor()` value are unchanged.
- Phase 4 will install a backend-specific specialiser (e.g. an
  MSL emitter that constant-folds bias values for an LLM bias-add
  pattern).  When that lands, `spec_cache_len()` will start rising
  and `route()` will start returning `Some(SpecialisedKernel)`s
  that the dispatch path will consume — once `executor-protocol`
  grows a way for backends to dispatch via a SpecKey-keyed kernel
  handle.

## 0.3.0 — 2026-05-04

### Added

- **Optional `matrix-metal` backend** behind a default-on `metal-backend`
  feature.  With the feature enabled, `image-gpu-core` registers both
  `matrix-cpu` and `matrix-metal` in the runtime and lets the planner
  pick per graph based on its cost model.  On non-Apple platforms the
  feature is a no-op (matrix-metal's `local_transport()` returns Err
  and we transparently fall back to CPU-only dispatch).
- `pipeline::last_executor()` (re-exported as `image_gpu_core::last_executor`)
  reports the executor that handled the most recent dispatch on this
  thread (`"cpu"`, `"metal"`, or `None`).  CLI demos use this to surface
  which backend ran without changing the public per-op signatures.

### Changed

- `pipeline::run_graph_with_constant_inputs` now plans against the
  full multi-executor registry when `metal-backend` is enabled, and
  inspects the resulting `ComputeGraph` for single-executor placement
  before dispatching.  See `pipeline.rs` for the V1 single-executor
  dispatch design notes.

### Limitations

- V1 only supports **single-executor placements**: if the planner
  splits a graph across CPU and Metal (with `Transfer` ops between),
  we re-plan on a CPU-only registry and run on CPU.  The matrix
  execution layer's runtime crate doesn't yet ship a multi-executor
  coordinator that can drive cross-executor dispatch end-to-end —
  that's V2 work.  In practice the image-filter graphs in this crate
  are short single-op chains that the planner places homogeneously,
  so the mixed-placement fallback rarely triggers.

## 0.2.0 — 2026-05-04

Major migration: backend swapped from `gpu-runtime` (per-backend hand-written
shaders for Metal / CUDA / CPU) to the **matrix execution layer**
(`matrix-ir` → `matrix-runtime` planner → `matrix-cpu` executor).
**Public API of v0.1 is preserved** — all five existing functions accept
and return the same `PixelContainer`s.

### Migration details

- Each operation now builds a `matrix_ir::Graph` describing its
  computation, runs it through the matrix-execution-layer planner, and
  dispatches via `matrix_cpu::local_transport()`.
- sRGB ↔ linear conversion stays in Rust (the piecewise transfer
  function is awkward to express in MatrixIR's V1 op set; could be
  added in V2 via `Where(Less(...), ...)`).
- v0.1's per-op shader bundles (MSL + CUDA C + Rust fallback) are gone.
- v0.1's dependency on `gpu-runtime`, `metal-compute`, `cuda-compute`
  is removed.  New deps: `matrix-ir`, `compute-ir`, `matrix-runtime`,
  `matrix-cpu`, `executor-protocol`.

### Added

- `gpu_sepia` — classic Microsoft sepia tone (3×3 colour matrix).
- `gpu_contrast(scale)` — adjust contrast around mid-grey 128.
- `gpu_posterize(levels)` — reduce to N distinct values per channel.

These three new ops complete the filter set needed for the upcoming
Instagram-style filter CLI.

### Changed

- `GpuError` simplified.  v0.1 had several variants tied to specific
  GPU backend errors; v0.2 has just `Other(String)` since the matrix
  execution layer's failure surface is much smaller.

### Bug fixes (in matrix-cpu, included in this PR)

- `Op::Const` handler in `matrix-cpu` was a stub that didn't actually
  materialise the constant's bytes into the output tensor's buffer.
  All graphs that used `GraphBuilder::constant()` produced zero-filled
  results.  Now Const correctly copies bytes from
  `graph.constants[i].bytes` into the op's output buffer.

### Tests

20 unit tests + 1 doctest pass.  Numerical results match v0.1 within
±1 LSB for tests that allow tolerance; tests that asserted exact
byte equality (`invert_rgb`, `invert_preserves_alpha`,
`invert_double_is_identity`) still pass exactly.

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `gpu_invert` — invert RGB channels; alpha unchanged.  Direct sRGB u8
  operation (no colorspace conversion needed).
- `gpu_colour_matrix` — apply a 3×3 colour matrix in linear light.  Uniforms:
  9 × `f32` in row-major order (36 bytes).
- `gpu_greyscale` — convert to greyscale using specified `LuminanceWeights`
  (Rec.709, BT.601, or Average).  Uniforms: 3 × `f32` (12 bytes).
- `gpu_gamma` — power-law gamma in linear light.  Uniforms: 1 × `f32` (4 bytes).
- `gpu_brightness` — additive brightness shift in sRGB u8, clamped to
  \[0, 255\].  Uniforms: 1 × `i32` (4 bytes).
- `LuminanceWeights` — enum for greyscale luminance weight sets.
- MSL compute shaders: `shaders/metal/{invert,colour_matrix,greyscale,gamma,brightness}.metal`
- CUDA C kernels: `shaders/cuda/{invert,colour_matrix,greyscale,gamma,brightness}.cu`
- CPU fallback Rust functions (CPU path, identical logic to GPU shaders).
- Thread dispatch model: one GPU thread per RGBA pixel via
  `Runtime::run_pixels()`.
- sRGB encode/decode implemented identically in Rust, MSL, and CUDA C to
  within ±1 LSB rounding.
- Feature flag `metal` (default: on): propagates to `gpu-runtime/metal`.
- Unit tests use `Runtime::cpu_only()` — no GPU required; pass on any
  platform.  GPU tests can be run with `-- --ignored` on a real GPU machine.
- 16 unit tests covering all operations: edge cases (clamping, identity,
  double-invert), colorspace round-trips, uniform-encoding correctness.
