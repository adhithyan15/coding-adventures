# VER05g — C# Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05d](VER05d-go-pbt-engine.md), [VER05e](VER05e-java-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-CS-PBT.

This specification defines the C# implementation of the Level 2 Property-Based
Testing (PBT) engine. C# had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1.

It is a line-for-line port of the behaviour of [VER05e](VER05e-java-pbt-engine.md)
(Java), which has already been through three security-review rounds, including
every hardening item that review produced. This document therefore does not
repeat the algorithms; it states what is the same (§1), what C# changes (§2–§4)
and the tests that pin it (§5).

---

## 1. Same as Java (normative by reference)

The following VER05e sections apply to the C# engine unchanged, with C# names
substituted for Java ones:

- §2 PRNG: SplitMix64, `bounded`, `boundedBig`, `fnv1a64`, `subSeed`
  (`SplitMix64(seed ^ fnv1a64(utf8(id))).NextU64()`); `bounded(1)` consumes one word.
- §3 generator algebra and draw order (the draw order is the cross-lane contract).
- §4 integrated shrinking and the **global attempt budget**
  `16 * maxSteps + 64`, which counts rejected attempts and is shared by every
  pass and every tuple component (no fresh per-pass budget).
- §5 runner, closed failure vocabulary, verdict honesty, re-verification of the
  witness, CI redaction (`CiMode.Unset` is treated as CI).
- §6 conformance runner, strict decoding, CWE-400 caps, coverage gates, negative controls.

## 2. Package and namespaces

`code/packages/csharp/test-bootstrap` (.NET 9, `CodingAdventures.TestBootstrap.CSharp`).
One project, four namespaces:

