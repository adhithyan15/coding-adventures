## [0.01] - Initial release

### Added

- The Perl property-based testing engine of VER05p: `Int`, `Prng` (SplitMix64,
  FNV-1a, `bounded`, `bounded_big`, `sub_seed`), `Gens`, `Shrink`, `Runner`,
  `Budget` (the VER05e 4.1 hierarchical shrink-budget tree), `Canon`, `StrictJson`,
  `Values`, `Tfv` and `Corpus`.
- Conformance with the shared `pbt-v1` corpus: 25 of 25 cases with exact outputs,
  minimal witnesses and shrink step counts; a negative control for every case and
  every corruption its op defines; sabotage hooks for the bisection, the unit step,
  the deletion phase, the RNG and the rejection threshold.
- Cross-lane pins the corpus cannot catch: the rejection draw at seed
  7046029254386353131, the 2^63 + 1 boundary, `bounded(1)` consuming a word,
  floored negative bisection, and an explicit test that a rejection threshold of 0
  still passes the corpus.

### Design (the Perl hazards, VER05p sections 3 to 5)

- 64-bit words are UVs; wrapping add and multiply are built from 32-bit halves and
  cross-checked against `use integer` arithmetic, `Math::BigInt` and Python vectors.
  Public integers are native IVs or normalized `Math::BigInt` values; floats, strings
  and `undef` are refused; fixture integers are capped at 1,000 digits before parsing.
- User values are inspected through `B` only (no overload, tie or DESTROY ever runs
  from the engine); user code runs through `Safe::call`, which classifies by `eval`,
  localizes `$@`, `$_` and the die/warn handlers, and turns `last`/`next` out of a
  property into a panic.
- The budget carrier is `local $Budget::CURRENT` (restored on every exit) with an
  optional explicit `parent`; a closed or stale handle is spent and never a fresh root.
- Engine subs are read-only globs after load.

### Notes

- No third-party dependency; `cpanfile` lists only `Test::More` (core) for tests.
- Coverage is measured on demand (see the README); no `Devel::Cover` dependency.
