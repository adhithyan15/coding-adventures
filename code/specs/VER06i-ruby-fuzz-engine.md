# VER06i — Ruby Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05h](VER05h-ruby-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06d](VER06d-python-fuzz-engine.md), [VER06e](VER06e-go-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md), [VER06h](VER06h-csharp-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-RB-FUZZ.

This specification defines the Ruby implementation of the Level 2
coverage-guided fuzz engine. There is **one** Ruby engine: module
`CodingAdventures::TestBootstrap::Fuzz` of the gem
`code/packages/ruby/test_bootstrap` (beside `Prng`, `Pbt`, `Tfv`, `CorpusJson`).
It reuses `Prng::SplitMix64` (stream and FNV-1a) and the strict corpus plumbing
VER05h introduced (`CorpusJson::StrictJson`, `Values`, `Canon`, `Clip`, `Safe`).
It conforms to the `fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/cases.json`,
25 cases) byte for byte and meets every campaign contract of VER06 §9.1. Ruby is
dynamically typed like Python, so the shape of the code follows
[VER06d](VER06d-python-fuzz-engine.md); the hardening follows
[VER06f](VER06f-java-fuzz-engine.md) and [VER06h](VER06h-csharp-fuzz-engine.md),
which went through several security-review rounds. **All of that hardening is in
the first version here.** The letter `i` is the next free one after `VER06h`
(the C# lane, branch `claude/pt-cs-fuzz`). This document records only what Ruby
changes; the shared reasoning is in the documents above.

---

## 1. Architecture and Placement

| File (`fuzz/`) | Role |
|---|---|
| `limits` | the CWE-400 ceilings |
| `mutate` | mutator algebra, interest lists, `mutate_sequence` (+ counted form) |
| `bitmap` | 65,536-edge map, admission, `corpus_admit`, `calculate_energy` |
| `signature` | `crash_signature`, frame normalisation, `format_tap_failure`, `tap_failures` |
| `failure_ids` | the closed failure-id vocabulary |
| `outcome`, `options`, `report` | outcomes, validated options, frozen reports and crash records |
| `handle` | per-execution coverage and budget handle, sentinels |
| `coverage_source` | `Coverage`-module branch edges, with honest fallback |
| `engine` | executor, minimiser, campaign (`Fuzz.run`, `Fuzz.run_outcomes`) |
| `hooks` | sabotage hooks for the negative controls |
| `corpus` | production runner for the shared corpus |

The package stays capability-free: no file, clock, environment, network or
process use (`Coverage` is an interpreter facility; no `Thread` or `Timeout` is
ever started). `required_capabilities.json` stays empty. The corpus runner takes
the raw JSON; reading `cases.json` and Go's reference-vector table is the job of
the tests. **Zero runtime dependencies**: the standard library only (`coverage`
and `timeout`, default gems).

## 2. PRNG and Stream Alignment

`SplitMix64#bounded(n)` already draws a word for `n == 1` (VER05 §2.2), so no
wrapper is needed (as in Go and Java; Python needs one). Stream words are
non-negative `Integer`s masked to 64 bits by the PRNG itself.

## 3. Mutator Algebra

Exactly VER06 §3 and VER06e §3. Inputs are `String`s of any encoding, treated as
raw bytes; **outputs are always new binary strings and no mutator modifies,
freezes or returns its argument**. A non-`String` input or non-`Integer`
argument (`true`, a `Float`, `nil`) is a `TypeError`.

* There is **no long/big split** (Java and C# need one): Ruby integers hold the
  corpus's 1,000-digit values exactly; offsets use Ruby's floored `%`.
* **Flips are applied by parity** (VER06 §9.2): the window wraps over the
  buffer, each unit is hit `count // n` times plus once more for the first
  `count % n` units, and only parity matters. The buffer is read as one
  little-endian `Integer` (bit `i` is bit `i % 8` of byte `i / 8`, LSB first),
  XORed with an all-ones mask when `count // n` is odd and with the wrapped window
  mask for the remainder: O(len) for any `count`.
* `byte_arithmetic` / `boundary_insert`: widths 1, 2, 4, 8 only; any other
  width, or a buffer shorter than the width, returns a copy unchanged. The offset
  wraps over `len - width + 1` positions.
* `mutate_sequence(data, seed, steps, max_len)` has the Python/Java draw order:
  per step `bounded(7)`, then that step's draws (flip: index; arithmetic: delta
  index, offset; boundary: value index, offset; delete: length then offset,
  skipped when `len <= 1`; insert: byte then offset; duplicate: length, source,
  destination, skipped when `len <= 1`). An empty buffer is refilled with `00`
  before every step. An insert or duplicate that would pass `max_len` is skipped
  **after** its draws. Sequence values and deltas are the 8-bit ones; the 16/32/64
  lists exist and are pinned but unused by the engine (VER06 §9.4 item 7).
* **Work cap (CWE-400).** `steps` above `MAX_ITERATIONS` (1,000,000) or input
  above `MAX_INPUT_BYTES` (1 MiB) is an `ArgumentError`. The bytes one call
  copies are charged: the initial copy, then the current length before each step.
  A call that would pass `MAX_SEQUENCE_WORK` (2^28) raises `WorkExhausted`
  carrying the work done. `mutate_sequence_counted` returns `[bytes, work]` and
  takes the cap as an argument; the corpus runner uses it for its aggregate (§8).

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1: a 65,536-bit map (8,192-byte string; edges reduced with floored
`%`), `record`, `has?`, `admit(trace)`, `covered_edges` (ascending), `count`,
`merge`, `reset`, `raw`. `corpus_admit(existing, trace)` counts *unique* novel
edges over the full, unreduced ids. `calculate_energy(base, len, new, total) =
max(1, floor(base * (1 + new / total) / sqrt(max(1, len)) + 0.5))`, bonus 1.0
when `total == 0` (half **away from zero**), `base` clamped to at least 1.

### 4.2 Coverage source (VER06 §7) — the decision

Ruby's standard library reports **real, in-process, per-execution coverage**
with no agent, flag or third-party code: the `Coverage` module with
`branches: true`. Two sources feed one per-execution edge bitmap:

1. **`ruby-coverage-branches`** (`CoverageSource.branches(load: [...])`). The
   campaign calls `Coverage.setup(branches: true)`, `load`s the target files
   (constraint below), `Coverage.resume`s, and after **every** execution calls
   `Coverage.result(stop: false, clear: true)`: the counters it returns are that
   execution's, and the call zeroes them for the next. Every counter above zero
   is an edge:

   ```text
   edge = fnv1a64("<basename>:<branch>:<target>") mod 65536
   branch, target = "<type>@<line>:<col>-<line>:<col>"   e.g. "if@3:4-7:7"
   ```

   This mirrors Python's `basename:qualname:from:to`. The id uses the **basename**
   and the branch's source position, never an absolute path or Ruby's internal
   creation counter, so it is stable across checkouts. Only the target is
   measured: the engine's own directory, interpreter library directories
   (`RbConfig` `rubylibdir`, `rubyarchdir`, `sitelibdir`, `vendorlibdir`, ...),
   installed gem directories, `(eval)` and `<internal:` pseudo-paths are
   excluded, as is anything outside an optional `include:` prefix list.
2. **`caller-reported`**: the target calls `h.edge(id)` (`Handle.edge_id(label)`
   gives a stable FNV-1a id), as in Go and Java. Both may be used at once.

**Constraint of `Coverage` (recorded, tested).** It instruments only files
compiled **after** `setup`, and reloading a file replaces its counters
(`lessons.d`: *Ruby Coverage reload resets per-file counters*). So the `load:`
files are loaded after `setup` and before the first execution, never during the
campaign. Code `require`d earlier is not measured, and the report then honestly
says `caller-reported` or `black-box-uninstrumented`.

**Coexistence with SimpleCov and any other tool.** `Coverage` is process-global
and `result(clear: true)` would zero *their* counters. The engine therefore
**never touches a `Coverage` run it did not start**: when `Coverage.running?`
is true, `setup` raises because a run is already set up, or `Coverage` cannot be
required or lacks `branches`, the source **falls back** to caller-reported edges
(the `load:` files are still loaded so the target behaves the same, only
unmeasured by the engine). The reason is the source's `mode` (`:branches`,
`:fallback_active_coverage`, `:fallback_unavailable`). When the engine started
`Coverage` it stops it at the campaign's end (in an `ensure`). For this gem's own
suite, which runs under SimpleCov: the source's logic is tested in-process
against an injected fake backend (so the 95 % gate counts it), and real
`Coverage` behaviour (real discovery, the fallback decision under a live run, the
staircase) is tested in **subprocesses** that load no SimpleCov. The few
delegating lines of the real backend are excluded from the gate with `:nocov:`
and are covered only by those subprocesses.

