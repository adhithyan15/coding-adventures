# VER05e — Java Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05a](VER05a-typescript-pbt-engine.md), [VER05b](VER05b-dart-pbt-engine.md), [VER05c](VER05c-python-pbt-engine.md), [VER05d](VER05d-go-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-JAVA-PBT.

This specification defines the Java implementation of the Level 2 Property-Based
Testing (PBT) engine. Java had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1. It is a port of the structure of [VER05d](VER05d-go-pbt-engine.md)
(Go) and [VER05c](VER05c-python-pbt-engine.md) (Python); every place where Java
forces a different shape is listed in §9.

---

## 1. Architecture and Placement

The package is `code/packages/java/test-bootstrap` (Gradle, Java 21, group
`com.codingadventures`, artifact `test-bootstrap`). One engine per lane, living
where the lane's PRNG lives; the fuzz engine (VER06f) will be added later as a
further Java package importing `prng`. Nothing in `prng` knows about property
testing.

```text
 tests / consumers                         conformance
 ───────────────────                       ───────────
 Runner.check(seed, gen, prop, options)    Corpus.evaluate(cases)
        │                                          │ (strict JSON parse, then
        ▼                                          │  the SAME engine functions)
   Gen<T> (sample, shrink)  ◄──────────────────────┘
     │ Gens.map / filter / flatMap / oneOf / frequency / list / tuple2 / tuple3
     ▼
   integrated shrinkers: Shrink.shrinkInt, shrinkList, shrinkBytes
     │
     ▼
   prng: SplitMix64, fnv1a64, subSeed         tfv: strict TFV v1 validator
```

| Java package (`com.codingadventures.testbootstrap.…`) | Role |
|---|---|
| `prng` | `SplitMix64`, `bounded`, `boundedBig`, `fnv1a64`, `subSeed` |
| `pbt` | `Gen`, `Gens`, `Shrink`, `Runner`, `Verdict`, `Options`, `CiMode`, `FailureIds`, `Limits`, `Corpus` |
| `corpusjson` | strict JSON reader (`StrictJson`), strict value decoding and kind-sensitive equality (`Values`), `Canon` witness rendering, `Clip` |
| `tfv` | strict validator for TFV v1 text, so `corpus_reject` cases are checked by a real parser, not skipped |

JDK only (`java.math.BigInteger`, `java.util`). No third-party runtime
dependency; JUnit 5 and JaCoCo are test-time only. The library performs no file,
clock, network or environment access, so `required_capabilities.json` is
capability-free. Reading `cases.json` is the job of the test, which walks up from
its working directory to the repository root and reads the committed fixture,
never a copy.

---

## 2. Deterministic PRNG (`prng`)

`SplitMix64` is the TF01 §5.1 / VER05 §2.1 state machine on a Java `long`. Java
`long` arithmetic wraps modulo 2^64, so no masking is needed; the right shift is
the unsigned `>>>`. Seed 0's first output is `0xe220a8397b1dcdaf`, pinned by a
test. The corpus runner reduces its (possibly wider or negative) decimal seeds
modulo 2^64 with `BigInteger.longValue()`, which keeps the low 64 bits of the
two's-complement form, exactly the reference's mask.

### 2.1 Bounded draws

`bounded(long n)` treats `n` as an **unsigned** 64-bit value (`1 <= n <= 2^64-1`)
and is uniform on `[0, n)`. The rejection threshold is `t = (2^64 - n) mod n`,
computed unsigned: `Long.remainderUnsigned(-n, n)` (`-n` is `2^64 - n` modulo
2^64). Raw words are compared with `Long.compareUnsigned` and reduced with
`Long.remainderUnsigned`; a signed `%` or `<` would silently misbehave once the
top bit is set. `n == 0` throws `IllegalArgumentException`.

**`bounded(1)` consumes one stream word** (the VER05 §2.2 loop run once), as the
Rust lane, the Go lane and the reference do.

