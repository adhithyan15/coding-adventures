## [0.3.0] - 2026-11-24

### Added

- PIF-003c (VER32 1.4 A1 to A3, G1, G2, 5.3 R-AX-1, 8.3, 9.2): `footprint`, the axioms a theorem's type and value depend on, transitively over the whole environment chain with explicit stacks (a constant's footprint includes itself; inductives contribute constructors and recursor, recursors their rules; an unknown constant is `UnknownConstant`).
- Classification: a `Warrant.Assume.*` or `Src.*` axiom that exactly matches a registry entry is a registered (c) assumption (`Src.*` never bucket (a), with the "consistency not established" note); every other axiom, including `propext`, `Classical.choice` and `Quot.sound`, is an unclassified (c) assumption because A1 needs `axiom.*` roles (kernel slice 4).
- `registry`: the registry file format (`pif-registry 1`; name CTE, universe parameters, binding-CTE digest of the type, environment-prefix digest), exact matching, and the per-file `GateInputs::axiom_registry` built with each file's environment prefix. `pif-check --registry <file>`.
- `claims` and the report: per claim the claim id, theorem, goal digest, base and full environment digests, footprint with class and bucket, bucket counts, certificate id, kernel crate version and commit (`PIF_KERNEL_COMMIT`), and `rungs: "pending"`.
- Tests: a theorem using an axiom (directly, through a definition, and only in its type) shows it in its footprint; a theorem using none shows an empty footprint; an axiom's footprint is itself; registry matching by name, parameters, statement digest and prefix; claim-file axioms refused unless registered exactly; the CLI report; the kernel version constant equals the kernel manifest.

### Changed

- `report::success_report` takes a `Provided` summary and the per-claim reports; `LoadOptions` gains `registry`.