**Rules added by the security review.**
* In a fallback mode the `load:` files are `require`d, never `load`ed: reloading a
  file another tool already compiled would reset that tool's counters and re-run
  its top-level side effects. Only a source that owns a fresh `Coverage.setup`
  `load`s.
* Every `load:` and `include:` entry is made absolute (`File.expand_path`) when
  the source is built, so a target that calls `Dir.chdir`, or a `$LOAD_PATH`
  search, cannot change what is loaded or measured. `include:` matching respects
  separators (`/a/lib` does not match `/a/lib_other`).
* A target may stop, suspend or consume `Coverage` itself. A failed read, stop or
  malformed result **degrades** the source (`degraded?`, mode
  `:fallback_degraded`) to caller-reported edges for the rest of the campaign; it
  never raises out of an execution, `finish` never raises, and a degraded
  campaign is never labelled `ruby-coverage-branches`.
* Files under the interpreter's library directories or `Gem.path` are plumbing
  and never measured. `excluded_files` and `status` (`all_excluded: true`) tell
  the caller when nothing it asked to load can yield a real edge.
* A failed `start` resets `mode` to `nil`. In a fallback mode a file is executed
  at most once: it is skipped if its path or realpath is in `$LOADED_FEATURES` or
  already instrumented by the running Coverage; `require` is used for `.rb` names
  only, anything else is `load`ed.
