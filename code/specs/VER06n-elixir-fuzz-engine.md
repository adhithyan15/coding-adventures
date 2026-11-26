# VER06n — Elixir Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05m](VER05m-elixir-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06i](VER06i-ruby-fuzz-engine.md), [VER06j](VER06j-c-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md), [VER06h](VER06h-csharp-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-EX-FUZZ.

This specification defines the Elixir implementation of the Level 2 coverage-guided
fuzz engine. There is **one** Elixir engine: module
`CodingAdventures.TestBootstrap.Fuzz` of the package
`code/packages/elixir/test_bootstrap`, beside `Prng`, `Pbt`, `Tfv`, `CorpusJson`
and `Safe` (VER05m). It reuses `Prng` (SplitMix64 and FNV-1a), `Safe` (catch-all
calls), `Tfv`, and the strict corpus plumbing of VER05m (`CorpusJson.StrictJson`,
`Values`, `Canon`, `Clip`); it does not duplicate any of them. It conforms to the
`fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for
byte, replays the Go lane's 820 oracle vectors, and meets every campaign contract
of VER06 §9.1.

Elixir has bignums and needs explicit masking like Ruby, so the algorithms follow
[VER06i](VER06i-ruby-fuzz-engine.md); the hardening list is the one VER06f, VER06h,
VER06i and VER06j converged on, **all of it in the first version**. The letter `n`
is the next free one after `m` (the Elixir PBT engine). This document records
only what the BEAM changes; the shared reasoning is in the documents above.

---

## 1. Architecture and placement

| Module (`Fuzz.`) | Role |
|---|---|
| `Limits` | the CWE-400 ceilings |
| `Mutate` | mutator algebra, interest lists, `mutate_sequence` (+ counted form) |
| `Bitmap` | 65,536-edge map, `corpus_admit/2`, `calculate_energy/4` |
| `Signature` | `crash_signature/2`, frame and kind normalisation, `format_tap_failure/4`, `tap_failures/3` |
| `FailureIds` | the closed failure-id vocabulary |
| `Handle` | per-execution coverage and budget handle |
| `Executor` | one execution in its own monitored process; classification |
| `Options`, `Report`, `Verdict` | validated options, inert reports, CI-redacted verdicts |
| `Engine` | minimiser and campaign (`run_outcomes/3`) |
| `Hooks` | sabotage hooks for the negative controls |
| `Corpus` | production runner for the shared corpus |
| `Fuzz` | the public facade: `run/3`, `run_outcomes/3`, `executor/2`, `verdict/3,4` |

The engine reads no file, clock, environment or network. It **does** spawn one
process per target execution (§5), so `required_capabilities.json` declares
`thread:spawn` and `thread:message` (the BEAM's processes and mailboxes).
**Zero dependencies**: stdlib and OTP only (no `:cover`, no `:crypto`).

## 2. PRNG and stream alignment

`Prng.bounded/2` already draws a word for `n == 1` and rejects below
`t = rem(2^64 - n, n)` (VER05 §2.2; VER05m §3). The generator is an immutable
value, so every function that draws returns the new generator. The per-iteration
draw order is exactly VER06i §6 (see §6 here). Pinned tests force the rejection
branch with an inverted SplitMix64 finalizer (a seed whose first word is a chosen
value), as `prng_pins_test.exs` does: a draw below the threshold is redrawn
(`n = 3`: raw 0), a draw equal to the threshold is kept, and the `2^63 + 1`
boundary. The fuzz-v1 corpus never reaches a rejection, so these are not optional.

## 3. Mutator algebra

Exactly VER06 §3 and VER06i §3: inputs are binaries, outputs are new binaries,
offsets wrap with floored `Integer.mod/2` (BEAM `rem/2` is truncated, so it is
**never** used for a wrap), widths are 1, 2, 4, 8 (anything else, or a buffer
shorter than the width, returns the input unchanged), flips are applied **by
parity** in O(len) for any count (the buffer is one little-endian integer;
bit `i` is bit `i mod 8` of byte `i div 8`, LSB first), and arithmetic and
boundary values are reduced modulo `2^(8w)` with `Bitwise.band/2`, since a BEAM
integer never wraps by itself. There is no long/big split: the corpus's
1,000-digit values are exact.

`mutate_sequence(data, seed, steps, max_len)` has the Python/Java/Ruby draw order
(VER06i §3): per step `bounded(7)`, then that step's draws; an empty buffer is
refilled with `<<0>>` before each step; an insert or duplicate over `max_len` is
skipped **after** its draws. Wrong types (non-binary data, non-integer offsets,
booleans and floats included) raise `ArgumentError`.

**Work cap (CWE-400), charging the actual work.** `steps` above `max_iterations()`
(1,000,000) or an input above `max_input_bytes()` (1 MiB) is an `ArgumentError`
**before anything is allocated**. The bytes a call copies are charged: the initial
copy, then the current length before each step (inserts and duplicates grow the
buffer, so `steps x initial length` would undercount). `mutate_sequence_counted/5`
takes a cap and returns `{:ok, bytes, work}` or `{:work_exhausted, work}`: a call
that would pass the cap stops **at the step that would cross it** and reports the
work done up to there; **a failed call is charged too**. The per-call cap is
`2^28`; the corpus runner's aggregate cap is `2^24` bytes: each case is handed what
is left, is charged what it did (failed calls included) and **a breach zeroes the
remaining allowance**, so every later case fails at once without running.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1 and VER06i §4.1. The map is a value (`Bitmap`): a `MapSet` of
reduced edge ids (`Integer.mod(edge, 65_536)`, floored, so `-1` is `65_535`) with
`raw/1` rendering the 8,192-byte map on demand (bit `e` is bit `e mod 8` of byte
`e div 8`). `corpus_admit(existing, trace)` counts the *unique* novel edges over
the **unreduced** ids. `calculate_energy(base, len, new, total) = max(1,
floor(base * (1 + new/total) / sqrt(max(1, len)) + 0.5))` with bonus `1.0` when
`total == 0`, `base` clamped to at least 1, and **half rounds away from zero**
(`floor(x + 0.5)`; `Float.round/1` is not used, so `7.5 -> 8`, `2.5 -> 3`,
`1.5 -> 2`). Pinned energy rows include exact ties (§9).

### 4.2 The coverage source: explicit edges through a handle (decision)

VER06 §7 names `:cover` for Elixir/Erlang. **This lane does not use it in the hot
path.** The decision, and why:

1. `:cover` instruments **modules**, compiles them into a process-global cover
   server, and reports **line-level, cumulative counters** that are only readable
   by analysis calls that walk an ETS table per module. There is no cheap
   "counters since the last execution" and no reset that is safe for the tool that
   is already running: `mix test --cover` (this package's own coverage gate) and
   the engine would **share one global cover server**, so `:cover.reset/0` between
   executions would zero the coverage the user's CI is measuring, and
   `:cover.compile_beam/1` re-compiles a module under a running test run.
2. Reading is O(instrumented lines) per execution through ETS, and instrumented
   code runs several times slower: the per-execution cost the engine can afford is
   microseconds, not a table scan.
3. Cover data is **keyed by module and line** across all processes, so a process a
   target left running pollutes every later trace, and a process we kill for a
   timeout can die mid-update of the cover server's state.

The engine therefore takes **target-reported edges through a handle**, as the Ruby
and Go lanes' `caller-reported` tier does: the target calls `Handle.edge(h, id)`
(`Handle.edge_id("label")` gives a stable FNV-1a id; ids are reduced mod 65,536)
at the branches it cares about. The cost is one `:atomics` compare-and-swap per
edge; nothing is global; a killed process cannot corrupt anything; and the target
needs no instrumented build. Honest labelling: `Report.coverage_tier` is
`caller-reported` if the target called `edge/2` at least once during the
campaign, else `black-box-uninstrumented` (the first passing seed is still
admitted, so there is something to mutate). **A `:cover`-based source is recorded
as a future optional tier** (`beam-cover-lines`) for a standalone fuzz run that
owns the cover server; it would be a separate module reading `:cover.analyse/3`
between executions and is out of scope here (§10).

## 5. Processes, outcomes and the budget

### 5.1 One monitored process per execution

Every target execution runs in **its own process**, spawned with
`:erlang.spawn_opt/2` and the options `:monitor` and `{:max_heap_size, %{size: W,
kill: true, error_logger: false}}` (`W = mem_budget / 8` words, clamped to at
least 1,024 words). `:off_heap` mailboxes are deliberately **not** used (§5.1, Memory). The engine process
(the caller) holds the monitor and waits in a `receive` that **always has an
`after`**: executions are bounded and no hang can survive.

* **Normal end.** The execution process computes its outcome and sends it as one
  message `{ref, outcome}` to the engine, then ends. A message and the later
  `:DOWN` from the same process are ordered, so after the `:DOWN` the engine does a
  `receive ... after 0` for the outcome.
* **Timeout.** `Options.timeout_ms` (default 5,000, ceiling 60,000) is a
  **wall-clock** bound enforced by the engine's own `receive ... after`: on expiry
  the execution process is killed with `Process.exit(pid, :kill)` and the outcome
  is `timeout`. The deterministic bound is the cooperative **step budget**
  (`Handle.tick/2`, `Options.step_budget`, default 200,000, ceiling 100,000,000);
  the wall-clock bound exists because a BEAM engine, unlike Ruby's, can preempt a
  loop that never calls `tick`. After the kill the engine waits for the `:DOWN`
  with a second bounded `receive`; if even that expires it demonitors, flushes and
  reports the timeout anyway (a process stuck in a native call is leaked, never
  waited for).
* **Memory (best effort).** `max_heap_size` with `kill: true` kills the process with
  reason `:killed` when its heap passes the budget, but it **does not count
  reference-counted binaries**, and an `:off_heap` mailbox is invisible to it. So the
  engine also **polls**: every 5 ms of waiting it sums `Process.info(pid, [:memory,
  :binary])` (the process's own memory, which includes an on-heap mailbox, plus the
  size of every referenced refc binary, deduplicated by binary id) and, past
  `max(mem_budget, 1 MiB)`, kills the process and reports `oom`. Dropping `:off_heap`
  is the choice that keeps the mailbox inside both `max_heap_size` and the poll. The
  budget is **best effort**: an allocation finished inside one poll interval is seen
  at the next poll, and a process that ends between polls is never measured. The
  outcome is `oom` **only** when the `:DOWN` is the one of **this execution's monitor
  reference and process** with reason `:killed`, or when the poll killed it; any other
  `:DOWN` or `:EXIT` in the engine's mailbox is ignored by the selective `receive`. A
  target that kills its own execution process (`Process.exit(self(), :kill)`) is
  indistinguishable on the BEAM and is classified `oom` too: it is **never a pass
  and never a crash of the campaign** (fail closed; recorded in §10). The cooperative
  `Handle.reserve(h, n)` charges `mem_budget` the same way; all routes give `oom`.
  The wall clock is a real deadline (§5.4).
* **Descendants.** A process the target links to (`spawn_link`, `Task.async`) dies
  when the target is killed or crashes, but a linked child **survives the target's
  NORMAL exit** (a link only propagates abnormal exits). A process the target spawns
  **unlinked** is outside the engine's reach (the BEAM has no process groups; no
  process-list diffing is attempted); it can only touch its own retired handle (§5.3).
* **Results and forgery.** The execution reports `{tag, pid, outcome}` and the engine
  matches the pid as well as the tag. A deliberately hostile target can recover the
  tag from its own backtrace and forge a result: targets are trusted not to attack the
  harness inside the VM, the same limit as the other lanes.
* **A dedicated engine process.** `Fuzz.run/3` (and the re-verification of
  `verdict/4`) runs in a **dedicated engine process**, never the caller's: the target's
  parent is that process, so anything a target sends "to its parent" cannot reach the
  caller's mailbox. After each execution the engine drops stray messages
  (`receive _ -> ... after 0`, at most 50,000 per drain). The caller gets the result
  through a monitored reply with a bounded `receive` (the bound is the number of
  executions times the wall clock plus the kill wait); an exception in the engine
  process is re-raised in the caller.
* **Input.** An input over `Limits.max_input_bytes/0` never reaches an execution
  (§6); a binary is immutable, so a target cannot alter a corpus seed.

### 5.2 Outcomes, classification, sticky exhaustion

* Outcomes: `:pass`, `{:crash, kind, frames}`, `:timeout`, `:oom`.
* **Crash** = `raise`, `throw` or `exit` escaping the target (VER05m §3): all three
  are caught with a catch-all `catch kind, reason` (via `Safe`) **in the execution
  process**, so an exception struct or a thrown term is classified there and only
  a small term crosses to the engine. **The message is never read**: not
  `Exception.message/1`, not `inspect/1`, not `to_string/1`, not any protocol of a
  target-supplied value. The kind is derived from the **error class and module
  only**:

  ```text
  error + exception struct   "error." <> module name without "Elixir."
  error + an Erlang reason   "error." <> tag, only for the closed set
                             badarg badarith badmatch function_clause case_clause
                             if_clause try_clause cond_clause undef badfun badarity
                             badkey badmap badrecord noproc system_limit else_clause
                             with_clause; anything else is "error.other"
  throw                      "throw"
  exit                       "exit"
  ```

  Every character outside `[A-Za-z0-9._<>$/-]` becomes `_`, capped at 64; the
  closed set means a target cannot mint arbitrary kind text through
  `:erlang.error(:any_atom)`. Anything the classifier cannot read is
  `"<unreadable>"`; classification never raises.
* **Frames.** From `__STACKTRACE__` of the catch (a bounded list, at most
  `Limits.max_walk/0` = 4,096 entries inspected): each `{m, f, a, _location}`
  becomes `Mod.fun/arity` (`arity` is the integer, or the length of an argument
  list); **no file, no line, no argument value**. Frames of the engine's own
  modules (`CodingAdventures.TestBootstrap.Fuzz.*`) and entries that are not that
  shape are dropped; consecutive identical frames collapse (so recursion depth
  does not change the signature); the innermost 64 are kept; every character
  outside `[A-Za-z0-9._<>$/#-]` becomes `_`, so **no frame contains `:`** (VER06 §9.4
  item 6).
