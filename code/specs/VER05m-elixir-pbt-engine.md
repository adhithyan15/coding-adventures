# VER05m — Elixir Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md), [VER05h](VER05h-ruby-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-EX-PBT.

This specification defines the Elixir implementation of the Level 2 Property-Based
Testing (PBT) engine. Elixir had no test-bootstrap and no property engine before
this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1.

(Letters `j`, `k` and `l` belong to the C++, Swift and F# lanes, which are open
on their own branches; `m` is the next free letter.)

Elixir is dynamically typed with bignum integers like Ruby, so the code follows
[VER05h](VER05h-ruby-pbt-engine.md). The hardening and the shrink budget follow
[VER05e §4.1](VER05e-java-pbt-engine.md) and [VER05g §3.1](VER05g-csharp-pbt-engine.md).
**The hierarchical budget tree is part of the first version**, not a later fix.
This document does not repeat the algorithms; it states what is the same (§1),
what the BEAM changes (§2 to §4) and the tests that pin it (§6).

---

## 1. Same as Java, C# and Ruby (normative by reference)

VER05e §2 (PRNG: SplitMix64, `bounded`, `bounded_big`, FNV-1a, `sub_seed`;
`bounded(1)` consumes one word), §3 (generator algebra and **draw order**), §4
(integrated shrinking, same phase order, `steps` counts accepted moves only), §4.1
and VER05g §3.1 (budget tree), §5 (runner, closed failure vocabulary, verdict
honesty, witness re-verification, CI redaction; an unset CI mode is CI) and §6
(conformance runner, strict decoding, CWE-400 caps, negative controls).

## 2. Package and namespaces

`code/packages/elixir/test_bootstrap` (underscore, like most packages of the
lane; the build tool names it `elixir/test_bootstrap`), OTP application
`:coding_adventures_test_bootstrap`, root module `CodingAdventures.TestBootstrap`,
Elixir `~> 1.15`. Zero dependencies, no NIF, no `:make` compiler.

| Module | Role |
|---|---|
| `Prng` | `new/1`, `next_u64/1`, `bounded/2`, `bounded_big/2`, `fnv1a64/1`, `sub_seed/2` |
| `Pbt.Gen`, `Pbt.Gens` | generator algebra |
| `Pbt.Shrink` | integer, sequence, byte shrinkers, `simulate_replay` |
| `Pbt.Budget` | the budget tree and its carriers |
| `Pbt.Runner`, `Pbt.Options`, `Pbt.Verdict`, `Pbt.FailureIds`, `Pbt.Limits`, `Pbt.Hooks` | runner |
| `Pbt.Corpus` | the pbt-v1 conformance evaluator |
| `CorpusJson.StrictJson`, `.Values`, `.Canon`, `.Clip` | strict decoding and canonical rendering |
| `Tfv` | strict TFV v1 validator (checks `corpus_reject` cases with a real parser) |
| `Safe` | exception, throw and exit classification |

The library does no file, clock, network or environment access, so
`required_capabilities.json` is capability-free. Tests read the committed fixture
by walking up to the repository root.

## 3. BEAM specifics

- **Integers are unbounded.** `import Bitwise`; every add, multiply and shift in
  SplitMix64 and FNV-1a is followed by `band(_, 0xFFFF_FFFF_FFFF_FFFF)`. The
  rejection threshold is `rem(2**64 - n, n)`. `bounded(0)` and `n > 2**64` raise
  `ArgumentError`; `bounded(2**64)` is one raw word. Seeds are any integer reduced
  modulo 2^64 (`band` on a negative integer gives the two's-complement wrap, as in
  the reference); booleans and floats are an `ArgumentError`.
- **Immutable generators.** The RNG is a value (`%Prng{state: s}`); a sampler is
  `(rng -> {value, rng})` and threads the stream explicitly. Draw order is the
  cross-lane contract.
