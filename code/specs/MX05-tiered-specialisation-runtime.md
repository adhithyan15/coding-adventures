# MX05 — Tiered Specialisation Runtime

## Status

Draft.  V1 spec.  Sits above MX01–MX04 — does not require any changes
to the IR, planner, protocol, or executor surface.  Read those first
([MX00](MX00-matrix-execution-overview.md) → MX04) for context.

## Why this layer exists

MX01–MX04 ship a tensor IR, a cost-model planner, a wire protocol, and
the first executors (`matrix-cpu`, `matrix-metal`).  Every dispatch
pays the cost of a *generic* kernel — one that handles the declared
dtype/shape but doesn't know anything about the actual data flowing
through.

Real workloads have structure.  An image-processing pipeline that
runs gamma correction on every frame sees the same dtype, the same
shape, often the same input range.  An LLM inference loop runs the
same matmul a thousand times before the user even reads the first
token.  A scientific simulation sweeps a parameter through hundreds
of fixed-shape iterations.

In every case, **after the first hundred or so dispatches the runtime
knows enough about what's flowing through to compile a much tighter
kernel**.  That's exactly how production JIT compilers work — V8,
HotSpot JVM, LuaJIT, Julia's tracing JIT, JAX's compile-then-cache.
The pattern is:

```
First call    →   Tier 0:  generic kernel for declared dtype
Calls 2..N    →   Tier 0 + sample inputs (1% rate, tiny overhead)
After N hits  →   Spawn specialisation job in a background worker
Spec ready    →   Tier 1:  specialised kernel, faster
```

What MX05 spec'd here adds to the matrix execution layer is the same
pattern:  a profile-guided specialisation runtime that observes
dispatches, identifies hot subgraphs, generates tighter kernels
asynchronously, and routes future dispatches through the cache.

The user never opts in.  They just observe their workload getting
faster after warm-up.

## Why this is cleaner here than in scalar JITs

Two design choices made in MX01–MX04 pay off here:

1. **The IR is data, not code.**  A subgraph hash uniquely identifies
   "what computation, structurally."  V8 and HotSpot have to work
   harder — they hash program counters, instruction sequences,
   polymorphic call sites.  We just hash a `compute_ir::ComputeGraph`
   slice (we already have a deterministic wire format from MX02).

2. **Each backend specialises independently.**  `matrix-metal` and
   `matrix-cuda` (future) each have their own kernel-generation
   conventions and their own caches.  They don't have to agree on
   what "specialised" means; the runtime hands each backend its own
   subgraphs.

## Reading order

To understand MX05 in full, read:

1. **MX00** — narrow-waist architecture
2. **MX01–MX04** — IR, planner, protocol, runtime (the V1 layer)
3. **This document** — the specialisation runtime that sits above

This spec assumes the reader is comfortable with the MatrixIR /
ComputeIR distinction and the planner / executor split.

## Architecture

Five new components, none of which require IR or protocol changes:

```
┌───────────────────────────────────────────────────────────────────┐
│ matrix-runtime                                                     │
│  ┌──────────────┐   ┌──────────────────┐   ┌──────────────────┐  │
│  │  Planner     │ → │  Profile sampler │ → │  Specialisation  │  │
│  │  (existing)  │   │  (NEW)           │   │  trigger (NEW)   │  │
│  └──────────────┘   └──────────────────┘   └──────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
                                  ↓
┌───────────────────────────────────────────────────────────────────┐
│ matrix-metal (or any executor)                                     │
│  ┌──────────────────┐  ┌────────────────────────────────────────┐ │
│  │ Generic kernels  │  │ Specialised kernel cache (NEW)         │ │
│  │ pipelines        │  │  HashMap<SpecKey, Pipeline>            │ │
│  │ (compile once    │  │  populated by background compile job   │ │
│  │  at startup)     │  │  evicted via LRU                       │ │
│  └──────────────────┘  └────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────┘
```

### 1. Profile sampler (matrix-profile crate)

