# VER06o — Haskell Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05n](VER05n-haskell-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06i](VER06i-ruby-fuzz-engine.md), [VER06j](VER06j-c-fuzz-engine.md), [VER06n](VER06n-elixir-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-HS-FUZZ.

This specification defines the Haskell implementation of the Level 2 coverage-guided
fuzz engine. There is **one** Haskell engine: the module tree
`CodingAdventures.TestBootstrap.Fuzz.*` of the package
`code/packages/haskell/test-bootstrap`, beside `Prng`, `Pbt`, `Tfv` and `CorpusJson`
(VER05n). It reuses `Prng` (SplitMix64 and FNV-1a), `Pbt.Safe` (`tryRecoverable`:
every asynchronous exception is rethrown), `Tfv`, `Util` and the strict corpus
plumbing of VER05n (`StrictJson`, `Values`, `Canon`, `Escape`); it duplicates none of
them. It conforms to the `fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/cases.json`,
25 cases) byte for byte, replays the Go lane's 820 oracle vectors, and meets every
campaign contract of VER06 §9.1.

`Word64` wraps natively and `Integer` is unbounded, so the algorithms are those of
[VER06n](VER06n-elixir-fuzz-engine.md) (offsets and counts are `Integer`, wraps are
floored `mod`, flips work by parity). The hardening list is the one VER06f, h, i, j
and n converged on, **all of it in the first version**. The letter `o` is the next
free one after `n`. This document records only what GHC changes.

---

## 1. Architecture and placement

| Module (`Fuzz.`) | Role |
|---|---|
| `Limits` | the CWE-400 ceilings |
| `Mutate` | mutator algebra, interest lists, `mutateSequence` (+ counted form) |
| `Bitmap` | 65,536-edge map, `corpusAdmit`, `calculateEnergy` |
| `Signature` | `crashSignature`, kind normalisation, `formatTapFailure`, `tapFailures` |
| `FailureIds` | the closed failure-id vocabulary |
| `Handle` | per-execution coverage and budget handle |
| `Types` | `FuzzOptions`, `Outcome`, `FuzzReport`, `CrashRecord`, `FuzzVerdict` (prefixed `Fuzz` so they never clash with the PBT `Options` and `Verdict`) |
| `Executor` | one execution in its own thread; classification |
| `Engine` | minimiser and campaign (`runOutcomes`) |
| `Hooks` | sabotage hooks for the negative controls |
| `Corpus` | production runner for the shared corpus |
| `Fuzz` | the public facade: `run`, `runOutcomes`, `executor`, `verdict` |

The engine reads no file, environment or network. It **does** fork one thread per
target execution, hands its result back through an `MVar`, and bounds every wait with
the RTS wall-clock timer (`System.Timeout`; no calendar time is read), so
`required_capabilities.json` declares `thread:spawn`, `thread:message` and `time:read`.
**Zero new dependencies**: GHC boot packages the library already uses (`base`,
`bytestring`, `containers`, `deepseq`, `stm`, `text`). No HPC and no `unix`.

## 2. PRNG and stream alignment

`Prng.bounded` already draws a word for `n == 1` and rejects below
`t = (2^64 - n) mod n` (unsigned negation, VER05 §2.2). Pinned tests force the
rejection branch with an inverted SplitMix64 finalizer (a seed whose first word is a
chosen value): a draw below the threshold is redrawn, one equal to it is kept, and
the `2^63 + 1` boundary. The corpus never reaches a rejection, so these are required.

## 3. Mutator algebra

Exactly VER06 §3 and VER06n §3. Inputs and outputs are strict `ByteString`s; offsets
wrap with floored `mod`; widths are 1, 2, 4, 8 (anything else, or a buffer shorter
than the width, returns the input unchanged); **flips work by parity and word-wise**:
bit `i` is bit `i mod 8` of byte `i div 8`, LSB first; every unit is hit
`count div units` times, and the first `count mod units` units from the wrapped start
once more. The work is O(len) for any count, including a 1,000-digit one. **No
`Integer` is ever built from input bytes** (the Elixir lane crashed on a
`2^(8 x 1 MiB)` integer): inversion is `B.map complement`, a range flip masks the two
edge bytes and complements the middle, and arithmetic and boundary values are
reduced modulo `2^(8w)` on a width of at most 8 bytes.