| Namespace (`CodingAdventures.TestBootstrap.…`) | Role |
|---|---|
| `Prng` | `SplitMix64` (`ulong` state), `Bounded`, `BoundedBig`, `Fnv1a64`, `SubSeed` |
| `Pbt` | `Gen<T>`, `Gens`, `Shrink`, `Runner`, `Verdict`, `Options`, `CiMode`, `FailureIds`, `Limits`, `Corpus` |
| `CorpusJson` | `StrictJson`, `Values`, `Canon`, `Clip` |
| `Tfv` | `TfvValidator` (the class cannot be called `Tfv`: a type may not share its namespace's name) |

BCL only (`System.Numerics.BigInteger`). No third-party runtime dependency; xunit and
coverlet are test-time only. The library performs no file, clock, network or
environment access, so `required_capabilities.json` is capability-free. Tests read
the committed fixture by walking up to the repository root, never a copy.

## 3. C# specifics

- **Unsigned arithmetic is native.** SplitMix64 and FNV-1a use `ulong` (wrapping,
  `unchecked`). The rejection threshold is `(0UL - n) % n` computed unsigned
  (`2^64 - n` modulo 2^64); a signed or `Python (-n) % n` form would be 0 and never
  reject. `Bounded(0)` throws `ArgumentOutOfRangeException`.
- **Integer widths.** `U8()` yields `byte`, `U64()` yields `ulong` (C# has the
  unsigned types Java lacks, so VER05e §9 row 5 does not apply), `Int64(min,max)`
  yields `long`, `I128()` and `Integer(min,max)` yield `BigInteger`. All shrink
  through the same `BigInteger` walk. `BigInteger >> 1` floors on negatives, as
  required.
- **Conventions.** Properties are `Func<T,bool>` (`true` = holds). Invalid
  arguments throw `ArgumentException` subclasses; a start value that does not
  falsify throws `NotFalsifyingException`. Tuples are `Pair<A,B>` and `Triple<A,B,C>`
  (records that also implement `ITuple` so `Canon` renders them as `{a,b}`).
- **Recoverable failure.** The runner, `Canon` and the corpus evaluator catch
  `Exception` **except** `OutOfMemoryException`. `StackOverflowException` cannot be
  caught in .NET, so depth is capped explicitly: `Canon` renders nesting deeper than
  16 as `...`, `StrictJson`, `Values` and the TFV validator cap at 64, and
  `Gen.Sample` / `Gen.Shrink` call `RuntimeHelpers.EnsureSufficientExecutionStack()`
  so runaway generator nesting becomes a catchable `InsufficientExecutionStackException`
  (reported as `property-exhausted`).
- **Rendering.** `Canon` quotes strings with `\"`, `\\`, `\n`, `\r`, `\t` and
  `\u00xx`, renders numbers with the invariant culture, hashes
  `iter:<i>;steps:<s>;shrunk:<canonical>` with FNV-1a for `repro-fnv`, and degrades
  an unrenderable witness (throwing `ToString`, enumerator, dictionary) to the
  constant `<unrenderable>`. A `BigInteger` above 100,000 bits renders as
  `<bigint N bits>` (decimal conversion of a huge value is itself a DoS).
- **JSON.** Hand-written strict parser (`StrictJson`): depth 64, 8 MiB, 200,000
  nodes, duplicate keys refused, strict UTF-8 (`UTF8Encoding(throwOnInvalid: true)`,
  no BOM tolerance), no leading zeros, no trailing data, numbers kept as literal text.
- **Hardening carried from Java review:** every public count/size entry point is
  capped (`Limits.MaxGenCount` = 1,000,000), `GenBytes` is capped even when
  `maxLen == 0`, `GenBounded` is bounded by `count × byte width ≤ 2^20`, sequence
  and integer shrinking run under one global budget, the deletion phase copies each
  candidate once, CI-redacted verdicts carry no seed/witness/minimal.
- **Sabotage hooks.** `internal static class Hooks` (bisection, unit step, deletion
  phase, RNG factory) is visible to the test assembly through `InternalsVisibleTo`;
  tests reset it in a `finally`/`Dispose`. Test parallelism is disabled for the
  assembly because the hooks are process-global.

### 3.1 Shrink budget: a tree of scopes (security review rounds 1 to 3)

Counting attempts is not enough: each attempt copies its candidate, so a 1 MiB `Bytes` witness
with a stalling property would cost ~16,000 x 1M element copies. The design is the one of
[VER05e section 4.1](VER05e-java-pbt-engine.md), ported to C#.

**A tree of budgets, charged hierarchically.** A scope (`ShrinkBudget`) is a node with a `parent`,
a `root`, two counters (**attempts**, one per property evaluation, cap `16 * maxSteps + 64`;
**work**, list elements copied, at least 1 per charge, cap `Limits.MaxShrinkWork` = 64M) and its
own caps. One lock, owned by the outermost root, guards every counter in the tree; user code
never runs while it is held. `Charge(n)` (an attempt) and `ChargeCopy(n)` (work only, so the
pbt-v1 corpus is unchanged) walk from the node to the root, check room on **every** node
(`cost > max - used`, which cannot wrap), then increment **all**: all or nothing, and the root
records that a refusal happened. Callers charge **before** building a candidate; once refused,
every loop ends and a sequence shrinker returns its input uncopied (the returned list or array
may alias the input). Accepted moves are applied in place. Byte shrinking charges `2 * size` per
attempt (the array conversion).

- The public entry points that run user code (`Runner.Check`'s shrink, `Gen.Shrink`,
  `Shrink.ShrinkInt`, `ShrinkList`, `ShrinkBytes`, `SimulateReplay`) open a **child** node of the
  ambient scope with a fresh local cap and charge every property evaluation they make. Internal
  combinators (tuple, list, filter, bytes, via the **internal** `Gen.ShrinkNested`) **join** their enclosing node.
  Owning depth is capped at 64; deeper scopes join. There is no classification of a call as
  "inside a property" or "inside a shrinker": the chain always reaches the outermost root, so
  the total work of one top-level `Runner.Check` is capped by that root whatever the nesting.
- `Runner.Check` runs its **whole body, case loop included,** in one scope (`EnterCheck`): a root
  when there is no ambient budget, a child otherwise, with an attempt cap of
  `numTests + 16 * maxSteps + 64`. A check **nested** in a property charges one attempt at entry
  (refused: `property-exhausted`, nothing evaluated), one per case after the first, the sampled
  value's size as work (a collection or array counts its length plus, one level deeper, the length
  of each element that is a collection, array or string, visiting at most 10,000 elements; a
  hostile collection counts 0), and one per `Filter` sampler retry. A top-level check always
  evaluates its first case. If any charge in its tree was refused, a top-level check that would
  pass returns `property-exhausted` instead; a falsified verdict is never overridden. The single
  re-verification charges the check scope; if refused, the original witness is reported with 0
  steps. `Filter` charges before its user predicate runs; `Limits.MaxRetries` = 10,000;
  `maxSteps` is validated and `maxSteps == 0` returns the input uncopied.
- **Ambient carrier.** The current node lives in an `AsyncLocal<ShrinkBudget?>`: it flows into
  new threads, `Task.Run` and async continuations, which then charge the same tree. A node's
  parent is the nearest **open** ancestor; a flow whose whole chain has closed gets a fresh root.
  **Pool-thread escape (pinned by a test).** A thread that does not flow the execution context
  (a long-lived consumer thread started before the scope, `ExecutionContext.SuppressFlow`,
  `ThreadPool.UnsafeQueueUserWorkItem`) has no ambient scope, so a shrinker running there opens a
  fresh root. `Task.Run` flows the context even onto an existing pool thread, so it is covered.
  Shrinkers should run on the calling thread, or on flows created inside the scope.
  **The bound is therefore per execution-context flow**: `SuppressFlow`, `UnsafeQueueUserWorkItem`,
  pre-existing consumer threads and a clean `ExecutionContext.Run` each get a fresh root, so code
  that deliberately detaches from the context escapes the outer budget (each such flow is still
  bounded by its own root).
