# VER07 — Microbenchmark Harness, Deterministic Cost Counters, and Timing Calibration

**Parent:** [VER00 §7.4](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER20](VER20-resource-timing-proofs.md), [VER26 §3](VER26-ct-compare-pilot.md).

This specification defines the architecture, calibration protocol, robust statistical estimators, deterministic cost counter abstractions, budget checking, and structured TAP/JSON reporting for the verified microbenchmark harness (Level 5 performance verification in the VER hierarchy).

It delivers **VER-P016**: the formal specification and zero-third-party-dependency microbenchmark harness in `code/packages/rust/test-bootstrap`.

---

## 1. Architectural Placement and Guiding Principles

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ L5: VER01 / VER28  Assurance Report & Ship Gate             │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER08 / VER22-24 Capability Analysis & Sandboxing       │
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER09-11 / 19-20 Prover Kernel, SMT, Timing Proofs     │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER05-06 / P009-015 Property Engine & Fuzzing (L2)      │
  ├─────────────────────────────────────────────────────────────┤
  │ L1: VER04 / P005-008 Warrant Compiler -> Tests (L1)         │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01 / P003-004 test-bootstrap Harness & TFV v1 (L0)    │
  └─────────────────────────────────────────────────────────────┘
  │ Performance: VER07 (Budgets & Counters) / VER20 (Proofs)    │ ◄── THIS SPEC
```

As established in **VER00 §7.4**, performance receives the same rigor as correctness:
1. **Proof vs. Measurement:** Proved bounds (such as WIR step bounds, memory bounds, and constant-time noninterference) establish mathematical guarantees. Measured benchmarks calibrate physical parameters ($t_{\text{step}}$ on platform $P$) and validate whether models reflect reality.
2. **Deterministic CI Gating:** Shared CI runners suffer from high thermal, scheduling, and virtualization noise. **Shared CI runners must never gate on noisy wall-clock measurements.** Gates enforce deterministic cost counters:
   - Abstract WIR / virtual machine step counts;
   - Heap allocation counts and total allocated bytes;
   - Hardware instructions retired, where hardware performance monitoring units (PMUs) are accessible, reporting honest `NotLive` otherwise.
3. **Advisory Wall-Clock Timing:** Wall-clock durations are calibrated and reported with robust statistics (median, median absolute deviation, bootstrap confidence intervals) to track empirical trends, but do not fail blocking gates on noisy runners.

---

## 2. Measurement Lifecycle and Calibration Protocol

A benchmark run executes a callable unit $f$ through four sequential stages:

```text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  1. Warmup   │ ──► │2. Calibrate  │ ──► │  3. Sample   │ ──► │ 4. Analysis  │
│  Iterations  │     │  Batch Size  │     │ Collection   │     │ & Statistics │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

### 2.1 Stage 1: Warmup
- **Goal:** Pre-fault instruction caches, trigger CPU dynamic frequency scaling stabilization, and initialize runtime thread state.
- **Budget:** Configurable warmup period (default: 100 iterations or 10 milliseconds, whichever completes first). Results during warmup are discarded.

### 2.2 Stage 2: Automatic Batch Calibration
Timer APIs (e.g. `clock_gettime(CLOCK_MONOTONIC)` or `mach_absolute_time`) have intrinsic call latency ($\approx 20\text{--}50\,\text{ns}$) and discrete granularity. Measuring a single iteration of a fast operation ($\approx 5\,\text{ns}$) is dominated by timer overhead.
- **Minimum Batch Target $\Delta t_{\min}$:** The harness calibrates a batch size $B \ge 1$ such that:
  $$\text{batch\_duration}(B) \ge \Delta t_{\min} \quad (\text{default } \Delta t_{\min} = 100\,\mu\text{s})$$
- **Calibration Loop:**
  1. Start with $B = 1$.
  2. Measure duration $t$ of running $f$ in a loop $B$ times.
  3. If $t < \Delta t_{\min}$, scale $B \leftarrow \max(B \times 2, \lceil B \times \frac{\Delta t_{\min}}{t} \rceil)$ up to a safety ceiling $B_{\max}$ (default $10^7$, configurable).
  4. Repeat until $t \ge \Delta t_{\min}$ or $B = B_{\max}$.

### 2.3 Stage 3: Sample Collection
- Collect $K$ independent sample batches (default $K = 50$, minimum $K = 10$).
- For each sample $i \in \{1, \dots, K\}$, measure:
  $$\text{elapsed\_time}_i, \quad \Delta\text{steps}_i, \quad \Delta\text{alloc\_bytes}_i, \quad \Delta\text{instructions}_i$$