- **Value model.** Integers (also u8, u64, i128), binaries are bytes, lists are
  lists, `Gens.tuple/1` yields a tuple (arity at most 64). Decoded corpus text is
  tagged `{:text, binary}` so equality tells bytes from text.
- **Property convention (decision).** A property is a function. It **holds iff it
  returns exactly `true`.** `false`, `nil`, `:ok` and every other term falsify:
  fail-closed, so a function that forgets its return value cannot pass
  vacuously. This is stricter than Ruby's truthiness; an `ExUnit` assertion
  property must end in a boolean expression.
- **Raise, throw, exit (decision).** The runner calls a property inside
  `try ... catch kind, value` and treats **all three kinds** (`:error`, `:throw`,
  `:exit`) the same way, without inspecting the value to decide: in the case loop
  it is `property-panic`; inside shrinking (and re-verification) it means "this
  candidate does not falsify" and is never accepted as smaller. Sample-time
  failures of any kind are `property-exhausted`. Redacted messages name only the
  kind and, for an exception struct, its module; local messages render the value
  with `Canon` (structural, never `inspect`). What is not catchable is not
  handled: a brutal `Process.exit(pid, :kill)` or a heap limit ends the process,
  so a caller that needs a wall-clock bound runs the check in its own process
  (the tests do, with `@tag timeout`).
- **Rendering.** `Canon.render/1` is total and never calls `inspect`, `to_string`
  or any user protocol: it walks terms structurally. Integers above 100,000 bits
  are `<bigint N bits>`; binaries that are valid UTF-8 are quoted with every
  control character escaped (C0, DEL, C1, U+2028/9, and the bidirectional
  controls U+061C, U+200E/F, U+202A-E, U+2066-9 as `\uXXXX`), others are
  `0x<hex>`; atoms, tuples `{}`, lists `[]`, maps `%{}` and structs `%Mod{}` are
  rendered with **maps and sets sorted by the bytes of their rendered key**, so a
  digest does not depend on iteration order; pids, refs, ports and functions are
  fixed placeholders. Depth deeper than 16 is `...`, more than 10,000 nodes or
  1 MiB of output truncates (UTF-8 safe); caps are applied before allocation
  (binaries are clipped to the remaining output budget, maps and sets read at most
  10,001 entries). Invisible formatting characters (U+00AD, U+034F, U+180E,
  U+200B-D, U+2060-4, U+FEFF, U+FFF9-B, the tag block) are escaped too. A failure while rendering degrades the
  whole witness to `<unrenderable>`. The CI digest is FNV-1a of
  `iter:<i>;steps:<s>;shrunk:<canonical>`, 16 hex digits.
- **JSON.** OTP 27's `:json` is not used: it keeps the last of two duplicate keys
  and turns numbers into floats unless callbacks are replaced, which is most of a
  parser. `StrictJson` is hand-written: nesting deeper than 64 (the 65th refused
  before recursing), documents over 8 MiB, more than 200,000 nodes, duplicate keys
  (compared as bytes), invalid UTF-8 and a BOM, raw control characters in strings,
  lone surrogate escapes, leading zeros, `+1`, `NaN`, comments, trailing commas
  and trailing data are refused. Numbers stay text (`{:number, text}`).
- **Predicates.** A `filter/3` predicate accepts only by returning exactly `true`,
  like a property.
- **Hardening carried from Java, C# and Ruby.** `Limits`: `max_gen_count` 1,000,000;
  `gen_bytes` capped even when `max_len == 0`; `gen_bounded` bounded by
  `count x ceil(bitlen/8) <= 2^20`; `max_retries` at most 10,000; `max_steps`
  validated; CI-redacted verdicts carry no seed, witness or minimal.
- **Sabotage hooks.** `Pbt.Hooks` holds the bisection move, the unit move, a flag
  that skips the deletion phase, the RNG factory and the default root work cap.
  They live in the **process dictionary** of the calling process and are set with
  `Hooks.with(overrides, fun)`, which restores them in `after`; a test swaps one,
  runs the corpus and expects problems. Production code never sets them.