* `crash_signature(kind, frames) = lower32(fnv1a64(kind <> ":" <> join(frames,
  ":")))` as 8 lowercase hex digits; the real values are pinned (§9).
* **Exhaustion is sticky.** The first breach of the step or the memory budget sets
  the handle's exhausted cell (compare-and-swap from 0, first wins) and throws a
  sentinel; afterwards **every** `tick` and `reserve` throws it again. The BEAM has
  no throwable a target cannot catch, so the guarantee is not the throw, it is
  stickiness plus the **post-run check**: after the execution ends (returned,
  raised, or was killed), the engine **retires the handle and then reads the
  exhausted cell**; a non-zero cell means `timeout` (steps) or `oom` (memory),
  whatever the target did with the throw: swallowing it and returning, raising
  something ordinary, or exiting normally is **never a pass and never a crash**.
* **Sentinels carry the execution's token.** The sentinel is the tuple
  `{:coding_adventures_fuzz_budget, token, which}` where `token` is the
  execution's `make_ref()`. The engine never trusts a sentinel it merely sees: a
  sentinel-shaped value thrown while the cell says the budget is not exhausted
  (forged, or retained from another execution) is a crash of kind
  `stale-budget-sentinel`, never a timeout.
* **Non-local exits.** A target is a function; it can only return, raise, throw or
  exit. Nothing the target does can abort the campaign: even `Process.exit(
  self(), :kill)` is a classified verdict (§5.1).