- Compute per-iteration values:
  $$x_i = \frac{\text{elapsed\_time}_i}{B}, \quad s_i = \frac{\Delta\text{steps}_i}{B}, \quad a_i = \frac{\Delta\text{alloc\_bytes}_i}{B}$$

### 2.4 Dead-Code Elimination Barrier
To prevent optimizing compilers from eliding pure benchmark computation or hoisting loop invariants outside the measurement window, benchmark harnesses must pass inputs and outputs through an identity compiler barrier:
```rust
std::hint::black_box(value)
```

---

## 3. Robust Statistical Estimation

Because system interrupts and CPU throttling produce asymmetric right-tailed noise, conventional mean and standard deviation are fragile. The harness computes robust non-parametric metrics:

### 3.1 Median $\tilde{x}$
For sorted per-iteration samples $x_{(1)} \le x_{(2)} \le \dots \le x_{(K)}$:
$$\tilde{x} = \begin{cases}
x_{((K+1)/2)} & \text{if } K \text{ is odd} \\
\frac{1}{2}\left(x_{(K/2)} + x_{(K/2 + 1)}\right) & \text{if } K \text{ is even}
\end{cases}$$

### 3.2 Median Absolute Deviation (MAD)
$$\text{MAD} = \text{median}\left(\left| x_i - \tilde{x} \right|\right)$$
Normalized MAD ($\text{nMAD}$) provides a consistent estimator of standard deviation under an asymptotic normal distribution:
$$\text{nMAD} = 1.4826 \times \text{MAD}$$

### 3.3 Percentiles ($p_{25}, p_{75}, p_{95}, p_{99}$)
Using the standard continuous linear interpolation method across sorted sample ranks $r = p \times (K - 1) + 1$.
When the two bracketing samples are equal, return their value exactly.

### 3.4 Bootstrap Confidence Intervals
To quantify estimation uncertainty without assuming parametric normality:
- Perform $M = 1000$ bootstrap resamples of size $K$ using the deterministic SplitMix64 PRNG (**TF01 §5.1**).
- For each resample $m$, calculate resample median $\tilde{x}^{*(m)}$.
- Sort the $M$ resample medians. The 95% two-sided confidence interval $[\text{CI}_{\text{lower}}, \text{CI}_{\text{upper}}]$ is:
  $$\text{CI}_{\text{lower}} = P_{0.025}\big(\tilde{x}^{*(1)}, \dots, \tilde{x}^{*(M)}\big), \quad \text{CI}_{\text{upper}} = P_{0.975}\big(\tilde{x}^{*(1)}, \dots, \tilde{x}^{*(M)}\big)$$
  where $P_p$ is the same linear-interpolation percentile as §3.3 (amended in
  the audit, §8: the earlier order-statistic wording differed from it only in
  interpolation weight).

---

## 4. Deterministic Cost Counters

The harness isolates deterministic counters from non-deterministic clocks:

```rust
pub trait CostCounter {
    fn name(&self) -> &'static str;
    fn start(&mut self);
    fn sample(&mut self) -> CounterSample;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterSample {
    Live(u64),
    NotLive,
    Unsupported,
}
```

### 4.1 Step Counter (`StepCounter`)
Hooks into abstract operation counters (such as Warrant Intermediate Representation instruction steps or VM dispatch loops). When running native code, reports explicit operation counts if instrumented.

### 4.2 Allocation Counter (`AllocCounter`)
Monitors heap activity during the measurement window. Measures total heap allocations and byte volumes. For zero-allocation cryptographic loops (such as constant-time byte comparisons), asserts $\Delta\text{alloc\_bytes} == 0$.

### 4.3 Hardware Instruction Counter (`HardwarePerfCounter`)
- **Linux:** Reads hardware performance monitoring events via `perf_event_open` for `PERF_COUNT_HW_INSTRUCTIONS` and `PERF_COUNT_HW_CPU_CYCLES`.
- **macOS / Windows / Non-privileged Environments:** Where kernel performance counter virtualization is disabled, restricted, or absent, the hardware counter safely returns `CounterSample::NotLive`. CI does not fail on `NotLive` hardware counters; it logs the state honestly.

---

## 5. Budget Specification and Verification

A benchmark may declare a performance budget:

```rust
#[derive(Clone, Debug, Default)]
pub struct Budget {
    /// Soft budget: maximum acceptable median duration in nanoseconds.
    pub max_median_duration_ns: Option<u64>,
    /// Hard budget: maximum acceptable deterministic steps per operation.
    pub max_steps: Option<u64>,
    /// Hard budget: maximum acceptable heap allocation bytes per operation.
    pub max_alloc_bytes: Option<u64>,
    /// Hard budget: maximum acceptable retired instructions per operation (checked only when live).
    pub max_instructions: Option<u64>,
}
```

### 5.1 Gating Rules
1. If `max_alloc_bytes` is specified and observed $\text{alloc\_bytes} > \text{max\_alloc\_bytes}$, verdict is **FAIL**.
2. If `max_steps` is specified and observed $\text{steps} > \text{max\_steps}$, verdict is **FAIL**.
3. If `max_instructions` is specified:
   - If counter is `Live(count)` and $\text{count} > \text{max\_instructions}$, verdict is **FAIL**.
   - If counter is `NotLive` or `Unsupported`, verdict is **PASS** with recorded annotation `counter: not-live`.
4. If `max_median_duration_ns` is exceeded:
   - In strict local mode: marked as warning or soft failure.
   - In CI mode: recorded in diagnostics as an advisory finding; does not fail the build gate.

---

## 6. Output Formats

### 6.1 TAP 14 Microbenchmark Protocol
Each benchmark emits a TAP 14 test point accompanied by a structured YAML diagnostic block:

```tap
TAP version 14
1..1
ok 1 - bench/ct_eq/16_bytes
  ---
  benchmark: ct_eq_16
  batch_size: 2048
  samples: 50
  timing:
    median_ns: 24.1
    mad_ns: 0.6
    nmad_ns: 0.89
    p95_ns: 25.8
    ci_95_ns: [23.8, 24.5]
  counters:
    steps: 16
    alloc_bytes: 0
    instructions: 72 # live or not-live
  budget:
    max_alloc_bytes: 0
    verdict: pass
  ...
```

### 6.2 Structured JSON Report
A machine-readable JSON summary is written to verify historical trends and facilitate baseline comparisons:

```json
{
  "version": "1.0",
  "name": "ct_eq_16",
  "batch_size": 2048,
  "samples": 50,
  "median_ns": 24.1,
  "mad_ns": 0.6,
  "p25_ns": 23.9,
  "p75_ns": 24.4,
  "p95_ns": 25.8,
  "ci_95_lower_ns": 23.8,
  "ci_95_upper_ns": 24.5,
  "steps": 16,
  "alloc_bytes": 0,
  "instructions": null,
  "budget_verdict": "pass"
}
```

---

## 7. Pilot Acceptance Gate (VER-P016)

Acceptance for VER-P016 requires:
1. Complete zero-third-party-dependency implementation of the microbenchmark harness in `code/packages/rust/test-bootstrap/src/bench.rs`.
2. Automatic calibration of batch sizes up to target threshold.
3. Accurate computation of median, MAD, percentiles, and deterministic SplitMix64 bootstrap confidence intervals.
4. Deterministic step, allocation, and hardware counter abstraction reporting `Live`, `NotLive`, and `Unsupported`.
5. Budget validation tests (passing, failing on hard counters, and graceful `NotLive` handling).
6. Comprehensive test suite achieving $\ge 90\%$ line and branch coverage.

---

## 8. Audit of the Rust lane: testable contracts and divergences

Tests: `code/packages/rust/test-bootstrap/tests/bench_statistics.rs`
(20 tests) plus the unit tests in `bench.rs`. Wall-clock values are never
asserted; everything else is.

### 8.1 Statistics, as hand-checked contracts

- Percentile by linear interpolation at rank `p (K - 1)`, `p` clamped to
  `[0, 1]`: for `[1,2,3,4,5]`, p25 = 2, p75 = 4, p95 = 4.8, p99 = 4.96; for
  `[1,2,3,4]`, p25 = 1.75, median = 2.5, p75 = 3.25. An empty slice panics.
- Median = the odd/even definition of §3.1; MAD = median of absolute
  deviations (a single outlier does not move it): `[1,2,3,4,100]` has median
  3 and MAD 1; `[1,2,3,4]` has MAD 1.0.
- nMAD = `1.482602218505602 x MAD`, i.e. `1 / Phi^-1(0.75)`. The earlier text
  quoted `1.4826`; the constant agrees to the digits quoted and the full value
  is the contract.
