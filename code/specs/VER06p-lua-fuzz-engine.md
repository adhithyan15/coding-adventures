# VER06p — Lua Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05o](VER05o-lua-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06i](VER06i-ruby-fuzz-engine.md), [VER06n](VER06n-elixir-fuzz-engine.md), [VER06o](VER06o-haskell-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-LUA-FUZZ.

This specification defines the Lua 5.4 implementation of the Level 2 coverage-guided
fuzz engine. There is **one** Lua engine: the module tree
`coding_adventures.test_bootstrap.fuzz.*` of the package
`code/packages/lua/test_bootstrap`, beside `Int`, `Prng`, `Canon`, `StrictJson`,
`Values`, `Tfv` and the `pbt-v1` runner of VER05o. It reuses all of them and
duplicates none (`Int` for offsets, counts and edge ids that may be native or Big;
`Prng` for SplitMix64 and FNV-1a, including its unsigned `udivmod`; the strict
corpus plumbing for the runner). It conforms to the `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte, replays the Go
lane's 820 oracle vectors, and meets every campaign contract of VER06 §9.1.

The algorithms are those of [VER06i](VER06i-ruby-fuzz-engine.md) (Ruby's native
bignums are replaced here by `Int` plus word-wise code) and the hardening list is the
one VER06f, h, i, j, n and o converged on, **all of it in the first version**. The
letter `p` is the next free one after `o`. This document records only what Lua
changes.

Lua is the first lane with **no thread, process or async-exception facility**: one
OS thread, one VM, cooperative coroutines. Section 5 is therefore the part of this
document that is new; everything else is a port.

---

## 1. Architecture and placement

| Module (`fuzz/`) | Role |
|---|---|
| `limits` | the CWE-400 ceilings |
| `mutate` | mutator algebra, interest lists, `mutate_sequence` (+ counted form) |
| `bitmap` | 65,536-edge map, `corpus_admit`, `calculate_energy` |
| `signature` | `crash_signature`, kind and frame normalisation, `format_tap_failure`, `tap_failures` |
| `failure_ids` | the closed failure-id vocabulary |
| `outcome`, `options`, `report` | outcomes, validated options, reports and crash records |
| `handle` | per-execution coverage and budget handle, sentinels |
| `executor` | one execution in a fresh coroutine under a count hook |
| `engine` | minimiser, campaign (`run_outcomes`), leak guard |
| `hooks` | sabotage hooks for the negative controls |
| `corpus` | production runner for the shared corpus |
| `init` | the public facade: `run`, `run_outcomes`, `executor`, `verdict`, `verdict_for` |

The package reads no file, environment or network. It **does** read a clock: the
per-execution wall bound polls `os.clock()` from inside the count hook, so
`required_capabilities.json` declares `time:read` (CPU time of the process; no
calendar time is read). It starts no thread and no process. **Zero dependencies**:
the Lua 5.4 standard library only. Every module captures the standard functions it
uses **at load** (`string.*`, `table.*`, `math.*`, `coroutine.*`, `debug.*`,
`collectgarbage`, `os.clock`, `pcall`, `error`, ...), so a target that patches a
library table or the string metatable cannot change what the engine does.

## 2. PRNG and stream alignment

`Prng.bounded(n)` already draws one word for `n == 1` and rejects words below
`t = (2^64 - n) mod n` computed **unsigned** (`Prng.udivmod`), so no wrapper is
needed. The engine obtains its generator through `Prng.new_rng` so the existing
`Hooks.new_rng` sabotage seam also covers it. The corpus never reaches a
rejection, so pinned tests force it (§9).

## 3. Mutator algebra

Exactly VER06 §3 and VER06i §3. Inputs and outputs are Lua strings (raw bytes);
a non-string input or a non-`Int` argument (a float, a boolean, `nil`) is an error.
Offsets, counts, deltas and values are `Int`s: native integers or `Int` Bigs
(up to the corpus's 1,000 digits), and wrap with floored `mod`.

* **No big integer is ever built from input bytes** (the Elixir lane crashed on a
  `2^(8 x 1 MiB)` integer). The input is only ever read as bytes. Arithmetic and
  boundary values are reduced to their **low 64 bits** (`Int.to_u64`) and masked to
  `8 * width` bits, which is exact because `width <= 8`. Offsets are reduced with
  `Int.mod` against a native modulus. Reads and writes use
  `string.unpack`/`string.pack` with a native width of 1, 2, 4 or 8.
* **Flips work by parity and byte-wise.** Bit `i` is bit `i mod 8` of byte
  `i div 8`, LSB first. Every unit is hit `count div units` times and the first
  `count mod units` units from the wrapped start once more; only parity matters. So
  the work is O(len) for any `count`, including a 1,000-digit one: a full inversion
  is one `gsub` through a 256-entry table, and the remainder is at most two ranges
  (the wrap), each of which masks its two edge bytes and inverts the middle.
* `byte_arithmetic` / `boundary_insert`: widths 1, 2, 4, 8 only; any other width, or
  a buffer shorter than the width, returns the input. The offset wraps over
  `len - width + 1` positions.
* `mutate_sequence(data, seed, steps, max_len)` has the Python/Java/Ruby/Elixir/Haskell
  draw order: per step `bounded(7)`, then that step's draws (flip: index;
  arithmetic: delta index, offset; boundary: value index, offset; delete: length then
  offset, skipped when `len <= 1`; insert: byte then offset; duplicate: length, source,
  destination, skipped when `len <= 1`). Every draw is made into a local first: Lua
  does not define argument evaluation order. An empty buffer is refilled with `00`
  before every step. An insert or duplicate over `max_len` is skipped **after** its
  draws. The seed is an `Int`, reduced modulo `2^64`.
* **Work cap (CWE-400), charging the actual work.** `steps` above 1,000,000 or an
  input above 1 MiB is an error **before anything is allocated**. The bytes a call
  copies are charged: the initial copy, then the current length before each step.
  `mutate_sequence_counted(data, seed, steps, max_len, cap)` returns `bytes, work`, or
  `nil, work` when the step that would cross the cap is reached; **a failed call is
  charged too**. The per-call cap is `2^28`; the corpus runner's aggregate is `2^24`:
  each case is handed what is left, is charged what it did, and **a breach zeroes the
  remaining allowance**, so every later case fails at once.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1 and VER06i §4.1. The map is a set of edge ids reduced with floored
`mod 65536` (so `-1` is `65535` and a Big is reduced by `Int.mod`).
`corpus_admit(existing, trace)` counts the *unique* novel edges over the
**unreduced** ids; a Big is keyed by its decimal text, so no float ever becomes a key.
`calculate_energy(base, len, new, total) = max(1, floor(base' * (1 + new/total) /
sqrt(max(1, len)) + 0.5))` with bonus `1.0` when `total == 0` and `base' = max(1,
base)`; half **away from zero**: `7.5 -> 8`, `2.5 -> 3`.

### 4.2 The coverage source: explicit edges through a handle (decision)

VER06 §7 names a platform source per lane; Lua has no coverage module. The candidates
are `debug.sethook` line events (what luacov uses) and target-reported edges.
**This lane does not use line hooks as the coverage source:**

1. A line hook costs a Lua-to-hook call on **every executed line** of every function
   the target runs, in the target's whole call tree including the standard library's
   Lua parts. That is a 10x to 50x slowdown of the target, paid by every one of up to
   a million executions.
2. A thread's hook is **one slot**. The same slot carries the engine's count hook
   (§5, the step and wall bound), luacov's line hook under `busted --coverage`, and any
   debugger. Chaining them is exactly the misattribution recorded in
   `lessons.d/luacov-attributes-a-line-to-the-frame-two-levels-above-its-hook.md`;
   sharing one slot would make the package's own coverage gate and the fuzz engine
   mutually exclusive.
3. A line hook is **thread-local**: a coroutine the target creates inherits it only at
   creation, `debug.sethook()` inside the target silently drops it, and the loss is
   not observable from outside. A target-reported edge cannot be dropped that way.
4. Lines are not edges. A line event names a position, not a branch taken; to get
   AFL-style edges the engine would pair consecutive lines, which still cannot see a
   branch whose two sides share a line.

The engine therefore takes **target-reported edges through a handle**, as the Ruby,
Elixir and Haskell lanes do: the target calls `Handle.edge(h, id)` (or `h:edge(id)`;
`Handle.edge_id("label")` gives a stable FNV-1a id; ids are reduced mod 65,536) at the
branches it cares about. One table write per edge; nothing is global; no instrumented
build. Honest labelling: `coverage_tier` is `caller-reported` if the target called
`edge` at least once during the campaign, else `black-box-uninstrumented` (the first
passing seed is still admitted, so there is something to mutate). A line-hook tier
(`lua-line-hook`) is a possible future optional tier for a standalone run that owns the
hook slot.

## 5. Isolation in a single-threaded VM

### 5.1 One coroutine per execution, under a count hook

Lua cannot kill a running function. The only things that can interrupt Lua code are
an **error raised from a debug hook** and a **yield**. So every execution:

1. creates a **fresh coroutine** whose body calls `target(data, h)` (a non-tail call:
   the body's frame marks where the target's frames end);
2. installs, on that coroutine only, a **count hook** (`debug.sethook(co, hook, "",
   1000)`: one call per 1,000 VM instructions);
3. `coroutine.resume`s it. The resume is itself a protected call; **the engine never
   wraps target code in a handler it controls**, and the error value is whatever the
   target raised.

The hook is the **step and wall budget**:

* **instructions**: every call adds 1,000; past `instruction_budget` (default
  100,000,000, ceiling 10,000,000,000) the execution is a **timeout**;
* **wall clock**: past `timeout_ms` of `os.clock()` since the execution began (default
  5,000, ceiling 60,000) the execution is a **timeout**. `os.clock()` is process CPU
  time, which is the right clock for a target that computes and the wrong one for a
  target that sleeps; Lua has no way to sleep without `os`/`io`, which a trusted
  target does not use;
* **memory** (§5.5).

A breach records itself (first wins) and raises the execution's **sentinel** from the
hook, which unwinds the target. Because a hook error is an ordinary Lua error, **a
target that wraps its loop in `pcall` catches it**: §5.2 is the answer to that.

A coroutine created by the **target** is *not* covered by inheritance: a debug hook is a
per-thread registry slot (`lua_newthread` copies only the C trampoline, which then finds no
Lua function for the new thread), so an unwrapped `coroutine.wrap(function() while true do end
end)()` would run forever. For the duration of an execution the executor therefore replaces
`coroutine.create` and `coroutine.wrap` with versions that install the **same hook closure** on
the coroutine they make (one closure, one shared budget, so a target cannot multiply its budget
by spawning), and restores the previous functions when the execution ends (in a `<close>` guard). A copy of
the replacements captured during the execution (for example by a module `require`d lazily from the
target) **falls through to the real functions afterwards**: no hook, no retired error. A wrapped coroutine
that outlives its execution runs a hook that finds its state retired and raises the retired
sentinel there. **Limit (recorded):** a coroutine made through a reference to `coroutine.create`
or `coroutine.wrap` that the target captured *before* the engine ran is not bounded, and the
replacement `wrap` passes an error through unprefixed (the real one prepends a position to a
string error).

### 5.2 Sentinels, sticky exhaustion, judged after retirement

Each execution has a **fresh `Handle`** (a table with no fields; its state lives in a
module-private weak table keyed by the handle, so a target holding the handle cannot
read or rewrite the counters through it). The state is: retired or live, the step and
memory budgets, the step and memory counters, the sticky exhausted marker, the edge
set and an edge-call counter. Three sentinels exist per execution (steps, memory,
retired). A **sentinel is an unforgeable unique table**: it has no fields, and it is
recognised by **identity**, through a private weak registry
(`sentinel -> {state, which}`), compared with `rawequal`, **never by message or by
content**. A table the target builds itself, or copies, is not in the registry: it is an
ordinary crash of kind `table`.

* `edge(h, id)` on a retired, retained or forged handle is a **no-op** and never
  touches any state.
* `tick(h, n)` and `reserve(h, n)` on a retired, retained or forged handle **raise the
  retired sentinel**.
* **Sticky, both directions.** The first breach of the step budget or the memory budget
  (cooperative `tick`/`reserve`, or the count hook) records itself; afterwards **every**
  `tick` and `reserve` raises the same sentinel again, **whichever budget broke**:
  `reserve(h, 0)` and `reserve(h, 1)` raise after a steps breach, `tick(h, 0)` raises
  after a memory breach (tested both ways). `tick(h)` counts one unit, `tick(h, n)`
  counts `n`, `tick(h, 0)` counts nothing but still checks.
* **The verdict is judged after retirement.** After the coroutine ends (returned,
  raised, yielded) the executor **retires the handle and then** reads the exhausted
  marker **and cross-checks the counters against the budgets** (`steps > step_budget`,
  `reserved > mem_budget`): a breach is a `timeout` (steps, instructions, wall) or an
  `oom` (memory) **whatever the target did with the error**. A target that caught the
  sentinel with `pcall` and returned, raised something ordinary, or yielded is never a
  pass and never a crash.
* **A sentinel the executor did not expect is a crash.** One that surfaces as the
  execution's error while that execution is not exhausted (retained from another
  execution, or the retired sentinel of a stale handle) is a crash of kind
  `stale-budget-sentinel`, never a timeout.

**Honest limit (recorded).** Everything is in one VM. In-process code can reach the
counters (`debug.getupvalue` on the module's functions, `debug.getregistry`), clear the
exhausted marker, or call `rawset` on any table. The cross-check catches casual
tampering (a counter left over its budget), not a determined in-process attacker.
Targets are trusted not to attack the harness, as in every other lane.

### 5.3 What cannot be stopped, and what is detected

* **A target that catches the budget error in an unbounded loop**
  (`while true do pcall(f) end`) **cannot be stopped in-process**: Lua has no
  throwable that `pcall` does not catch and a Lua hook cannot yield (a yield across the
  hook's C boundary is an error). The CI job timeout is the only backstop: fail closed,
  but no verdict. An out-of-process runner is future work. (The sticky marker still
  makes every later `tick` raise, so a cooperative target stops.)
* **A target that clears or alters the hook** (`debug.sethook()`, or `debug.sethook(f, "", 1 << 30)`
  which keeps the function and makes it fire once in a billion instructions, or a changed mask)
  removes or defeats the only bound. If it returns, the executor **detects it**: after the run it
  reads **all three** results of `debug.gethook(co)` (function by `rawequal`, mask, count) and
  compares them with what it installed, and it does the same for **every coroutine the target made
  through the swapped `create`/`wrap`** (held strongly in a list for the execution, suspended ones included; see the cap below). The hook
  also re-checks its own thread's count and mask on every poll and breaches if they changed (a
  tampered count may mean the hook never fires again, so the post-run check is the backstop). The
  execution is a crash of kind `hook-cleared` (or a timeout, if the hook caught it first), never a
  pass, and counts as **leaked** (§5.4). **Residual:** a target can tamper, do unbounded work, and
  restore the exact hook before returning; and if it never returns, the first bullet applies.
* **A target that poisons the engine itself.** The handle metatable is locked
  (`__metatable = false`) and its `__index` is a closure over a method table nothing on the handle
  reaches. The module table is reachable, so after every execution the executor checks that every
  `Handle` function, the metatable and the method table are exactly as loaded (`Handle.intact`),
  restores them (`Handle.repair`), and reports the execution as a crash of kind `engine-tampered`,
  leaked. A poisoning attempt therefore cannot reach the next execution or re-verification.
* **A target that yields out of the engine's coroutine** (`coroutine.yield()` in the
  body) returns control to the executor with the coroutine **suspended**. That is a
  **protocol violation**: the target was supposed to run to completion. The execution
  is a crash of kind `yielded` (never a pass), counts as **leaked**, and the executor
  `coroutine.close`s the suspended coroutine (under the same hook, so a hostile `__close`
  is bounded) so that nothing of it can be resumed later.
* **Rewriting the engine through `require`.** Every engine module captures, **at load**, every
  engine function and every `Limits` constant it uses as locals and calls only those; the hook
  count, mask, budgets and sentinels of an execution come from locals and per-execution state,
  never from a mutable module table (replacing `Handle.intact`, `Executor.make_hook` or setting
  `Limits.HOOK_COUNT` changes nothing). The corpus runner copies `Hooks` (the intentional test
  seam) when a run starts. `Handle.intact` also compares the full key set of the handle metatable.
  Coroutines made through the swapped `create`/`wrap` are held **strongly** in a list for the
  duration of one execution (capped at 4,096 **alive at once**: when the list is full it is compacted, every dead
  coroutine's hook being checked and the entry dropped; only if 4,096 are still alive does the
  execution end, as a crash of kind `too-many-coroutines`: never a pass, not a timeout, **not
  counted as leaked**, so an ordinary per-token generator parser is unaffected), so one that cleared its hook, finished and was collected is still checked; a wrapped
  coroutine is also checked after every resume.
* **A create/wrap captured in one execution** and called in a *later* one makes unhooked
  coroutines there (the same class as `debug.getupvalue(coroutine.create, ...)` reaching the real
  function): the replacements fall through to the real functions once their execution is over.
* **The residual, stated plainly.** The engine protects against accidental and casual
  interference. A deliberately hostile target in the same Lua state can still defeat it through the
  debug library (`debug.getupvalue`/`setupvalue`/`getregistry`/`getmetatable`) and through code that
  tampers and then restores before returning. Targets are trusted not to attack the harness: the
  same trust model as the Elixir, Haskell and Ruby lanes. An out-of-process runner is tracked as
  follow-up PT-F14.
* **A single C call runs to completion before the hook can fire.** `string.rep` with a
  huge count, a pathological `string.find` pattern, `table.concat`, `table.sort` of a
  huge array with no Lua comparator: none is interrupted, and a huge `string.rep` also
  allocates before any memory poll. The count hook sees VM instructions only.

### 5.4 Leaked executions, capped in every path

An execution is **leaked** when the target escaped the execution contract:
it yielded out of its coroutine, or it replaced or cleared the hook. (Haskell leaks a
*worker thread*; Lua has no worker, so the concept is "the guard failed", counted the
same way and for the same reason: a target that does it once will do it every time, and
a campaign must not keep running a target it cannot bound.) A shared **leak tracker**
counts them. After `MAX_LEAKED = 4` the campaign is **aborted**: the report has
`leaked > 0` and `aborted = true`, is never clean, the verdict reason is
`executions-leaked`, and the public line is `fuzz-timeout` with the signature of
`executions-leaked` (no new failure id: the vocabulary is shared by every lane).

The cap holds in **every path where an execution happens**, because they all go through
one guarded executor (`Engine.guard`): the triage of seeds, the mutation loop, **every
minimisation candidate** (an aborted campaign probes nothing more and the crash keeps
the input it has), and **re-verification** in `verdict`. A report that already leaked
four executions re-verifies nothing (every crash is `unverified`), and re-verification
itself stops, `unverified` for the rest, at the cap. `verdict_for(target, report,
options, target_id, tracker)` wires the executor and the verdict to **one** tracker,
seeded from `report.leaked`; plain `verdict` with an arbitrary executor applies the
report's cap and counts the leaks its own executions produce.

### 5.5 Memory (decision: polled `collectgarbage("count")`, best effort)

Lua has no per-coroutine heap and no allocation limit. The count hook polls
`collectgarbage("count")` (KiB in use by the whole state) and compares the **growth since
the execution began** with `mem_budget` (default 64 MiB, ceiling 1 GiB; polled
against `max(mem_budget, 1 MiB)`). Because the count includes garbage not yet collected,
a count above the budget is first checked again **after a full `collectgarbage("collect")`**;
only live growth over the budget is an `oom`. After a collection that finds live growth under the
budget the next full collection waits until the count has grown by the remaining headroom (live
memory cannot exceed the count), so near-budget churn does not collect on every poll. Stated honestly:

* it is **best effort**: memory is judged once per 1,000 instructions, so a loop that
  allocates a megabyte per iteration can overshoot the budget by about 1,000 instructions'
  worth of allocation before it is seen;
* **a single huge allocation is not stopped**: `string.rep("x", 2^30)` allocates in one C
  call before any poll (§5.3). The cooperative `reserve(h, n)` exists for a target that
  can say what it is about to allocate;
* the growth is whole-state: memory the *engine* allocates during the execution is
  charged to the target (retention is bounded by §6, so this is small);
* the baseline is the count when the execution starts, which includes uncollected garbage:
  when more than 8 MiB has been allocated since the last full collection the engine made, the
  execution starts with one (one collection per 8 MiB allocated, amortised), so the baseline
  error is at most 8 MiB;
* a real allocation failure surfaces as the Lua error string `not enough memory`,
  indistinguishable by `type` from a string a target raised, so it is a crash of kind
  `string` (classification is by `type()` alone, §5.6).

### 5.6 Outcomes and classification

* Outcomes: `pass`, `crash{kind, frames}`, `timeout`, `oom`; each carries an `escaped` flag
  (§5.4).
* **Crash** = any error value escaping the coroutine (other than a budget sentinel of
  this execution), the two protocol failures above, or a stale sentinel. **The kind is
  `type()` of the error value and nothing else**: `string`, `table`, `number`,
  `boolean`, `nil` (`error(nil)`), `function`, `thread`, `userdata`. The engine **never
  calls `tostring`, never reads a field or the message, never applies `#`, `..` or a
  comparison with a metamethod to a target value**: a table error whose `__tostring`
  raises or whose `__index` loops costs nothing. All error values of one `type` raised at
  one place are one crash.