* **The tier is evidence, not a sticky flag**: `ruby-coverage-branches` needs real
  edges in the last 16 executions (`real_recent?`). A result that lacks a file the
  source loaded, or lacks its branch data (the target restarted Coverage), degrades
  the source; a degraded source never stops a run it can no longer prove is its own,
  and stops one it started only if it is still running. A target that clears the
  counters itself is labelled down by the evidence rule. Limit: a restart after the
  last read is not seen, and `finish` would stop that run. Prefix matching in
  `include:` uses POSIX separators (Ruby reports forward slashes on every platform).
* Require the target's dependencies **before** `start`: every execution costs
  `Coverage.result`, which is O(instrumented branches), and anything loaded after
  `setup` is instrumented (then filtered out).

**The label is honest.** `Report#coverage_tier` is `ruby-coverage-branches` when
at least one real edge was collected, else `caller-reported` when the target
called `edge`, else `black-box-uninstrumented`. The first passing seed is still
admitted in the black-box tier so there is something to mutate.

**Known limits.** Branch coverage sees no straight-line code (use `Handle#edge`).
Counters are process-global, so a thread a target leaves running can add edges to
a later trace (noise; coverage only decides admission). Collecting costs
O(measured branches) per execution.

### 4.3 Power schedule

`energy = calculate_energy(base, len(seed), seed.new_edges, cumulative.count)`
slots per turn. Tests pin energy rows including exact half-way ties.

## 5. Outcomes, Dedup, Minimisation

* Outcomes: `pass`, `crash{kind, frames}`, `timeout`, `oom`.
* **Crash** = any exception escaping the target except the engine's sentinels
  and the process's own: `SignalException`, `SystemExit` and
  `Timeout::ExitException` propagate out of the campaign; `NoMemoryError` is an
  OOM outcome. `SystemStackError` raised by the target is a **crash** of kind
  `SystemStackError` (frames collapsed), never an OOM.
* **Kind.** The class name via `Kernel#class` and `Module#name` **bound from
  their defining modules**, so a hostile `def class`, `def self.name`, `to_s`,
  `inspect` or `message` can neither raise nor lie. `::` becomes `.`, characters
  outside `[A-Za-z0-9._<>$/-]` become `_`, capped at 64; anonymous is
  `<anonymous>`. **The message is never read** (not `message`, `to_s`, `inspect`,
  `full_message`), not in the signature, not in any output. Class tests use the
  engine's own classes (`NoMemoryError === e`), never the exception's `is_a?`.
* **Frames.** From `Exception#backtrace_locations` bound from `Exception` and
  guarded: `nil`, a non-array or any error means **no frames**. Each location is
  `basename#label`, both sanitised as above, so **no frame contains `:`**
  (VER06 §9.4 item 6). Engine, interpreter, gem and `<internal:` frames are
  dropped, the stack is cut at the executor's own frame, consecutive identical
  frames collapse, the innermost 64 are kept, at most `MAX_WALK` (4,096)
  locations are inspected. No line number is used.
* `crash_signature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by ":"))`,
  8 lowercase hex; tests pin real values cross-checked against the Python oracle.
* **Timeout** = more than `Options#step_budget` calls to `h.tick` (default
  200,000, ceiling 100,000,000): a step budget, never wall-clock. **OOM** =
  `h.reserve(n)` passing `Options#mem_budget` (default 64 MiB, ceiling 1 GiB),
  **or** a real `NoMemoryError` (caught only around the target). `reserve` takes
  an `Integer`; non-positive reserves nothing.
