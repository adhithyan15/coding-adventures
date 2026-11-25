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

## Conformance

`cabal test all` runs two suites: `spec` (unit, hazard and negative-control tests) and
`pbt-v1-conformance`, which loads `code/specs/fixtures/pbt-v1/cases.json` and must
print `pbt-v1: 25/25`.

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