* **Frames.** Read from the dead coroutine's **intact stack** with
  `debug.getinfo(co, level, "Slf")` (an errored coroutine keeps its stack until it is
  closed or collected), innermost first. A frame is `basename#line`, with every character
  outside `[A-Za-z0-9._<>$/#-]` replaced by `_`, the basename capped at 64 characters,
  so **no frame contains `:`** (VER06 §9.4 item 6). C functions, the engine's own files
  and the executor's body frame are dropped, the walk stops at the body, consecutive
  identical frames collapse, the innermost 64 are kept, and at most `MAX_WALK` (4,096)
  levels and 1 MiB of source text are inspected (a hostile chunk name is read once per
  level). A coroutine that ended by a budget has no frames (it is not a crash).
* `crash_signature(kind, frames) = lower32(fnv1a64(kind .. ":" .. join(frames, ":")))` as 8
  lowercase hex digits; real values are pinned (§9).
* `Handle.reserve(h, n)` takes an `Int`; non-positive reserves nothing.
* **Raw fields.** `report.seed` and `report.corpus`, and a crash record's `minimal_input`, are plain
  readable fields: they are redacted only in the printed (`tostring`) and TAP output, and in the
  verdict when `ci` is not `"off"`. Callers must not log them.

## 6. Campaign contracts (VER06 §9.1)

