# VER06q — Perl Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05p](VER05p-perl-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06i](VER06i-ruby-fuzz-engine.md), [VER06n](VER06n-elixir-fuzz-engine.md), [VER06o](VER06o-haskell-fuzz-engine.md), [VER06p](VER06p-lua-fuzz-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-PL-FUZZ.

This specification defines the Perl 5 implementation of the Level 2 coverage-guided fuzz
engine. There is **one** Perl engine: the module tree
`CodingAdventures::TestBootstrap::Fuzz::*` (facade
`CodingAdventures::TestBootstrap::Fuzz`) of the existing package
`code/packages/perl/test-bootstrap`, beside `Int`, `Prng`, `Safe`, `Canon`,
`StrictJson`, `Values`, `Tfv` and the `pbt-v1` runner of VER05p. It reuses all of them and
duplicates none: `Int` for offsets, counts and edge ids that may be native or
`Math::BigInt`; `Prng` for SplitMix64, FNV-1a and the unsigned rejection threshold;
`Safe::call` for running user code; `Safe::seal` for the read-only globs; `Canon::to_bytes`
for byte strings; the strict corpus plumbing for the runner. It conforms to the `fuzz-v1`
corpus (`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte, replays the Go
lane's 820 oracle vectors, and meets every campaign contract of VER06 §9.1.

The algorithms are those of [VER06i](VER06i-ruby-fuzz-engine.md) and
[VER06p](VER06p-lua-fuzz-engine.md); the hardening list is the one the sibling lanes
converged on after three security-review rounds, **all of it in the first version**. The
letter `q` is the next free one after `p`. This document records only what Perl changes.

Perl is, like Lua, a **single-threaded interpreter with no way to kill a running function**:
code can be interrupted only by an exception thrown from a *signal handler* (delivered
between ops) or by cooperation. Section 5 is therefore the part that is new; the rest is a
port.

---

## 1. Architecture and placement

| Module (`Fuzz::`) | Role |
|---|---|
| `Limits` | the CWE-400 ceilings |
| `Mutate` | mutator algebra, interest lists, `mutate_sequence` (+ counted form) |
| `Bitmap` | 65,536-edge map, `corpus_admit`, `calculate_energy` |
| `Signature` | `crash_signature`, kind and frame normalisation, `format_tap_failure`, `tap_failures` |
| `FailureIds` | the closed failure-id vocabulary |
| `Outcome`, `Options`, `Report` | outcomes, validated options, reports and crash records |
| `Handle` | per-execution coverage and budget handle, sentinels |
| `Executor` | one execution under `Safe::call`, an `alarm` bound and a `__DIE__` frame recorder |
| `Engine` | minimiser, campaign (`run_outcomes`), leak guard |
| `Hooks` | sabotage hooks for the negative controls |
| `Corpus` | production runner for the shared corpus |
| `Fuzz` (facade) | `run`, `run_outcomes`, `executor`, `verdict`, `verdict_for` |

**Core Perl only, no dependencies:** `Scalar::Util`, `B`, `Math::BigInt`, `Time::HiRes`,
`POSIX`, `Sub::Util` (all core; the package already requires 5.26). It reads no file,
environment or network and starts no process. It **does** read a clock and arm a timer: the
per-execution wall bound is a one-shot `Time::HiRes::alarm` and a post-run
`Time::HiRes::time`, so `required_capabilities.json` declares `time:read`.

Every module captures, **at load**, every engine function and every `Limits` constant it
uses as file lexicals and calls only those (no `->can`, no `->isa`, no method dispatch on an
engine package: every internal call is a plain, fully qualified or captured sub call). After
load `CodingAdventures::TestBootstrap::Safe::seal` makes the globs of every new sub
read-only (the existing mechanism of VER05p §4), so `*Fuzz::Mutate::splice = sub {...}` and
`local *Fuzz::Engine::guard` die. Together these mean that replacing, `local`ising or
planting a sub in an engine package changes **nothing the engine does**.

## 2. PRNG and stream alignment

`Prng::Rng::bounded` already draws one word for `n == 1` and rejects words below
`t = (2^64 - n) mod n` computed **unsigned** (`Prng::threshold`), so no wrapper is needed.
The engine obtains its generator through `Prng::new_rng`, so the existing
`Hooks::H{new_rng}` sabotage seam covers it. The corpus never reaches a rejection, so pinned
tests force it (§9).

## 3. Mutator algebra

Exactly VER06 §3 and VER06i §3. Inputs and outputs are Perl byte strings (a string with the
utf8 flag is reduced to bytes: downgraded if every character fits, else its UTF-8 octets,
the rule of `Canon::to_bytes`); a reference, `undef`, or a non-`Int` argument (a float, a
numeric string) is an error. Offsets, counts, deltas and values are `Int`s (native IVs or
normalized `Math::BigInt` up to the corpus's 1,000 digits) and wrap with floored `mod`.

* **No big integer is ever built from input bytes** (the Elixir lane crashed on a
  `2^(8 x 1 MiB)` integer). The input is only ever read as bytes with `substr`/`ord`/`unpack`
  of **at most 8 bytes**; never `unpack 'C*'` of a caller buffer (lesson 93). Arithmetic and
  boundary values are reduced to their **low 64 bits** (`Int::to_u64`), added with the
  wrapping `Prng::add64`, and masked to `8 * width` bits, which is exact because
  `width <= 8`. Offsets are reduced with `Int::mod` against a native modulus. Reads and
  writes use `pack`/`unpack` with `C`, `v`/`n`, `V`/`N`, `Q<`/`Q>`.
* **Flips work by parity and byte-wise.** Bit `i` is bit `i mod 8` of byte `i div 8`, LSB
  first. Every unit is hit `count div units` times and the first `count mod units` units from
  the wrapped start once more; only parity matters. So the work is O(len) for any `count`,
  including a 1,000-digit one: a full inversion is one string XOR with `"\xff" x len`
  (both operands are fresh strings, so Perl's string-XOR, never numeric XOR, applies), and the
  remainder is at most two ranges (the wrap), each masking its two edge bytes.
* `byte_arithmetic` / `boundary_insert`: widths 1, 2, 4, 8 only; any other width, or a buffer
  shorter than the width, returns the input. The offset wraps over `len - width + 1`
  positions.
* `mutate_sequence($data, $seed, $steps, $max_len)` has the Python/Java/Ruby/Elixir/Haskell/Lua
  draw order: per step `bounded(7)`, then that step's draws (flip: index; arithmetic: delta
  index, offset; boundary: value index, offset; delete: length then offset, skipped when
  `len <= 1`; insert: byte then offset; duplicate: length, source, destination, skipped when
  `len <= 1`). Every draw is made into a lexical first. An empty buffer is refilled with `00`
  before every step. An insert or duplicate over `max_len` is skipped **after** its draws.
  The seed is an `Int`, reduced modulo `2^64`.
* **Work cap (CWE-400), charging the actual work.** `steps` above 1,000,000 or an input above
  1 MiB is an error **before anything is allocated**. The bytes a call copies are charged:
  the initial copy, then the current length before each step.
  `mutate_sequence_counted($data, $seed, $steps, $max_len, $cap)` returns `($bytes, $work)`,
  or `(undef, $work)` when the step that would cross the cap is reached; **a failed call is
  charged too**. The per-call cap is `2^28`; the corpus runner's aggregate is `2^24`: each
  case is handed what is left, is charged what it did, and **a breach zeroes the remaining
  allowance**, so every later case fails at once.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1 and VER06i §4.1. The map is a set of edge ids reduced with floored `mod 65536`
(so `-1` is `65535`, and a big is reduced by `Int::mod`). `corpus_admit(\@existing, \@trace)`
counts the *unique* novel edges over the **unreduced** ids; a big is keyed by its decimal
text, so no float ever becomes a key. `calculate_energy(base, len, new, total) = max(1,
floor(base' * (1 + new/total) / sqrt(max(1, len)) + 0.5))` with bonus `1.0` when
`total == 0` and `base' = max(1, base)`; half **away from zero**: `7.5 -> 8`, `2.5 -> 3`.

### 4.2 The coverage source: explicit edges through a handle (decision)

VER06 §7 names a platform source per lane; Perl core has no coverage module (`Devel::Cover`
is not core and is a test tool). The candidates are the debugger hook (`$^P` flags with
`DB::DB`, what `Devel::Cover` and `Devel::NYTProf` use) and target-reported edges.
**This lane does not use `Devel::*`, `$^P` or `DB::DB` as the coverage source:**

1. `DB::DB` is called for **every statement executed** in the target's whole call tree,
   including every core module it loads. That is a 10x to 50x slowdown paid by each of up to
   a million executions.
2. `$^P` and `DB::DB` are **process-global and single-slot**: the package's own line-coverage
   gate (the recorder in `t/lib/LineCov.pm`), `Devel::Cover` and any debugger need the same
   slot. Sharing it would make the fuzz engine and the coverage gate mutually exclusive.
3. Turning the debugger flags on is not free of side effects: `$^P` makes perl retain the
   source lines of every compiled file and eval (`@{"_<$file"}`) and the `%DB::sub` table, a
   memory growth the engine's own budgets cannot see; and a target can clear `$^P` or install
   its own `DB::DB`, which silently drops the signal with no way to observe the loss.
4. Lines are not edges. A line event names a position, not a branch taken; to get AFL-style
   edges the engine would pair consecutive lines, which still cannot see a branch whose two
   sides share a line.

The engine therefore takes **target-reported edges through a handle**, as the Ruby, Elixir,
Haskell and Lua lanes do: the target calls `$h->edge($id)` at the branches it cares about
(`Handle::edge_id("label")` gives the stable slot of an FNV-1a label: the low 16 bits of
`fnv1a64(label)`, the slot every lane reduces the full id to; ids are reduced mod 65,536).
One hash store per edge; nothing is global; no instrumented build. Honest labelling:
`coverage_tier` is `caller-reported` if the target called `edge` at least once during the
campaign, else `black-box-uninstrumented` (the first passing seed is still admitted, so there
is something to mutate).

## 5. Isolation in one interpreter

### 5.1 One call per execution, through `Safe::call`, under an `alarm`

Every execution:

1. makes a **fresh `Handle`** (§5.2);
2. installs `local $SIG{ALRM}` (the *budget handler*) and arms a one-shot
   `Time::HiRes::alarm($timeout_ms / 1000)`; any alarm the caller already had pending is
   remembered and **restored with its remaining time** after the run (so a test's own
   `alarm` wrapper survives, and an outer deadline that is nearer than ours bounds this
   execution instead);
3. runs `target($data, $handle)` through **`Safe::call`**, wrapped in a named closure that
   installs a `local $SIG{__DIE__}` frame recorder (§5.6). `Safe::call` localises `$@`, `$_`
   and the `__DIE__`/`__WARN__` handlers, classifies by `eval` and never by `$@`, and turns
   `last`/`next`/`redo` out of the target into an `escaped` status;
4. disarms the alarm, **retires the handle, then** judges the verdict (§5.2).

The **budget handler** is the wall bound. It runs only while the execution is live (a flag
cleared first thing after the call, retried inside an `eval` until it is down so that a re-armed
timer cannot die outside one; a late delivery is a no-op), marks the execution
exhausted (first breach wins) and `die`s with the execution's **steps sentinel**. It then
**re-arms** a short timer (50 ms) so that a target that swallowed the sentinel with `eval`
and goes on working is interrupted again. The step budget proper is cooperative (`tick`).

**What cannot be stopped (recorded, §10 item 2).** Perl delivers signals **safely**: the C
handler only sets a flag and the Perl handler runs **between ops**. So

* a single long op is not interruptible: a huge `x` repetition, a `sort` of a huge list with
  no Perl comparator, `join`/`pack` of a huge list, and a **pathological regular
  expression** run to completion before the handler can run;
* a target that catches the sentinel in an unbounded loop
  (`while (1) { eval { work() } }`) cannot be stopped in-process: the handler dies *inside*
  the target's own `eval` every 50 ms and the loop continues. The CI job timeout is the only
  backstop: fail closed, but no verdict. (The sticky marker still makes every later `tick`
  die, so a cooperative target stops.) An out-of-process runner is future work (PT-F14).

### 5.2 Handles, sentinels, sticky exhaustion, judged after retirement

A `Handle` is a blessed **empty scalar reference**. Its state (budgets, counters, edge set,
exhausted marker, retired flag) lives in a **file lexical** hash keyed by `refaddr` (inside-out),
never in the object, so a target holding the handle cannot read or rewrite a counter through it.
Two sentinels exist per execution (steps, memory) plus one process-wide **retired**
sentinel. A **sentinel is a unique blessed object** (class `Fuzz::Handle::Sentinel`, an empty
scalar) recognised by **`refaddr`** through a private registry
(`refaddr -> [execution, which]`, cleaned by the sentinel's `DESTROY`, so the registry holds
only live sentinels), **never by message or by content**. An object the target builds itself,
or copies, is not in the registry: it is an ordinary crash of its class (kind `CodingAdventures__TestBootstrap__Fuzz__Handle__Sentinel`, normalised and capped at 64 characters).

* `edge($h, $id)` on a retired, retained or forged handle (an object the registry has no
  state for) is a **no-op** and touches no state. The state entry is removed at retirement.
* `tick($h, $n)` and `reserve($h, $n)` on a retired, retained or forged handle **die with the
  retired sentinel**.
* **Sticky, both directions.** The first breach of the step budget or the memory budget
  (cooperative `tick`/`reserve`, or the alarm) records itself; afterwards **every** `tick` and
  `reserve` dies with the same sentinel again, **whichever budget broke**: `reserve($h, 0)` and
  `reserve($h, 1)` die after a steps breach, `tick($h, 0)` dies after a memory breach (tested
  both ways). `tick($h)` counts one unit, `tick($h, $n)` counts `$n`, `tick($h, 0)` counts
  nothing but still checks. `reserve` takes an `Int`; a non-positive value reserves nothing; a
  big value is a breach.
* **The verdict is judged after retirement.** After the call ends (returned, died, escaped) the
  executor retires the handle **and then** reads the exhausted marker **and cross-checks the
  counters against the budgets** (`steps > step_budget`, `reserved > mem_budget`: counters are
  incremented first and compared after, so an honest breach leaves them over): a breach is a
  `timeout` (steps, wall) or an `oom` (memory) **whatever the target did with the error**. A
  target that caught the sentinel with `eval` and returned, died with something ordinary, or
  left through `last` is never a pass and never a crash.
* **A sentinel the executor did not expect is a crash.** One that surfaces as the execution's
  error while that execution is not exhausted (retained from another execution, or the retired
  sentinel of a stale handle) is a crash of kind `stale-budget-sentinel`, never a timeout.

**Honest limit (recorded).** Everything is in one interpreter. In-process code can reach the
file lexicals only through `PadWalker`-class tricks (not core) but can `delete` stash entries,
`local`ise package variables, or `bless` over the sentinel; the cross-check and `Handle::intact`
(below) catch casual tampering, not a determined in-process attacker. Targets are trusted not
to attack the harness, exactly as in every other lane.

### 5.3 What is detected after the run, and capped

After the call returns the executor checks, **before judging anything**:

* **the alarm.** Only a **replaced handler** (`$SIG{ALRM}` is no longer the budget handler,
  by `refaddr`), or a run that **overran its deadline** (measured with `Time::HiRes::time`, with a
  0.25 s margin) while the budget handler never fired and the timer was found cancelled or
  re-armed for later, is **tampering**: a crash of kind `alarm-tampered` (a timeout, if the budget
  had already broken), never a pass, and **leaked**. A timer found at 0 that never fired, in a run
  that stayed within its deadline, is the honest `eval { local $SIG{ALRM} = ...; alarm N; ...;
  alarm 0 }` idiom and is **not** tampering. **A target that uses `alarm` itself suspends the
  engine's wall-clock bound while its own alarm is active** (there is one timer per process); the
  cooperative step budget and the post-run elapsed check still apply. Where `Time::HiRes::alarm`
  is missing the engine falls back to `CORE::alarm`, which takes whole seconds: the timer is
  rounded **up** and every comparison above uses the rounded span, so an honest execution is
  never tampered (`$Executor::FORCE_INTEGER_ALARM` is the test seam that forces this path);
* **the engine.** `Handle::intact` compares the engine's `Handle` subs, the package `@ISA` and
  the set of subs in the package with their load-time snapshot (a `delete $Fuzz::Handle::{tick}`
  followed by a new `sub tick`, which the read-only globs do not stop, is caught); a difference
  is a crash of kind `engine-tampered`, **leaked**, and `Handle::repair` restores the originals;
* **a fork.** If `$$` changed, this is the *child* of a target that forked and returned into the
  engine; it calls `POSIX::_exit(86)` at once (no `END` blocks, no flush), so the campaign does
  not run twice. The parent cannot see its child; a child that lingers is the target's own
  business.

`exit`, `exec` and `kill` inside a target end or replace the process and are **not contained**
(an `END` block of the host still runs), as `exit` is in the PBT engine. A target that restores
the exact handler and timer it tampered with before returning, or that tampers and then
never returns, is the residual (the first bullet of §5.1 applies).

### 5.4 Leaked executions, capped in every path

An execution is **leaked** when the target escaped the execution contract: it replaced or
cancelled the alarm, or tampered with the engine's own subs. A shared **leak tracker** counts
them. After `MAX_LEAKED = 4` the campaign is **aborted**: the report has `leaked > 0` and
`aborted`, is never clean, the verdict reason is `executions-leaked`, and the public line is
`fuzz-timeout` with the signature of the bare kind `executions-leaked` (no new failure id: the
vocabulary is shared by every lane, and the Haskell and Lua lanes do the same).

The cap holds in **every path where an execution happens**, because they all go through one
guarded executor (`Engine::guard`): the triage of seeds, the mutation loop, **every
minimisation candidate** (an aborted campaign probes nothing more and the crash keeps the input
it has), and **re-verification** in `verdict`. A report that already leaked four executions
re-verifies nothing (every crash is `unverified`), and re-verification itself stops,
`unverified` for the rest, at the cap. `verdict_for($target, $report, $options, $target_id,
$tracker)` wires the executor and the verdict to **one** tracker, seeded from `report->{leaked}`;
plain `verdict` with an arbitrary executor applies the report's cap and counts the leaks its own
executions produce.

### 5.5 Memory (decision: cooperative `reserve` accounting only)

Perl has no per-interpreter heap limit and no allocation hook in core. The honest options were
(a) cooperative accounting, (b) polling the process RSS from `/proc/self/statm` (Linux only) or
`ps` (a child process) on the alarm tick, (c) `setrlimit` (`BSD::Resource`, not core). This lane
takes **(a) alone**:

* `reserve($h, $n)` declares an allocation about to be made on behalf of the input; past
  `mem_budget` (default 64 MiB, ceiling 1 GiB) the execution is an `oom`, sticky, judged after
  retirement. A target that can say what it is about to allocate is bounded exactly.
* An RSS poll would need a file or process capability for every campaign, works on one
  platform, sees whole-process growth (the engine's own retained corpus included), and **still
  cannot interrupt the single op that allocates** (`"x" x 2**31` allocates before any signal can
  run). It is the same best-effort, post-hoc bound at a higher price, so it is not built.
* **A single huge allocation is not stopped**, and a loop that allocates without ever calling
  `reserve` is bounded only by the wall clock. A real allocation failure aborts the process
  (`Out of memory!`); it is not catchable.

### 5.6 Outcomes and classification

* Outcomes: `pass`, `crash{kind, frames}`, `timeout`, `oom`; each carries an `escaped` flag
  (§5.4). Only the `Outcome` module builds one and `Outcome::is` recognises them by `refaddr`
  in a private registry, so an outcome a test `exec` fabricates is refused.
* **Crash** = any error value escaping the call (other than a budget sentinel of this
  execution), an `escaped` status, the protocol failures above, or a stale sentinel. **The
  kind comes from `ref`, `blessed` and `reftype` and nothing else:** `undef`, `string` (a plain
  message), the **class name** for a blessed value, or the `reftype` (`HASH`, `ARRAY`, `CODE`,
  ...), each reduced to `[A-Za-z0-9._<>$/-]` and 64 characters (so `Foo::Bar` is `Foo__Bar`). The
  engine **never stringifies an error value, never reads a field of it, never numifies it, and
  never applies a comparison or a method to it**: an object whose `""`, `eq`, `bool`, `==`,
  `0+`, `@{}` or `%{}` overloads die, or a tied value, costs nothing. All error values of one kind
  raised at one place are one crash.
* **Frames.** A `local $SIG{__DIE__}` recorder runs at the **die site** (before unwinding) and
  walks `caller` outwards, innermost first, up to the named executor closure. A frame is
  `basename#line` with every character outside `[A-Za-z0-9._<>$/#-]` replaced by `_` and
  `(eval N)` normalised to `eval`, so **no frame contains `:`** (VER06 §9.4 item 6). Frames in
  the engine's own files are dropped, consecutive identical frames collapse, the innermost 64
  are kept, and at most `MAX_WALK` (4,096) levels are inspected. The recorder reads **no**
  argument: the error is never touched. The **last** `die` before the error escapes wins (a
  `die` inside a destructor run while unwinding would replace it); a target that installs its
  own `local $SIG{__DIE__}` hides its frames (the signature is then the bare kind). A budget
  breach has no frames.
* `crash_signature(kind, frames) = lower32(fnv1a64(kind . ":" . join(":", frames)))` as 8
  lowercase hex digits; real values are pinned (§9).
* **Raw fields.** `seed`, `corpus` and a crash record's `minimal_input` are **private**
  (VER06 §6.3): they live in inside-out tables behind accessors (`Report::seed`,
  `Report::corpus`, `$rec->minimal_input`), are never in a printed (`""`) form, and the verdict
  carries them only when `ci` is `off`. Callers must still not log what an accessor returns.

## 6. Campaign contracts (VER06 §9.1)

`run_outcomes(\@initial_corpus, $options, $exec, \%extras)` is the outcome-level campaign
(`$exec->($data) -> ($outcome, \@edges)`); `run(\@initial_corpus, $options, $target, \%flags)`
wraps a target `sub ($data, $handle)` in the executor of §5. Per iteration the stream draws:
`bounded(#corpus)` for the seed; per energy slot `1 + bounded(4)` steps, one `next_u64`
sub-seed, then, with more than one seed, `bounded(4)` and on 0 a donor index and two split
draws. Options (`Options->new({...})`, every field validated, unknown fields refused):
`max_iterations 1000`, `seed 42` (an `Int`), `max_input_len 4096`, `base_energy 10`,
`minimize_budget 2000`, `step_budget 200000`, `mem_budget 64 MiB`, `timeout_ms 5000` (1 to
60,000), `ci "unset"`. (There is no `instruction_budget`: Perl has no instruction hook.)

**Caps (CWE-400), all enforced before the allocation they bound.** Seeds at most 10,000, each
at most 1 MiB; a candidate is truncated to `max_input_len` (itself at most 1 MiB); distinct
crash signatures at most 1,000 (`crashes_dropped`); **retained bytes** (corpus seeds plus crash
inputs, counting actual string lengths) at most **256 MiB**: an over-cap admission is counted
in `admissions_dropped`, an over-cap crash record keeps no input (`input_dropped`). The check is
made **before** the bytes are stored. Perl `substr` copies, so a retained string never pins a
larger parent buffer; what is retained is exactly what is counted.

**Minimisation (VER06 §5.3).** Block deletion from a first chunk of `max(1, len div 2)`, halving
to 1, then byte zeroing; exact signature only; `minimize_budget` caps one crash and the **whole
campaign** spends at most `max_iterations` minimisation executions. **Every candidate execution
counts, accepted or rejected**, and the budget is checked before each one. The candidate
sequences are pinned (§9).

**Honest report.** `clean($report)` needs at least one mutation iteration and no crash, timeout,
oom, drop or leaked execution. **Zero executions is never a pass.**

**Verdicts, CI mode and re-verification.** Before a crash is reported its minimal input is run
once more through the guarded executor (one execution per crash, at most 1,000); the verdict
records `reproduced`, `flaky` (the signature did not come back; still failing, fail closed) or
`unverified` (no input kept, or the leak cap) — never `passed`. `passed` only for a clean report.
`ci = "on"` and `"unset"` (an unset mode is CI, fail closed) carry **no seed and no witness**;
only `"off"` fills them.

## 7. Private reporting (VER06 §6, §9.3)

`format_tap_failure($n, $target, $failure_id, $sig)` emits `not ok N - <token>
[<failure-id>:<hex8>]` only. `failure_id` must be registered and `sig` exactly eight lowercase
hex digits, else an error; `n` an `Int` in `1..2^31-1`. **The target token is the byte grammar of
VER06 §9.3 and every other lane:** the bytes of the target are truncated to 128 **bytes** first,
then every **byte** outside `[A-Za-z0-9._/-]` becomes `_` (a multi-byte character becomes one
`_` per byte; a cut in the middle of one leaves a single `_`; an embedded NUL is a byte like any
other; a string with the utf8 flag is reduced to bytes by `Canon::to_bytes` first). Nothing can
forge or hide a line or start a `#` directive. Pinned cross-lane values were produced by running
the Ruby lane's `format_tap_failure` on the same inputs (§9). `tap_failures($report, $target,
$first_test)` gives one line per crash signature, then one for timeouts, one for ooms and one for
leaked executions. Leaked executions use the registered id `fuzz-timeout` (not `fuzz-leak`) with
the signature of a bare kind; the Perl kind is `executions-leaked`. Hashes are
`crash_signature("timeout", [])`, `("oom", [])` and `("executions-leaked", [])`.

## 8. Corpus runner

`Fuzz::Corpus::evaluate` returns the evaluated count and the problems. Strictness and caps are
the shared ones (canonical ints, lowercase even hex, 10,000 elements and steps, 1,000-digit ints,
depth 64, problems clipped to 400); an unknown op, a non-object case, a missing or duplicate id or
a wrong field set is a **problem**, never a skip, and every problem about a case starts with the
case id, escaped. Invariants beyond expected values: flips, arithmetic and boundary keep the
length, splice length is prefix + suffix, admission equals set difference, no TAP newline, privacy
recomputed from the bytes, every op appears, a registered id is accepted and an unregistered one
rejected, exactly 25 cases. Every engine entry point the runner calls goes through `Fuzz::Hooks`
(a hash of subs, **copied when a run starts**; a test installs a wrong one and the run must fail).
The Go lane's committed vector table is **parsed from that file** by the conformance test (no
copy): all 820 rows must match.

## 9. Tests, pins and negative controls

* `fuzz-v1`: 25 evaluated, 0 problems; 820 of 820 vectors.
* **Negative controls (the runner must FAIL)** for every case and every corruption its op
  defines (wrong output, wrong admission count, flipped duplicate verdict, flipped validity, a tap
  line changed by a character, a valid TFV in place of a reject text); every problem **starts with
  that case's id**; dropped, duplicated and truncated corpora fail; unknown ops and bad field sets
  fail; every sabotage hook makes the run fail.
* **Pinned values for the corpus blind spots**, cross-checked against the Ruby lane (through the
  Lua lane's pin file, which was produced from it) and re-run through the Ruby lane here: every
  interest value and delta; unequal splice splits; energy rows including exact ties
  (`10,4,3,6 -> 8`, `3,4,0,0 -> 2`, `5,4,0,0 -> 3`, `25,100,0,0 -> 3`, `1,4,0,0 -> 1`); real
  `sig_hash` values (`panic eval step -> 4eca7ae8`, `segv alloc write -> a0aa3aa2`, `timeout ->
  4e61ea76`, `oom -> 7146a410`, `stale-budget-sentinel -> 8a20b3bf`, `ArgumentError a.rb#x ->
  c93b2322`); the rejection draw (seeds 3 and 4 against `n = 2^63 + 1`, whose first word is below
  the threshold `2^63 - 1`, and `n = 2^64`); the minimiser's candidate sequences for a 9-byte, a
  1-byte, a 3-byte and a budget-cut case; three whole campaigns driven by one deterministic target.
* **TAP pins** from the Ruby lane's `format_tap_failure`: `a#b -> a_b`; `a b c -> a_b_c`; `\n`, `\r`
  -> `_`; e-acute + U+202E + `x` -> `_____x`; 200 `x` -> 128 `x`; 127 `x` + e-acute + `y`s -> 127
  `x` + one `_`; empty target -> `not ok 3 -  [fuzz-crash:3a8f19b2]`.
* **Hardening tests:** retained, forwarded and forged handles are inert (`edge` a no-op, `tick`
  and `reserve` die with a sentinel compared by `refaddr`); handle state is not in the object;
  sticky exhaustion in both directions, judged after retirement even when the target swallowed
  the sentinel with `eval` and returned, died or left through `last`; a forged and a stale sentinel
  are crashes; a runaway loop is stopped by the alarm; a replaced handler, and a cancelled or re-armed timer that let the run overrun its deadline, are
  tampered executions that count as leaked, while the honest `alarm` idiom is a pass and the
  whole-second fallback is exercised through its seam; a planted `Handle` sub is caught and
  repaired; a forked child exits; the leak cap holds in triage, the mutation loop, minimisation and
  re-verification (`verdict_for` shares the tracker); a message-hostile error (overloads and a
  tie that die) is classified without running them; CI mode carries no seed or witness; zero
  executions is not clean; the retained-bytes cap, the input cap, the campaign-wide minimisation
  budget and the aggregate `mutate_sequence` cap (a breach zeroes the allowance) are exact; a 1 MiB
  flip with a 1,000-digit count is cheap and builds no bignum from bytes; an outer `alarm` survives
  an execution; no engine source uses `->can` or `->isa`; the new subs are sealed.
* **Bounded tests:** every test runs a target that terminates by construction or is stopped by the
  engine's own budgets (small `timeout_ms`, small `step_budget`), and runs under the `bounded_by`
  alarm of the test helper as a second net; no test contains a target that can only be stopped from
  outside (the unbounded `eval` loop of §5.1 is documented, not tested).
* At least 95% line coverage of the Fuzz tree, measured with the in-repo recorder
  `t/lib/LineCov.pm` (no `Devel::Cover` dependency; run with `PERL5DB`, see the README). The
  recorder is a debugger (`DB::DB` plus the saved source arrays): the breakable lines are
  perl's own, the hits are what `DB::DB` sees. Perl gives the single statement of an
  `if`/`elsif` body that calls a sub no statement of its own (it is attributed to the
  condition's line), and an `elsif` condition is never reported, so the recorder **undercounts**:
  the figure is a lower bound. (This is a test tool only; section 4.2 still holds.)

## 10. Divergences and open items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | explicit edges through the handle; `Devel::*`/`$^P`/`DB::DB` rejected for the hot path |
| 2 | Isolation | one `Safe::call` per execution under a one-shot `alarm` (safe signals: delivered between ops); a long single op, a pathological regex, and an unbounded `eval` loop cannot be stopped in-process (§5.1) |
| 3 | Memory | cooperative `reserve` accounting only; no RSS poll, no `setrlimit`; a single huge allocation is not stopped (§5.5) |
| 4 | Timeouts | wall clock only (`Time::HiRes::alarm`, 1 ms to 60 s); the step budget is cooperative `tick`; there is no instruction budget |
| 5 | Frames | `basename#line` of the die site and its callers from a `__DIE__` recorder; error messages are never read |
| 6 | Sentinel is catchable | by design; stickiness + post-run verdict + counters cross-check |
| 7 | Leaked executions | tampered alarm and tampered engine, cap 4 in every path (§5.3, §5.4); a forked child exits; `exit`/`exec`/`kill` are not contained |
| 8 | Retained-bytes cap, input cap, campaign-wide minimisation budget, aggregate sequence cap | as Java, Ruby, Elixir, Haskell and Lua, `2^24` aggregate, `2^28` per call |
| 9 | CI mode | `"unset"` is CI; no seed or witness; crash witnesses re-verified |
| 10 | VER06 §9.4 items 7, 8, 9 | **open**, as in every lane |
| 11 | Corpus weaknesses (PT-F1) | this lane replays Go's 820 vectors and pins sig_hash values, energy ties, constants, unequal splices, the rejection draw and the bisection sequences |
| 12 | Trust model | cooperative: targets are trusted not to attack the harness (stash surgery, `PadWalker`, `exit`); out-of-process runner tracked as PT-F14 |
| 13 | Build | `BUILD` runs `prove -l -v t/` over the whole test directory (pbt-v1 and fuzz-v1 conformance included); `perl/test-bootstrap` is a repository-input consumer of `fuzz-v1` and of Go's vector table in the Go build tool, failing closed when the package is absent |