### 3.1 Shrink budget: a tree of scopes

The design is VER05g §3.1 line for line; this section states the BEAM mapping.

**The tree.** A node (`Pbt.Budget`) is an immutable struct with a `parent`, a
`depth`, caps (`max_attempts`, `max_work`) and a `:atomics` ref holding four
counters: attempts (one per property evaluation, cap `16 * max_steps + 64`),
work (elements copied, at least 1 per charge, cap `Limits.max_shrink_work` =
64M), a `closed` flag and, on the root, a `refused` flag.
`charge/2` (an attempt) and `charge_copy/2` (work only, so the corpus is
unchanged) are **lock-free reserve-and-rollback**: walk from the node to the
root; on each node run a compare-and-swap loop on the attempts and work cells
(read used; refuse if `cost > max - used`, or attempts `>= max_attempts`; else
`compare_exchange(used, used + cost)`, retrying when another process won, which
is lock-free because a failed CAS means someone made progress). A refusal at any
node subtracts every reservation of that walk again (the partial one on the
current node included), sets the root's `refused` flag and returns false. The
semantics: all or nothing per charge; a concurrent charge may transiently see
another charge's reservation that is later rolled back, so it may refuse close to
the cap (fail closed, never open); counters never exceed a cap at any node; there
is no lock and no hang. A process killed mid-walk can only leak reservations
(over-counting, conservative); there is no stuck state. Callers charge
**before** building a candidate; once refused, every loop ends and a sequence
shrinker returns its input. A closed node, or a closed node anywhere on the path,
refuses every charge, and `room?/1` is plain reads over the path.