`run_outcomes(initial_corpus, options, exec, extras)` is the outcome-level campaign
(`exec(data) -> outcome, edges`); `run(initial_corpus, options, target)` wraps a target
`function(data, h)` in the executor of §5. Per iteration the stream draws:
`bounded(#corpus)` for the seed; per energy slot `1 + bounded(4)` steps, one `next_u64`
sub-seed, then, with more than one seed, `bounded(4)` and on 0 a donor index and two split
draws. Options (`Options.new{...}`, every field validated, unknown fields refused):
`max_iterations 1000`, `seed 42` (an `Int`), `max_input_len 4096`, `base_energy 10`,
`minimize_budget 2000`, `step_budget 200000`, `mem_budget 64 MiB`, `timeout_ms 5000`,
`instruction_budget 100,000,000`, `ci "unset"`.

**Caps (CWE-400), all enforced before the allocation they bound.** Seeds at most 10,000,
each at most 1 MiB; a candidate is truncated to `max_input_len` (itself at most 1 MiB);
distinct crash signatures at most 1,000 (`crashes_dropped`); **retained bytes** (corpus
seeds plus crash inputs, counting actual string sizes) at most **256 MiB**: an over-cap
admission is counted in `admissions_dropped`, an over-cap crash record keeps no input
(`input_dropped`). The check is made **before** the bytes are stored. Lua strings are
immutable values, so a retained string never pins a larger parent buffer (no slice
copies are needed, unlike Haskell); what is retained is exactly what is counted.