### 5.3 Fresh handle per execution, retired atomically

Each execution gets a **fresh `Handle`**: `%Handle{cells: :atomics, token:
make_ref()}`. The cells hold, in `:atomics` (so any process holding the handle
acts on the **same** state atomically, with no lock that a killed process could
leave held): the state (0 live, 1 retired), the **token hash** (`:erlang.phash2/2`
of the handle's token), the step and memory budgets, the step and memory
counters, the exhausted cell, an edge-call counter and the 1,024-word edge bitmap.
Every call (`edge`, `tick`, `reserve`) first checks `state == live` **and** that the
struct's token hashes (`phash2`, 32 bits) to the stored token hash: a handle struct
rebuilt with another token (or a stale one) is inert. This is an identity check, not
an unforgeable capability: **inside one VM a hostile target can write the handle's
atomics** (clear the exhausted cell, reset a counter). The engine cross-checks the
counters with the budgets after the run (`steps_used > step_budget` is a timeout,
`mem_used > mem_budget` an oom, even if the cell was cleared), which catches casual
clearing, not a determined in-VM attacker; targets are trusted not to attack the
harness, as in every other lane. The engine retires the handle with an
atomic exchange of the state cell **after the execution has ended**, then reads
the edges and the exhausted cell. After retirement:

* `edge/2` is a **no-op** and never touches the bitmap (a write that raced the
  retirement lands in a bitmap nobody reads again);
* `tick/2` and `reserve/2` **throw the retired sentinel**, which is a crash of kind
  `stale-budget-sentinel` if it somehow reaches a later execution;
* the budgets, counters and edges of a handle that leaked to another process, was
  stored in `:persistent_term`, an ETS table or a process dictionary, or was
  retained by the target, **cannot influence any later execution or the campaign**:
  each later execution has its own atomics. A test forwards and retains a handle
  and shows it inert.

### 5.4 Round-2 hardening: orphans, floods, deadline

* **A launcher between engine and target.** The target's *parent* is a throwaway
  **launcher** process, never the engine. Whatever the target sends "to its parent"
  piles up in the launcher's mailbox (heap-limited, and polled: a launcher blocked in
  `receive` never garbage collects, so `max_heap_size` alone would not stop a flood;
  the poll reads its `:memory`, which includes the mailbox, and kills both when it is
  past the budget: `oom`). The launcher is linked to the target (an abnormal target
  death takes the launcher with the same reason, which is how the engine reads the
  `:killed` of the heap limit) and, inside a campaign, to the engine.
