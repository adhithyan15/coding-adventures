# VER05h — Ruby Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05c](VER05c-python-pbt-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-RB-PBT.

This specification defines the Ruby implementation of the Level 2 Property-Based
Testing (PBT) engine. Ruby had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1.

Ruby is dynamically typed like Python, so the shape of the code follows
[VER05c](VER05c-python-pbt-engine.md). The hardening, and above all the shrink
budget, follow [VER05e §4.1](VER05e-java-pbt-engine.md) and
[VER05g §3.1](VER05g-csharp-pbt-engine.md), which went through several security
review rounds in their lanes. **The hierarchical budget tree is part of the first
version here**, not a later fix: earlier lanes needed six review rounds to reach
it by patching. This document therefore does not repeat the algorithms; it states
what is the same (§1), what Ruby changes (§2 to §4) and the tests that pin it (§6).

---

## 1. Same as Java and C# (normative by reference)

The following VER05e sections apply to the Ruby engine unchanged, with Ruby
names substituted:

- §2 PRNG: SplitMix64, `bounded`, `bounded_big`, `fnv1a64`, `sub_seed`
  (`SplitMix64.new(seed ^ fnv1a64(utf8(id))).next_u64`); `bounded(1)` consumes one word.
- §3 generator algebra and **draw order** (the cross-lane contract).
- §4 integrated shrinking, with the same phase order and the rule that `steps`
  counts accepted moves only, so the corpus step counts hold.
- §4.1 and VER05g §3.1: the budget tree (restated for Ruby in §3.1 below).
- §5 runner, closed failure vocabulary, verdict honesty, re-verification of the
  witness, CI redaction (`:unset` is treated as CI).
- §6 conformance runner, strict decoding, CWE-400 caps, coverage gates, negative controls.

## 2. Package and namespaces

`code/packages/ruby/test_bootstrap`, gem `coding_adventures_test_bootstrap`,
`require "coding_adventures_test_bootstrap"`, Ruby >= 3.2 (Fiber storage; the
repository pins 3.4). One gem, four modules under `CodingAdventures::TestBootstrap`:

| Module | Role |
|---|---|
| `Prng` | `SplitMix64`, `bounded`, `bounded_big`, `fnv1a64`, `sub_seed` |
| `Pbt` | `Gen`, `Gens`, `Shrink`, `Runner`, `Verdict`, `Options`, `FailureIds`, `Limits`, `ShrinkBudget`, `Hooks`, `Corpus` |
| `CorpusJson` | `StrictJson`, `Values`, `Canon`, `Clip` |
| `Tfv` | strict TFV v1 validator, so `corpus_reject` cases are checked by a real parser |

Ruby core only (`Mutex`, `Fiber`, `Thread`). No runtime gem dependency; `minitest`
and `simplecov` are test-time only. The library performs no file, clock, network
or environment access, so `required_capabilities.json` is capability-free. Tests
read the committed fixture by walking up to the repository root, never a copy.

## 3. Ruby specifics

- **Integers are unbounded.** Ruby `Integer` is a bignum, so nothing overflows and
  nothing wraps by itself. SplitMix64 and FNV-1a therefore mask with
  `& 0xFFFF_FFFF_FFFF_FFFF` after **every** add and multiply. The rejection
  threshold is `(2**64 - n) % n` (spelled out: `(-n) % n` is `0` in Ruby, as in
  Python, and would never reject). `bounded(0)` and `n > 2**64` raise
  `ArgumentError`; `bounded(2**64)` is one raw word. Seeds are any `Integer`
  (`true`, `false`, a `Float` are a `TypeError`) reduced modulo 2^64, so a negative
  corpus seed wraps exactly as in the reference.
- **Value model.** `true`/`false`; `Integer` (also for `u8`, `u64`, `i128`); bytes
  are `String`s with `Encoding::BINARY` (`Values.same` and `Canon` tell bytes from
  text by encoding); lists and tuples are `Array`. `Gens.i128` is two words, high
  first, read as two's complement.
- **Property convention.** A property is a block. It **holds iff it returns a
  truthy value**; `false` and `nil` falsify. This is deliberately fail-closed: a
  block that forgets to return a value falsifies instead of passing vacuously.
- **Shrinkers.** A shrinker is any callable `(value, falsifies, max_steps) ->
  [value, steps]`; `falsifies` returns truthy while the candidate still falsifies.
  Public `shrink_int`, `shrink_list`, `shrink_bytes` and `simulate_replay` take the
  property as a block (truthy = holds) and return `[minimal, steps]`
  (`simulate_replay` returns `[initial_fail, minimal]`).
- **Recoverable failure.** The runner, `Canon` and the corpus evaluator rescue
  `Exception` and re-raise only a fixed set that must never be swallowed:
  `NoMemoryError`, `SignalException` (so `Interrupt`), `SystemExit` and
  `Timeout::ExitException`. Everything else, including `StandardError`,
  `SystemStackError`, `NotImplementedError` and `Minitest::Assertion` (not a
  `StandardError`), is a `property-panic` when raised by a property.