**Minimisation (VER06 §5.3).** Block deletion from a first chunk of `max(1, len // 2)`,
halving to 1, then byte zeroing; exact signature only; `minimize_budget` caps one crash
and the **whole campaign** spends at most `max_iterations` minimisation executions.
**Every candidate execution counts, accepted or rejected**, and the budget is checked
before each one. The candidate sequences are pinned (§9).

**Honest report.** `clean(report)` needs at least one mutation iteration and no crash,
timeout, oom, drop or leaked execution. **Zero executions is never a pass.**

**Verdicts, CI mode and re-verification.** Before a crash is reported its minimal input
is run once more through the guarded executor (one execution per crash, at most 1,000);
the verdict records `reproduced`, `flaky` (the signature did not come back; still failing,
fail closed) or `unverified` (no input kept, or the leak cap) — never `passed`. `passed`
only for a clean report. `ci = "on"` and `"unset"` (an unset mode is CI, fail closed)
carry **no seed and no witness**; only `"off"` fills them.

## 7. Private reporting (VER06 §6, §9.3)

`format_tap_failure(n, target, failure_id, sig)` emits `not ok N - <token>
[<failure-id>:<hex8>]` only. `failure_id` must be registered and `sig` exactly eight
lowercase hex digits, else an error; `n` an `Int` in `1..2^31-1`. **The target token is the
byte grammar of VER06 §9.3 and every other lane:** the bytes of the target are truncated to
128 **bytes** first, then every **byte** outside `[A-Za-z0-9._/-]` becomes `_` (a multi-byte
character becomes one `_` per byte; a cut in the middle of one leaves a single `_`; an
embedded NUL is a byte like any other). Nothing can forge or hide a line or start a `#`
directive. Pinned cross-lane values were produced by running the Ruby lane's
`format_tap_failure` on the same inputs (§9). `tap_failures(report, target, first_test)`
gives one line per crash signature, then one for timeouts, one for ooms and one for leaked
executions. Leaked executions use the registered id `fuzz-timeout` (not `fuzz-leak`) with the
signature of a bare kind, the convention the Haskell lane uses for `workers-leaked`; the Lua kind
is `executions-leaked`. Hashes are `crash_signature("timeout", {})`, `("oom", {})` and
`("executions-leaked", {})`.

