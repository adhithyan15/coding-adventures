## 0.1.0

- Initial Dart lane of the capability-cage identity (slice 1): the
  11x17 `Category`/`Action` taxonomy, `Capability` declarations,
  `Manifest` JSON loading with schema/pair/RWS validation,
  `matchTarget` glob matching, and `CapabilitySurfaceSummary` counting.
  Audit logging, backends, and secure wrappers follow in later slices.
- Tests mirror the reference suites (81 unit tests incl. the exhaustive
  taxonomy-fixture check), add 8 seeded property tests over the in-lane
  `pbt` package and a 2,000-iteration fuzz campaign over the in-lane
  `fuzz` package; 100% line/branch and 97.9% function coverage (implicit
  enum constructors only).