* **Sticky exhaustion.** The first breach of either budget stores its sentinel;
  afterwards `tick` **and** `reserve` rethrow it. The sentinel is an `Exception`
  subclass, **not** a `StandardError`: a plain `rescue => e` misses it, but
  `rescue Exception` catches it and Ruby has no throwable a target cannot catch.
  The guarantee comes from stickiness plus the **post-run check**: after the
  target returns *or* raises, an exhausted handle means timeout or OOM — never a
  pass, never a crash.
* **Handle lifetime.** A **fresh `Handle` per execution**, retired in an
  `ensure` when the target returns. Every mutation takes the handle's `Mutex`.
  After retirement `edge` is a no-op and `tick`/`reserve` raise a *retired*
  sentinel, so a thread, fiber or `Enumerator` a target spawned touches only its
  own dead handle. The executor reads state **after** retiring. Each sentinel
  carries its **campaign token** and **epoch**; one stored and re-raised in a
  *later* execution (or a retired one) is a crash `stale-budget-sentinel`; one
  from another campaign is re-raised to its owner.
* **Unforgeable sentinels.** A sentinel carries no fields. Its owner, epoch and
  budget live in the issuing `CampaignToken`'s private table keyed by the object
  itself, and the executor trusts identity, never fields. A target that builds its
  own `BudgetExceeded.new` (or `dup`s a real one) has an object no table knows:
  an ordinary **crash**, never a timeout, and never aborts the campaign. One
  retired sentinel is memoised per handle, so a stale thread cannot mint objects.
