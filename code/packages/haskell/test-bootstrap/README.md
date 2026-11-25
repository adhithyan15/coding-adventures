# test-bootstrap (Haskell)

A zero-dependency property-based testing engine with integrated shrinking, a
hierarchical STM shrink budget, strict JSON and TFV validators and the shared
**pbt-v1** conformance corpus runner. It is the Haskell lane of the VER05 family
(spec: [`VER05n`](../../../specs/VER05n-haskell-pbt-engine.md)); the Java, C#, Ruby,
C and other lanes implement the same algorithms and are checked against the same 25
corpus cases, with exact outputs, minimal witnesses **and shrink step counts**.

Only GHC boot packages are used: `base`, `bytestring`, `containers`, `text`,
`deepseq`, `stm`.

## Where it fits

```text
Warrant claims -> generators (draw order pinned by pbt-v1) -> candidate
                  integrated shrinker -> minimal counterexample
                  runner: verdict honesty, re-verified witness, CI redaction
```

## Usage

```haskell
import CodingAdventures.TestBootstrap

main :: IO ()
main = do
  -- every x in [0, 100000] is at most 100?  No: it shrinks to exactly 101.
  v <- checkPure 42 (int 0 100000) defaultOptions {optCi = CiOff} (<= 100)
  print (vFailureId v, vMinimal v, vShrinkSteps v)
  -- (Just "property-counterexample", Just "101", 15)
```

A property is `a -> PropM Bool` (`pureProp` lifts an `a -> Bool`) and holds only when
it returns exactly `True`. A generated value must be `NFData` and `Canonical`
(`toVal :: a -> Val`); the engine renders witnesses through `Val`, never `show`.

### Verdict honesty

- zero cases is never a pass (`property-exhausted`);
- a property that throws, or whose `Bool` is a lazy `error`, is `property-panic`;
- the reported witness is re-verified; a lying shrinker cannot change it;
- `CiUnset` (the default) and `CiOn` redact: no seed, no counterexample, no minimal
  value, only an opaque digest. `ciFromLookup <$> lookupEnv "CI"` reads the flag;
  the library never touches the environment.

### The budget handle is explicit

Haskell has no inheritable thread-local, so the shrink budget is an explicit
parameter. A property that nests a check uses `checkM`; one that starts a thread uses
`forkWithBudget` so the thread charges the **same** tree. A thread started with a bare
`forkIO` has no handle and gets a **fresh root** (documented in VER05n 5.1). A handle
whose scope has closed is *spent*: it refuses every charge.

```haskell
prop :: Integer -> PropM Bool
prop x = do
  inner <- checkM 7 (int 0 9) opts (pureProp (<= 9))   -- charged to the outer check
  pure (vPassed inner)
```

- `CiUnset`/`CiOn` digests (`repro-fnv`) are an **unkeyed FNV-1a fingerprint**, not a
  secret: a small witness space can be confirmed by brute force. It is not a redaction
  guarantee.

### Exceptions

Asynchronous exceptions (`ThreadKilled`, `UserInterrupt`, `StackOverflow`,
`timeout`, ...) are never swallowed: `check` rethrows them, so
`System.Timeout.timeout` works around a check. Classification is by exception type,
never by message text. The package is built with `-fno-omit-yields` so `timeout` also
interrupts a non-allocating loop; add the flag to your own test stanza for the same
guarantee on your properties. Custom generators and shrinkers must return **finite**
values (the engine `force`s them). `forkWithBudget` children run unmasked, drop their
exceptions (`forkWithBudgetHandler` receives the type name only) and are not killed when
the scope closes; they hold a spent handle.

## Coverage-guided fuzzing (VER06o)

> **The wall-clock limit only holds if the TARGET and the code it calls are compiled
> with `-fno-omit-yields`.** GHC interrupts a thread only at a safe point: a
> non-allocating loop, or a long non-yielding call (bignum `^` with a huge exponent, an
> unsafe FFI call) in code built without the flag hangs the **whole process**, and the CI
> job timeout is the only backstop (fail closed, but no verdict). Add
> `ghc-options: -fno-omit-yields` to your test stanzas. An out-of-process runner is future
> work.