`mutateSequence(data, seed, steps, maxLen)` has the Python/Java/Ruby/Elixir draw
order: per step `bounded 7`, then that step's draws; an empty buffer is refilled with
`00` before each step; an insert or duplicate over `maxLen` is skipped **after** its
draws; steps 4 and 6 with `n <= 1` draw nothing. The seed is reduced modulo `2^64`.

**Work cap (CWE-400), charging the actual work.** `steps` above 1,000,000 or an input
above 1 MiB is an `ArgumentError` **before anything is allocated**. The bytes a call
copies are charged: the initial copy, then the current length before each step (inserts
grow the buffer). `mutateSequenceCounted` takes a cap (and `mutateSequenceWithCap` exposes it as a test seam) and returns `WorkOk bytes work`
or `WorkExhausted work`, stopping **at the step that would cross it**; **a failed call
is charged too**. The per-call cap is `2^28`; the corpus runner's aggregate is `2^24`:
each case is handed what is left, is charged what it did (failed calls included) and a
**breach zeroes the remaining allowance**, so every later case fails at once.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1 and VER06n §4.1. The map is a value (`IntSet` of edge ids reduced with
floored `mod 65536`, so `-1` is `65535`). `corpusAdmit` counts the *unique* novel
edges over the **unreduced** ids. `calculateEnergy base len new total = max 1
(floor (base' * (1 + new/total) / sqrt (max 1 len) + 0.5))` with bonus `1.0` when
`total == 0`, `base' = max 1 base`, half **away from zero** (`floor (x + 0.5)`; not
banker's rounding): `7.5 -> 8`, `2.5 -> 3`. Pinned energy rows include exact ties (§9).

### 4.2 The coverage source: explicit edges through a handle (decision)

VER06 §7 names a platform source per lane. For GHC it is HPC (`-fhpc`, `.tix`).
**This lane does not use HPC in the hot path:**

1. HPC counters live in a process-global `.tix` table written at exit and readable
   only through `Trace.Hpc.Tix` as **cumulative per-module tick vectors**. There is no
   "counters since the last execution" and no safe reset: the package's own coverage
   gate (`cabal test --enable-coverage`) uses the same table, so a reset between
   executions would zero the coverage CI is measuring.
2. Reading is O(instrumented ticks) per execution, and it requires the target to be
   compiled with `-fhpc` into the same process.
3. Tick data is global across threads: a thread a target left running pollutes every
   later trace, and a thread the engine kills can die mid-update.

The engine therefore takes **target-reported edges through a handle**, as the Ruby
and Elixir lanes do: the target calls `edge h id` (`edgeId "label"` gives a stable
FNV-1a id; ids are reduced mod 65,536) at the branches it cares about. One atomic
update per edge; nothing is global; no instrumented build. Honest labelling:
`coverageTier` is `caller-reported` if the target called `edge` at least once during
the campaign, else `black-box-uninstrumented` (the first passing seed is still
admitted, so there is something to mutate). An HPC-based source is a possible future
optional tier (`ghc-hpc-ticks`) for a standalone run that owns the `.tix` table.

## 5. Threads, outcomes and the budget

### 5.1 One thread per execution, under `timeout`

Every execution runs in **its own forked thread** (`forkIO`), under the engine's
`System.Timeout.timeout` (`timeoutMs`, default 5,000, ceiling 60,000). The worker is
created with exceptions masked and unmasks only around the target, and it always fills
a result `MVar`, so the engine's wait never depends on the target behaving. On expiry
the engine `killThread`s the worker from a helper thread and waits a bounded
`killWaitMs` for it to finish; a worker stuck in an uninterruptible foreign call is
**leaked, never waited for**. The deterministic bound is the cooperative **step
budget** (`tick`); the wall clock exists because a non-yielding loop must still be
stopped. `-fno-omit-yields` (already on the shared stanza) makes a loop that never
allocates reach a safe point, so `killThread` and `timeout` can interrupt it.

Exceptions are caught **only through `tryRecoverable`**: every asynchronous exception
is rethrown, never swallowed. The one asynchronous exception the worker handles itself
is its own `AllocationLimitExceeded` (Memory, below). A thread **the target forks** is
outside the engine's reach (GHC has no thread groups); it can only touch its own
retired handle (§5.3), and it may outlive the execution. This is recorded in §10.

**Memory (decision: approach (a), the per-thread allocation limit).** GHC has no
per-thread heap limit. The worker calls `setAllocationCounter limit` and
`enableAllocationLimit` (`GHC.Conc`), `limit = max memBudget 1 MiB`. When the thread's
allocation counter goes below zero the RTS throws `AllocationLimitExceeded` to it.

*Semantics, stated honestly:* the limit counts **total bytes allocated** by the
thread, not residency: a target that allocates and drops 2 GiB in a loop hits a 1 GiB
limit although its live heap is tiny, and memory allocated by another thread or by
foreign code is not charged. Accuracy is about 4 KiB. The default `memBudget` is
therefore **256 MiB of allocation per execution** (not Elixir's 64 MiB of heap), and
the same number caps cooperative `reserve`. A target needing more raises `memBudget`.

*Classification, own limit only.* An `AllocationLimitExceeded` is `oom` only when it is
**this worker thread's own limit tripping**. The worker catches it, then reads its own
`getAllocationCounter`: after a real trip the RTS has reset the counter to its small
re-initialisation allowance (100 KiB by default), so a counter below `limit / 8` means
the thread consumed essentially all of it. A target that merely *throws* the exception
value itself (`throwIO AllocationLimitExceeded`) while its counter is nearly full is
**a crash of kind `AllocationLimitExceeded`**, never an oom. A target that catches the
trip and carries on is still judged at the end: the worker disables the limit after the
target ends and reads the counter once more, and a counter below `limit / 8` is `oom`
(a swallowed breach is sticky evidence, like the step budget). The fallback (b), a
polling thread, is not used: nothing in GHC lets a poll read one thread's allocation.
Residency limits for the whole process remain RTS flags (`-M`) and are outside the
engine's reach.

### 5.2 Outcomes, classification, sticky exhaustion

* Outcomes: `Pass`, `Crash kind frames`, `Timeout`, `Oom`.
* **Crash** = any synchronous exception escaping the target (`error`, `throwIO`,
  arithmetic, pattern failure, `ExitCode`, ...). The kind is the exception's **type
  constructor name** (`tyConName . typeRepTyCon . typeOf`), never `displayException`,
  never `show` of anything the target supplied; every character outside
  `[A-Za-z0-9._<>$/-]` becomes `_`, capped at 64; anything the classifier cannot
  read is `<unreadable>`. Classification never throws. GHC exceptions carry no
  portable stack, so **frames are empty** (all `ErrorCall`s of one target are one
  crash; recorded in §10).
* `crashSignature kind frames = lower32 (fnv1a64 (kind ++ ":" ++ join ":" frames))`
  as 8 lowercase hex digits; real values are pinned (§9).
* **Exhaustion is sticky.** The first breach of the step or memory budget records
  itself (first wins) and throws a `BudgetSentinel`; afterwards **every** `tick` and
  `reserve` throws it again, **whichever budget breached**: a later `reserve` throws
  after a STEPS breach and a later `tick` after a MEMORY breach (tested both ways).
  Haskell cannot make an exception uncatchable, so the guarantee is stickiness plus
  the **post-run verdict**: after the execution ends (returned, raised or killed) the
  engine **retires the handle and then** reads the exhausted state and cross-checks
  the counters against the budgets; a breach means `Timeout` (steps) or `Oom` (memory)
  **whatever the target did with the exception**. A target that caught the sentinel and
  returned, raised something ordinary or exited normally is never a pass and never a
  crash.
* **Sentinels carry the execution's token.** `BudgetSentinel token which`. The engine
  never trusts a sentinel it merely sees: one thrown while the state says the budget is
  not exhausted (forged, or retained from another execution) is a crash of kind
  `stale-budget-sentinel`, never a timeout.

### 5.3 Fresh handle per execution, retired when the execution ends

Each execution gets a **fresh `Handle`**: one `IORef` holding an immutable strict
state, updated only with `atomicModifyIORef'`, so any thread holding the handle acts on
the same state atomically and no lock exists that a killed thread could leave held. The
state is: live or retired, the identity token hash (from `Data.Unique`), the step and
memory budgets, the step and memory counters, the sticky exhausted flag, an edge-call
counter and the **1,024-word edge bitmap** (a 1,024-entry `IntMap Word64`). Every call
(`edge`, `tick`, `reserve`) first checks `live` **and** that the handle's token matches
the stored one, so a handle rebuilt with another token is inert. After retirement:

* `edge` is a **no-op** and never touches the bitmap;
* `tick` and `reserve` **throw the retired sentinel** (a crash of kind
  `stale-budget-sentinel` if it reaches a later execution);
* the budgets, counters and edges of a handle that was forwarded to another thread,
  stored in an `IORef` or `MVar`, or retained by the target **cannot influence any
  later execution or the campaign**: each later execution has its own state. A test
  forwards and retains a handle and shows it inert.

**Honest limit:** inside one GHC process a hostile target can reach the handle's
mutable state (the `IORef` is a heap object it can be handed, and `unsafe*` and
`Foreign` can write anywhere). The engine cross-checks the counters against the budgets
after the run (`stepsUsed > stepBudget` is a timeout even if the flag was cleared), which
catches casual tampering, not a determined in-process attacker. Targets are trusted
not to attack the harness, as in every other lane.

### 5.4 Isolation limits (recorded)

* **The wall clock holds only if the target and every package it calls are compiled with
  `-fno-omit-yields`.** GHC interrupts a thread at a safe point only; a non-allocating
  loop, or a long non-yielding call (bignum `^` with a huge exponent, an unsafe FFI call)
  in code built without the flag hangs the whole process, and the CI job timeout is the
  only backstop: fail closed, but no verdict. Users add `ghc-options: -fno-omit-yields` to
  their test stanzas. An out-of-process runner is future work.
* **Leaked workers.** A worker that never answers the kill (a masked loop) is counted per
  campaign. After the first leak later kill waits are `leakedKillWaitMs` (500 ms) instead
  of `killWaitMs`, and after `maxLeakedWorkers` (4) the campaign is aborted: the report has
  `leakedWorkers > 0`, is never clean, the verdict reason is `workers-leaked` and the public
  line is `fuzz-timeout` with the signature of `workers-leaked`. (No new registered failure
  id: the vocabulary is shared by every lane.)
* **"Leaked" means "did not answer the kill within the wait"**, so a legitimate target
  whose cleanup takes longer than that and that times out four times is aborted, fail
  closed.
* **The cap holds everywhere an execution happens:** the minimiser's probe and passes
  ask the abort flag before every candidate (an aborted campaign probes nothing more), and
  `verdict` re-verification shares one leak counter with its executor (`verdictTracked` +
  `executorTracked`): a report that already leaked 4 workers re-verifies nothing (all
  `Unverified`), and re-verification itself stops, `Unverified` for the rest, at the cap.
* The allocation limit is **not delivered under `mask`**, which honest `modifyMVar` and
  `bracket` acquire code also is; such a region can overshoot `memBudget` until it unmasks.
* A hostile target can call `setAllocationCounter` or `disableAllocationLimit` on its own
  thread to fake or hide an oom: the trusted-target model.
* **Retained inputs are copies.** A `ByteString` slice pins its parent buffer, so exactly
  the three retention points (corpus admission, a novel crash's minimal input, a repeat
  crash's smaller witness) `B.copy` and charge the copy; executed candidates are not
  copied (each retention copy is pinned by its own test, which fails if it is removed):
  retained-bytes accounting can no longer under-count by the slice-to-parent ratio.
  `mutateSequence` keeps `maxLen = 1 MiB` as in the Ruby and Elixir lanes; `maxInputLen`
  truncates afterwards.
* `Handle` is exported abstractly; its representation and the forge seams live in
  `Fuzz.Handle.Internal` (unsafe: it breaks the engine's guarantees, tests only, hidden from the haddock home).

* A thread the target forks, a `forkOS` thread, or a foreign call that never returns is
  outside the engine's reach (leaked, not waited for).
* A target that **masks** asynchronous exceptions around a non-allocating,
  non-interruptible loop defers `killThread` and the allocation limit. The execution is
  then reported `Timeout` after `killWaitMs` and the worker is leaked.
* `reserve` and `tick` are cooperative; the allocation limit and the wall clock are the
  backstops.
* The worker reports through its own result `MVar`; the engine reads only the
  execution's own `MVar`, so a target cannot forge another execution's result. A
  deliberately hostile target can still write the engine's data structures (§5.3).

## 6. Campaign contracts (VER06 §9.1)

`runOutcomes initialCorpus options exec` is the outcome-level campaign (`exec ::
ByteString -> IO (Outcome, [Integer])`); `run initialCorpus options target` wraps a
target `ByteString -> Handle -> IO ()` in the executor of §5. Per iteration the
stream draws: `bounded (len corpus)` for the seed; per energy slot `1 + bounded 4`
steps, one `nextU64` sub-seed, then, with more than one seed, `bounded 4` and on 0 a
donor index and two split draws. `FuzzOptions` (fields `fzMaxIterations` ...): `maxIterations 1000`, `seed 42`,
`maxInputLen 4096`, `baseEnergy 10`, `minimizeBudget 2000`, `stepBudget 200000`,
`memBudget 256 MiB`, `timeoutMs 5000`, `ci CiUnset`; every field is validated
(`ArgumentError` out of range).

**Caps (CWE-400), all enforced before the allocation they bound.** Seeds at most
10,000, each at most 1 MiB; a candidate is truncated to `maxInputLen` (itself at most
1 MiB); distinct crash signatures at most 1,000 (`crashesDropped`); **retained bytes**
(corpus seeds plus crash inputs) at most **256 MiB**: an over-cap admission is counted
in `admissionsDropped`, an over-cap crash record keeps no input. The check is made
**before** the bytes are stored.

**Minimisation (VER06 §5.3).** Block deletion from a first chunk of `max 1 (len div 2)`,
halving to 1, then byte zeroing; exact signature only; `minimizeBudget` caps one crash
and the **whole campaign** spends at most `maxIterations` minimisation executions.
**Every candidate execution counts, accepted or rejected**, and the budget is checked
before each one. The halves are pinned (§9).

**Honest report.** `isClean` needs at least one mutation iteration and no crash,
timeout, oom or drop. **Zero executions is never a pass.**

**Verdicts, CI mode and re-verification.** Before a crash is reported its minimal input
is run once more through the executor (one execution per crash, at most 1,000); the
verdict records `Reproduced`, `Flaky` (the signature did not come back; still
failing, fail closed) or `Unverified` (no input kept) — never `Passed`. `passed` only
for a clean report. `CiOn` and `CiUnset` (an unset mode is CI, fail closed) carry **no
seed and no witness**; only `CiOff` fills them.

## 7. Private reporting (VER06 §6, §9.3)

`formatTapFailure n target failureId sig` emits `not ok N - <token>
[<failure-id>:<hex8>]` only. `failureId` must be registered and `sig` exactly eight
lowercase hex digits, else `ArgumentError`; `n` in `1..2^31-1`. **The target token is
the byte grammar of VER06 §9.3 and every other lane:** the UTF-8 bytes of the target
are truncated to 128 **bytes** first, then every **byte** outside `[A-Za-z0-9._/-]`
becomes `_` (a multi-byte character becomes one `_` per byte; a cut in the middle of
one leaves a single `_`). Nothing can forge or hide a line or start a `#` directive.
Pinned cross-lane values were produced by running the Ruby lane's `format_tap_failure`
on the same inputs (§9). `tapFailures report target firstTest` gives one line per crash
signature, then one for timeouts and one for ooms, hashed as
`crashSignature "timeout" []` and `crashSignature "oom" []`.

## 8. Corpus runner

`Fuzz.Corpus.evaluate` returns the evaluated count and the problems. Strictness and
caps are the shared ones (canonical ints, lowercase even hex, 10,000 elements and
steps, 1,000-digit ints, depth 64, problems clipped to 400); an unknown op, a
non-object case, a missing or duplicate id or a wrong field set is a **problem**, never
a skip, and every problem about a case starts with the case id escaped by `Escape`.
Invariants beyond expected values: flips, arithmetic and boundary keep the length,
splice length is prefix + suffix, admission equals set difference, no TAP newline,
privacy recomputed from the bytes, every op appears, a registered id is accepted and
an unregistered one rejected, exactly 25 cases. Every engine entry point the runner
calls goes through `Hooks` (a record of functions; a test installs a wrong one and the
run must fail). The Go lane's committed vector table is **parsed from that file** by
the conformance suite (no copy): all 820 rows must match.

## 9. Tests, pins and negative controls

* `fuzz-v1`: 25 evaluated, 0 problems; 820 of 820 vectors.
* **Negative controls (the runner must FAIL)** for every case and every corruption its
  op defines (wrong output, wrong coverage count, flipped admission, wrong duplicate
  verdict, flipped validity, a tap line changed by a character, a valid TFV in place
  of a reject text); every problem **starts with that case's id**; dropped, duplicated
  and truncated corpora fail; unknown ops and bad field sets fail; every sabotage hook
  makes the run fail.
* **Pinned values for the corpus blind spots**, cross-checked against the Ruby and
  Elixir lanes: every interest value and delta; unequal splice splits swapped; energy
  rows including exact ties (`10,4,3,6 -> 8`, `3,4,0,0 -> 2`, `5,4,0,0 -> 3`,
  `25,100,0,0 -> 3`, `1,4,0,0 -> 1`); real `sig_hash` values (`panic eval step ->
  4eca7ae8`, `segv alloc write -> a0aa3aa2`, `timeout -> 4e61ea76`, `oom ->
  7146a410`, `stale-budget-sentinel -> 8a20b3bf`, `ArgumentError a.rb#x ->
  c93b2322`); the rejection draw in bounded sampling (a seed whose first word is below,
  equal to, and the `2^63 + 1` boundary); the minimiser's bisection halves (the exact
  candidate sequence for a 9-byte, a 1-byte and a zeroing case).
* **TAP pins** from the Ruby lane: `a#b -> a_b`; `a b c -> a_b_c`; `\n` and `\r` ->
  `_`; e-acute + U+202E + `x` -> `_____x`; 200 `x` -> 128 `x`; 127 `x` + e-acute +
  `y`s -> 127 `x` + one `_`; empty target -> `not ok 3 -  [fuzz-crash:3a8f19b2]`.
* **Hardening tests:** retained and forwarded handles are inert; sticky exhaustion in
  both directions (steps then reserve, memory then tick), judged after retirement even
  when the target swallowed the sentinel and returned, raised, or exited; a forged
  sentinel is a crash; an allocation bomb is an `oom` through the thread's own limit; a
  forged `AllocationLimitExceeded` is a crash, not an oom; a hang is killed by the wall
  clock; a non-allocating loop is interrupted; async exceptions are rethrown by
  `tryRecoverable`; a message-hostile exception is classified by type only; CI mode
  carries no seed or witness; zero executions is not clean; the retained-bytes cap,
  the input cap, the campaign-wide minimisation budget and the aggregate
  `mutateSequence` cap (a breach zeroes the allowance) are exact; a 1 MiB flip with a
  1,000-digit count is cheap and builds no big integer.
* **Bounded tests:** every test runs under the harness's `timeout`, and every wait in
  a test is bounded.
* At least 95% HPC expression coverage over the whole package.

## 10. Divergences and open items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | explicit edges through the handle; HPC rejected for the hot path, a future `ghc-hpc-ticks` tier |
| 2 | Memory | per-thread **allocation** limit (total bytes, not residency), classified only from the worker's own limit; default 256 MiB |
| 3 | Timeouts | step budget **and** a wall clock (`timeout` + `killThread`) |
| 4 | Frames | none: GHC exceptions have no portable stack, so a crash is its exception type |
| 5 | Leaked workers (masked loops), missing `-fno-omit-yields` in the target, threads the target forks, masked loops, uninterruptible foreign calls | outside the engine's reach; the worker is leaked, not waited for |
| 6 | Sentinel is catchable | by design; stickiness + post-run verdict |
| 7 | Retained-bytes cap, input cap, campaign-wide minimisation budget, aggregate sequence cap | as Java, Ruby and Elixir, `2^24` aggregate, `2^28` per call |
| 8 | CI mode | `CiUnset` is CI; no seed or witness; crash witnesses re-verified |
| 9 | VER06 §9.4 items 7, 8, 9 | **open**, as in every lane |
| 10 | Corpus weaknesses (PT-F1) | this lane replays Go's 820 vectors and pins sig_hash values, energy ties, constants, unequal splices, the rejection draw and the bisection halves |
| 11 | Build | `BUILD` and `BUILD_windows` run `cabal test all` (suites `spec`, `pbt-v1-conformance`, `fuzz-v1-conformance`); `haskell/test-bootstrap` is a repository-input consumer of `fuzz-v1` and of Go's vector table in the Go build tool, failing closed when the package is absent |