* **No orphans.** The engine (a) traps exits and monitors its caller; on the caller's
  `:DOWN`, or when it was already queued, it kills the current execution and exits; (b)
  is linked to the launcher, which is linked to the target, so killing the engine (the
  caller's bound expiring kills it) takes both with it. The engine drops the
  resulting `{:EXIT, _, _}` messages after each execution; a target crash never kills
  it. A target that itself traps exits, or processes it spawned unlinked, can survive
  the engine's death (an in-VM limit).
* **The drain watches the caller too.** The post-execution drain is given the
  engine's caller monitor: if it swallows the caller's `:DOWN` (the caller died
  between the wait and the drain) no execution is live, so the engine exits at once.
  The caller's `bound_ms:` flag (a test seam) is validated as an integer in
  `0..4_294_967_295` **before** an engine is spawned; floats, negatives and
  `:infinity` are an `ArgumentError`.
* **A bounded engine.** The engine is spawned with `max_heap_size` (`kill: true`,
  sized from the 256 MiB retained cap plus `mem_budget` plus 64 MiB); its death is a
  clean `RuntimeError` in the caller. Its own mailbox is also polled: more than
  10,000 queued messages, or engine memory that grew by more than the budget since
  the execution began, kills the target as `oom`.
* **A real deadline.** The wall clock is `System.monotonic_time(:millisecond) +
  timeout_ms`, recomputed every 5 ms slice (so polling cannot stretch it;
  `required_capabilities.json` declares `time:read`). `:memory` is read every slice;
  the expensive `:binary` item every 4th slice or whenever `:memory` is above a
  quarter of the budget; more than `budget / 48` refc binaries is over budget at
  once (each costs the target at least 48 bytes of heap). The caller's bound is twice
  the per-execution worst case times the number of executions plus a minute, capped at
  the 32-bit limit.
* **Accepted limits (recorded).** One non-yielding BIF that allocates before any poll
  (the poll sees it afterwards); memory parked outside the process (ETS,
  `:persistent_term`, atoms, unlinked children); a target forging a result or
  writing the handle's atomics (§5.3): targets are trusted not to attack the harness
  inside the VM, as in every other lane.

## 6. Campaign contracts (VER06 §9.1)

`Fuzz.run_outcomes(initial_corpus, options, exec)` is the outcome-level campaign
(the executor is a function `(data, handle) -> outcome`); `Fuzz.run(initial_corpus,
options, target)` wraps a target function `(data, handle) -> any` in the executor
of §5. `Options` fields: `max_iterations 1000`, `seed 42`, `max_input_len 4096`,
`base_energy 10`, `minimize_budget 2000`, `step_budget 200000`, `mem_budget
64 MiB`, `timeout_ms 5000`, `ci :unset`; every field validated (`ArgumentError`
out of range or the wrong type; booleans and floats are refused). Determinism,
budget, honest report, dedup and feedback are VER06e §6 with the same
per-iteration draw order: `bounded(len(corpus))` for the seed; per energy slot
`1 + bounded(4)` steps, one `next_u64` sub-seed, then, with more than one seed,
`bounded(4)` and on 0 a donor index and two split draws.

**Caps (CWE-400), all enforced before the allocation they bound.** Seeds at most
10,000, each at most 1 MiB (`Limits.max_input_bytes/0`; a candidate is truncated to
`max_input_len`, itself at most 1 MiB); distinct crash signatures at most 1,000
(`crashes_dropped`); **retained bytes** (corpus seeds plus crash inputs) at most
`Limits.max_retained_bytes/0` = 256 MiB: an over-cap admission is counted in
`admissions_dropped`, an over-cap crash record keeps no input (`input_dropped`).
`mutate_sequence` is bounded as §3.

**Minimisation (VER06 §5.3).** Block deletion from a first chunk of
`max(1, len div 2)`, halving to 1, then byte zeroing; exact signature only (never
trading one crash for another); `Options.minimize_budget` (default 2,000, ceiling
100,000) caps one crash, and the **whole campaign** spends at most
`max_iterations` minimisation executions in total. **Every candidate execution
counts, accepted or rejected**, and the budget is checked before each one. The
bisection halves are pinned (§9): the chunk sizes tried are `max(1, len div 2)`,
then halved down to 1, and a 1-byte crasher starts at chunk 1 (that is what makes
the empty input reachable).

**Honest report.** `Report.clean?/1` needs at least one mutation iteration and no
crash, timeout, oom or drop. **Zero executions is never a pass.** A campaign with
`max_iterations == 0`, an empty corpus that never got past triage, or a corpus of
crashers (which are never mutated, so `iterations == 0`) is not clean.

**Verdicts, CI mode and re-verification.** `Fuzz.verdict(report, options, exec)`
turns a report into a `Verdict`. *Before any crash is reported, its minimal input
is run once more through the executor* (one execution per crash, at most 1,000 in
total) and the verdict records `reproduced`; a crash whose signature does not come
back is **still failing** (fail closed) but is reported as `flaky` and its witness
is never presented as a reproduction. A verdict is `passed` only when the report
is clean. `options.ci` is `:on`, `:off` or `:unset`; **`:unset` is treated as CI
and fails closed**: in CI mode the verdict and its TAP lines carry **no seed, no
corpus and no witness bytes** (`seed`, `witnesses` are `nil`, and only the closed
failure id and the 8-hex signature remain). Only `ci: :off` fills them.

## 7. Private reporting (VER06 §6)

`format_tap_failure(n, target, failure_id, sig)` emits `not ok N - <target>
[<failure-id>:<hex8>]` only. `failure_id` must be one of the five registered ids and
`sig` exactly eight lowercase hex digits, else `ArgumentError`; `n` an integer in
`1..2^31-1`. **The target is reduced to its grammar as VER06 §9.3 and every other
lane do** (Ruby `Signature.token`): truncated to 128 **bytes** first, then every
**byte** outside `[A-Za-z0-9._/-]` becomes `_` (so a multi-byte character becomes one
`_` per byte, and a cut in the middle of one leaves a single `_`). `#`, spaces,
controls, bidi and invisible characters all become `_`, so nothing can forge or
hide a line or start a TAP directive. The registered-grammar fields (`failure_id`,
`sig`) are checked, not escaped. The VER05m `Canon` escape set is used only for
the corpus runner's problem lines. Pinned cross-lane values were computed with the
Ruby lane's `format_tap_failure`. `tap_failures(report, target, first_test)`: one line per
crash signature, then one for timeouts and one for ooms, whose hash is
`crash_signature("timeout" | "oom", [])`.

## 8. Corpus runner

`Fuzz.Corpus.evaluate(cases)` returns `%{evaluated: n, problems: [...]}`.
Strictness and caps are the shared ones (canonical ints, lowercase even hex,
10,000 elements and steps, 1,000-digit ints, depth 64, 8 MiB, problems clipped to
400); an unknown op, a non-object case, a missing or duplicate id or a wrong field
set is a **problem**, never a skip, and every problem about a case starts with the
case id **escaped with `Canon.escape/1`**. Invariants beyond expected values: flips,
arithmetic and boundary keep the length, splice length is prefix + suffix,
admission equals set difference, no TAP newline, privacy recomputed from bytes,
every registered id accepted, every op appears, exactly 25 cases.
`mutate_sequence` work is the aggregate of §3. `corpus_reject` is judged by
`Tfv.valid?/1`. Every engine entry point the runner calls goes through `Hooks`
(process-dictionary overrides set with `Hooks.with/2`, as `Pbt.Hooks`), so a test
can sabotage one and the run must fail.

## 9. Tests, pins and negative controls

* All 25 `fuzz-v1` cases: 25 evaluated, 0 problems. The 820 reference vectors of the
  Go lane's committed table are parsed from that file (no copy) and all must match.
* **Negative controls (the runner must FAIL)**: for every case and every
  corruption its op defines (a wrong expected output, a wrong coverage count, a
  flipped admission, a wrong duplicate verdict, a flipped validity, a tap line
  changed by a character, a valid TFV in place of a reject text), the run fails and
  every problem **starts with that case's id**; dropped, duplicated and truncated
  corpora fail; unknown ops and bad field sets fail; every sabotage hook makes the
  run fail; nothing leaks after `Hooks.with/2`.
* **Pinned values for the corpus blind spots**: every interest value and delta;
  unequal splice splits with target and donor swapped; energy rows including exact
  half-way ties (`10,4,3,6 -> 8`, `3,4,0,0 -> 2`, `5,4,0,0 -> 3`,
  `25,100,0,0 -> 3`, `1,4,0,0 -> 1`); the real `sig_hash` values from the Python
  oracle (`panic eval step -> 4eca7ae8`, `segv alloc write -> a0aa3aa2`, `timeout
  -> 4e61ea76`, `oom -> 7146a410`, `stale-budget-sentinel -> 8a20b3bf`,
  `ArgumentError a.rb#x -> c93b2322`); the rejection draw in bounded sampling (a
  seed whose first word is below the threshold, equal to it, and the `2^63 + 1`
  boundary); and the minimiser's bisection halves (the exact sequence of chunk
  sizes and the number of executions spent, with accepted and rejected candidates).