- **`ShrinkNested` is internal** (it is package-private in Java). It joins the ambient scope, or
  opens a fresh root, and as defence in depth charges every evaluation of `falsifies` to that
  budget: leaf shrinkers (`IntWalk`, byte and bool walks) charge nothing themselves, so a public
  recursive entry would otherwise run unbounded uncharged evaluations.
- **Stacked filters.** Every `Filter` retry made inside a check is charged: an attempt up the chain
  in a nested check, one element of work against the check root's 64M allowance in a top-level
  check. So `Filter(Filter(Filter(g, rare), rare), rare)`, whose retries multiply (10,000 per level),
  is cut off after 64M predicate calls with `property-exhausted` instead of running ~10^12. Charging
  work (not attempts) at top level keeps ordinary filters unaffected: pbt-v1 and the existing
  top-level filter tests are unchanged, whereas charging attempts would have made a filter accepting
  1 in 1,000 exhaust a default 100-case check.
- **Generation work estimate.** Besides strings, arrays and non-generic `ICollection`, any other
  `IEnumerable` (`HashSet<T>`, `ImmutableArray<T>`, lazy sequences) is counted by enumerating up to
  10,000 elements, and an `ITuple` counts its items.
- **Why a tree and not classification.** The first C# design decided per call whether to join or
  isolate the ambient budget. On the Java port that scheme had a new hole in each of four review
  rounds (a custom shrinker calling the public shrink reopened amplification; a depth counter
  shared across threads; filter predicates classified wrong; an isolated scope did not reset the
  counter). Hierarchical charging makes the bound independent of classification: isolation only
  means a fresh *local* cap, never an unbounded one.

Also: `Canon` guards user `ToString()` with `TryEnsureSufficientExecutionStack`, which only covers
`Canon`'s own recursion: a self-recursive `ToString` still overflows the process, so witness types
MUST have a bounded, shallow `ToString`. It caps total output at 1 MiB and escapes C0/C1 controls
and U+2028/U+2029 (as does the TFV diagnostic quoter). Candidate lists are fresh `List` objects but
their elements are not cloned. Nested generators are length-bounded per level, not in total.
The pbt-v1 results are unchanged: 25/25 with exact minimals and step counts.

## 4. Divergences from Java and Go

| # | Topic | C# | Java / Go | Reason |
|---|---|---|---|---|
| 1 | `u64`/`u8` | `ulong`/`byte` | Java `BigInteger`/`Integer` | C# has unsigned types |
| 2 | stack overflow | cannot be caught; explicit depth caps and `EnsureSufficientExecutionStack` | Java catches `StackOverflowError` | .NET terminates the process |
| 3 | OOM | `OutOfMemoryException` never caught | same idea (`OutOfMemoryError`) | VM cannot continue safely |
| 4 | tuples | `Pair`, `Triple` implement `ITuple` | Java records via reflection | cheaper, no reflection |
| 5 | huge `BigInteger` render | `<bigint N bits>` above 100,000 bits | not capped in Java | .NET decimal conversion is super-linear |
| 6 | TFV class name | `TfvValidator` | `Tfv` | namespace/type clash |
| 7 | seed type | `ulong`; corpus seeds reduced mod 2^64 with `& ulong.MaxValue` | `long` | native unsigned |
| 8 | `Verdict.Seed` | `ulong?` | `Long` | as above |
| 9 | JSON depth | at most 64 nested containers (the 65th is refused before recursing) | Java checks `depth > 64` on values and so admits 65 | an exact, testable bound |
| 10 | list generator name | `Gens.ListOf` | `Gens.list` | `List` would collide with `System.Collections.Generic.List<T>` inside the class |

## 5. Build and inputs

`BUILD` and `BUILD_windows` run `dotnet test … /p:CollectCoverage=true
/p:Threshold=95 /p:ThresholdType=line`. The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `csharp/test-bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`), alongside python, go, java and kotlin.

## 6. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems.
- Negative controls: for every case and every corruption its op defines, the run
  fails and every problem starts with that case's id and no other; dropped and
  truncated corpora fail; the engine hooks make the corpus run fail.
- Pinned tests for what the corpus cannot catch: a seed whose first draw lies below
  the rejection threshold (the SplitMix64 finalizer is invertible; output 0 needs
  state 0, i.e. seed `0 - 0x9e3779b97f4a7c15`), the `2^63+1` threshold boundary, and
  bisection accepting the left half then the right half with exact step counts.
- A regression test per hardening item of §3.
- At least 95% line coverage, enforced by the BUILD.
