## 0.1.0

- Initial Dart lane of the clock identity: `Clock` square-wave generator
  with edge records and listeners, `fullCycle`/`run`/`reset`,
  `ClockDivider` frequency division, and `MultiPhaseClock` non-overlapping
  phase rotation. Zero runtime dependencies.
- Reference panics become `ArgumentError`s with identical messages
  (validation order preserved); out-of-range phase reads throw `RangeError`
  via list indexing, as in the reference.
- Tests mirror the reference suite (25 unit tests), add 8 seeded property
  tests over the in-lane `pbt` package and a 2,000-iteration differential
  fuzz campaign over the in-lane `fuzz` package; 100% line/branch/function
  coverage.