## 8. Corpus runner

`Fuzz.Corpus.evaluate` returns the evaluated count and the problems. Strictness and caps are
the shared ones (canonical ints, lowercase even hex, 10,000 elements and steps, 1,000-digit
ints, depth 64, problems clipped to 400); an unknown op, a non-object case, a missing or
duplicate id or a wrong field set is a **problem**, never a skip, and every problem about a
case starts with the case id, escaped. Invariants beyond expected values: flips, arithmetic
and boundary keep the length, splice length is prefix + suffix, admission equals set
difference, no TAP newline, privacy recomputed from the bytes, every op appears, a registered
id is accepted and an unregistered one rejected, exactly 25 cases. Every engine entry point
the runner calls goes through `Hooks` (a table of functions; a test installs a wrong one and
the run must fail). The Go lane's committed vector table is **parsed from that file** by the
conformance suite (no copy): all 820 rows must match.

## 9. Tests, pins and negative controls

* `fuzz-v1`: 25 evaluated, 0 problems; 820 of 820 vectors.
* **Negative controls (the runner must FAIL)** for every case and every corruption its op
  defines (wrong output, wrong admission count, flipped duplicate verdict, flipped validity,
  a tap line changed by a character, a valid TFV in place of a reject text); every problem
  **starts with that case's id**; dropped, duplicated and truncated corpora fail; unknown ops
  and bad field sets fail; every sabotage hook makes the run fail.
