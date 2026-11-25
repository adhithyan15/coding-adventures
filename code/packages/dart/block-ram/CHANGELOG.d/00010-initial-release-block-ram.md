## 0.1.0

- Initial Dart lane of the block-ram identity: gate-level `SramCell`
  and `SramArray`, `SinglePortRAM` with read-first / write-first /
  no-change modes, `DualPortRAM` with same-address collision errors,
  and `ConfigurableBRAM` with budget-preserving reconfiguration —
  all with cycle-accurate 0→1 edge semantics. Bit validation is
  fail-closed (`ArgumentError`, always), strengthening the
  reference's debug-only asserts; check placement still mirrors the
  reference exactly (read-path data is never inspected).
- Tests mirror the reference suite (all 14 tests) and add the
  untested-corner matrix, 8 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line coverage.