`boundedBig(BigInteger span)` is uniform on `[0, span)` for any `span >= 1`,
identical to VER05d §2.1: `bounded(span)` below 2^64, one raw word at exactly
2^64, k-word rejection above (`k = max(2, ceil(bitlen(span-1)/64))`, first draw
most significant, `t = (2^(64k) - span) mod span`). The Dart lane's unrejected
`raw % span` is **not** followed (VER05 §7.2); a test checks the three thirds of
a `3 * 2^126` span.

### 2.2 FNV-1a and sub-seeds

`fnv1a64` is the 64-bit FNV-1a hash (offset `0xcbf29ce484222325`, prime
`0x100000001b3`). `subSeed(runSeed, id)` is

```text
sub_seed(run_seed, id) = SplitMix64( run_seed XOR fnv1a64(utf8(id)) ).nextU64()
```

per VER05 §2.3, as the Rust, Python and Go lanes do (TypeScript and Dart omit the
final scramble). The corpus has no `sub_seed` case, so vectors computed with the
independent reference are pinned in the Java tests.

---

## 3. Generator Algebra (`Gen<T>`, `Gens`)

`Gen<T>` wraps `sample(SplitMix64)` and an optional integrated shrinker (§4).
Sampling throws the unchecked `ExhaustedException` when generation fails on data
(a filter that never accepts); composition propagates it. Draw order is the
compatibility contract; `pbt-v1` pins it.

### 3.1 Integer representation

