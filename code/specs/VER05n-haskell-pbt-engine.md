# VER05n — Haskell Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md), [VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-HS-PBT.

This specification defines the Haskell implementation of the Level 2 Property-Based
Testing (PBT) engine. Haskell had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1 and the hierarchical shrink-budget tree of
[VER05e §4.1](VER05e-java-pbt-engine.md), which is part of the first version.

(Letters `j`, `k` and `l` belong to the C++, Swift and F# lanes and `m` to the
Elixir lane, all open on their own branches; `n` is the next free letter.)

Haskell is statically typed and lazy. The algorithms are those of the Ruby and C
lanes ([VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md)); this
document does not repeat them. It states what is the same (§1), what laziness,
asynchronous exceptions and the absence of an ambient thread-local change (§3 to
§6), and the tests that pin each hazard (§10).

---

## 1. Same as Java, C#, Ruby and C (normative by reference)

- VER05e §2: SplitMix64; `bounded(n)` with the rejection threshold
  `t = (2^64 - n) mod n`; `bounded_big`; FNV-1a 64; `bounded(1)` **consumes one
  word**; `sub_seed(run_seed, id) = SplitMix64(run_seed xor fnv1a64(utf8(id))).next_u64()`.
- VER05e §3: the generator algebra and its **draw order**, which is the
  cross-lane contract that `pbt-v1` pins.
- VER05e §4: integrated shrinking, in the order target probe, integer walk
  (bisection `current - floor(diff/2)` when `|diff| > 1`, else the unit step
  `current - sign(diff)`), and the sequence shrinker (empty or prefix probe,
  bisection left then right, single deletion right to left, element-wise walk
  left to right without a target probe). **`steps` counts accepted moves only**, the
  target probe is one move, and `steps < max_steps` guards every loop head.
- VER05e §4.1: the budget tree (restated for Haskell in §5).
- VER05 §7.1 and VER05e §5: the runner, closed failure vocabulary, verdict
  honesty, witness re-verification, CI redaction.
- VER05e §6: conformance runner, strict decoding, CWE-400 caps, negative controls.

## 2. Package and modules

`code/packages/haskell/test-bootstrap`, Cabal package `test-bootstrap`, root module
`CodingAdventures.TestBootstrap`, GHC 9.4.8 (the version CI installs), Haskell2010,
`-Wall -Werror`. **Only GHC boot packages**: `base`, `bytestring`, `containers`,
`text`, `deepseq`, `stm`; the test suites also use `directory` and `filepath` to find
the committed fixture. No Hackage dependency, no `hspec`, no FFI.

| Module | Role |
|---|---|
| `Prng` | `Rng`, `newRng`, `nextU64`, `bounded`, `boundedBig`, `fnv1a64`, `subSeed` |
| `Pbt.Types` | `Limits`, failure ids, the error type, `Options`, `Verdict`, `Config` (sabotage hooks) |
| `Pbt.Safe` | exception classification (§4), `tryRecoverable`, safe message rendering |
| `Pbt.Budget` | the budget tree, STM counters, scopes (§5) |
| `Pbt.Shrink` | integer walk, sequence shrinker, `shrinkInt`, `shrinkList`, `shrinkBytes`, `simulateReplay` |
| `Pbt.Gen` | `Gen`, the generator algebra (`int`, `bytes`, `list`, `tuple2`, `mapG`, `filterG`, `oneOf`, ...), generator-attached shrinking, the corpus entry points `genPrimitive`, `genBounded`, `genBytes` |
| `Pbt.Runner` | `check`, `checkM`, `PropM`, verdicts |
| `Pbt.Corpus` | the pbt-v1 conformance evaluator |
| `CorpusJson.Val` | the value model and `Canonical` class |
| `CorpusJson.Canon` | total, capped canonical rendering (§7) |
| `CorpusJson.StrictJson` | strict JSON reader (§8) |
| `CorpusJson.Values` | strict decoding of corpus values, kind-sensitive equality |
| `Tfv` | strict TFV v1 validator, so `corpus_reject` is checked by a real parser |