- **Rendering.** `Canon` is total: `nil`, booleans, integers (above 100,000 bits
  `<bigint N bits>`), text strings quoted with escapes (`\uXXXX` for C0, DEL, C1,
  U+2028/9), binary strings as `0x<hex>`, arrays and hashes recursively (nesting
  deeper than 16 renders as `...`, more than 10,000 nodes truncates), any other
  object through `inspect` with control characters escaped. An error or
  `SystemStackError` from a user `inspect`, `to_s` or `each` degrades the **whole
  witness** to `<unrenderable>`. Total output is capped at 1 MiB. The CI digest is
  FNV-1a of `iter:<i>;steps:<s>;shrunk:<canonical>`, 16 hex digits. A user
  `inspect` that itself returns a gigabyte string cannot be stopped before it
  returns; the result is then truncated.
- **JSON.** Ruby's `JSON.parse` keeps the last duplicate key, so the corpus loader
  owns a strict parser (`StrictJson`): nesting deeper than 64 containers (the 65th
  is refused before recursing), documents over 8 MiB, more than 200,000 nodes,
  duplicate keys, invalid UTF-8 (and a BOM), raw control characters in strings,
  lone surrogate escapes, leading zeros, `+1`, `NaN`, comments, trailing commas and
  trailing data are refused. Numbers stay literal text (`StrictJson::Number`).
- **Hardening carried from Java and C#.** Every public count and size is capped
  (`Limits::MAX_GEN_COUNT` = 1,000,000; `gen_bytes` is capped even when
  `max_len == 0`; `gen_bounded` is bounded by `count x ceil(bitlen/8) <= 2^20`),
  `max_retries` is capped at 10,000, `max_steps` is validated, CI-redacted verdicts
  carry no seed, witness or minimal.
- **Sabotage hooks.** `Pbt::Hooks` holds the bisection move, the unit move, a flag
  that skips the deletion phase and the RNG factory as module-level accessors;
  tests swap one, run the corpus and expect problems, and reset in `ensure`. Ruby
  has no package-private visibility, so these are a documented test seam.

### 3.1 Shrink budget: a tree of scopes

The design is VER05g §3.1 line for line; this section states the Ruby mapping.

**A tree of budgets, charged hierarchically.** A scope (`Pbt::ShrinkBudget`) is a
node with a `parent`, a `root`, two counters (**attempts**, one per property
evaluation, cap `16 * max_steps + 64`; **work**, elements copied, at least 1 per
charge, cap `Limits::MAX_SHRINK_WORK` = 64M) and its own caps. One `Mutex`, owned
by the outermost root and shared by the whole tree, guards every counter; user
code never runs while it is held (a Ruby `Mutex` is not re-entrant, so this is
also a correctness requirement). `charge(n)` (an attempt) and `charge_copy(n)`
(work only, so the pbt-v1 corpus is unchanged) walk from the node to the root,
check room on **every** node (`cost > max - used`), then increment **all**: all or
nothing, and the root records that a refusal happened. Callers charge **before**
building a candidate; once refused, every loop ends and a sequence shrinker
returns its input uncopied (the returned array may be the input object). Accepted
moves are applied to the working array in place. Byte shrinking charges
`2 * size` per attempt (the array conversion).

- The public entry points that run user code (`Runner.check`'s shrink,
  `Gen#shrink`, `Shrink.shrink_int`, `shrink_list`, `shrink_bytes`,
  `simulate_replay`) open a **child** node of the ambient scope with a fresh local
  cap and charge every property evaluation they make, including the initial check.
  Internal combinators (tuple, list, filter, bytes, via `Gen#shrink_nested`) **join**
  their enclosing node. Owning depth is capped at 64; deeper scopes join. Closed
  scopes resolve to the nearest open ancestor. There is no classification of a call
  as "inside a property" or "inside a shrinker": the chain always reaches the
  outermost root, so the total work of one top-level `Runner.check` is capped by
  that root whatever the nesting.
- `Runner.check` runs its **whole body, case loop included,** in one scope, with an
  attempt cap of `num_tests + 16 * max_steps + 64`. A check **nested** in a property
  pays 1 attempt at entry (refused: it returns `property-exhausted` without
  evaluating anything), 1 per case after the first, the sampled value's size as
  work (an `Array`/`String`/`Hash` counts its length plus, one level deeper, the
  length of each such element, visiting at most 10,000 elements; any other
  `Enumerable` is counted by enumerating at most 10,000; a hostile one counts 0),
  and 1 attempt per `filter` sampler retry. A **top-level** check always evaluates
  its first case; its filter retries are charged as **work** (`charge_copy(1)`)
  against the 64M allowance, not as attempts. If any charge in its tree was
  refused, a top-level check that would pass returns `property-exhausted`; a
  falsified verdict is never overridden. The single re-verification charges the
  check scope; if refused, the original witness is reported with 0 steps. `filter`
  charges before its user predicate runs; `max_steps` is validated and
  `max_steps == 0` returns the input uncopied.