* **Pinned values for the corpus blind spots**, cross-checked against the Ruby lane: every
  interest value and delta; unequal splice splits; energy rows including exact ties
  (`10,4,3,6 -> 8`, `3,4,0,0 -> 2`, `5,4,0,0 -> 3`, `25,100,0,0 -> 3`, `1,4,0,0 -> 1`); real
  `sig_hash` values (`panic eval step -> 4eca7ae8`, `segv alloc write -> a0aa3aa2`, `timeout ->
  4e61ea76`, `oom -> 7146a410`, `stale-budget-sentinel -> 8a20b3bf`, `ArgumentError a.rb#x ->
  c93b2322`); the rejection draw (seeds 3 and 4 against `n = 2^63 + 1`, whose first word is
  below the threshold `2^63 - 1`, and `n = 2^64`); the minimiser's candidate sequences for a
  9-byte, a 1-byte, a 3-byte and a budget-cut case.
* **TAP pins** from the Ruby lane: `a#b -> a_b`; `a b c -> a_b_c`; `\n`, `\r` -> `_`; e-acute +
  U+202E + `x` -> `_____x`; 200 `x` -> 128 `x`; 127 `x` + e-acute + `y`s -> 127 `x` + one `_`;
  empty target -> `not ok 3 -  [fuzz-crash:3a8f19b2]`.