`CodingAdventures.TestBootstrap.Fuzz` is the Haskell lane of the shared fuzz engine
(spec `code/specs/VER06o-haskell-fuzz-engine.md`). It mutates a seed corpus with the
cross-lane mutator algebra, keeps inputs that reach new edges, deduplicates and
minimises crashes, and reports through the closed `fuzz-*` failure vocabulary.

```haskell
import qualified Data.ByteString as B
import Control.Exception (throwIO)
import Control.Monad (when)
import CodingAdventures.TestBootstrap.Fuzz
import qualified CodingAdventures.TestBootstrap.Fuzz.Handle as H

main :: IO ()
main = do
  report <- run [B.pack [0, 0]] defaultFuzzOptions {fzMaxIterations = 500, fzSeed = 7} $ \dat h -> do
    H.edge h (H.edgeId "entered")                 -- report the branches you care about
    when (dat == B.pack [0xFF, 0xFF]) (throwIO (userError "boom"))
  v <- verdict report defaultFuzzOptions {fzCi = CiOff} (executor target defaultFuzzOptions) "my_target"
  mapM_ print (fvLines v)                         -- not ok 1 - my_target [fuzz-crash:xxxxxxxx]
```

* **Coverage is target-reported** (`H.edge`), not HPC: HPC counters are a process-global,
  cumulative `.tix` table with no safe per-execution reset (and `--enable-coverage` uses
  the same table). The coverage tier says so (`caller-reported`, else
  `black-box-uninstrumented`).
* **One thread per execution**, under a wall-clock `timeout`; every asynchronous exception
  is rethrown, never swallowed. Crashes are classified by exception **type** only (never
  the message) and have no frames, so all `ErrorCall`s of one target are one crash.
* **Memory is a per-thread ALLOCATION limit** (`setAllocationCounter` /
  `enableAllocationLimit`): it counts total bytes allocated, not residency, to about 4 KiB.
  The default `fzMemBudget` is 256 MiB of allocation. It is `oom` only when the worker's own
  limit trips. Threads the target forks, masked loops and foreign calls that never return
  are outside the engine's reach (the worker is leaked, never waited for).
* **Budgets are cooperative and sticky**: `H.tick` and `H.reserve` throw a sentinel on the
  first breach and on every later call (either budget); the engine judges after the handle
  is retired, so a target that catches the sentinel is still a timeout or oom. Inside one
  GHC process a hostile target can still reach the handle's mutable state.
* The allocation limit is not delivered while the worker is masked (honest
  `modifyMVar` / `bracket` acquire code is masked too), and a hostile target can call
  `setAllocationCounter` / `disableAllocationLimit` on its own thread to fake or hide an oom
  (the trusted-target model). A worker that ignores the kill (a masked loop) is leaked; after
  the first leak later waits are short, and a campaign that leaks 4 workers is aborted with
  the `workers-leaked` verdict reason (public line `fuzz-timeout`), never a pass.
* Retained corpus seeds and crash inputs are **copies**, so a slice of a large buffer never
  pins its parent while only its own length is charged.
* A zero-execution campaign is never a pass; crash witnesses are re-run and reported as
  reproduced, flaky or unverified; an unset CI mode counts as CI and carries no seed or
  witness. Seeds are capped at 10,000 and 1 MiB, retained bytes at 256 MiB.

## Conformance

`cabal test all` runs three suites: `spec` (unit, hazard, isolation and negative-control
tests), `pbt-v1-conformance`, which loads `code/specs/fixtures/pbt-v1/cases.json` and must
print `pbt-v1: 25/25`, and `fuzz-v1-conformance`, which loads
`code/specs/fixtures/fuzz-v1/cases.json` (`fuzz-v1: 25/25`) and replays the Go lane's 820
oracle vectors parsed from `code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go`
(`vectors: 820/820`).

```bash
cd code/packages/haskell/test-bootstrap
cabal test all
cabal test all --enable-coverage    # HPC; the changelog quotes the measurement
```

## Limits

Counts, lengths and retries are capped at 10,000 (fixtures) or the limits in
`Pbt.Types`; fixture integers at 1,000 digits; JSON at depth 64 / 8 MiB / 200,000
nodes; rendering at depth 16 / 10,000 nodes / 1 MiB. `property-timeout` is registered
but never produced.
