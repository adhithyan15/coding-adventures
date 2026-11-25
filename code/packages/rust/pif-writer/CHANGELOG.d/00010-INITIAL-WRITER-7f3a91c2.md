## [0.1.0] - 2026-11-24

### Added

- PIF-004a: `pif-writer`, an untrusted producer library for the Proof Interchange Format (VER32). `Writer` hands out typed `Name`, `Level` and `Expr` handles and emits canonical text: dense backward-only indices, maximal sharing, post-order creation-order emission, the grammar's exact spacing and escaping, the `features` line (automatic for `lits`, `quotient`, `mutual`), `import` lines and `end <count>`.
- Builders for names (string and numeric components, dotted helper), levels (`succ`, `max`, `imax`, `param`), expressions (`var`, `sort`, `constant`, `app`, `lam`, `pi`, `let_`, `nat`, `string`, `proj`) and the records `axiom`, `definition`, `theorem`, `opaque`, `inductive`, `quot`, `prim`, `goal`.
- Refusals that mirror reader rules: bad name components, names over 32 components or 4096 decoded bytes (`MAX_NAME_COMPONENTS`, `MAX_NAME_BYTES`; `name_num` is fallible), `ascii_names_only()` for reviewed files (R-NAM-3), anonymous declaration names, duplicate universe parameters, bad claim ids, digests, numerals and bound-variable indices.
- Tests: the VER32 4.6 worked example is reproduced byte for byte; every emitted file is read by `lean_kernel::pif::read`; 300 generated files; determinism and sharing properties. Line coverage 99.5%.