> **Implementation status (Phase 1 + Phase 2a landed)**: shipped as
> the `matrix_runtime::profile` module rather than a separate
> `matrix-profile` crate.  Phase 1 added per-op invocation counters
> and the `Profiler` / `ProfileObservation` / `TensorObservation`
> data types.  Phase 2a (this update) adds `Profiler::sample_tensor`,
> `Profiler::tensor_observation`, `Profiler::should_sample`, and
> `Profiler::set_sample_rate` — the data plumbing for range
> observation.  No specialisation policy yet; that lands in Phase 2b
> (auto-narrow Cast insertion) and Phase 3 (SpecKey + Specialiser
> trait + cache).  Sampling is deterministic via a modulo counter
> rather than a PRNG so tests stay reproducible.

A new crate, `matrix-profile`, that owns the observation logic:

- Per-dispatch counters keyed by `(graph_subhash, op_index)`.
- With probability `p` (default 1%), sample input min/max/distribution
  into a small running statistic (count, sum, sum-of-squares, min,
  max — bounded to ~64 bytes per tensor regardless of size).
- Tiny overhead.  Bounded memory.  Counters can be reset.

The sampler produces a snapshot per `(graph_subhash, op_index)` of
the form:

```rust
pub struct ProfileObservation {
    pub graph_subhash: u64,
    pub op_index: u32,
    pub invocation_count: u64,
    pub input_observations: Vec<TensorObservation>,
    pub output_observations: Vec<TensorObservation>,
    pub mean_dispatch_ns: u64,
}

pub struct TensorObservation {
    pub declared_dtype: DType,
    pub shape: Shape,
    pub observed_min: f64,        // narrowing toward integer/narrow-float
    pub observed_max: f64,
    pub observed_zeros: u64,      // sparsity hint
    pub samples: u64,
}
```

### 2. Specialisation trigger

A small policy module that consumes profile observations and decides
when to specialise:

```rust
pub trait SpecialisationPolicy {
    fn should_specialise(&self, obs: &ProfileObservation) -> Option<SpecKey>;
}
```

V1 default policy:

- Trigger when `invocation_count >= 1_000` AND
- One of:
  - **Dtype narrowing**: declared dtype is wider than smallest dtype
    that contains observed range (e.g. F32 declared, range fits
    in F16 or even I8)
  - **Shape stability**: same shape seen for ≥ 95% of invocations
  - **Constant input**: an input's `observed_min == observed_max` for
    ≥ 95% of invocations (fold into kernel)

Policy is pluggable.  Frameworks targeting specific workloads (LLM
inference, image processing, scientific simulation) can supply their
own policies that bias toward their workload's specialisation
opportunities.

### 3. SpecKey

The unique identifier for a specialised kernel:

```rust
pub struct SpecKey {
    pub op_kind: u8,              // matrix_ir::Op wire tag
    pub dtype: DType,             // narrowed from declared if applicable
    pub shape_class: ShapeClass,  // see below
    pub range_class: RangeClass,  // see below
    pub backend_id: u32,          // executor-specific (e.g. metal vs cuda)
}

pub enum ShapeClass {
    Static(Shape),                // exact shape known
    Bucketed(ShapeBucket),        // small/medium/large
    Dynamic,                      // no specialisation on shape
}

pub enum RangeClass {
    InRange { min: f64, max: f64 },
    Constant(f64),                // single observed value
    NonNegative,
    Sparse(f32),                  // % zeros
    Unknown,
}
```

`SpecKey` is `Hash + Eq`, suitable for `HashMap` lookup.  Backends
construct keys per their own specialisation conventions; the runtime
is agnostic to the key's semantic meaning.

### 4. Specialised kernel generator (per-backend)

Each executor implements:

```rust
pub trait Specialiser {
    fn can_specialise(&self, key: &SpecKey) -> bool;
    fn compile_specialised(&self, key: &SpecKey, source_hint: &Op)
        -> Result<SpecialisedPipeline, SpecError>;
}
```

For `matrix-metal`, this means generating MSL with the observed
parameters baked in:

- Narrower dtype (`#define DTYPE half` instead of `float`)
- Exact shape unrolled (`uint M = 1024;`)
- Constant inputs folded directly into kernel source

The compile happens in a background worker thread so live dispatches
aren't blocked.  Once ready, the specialised pipeline is inserted
into the cache.

### 5. Specialised cache + dispatch routing

