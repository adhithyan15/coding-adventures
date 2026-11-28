# VER07b — Paired microbenchmark comparisons

[VER07](VER07-microbenchmarks.md) defines measurement of an individual benchmark. A paired experiment compares a baseline and a candidate on the same workload. The existing [benchmark runner](benchmarking-tools.md) can orchestrate builds and command-level workloads. Each language runner owns the timing of its in-process batches.

## Collect observations

1. Choose the primary workload, minimum meaningful factor, confidence level, and number of pairs before collecting data. Defaults are 40 pairs, a factor of 1.05, and 95% confidence. Use more pairs or longer batches when a pilot shows high variability. Collect a fresh confirmation run after changing the plan.
2. Verify both variants meet the same correctness criterion outside the timer. Warm both variants and use a common batch size large enough for both to produce usable durations. Exclude warmup, calibration, and setup from samples. Each timed batch executes the same logical work. Prepare independent or resettable workload state if a variant mutates its input.
3. Execute both batches of each pair adjacently, with baseline-first and candidate-first orders shuffled and balanced to within one pair. Record order, elapsed nanoseconds, and operations per batch. Retain every measured pair. If a variant fails, the experiment fails. Repeat independently when serial dependence, thermal drift, or order effects make pairs unreliable.
4. Record the workload, invocation and revision, runtime/compiler settings, machine, and configured seed alongside exported results. Analyze across pairs. Repetitions within one timed batch contribute one measurement per variant.

## Analyze a latency metric

Within each pair compute the candidate duration divided by the baseline duration. Both durations must be positive and measured at the same batch size. The estimated effect is the median ratio. Compute a two-sided distribution-free confidence interval for the median by sorting the ratios and choosing the largest one-based rank `k` for which

```text
2 × P(Binomial(n, 1/2) ≤ k − 1) ≤ 1 − confidence.
```

The sample median and exact order-statistic interval are reusable primitives
specified in [VER07c](VER07c-rank-statistics.md). Benchmark-specific ratios,
practical thresholds, collection, and verdict labels remain in the benchmark
layer.

The endpoints are the `k`-th smallest and `k`-th largest ratios. If no rank qualifies, return an inconclusive verdict with no interval. This procedure assumes independent pairs from a stable sampling process. Coverage can exceed the requested level because the binomial distribution is discrete. Report the requested confidence and observed interval. A result applies to the measured workload and environment.

Let `f > 1` be the minimum meaningful factor. Ratios below 1 favor the candidate.

| Verdict | Condition |
|---|---|
| `faster` | Upper endpoint is strictly below `1 / f`. |
| `slower` | Lower endpoint is strictly above `f`. |
| `equivalent` | The entire interval lies in the closed range `[1 / f, f]`. |
| `inconclusive` | The interval overlaps a decision boundary, or there is no usable interval. |

Report the ratio, interval, verdict, batch size, and raw pairs so consumers can reproduce the claim. Use the [shared fixture](fixtures/microbenchmark-pairs-v1.json) when porting the analyzer. Tests should use known observations and fake clocks. Use a fixed-sample analysis for each experiment, and predeclare a primary outcome or adjust for multiple comparisons if selecting among workloads. Keep noisy shared-CI timing advisory under VER07.

## Language implementation status

| Package lanes | Status |
|---|---|
| Go, Python | Reusable statistics and reference collectors merged. The Windows Go clock repair and fixture-registration foundation are also merged. |
| C, C++, C#, Dart, Elixir, F#, Haskell, Java, Kotlin, Lua, OCaml, Perl, Ruby, Rust, Swift, TypeScript | Native statistics libraries, collectors and indexed-array drivers implemented and locally reviewed. Dependency-order PRs and required CI remain open; see the [delivery backlog](microbenchmark-backlog.md). |
| Mosaic, Starlark, Twig, WASM | Guest entrypoints implemented with Rust host collection/statistics, explicit boundaries and separate raw overhead diagnostics. These use workload forms actually supported by each VM: rendering/event traversal, bytecode addition, Twig recursion and WASM recursion/loops. They are not four independent statistics ports or cross-language measurements of an identical array kernel. Required CI and release checks remain open. |