- **Ambient carrier.** The current node lives in **Fiber storage**
  (`Fiber[:coding_adventures_pbt_budget]`, Ruby >= 3.2). `Thread.new` and
  `Fiber.new` start with a copy of the creating fiber's storage, so a thread or
  fiber created inside a scope holds the same node and charges the same tree, also
  after the spawning inner scope has closed. A node's parent is the nearest
  **open** ancestor; a flow whose whole chain has closed gets a **fresh root**.
  `close` restores the previous ambient value first, in an `ensure`.
  `Thread#[]` is *not* used: it is fiber-local and not inherited by new threads.
  **Thread escape (pinned by a test).** A thread that already existed when the
  scope opened (a pooled worker) holds an empty storage, so a shrinker running
  there opens a fresh root; so does a fiber created with `storage: nil`. The bound
  is therefore **per storage flow**; each such flow is still bounded by its own
  root. Shrinkers should run on the calling thread or on threads created inside
  the scope. Ractors are not supported.
- **`shrink_nested` is internal.** Ruby cannot restrict a method to a gem, so it is
  public but `:nodoc:`; as defence in depth it charges every evaluation of
  `falsifies` to the budget it joined (or a fresh root).
- Stacked filters, the generation work estimate and the reasons for a tree rather
  than join/isolate classification are as in VER05g §3.1.

## 4. Divergences from Java, C# and Python

| # | Topic | Ruby | Others | Reason |
|---|---|---|---|---|
| 1 | property result | truthy holds | Java/C#: `true`; Python: only `False` fails | fail-closed idiom |
| 2 | integers | one `Integer` | `BigInteger`/`ulong`/`byte` | Ruby has one integer type |
| 3 | bytes | binary `String` | `byte[]` | no byte array type |
| 4 | tuples | `Gens.tuple(*gens)` yields an `Array`, arity <= 64 | `Pair`/`Triple` | dynamic typing |
| 5 | ambient carrier | Fiber storage | `AsyncLocal` / `InheritableThreadLocal` | Ruby >= 3.2 |
| 6 | `shrink_nested` | public, `:nodoc:` | internal / package-private | no gem-level visibility |
| 7 | stack overflow | `SystemStackError` rescued | .NET cannot catch it | Ruby can; depth caps still apply to JSON, TFV and `Canon` |
| 8 | out of memory | `NoMemoryError` never rescued | same | VM cannot continue |
| 9 | list generator | `Gens.list` | `Gens.ListOf` (C#) | no clash in Ruby |

**Trust boundary.** The public `ShrinkBudget#add`, `#mark_refused`, `.default_max_work=`
and the `Hooks` accessors are an in-process trust boundary that Ruby cannot make
private: code in the same process can already redefine anything, so they are
documented test seams, not an access control.

## 5. Build and inputs

`BUILD` and `BUILD_windows` run `bundle exec rake test` (simplecov enforces 95%
line coverage from `test/test_helper.rb`) and, on POSIX, `standardrb --no-fix lib/ test/`.
The package reads `code/specs/fixtures/pbt-v1/` in its tests, so `ruby/test_bootstrap`
is registered as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`) beside python, go, java and kotlin; the existing selection
tests that enumerate every consumer fail closed until Ruby is added to them.
`required_capabilities.json` is empty.

## 6. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems.
- Negative controls: for **every** case and every corruption its op defines
  (flipped output, wrong minimal, steps + 1, wrong failure id, flipped validity, a
  valid TFV text in place of a reject text), the run fails **and every problem
  starts with that case's id and no other**; dropped and truncated corpora fail; an
  unknown op fails; the engine hooks make the corpus run fail.
- Pinned tests for what the corpus cannot catch: a seed whose first draw lies below
  the rejection threshold (the finalizer is invertible; output 0 needs state 0, i.e.
  seed `(0 - 0x9e3779b97f4a7c15) mod 2^64`), the `2^63 + 1` threshold boundary, and
  bisection accepting the left half and then the right half with exact step counts.
- Budget-tree tests, the required set of VER05e's `ShrinkBudgetTest`: nesting depth
  1 to 3 plus the branching case, a recursing custom shrinker, a filter predicate
  that shrinks, a filter sampler rejecting everything, threads spawned in a scope,
  concurrent charging, a refused take leaving every counter unchanged, a huge cost,
  ambient restore after a raise, depth-cap joining, re-entrancy equality, starvation
  never passing, refused re-verification.
- A regression test per hardening item of §3.
- At least 95% line coverage, enforced by the test helper.
