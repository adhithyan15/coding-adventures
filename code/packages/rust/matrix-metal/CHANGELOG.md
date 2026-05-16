# Changelog — matrix-metal

## 0.11.2 — 2026-05-13

### Compatibility update — `Op::Concat` keep-up

`matrix-ir` 0.3.0 added `Op::Concat` (wire tag 0x1D).  matrix-metal
does **not** claim Concat in its `supported_ops` bitset, so the
planner routes Concat ops to CPU — but the local `op_kind_name`
helper gained an arm so the exhaustive match still compiles.

Same pattern as the 0.11.1 keep-up for Op::Slice.

## 0.11.1 — 2026-05-13

### Compatibility update — `Op::Slice` keep-up

`matrix-ir` 0.2.0 added `Op::Slice` (wire tag 0x1C).  matrix-metal
does **not** claim Slice in its `supported_ops` bitset, so the
planner routes Slice ops to CPU — but the local `op_kind_name`
helper used in error messages gained an arm so the exhaustive
match still compiles.

Real Slice support on Metal will come when a workload actually
benefits from it (likely after DSP01 Phase 5 specialisation lands
and FFT pipelines pull image-domain ops onto the GPU).

## 0.11.0 — 2026-05-14

### Added — MX05 Phase 5 (kernel eviction)

- `SpecialisedTable::evict(handle) -> bool`
- `MetalExecutor::evict_specialised(handle) -> bool` — Apple-only,
  with non-Apple stub returning `false`.

When evicting, the boxed closure that owns the compiled
`MetalComputePipeline` is dropped; the Metal driver releases the
pipeline state object.

## 0.10.0 — 2026-05-13

### Added — MX05 Phase 4.10 (MatMul emitter with folded matrix)

The MSL emitter now supports `Op::MatMul (wire tag 0x15)` for f32
where the **RHS** matrix is observed as a stable constant.  The
constant matrix's elements are baked into the kernel source as
float literals; the kernel reads only the variable LHS matrix and
computes the dot product per output element.

#### Supported shapes

| Constant size | Matrix shape | Kernel size                                |
|---------------|--------------|--------------------------------------------|
| 16 bytes      | 2×2          | 2 column branches × 2-term dot product     |
| 64 bytes      | 4×4          | 4 column branches × 4-term dot product     |

Other sizes return `None` — V1 hard-caps at 16 elements because
the bake-in approach (each constant as a literal) only scales
to small matrices.  Larger matrices need a different
representation (e.g. uniform buffer of constants) which lands in
a later phase.

#### Constraints

- `dtype = F32`
- `folded_slot = Some(1)` (RHS folded).  LHS-folded MatMul returns
  `None` in V1 because the runtime variable dimension sits on a
  different axis and needs a separate kernel shape.

#### Entry-point convention

```
specialised_matmul_<dim>x<dim>_rhs_const_f32_0xHHHHHHHHHHHHHHHH
```

#### Kernel shape

The kernel fans out one thread per output element.  Grid uniform
`n = m * dim` (m = runtime rows, dim = constant matrix side).
Each thread:

  1. Decodes `r = gid / dim`, `c = gid % dim`
  2. Reads `a[r * dim + k]` for `k in 0..dim`
  3. Picks the right column's constants via `if (c == 0) { ... } if (c == 1) { ... }`
  4. Writes the dot product to `out[gid]`

Branch-per-column is fine for 2×2 / 4×4 — branch prediction is
trivial.  Larger matrices would want a branch-free formulation.

#### Tests (40 → 45 emitter; 19 → 45 total since Phase 4.2)

- `matmul_2x2_rhs_folded_emits_kernel` — 2×2 happy path
- `matmul_4x4_rhs_folded_emits_kernel` — 4×4 (still fits the cap)
- `matmul_unsupported_size_returns_none` — 3×3 and 5×5 rejected
- `matmul_lhs_folded_returns_none` — pins the Phase 4.x deferral
- `matmul_no_folded_slot_returns_none` — pins the "can't guess" guard

## 0.9.0 — 2026-05-13