The library performs no file, clock, network or environment access, so
`required_capabilities.json` is capability-free. The CI flag is a parameter
(`ciFromLookup <$> lookupEnv "CI"` is the caller's call), never read inside. Tests
read the committed fixture by walking up to the repository root, never a copy.

## 3. Haskell specifics

- **Integers.** `Word64` wraps natively, so SplitMix64 and FNV-1a need no masking.
  Every integer a generator, a fixture or a shrinker can carry is an `Integer`
  (u8, u64 and i128 included, as in the Java and Go lanes). `bounded(n)` takes an
  `Integer` in `[1, 2^64]` (`2^64` is one raw word); the threshold is computed in
  `Word64` as `negate n `mod` n`, which is exact because unsigned negation wraps
  modulo `2^64` (the same spelling as C; the same expression on a signed type is wrong
  for `n > 2^63`, pinned at `n = 2^63 + 1`). `boundedBig` is the VER05 §7.2 k-word
  rejection above `2^64`. A seed is a `Word64`; a fixture seed outside `[0, 2^64-1]` is
  a problem (`caseError`), never reduced modulo 2^64. **Fixture integers are
  capped at 1,000 digits** and counts, lengths and retries at 10,000 *before* any
  conversion to `Int`, so `Integer` input can never overflow an `Int` or allocate
  without bound.
- **Value model.** `Val` is `VNil | VBool | VInt Integer | VBytes ByteString |
  VText Text | VList [Val] | VTuple [Val]`. User types participate through
  `class Canonical a where toVal :: a -> Val` (instances for `Bool`, `Integer`,
  `Int`, `Word8`, `Word64`, `ByteString`, `Text`, lists, pairs, triples, `Maybe`,
  `()`). Rendering and equality never call `show` on a user value.
- **Property convention.** A property is `a -> PropM Bool` (`pureProp` lifts an
  `a -> Bool`). It **holds iff it returns exactly `True`** once forced to weak head
  normal form; a lazily bottom result is a panic (§4), never a pass.
- **Invalid generators fail closed.** A generator constructor given invalid
  arguments (`min > max`, an empty `oneOf`, a zero-total `frequency`) returns a
  `Gen` whose sampler throws `ArgumentError`; `check` reports it as
  `property-exhausted`, so an invalid generator can never pass. Public non-generator
  entry points (`shrinkInt`, `check` with invalid options) throw `ArgumentError` in
  `IO`, and a start value that does not falsify throws `NotFalsifying`.
- **Generator type.** `Gen a` is a sampler `Maybe Budget -> Rng -> IO a` and an
  optional shrinker. Sampling runs in `IO` because filters charge the budget and
  because the `Rng` is a mutable stream (an `IORef Word64`); a stream must not be
  shared between threads. `map` and `flatMap` drop shrinking (no inverse); `filter`
  keeps it and rejects candidates failing the predicate.
- **`int lo hi` refuses a span wider than 16,384 bits** (an invalid generator, CWE-400 on the
  k-word rejection loop).
- **Tuples** are `tuple2` and `tuple3` (no arity-n generics), shrinking one
  coordinate at a time in passes until a pass makes no progress.
- **Type-driven hardening.** `check` requires `NFData a` and `Canonical a`.
- **Custom generators and shrinkers must return FINITE values.** The engine forces every
  sampled candidate and every shrinker result with `force`, which is unbounded on an
  infinite or cyclic value (only rendering and equality are bounded). `timeout` can still
  interrupt it (§4.2), but a finite value is the contract.

## 4. Laziness and asynchronous exceptions

These are the two hazards that do not exist in the other lanes. Each has a test (§10).

### 4.1 Laziness: an exception can hide in a thunk

`try (pure (error "boom"))` returns `Right`; the exception fires later, outside the
`try`. The engine therefore never trusts a value it has not forced **inside** the
handler:

- a sampled candidate is forced with `evaluate (force v)` inside the sampler's
  handler (so a lazy `error` in a generated value is a *generator error*,
  `property-exhausted`, not an engine crash);
- the property's result is forced with `evaluate` inside the property's handler (a
  lazy `error` in the `Bool` is `property-panic`);
- a shrinker's result `(value, steps)` and every shrink candidate's property result
  are forced inside the handler that falls back to the original witness;
- `Canon` and equality run over `Val`, whose lists are lazy: they enforce depth,
  node and output caps **as they traverse**, so an infinite `VList` renders as a
  truncated string and a `Val` whose tail throws renders `<unrenderable>`;
- the verdict fields are strict (bang patterns) and built from already rendered
  `Text`, so no thunk in a `Verdict` can throw when a caller reads it.

### 4.2 Asynchronous exceptions

`SomeException` also catches `ThreadKilled`, `UserInterrupt`, `StackOverflow`,
`HeapOverflow` and `System.Timeout`'s exception. Swallowing them would defeat
`timeout`, `killThread` and Ctrl-C, and would let a property that overflows the stack
report a verdict instead of stopping.

- `Pbt.Safe.tryRecoverable` runs an action and **rethrows** anything that is a
  `SomeAsyncException` (the base class of every asynchronous exception, including
  `Timeout`); every other `SomeException` becomes a `Left`.
- Classification is by **exception type only** (`fromException`). The text of
  `displayException` is never inspected to decide anything; it appears only in
  non-redacted messages, after being forced inside a handler, clipped to 200 characters
  and escaped by `Canon`'s escaper. Redacted messages carry only the exception's type
  name (`typeOf`).
- **Non-allocating loops.** A tight loop that never allocates reaches no safe point, so
  `timeout` cannot interrupt it. Every stanza of the package is compiled with
  `-fno-omit-yields` (the library AND both test suites), and a test pins that `timeout`
  interrupts `spin n = spin n`. A downstream program that links the library but compiles
  its own properties without the flag still has this limitation for ITS code, and must
  add the flag itself.
- Scopes are opened with `bracket`, so a scope always closes, also when it is left by
  an asynchronous exception; the release action is a single non-blocking `atomically`
  (no `retry`), so it cannot itself be interrupted while masked.
- `Pbt.Budget` charges are `atomically` transactions with no `retry`: no budget
  operation can block, so none is an interruption point that could strand a counter
  half updated (and an STM transaction is atomic by construction).

## 5. The shrink-budget tree

A direct port of VER05e §4.1. A scope (`Budget`) is a node with a `parent`, a `depth`,
a count of enclosing checks and caps, and four STM cells: **attempts** (one per
property evaluation, cap `16 * max_steps + 64`), **work** (elements copied, at least 1
per charge, cap `max_shrink_work` = 64M), `closed`, and a `refused` flag **shared by
the whole tree** (children alias the root's cell).

- **All or nothing.** `charge b n` (an attempt) and `chargeCopy b n` (work only, so
  attempt counts and the corpus are unchanged) run in **one `atomically`**: walk from
  `b` to the root, require on every node `not closed`, `attempts < max_attempts` and
  `cost <= max_work - work` (the overflow-safe `cost > max - used` test; `maxBound`
  is refused), then increment every node. A refusal sets the shared `refused` flag and
  leaves every counter unchanged. Because the whole walk is a single STM transaction
  there is no lock, no lock order, no leaked reservation and no rollback code.
  Callers charge **before** building a candidate; once refused, every loop ends.
- **Accounting.** One property evaluation is exactly one attempt and one unit of work,
  charged by the `falsifies` function the entry point hands to a shrinker;
  a sequence shrinker additionally charges the **copy** (`chargeCopy`, the candidate's
  size) before building each candidate. Shrinkers themselves never re-charge the
  evaluation, so attempts equal evaluations (the C lane's accounting; the Ruby lane
  charges some evaluations several times).
- **Public entries open a child, internal combinators join.** `shrinkInt`,
  `shrinkList`, `shrinkBytes`, `simulateReplay`, `shrinkGen` and the runner's shrink
  always open a **child** of the scope they are given with a fresh local cap and
  charge every evaluation including the initial one. `tuple`, `list`, `filter` and
  `bytes` shrinkers **join** the node they are handed. There is no classification of a
  call as "inside a property"; the chain always reaches the outermost root, so the
  total work of one top-level `check` is capped by that root whatever the nesting.
  Owning nesting is capped at depth 64; deeper scopes join.
- **`check` wraps its whole body, case loop included,** in one scope (a root with no
  parent, a child otherwise) with attempt cap `num_tests + 16 * max_steps + 64`. A
  check **nested** in a property pays one attempt at entry (refused: it returns
  `property-exhausted` without evaluating anything), one per case after the first, the
  work of every sampled value, and one attempt per filter retry. A starved nested check
  is exhausted, never a pass; a **top-level** check has no parent and always
  evaluates its first case. **Top-level filter retries are charged as work**
  (`chargeCopy 1`), because the top-level attempt cap would wrongly exhaust a
  legitimate rare filter; `max_retries` is at most 10,000.
- **Nesting is capped.** Each scope carries a count of enclosing checks. A check whose
  parent already has 64 enclosing checks, or sits at depth 63 or more, evaluates
  nothing, marks the tree refused and returns `property-exhausted`.
- **Starvation is never a pass.** If any charge in the tree was refused, a top-level
  check that would pass returns `property-exhausted`; a falsified verdict is never
  overridden. The single re-verification charges one attempt to the check scope (not
  the possibly spent shrink scope); if refused, the shrunk value is not adopted and
  the **original witness is reported with 0 steps**.
- **A closed or stale handle is spent.** A node whose own `closed` flag, or whose
  ancestor's, is set refuses every charge. There is **no resolution to a nearest open
  ancestor** (the Ruby and Elixir lanes resolve upward, Java lets a closed ancestor be
  charged) and **a closed handle is never a way to get a fresh root**. A thread that
  outlives the check that spawned it and still holds its handle therefore finds every
  charge refused; a check it starts under that handle returns `property-exhausted`.
- Generation work is approximate: a sampled value's work is its collection length plus,
  one level deeper, the length of each child collection, byte string or text, visiting at
  most 10,000 children, computed over the lazy `Val` view so a hostile `toVal` costs 0.

### 5.1 The ambient budget is an explicit parameter (decision)

Java and Ruby carry the current scope in an inheritable thread-local or fiber storage.
Haskell has neither: `forkIO` copies nothing. Three designs were considered.

| Design | Result |
|---|---|
| Global or thread-local ambient state (`IORef` keyed by `ThreadId`, `unsafePerformIO`) | Rejected. A hidden global, unsound under `forkIO` and `async`, leaks across tests, and makes "which budget am I charging?" invisible in types |
| Implicit parameters (`?budget :: Budget`) | Rejected. A GHC extension, easy to drop silently at a `forkIO` boundary, and it infects every signature |
| **Explicit parameter, with a `ReaderT`-shaped monad for properties** | **Chosen** |

The explicit design has two layers:

1. Every engine function that can open a scope takes `Maybe Budget` (the parent):
   `Nothing` is a fresh root, `Just h` a child of `h`. This is the C lane's explicit
   context parameter. `check` is `checkWith Nothing Nothing`.
2. Properties run in `PropM`, a hand-written `Budget -> IO a` newtype (no `mtl` or
   `transformers`) with `Functor`, `Applicative`, `Monad` and `MonadIO` instances.
   `askBudget` returns the scope the running check opened; `checkM` is `checkWith`
   with that scope as the parent. A property that nests a check, a shrinker or a
   generator therefore charges the outer tree by writing `checkM`, and passes the
   handle to `shrinkInt (Just h)` or `sample (Just h)` by calling `askBudget`.

**Threads.** `forkIO` does not inherit the handle. The carriers are
`withBudget :: Budget -> PropM a -> IO a` (run a `PropM` action under a handle) and
`forkWithBudget :: PropM () -> PropM ThreadId` (fork a thread that runs the action under
the **same** handle; the body runs unmasked through `forkIOWithUnmask`, and every exception
it throws is dropped, so nothing reaches the default stderr handler with its `show` text;
`forkWithBudgetHandler` instead delivers the exception's TYPE name to a caller-supplied
handler. The child is not killed when the scope closes: it keeps a spent handle and every
charge it makes is refused). A thread started with a bare `forkIO` has no handle and
**gets a fresh root** when it calls a top-level `check`; its work is bounded by that root
only. This is the same documented escape as the Elixir process dictionary and Swift
thread-local, pinned by a test, not hidden. A thread that carries a handle shares one
global budget through the STM cells; if the scope that spawned it has closed, every
charge it makes is refused (§5, spent handles).

## 6. Runner, verdicts and failure vocabulary

`checkWith parent override seed gen options prop` (`check` is `checkWith Nothing Nothing`,
`checkM` is `checkWith` with the running property's scope as the parent) runs `num_tests`
(default 100) cases from `SplitMix64(seed)`. It always returns a verdict; invalid options throw
`ArgumentError` instead (never a pass).

- `num_tests == 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- a generator error of any kind (an exception from the sampler, including a forced lazy
  one, an exhausted filter, a refused budget) → `property-exhausted`.
- a recoverable exception from the property, or a lazily bottom `Bool` →
  `property-panic`. Asynchronous exceptions propagate (§4.2).
- the property returns `False` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is **re-verified**: if the shrinker throws, returns a value that no
longer falsifies, a value on which the property throws, or an out-of-range step count,
the original candidate is reported with 0 shrink steps. While shrinking, a candidate on
which the property throws counts as *not falsifying*. `shrinkOverride` replaces the
generator's shrinker so a defective shrinker can be tested.

Closed failure ids: `property-counterexample`, `property-exhausted`, `property-timeout`,
`property-panic`; `isFailureId` is true only for those four texts. `property-timeout`
is registered but never produced (VER05 §7.4 #5); there is no wall-clock timeout in
the engine, a caller that needs one wraps `check` in `System.Timeout.timeout`, which is
safe because asynchronous exceptions are never swallowed (§4.2).

### 6.1 CI redaction

`ci` is `CiUnset` (the default), `CiOn` or `CiOff`. **`CiUnset` is treated as CI**:
the engine fails closed. `ciFromLookup :: Maybe String -> CiMode` is pure: `Nothing`,
empty, `0` or `false` (trimmed, any case) is `CiOff`, anything else `CiOn`. A redacted
verdict carries **no seed, no counterexample and no minimal value**, and its message
carries no concrete value: only the iteration, the step count, the exception type name
for a panic and `repro-fnv`, the FNV-1a of `iter:<i>;steps:<s>;shrunk:<canonical>`
as 16 lowercase hex digits. Caveat (VER05 §7.4 #3): `repro-fnv` is a correlation id, not
a secret. It is an UNKEYED FNV-1a: anyone who can guess a small witness can confirm the
guess by brute force, so the redacted verdict is **not a redaction guarantee** for small
witness spaces. The format is the cross-lane one and is deliberately not changed here.

## 7. Canonical rendering (`Canon`)

`render :: Val -> Text` is total and bounded; it never calls `show` on a user value and
never throws.

```text
VNil               nil              VBool          true / false
VInt               decimal; above 100,000 bits  <bigint N bits>   (the bit length is
                   found by shifting, never by showing the number)
VBytes             0x<lowercase hex>
VText              "quoted", with \" \\ \n \r \t and \uXXXX escapes below
VList              [a,b]            VTuple         (a,b)
```

- **Caps, enforced as the value is traversed and before allocation:** nesting deeper
  than 16 renders `...`; more than 10,000 nodes truncates; the output is capped at
  1 MiB; a text or byte string is clipped to the remaining output budget **before**
  it is escaped or hex-encoded; an integer is tested against `2^100000` before
  `show`. Truncated output ends in `...`. A lazy list is consumed only as far as the
  caps allow, so an infinite list terminates.
- **Escapes** (copied exactly from the Elixir lane's `Canon`, VER05m §3): every
  C0 control, DEL (U+007F) and C1 control (U+0080 to U+009F); U+2028 and U+2029;
  the bidirectional controls U+061C, U+200E, U+200F, U+202A to U+202E and U+2066
  to U+2069; the invisible formatting characters U+00AD, U+034F, U+180E, U+200B to
  U+200D, U+2060 to U+2064, U+FEFF, U+FFF9 to U+FFFB and the tag block U+E0000 to
  U+E007F. `\n`, `\r` and `\t` have short forms; every other escaped character is
  `\uXXXX` (four lowercase hex digits), or `\u{XXXXX}` above U+FFFF. `"` and `\` are
  escaped only inside a quoted text. The same escaper is used for case ids, message
  fragments and exception text.
- A failure while rendering (a `toVal` that throws, a lazy tail that throws) turns the
  **whole** witness into `<unrenderable>`; asynchronous exceptions are rethrown.
- `Val` equality (`sameVal`) is kind-sensitive (`true` is not `1`, bytes are not text, a
  list is not a tuple), compares at most 10,000 nodes to depth 64, and answers `False`
  past either cap.

## 8. Conformance, strict parsing and negative controls

`StrictJson.parse :: ByteString -> Either JsonError Json`:

- document size at most 8 MiB, at most 200,000 values (object keys count), container
  nesting at most 64 (the 65th is refused before recursing);
- invalid UTF-8 refused (overlong forms, surrogates, code points above U+10FFFF), a
  leading byte-order mark refused, raw control characters in strings refused, lone or
  reversed surrogate escapes refused, leading zeros, `+1`, `NaN`, comments, trailing
  commas and **trailing data** refused;
- **duplicate object keys are refused, compared by their decoded bytes** (`"a"` and
  `"a"` are the same key); strings are kept as UTF-8 `ByteString`;
- **numbers are kept as text** (`JNumber ByteString`), so a 30-digit integer is never
  converted to a `Double`.

`Tfv.validate` is VER05d's validator: header `tfv 1`, one space between tokens,
lowercase even hex, canonical integers (at most 1,000 digits), counts that agree and
are never trusted for allocation, no duplicate field, record id or map key, strict
UTF-8 in `S`, depth 64, at most 1 MiB and 200,000 tokens.

`Pbt.Corpus.evaluate` takes the parsed case list and evaluates every case with the
engine's own public functions, returning problem lines (none means conform) and the
evaluated count.

- **Strict decoding:** exactly the fields of the op; ints are canonical decimal strings
  (at most 1,000 digits); bytes even-length lowercase hex; an unknown op, property or
  malformed value is a *problem*, never a skip; a non-string id is a problem. Every
  problem line starts with its case id (escaped and clipped).
- **CWE-400 caps:** counts, retries, lengths and list sizes above 10,000 are problems;
  `count x max_len <= 2^20` for `gen_bytes`; `count x ceil(bitlen/8) <= 2^20` for
  `gen_bounded`; a `gen_*` case whose `outputs` length is not `count` is refused
  before generating; at most 10,000 cases; fragments are clipped to 200 characters
  and problems to 400, at most 1,000 problems are kept.
- **Coverage gates:** every op and all four failure ids present, an unregistered id
  rejected, case count equal to 25.
- `corpus_reject` texts must be rejected by `Tfv.validate`; `filter_exhausted` runs end
  to end through `check` and counts predicate calls exactly (`max_retries`).

The conformance suite (`test-suite pbt-v1-conformance`, a separate Cabal target so the
BUILD runs and names it) loads the committed fixture, asserts zero problems and exactly
25 evaluated, and prints `pbt-v1: 25/25`. The unit suite carries the **negative
controls**: for every one of the 25 real cases the test applies each corruption its op
defines (flipped output, wrong minimal, steps+1, wrong failure id, flipped validity, a
valid TFV in place of a reject text) and asserts that the run fails **and every problem
names that case's id and no other**. Truncated and dropped-case lists and an unknown op
are problems. Engine sabotage through `Config` (a changed bisection, a changed unit
step, a skipped deletion phase, a shifted RNG, a wrong rejection threshold) must make
the corpus run report problems. `Config` is a plain value passed down, not global
mutable state: there is no `unsafePerformIO` anywhere in the package.

### 8.1 Pinned tests for what the corpus cannot catch

The corpus draws spans small enough that a wrong rejection threshold almost never
fires and it never exercises the bisection halves independently. Values below are taken
from the C and Ruby lanes and are re-derived by the independent reference
`code/scripts/pbt_conformance.py`.

- **The rejection draw.** SplitMix64's finalizer is a bijection, so state 0 yields the
  word 0; the seed `0 - 0x9e3779b97f4a7c15 mod 2^64` (`7046029254386353131`) therefore
  starts a stream whose first word is `0` and whose second is `16294208416658607535`.
  `bounded(3)` has threshold 1, so the `0` **must be rejected** and the answer is
  `16294208416658607535 mod 3 = 1`. A threshold spelled `(-n) mod n` on a signed or
  unbounded integer is 0 and answers 0.
- **The `2^63 + 1` boundary.** `n = 2^63 + 1` has threshold `2^63 - 1`. Seed 3's first
  word `2092789425003139053` is below it, so `bounded` answers from the second word,
  `3694763184872335752`; seed 1's first word is above it and is accepted,
  `1227844342346046656`.
- **The blind spot is real and pinned.** A rejection threshold of 0 (the classic
  `(-n) mod n` bug on an unbounded or signed integer) still passes all 25 corpus cases;
  only the forced-rejection pin above catches it. The unit suite asserts both facts, so
  nobody mistakes 25/25 for proof that `bounded` is unbiased.
- **`bounded(1)` consumes a word:** after `bounded 1` from seed 7 the next word is
  `309689372594955804`.
- **Shrink step counts on the edge cases:** `shrinkInt 54321 0 0 100000 (<=100)` is
  `(101, 15)`; everything failing is `(0, 1)`; `maxSteps = 0` returns the start with 0;
  `maxSteps = 3` returns exactly 3 steps; `shrinkInt 100 0 (-100) 500` with
  `inRange (-10) 50` is `(51, 49)`; a negative bisection floors (`-81` gives candidate
  `-40`, not `-41`); `shrinkInt 99 10 10 500 (<5)` is `(10, 1)`;
  `shrinkList [0,0,0,0,5,0,0,0]` with "does not contain 5" is `([5], 3)` and its
  evaluation order is exactly initial, `[]`, `[0,0,0,0]` (rejected), `[5,0,0,0]` (the
  right half accepted), `[5,0]`, `[5]`, then the element probes `[3]`, `[4]`;
  `[5,0,0,0,0,0,0,0]` and `[0,0,0,0,0,0,0,5]` take 3 steps; `shrinkBytes
  0102030405060708 (len < 5)` is `(0000000000, 16)`; `simulateReplay 42 (<=100)` is
  `(413, 101)`.

### 8.2 Accounting note

One property evaluation is one attempt (§5). The pbt-v1 results do not depend on the
accounting, so the corpus (maximum 1,000 steps, at most 8 elements) never approaches the
`16 * max_steps + 64` cap; the budget-tree tests assert exact counts against this
accounting (for example a nested check inside the case loop of an outer check of 6 cases
charges exactly `6 x (1 + 3)` attempts to the shared root).

## 9. Build and inputs

`BUILD` is `cabal test all` under the Darwin linker guard every Haskell front carries
(`DEVELOPER_DIR=/Library/Developer/CommandLineTools` when `uname -s` is Darwin) and runs
both test suites, `spec` (units, hazards, negative controls) and `pbt-v1-conformance`.
`BUILD_windows` runs the same `cabal test all` through `cmd.exe`. The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `haskell/test-bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`) beside the other lanes; the selection tests that enumerate
consumers fail closed until Haskell is added. Coverage is measured with HPC
(`cabal test all --enable-coverage`, with the `hpc` that ships with the pinned GHC);
sibling Haskell packages run no coverage gate, so none is added to `BUILD`, and the
changelog quotes the measurement (96% of expressions of the library at release (6301/6558); the
residue is mostly error-message strings of branches that are asserted by outcome, not
by text).
`required_capabilities.json` is empty.

## 10. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems.
- Negative controls (§8) and the sabotage hooks.
- The pinned tests of §8.1.
- **Hazard 1 (laziness):** a property returning `error "x"` as its `Bool` is
  `property-panic`; a generator whose value contains a lazy `error` is
  `property-exhausted`; a shrinker returning a lazy bottom keeps the original witness; a
  `Canonical` instance with a throwing or infinite `toVal` renders safely; an infinite
  `VList` renders in bounded time and space.
- **Hazard 2 (asynchronous exceptions):** `timeout` cancelling a property that loops;
  `killThread` of a thread inside `check` leaves the scope closed; `throwIO ThreadKilled`,
  `UserInterrupt` and `StackOverflow` thrown **by the property** propagate out of
  `check`; a synchronous `ErrorCall`, `ArithException` and a custom exception are
  `property-panic`; classification does not depend on message text (an
  `ErrorCall "ThreadKilled"` is a panic).
- **Hazard 3 (budget tree):** the required set of VER05e's `ShrinkBudgetTest`
  (nesting depth 1 to 3 and the branching case, a recursing custom shrinker, a filter
  predicate that shrinks, a filter sampler rejecting everything, concurrent charging, a
  refused take leaving every counter unchanged, a huge cost, scope release after an
  exception, depth-cap joining, re-entrancy equality, starvation never passing, a
  refused re-verification, top-level filter retries as work); Haskell-specific: a thread
  holding the handle charges the same tree, a thread started with `forkIO` gets a fresh
  root (pinned), a closed handle is spent and never a fresh root, a stale handle used
  for `checkM` is exhausted.
- **Hazard 4 (arithmetic):** `Word64` wrap at `maxBound`, a negative and an over-large seed
  wrap, a 1,001-digit fixture integer refused, a 1,000-digit one accepted.
- **Hazard 5 (hostile values):** `Canon` and `sameVal` on an infinite list, a deep
  nesting, a value with a million nodes and a 10 MiB text.
- **Hazard 6 (no hangs):** every test runs under `System.Timeout.timeout` (60 seconds)
  and fails instead of hanging.
- A regression test per hardening item of §3 to §8 (strict JSON and TFV rejection
  lists, caps, CI redaction, failure ids).

## 11. Divergences from other lanes

| # | Topic | Haskell | Others | Reason |
|---|---|---|---|---|
| 1 | property result | exactly `True` holds | Ruby truthy; Java/C# `true` | fail-closed, and a typed `Bool` |
| 2 | ambient scope | none: explicit `Maybe Budget` plus `PropM` | thread-local / fiber storage | §5.1 |
| 3 | locking | one STM transaction per charge | Java/C# lock, Ruby Mutex, C none | composable, no lock order |
| 4 | closed ancestor | spent: refuses every charge | Java charges it, Ruby/Elixir resolve upward | stale handles are never a fresh root |
| 5 | attempt accounting | one attempt per evaluation | Ruby charges several | C-lane accounting |
| 6 | async exceptions | `SomeAsyncException` rethrown | Ruby re-raises a fixed set | §4.2 |
| 7 | laziness | forced inside every handler | n/a | §4.1 |
| 8 | invalid generator | sampler throws, `check` is exhausted | Ruby/Java throw at construction | a pure constructor must not be a thunk bomb |
| 9 | hooks | `Config` value passed down | module accessors, function-pointer struct | no global mutable state |
| 10 | integers | `Integer`, 1,000-digit fixture cap | Ruby/Python unbounded, C 300 digits | Ruby's cap |
| 11 | tuples | `tuple2`, `tuple3` | Ruby n-ary | no arity-n generics |
| 12 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |
| 13 | JSON | hand-written strict parser | Go `encoding/json` | zero-dependency rule |