Java has no 128-bit integer. **All integers that the corpus or the full-range
generators can produce are `BigInteger`** (the Go lane's `*big.Int`). `i128()`
yields a `BigInteger` in `[-2^127, 2^127-1]`; `integer(min, max)` is arbitrary
precision. `BigInteger` is immutable, so the Go aliasing hazard does not exist.
Convenience generators for native widths draw exactly as their big counterparts
and shrink through the same walk: `u8()` yields `Integer` in `[0, 255]`,
`int64(min, max)` yields `Long`. `u64()` yields a `BigInteger` in `[0, 2^64-1]`,
because Java's `long` is signed and a "Long holding an unsigned value" would
print and compare wrongly in user code (§9 row 5).

### 3.2 Table

| Java | VER05 | Sampling rule |
|---|---|---|
| `bool()` | `gen_bool` | `bounded(2) == 1`; shrinks `true` to `false` |
| `u8()` | `gen_u8` | `bounded(256)` |
| `u64()` | `gen_u64` | `nextU64()` |
| `i128()` | `gen_i128` | words `w1`, `w2`; `(w1 << 64) \| w2`, signed past 2^127 |
| `integer(min, max)` | `gen_i128(min, max)` | `min + boundedBig(max-min+1)` |
| `int64(min, max)` | — | same draws as `integer` |
| `bytes(minLen, maxLen)` | `gen_bytes` | `L = minLen + bounded(span)`, then `L` draws of `bounded(256)` |
| `constant(v)` | — | draws nothing |
| `map(g, f)` | `map` | no extra draws; **drops shrinking** |
| `filter(g, pred)` / `filter(g, pred, n)` | `filter` | up to `n` (default 100) samples then `ExhaustedException`; shrinking preserved, candidates failing `pred` rejected |
| `flatMap(g, f)` | — | sample, then sample from `f(value)`; no shrinking |
| `oneOf(gens)` | `one_of` | `bounded(len)`, then sample that arm |
| `frequency(arms)` | `frequency` | `bounded(total)`; arms in order; zero-weight arms never selected; total `1..2^32` |
| `list(elem, minLen, maxLen)` | `list` | length via `bounded(span)`, then elements in order |
| `tuple2`, `tuple3` | — | sampled left to right; shrink coordinate-wise (`Pair`, `Triple` records) |

Corpus entry points: `Gens.genPrimitive(seed, type, count)`, `genBounded`,
`genBytes`, `Shrink.simulateReplay(seed, prop)`.

Invalid constructor arguments (`min > max`, `minLen > maxLen`, a length above
`MAX_INPUT_BYTES`/`MAX_LIST_LEN`, an empty `oneOf`, a zero-total `frequency`)
throw `IllegalArgumentException`.

---

## 4. Integrated Shrinking

All shrinkers take a predicate `prop` returning `true` when the property
**holds** (the corpus convention) and return the minimal value with the step
count. `steps` counts **accepted moves only**; the target probe is one move. A
start value that does not falsify throws `NotFalsifyingException` (a caller bug).
`0 <= maxSteps <= MAX_SHRINK_STEPS`; `maxSteps == 0` returns the start with zero
steps.

The algorithms are those of VER05d §4 line for line:

1. `shrinkInt(initial, target, min, max, prop, maxSteps)`: target probe (one
   step), then the walk — bisection candidate `current - floor(diff/2)` when
   `|diff| > 1`, else (or if refused) the unit step `current - sign(diff)`;
   candidates stay in `[min, max]`.
2. Sequence shrinking (`shrinkList`, `shrinkBytes`, `Gens.list`, `Gens.bytes`):
   empty/prefix probe; bisection (left, then right half) while
   `len > max(1, minLen)`; single deletion right to left; element-wise walk left
   to right **without** the target probe. `steps < maxSteps` guards every loop
   head. Candidates passed to `prop` are fresh copies.
3. Generator-attached shrinking: `integer(min,max)` toward the in-range point
   nearest 0; `bytes`/`list` respect `minLen`; `bool` toward `false`; tuples one
   coordinate at a time in repeated passes until a pass makes no progress.
   `map`, `flatMap`, `oneOf`, `frequency` have no shrinker.

**Attempt budget (security review, CWE-400).** A rejected attempt copies the
whole list but is not an accepted step, so `maxSteps` alone does not bound the
work: a 20,000-element list whose every smaller candidate is rejected would cost
O(n^2). Sequence shrinking therefore also charges every evaluation of the
property (the prefix probe, both halves, every single deletion and every
element-walk candidate, accepted or rejected) against one budget of
`ATTEMPT_FACTOR * maxSteps + ATTEMPT_SLACK = 16 * maxSteps + 64`. When it is
spent nothing further is accepted and every loop ends, so the result is still a
value that falsifies. The pbt-v1 corpus (maxSteps 1000, at most 8 elements) never
approaches it; its exact minimals and step counts are unchanged.

The budget is also **global per shrink**, not only per sequence: `tuple2`/`tuple3`
wrap the property once with a budget of the same size shared by every pass of the
repeat-until-no-progress loop and every component (otherwise each pass would give
each component a fresh budget, about `8 * maxSteps^2` evaluations), and
`Runner.check` wraps the property once per falsified case so nested and composed
shrinkers together stay inside one `16 * maxSteps + 64`. The final re-verification
of the reported witness uses the unbudgeted property. A rejected deletion copies
the list once (the fresh candidate goes straight to the property); an accepted
move is rebuilt from the unmutated current list.

`floor(diff/2)` on a negative `diff` is floored division (`BigInteger.shiftRight(1)`
floors), matching Python's `//` and Go's Euclidean division by 2.

### 4.1 Shrink budget (ported from the VER05g security review, CWE-400)

The attempt budget above counts property evaluations but not the work each one
costs: a 1 MiB `bytes` witness whose property stalls bisection would cost about
16,000 attempts x 1M elements of copying, tens of GB. Nested shrinkers (list of
lists, tuple components), initial copies and per-pass conversions made it worse.

**Design: a tree of budgets, charged hierarchically.** A scope is a node with a
`parent`, a `root`, two counters (**attempts**, one per property evaluation, cap
`16 * maxSteps + 64`; **work**, list elements copied, at least 1 per charge, cap
`Limits.MAX_SHRINK_WORK` = 64M) and its own caps. One lock, owned by the
outermost root, guards every counter of the tree (no per-node locks); user code
never runs while it is held.

- `charge(elements)` (an attempt) and `chargeCopy(elements)` (work only, so
  attempt counts and the pbt-v1 corpus are unchanged) walk from the node to the
  root and check room on **every** node, then increment **all**: all or nothing,
  so a refusal leaves every counter unchanged. The room test is written
  `cost > max - used`, which cannot wrap (`Long.MAX_VALUE` is refused). Callers
  charge **before** building a candidate; once refused, every loop ends and a
  sequence shrinker returns its input uncopied (**the returned list or array may
  alias the input**). Accepted moves mutate `current` in place (no second copy).
  Byte shrinking charges `2 * size` per attempt (the array conversion).
- The public entry points that run user code (`Runner.check`'s shrink,
  `Gen.shrink`, `Shrink.shrinkInt`, `shrinkList`, `shrinkBytes`,
  `simulateReplay`) always open a **child** node of the ambient scope with a fresh
  local cap, and charge every property evaluation they make (including the
  initial check). Internal combinators (`tuple2`/`tuple3`, `list`, `filter`,
  `bytes`, via the package-private `Gen.shrinkNested`) **join** their enclosing
  node. There is no classification of a call as "inside a property" or "inside a
  shrinker": the chain always reaches the outermost root, so the total work of one
  top-level `Runner.check` is capped by that root **whatever the nesting**: a
  property calling `Runner.check`, a custom `Shrinker` recursing into `Gen.shrink`,
  a `filter` predicate doing so, or `shrinkInt`/`simulateReplay` in a property.
  Owning nesting is capped at `ShrinkBudget.MAX_DEPTH` = 64; deeper scopes join, so
  the walk stays short.
- `Runner.check` runs its **whole body, case loop included,** in one scope: a root
  when there is no enclosing budget, a child otherwise, with an attempt cap of
  `numTests + (16 * maxSteps + 64)`. A check **nested** in a property (an ambient
  scope exists at entry) charges one attempt at entry (refused: it returns
  `property-exhausted` without evaluating anything), one per case after the first,
  and the size of every sampled collection or array as work; so neither nesting
  depth, branching (m checks per evaluation), `numTests`, nor large generated
  values multiply the outer budget, including during the generation phase, before
  any shrink. A starved nested check is exhausted, never a pass; a **top-level**
  check has no ambient scope and always evaluates its first case. Runner's former
  private attempt counter is folded into the node. The single re-verification
  charges one attempt to the check scope (not the possibly spent shrink scope); if
  the chain refuses it the shrunk value is not adopted and the original witness is
  reported with 0 steps. A re-entrant check is standalone-like while the outer
  budget has room; near exhaustion it may be starved. `Gen.shrink` and
  `shrinkNested` validate `maxSteps`; `maxSteps == 0` returns the input uncopied;
  `filter` charges before its user predicate runs.
- **Filter sampling.** Every retry of a `filter` sampler is charged whenever a
  budget exists, so stacked filters (`filter(filter(filter(g, rare), rare), rare)`)
  cannot multiply uncharged retries. Inside a nested check the retry is one
  **attempt** up the chain; at a top-level check root it is one element of **work**
  against the 64M allowance (`chargeCopy(1)`), because the top-level attempt cap
  (`numTests + 16 * maxSteps + 64`, about 16k) would wrongly exhaust a legitimate
  rare-but-satisfiable filter. When refused the sample throws `ExhaustedException`
  (property-exhausted; the root's refused flag also turns a would-be pass into
  exhausted). `maxRetries` is capped at `Limits.MAX_FILTER_RETRIES` = 10,000 (the C#
  value; the corpus `filter_exhausted` case is unchanged).
- **Starvation is never a pass.** The root records that a charge was refused. On
  the pass path a top-level check returns `property-exhausted` instead of `passed`
  if any charge in its tree (a nested check, a nested generator) was refused. A
  falsified verdict is never overridden.
- **Generation work is approximate.** A sampled value's work is its collection or
  array length plus, one level deeper, the length of each element that is itself a
  collection, array or string (visiting at most 10,000 elements); a string counts
  its length; deeper structure is not counted, and a hostile collection counts 0.
- Rendering a witness for the non-redacted message goes through the same
  total wrapper as the redacted path (`Canon.render` is already total, so this is
  defence in depth).
- The root work cap is fixed at root creation. `ShrinkBudget.defaultMaxWork` is a
  package-private test hook (restored by `Hooks.reset`), read only there.

**Ambient scope.** The current node lives in an `InheritableThreadLocal`, so a
thread created inside a scope starts with that node and charges the same tree
(also after the spawning inner scope has closed: charging a closed ancestor is
allowed). A node's parent is the nearest **open** ancestor; a thread whose whole
chain is closed (it outlived the outermost root) gets a **fresh root** and never
charges a dead one. `close()` restores the previous ambient value first, in a
`finally`, before recording stats.

**Known limits.** Lazily created `ForkJoinPool`/executor workers may inherit the
`AMBIENT` of whichever check happens to be running when they are created, and so
share (or starve) that budget. The package-private statics (`defaultMaxWork`, the
`Hooks` seams) are only closed by package privacy: seal the jar (or accept that
same-package code can reach them). **Pool-thread escape (pinned by a test).** A thread that already existed when the scope
opened (a pooled executor thread, a `ForkJoinPool` worker) has no ambient scope,
so a shrinker running there opens a fresh root. Shrinkers should run on the
calling thread, or on threads created inside the scope.

**Why a tree and not classification.** Four review rounds each found a hole in a
scheme that decided, per call, whether to *join* or to *isolate* the ambient
budget: (1) public `Gen.shrink` always isolated, so a custom shrinker calling it
per component reopened the nested amplification; (2) a property-depth counter on
the shared root made workers of one parallel shrinker isolate each other; (3) a
per-thread depth cell fixed that, but an isolated scope did not reset the cell, so
custom shrinkers under a property isolated on every call, and a filter-predicate
test passed without the guard it claimed to test; (4) every classification
needed a matching test and a new place to get it wrong. Hierarchical charging
makes the bound independent of classification: isolation only means a fresh
*local* cap, never an unbounded one.

The pbt-v1 results are unchanged: 25/25 with exact minimals and step counts.

---

## 5. Runner, Verdicts and Failure Vocabulary

`Runner.check(seed, gen, prop, options)` runs `options.numTests()` (default 100)
cases from `SplitMix64(seed)`. `prop` returns `true` when the property holds.
Invalid options (`numTests` outside `[0, MAX_CASES]`, `maxShrinkSteps` outside
`[0, MAX_SHRINK_STEPS]`, a null property) throw `IllegalArgumentException`; no
`Verdict` is produced, so there is no way to mistake the call for a pass.

- `numTests == 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- generator exception → `property-exhausted`.
- property throws (`RuntimeException`, `AssertionError`, `StackOverflowError`) →
  `property-panic`.
- property returns `false` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is re-verified (VER05 §7.1.2): if the shrinker throws,
returns a value that no longer falsifies, one on which the property throws, or
an out-of-range step count, the **original** candidate is reported with 0 shrink
steps. `Runner.check(..., Shrinker override)` takes an explicit shrinker so a
defective shrinker can be tested. While shrinking, a candidate on which the
property throws counts as *not falsifying*.

Closed failure ids: `property-counterexample`, `property-exhausted`,
`property-timeout`, `property-panic`; `FailureIds.isPbtFailureId(s)` is true only
for those four strings. `property-timeout` is registered but never produced.

### 5.1 CI redaction

`Options.ci()` is `CiMode.UNSET` (default), `ON` or `OFF`. **`UNSET` is
treated as CI**: the engine fails closed and redacts. The package does not read
the environment; `Options.ciFromLookup(Function<String,String>)` is a pure helper
(the caller passes `System::getenv`): `CI` set to anything except empty, `0` or
`false` (case-insensitive, trimmed) means CI. When redacting, the message and the
`Verdict` carry **no concrete value and no seed**: only iteration, shrink steps
and `repro-fnv`, the FNV-1a of `iter:<i>;steps:<s>;shrunk:<canonical value>`.
Panic text is replaced by the class name of the thrown value. Caveat (VER05 §7.4
#3): **`repro-fnv` is a correlation id, not a secret**; it is brute-forceable over
a small witness space.

`Canon` is total and bounded: nesting deeper than 16 renders as `...`, more than
10,000 nodes truncates, and a value whose `toString` throws renders as its class
name. A hostile iterator, `entrySet`, `CharSequence` or array accessor (or a
`StackOverflowError`, `AssertionError` or `LinkageError` inside one; never
`OutOfMemoryError` or other `VirtualMachineError`s) degrades the whole witness to the constant
`<unrenderable>`, and `Runner` also guards its own call, falling back to a
constant digest preimage, so `check` always returns a verdict for a deep or
hostile witness.

**What the digest does and does not do.** `repro-fnv` identifies a failure (same
iteration, steps and witness give the same digest) but does **not** hide a
low-entropy witness: FNV-1a over `iter;steps;render` is unkeyed, so an attacker
who knows the property can enumerate small witnesses and match it. **There is no
wall-clock timeout**: `property-timeout` is registered and never produced, so a
property that loops forever hangs the run (the case and shrink budgets bound only
the number of evaluations, not their duration).

---

## 6. Conformance and Negative Controls

`Corpus.evaluate(List<Object> cases)` evaluates every case with the engine's
public functions and returns a `Report` (evaluated count plus problem strings;
no problems means conform). `Corpus.parse(...)` is the strict JSON reader:
exactly `{schema_version: 1, cases: [...]}`, duplicate keys refused, malformed
tokens/trailing data refused, nesting depth capped at 64, 8 MiB cap. The JDK has
no JSON library, so `StrictJson` is a small hand-written parser.

- **Strict decoding:** exactly the fields of the op; ints canonical decimal
  strings (at most 1000 digits); bytes even-length lowercase hex; an unknown op,
  property name or malformed value is a *problem*, never a skip. A non-string id
  is a problem, not a crash. Every problem line for a case starts with its id.
- **CWE-400 caps** as VER05d §6: counts, retries, lengths and list/bytes sizes
  above 10,000 are reported problems; `count x max_len <= 2^20` for `gen_bytes`
  and `count x ceil(bitlen/8) <= 2^20` for `gen_bounded`; a `gen_*` case whose
  `outputs` length is not `count` is refused before generating.
- **Generation count:** `genPrimitive`, `genBounded` and `genBytes` reject a
  `count` above `Limits.MAX_GEN_COUNT` (1,000,000) in every case, including
  `maxLen == 0` where the byte-total budget would otherwise never apply.
  `genBounded` also enforces `count x ceil(bitlen(hi - lo) / 8) <= 2^20` itself
  (the corpus runner relies on it rather than repeating the check).
- **JSON size:** besides depth (64) and bytes (8 MiB), `StrictJson` caps the total
  number of values in a document at `MAX_NODES = 200,000` (8 MiB of `0,` would
  otherwise be ~4 million boxed nodes, hundreds of MB of heap), with a short error.
- **Bounded reports:** fixture-derived fragments are clipped (200 chars per
  fragment, 400 per problem); `tfv` clips the token it quotes.
- **Recursion depth:** decode and comparison are capped at 64 levels even for
  hand-built trees.
- **Coverage gates:** every op and all four failure ids present, an unregistered
  id rejected, and case count equal to `PBTV1_CASE_COUNT` (25).
- `corpus_reject` texts must be rejected by `Tfv.validate`.
- `filter_exhausted` runs end to end through `Runner.check` and counts predicate
  calls exactly (`max_retries`).
- An evaluator that throws on a case is a problem for that case.

`CorpusConformanceTest` loads the committed fixture, asserts zero problems and
exactly 25 evaluated, and carries the negative controls. **Per-operation
corruption, not a single case:** for *every one* of the 25 real cases the test
deep-copies the corpus, applies the corruption its op defines (flipped output,
wrong minimal, steps+1, wrong failure id, flipped validity, a valid TFV in place
of a reject text), and asserts that the run fails **and that a problem names that
case's id**, and no other case's id (the lesson recorded as PT-F5: a single-case
negative control proves only that one path). Truncated and dropped-case lists and
an unknown op are problems. Engine sabotage through package-private hooks (a
changed unit step, a changed bisection, a skipped deletion phase, a shifted RNG)
must make the corpus run report problems.

Because the corpus draws spans small enough that a wrong rejection threshold
almost never fires, `SplitMix64Test` additionally forces the rejection branch:
SplitMix64's finalizer is invertible, so the test computes a state whose next
output is exactly 0, which a correct `bounded(3)` (threshold 1) must reject and a
mistaken threshold would accept. Wide-span vectors from the reference
(`2^63+1`) pin the large-threshold case.

Property-level tests assert that a falsifiable property is never a pass, a
zero-case run is not a pass, a lying or throwing shrinker cannot change the
verdict, and CI messages never contain the witness or seed. The independent
reference `code/scripts/pbt_conformance.py` must still report 0 problems.

---

## 7. Build and Inputs

`BUILD` and `BUILD_windows` run `gradle test check`. `check` is wired to
`jacocoTestCoverageVerification` with a LINE coverage minimum of 95%. The package
reads `code/specs/fixtures/pbt-v1/` in its tests, so `java/test-bootstrap` is
registered as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`), as `python/test-bootstrap` and `go/test-bootstrap`
are. `required_capabilities.json` is empty.

