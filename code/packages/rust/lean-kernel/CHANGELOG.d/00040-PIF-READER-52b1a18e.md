## [0.4.0] - 2026-11-24

### Added

- PIF-002: the trusted Proof Interchange Format reader (`src/pif.rs`, VER32 section 5, VER11a section 4.2 row "Export reader", at most 600 counted lines, enforced in `tests/ceiling.rs`). `pif::read(env, bytes, &GateInputs) -> Result<ReadReport, PifError>` turns PIF text into calls on `Environment`: names, levels and expressions into dense, backward-only tables (bound variables through `try_bvar`), `axiom`/`def`/`theorem`/`opaque` records into `Declaration` constructors of the matching kind, inductive blocks into `Environment::add_inductive`, goal records into checked goal bindings.
- Every reader rule of VER32 5.3 that does not need a later kernel slice, each with its error kind in `PifError`: R-LEX-1 to 4, R-VER-1 to 3, R-IMP-1, R-IDX-1 to 3, R-NAM-1 to 3, R-CNT-1 and 2, R-FEA-1, R-AX-1, R-DER-1, R-GOL-1 to 4, R-DEC-1 (forwarded) and R-LIM-1 (every section 7 limit that applies).
- The accepted language is exactly the PIF-001 grammar's: ASCII only, one space between fields, byte-exact brackets, nothing after `end <count>` and its LF.
- Gate-fed inputs in `GateInputs`: expected import chain, allowed features, withdrawn logic versions, file class (`Library`, `Reviewed`, `Claim`, default `Claim`), the axiom registry predicate, goal constants, prior claim ids and `PifLimits`.
- `ReadReport` lists every accepted declaration in order with its kind read back from the environment (never from the record), plus the goals. `read` takes the environment by value and returns it only on success, so a half-read file cannot leak.
- Fail-closed `Unsupported` or `FeatureNotDeclared` for everything the kernel cannot do yet: literals (`enat`, `estr`, `lits`), `quot`, `prim` roles, mutual and nested blocks, `eproj`, and any declared feature.
- Tests: the VER32 4.6 worked example is read into a fresh environment and the kernel checks that `Pif.and` is commutative end to end; one negative test per rule; kind fidelity (theorem stays a `Theorem`); unsound-acceptance attempts (a theorem that does not prove its type, a goal on an axiom, `axiom propext : False` in a claim file, duplicate claim ids, forward references, index reuse, truncation); hostile input (every section 7 limit, a million-deep chain, doubling expression and level DAGs, every truncation of the example, thousands of deterministic mutations) on a large-stack thread.

### Changed

- `tests/ceiling.rs` gains the "export reader" row (600 lines) and lists `pif.rs`.

### Deferred

- R-PRM-1 to 4, `prim`, literals, `quot`, mutual/nested blocks and projections (slices 3b and 4); R-IMP-2 (SHA-256 and the resolver, outside the reader's 600 lines); the binding-CTE digest of axiom statements (the gate supplies the registry predicate).