**Scopes.** `enter_child/2` (public entry points: `Gen.shrink`,
`Shrink.shrink_int/shrink_list/shrink_bytes/simulate_replay`, the runner's shrink)
opens a child of the ambient scope with a fresh local cap, or a fresh root when
none is open; at depth 64 it joins instead of nesting. `enter/2` (internal
combinators) joins the ambient scope. `enter_check/3` wraps the **whole** body of
`Runner.check` with an attempt cap of `num_tests + 16 * max_steps + 64`. A check
nested in a property pays 1 attempt at entry (refused: `property-exhausted`
without evaluating anything), 1 per case after the first, the sampled value's size
as work (binaries count their bytes; a list, map or tuple its length plus, one
level deeper, the length of each such element, visiting at most 10,000 elements;
anything else 0), and 1 attempt per `filter` retry. A **top-level** check always
evaluates its first case; its filter retries are charged as **work**. If any
charge in its tree was refused, a top-level check that would pass returns
`property-exhausted`; a falsified verdict is never overridden. The single
re-verification charges the check scope; if refused, the original witness is
reported with 0 steps. A closed scope resolves to the nearest open ancestor; a handle installed with
`with_handle/2` whose whole chain is closed is **spent** (`current/0` returns it
and every charge refuses), never a way to obtain a fresh root. A node stores only its
parent, never a separate "previous ambient" node: a second reference to the same
ancestor would make the term twice as large per level, so copying or comparing a
depth-64 handle (as `Budget.async/1` does) would never finish. `close/1` restores
the parent, which resolves identically.

**The carrier (decision).** BEAM processes share nothing, so the budget needs
(a) a place for the *ambient* node and (b) a way to reach other processes.

1. *Ambient node:* the process dictionary key `:coding_adventures_pbt_budget` of
   the process running the check. `enter*` saves the previous value and restores
   it first thing in an `after`, so a raise, throw or exit cannot leave a stale
   ambient node.
2. *Shared counters:* `:atomics`, which any process holding the ref can charge
   atomically, so the tree is one global budget however many processes hold a
   handle to it.
3. *Spawned processes (the escape, pinned by a test).* The process dictionary does
   **not** inherit: a plain `spawn`, `Task.async` or `GenServer` started inside a
   scope has no ambient node, so a `check` or shrinker running there opens a
   **fresh root**. The bound is then **per process flow**: each such flow is
   bounded by its own root. This is the same escape class as Swift's thread
   escape in VER05k and is documented rather than hidden. To stay inside the
   budget, carry the handle explicitly: `Budget.capture/0` returns the usable
   ambient node (or `nil`), `Budget.with_handle/2` installs it in the receiving
   process for the duration of a function, and the helpers `Budget.spawn/1`,
   `Budget.spawn_link/1` and `Budget.async/1` do both. A process that carries the
   handle charges the same tree, also after the spawning scope has closed (the
   handle resolves to the nearest open ancestor, or to a fresh root when the whole
   chain has closed).

## 4. Divergences from Java, C#, Python and Ruby

| # | Topic | Elixir | Others | Reason |
|---|---|---|---|---|
| 1 | property result | only `true` holds | Ruby: truthy; Python: only `False` fails | fail-closed idiom |
| 2 | raise kinds | `:error`, `:throw`, `:exit` all caught | exceptions only | BEAM has three |
| 3 | ambient carrier | process dictionary + `:atomics` | fiber storage / `AsyncLocal` | no inheritance on BEAM |
| 4 | RNG | immutable value, sampler returns `{value, rng}` | mutable object | functional language |
| 5 | locking | none: CAS reserve and rollback | `Mutex` / `lock` | a killed holder must not hang or starve the tree |
| 6 | flat map | `Gens.bind/2` (alias `and_then/2`) | `flat_map` | no clash in Elixir |
| 7 | tuples | `Gens.tuple/1` yields a tuple | `Pair`/`Triple`/Array | native tuples |
| 8 | rendering | structural, no `inspect` | `inspect` for unknown objects | user protocols can loop |
| 9 | hooks | process dictionary, `Hooks.with/2` | module accessors | no global mutable module state |

## 5. Build and inputs

`BUILD` and `BUILD_windows` run `mix test --cover`; `mix.exs` sets
`test_coverage: [summary: [threshold: 95]]`. The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `elixir/test_bootstrap` is
registered as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`) beside python, go, java, kotlin, c, csharp and ruby; the
existing selection tests that enumerate every consumer fail closed until Elixir
is added to them. `required_capabilities.json` is empty.

## 6. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems.
- Negative controls: for **every** case and every corruption its op defines
  (flipped output, wrong minimal, steps + 1, wrong failure id, flipped validity, a
  valid TFV text in place of a reject text), the run fails **and every problem
  starts with that case's id and no other**; dropped and truncated corpora fail; an
  unknown op fails; every engine hook makes the corpus run fail.
- Pinned tests for what the corpus cannot catch: a seed whose first draw lies
  below the rejection threshold (the finalizer is inverted in the test to build
  states that produce chosen words), the `2^63 + 1` threshold boundary (a draw of
  exactly the threshold is kept, one below is rejected), and bisection accepting
  the right half and then the left half with exact step counts.
- Budget-tree tests, the required set of VER05e's `ShrinkBudgetTest`: nesting depth
  1 to 3 plus the branching case, a recursing custom shrinker, a filter predicate
  that shrinks, a filter sampler rejecting everything, spawned processes in a scope,
  concurrent charging, a refused take leaving every counter unchanged, a huge cost,
  ambient restore after a raise, depth-cap joining, re-entrancy equality,
  starvation never passing, refused re-verification. BEAM-specific: a nested check
  in a plain spawned process (fresh root, pinned), in a process carrying the handle
  (charges the root), `throw` and `exit` in a property, shrink candidate and
  generator, a reservation leaked by a killed process refusing quickly, concurrent charging
  on a shared tree, a closed handle being spent.
- A regression test per hardening item of §3.
- At least 95% line coverage, enforced by `test_coverage` in `mix.exs`.