* **Hardening tests:** retained, forwarded and forged handles are inert; sticky exhaustion in
  both directions, judged after retirement even when the target swallowed the sentinel with
  `pcall` and returned, raised or yielded; a forged and a stale sentinel are crashes; a runaway
  loop is stopped by the hook (instructions and wall clock); an allocation loop is an `oom`;
  yield and hook-clearing are crashes that count as leaked; the leak cap holds in triage, the
  mutation loop, minimisation and re-verification (`verdict_for` shares the tracker); a
  message-hostile error is classified by type only and no metamethod runs; CI mode carries no
  seed or witness; zero executions is not clean; the retained-bytes cap, the input cap, the
  campaign-wide minimisation budget and the aggregate `mutate_sequence` cap (a breach zeroes
  the allowance) are exact; a 1 MiB flip with a 1,000-digit count is cheap and builds no Big.
* **Bounded tests:** every test runs a target that terminates by construction or is stopped by
  the engine's own budgets (small `timeout_ms` and `instruction_budget`); no test contains a
  target that can only be stopped from outside (§5.3 first bullet is documented, not tested).
* At least 95% luacov line coverage over the whole package.

## 10. Divergences and open items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | explicit edges through the handle; line hooks rejected for the hot path, a future `lua-line-hook` tier |
| 2 | Isolation | one coroutine per execution under a count hook; no kill, no thread; an unbounded `pcall` loop or a cleared hook that never returns cannot be stopped in-process (§5.3) |
| 3 | Memory | polled `collectgarbage("count")` growth (after a full collect), best effort; a single huge C allocation is not stopped (§5.5) |
| 4 | Timeouts | instruction budget **and** `os.clock()` wall bound, both from the count hook; a single C call is not interrupted |
| 5 | Frames | `basename#line` of the dead coroutine's stack (Ruby has labels, Haskell none); error messages never read |
| 6 | Sentinel is catchable | by design; stickiness + post-run verdict + counters cross-check |
| 7 | Leaked executions | yield and hook tampering, cap 4 in every path (§5.4) |
| 8 | Retained-bytes cap, input cap, campaign-wide minimisation budget, aggregate sequence cap | as Java, Ruby, Elixir and Haskell, `2^24` aggregate, `2^28` per call |
| 9 | CI mode | `"unset"` is CI; no seed or witness; crash witnesses re-verified |
| 10 | VER06 §9.4 items 7, 8, 9 | **open**, as in every lane |
| 11 | Corpus weaknesses (PT-F1) | this lane replays Go's 820 vectors and pins sig_hash values, energy ties, constants, unequal splices, the rejection draw and the bisection sequences |
| 12 | Build | `BUILD` and `BUILD_windows` run `busted` over the whole tests directory (pbt-v1 and fuzz-v1 conformance included); `lua/test_bootstrap` is a repository-input consumer of `fuzz-v1` and of Go's vector table in the Go build tool, failing closed when the package is absent |