### Added — MX05 Phase 4.7 (unary ops with folded input constant)

The MSL emitter now supports **unary f32 ops** when their single
input is itself observed as a stable constant `K`.  In that case
every output element equals `f(K)`, so the kernel collapses to a
**memset of the precomputed value** — zero input buffers, just
the output.

| `op_kind`     | `f(K)` baked in | Example                       |
|---------------|-----------------|-------------------------------|
| `0x00` Neg    | `-K`            | `K = 3.0`  → output `-3.0`    |
| `0x01` Abs    | `\|K\|`           | `K = -7.5` → output `7.5`     |
| `0x02` Sqrt   | `√K`            | `K = 16.0` → output `4.0`     |
| `0x03` Exp    | `e^K`           | `K = 0.0`  → output `1.0`     |
| `0x04` Log    | `ln K`          | `K = 1.0`  → output `0.0`     |
| `0x05` Tanh   | `tanh K`        | `K = 0.0`  → output `0.0`     |
| `0x06` Recip  | `1/K`           | `K = 4.0`  → output `0.25`    |

#### Kernel signature

The unary-folded-input kernel has **zero input buffers** — `K` is
baked into the source as a literal.  Binding order:

  - `out [[buffer(0)]]`
  - `n   [[buffer(1)]]`

(vs the binary-folded-constant kernel's `(a, out, n)` at slots
0/1/2.)

Entry points embed `_input_const_` to distinguish from the binary
`_const_` / `_lhs_const_` / `_rhs_const_` naming:

```
specialised_sqrt_input_const_f32_0xHHHHHHHHHHHHHHHH
```

#### Runtime install path

`MetalExecutor::install_specialised_from_emitted` now branches on
`emitted.input_buffer_count`:

  - `n_in == 0` → bind `(out, n)` to slots `(0, 1)` — memset kernel
  - `n_in == 1` → bind `(a, out, n)` to slots `(0, 1, 2)` — existing
    binary path

The branch is internal to the closure stored in `SpecialisedTable`;
no protocol or public API changes.

#### Tests

9 new emitter tests (31 → 40 in matrix-metal; the `lib` test count
went 35 → 40, +5 op-specific + +4 structural):

- `neg_f32_with_folded_input_emits_memset_kernel`
- `abs_f32_with_folded_input_emits_memset_kernel`
- `sqrt_f32_with_folded_input_emits_memset_kernel`
- `exp_f32_with_folded_input_emits_memset_kernel`
- `log_f32_with_folded_input_emits_memset_kernel`
- `tanh_f32_with_folded_input_emits_memset_kernel`
- `recip_f32_with_folded_input_emits_memset_kernel`
- `unary_input_const_entry_names_distinct_from_binary_const` —
  proves namespace separation
- `unary_ops_return_none_without_folded_slot` — pins the
  "no folded_slot → no kernel" guard

Plus `returns_none_for_unsupported_op_kind` was retargeted: Op::Neg
is now supported, so the test now uses `Op::ReduceSum` (0x0E) as
the "not yet" exemplar — reductions are future phase work.

## 0.8.0 — 2026-05-13

### Added — MX05 Phase 4.6 (Sub/Div/Pow with folded constant unlock)

The MSL emitter now supports the three remaining binary f32 ops with
a folded constant, picking the LHS- or RHS-folded variant based on
the new `SpecKey::folded_slot` field that matrix-profile v0.2 added.

| `op_kind`  | `folded_slot = Some(0)` (LHS) | `folded_slot = Some(1)` (RHS) |
|------------|-------------------------------|-------------------------------|
| `0x08` Sub | `K - a[gid]`                  | `a[gid] - K`                  |
| `0x0A` Div | `K / a[gid]`                  | `a[gid] / K`                  |
| `0x0D` Pow | `pow(K, a[gid])`              | `pow(a[gid], K)`              |

