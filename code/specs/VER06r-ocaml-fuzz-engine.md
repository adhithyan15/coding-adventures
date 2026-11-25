# VER06r — OCaml Coverage-Guided Fuzz Engine

**Parent:** [VER06](VER06-coverage-guided-fuzz-engine.md). Siblings and sources: [VER06i](VER06i-ruby-fuzz-engine.md) (Ruby, the cross-lane reference), [VER06j](VER06j-c-fuzz-engine.md), [VER06o](VER06o-haskell-fuzz-engine.md) (Haskell, the closest typed-functional analogue), VER06p (Lua), VER06q (Perl), VER06n (Elixir) (open on their own branches). Builds on [VER05q](VER05q-ocaml-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-OCAML-FUZZ. Governing contracts: [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md).

This specification defines the OCaml 5 implementation of the Level 2 coverage-guided fuzz
engine. There is **one** OCaml engine: the `Fuzz_*` modules of the existing package
`code/packages/ocaml/test-bootstrap`, beside `Prng`, `Gen`, `Runner`, `Strict_json`, `Tfv`
and `Corpus` (VER05q). It reuses `Prng` (SplitMix64, FNV-1a, unsigned draws), `Bigint`,
`Safe` (`Sys.Break` and `Out_of_memory` are never a verdict), `Escape`, `Values`,
`Strict_json`, `Tfv`, `Types.ci_mode` and `Error`; it duplicates none of them. It conforms to
the `fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte,
replays the Go lane's 820 oracle vectors, and meets every campaign contract of VER06 §9.1.

The algorithms are those of VER06n, VER06o and VER06i and are **not restated**: the mutator
algebra, the draw order of `mutate_sequence` and of the campaign, the power schedule, the
minimiser's bisection, the signature, the verdict. The letter `r` is the next free one after
`q`. This document records what OCaml changes and the hardening, **all of it in the first
version**.

## 1. Modules

| Module | Role |
|---|---|
| `Fuzz_limits` | the CWE-400 ceilings |
| `Fuzz_mutate` | mutator algebra, interest lists, `mutate_sequence` (+ counted form) |
| `Fuzz_bitmap` | 65,536-edge map, `corpus_admit`, `calculate_energy` |
| `Fuzz_failure_ids` | the closed failure-id vocabulary |
| `Fuzz_signature` | `crash_signature`, kind normalisation, TAP token and line |
| `Fuzz_types` | options, outcomes, report, verdict |
| `Fuzz_handle` | the per-execution coverage and budget handle, **abstract** |
| `Fuzz_handle_internal` | the representation and the test seams (unsafe; tests only) |
| `Fuzz_executor` | one execution in its own domain, classification |
| `Fuzz_engine` | minimiser and campaign (`run_outcomes`) |
| `Fuzz_hooks` | sabotage hooks for the negative controls |
| `Fuzz_corpus` | production runner for the shared corpus |
| `Fuzz` | the public facade: `run`, `executor`, `verdict`, `verdict_for` |

Stdlib plus `unix` (which ships with the compiler; used only for `Unix.gettimeofday` and
`Unix.sleepf`): `Domain`, `Atomic`, `Mutex`, `Gc`. No `threads`, no third-party library, no bisect in
the hot path. `required_capabilities.json` declares what is really used:
`thread:spawn` (the executor and the tests spawn domains) `time:read` (the wall clock bounds a
wait) and `time:sleep` (the waiter backs off with `Unix.sleepf`). The tests print nothing, so no
`stdout:write`.

## 2. Numbers

OCaml's native `int` has 63 bits and wraps silently, so **every size and cost sum is checked
before it is made** (`a <= cap - b` instead of `a + b <= cap`). A 64-bit word lives in an
`int64`; unsigned comparison, division and remainder use `Int64.unsigned_*`. Mutator
arguments that a fixture or the Go oracle feeds as 30-digit or 1,000-digit integers (offsets,
counts, deltas, values) are `Bigint.t`, reduced to a native index with `Bigint.rem_pos`
against a **native** modulus (at most 8 MiB) before use. **No `Bigint` is ever built from
input bytes**: a flip is a byte or word loop; arithmetic and boundary values are reduced
modulo `2^(8 w)` for `w <= 8` through `Bigint.to_int64_wrap` (the value modulo `2^64`, and
`2^(8 w)` divides `2^64`). Flips work by **parity**: a count of `10^999` costs O(len).

Edge ids a target reports are native `int`s reduced with `land 0xFFFF` (equal to the floored
`mod 65536` of the other lanes, also for negative ids). `corpus_admit` takes the fixture's
unreduced `u64` ids as `int64`s (equality only).

## 3. Coverage source (decision)

VER06 §7 names a platform source per lane. For OCaml it would be `bisect_ppx` or the
runtime's counters. **This lane does not use them in the hot path:**

1. bisect_ppx counters are a process-global table written at exit, with no safe "since the
   last execution" read and no reset; the package's own coverage gate uses the same table, so
   a reset between executions would zero the coverage CI is measuring.
2. It requires the target and every library it calls to be built with the ppx and the
   `--instrument-with` flag; a plain `dune build` target reports nothing.
3. Counters are shared by every domain: a domain a target left running pollutes every later
   trace.

The engine takes **target-reported edges through a per-execution handle**, as the Ruby,
Elixir and Haskell lanes do: the target calls `Fuzz_handle.edge h id` (`edge_id "label"`
gives a stable FNV-1a id). One locked bit-set per edge; nothing is global. `coverage_tier`
is `caller-reported` when the target called `edge` at least once during the campaign, else
`black-box-uninstrumented` (the first passing seed is still admitted, so there is something
to mutate).

## 4. Isolation (decision: a fresh domain per execution)

**Every execution runs in its own freshly spawned domain.** The alternative, running the
target in the engine's domain under a step budget with a watchdog domain setting a flag that
`tick` polls, was rejected:

* a runaway target (a loop that never calls `tick`) would hang the engine's own domain: no
  verdict, no leak accounting, the CI job timeout the only outcome. With a worker domain the
  engine regains control at the deadline, reports `Timeout` and carries on;
* a fresh domain has fresh domain-local state (`Domain.DLS`), so a target that poisons a DLS
  key cannot influence the next execution or the campaign;
* the allocation counters of a fresh domain give the memory estimate of §6 a clean baseline.

Costs: a spawn per execution (tens of microseconds) and the runtime's ceiling of about 127 domains
in all. Leaked workers (§7) occupy that ceiling, so they are capped per campaign **and** per process,
and a failed spawn is a typed `Error.Engine_error` naming the ceiling (never a verdict, never a pass).

**OCaml has no safe asynchronous interruption of another domain.** The only bounds on a
running target are cooperative: the step budget (`tick`), the memory budget (`reserve` and
the allocation polling of §6) and **retirement**: when the engine gives up on a worker it
retires the handle, so a target that still calls `tick` or `reserve` gets the retired
sentinel and unwinds. **A non-cooperating tight loop (no `tick`, no `reserve`) cannot be
stopped in-process.** The engine reports `Timeout`, leaks the domain (it keeps spinning until
the process exits) and counts it; after `max_leaked_workers` (4) the campaign is aborted
(§7). The CI job timeout is the backstop for a process that is itself wedged, and **PT-F14,
the out-of-process runner**, is the future fix; this document does not pretend otherwise.

**The deadline is a wall-clock bound** (`Unix.gettimeofday`). The waiter polls an atomic cell and
sleeps with a backoff from 50 microseconds to 1 ms, so neither its own spinning nor the CPU time
of leaked spinners can make an honest target time out (an earlier draft used `Sys.time`, process
CPU time, and gave a 3 s target a false timeout at a 5 s budget). `Unix.gettimeofday` is not monotonic, so elapsed time is also accumulated from the sleeps
themselves (each round adds the observed delta clamped to [sleep, sleep + 100 ms]) and the raw wall
clock is trusted only while no step has been seen: a backward step cannot hang the wait for the
length of the step and a forward jump cannot cut it short (tested with an injected clock). If the
waiter itself is interrupted (`Sys.Break`) the handle is retired first, so a cooperating worker
unwinds.

`Stack_overflow` raised in the worker is a crash of kind `Stack_overflow`. `Sys.Break` and
`Out_of_memory` are **never a verdict**: the worker hands them back and the engine re-raises
them in its own domain.

## 5. The handle, sentinels, sticky exhaustion

### 5.1 Fresh handle per execution, retired after it

Each execution gets a fresh `Fuzz_handle.t`, **abstract in the interface**. Its state (live or
retired, an unforgeable identity token, step and memory budgets and counters, the sticky
exhausted flag, the edge-call counter, a 65,536-bit edge map) is guarded by one `Mutex` held
only around integer arithmetic (nothing inside can raise; no target code runs under it).
Every call first checks that the handle is **live and that its token is the one stored in the
state**. After retirement:

* `edge` is a **no-op** (it never touches the map);
* `tick` and `reserve` **raise `Sentinel (token, Retired)`**.

A handle retained in a closure, forwarded to another domain or stored in a global is inert
after its execution; each later execution has its own state. A **forged** handle (the seam
`Fuzz_handle_internal.forge` builds a handle sharing a state but carrying another token) is
inert in the same way.

`Fuzz_handle.token` is an abstract type: only `Fuzz_handle_internal` can mint one. The engine
never trusts a sentinel it merely sees: a `Sentinel` that escapes the target is honoured
**only** if the post-run state says the budget was exhausted; any other (forged with a
retained token, thrown by hand, retained from another execution) is a crash of kind
`stale-budget-sentinel`, never a timeout.

`Fuzz_handle_internal` holds the representation and the test seams (`forge`,
`clear_exhausted`, `mint_token`, raw counters). It is **unsafe and not for targets**; a holder
of the abstract handle cannot reach it from the handle. A hostile in-process target can still
call it; targets are trusted not to attack the harness, as in every other lane.

### 5.2 Sticky exhaustion in both directions, and the post-run verdict

The first breach of the step or memory budget records itself (first wins) and raises the
sentinel; **afterwards every `tick` and `reserve` raises it again, whichever budget breached**:
`reserve 0` and `reserve 1` after a steps breach, `tick 0` after a memory breach (tested both
ways). OCaml cannot make an exception uncatchable, so the guarantee is stickiness plus the
**post-run verdict**: after the execution ends the engine **retires the handle and then**
reads the exhausted state, the sticky flag **cross-checked against the counters**
(`steps > step_budget` is a timeout, `mem > mem_budget` an oom even if the flag was cleared).
A target that swallowed the sentinel and returned, raised something ordinary or exited
normally is never a pass and never a crash. The cross-check is tested through the seam that
clears the flag, and the same scenario with the cross-check disabled (`~cross_check:false`)
gives a pass, so the test fails if the cross-check is removed.

`tick n` and `reserve n` clamp `n` to `[0, budget + 1]` before adding, so counters cannot wrap.

## 6. Memory (decision: best-effort Gc deltas, polled)

OCaml has no per-domain heap limit. The worker records `Gc.counters ()` when it starts
(`arm`); every `tick` and `reserve` called **by that domain** reads it again, and the bytes
allocated since the baseline, `8 * (d_minor + d_major - d_promoted)`, are added to the
reserved bytes. `Gc.counters` itself allocates about 150 bytes on the worker, which would be charged
to the target, so a reading is taken on the first and every 64th `tick`/`reserve` call, and the sampler is
**net of its own cost**: `arm` calls `Gc.counters` twice back to back (the difference is exactly one
call's allocation) and takes the second reading as the baseline, so reading number k contains k calls'
worth of the sampler's own allocation and `k * self` is subtracted; the target never pays for it (5M
`tick`s or `reserve 0`s pass under the 64 KiB minimum budget), and the rest of the call path (including `edge`) is allocation-free: no closure,
no option, no tuple; the lock is `lock`, a match that cannot leak the lock, `unlock`. Five million
`tick`s, `reserve`s or `edge`s under 256 MiB pass (tested). The total is compared with `mem_budget` (default **256 MiB of
allocation per execution**; options below 64 KiB are refused, because the engine's own baseline would make
every execution an oom). The worker takes one last reading after the target returns, so a target that
allocates a lot and never ticks is still judged `Oom`.

Semantics, honestly: this counts **total bytes allocated by the domain**, not residency;
`Gc.counters` is "the current domain or potentially previous domains", so a domain
terminating mid-execution can add to the reading (an over-estimate, fail closed); in native
code `minor_words` is an approximation; a call from another domain is ignored. It is a
**cooperative bound**: a target that allocates in a loop and never calls `tick` or `reserve`
runs until it returns.

## 7. Leaked executions are capped at 4 on every path

A worker that has not answered within the kill wait (`kill_wait_ms` 2,000, then
`leaked_kill_wait_ms` 500 once one leak is counted) is **leaked, never joined**. One `leaks`
counter (an `Atomic.t`) is shared by every execution a campaign makes:

* the triage runs and the mutation loop (the campaign aborts, `leaked_workers > 0`, never
  clean, reason `workers-leaked`, public line `fuzz-timeout` with the signature of the bare
  kind `workers-leaked`);
* **the minimiser** asks the abort flag before every candidate;
* **re-verification**: `verdict_for target report options id` wires the executor and the
  verdict to ONE shared counter; a report that already leaked 4 verifies nothing
  (`Unverified`), and re-verification itself stops, `Unverified` for the rest, at the cap. The
  plain `verdict` (arbitrary `exec`) applies only the report's own count.

**A process-wide cap.** The per-campaign cap alone would let about 30 hostile campaigns exhaust the
runtime's domains, after which `Domain.spawn` fails opaquely. `Fuzz_executor.global_leaks` counts the
leaked workers alive in the whole process; a worker that finally returns subtracts itself (a
compare-and-set on the execution's result cell decides who won the race, so the count cannot drift;
both the per-campaign and the global counter are raised before the race and lowered if it is lost or
an exception interrupts it, the handler reading the cell to decide).
At 16 `run_tracked` (and so `Fuzz.run`, `executor` and `verdict_for`) raises
`Error.Engine_error`: fail closed, never a pass, never a report that hides the refusal. A failed
`Domain.spawn` is the same typed error.

"Leaked" means "did not answer within the wait": a legitimate target whose cleanup is slower
and that times out four times is aborted, fail closed.

## 8. Caps, enforced before the allocation

| Cap | Value |
|---|---|
| seeds / one seed, a candidate, a sequence input | 10,000 / 1 MiB |
| `mem_budget` option | at least 64 KiB (the engine's own baseline), at most 1 GiB |
| leaked workers | 4 per campaign, 16 per process |
| distinct crash signatures kept (`crashes_dropped`) | 1,000 |
| retained bytes (corpus plus crash inputs) | 256 MiB; over-cap: `admissions_dropped`, or a crash record without its input |
| `mutate_sequence` steps | 1,000,000 |
| `mutate_sequence` work per call | 2^28 bytes copied |
| corpus runner aggregate `mutate_sequence` work | 2^24, a breach **zeroes** the allowance |
| minimisation | `minimize_budget` per crash and **`max_iterations` for the campaign**; rejected candidates count |

The retention check is made **before** the bytes are stored. **Retained inputs are copies**
(`String.sub`) and the copy is charged: corpus admission, a novel crash's minimal input, a
repeat crash's smaller witness. Each copy is pinned by a test that fails if it is removed.
`mutate_sequence` charges the bytes it actually copies (the initial copy, then the current
length before each step); a failed call is charged too; all sums are overflow-checked (§2).

## 9. Campaign contracts and verdicts (VER06 §9.1)

Per iteration the stream draws: `bounded (len corpus)` for the seed; per energy slot
`1 + bounded 4` steps, one `next_u64` sub-seed, then, with more than one seed, `bounded 4` and
on 0 a donor index and two split draws. A campaign is a pure function of (corpus, options,
exec). `is_clean` needs at least one mutation iteration and no crash, timeout, oom, drop or
leak: **zero executions is never a pass**. Every crash witness is **re-run** before it is
reported (`Reproduced`, `Flaky` (still failing, fail closed) or `Unverified`, never a pass).
`Ci_on` and `Ci_unset` (an unset mode is CI) carry **no seed and no witness**; only `Ci_off`
fills them. A leaked or tampered execution is `fuzz-timeout` with a bare-kind signature; no
new failure id exists.

Crash kind = the exception **constructor name** (`Printexc.exn_slot_name`), never
`Printexc.to_string` (a payload, and a registered printer runs user code); characters outside
`[A-Za-z0-9._<>$/-]` become `_`, capped at 64; classification never raises. OCaml exceptions
carry no portable stack, so **frames are empty**. `crash_signature kind frames` is the low 32
bits of `fnv1a64 (kind ^ ":" ^ frames joined by ":")` as 8 lowercase hex digits (VER06 §9.4
item 6).

## 10. Private reporting (VER06 §6, §9.3)

`format_tap_failure n target failure_id sig` emits `not ok N - <token> [<failure-id>:<hex8>]`
only. `failure_id` must be registered and `sig` exactly eight lowercase hex digits, else
`Error.Argument_error`; `n` in `1..2^31-1`. **The target token is the byte grammar of
VER06 §9.3 and every other lane:** the bytes of the target are truncated to 128 **bytes**
first, then every **byte** outside `[A-Za-z0-9._/-]` becomes `_` (a multi-byte character
becomes one `_` per byte; a cut inside one leaves a single `_`). Pinned values were produced
by running the Ruby lane's `format_tap_failure`: `a#b` -> `a_b`; spaces; `\n`; `\r`; e-acute +
U+202E + `x` -> `_____x`; 200 `x` -> 128 `x`; 127 `x` + e-acute + `y`s -> 127 `x` + one `_`;
the empty target -> `not ok 3 -  [fuzz-crash:3a8f19b2]`. `tap_failures` gives one line per
crash signature, then timeouts, ooms, leaked workers, with the bare-kind signatures
`timeout`, `oom`, `workers-leaked`.

## 11. Corpus runner, tests, build

`Fuzz_corpus.evaluate` returns the evaluated count and the problems; strictness and caps are
the shared ones. An unknown op, a non-object case, a missing or duplicate id or a wrong field
set is a **problem**, never a skip, and every problem about a case starts with the case id,
escaped. Every engine entry point the runner calls goes through `Fuzz_hooks` (a record of
functions), so a negative control installs one wrong function and the run must fail.

The package embeds the two fixtures **as generated OCaml modules checked byte for byte in
`BUILD`**, like the pbt-v1 corpus, so the test binary reads no file:
`test/fuzz_corpus_data.ml` from `code/specs/fixtures/fuzz-v1/cases.json` and
`test/go_vectors_data.ml` from `code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go`.
The vector table is **parsed from that text** (no copy): all 820 rows must match. The build
tool treats `ocaml/test-bootstrap` as a consumer of both inputs and fails closed when the
package is absent.

Tests: `fuzz-v1` 25 of 25; 820 of 820 vectors; **negative controls** (the runner must FAIL on
every mutated fixture, each problem starting with its case id; dropped, duplicated and
truncated corpora fail; every sabotage hook fails the run); **pinned values** for the corpus
blind spots (interest values and deltas; unequal splits; energy rows including ties; real
`sig_hash` values; rejection draws; minimiser bisection traces) cross-checked against the Ruby
and Haskell lanes, plus a **whole-campaign cross-check** against the Ruby lane; **hardening
tests** for §4 to §8; every test is bounded (a worker that must be released is released by an
atomic flag). At least 95% bisect coverage over the package.

## 12. Divergences and open items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source | explicit edges through the handle; bisect_ppx rejected for the hot path (§3) |
| 2 | Isolation | fresh domain per execution; a non-cooperating loop cannot be stopped in-process (§4); out-of-process runner is PT-F14 |
| 3 | Deadline | wall clock (`Unix.gettimeofday`) with a sleeping waiter; `unix` ships with the compiler |
| 4 | Memory | `Gc.counters` deltas at `tick`/`reserve`/end of run: allocation, not residency, cooperative (§6) |
| 5 | Frames | none: OCaml exceptions have no portable stack |
| 6 | Sentinel is catchable | by design; stickiness plus the post-run verdict |
| 7 | Leaks | capped at 4 on every path, one shared counter (§7) |
| 8 | VER06 §9.4 items 7, 8, 9 | **open**, as in every lane |
| 9 | Build | `BUILD` runs pbt-v1 and fuzz-v1, `cmp`-checks the three embedded modules, `@fmt` and the bisect report with one explicit `--expect` per source file (the directory form does not match Windows `src\\` paths) |
| 10 | Real `Stack_overflow` | classified as a crash kind; the test raises the exception by hand because a genuine overflow of an OCaml 5 stack costs about 1 GiB |
| 11 | Domain ceiling | reachable: about 127 domains per process, and leaked workers hold them. Per-campaign cap 4, process-wide cap 16 (§7); a failed `Domain.spawn` is `Error.Engine_error`; both tested |