* **Non-local exits abort the campaign.** A `break`, `return` or `throw` in the
  target is the caller's own code transferring control, and it propagates to the
  caller exactly as Ruby defines (`Fuzz.run` returns the `break` value, a `throw`
  reaches the caller's `catch`). The executor's `ensure` retires the handle (and
  `Fuzz.run`'s `ensure` finishes the coverage source) and **returns nothing**:
  a `return` there would also cancel `Thread#kill` and `Thread.exit`, which unwind
  as non-rescuable fatal exits, so a watchdog could not stop a campaign. Exception:
  a `break` from a proc whose defining method has returned raises `LocalJumpError`,
  an ordinary exception and so an ordinary crash. A custom stop exception raised
  into the thread with `Thread#raise` is likewise an ordinary exception, and so a
  crash; a caller that needs it to end the campaign should raise a
  `SignalException`, `SystemExit` or `Timeout::ExitException` (the pass-through
  list, `PROCESS_OWNED` plus `Timeout::ExitException`).
* **Sentinel table.** Provenance is a `WeakMap` keyed by sentinel identity (value:
  `epoch * 4 + budget code`, an Integer), guarded by one `Mutex`: a campaign that
  times out on every one of a million executions keeps only live sentinels, not
  every backtrace. A target that holds a sentinel keeps just that entry alive,
  which is what stale detection needs. Reaching sentinels through `ObjectSpace` is
  the target's own trust level.
* `Timeout::ExitException` is looked up at the moment of the rescue (it exists
  only once `timeout` is loaded), never at load time.
* Internal objects (`Campaign`, `Bitmap`, seed and crash entries, `SplitMix64`,
  `Handle`) have `inspect`/`to_s` that never show bytes or RNG state.
* **Input copy.** `dup` then `b`, **outside** the guarded region: an engine OOM
  on the copy is not a verdict about the target, and a target that mutates its
  input cannot alter a corpus seed.
* **Minimisation** (VER06 §5.3 as Python/Java): first chunk `max(1, len/2)`,
  exact signature only, block deletion then byte zeroing. `Options#minimize_budget`
  (default 2,000, ceiling 100,000) caps one crash and the campaign spends at most
  `max_iterations` minimisation executions in total. **Every candidate execution
  counts, accepted or rejected**, and the budget is checked before each.

**What Ruby cannot do (recorded).** The engine starts no thread and cannot
pre-empt a loop that never calls `tick`; CI needs an external wall-clock limit. A
real `NoMemoryError` may leave the VM unhealthy; `reserve` is the cooperative
hook. A target that rescues `Exception` and discards it hides *its own* crashes
(not the budgets: those are sticky).

## 6. Campaign Contracts (VER06 §9.1)

`Fuzz.run_outcomes(initial_corpus, options, exec)` is the outcome-level
campaign; `Fuzz.run(initial_corpus, options, coverage: nil) { |data, handle| }`
wraps a block in the executor. `Options` defaults: `max_iterations 1000`, `seed
42`, `max_input_len 4096`, `base_energy 10`, `minimize_budget 2000`,
`step_budget 200000`, `mem_budget 64 MiB`; every field validated (`TypeError` for
a non-`Integer`, `ArgumentError` out of range). Determinism, budget, honest
report, dedup and feedback are VER06e §6, with the same per-iteration draw order
(`bounded(len(corpus))`; per slot `1 + bounded(4)`, one sub-seed, then with more
than one seed `bounded(4)` and on 0 a donor index and two splits).

**Caps (CWE-400).** Seeds ≤ 10,000, each ≤ 1 MiB; distinct crash signatures ≤
1,000 (`crashes_dropped`); **retained bytes** (seeds plus crash inputs) ≤
`MAX_RETAINED_BYTES` (256 MiB): an over-cap admission is counted in
`admissions_dropped`, an over-cap crash record keeps no input
(`input_dropped? == true`, `minimized? == false`).

**Reports are inert.** `Report` and `CrashRecord` show no input bytes, seed or
corpus in `inspect`/`to_s`; collections are frozen copies. `clean?` needs at
least one iteration and no crash, timeout, OOM or drop.

## 7. Private Reporting (VER06 §6)

`format_tap_failure(n, target, failure_id, sig)` emits
`not ok N - <target> [<failure-id>:<hex8>]` only. `failure_id` must be one of the
five registered ids and `sig` exactly 8 lowercase hex digits, else
`ArgumentError`; `n` an `Integer` in `1..2^31-1`. The target is **truncated to
128 bytes before it is encoded**, then every byte outside `[A-Za-z0-9._/-]`
(control characters, newlines, multi-byte characters) becomes `_`.
`tap_failures(report, target, first_test)`: one line per crash signature, then
timeouts and OOMs, whose hash is `crash_signature("timeout" | "oom", [])`.

## 8. Corpus Runner

`Corpus.evaluate(cases)` returns `Report(evaluated, problems)`. Strictness and
caps are the shared ones (canonical ints, lowercase-even hex, 10,000
elements/steps, 1,000-digit ints, depth 64, 8 MiB, problems clipped to 400);
an unknown op, non-object case, missing/duplicate id or wrong field set is a
**problem**, never a skip, and every problem starts with the case id **escaped
with `Canon.escape`**. Invariants beyond expected values: flips, arithmetic and
boundary keep the length, splice length is prefix + suffix, admission equals set
difference, no TAP newline, privacy recomputed from bytes, every registered id
accepted, every op appears, 25 cases. The aggregate `mutate_sequence` work is
capped at 2^24 bytes **actually copied**: each call gets what is left as its cap
and is charged what it did, **including the initial copy and a call that hits its
cap**; a breach zeroes the aggregate so every later case fails at once.
`corpus_reject` is judged by `Tfv.valid?`.

## 9. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | real `Coverage` branches; caller-reported fallback; never touches a run it did not start |
| 2 | Frames (§5) | `basename#label`, no path/line/message, colon-free |
| 3 | `NoMemoryError` | OOM around the target only; `SystemStackError` is a crash |
| 4 | `mutate_sequence` work cap | `MAX_SEQUENCE_WORK`; Go caps steps only |
| 5 | Retained-bytes cap | as Java |
| 6 | Sentinel is catchable | by design; stickiness + post-run check |
| 7 | §9.4 items 7, 8, 9 | **open**, as in every lane |
| 8 | Hang protection | cooperative (`tick`); CI needs an external limit |
| 9 | Already-loaded code | only `load:`ed files are measured |
| 10 | Corpus weaknesses (PT-F1) | this lane replays Go's 820 oracle vectors and pins `sig_hash` values, energy ties, every interest/delta constant and unequal splice splits swapped |

## 10. Conformance Verification

* All 25 `fuzz-v1` cases through `Corpus.evaluate` (0 problems), per-op per-case
  corruption controls naming exactly that case, drop/truncation controls, and
  sabotage via the hooks.
* The 820 reference vectors parsed from Go's committed table.
* Campaign tests for every §5/§6 contract (dedup, the three sticky-exhaustion
  shapes, stale handle from a spawned thread, stale sentinel, OOM mapping,
  hostile exception class, privacy, retained cap, minimisation budget); real
  coverage discovery and the staircase in subprocesses.
* SimpleCov gate ≥ 95 % line, `standardrb` clean.