Entry-point names embed the variant: `specialised_sub_lhs_const_f32_0xH…H`
vs `specialised_sub_rhs_const_f32_0xH…H`, so two SpecKeys differing
only in `folded_slot` produce distinct compiled kernels (no
collision when both end up in an executor's `SpecialisedTable`).

#### New helper

`emit_binary_f32_with_const_at_slot(handle, op_name, lhs_template,
rhs_template, constant, folded_slot)` — mirror of Phase 4.5's
`emit_binary_f32_with_rhs_const` but takes both templates and
selects between them.

#### `folded_slot = None` for non-commutative ops

The emitter still returns `None` for Sub / Div / Pow if the policy
didn't tell us which slot was folded — we won't guess.  Test
`sub_div_pow_return_none_without_folded_slot` pins this.

#### Tests (12 new emitter tests; 19 → 31 total)

- `sub_f32_rhs_folded_emits_kernel`
- `sub_f32_lhs_folded_emits_kernel`
- `div_f32_rhs_folded_emits_kernel`
- `div_f32_lhs_folded_emits_kernel`
- `pow_f32_rhs_folded_emits_kernel`
- `pow_f32_lhs_folded_emits_kernel`
- `lhs_and_rhs_variants_have_distinct_entry_points`
- `sub_div_pow_return_none_without_folded_slot` (replaces the old
  Phase 4.5 deferral guard)
- 5 existing Phase 4.5 tests updated to set `folded_slot` (semantic
  no-op for commutative ops).

The existing 14 Phase 4.2 tests still pass — emitter output for
commutative ops is byte-identical.

### Updated `returns_none_for_unsupported_op_kind`

Phase 4.5 used `Op::Sub` (0x08) as the "unsupported" exemplar; now
that Sub is supported we pivot the test to `Op::Neg` (0x00), which
is unary and still outside the emitter's binary-with-constant
shape.

## 0.7.0 — 2026-05-12

### Added — MX05 Phase 4.5 (MSL emitter supports more binary ops)

Extends the `msl_emitter` module beyond Op::Add to cover the rest
of the **commutative** f32 binary ops the workspace uses.  Each
new shape follows the same `specialised_<op_name>_const_f32_0xH…H`
entry-point convention as Phase 4.2's Add kernel.

#### Newly-supported SpecKey shapes

| `op_kind`  | `dtype` | `range_class`        | MSL body                  |
|------------|---------|----------------------|---------------------------|
| `0x09` Mul | F32     | `Constant { 4 B }`   | `a[gid] * K`              |
| `0x0B` Max | F32     | `Constant { 4 B }`   | `max(a[gid], K)`          |
| `0x0C` Min | F32     | `Constant { 4 B }`   | `min(a[gid], K)`          |

(Op::Add 0x07 was already supported in v0.6.0.)

#### Refactor: shared `emit_binary_f32_with_rhs_const` helper

Replaced the dedicated `emit_add_f32_with_rhs_constant` with a
generic helper that takes an `op_name` and an `expr_template`.
The template substitutes `{a}` → `a[gid]` and `{k}` → the
formatted constant literal.  Adding a new commutative binary op
is now a one-line match-arm change in `emit_specialised_kernel`.

#### What's still not supported (Phase 4.6 territory)

`Op::Sub`, `Op::Div`, and `Op::Pow` are mathematically
non-commutative:

- `LHS - K`  ≠  `K - LHS`
- `LHS / K`  ≠  `K / LHS`
- `LHS^K`    ≠  `K^LHS`

Today's `SpecKey` doesn't encode which input slot the policy
folded — it just records the constant bytes.  Emitting one of the
two non-commutative variants risks wrong output if the policy
happened to pick the opposite slot from the emitter's assumption.

Phase 4.6 will extend `SpecKey` with a `folded_slot: u8` field
(or equivalent encoding) and unlock these.  Until then,
`emit_specialised_kernel` returns `None` for these op_kinds and
the runtime falls back to generic dispatch — `sub_div_pow_return_none_until_folded_slot_lands`
test pins this behaviour.

#### Tests

5 new emitter tests (14 → 19 total):

- `mul_f32_with_constant_emits_kernel`
- `max_f32_with_constant_emits_kernel`
- `min_f32_with_constant_emits_kernel`
- `distinct_ops_produce_distinct_entry_point_prefixes` — proves
  emitted entry-point names don't collide across op kinds with the
  same handle.
- `sub_div_pow_return_none_until_folded_slot_lands` — pins the
  Phase 4.6 deferral.

Plus the existing 14 Phase 4.2 tests still pass unchanged
(emitter output for `Op::Add` is byte-identical).

## 0.6.1 — 2026-05-12

### Added — non-Apple stubs for the Phase 4.2 install API

So downstream crates (image-gpu-core, matrix-runtime) can call
`install_specialised_from_emitted` and friends without `#[cfg]`-gating
every call site:

- `MetalExecutor::install_specialised(handle, kernel)` — no-op on
  non-Apple.
- `MetalExecutor::install_specialised_from_emitted(handle, emitted)`
  — returns `Err("unavailable on non-Apple targets")` on non-Apple.
- `MetalExecutor::specialised_count()` — returns `0` on non-Apple.

No behaviour change on Apple targets.

## 0.6.0 — 2026-05-12

### Added — MX05 Phase 4.2 (MSL emitter + specialised dispatch lands on Metal)

- **New `msl_emitter` module** (all platforms, including non-Apple CI).
  Pure code generator: given a [`SpecKey`] + handle, returns an
  [`EmittedKernel`] containing a self-contained MSL string with
  observed constants folded in as literal values, the entry-point
  name (`specialised_<op>_<variant>_<dtype>_0xHHHH…`), and the
  expected input/output buffer counts.  v0.6.0 minimum-viable scope:
  **F32 binary Add with a 4-byte RHS constant** (`op_kind=0x07`,
  `dtype=F32`, `range_class=Constant`).  All other `SpecKey` shapes
  return `None`.  The emitter is the centrepiece of Phase 4.2 and is
  the only piece that runs everywhere — compile and dispatch require
  a real Metal device.

- **New `specialised_table` module** (Apple-only).
  `HashMap<u64, Box<MetalSpecialisedKernelFn>>` keyed by handle, with
  `install` / `get` / `contains` / `len`.  Mirrors
  `matrix_cpu::SpecialisedTable` from Phase 4.1; differences:
  - Closure signature takes `&mut DispatchCtx` (not `&mut BufferStore`)
    so closures can encode through the same queue/buffers/pipelines
    as the generic dispatcher.
  - Closure trait bound is `Send` (not `Send + Sync`), because
    `MetalComputePipeline` wraps a raw Obj-C pointer and isn't
    `Sync`.  The `Mutex<State>` upstream still makes the executor
    `Sync` (since `Mutex<T>: Sync where T: Send`), so this is
    sound — see the rustdoc on `MetalSpecialisedKernelFn` for the
    full reasoning.

- **New APIs on `MetalExecutor`** (Apple-only):
  - `install_specialised(handle, kernel: Box<MetalSpecialisedKernelFn>)`
    — install a pre-built closure.
  - `install_specialised_from_emitted(handle, EmittedKernel) -> Result<(), String>`
    — compile MSL → look up entry → build pipeline → wrap in a
    dispatching closure → install.  The convenience layer over the
    emitter; this is the path the runtime will use once it
    auto-installs specialised kernels on cache hits.
  - `specialised_count() -> usize` — accessor for tests/metrics.

- **`ExecutorRequest::DispatchSpecialised` handler is now live** on
  Apple targets.  Handle hit → constructs a fresh `DispatchCtx`,
  invokes the closure, returns `DispatchDone { job_id, timings }`.
  Handle miss → `NOT_IMPLEMENTED`.  Closure error → `RUNTIME_ERROR`.
  Closure panic → `catch_unwind(AssertUnwindSafe(...))` → clean
  `RUNTIME_ERROR` (security hardening — same shape as the
  matrix-cpu Phase 4.1 fix).

### What this unlocks

Phase 4.1 closed the loop on the CPU side.  Phase 4.2 does the same
on Metal: specialised kernels emitted from a `SpecKey` get compiled
to a `MetalComputePipeline` keyed by handle, and `DispatchSpecialised`
routes invocations to them.  This is the moment GPU specialisation
goes from "the SpecCache tracks handles" to "the GPU actually runs
specialised kernels with folded constants".

Next phase: matrix-runtime auto-installation — observing a
`SpecRouter` cache hit and calling `install_specialised_from_emitted`
on the target executor without any user intervention.

### Security hardening

- **Panic-safe specialised dispatch.**  The closure invocation in
  the `DispatchSpecialised` handler is wrapped in
  `std::panic::catch_unwind(AssertUnwindSafe(...))`.  A kernel that
  panics (e.g. one that indexes attacker-supplied empty `inputs[0]`)
  surfaces as a clean `Error { code: RUNTIME_ERROR, message:
  "specialised kernel 0x… panicked: …", .. }` instead of unwinding
  through the mutex guard.  Same shape as the matrix-cpu Phase 4.1
  fix.  Regression test
  `dispatch_specialised_kernel_panic_becomes_runtime_error_not_unwind`
  installs a panicking kernel, fires a panic-inducing request,
  asserts the error response, AND fires a follow-up Heartbeat to
  prove the mutex isn't permanently poisoned.

- **Buffer-count validation in `install_specialised_from_emitted`**.
  The dispatching closure asserts `inputs.len() == n_in` and
  `outputs.len() == n_out` (captured from the `EmittedKernel`) before
  touching any raw pointers, so a wire request that arrives with the
  wrong number of buffers gets a clear `RUNTIME_ERROR` instead of
  reading past the end of a slice.

### New dependency

- **`matrix-profile`** (path-only, zero external deps) — needed
  because `emit_specialised_kernel` takes a `SpecKey`.

### Tests (16 new, all passing)

In `msl_emitter::tests` (14 — every test runs on every platform,
including non-Apple CI):

- `add_f32_with_constant_emits_kernel`
- `returns_none_for_unsupported_op_kind`
- `returns_none_for_unsupported_dtype`
- `returns_none_when_range_class_not_constant`
- `returns_none_when_constant_byte_length_wrong`
- `returns_none_when_constant_bytes_empty`
- `handle_appears_zero_padded_in_entry_point`
- `distinct_handles_produce_distinct_entry_points`
- `distinct_constants_produce_distinct_sources_same_handle`
- `emission_is_deterministic`
- `format_f32_literal_round_trips_normal_values`
- `format_f32_literal_handles_non_finite`
- `format_f32_literal_always_has_f_suffix_or_macro`
- `emitted_source_passes_structural_sanity`

In `specialised_table::tests` (5 — Apple-only):

- `install_then_lookup_finds_kernel`
- `lookup_of_missing_handle_returns_none`
- `install_overwrites_prior_kernel`
- `specialised_table_is_send`
- `debug_impl_shows_handles_not_pointers`

In `tests/integration.rs §7` (8 — Apple-only, run on macOS CI):

- `dispatch_specialised_returns_not_implemented_when_handle_unknown`
- `dispatch_specialised_runs_emitted_add_const_kernel` — the
  full end-to-end test: emit MSL with `7.5` folded in, install,
  upload `[1,2,3,4]`, dispatch, download, assert `[8.5, 9.5, 10.5, 11.5]`.
- `install_specialised_with_raw_closure`
- `dispatch_specialised_kernel_error_becomes_runtime_error`
- `dispatch_specialised_kernel_panic_becomes_runtime_error_not_unwind` (security)
- `install_specialised_overwrites_prior_kernel`
- `install_specialised_from_emitted_rejects_malformed_msl`
- `dispatch_specialised_wrong_buffer_count_errors_cleanly`

Total test count (Apple): 19 unit + 25 integration (was 17 + 17).
Non-Apple targets exercise the 14 emitter unit tests.

## 0.5.0 — 2026-05-05

### Added

- **`Op::ReduceSum / ReduceMax / ReduceMean` support** for **single-
  axis** F32 reductions.  Capability bitset now includes tags 0x0E,
  0x0F, 0x10.

  Three new MSL kernels (`reduce_sum_f32`, `reduce_max_f32`,
  `reduce_mean_f32`) share a `REDUCE_F32_BODY` macro that:

    1. Decomposes `gid` into an output multi-index using `out_dims`.
    2. Builds a template input multi-index, skipping/adjusting the
       reduced axis based on `keep_dims`.
    3. Sweeps `i = 0..reduce_size`, slotting `i` into the reduce-axis
       position and accumulating.
    4. Writes the result (sum: as-is; max: starting from `-INFINITY`;
       mean: divided by `reduce_size`).

  Supports `keep_dims = true` and `keep_dims = false`.  Up to rank 4
  (matching this backend's advertised `max_tensor_rank`).

  **Multi-axis reductions** (`axes.len() > 1`) return an Err at
  dispatch time with a clear message — the runtime can either surface
  the error or decompose into a chain of single-axis reductions.
  Decomposition is V2 work.

### Tests (4 new integration tests)

- `reduce_sum_axis1_on_gpu` — `[[1,2,3],[4,5,6]]` reduce-sum axis 1 → `[6, 15]`.
- `reduce_max_axis0_keep_dims_on_gpu` — `[[1,5,3],[4,2,6]]` reduce-max axis 0 with keep_dims → `[[4, 5, 6]]`.
- `reduce_mean_axis1_on_gpu` — `[[2,4,6,8],[1,3,5,7]]` reduce-mean axis 1 → `[5.0, 4.0]`.
- `reduce_multi_axis_returns_error` — verifies V1 multi-axis attempt fails cleanly with a "single-axis" error message so the runtime can fall back.

Total integration tests: 17 (was 13).

### Notes

- The kernels are thread-per-output-element with sequential reads
  along the reduce axis.  Performance is suboptimal for very long
  reduce axes (no tree reduction within a threadgroup); fine for
  the rank-2/3/4 reduce sizes typical in image / ML graphs (hundreds
  to thousands).  V2 polish: tile-and-tree reduction kernels.
- Reduction completes the V1 elementwise+reduction op set on Metal.
  Combined with shape ops (Reshape/Transpose/Broadcast in 0.1.1–
  0.3.0) and casts (0.4.0), matrix-metal now handles the bulk of
  ML-style F32 graph patterns end-to-end.

## 0.4.0 — 2026-05-05

### Added

- **`Op::Cast` support (F32 output paths only).**  Capability bitset
  now includes tag 0x1A.

  matrix-metal advertises `supported_dtypes = F32`, which constrains
  the planner's capability filter to route only Casts whose **output**
  dtype is F32 to us.  That leaves three input-dtype paths to handle:

    - `F32 → F32` (degenerate identity cast)
    - `U8 → F32` (widening conversion)
    - `I32 → F32` (widening conversion)

  Each is a one-line elementwise scalar cast.  MSL's implicit
  conversions match Rust's `as` semantics for these widening paths
  (no rounding mode ambiguity; every U8 and I32 value fits in F32
  exactly or with at most one rounding step).

  The other three Cast directions (`F32 → U8 / I32`, `U8 → I32`,
  `I32 → U8`) need `supported_dtypes` to advertise U8 / I32 — and
  that would also let the planner route U8/I32 elementwise ops to us
  which we don't yet implement.  Keeping the dtype bitset at F32 only
  means those casts stay on CPU; we ship the F32-output ones today.

### Tests (3 new integration tests)

- `cast_u8_to_f32_on_gpu` — `[0, 1, 200, 255]` (u8) → `[0.0, 1.0, 200.0, 255.0]` (f32).
- `cast_i32_to_f32_on_gpu` — `[0, 1, -1, 1_000_000, i32::MIN]` (i32) → matching f32 values; verifies that the largest-magnitude path still round-trips correctly (i32::MIN is exactly representable in f32 with no rounding).
- `cast_f32_to_f32_on_gpu_is_identity` — degenerate path; confirms PI and other arbitrary f32 values round-trip byte-exactly.

Total integration tests: 13 (was 10).

### Notes

- Defence in depth: `cast_dispatch` returns Err if the planner ever
  routes a non-F32-output cast to us (it shouldn't, given the
  `supported_dtypes` bitset, but the runtime check guards against
  planner bugs and future capability changes).
- This unblocks ML graphs that use U8 image inputs and I32 index
  buffers but produce F32 intermediates — e.g. `image-gpu-core`'s
  current pattern of u8-pixels → f32-cast → matmul → u8-pack stays
  on Metal for the **u8→f32** half (the f32→u8 pack still falls
  back to CPU).

## 0.3.0 — 2026-05-05

### Added

- **`Op::Broadcast` support.**  Completes the shape-op trio (Reshape
  in #2077, Transpose in #2122, Broadcast now).  General N-D
  axis-replication kernel up to rank 4 (matching this backend's
  advertised `max_tensor_rank`).  Capability bitset now includes
  tag 0x13.

  The MSL kernel walks the output linearly: for each output element,
  decomposes the linear index into an output multi-index using the
  target dims, then builds the input multi-index by clamping each
  size-1 input axis to index 0 and copying every non-broadcast axis
  through.  Re-flattens with the input dims and reads.  Memory
  access is **read-fan-in** — many output threads read the same
  input element when broadcasting along a hot axis, which Metal
  handles well via its texture cache on Apple Silicon.

  The args struct (rank, output numel, in_dims[4], out_dims[4]) is
  encoded as 40 bytes (rounded to 48 for MSL alignment) and passed
  via `set_bytes`.

  Edge cases:
    - Rank 0 (scalar) is a no-op single-element copy.
    - Rank > 4 returns an Err.
    - Empty output (numel = 0) returns Ok without dispatching.
    - Input rank ≠ output rank returns an Err.
    - Input dim ≠ 1 and ≠ output dim is enforced at the matrix-ir
      validator level; the kernel doesn't re-check.

### Tests (2 new)

- `broadcast_row_to_matrix_on_gpu` — `(1, 3) → (4, 3)` broadcast on axis 0.
- `broadcast_column_to_matrix_on_gpu` — `(3, 1) → (3, 4)` broadcast on axis 1.

Total: 10 integration tests (was 8).

### Notes

- All three V1 shape ops (Reshape, Transpose, Broadcast) now run on
  Metal.  The capability filter no longer routes any pure shape op
  to CPU, so future graphs that mix elementwise + matmul + arbitrary
  shape ops can stay end-to-end on GPU under uniform-Metal placement.
- ML-style "bias add" (`out = x + bias` where bias broadcasts across
  the batch axis) is now expressible end-to-end on Metal.

## 0.2.0 — 2026-05-05

### Added

- **`Op::Transpose` support.**  General N-D permutation kernel up to
  rank 4 (matching this backend's advertised `max_tensor_rank`).
  Capability bitset now includes tag 0x12.

  The MSL kernel walks the output linearly: for each output element,
  it decomposes the linear index into an output multi-index using
  the output dims, reverses the permutation to get the input
  multi-index, then re-flattens with the input dims.  Cost per
  element is O(rank) divides + O(rank) multiplies.  Memory access
  is non-coalesced for non-trivial permutations — that's the price
  of generality.  V2 could special-case the rank-2 matrix-transpose
  path with a tiled shared-memory kernel; V1 keeps the kernel small.

  The args struct (rank, output numel, perm[4], in_dims[4],
  out_dims[4]) is encoded as 56 bytes (rounded to 64 for MSL
  alignment) and passed via `set_bytes`.

  Edge cases:
    - Rank 0 (scalar) is a no-op memcpy.
    - Rank > 4 returns an Err (the planner shouldn't route those to
      us once it sees `max_tensor_rank: 4`, but the dispatch defends
      in depth).
    - Empty output (numel = 0) returns Ok without dispatching.

### Tests (2 new)

- `transpose_2x3_to_3x2_on_gpu` — rank-2 matrix transpose with `perm = [1, 0]`.
- `transpose_3d_perm_021_on_gpu` — rank-3 with `perm = [0, 2, 1]` (swaps the last two axes only); confirms the kernel's permutation logic generalises beyond the rank-2 case.

Total tests: 8 integration (was 6).

### Notes

- `Op::Broadcast` (tag 0x13) is still V2 work.  Broadcasting needs
  proper stride logic — strides become non-trivial when broadcasting
  across multiple axes, and the kernel needs to know which axes are
  size-1-broadcast vs ordinary.  Out of scope for this PR.

## 0.1.1 — 2026-05-04

### Added

- **`Op::Reshape` support.**  Reshape is metadata-only in SSA — same
  numel, different shape — so the implementation is a same-size memcpy
  from the input buffer to the output buffer (going through
  `BufferStore`'s host-side read/write, which on Apple Silicon's
  unified memory is essentially `memcpy`).  Capability bitset now
  advertises tag 0x11 alongside the elementwise ops, MatMul, and
  Const.  `Op::Transpose` (0x12) and `Op::Broadcast` (0x13) need real
  data movement / index expansion and remain V2 work.

  Why this matters: it lets `image-gpu-core`'s sepia /
  colour-matrix graphs (which always reshape `pixels` and the matrix
  before `MatMul`) qualify for uniform-Metal placement under MX04's
  pass 2b.  Without Reshape support those graphs would always have a
  capability hole that prevented uniform placement and forced a
  CPU-only re-plan in the consumer.

### Fixed

- **Dispatch no longer fails on a strict `executor != our_id` check.**
  V0.1's dispatch handler aborted if the placed op's `executor` field
  didn't match `MetalExecutor`'s `our_id`, but the runtime never
  actually called `MetalExecutor::set_our_id`, so `our_id` stayed at
  `u32::MAX` and every dispatch routed by a multi-executor runtime
  failed.  V1 single-transport-per-executor doesn't need the strict
  check anyway — if our `handle()` was called, the dispatch was for
  us — so the check is now just `executor != CPU_EXECUTOR`.  Real
  routing-correctness checking is V2 work that needs the runtime to
  push the assigned id into each executor at registration time.

### Tests

- New `reshape_preserves_bytes_on_gpu` integration test confirms
  Reshape round-trips a 6-element f32 vector into a 2×3 shape with
  byte-identical contents.

## 0.1.0 — 2026-05-04

Initial release.  First specialised executor for the matrix execution
layer.

### Added

- `MetalExecutor` — implements the executor-protocol contract on
  Apple Metal.  Mutex-guarded internal state with poison recovery.
- V1 op support: F32 elementwise unary (Neg, Abs, Sqrt, Exp, Log,
  Tanh, Recip), F32 elementwise binary (Add, Sub, Mul, Div, Max, Min,
  Pow), F32 MatMul (rank-2), Const.
- `BufferStore<MetalBuffer>` — bounds-checked HashMap keyed by
  `BufferId`.  Mirrors `matrix-cpu::BufferStore` API.
- MSL kernel library (`src/kernels.rs`) compiled once at executor
  startup.  Pipelines cached by entry-point name.
- `local_transport()` and `register()` helpers.
- `profile()` advertises capability bitset (16 ops × F32 only) and
  cost model defaults sized for Apple Silicon (5 TFLOPS f32, 50 GB/s
  unified memory, 5 µs launch overhead).
- Up-front graph validation (16 MiB per-tensor cap, byte_size overflow
  check) — same hardening as `matrix-cpu`.
- Non-Apple platforms compile a stub that always returns
  `DEVICE_LOST` from `handle()` and `Err` from `MetalExecutor::new()`.

### Tests

5 integration tests pass on real Metal hardware:
- `neg_f32_on_gpu` — elementwise unary
- `add_f32_on_gpu` — elementwise binary
- `matmul_2x2_on_gpu` — `[[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]`
- `local_transport_heartbeat` — protocol round-trip via LocalTransport
- `dispatch_rejects_oversized_tensor` — validation guard

### Constraints

- Zero external Cargo dependencies.  Only path deps to `matrix-ir`,
  `compute-ir`, `executor-protocol`, `matrix-runtime`, `metal-compute`.
- F32 only in V1 — every other dtype falls back to `matrix-cpu` via
  the planner's capability filter.
