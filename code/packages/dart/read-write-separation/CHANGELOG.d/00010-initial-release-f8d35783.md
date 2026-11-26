## 0.1.0

- Initial Dart lane of the read-write-separation identity: `Capability`
  declarations with flavor/trust/justification annotations,
  `classifyCapability` with the category/action(/target) default tables,
  `summarizeManifest` (13 counters plus risk flags), and `validateManifest`
  rejecting untrusted-input/actuation mixing and same-resource read/write
  overlap (exact or `prefix*` glob). Zero runtime dependencies.
- One mapping note: validation throws `RwsViolation` instead of returning
  `Result`; default tables, overlap and glob rules, dedup, and both message
  texts mirror the reference exactly.
- Tests mirror the reference suite (35 unit tests), add 6 seeded property
  tests over the in-lane `pbt` package and a 2,000-iteration fuzz campaign
  over the in-lane `fuzz` package; 100% line/branch and 95.5% function
  coverage (uncoverable enum constructors only).