- Bootstrap interval: `M = 1000` resamples of size `K` with SplitMix64
  `bounded(K)` draws, medians sorted, bounds taken by **the same linear
  interpolation as the percentiles** at p = 0.025 and 0.975. The earlier text
  (order statistics `floor(0.025 M)` and `ceil(0.975 M)`) is replaced by this
  rule; the two differ by interpolation weight only. It is checked against an
  independent re-derivation for several seeds and resample counts, always lies
  inside `[min, max]` of the samples, and is `(c, c)` for a constant sample.

### 8.2 Budgets

The hard rules of §5.1 hold exactly at the limit (a step budget of 10 passes at
exactly 10 per iteration and fails at 9 with `observed = 10, limit = 9`; an
allocation budget of 0 fails on 1 recorded byte). A soft timing budget
produces `PassWithWarning` and never a failure.

**New contract: a declared hard budget that could not be checked is reported.**
`BenchmarkResult::unchecked_hard_budgets` lists the hard budgets
(`steps`, `alloc_bytes`, `instructions`) that were declared but not measured:
no counter of that kind attached, an unknown counter name, or a counter that
was `NotLive`/`Unsupported`. The verdict rules are unchanged (a `NotLive`
instruction budget is still a Pass, §5.1 rule 3) but the TAP YAML
(`unchecked_hard_budgets: [..]`) and JSON now say so. `bench()` with
`max_steps: Some(1)` used to be indistinguishable from a budget that held.

**`AllocCounter` measures the real heap (LH-003a; was cooperative).** The
first `AllocCounter` counted only what a closure recorded with
`record_alloc`, so a closure that allocated 4 KiB passed
`max_alloc_bytes: Some(0)` and every "zero allocation" claim was vacuous.
`record_alloc` is gone. `counting_alloc::CountingAllocator` is a
`GlobalAlloc` wrapper that delegates to `System` and tallies allocations,
frees and bytes per thread; a benchmark binary installs it with
`install_counting_allocator!()` (only a final binary can choose the global
allocator), and `AllocCounter` reads the calling thread's tally. Contracts:

- `Vec::<u8>::with_capacity(1)` fails `max_alloc_bytes: Some(0)` with
  `observed = 1`; `1 + 1` passes with `alloc_bytes_per_iter = 0.0`
  (`tests/alloc_budget.rs`).
- If the allocator is *not* installed, `AllocCounter` reports `NotLive`
  (proved by probing with a real allocation, which leaves the tally
  untouched) and `alloc_bytes` stays in `unchecked_hard_budgets`; a zero is
  never fabricated.
- Limits: allocations on threads the closure spawns, memory not obtained
  through Rust's global allocator, and in-place `realloc` growth (counted
  conservatively as free plus allocate) are not or only roughly counted.

### 8.3 Output

Benchmark names are escaped (`"`, `\`, control and line-separator
characters) in both the YAML block and the JSON, so a name cannot break the
JSON or add a TAP line.

### 8.4 Divergences found in the audit

| # | Spec | Rust lane | Resolution |
|---|---|---|---|
| 1 | §2.2 `B_max = 10^8` | `max_batch_size` defaults to `10^7` | Spec amended to the default in `BenchmarkConfig` (configurable) |
| 2 | §2.3 minimum `K = 10` samples | Any `K >= 1` runs (`K = 0` panics) | Open: recorded; statistics on fewer than 10 samples are not meaningful |
| 3 | §3.4 CI by order statistics | Linear interpolation | Spec amended (8.1) |
| 4 | §3.2 `nMAD = 1.4826 x MAD` | `1.482602218505602 x MAD` | Spec amended (8.1) |
| 5 | §6.1 / §6.2 field names (`samples`, `timing:`, `verdict: pass` ... `version`, `ci_95_lower_ns`, `steps`, `budget_verdict: "pass"`) | `sample_count`, `timing_ns:` with unsuffixed keys, `steps_per_iter`, `budget_verdict` strings `"failed: <reason>"`/`"warning: <reason>"`, no `version`, no `ok N - bench/...` line | Open: the Rust output is the de facto format; a consumer written to the spec examples will not parse it |
| 6 | §5 hard budgets | Declared but unmeasurable budgets were silent | Fixed (`unchecked_hard_budgets`) |
| 7 | §4.2 "monitors heap activity" | Cooperative counter only | Documented (8.2) |
