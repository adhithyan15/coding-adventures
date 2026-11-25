# VER06j — C Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05i](VER05i-c-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06e](VER06e-go-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md), [VER06h](VER06h-csharp-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-C-FUZZ.

This specification defines the C implementation of the Level 2 coverage-guided
fuzz engine. It lives in the existing package `code/packages/c/test-bootstrap`
beside the property engine of VER05i, whose PRNG, bignum, strict JSON/TFV
readers, allocation seam (`pbt_alloc` and the failure-injection hook) and
iso-harness build it reuses. It conforms to the shared `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and meets
every campaign contract of VER06 §9.1. It follows the audited Go, Java and C#
lanes and carries their security hardening over from the start.

The letter `j` follows VER06i (Ruby, PR #20028), the highest letter in use.

---

## 1. Architecture and Placement

| File | Role |
|---|---|
| `include/fuzz.h` | the whole public API (prefix `fz_`, constants `FZ_`) |
| `include/fuzz_hooks.h` | test seams (sabotage and cap overrides); NOT stable API |
| `src/fuzz_mutate.c` | mutator algebra, interest lists, `fz_mutate_sequence` |
| `src/fuzz_bitmap.c` | `fz_bitmap`, `fz_corpus_admit`, `fz_calculate_energy` |
| `src/fuzz_sig.c` | crash normalisation, `fz_crash_signature`, failure ids, TAP |
| `src/fuzz_engine.c` | handles, executor, minimiser, campaign, report |
| `src/fuzz_corpus.c` | production corpus runner `fz_corpus_evaluate` |
| `src/fuzz_sancov.c` | OPTIONAL sanitizer-coverage callbacks (§4.3); never in the pure-ISO build |

The library is capability-free: no file, clock, environment, network or process
use. `required_capabilities.json` stays `[]`. It performs no I/O; reading
`cases.json` is the job of the tests. The only libm functions used are `sqrt`
and `floor` (energy, §4.1), so Linux links `-lm`.

Memory: every allocation goes through `pbt_alloc` / `pbt_realloc` /
`pbt_free` (VER05i §9), every `count * width` is overflow-checked, and every
path is leak-free. An allocation failure surfaces as `FZ_E_NOMEM` and the
campaign returns with nothing leaked and no partial report; an out-of-memory
inside the corpus runner is a *problem*, never a pass.

## 2. PRNG and Stream Alignment

The campaign and `fz_mutate_sequence` use `pbt_rng` (SplitMix64, VER05 §2).
`pbt_rng_bounded(1)` already consumes one word (VER05 §2.2), so a one-byte
buffer keeps the streams aligned and no wrapper is needed.

## 3. Mutator Algebra

Exactly VER06 §3 and `scripts/fuzz_conformance.py`, with the Go lane's rules
where the reference is silent. Inputs are `(const uint8_t *, size_t)`; outputs
are an owned `fz_bytes` (`p` always non-NULL on success, even for length 0);
no mutator touches its argument; a no-op returns a copy.

* Flips apply by **parity** (VER06 §9.2), O(len) for any count.
* Arithmetic and boundary: widths {1, 2, 4, 8}, `FZ_LITTLE` / `FZ_BIG`; any
  other width or `len < width` returns a copy; delta and val are reduced
  modulo 2^(8·width) (two's complement for negatives); the offset wraps over
  `len - width + 1`.
* Offsets use FLOOR modulo (`-1 mod 8 == 7`); lengths and splits saturate.
* Every mutator has an `int64_t` form and a `_big` form taking a `pbt_big`
  (the corpus carries integers of up to 1,000 digits; the bignum holds 300, so
  a wider integer is a reported *problem*, §9). The big form reduces exactly,
  then calls the same core.
* Inputs above `FZ_MAX_MUTATE_BYTES` (2^30) and any output size that
  overflows are `FZ_E_LIMIT` (CWE-190).
* `fz_mutate_sequence(seed, steps, max_len)`: the reference draw order, 8-bit
  interest values `{0, 1, 127, 128, 255}` and deltas `{-35, -8, -1, 1, 8, 35}`;
  a block insert or duplicate that would grow the buffer past `max_len` is
  skipped AFTER its draws; `steps` above `FZ_MAX_ITERATIONS` and an input above
  `FZ_MAX_INPUT_BYTES` are `FZ_E_LIMIT`. Steps run **in place** on one growing
  buffer.
* **Work accounting (divergence from Go, as C#).** `fz_mutate_sequence_budget`
  takes a work cap and reports the work it did. Work is the number of bytes
  copied: the initial copy (`len`) plus, per step, the length of the buffer the
  step produced (at least 1: what a functional implementation would copy; the
  in-place implementation copies at most that). A charge that would exceed the
  cap is refused BEFORE the step runs and the call fails `FZ_E_WORK`; the work
  of failed calls counts too (`*work_done` is written on every return).

## 4. Coverage

### 4.1 Bitmap, admission, energy

`fz_bitmap`: 8,192-byte map (edges mod 65,536) plus the list of ids in order
first seen, so listing the edges of one execution is O(edges hit). Operations:
`record`, `has`, `admit(trace, n, &new)`, `count` (population), `edges`
(ascending), `merge`, `reset` (O(edges)). `fz_corpus_admit(existing, trace)`
counts *unique* novel edges using sorted copies (O(n log n), `FZ_E_NOMEM` on
failure).

`fz_calculate_energy(base, len, new, total) = max(1, floor(base·(1 + new/total)
/ sqrt(max(1, len)) + 0.5))`, bonus 1.0 when `total == 0`, base at least 1,
clamped to `FZ_MAX_ITERATIONS`, half rounded away from zero. Rows, including
half-way ties, are pinned by tests (the corpus does not pin them).

### 4.2 Coverage source: the decision

C offers three sources, in decreasing order of portability:

1. **Caller-reported edges (default).** The target receives an `fz_handle *`
   and calls `fz_edge(h, id)` at the program points it wants guided. Ids are
   `uint64_t`, reduced modulo 65,536. `fz_edge_id(label)` (FNV-1a) gives stable
   ids from names. Pure ISO C; no compiler support needed; deterministic.
2. **Compiler sanitizer coverage, `-fsanitize-coverage=trace-pc-guard`
   (optional, §4.3).** Real basic-block / edge coverage with no source
   changes, but the callbacks are a compiler extension.
3. **`gcov` / `-fprofile-arcs` counters.** Rejected: the counters are private
   to the runtime, are flushed at exit, and have no supported in-process reader.

**The label is honest.** `fz_report.tier` is `"caller-reported"` when edges
were collected through `fz_edge`, `"sancov-trace-pc-guard"` when at least one
execution collected an edge through the sanitizer-coverage callbacks, and
`"black-box-uninstrumented"` when none was collected (the first passing seed is
still admitted so there is something to mutate, but there is no feedback). The
engine never claims instrumentation it did not perform.

### 4.3 Sanitizer coverage (optional)

`src/fuzz_sancov.c` implements `__sanitizer_cov_trace_pc_guard_init` and
`__sanitizer_cov_trace_pc_guard`. It is **not ISO C** (the symbols exist only
because clang's instrumentation pass calls them), so:

* it is **never** in the library sources of `tools/run.sh`'s pure-ISO build or
  of the sanitizer pass; it is compiled only by a dedicated step together with
  one test target translation unit that is itself compiled with
  `-fsanitize-coverage=trace-pc-guard`;
* **the engine is not instrumented**: only the target translation unit carries
  the flag, so the engine's own branches never pollute the map;
* the init callback numbers the guards 1, 2, 3 ... in the order the runtime
  hands them over (deterministic for a given binary); the guard callback maps
  guard id `g` to edge `(g * 0x9E3779B97F4A7C15) >> 48` (16 bits) and records
  it in the **active** handle (`fz_active_handle()`), a no-op outside a target
  run. The engine core has no dependency on any of this: the pure-ISO build
  contains no reference to a sancov symbol.
* The step is skipped, with a message, for a compiler that does not accept the
  flag.

### 4.4 Power schedule

As VER06 §4.3 and the Rust/Go lanes: `energy = fz_calculate_energy(base,
len(seed), seed.new_edges, cumulative.count())` mutation slots per turn.

## 5. Outcomes, Crashes, Budgets, Handles

### 5.1 What a crash is in C

C has no exceptions. A target is

```c
int target(const uint8_t *data, size_t len, fz_handle *h, fz_crash *out);
```

It returns 0 for a pass and nonzero for a crash. A crash fills in `out`:
`fz_crash_set(out, kind)` sets a short identifier and `fz_crash_push(out,
frame)` pushes frame labels, innermost first, copying them into the
engine-owned struct (so labels that point into the target's dead stack frame
are safe). The engine zeroes `out` before the call and **normalises again after
it**, whatever the target wrote: kind and frames are NUL-terminated within their
arrays, restricted to `[A-Za-z0-9._<>$/-]` (anything else becomes `_`, so **no
frame contains `:`**, closing VER06 §9.4 item 6 for this lane), kind capped at 64
bytes, each frame at 64, consecutive identical frames collapsed (a recursive
failure hashes alike at any depth), at most the innermost 64 frames kept, an
empty kind becomes `unspecified`. A frame count above the array is clamped.

**What C cannot do (recorded, not hidden).**

1. A *real* crash (SIGSEGV, SIGABRT, SIGFPE, a stack overflow, `abort`,
   `exit`, a wild write) cannot be caught portably in ISO C. Catching a signal
   and continuing after a fault is undefined behaviour, and `longjmp` out of a
   signal handler for a synchronous fault is too. **In-process signal handling
   is out of scope.** A target that can fault for real must be run in a
   separate process (a fork-server or a harness that reports the exit status as
   a crash outcome through `fz_run_outcomes`, which takes an arbitrary
   executor). The in-process engine reports crashes the target *detects and
   declares* (a checked invariant, a failed bounds test, a returned error code).
2. The engine spawns no threads or timers and cannot pre-empt a loop that never
   calls `fz_tick`; CI must apply an external wall-clock limit.
3. A target that swallows its own failures hides them.
   **Threads:** the engine is single-threaded. A target must not start threads
   that outlive its return, nor call `fz_edge` / `fz_tick` / `fz_reserve` from
   another thread: the coverage map and handle are unsynchronised. A target must
   join its threads before returning. `fz_bitmap_record` refuses to grow its
   edge list past the map (defence in depth, not thread safety), the sancov
   callback has a re-entrancy guard, and `fz_hooks` is a mutable test-seam
   global that production code never writes. Keeping the `fz_crash *` past the
   call is misuse.
4. Global state of the target is not isolated between executions.

### 5.2 Budgets: sticky exhaustion

* **Timeout** = more than `step_budget` calls to `fz_tick(h)` in one
  execution (default 200,000, ceiling 100,000,000). A step count, never
  wall-clock, so timeouts are deterministic.
* **OOM** = the running total of `fz_reserve(h, n)` exceeds `mem_budget`
  (default 64 MiB, ceiling 1 GiB). The overflow-safe test is
  `n > budget - reserved`. `fz_reserve` is the cooperative accounting hook:
  call it before an allocation sized by the input.
* Both return **nonzero when exhausted**. Exhaustion is **sticky and shared**:
  the first breach of either budget is recorded in the handle, and afterwards
  both `fz_tick` and `fz_reserve` return exhausted at once.
* **After the target returns, an exhausted handle means TIMEOUT or OOM** (by
  the budget that broke first), never a pass and never a crash: a target that
  ignored the return value and returned 0, and one that then returned nonzero,
  are both a timeout/OOM. Counted, never a crash signature.

### 5.3 Handles: one per execution, retired after

The campaign issues a FRESH handle for every execution and **retires** it as soon
as the target returns. A retired handle's `fz_edge` does nothing, `fz_tick` and
`fz_reserve` return exhausted and count nothing, and `fz_handle_user` is NULL.
Handles live in engine-owned static storage (a ring of `FZ_HANDLE_RING` = 4,096
slots, each carrying its own live flag, budgets, a never-reused epoch number and
the coverage pointer, which is cleared on retirement), never on the stack, so a
target that kept the pointer can neither corrupt a later execution nor read freed
memory, even after the campaign has returned. A slot is reissued only when it is
not live (a campaign nested inside a target keeps its own handle live), scanning
from the next ring position. **Limit, stated plainly:** a pointer kept for more
than 4,095 later executions can alias the handle of a later execution; the alias
is memory-safe and merely attributes late calls to that execution.
The 4,096-slot aliasing window is also documented at `fz_edge` in `fuzz.h`. `fz_handle_epoch` lets tests prove a handle is the one they were given.

### 5.4 Copies

The input is copied into a fresh heap buffer before every call (and the buffer
freed after), so a target that writes through the pointer, or keeps it, cannot
corrupt the corpus. An allocation failure of that copy is an engine error
(`FZ_E_NOMEM`), not charged to the target.

### 5.5 Signature, dedup, minimisation

`fz_crash_signature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by
":"))`, 8 lowercase hex, computed by streaming (no length cap on the corpus
runner's strings; frames are not escaped, so one frame `x:y` and two frames `x`,
`y` hash alike: this lane never produces a frame with a colon). Real values are
pinned in tests against the Python oracle.

`fz_minimize_crashing_input(data, sig, test, budget)`: VER06 §5.3 as in Go
(first chunk `max(1, len/2)` so the empty input is reachable; exact signature
only; pass 2 zeroes single bytes). Minimisation is budgeted in two layers:
`minimize_budget` (default 2,000, ceiling 100,000) caps ONE crash, and the
whole campaign spends at most `max_iterations` minimisation executions in total.
**Every candidate execution counts, accepted or rejected**, and the budget is
checked before each. A crash found after the total is spent is recorded with
`minimized == 0`.

## 6. Campaign Contracts (VER06 §9.1)

`fz_run_outcomes(corpus, options, exec, user, &report)` is the outcome-level
campaign; `fz_run(corpus, options, target, &report)` wraps a target in the
executor. Options (defaults from `fz_options_default()`): `seed 42`,
`max_iterations 1000`, `max_input_len 4096`, `base_energy 10`,
`minimize_budget 2000`, `step_budget 200000`, `mem_budget 64 MiB`, `user`
(retrievable by the target through `fz_handle_user`). Every field is validated
(`FZ_E_ARG`).

Determinism, budget, honest report, dedup and feedback are VER06e §6 verbatim,
with the same draw order per iteration. Caps (CWE-400/190), all with counted
drops: `FZ_MAX_INITIAL_CORPUS` 10,000 seeds of at most `FZ_MAX_INPUT_BYTES` each,
`FZ_MAX_CRASHES` 1,000 distinct signatures (more only increment
`crashes_dropped`), and **retained bytes** `FZ_MAX_RETAINED_BYTES` (256 MiB):
the campaign keeps at most that many bytes of corpus seeds and crash inputs; an
admission that would exceed it is not admitted and is counted in
`admissions_dropped`; a crash record that would exceed it keeps no input
(`minimal == NULL`, `input_dropped == 1`, `minimized == 0`) and is not minimised.

## 7. Private Reporting (VER06 §6)

`fz_format_tap_failure` emits `not ok N - <target> [<failure-id>:<hex8>]` only.
The failure id must be one of the five registered (`fuzz-crash`,
`fuzz-timeout`, `fuzz-oom`, `fuzz-leak`, `fuzz-panic`; the last two are
registered but this engine never produces them); the signature must be exactly 8
lowercase hex digits; the target is truncated to 128 bytes and reduced bytewise
to `[A-Za-z0-9._/-]` (every other byte, controls included, becomes `_`). No
argument can forge a TAP line. `fz_tap_failure_line(report, i, ...)` renders a
report: one line per distinct crash signature, then one for timeouts and one for
OOMs (their hash is the signature of the bare kind `timeout` / `oom`).
`fz_report_summary` prints counts, tier and cleanliness only. **No input byte and
no seed ever appears in any rendering**; minimal inputs and the corpus are held
in the report only for explicit access (documented PRIVATE) and the campaign
seed is not stored at all. Case ids in corpus problem lines are clipped to 200
bytes with every non-printable byte shown as `?`.

## 8. Test Seams (`fuzz_hooks.h`)

A conformance run that cannot fail proves nothing. `fz_hooks` holds function
pointers (NULL = the real one) for every engine entry point the corpus runner
calls (the nine mutators, `corpus_admit`, `is_duplicate`, `is_failure_id`,
`format_tap`, `tfv_validate`), plus overrides for the interest and delta tables,
the energy rounding, a post-hash transform of the signature, and the caps
(`max_crashes`, `max_retained_bytes`). Production code only reads them;
`fz_hooks_reset()` restores production values.

## 9. Corpus Runner

`fz_corpus_evaluate(document, &report)` evaluates a parsed `cases.json` and fills
a `pbt_corpus_report` (shared with VER05i: `evaluated`, `nproblems`, the first
1,000 problems each starting with the case id). Strictness and caps are the
shared ones: exact field sets, canonical decimal ints, lowercase even hex; 10,000
elements/bytes/steps, ints within the bignum's 300 digits / 1,000 bits (a wider
one is a problem, a documented limit of this lane), depth 64, documents over
8 MiB refused by the parser, every problem clipped to 400 bytes; an unknown op,
non-object case, missing/duplicate/non-string id or wrong field set is a
**problem**, never a skip. Gates: exactly 25 cases, every op exercised, a
failure-id acceptance and a rejection seen, every registered id accepted.
Engine invariants beyond the expected values: flips/arithmetic/boundary keep the
length, splice length is prefix plus suffix, admission equals set difference, no
TAP newline, the privacy verdict recomputed from the bytes.

The aggregate `mutate_sequence` work of one corpus is capped at `2^24` bytes
**actually copied** (§3): each call is handed what is left as its work cap, the
work it really did is deducted **including a call that hits its cap**, the
aggregate is then ZEROED and every later `mutate_sequence` case fails at once
without running. `corpus_reject` is judged by `pbt_tfv_validate`.

## 10. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | caller-reported default; optional `trace-pc-guard` unit |
| 2 | Real crashes (§5.1) | not catchable in ISO C; separate process via `fz_run_outcomes` |
| 3 | Crash payload | declared by the target (`fz_crash`), engine-owned copy, normalised twice |
| 4 | Handles (§5.3) | static ring, live flag + epoch; 4,095-execution aliasing limit |
| 5 | Budget exhaustion | returned (not thrown), sticky, shared |
| 6 | Mutate-sequence work | counted in bytes copied, per call and aggregate (as C#, not Go's steps×len estimate) |
| 7 | Wide integers | corpus ints above 300 digits are a problem, not evaluated |
| 8 | Retained-bytes cap | as Java/C# |
| 9 | §9.4 items 7, 8, 9 | **open**, as in Go, Java and C# |
| 10 | Corpus weaknesses (backlog PT-F1/F7) | the splice case has equal splits, `mutate_sequence` does not pin every constant, energy and signatures are unpinned; this lane checks the 820 oracle vectors of Go's `fuzz/reference_vectors_test.go` through the `int64` and `pbt_big` entry points and pins the rest with targeted tests |
| 11 | Hang protection | cooperative: only `fz_tick` counts; CI needs an external limit |

## 11. Conformance Verification

* All 25 `fuzz-v1` cases through `fz_corpus_evaluate` (0 problems, 25 evaluated),
  with per-case corruption controls (each must fail and name exactly that case),
  dropped and truncated corpora, and engine sabotage through `fz_hooks`.
* The 820 reference vectors through both the `int64` and `pbt_big` paths.
* Pinned blind spots: real signature hashes (cross-checked with the Python
  oracle), the power-schedule rows including half-way ties, every interest and
  delta constant, unequal splice splits with target and donor swapped.
* Campaign tests for every §6 contract: determinism, dedup, staircase feedback,
  sticky exhaustion (exhaust-then-call, exhaust-then-return), a stale handle
  being inert, the retained and crash caps, the minimisation budget, privacy of
  every rendering.
* An out-of-memory sweep (injection at every allocation) over the campaign, the
  minimiser and the corpus runner: no crash, no leak, no false success.
* The whole suite under every available compiler (strict ISO flags) and under
  ASan + UBSan, via `tools/run.sh`; the optional sanitizer-coverage test where a
  compiler supports it.