Each executor extends its existing pipeline cache:

```rust
pub struct ExecutorState {
    // existing
    generic_pipelines: HashMap<String, Pipeline>,

    // NEW
    spec_pipelines: LruCache<SpecKey, SpecialisedPipeline>,
    spec_inflight: HashSet<SpecKey>,  // jobs currently being compiled
}
```

On each dispatch:

1. Compute `SpecKey` for the op.
2. Cache hit → use specialised pipeline.
3. Cache miss + `spec_inflight.contains(&key)` → use generic, the
   spec is being compiled, will hit on a future call.
4. Cache miss + not in flight → use generic; if the trigger says
   "specialise this", enqueue a compile job and add to `spec_inflight`.

LRU eviction caps memory.  Default cap: 256 specialised pipelines per
executor (~10–100 MiB).

## Specialisation dimensions worth tracking

Not just dtype-narrowing.  V8 and HotSpot demonstrate that specialising
on **anything observable** is fair game if the hit rate justifies it:

| Dimension | Win | Threshold |
|---|---|---|
| **Dtype range**: declared F32, observed values fit in F16/I8 | Memory bandwidth + compute | 95% range coverage |
| **Static shape**: same shape seen repeatedly | Compiler unrolls, register pressure improves | 95% shape coverage |
| **Stride pattern**: contiguous vs strided | Drops stride math | 100% over hot ops |
| **Constant input**: input that's always the same | Fold into kernel; potentially fold subsequent ops | 95% value coverage |
| **Sparsity**: input mostly zero | Sparse kernel | >70% zeros |
| **Aliased outputs**: same buffer reused | In-place fused kernel | Repeated pattern |

V1 implementation can start with dtype + shape + constant.  Sparsity
and aliasing are V2 polish.

## Configuration

Default config exposed via the runtime API:

```rust
pub struct SpecConfig {
    pub specialise_threshold: u64,        // default: 1_000 invocations
    pub sample_rate: f32,                 // default: 0.01 (1%)
    pub max_spec_cache_per_executor: usize, // default: 256
    pub background_compile_threads: usize,  // default: 1
    pub deopt_on_distribution_shift: bool, // default: false (V1 = stable)
}
```

Can be set per-runtime; defaults work for most workloads.  Programs
running short workloads (e.g. CLI tools that dispatch a few times and
exit) can `disable_specialisation()` to avoid the profiling overhead
entirely.

## Threshold rationale

Production JITs all use thresholds in the 1k–10k invocation range.
The reason is the cost-benefit calculation:

- Compilation cost: 5–50 ms for an MSL kernel; similar for PTX
- Per-dispatch savings from specialisation: typically 5–30%
- Per-dispatch baseline: 10 µs–10 ms depending on workload

For a 100 µs op with 10% specialisation savings (10 µs/call), break-even
is **5,000 invocations** just on compilation.  For a 1 ms op with 30%
savings (300 µs/call), break-even is **17 invocations** — but you also
need to be sure the workload is stable, which is where the count
threshold protects us from speculatively specialising one-shot
workloads.

V1 default `1_000` invocations is conservative enough to skip CLI
tools and short scripts, aggressive enough to catch the second epoch
of an LLM inference run.

## Phased delivery

| Phase | Work | Win | Effort |
|---|---|---|---|
| **1** | Counters + cache infrastructure.  Single-tier:  `(op, dtype)` cache. No range observation yet. | Per-backend dtype variants compile lazily. | Small |
| **2** | Range observation + auto-narrow Cast insertion. | First real "JIT-like" specialisation: F32 → F16 / I8 if values fit. | Medium |
| **3** | Shape specialisation.  Compile fixed-size variants for hot shapes. | Compiler can unroll loops, reduce register pressure. | Medium |
| **4** | Constant folding into kernels. | Biggest LLM/CV win — attention masks, conv weights, image mid-grey. | Medium |
| **5** | Full deopt + stale-spec cleanup. | Workloads with shifting distributions stay correct. | Medium |

Each phase is independently deliverable.  Phase 1 alone gives the
runtime a meaningful win (lazy compilation of dtype variants without
requiring all variants at startup).

## What about CPU?

`matrix-cpu` benefits from MX05 too, just less dramatically:

- Dtype specialisation: SIMD width lines up with narrower types
  (8× I8 in a 256-bit register vs 8× F32)
- Shape specialisation: enables bounded-loop optimisations
- Constant folding: same as GPU

V1 of MX05 should support `matrix-cpu` even though the headline win
is on GPU executors.  The interface is the same.

## What about cross-process / cross-host caching?

Out of scope for V1.  V2 could:

- Persist specialised kernels to disk (`~/.cache/matrix-runtime/spec/<hash>.metallib`)
- Share via remote cache (S3-style) for fleet-wide warm-up
- Authenticated kernel cache so a malicious shared cache can't inject
  poisoned kernels

The wire format defined in MX03 already allows shipping pre-compiled
kernels (`KernelSource::Native`); persistence is "just" cache layering
on top.

## Test methodology

Specialisation is hard to unit-test in isolation because the value
emerges from observed workloads.  Per-component tests:

1. **Profile sampler** — feed synthetic dispatches, assert observation
   stats are correct.  Test sampling rate honoured.
2. **Trigger policy** — feed observations, assert the right `SpecKey`s
   are produced.
3. **Per-backend specialiser** — feed `(SpecKey, Op)` pairs, assert
   compiled pipelines exist and produce numerically-correct output.
4. **Cache eviction** — fill cache past LRU cap, assert oldest is evicted.
5. **End-to-end** — run a synthetic 10k-iteration loop, assert that
   specialised pipelines are warm and faster than generic after some
   number of iterations.  Cross-backend test: same workload should
   converge to the same specialisation choices on CPU and Metal.
6. **Deopt safety** — change input distribution mid-run, assert the
   runtime doesn't produce wrong results from a stale spec.

## Constraints

- **Zero external dependencies** — all crates remain `core`/`alloc`/`std`
  only, plus the existing path deps to `matrix-ir`, `compute-ir`,
  `executor-protocol`, `matrix-runtime`, and per-backend executors.
- **No IR changes** — MX01's vocabulary stays exactly as it is.
- **No protocol changes** — MX03's message types stay exactly as
  they are.  Specialisation is purely an executor-internal concern.
- **No planner changes** — MX04's algorithm stays exactly as it is.
  The profiler hooks into the runtime's dispatch loop, not the
  planner.

## Out of scope (V1 of MX05)

- **Cross-process / fleet-wide cache** — single-process for V1.
- **Auto-tuning of threadgroup sizes** — V1 uses fixed sizes.
  V2 can microbenchmark per-shape.
- **Speculative parallel execution on multiple specialisations** —
  V1 picks one, runs it.
- **Kernel fusion across ops** — adjacent ops on the same executor
  could be fused into one kernel; V2 work.

## Open questions

1. **How frequent should distribution-shift detection be?**  V1
   default: never.  V2: check every Nth invocation, deopt if shift
   exceeds threshold.  Cost vs correctness trade-off.

2. **Should specialisation be visible to the user?**  Yes via
   inspection APIs (`Runtime::spec_stats() → Vec<SpecStats>`), but
   not as a user-facing config knob.  Default behaviour stays
   automatic.

3. **What's the right default for `sample_rate`?**  1% is V8's
   conservative default; 10% costs more but reaches the threshold
   faster.  V1 ships 1% with `set_sample_rate()` for tuning.

4. **Does specialisation invalidate when the planner re-routes ops?**
   E.g., a graph that ran on Metal yesterday now lands on CPU because
   Metal is unhealthy.  V1: each backend has its own cache; cross-
   backend invalidation is N/A.

## Cross-references

- **MX00** — architecture overview (cost-model planner)
- **MX01** — `matrix-ir::Op` wire tags used in `SpecKey.op_kind`
- **MX02** — `compute-ir::ComputeGraph` is what the profiler hashes
- **MX03** — `KernelSource` is what the specialiser produces (in
  generated form per backend)
- **MX04** — runtime is where the profiler + trigger live
- **Future MX06** — `matrix-cuda` executor will use the same
  specialisation interface; nothing in MX05 is Metal-specific
- **Future MX07** — kernel fusion across adjacent ops could build on
  MX05's specialised cache infrastructure