* **Hardening tests**: a retained and a forwarded handle are inert (no edge, no
  budget effect, `tick` throws the retired sentinel); a swallowed step breach is a
  timeout; a swallowed memory breach is an oom; a swallowed breach followed by a
  return, by a raise and by `exit(:normal)` is still a timeout; a forged sentinel
  is a crash; a heap bomb is an oom through `max_heap_size` and an unrelated
  `:DOWN` in the mailbox is not one; a hang is killed by the wall clock; a message
  and `inspect` hostile exception is classified without either being called; CI
  mode carries no seed or witness; zero executions is not clean; the retained-bytes
  cap, the input cap, the campaign-wide minimisation budget and the aggregate
  `mutate_sequence` cap (a breach zeroes the allowance) are exact.
* **Bounded tests**: every test has a `@tag timeout`, every test `receive` has an
  `after`, and `assert_receive`/`refute_receive` carry explicit timeouts, so no
  test can hang.
* At least 95% line coverage, enforced by `test_coverage` in `mix.exs`.

## 10. Divergences and open items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | explicit edges through the handle; `:cover` rejected for the hot path, recorded as a future `beam-cover-lines` tier |
| 2 | Timeouts | step budget **and** a wall clock enforced by killing the execution process (Ruby is cooperative only) |
| 3 | OOM | `max_heap_size` with `kill: true`, classified only from the monitored execution process; a self-kill is indistinguishable and also reads as `oom` |
| 4 | Unlinked descendants | not reachable by the engine; linked ones die with the execution process |
| 5 | Sentinel is catchable | by design; stickiness + post-run check |
| 6 | Retained-bytes cap, input cap, campaign-wide minimisation budget, aggregate sequence cap | as Java and Ruby, 2^24 aggregate, `2^28` per call |
| 7 | CI mode | `:unset` is CI; no seed or witness; crash witnesses re-verified |
| 8 | §9.4 items 7, 8, 9 of VER06 | **open**, as in every lane |
| 9 | Corpus weaknesses (PT-F1) | this lane replays Go's 820 oracle vectors, pins sig_hash values, energy ties, every constant, unequal splice splits, the rejection draw and the bisection halves |
| 10 | Build | `BUILD` and `BUILD_windows` run `mix test --cover` (threshold 95); `elixir/test_bootstrap` is a repository-input consumer of `fuzz-v1` and of Go's reference-vector table in the Go build tool, failing closed when the package is absent |