## 8. Reference

Pinned vectors and the corpus come from the independent reference
`code/scripts/pbt_conformance.py`; the Java lane never trusts its own output to
define expected values.

## 9. Divergences from VER05 / other lanes

| # | Topic | Java | Others | Reason |
|---|---|---|---|---|
| 1 | `sub_seed` | `SplitMix64(seed ^ fnv).nextU64()` | TS/Dart omit `nextU64()` | follow VER05 §2.3, Rust, Python, Go |
| 2 | `bounded(1)` | consumes one word | Python raw does not | VER05 §2.2 loop |
| 3 | wide spans | k-word rejection | Dart: `raw % span` | VER05 §7.2 |
| 4 | invalid arguments | `IllegalArgumentException` everywhere; `NotFalsifyingException` for a non-falsifying start; no error-plus-verdict pair | Go: panics for constructors, errors for runtime | Java idiom; no `Verdict` is ever produced for bad input |
| 5 | integers | `BigInteger`; `u64()` is `BigInteger`, `u8()` is `Integer`, `int64()` is `Long` | Go: `uint64`, `uint8` | no unsigned types; a `Long` holding an unsigned value mis-prints and mis-compares |
| 6 | tuples | `tuple2`, `tuple3` with `Pair`, `Triple` records | Python variadic | static typing |
| 7 | property convention | `Predicate<T>`: `true` means holds | Python: only `False` fails | as Go |
| 8 | CI default | redact unless `CiMode.OFF`; `ciFromLookup` pure helper | Rust reads `CI` | capability-free package, fail closed |
| 9 | `corpus_reject` | `Tfv.validate`, a validator only | Python has the full parser | no Java TF01 runner yet |
| 10 | recoverable failure | `RuntimeException`, `AssertionError`, `StackOverflowError` from a property are recovered; `OutOfMemoryError`, `ThreadDeath` and `System.exit` are not | Go: all panics | catching every `Throwable` would also swallow VM errors that cannot be safely continued from |
| 11 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |
| 12 | JSON | hand-written strict parser | Go `encoding/json`, Python `json` | JDK has no JSON library; zero third-party dependency rule |
| 13 | sabotage hooks | package-private static fields in `pbt` | Go: unexported package variables | same purpose; tests reset them in `finally` |
| 14 | string rendering | `Canon` quotes strings with Java escapes | Go `strconv.Quote` | no cross-lane digest vector is pinned for strings |
