# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-039b-2b, VOS-D104, VOS-D055 bound half, VOS-D112)

- **CC09**, `bound` rows against the ceilings of `contract.vos` (`src/bounds.rs`), run by `check-component` on
  every implementation.  The operation-to-ceiling mapping is DERIVED from the parsed document (X itself when the
  contract has a ceiling for X, else X with ONE `_out` or `_img` suffix stripped, else ghost-only and listed in a
  note), never declared: the `realises=` field of VOS02 4.7 is not built (the bindings reader rejects it and an
  author-declared map could point `insert_img` at the loosest ceiling).  Fails on a non-literal bound, a bound above
  its ceiling, a missing `steps` or `heap` row of a mapped operation (production and `any`), and a ceiling
  operation that no document operation maps to.  `depth` is compared when present, a note when absent;
  `reference` rows are optional.  `MAP_SUFFIXES` is asserted against VOS04 by a test.
- **CC10**, the dependencies and build of a contract crate (`check-component`, kind `contract`): no entry in
  `[dependencies]`, `[build-dependencies]` or `[target.*.dependencies]` (the underscore alias too), no `build` key or
  `build.rs`, no `[lib] proc-macro`; `[dev-dependencies]` allowed.  Closes the dependency half of VOS-D112 (a path
  dependency supplying an attribute macro or derive).  An unreadable `Cargo.toml` is no verdict.
- **`check-publish [--root R]`** (`src/publish.rs`), the VOS-D104 sweep: the direct `vos-*` children with a
  `Cargo.toml` of `code/packages/rust` and `code/programs/rust` must say `publish = false`, except kinds
  `production`, `reference` and `contract` (exempt with a note: the publish policy is undecided).  `PB01` missing or
  not `false`, `PB02` unreadable TOML, a symlinked crate directory or `Cargo.toml` is no verdict.  Runs in this
  crate's BUILD and BUILD_windows.
- `publish = false` in this crate's `Cargo.toml`.
- Tests: `bounds` (7), `publish` (18), CC09 controls (9) in `component`, CC10 controls (5) in `contract`; the
  component fixtures name their contract and carry `op` and `bound` rows.

### Fixed (security review of VOS-039b-2b)

- `check-publish` printed author-controlled `vos-*` directory names raw, so a directory named with a newline or an
  ANSI escape could forge report lines; every name in PB01, PB02 and the `exempt` note now goes through `esc()`.
  Test (unix only: Windows forbids such names).
- The kind exemption was the author's word alone (a tooling crate escaped PB01 by adding a one-line `vos.toml`).  A
  kind-exempt crate is now exempt only when `repository_inputs.go` registers it (`vosImplementationRow` or
  `vosContractRow`, read by CC05's own scanner); otherwise the new code **PB03**.  A missing or unreadable row file
  is no verdict (read only when a crate claims an exempt kind).
- CC10 stands alone: it also refuses a top-level `[patch]`, `[replace]` or `[workspace]` and a `[lib] crate-type`
  naming `proc-macro` (SH15 caught these too).

### Fixed (security review of VOS-039b-2a)

- `check_enums`: the constructor-written-with-braces lookup is a precomputed map (enum name to constructor set), not
  a scan of every enum per constructor.

### Added (VOS-039b-2a, VOS-D077)

- **CK10**, the Rust-enum/Warrant-variant correspondence of `check-contract` (`src/enums.rs`).  Every `enum` of the
  contract crate's `src/**/*.rs` (read with `vos-shape-check`'s hardened `loader::load_sources` and its token-tree
  scan, never a new lexer) and the Warrant `variant` of the same name are each rendered to one
  `variant <Name> = <C1> | <C2>(<T>, ..)` line by one function and the bytes are compared; both lines are printed on
  a mismatch.  Type map `u8` to `U8`, `bool` to `Bool`, `[u8; N]` to `Bytes<N>`, a same-crate name to itself,
  anything else `?`-prefixed (never equal).  A tuple variant renders types only (Warrant names its fields freely);
  a named-field variant renders its field names too.  Direction Rust to Warrant: a model-only Warrant variant is
  allowed; a rename on either side leaves a Rust enum without a twin.
- `warrant/lemmas.table` is `vos-lemmas 2`: the Rust lines are published as `variant ...` rows after the `config`
  rows, so CK09 byte-pins them.  The tables of `vos-sched-contract` and the fixture were regenerated with
  `gen-contract`.
- Tests: reordered variants, a renamed constructor, a changed payload arity, type and array width, an unmapped and an
  alias type, a renamed enum on either side, a renamed named field, a free tuple-field name (not a finding), a stale
  variant row (CK09), an unlexable source and an unreadable enum (fail closed), no `src/`, a symlink under `src/`
  (no verdict), every Warrant type form, and the real contract's two rows.
- The `check-component` note for a contract crate no longer lists the command-type rule as unchecked (SH19 runs in the
  contract crate's BUILD).

### Fixed (VOS-039b security hardening)

- `check-contract` / `gen-contract`: a symlinked crate directory, `warrant/` or `warrant/mutants/` is a `CK01`
  finding before anything is read or written; the CK05 mutant-stem message is escaped; CK02/CK05/CK06 findings
  return before any evaluation is scheduled; at most 64 lemmas (`MAX_LEMMAS`, `CK03`).

### Added (VOS-039b-1, VOS02 D-GD11)

- `vos-link gen-contract` and `check-contract <contract-crate-dir>` (VOS04 5.6, `src/contract_check.rs`): the
  generic contract check that retires `vos-sched-contract-check`.  Every lemma of `warrant/lemmas.part` is
  evaluated over the whole domain of its binders (Bool, U8; budget 400,000), each against the one-`model`
  mutant `warrant/mutants/<lemma>.part` replaced by parsed span, each `config` key through a generated
  `cfg_<KEY>()` model with tool-made V-1 and V+1 mutants, and `warrant/lemmas.table` is generated and
  byte-compared.  Codes `CK01` to `CK09`.  Workers on `WARRANT_STACK_BYTES` threads, at most 8, results
  collected by index (deterministic).
- `gen-build` and `check-component` accept kind `contract`: `canonical_contract_build` (identical BUILD and
  BUILD_windows), CC02 file set, CC03 byte equality, CC05 `vosContractRow`.  `row_in_consumers_with` takes the
  row function name.
- `inputs::load_contract_package`: the contract half of `load` (kind, `contract.vos`, tier files, part
  admission), shared with `check-contract`; `check-contract` maps GD01, GD02, GD05 to CK01.
- `tests/contract.rs` (37 tests) on a toy N = 2 contract; two CLI tests; `Repo::new` copies `warrant/`
  recursively (the real contract now has `warrant/mutants/`).  Line coverage of `src/contract_check.rs`
  96.5% (tarpaulin, LLVM engine), 97.4% of `src/`.

### Changed (VOS-039b-1)

- `check-component` no longer refuses kind `contract` (it is judged by the contract branch); the message of
  `CC01` now reads "is not a component kind".  A crate of another kind (`spine`, ...) is still refused.

### Added (VOS-039a, VOS-D070, VOS-D082, VOS-D099)

- `vos-link gen-build <crate-dir>`: writes the canonical region of `BUILD` and `BUILD_windows` (VOS04 5.2
  lines 1 to 9 with every sabotage line on the shared `warrant_negctl.py`, the Linux tarpaulin gate, the
  `check-component` line, then a marker) and `warrant/check_evidence.sh` from `excused-controls`.  Author
  lines below the marker are kept and count for nothing.
- `vos-link check-component <crate-dir>` (VOS04 5.6): CC01 kind, CC02 file set, CC03 BUILD files byte-equal
  to `gen-build`, CC04 inputs of generation (bindings, document, `warrant/sabotage.plan`, tolerance script),
  CC05 build-tool row (Go comments and raw strings removed, only inside the consumers slice), CC06
  `publish = false` for kind `any`, CC07 `coverage-floor`, CC08 the manifest bound to the document's claims
  (counts, passing controls per tier, whole-id excuses).  Generated-then-compared replaces the first draft's
  text lint, which single-line tricks bypassed.
- Review round 2: (M1) every value pasted into BUILD is validated in one place, `interpolation_problems`, and an evidence feature must be `[a-z][a-z0-9_-]*` and on the central list; (M2) the manifest's `spec_sha256` and `source_sha256` must equal the document's and `src/`'s digests, and the generated region removes the manifest before the run; (M3) every claim that calls a bound operation needs a plan entry (fp-bitmap gained `dispatch_safe`); (M4) each `excused-controls` entry carries a `VOS-Dnnn` reason that must be the first cell of exactly one backlog row whose status begins `open` or `queued` (an allowlist).
- Dependencies: `bounded-json` (the manifest) and `warrant-grammar` (the bindings reader, the generator's own).
- `tests/component.rs` (74 tests): fixture generated by `gen-build`, then one control per rule and one
  `bypass_*` per trick of the review, each with its own code and text.

### Added (VOS-012b0, VOS-D061)

- The `representation` tier (VOS02 2.7), the contract's fifth: a production crate carries **exactly
  one** of `canonical` and `representation` (by `claim_sha256`; both, or neither, is a CD07 finding)
  and no `any`; an `any` crate carries neither.  The cone rule is skipped for a crate that carries
  only the representation tier (its history register is outside the cone by design).  The report
  header prints `mode representation`, **derived from `carries`** (there is no `mode` key), and such
  a crate gets the representation note in place of the R6 cone note.
- `tests/representation.rs` (16 tests): the positive fixture `vos-fixture-rep` and its golden
  document, both/neither/representation-plus-`any`/`-any`-carrying-representation controls, a missing
  or mistyped `rep` or `hist` (CD04), a tampered or weakened representation claim (CD02, CD03, CD05,
  CD06, CD08), and `mode` as an unknown key.  `a_contract_without_the_needed_tier...` also drops the
  representation tier.

### Changed (VOS-D081)

- The golden `tests/fixtures/expected/vos-fixture-bitmap.warrant` was re-blessed: the only difference is
  the safety block (the new `safety_init` claim) and its digest.

### Changed (VOS-011 PR, VOS-D064)

- `warrant/lemmas.part` of a contract package is admitted when present (a forged marker is a
  `GD01`) and **never embedded**; `check-doc` does not require it (VOS02 R1.4).  The golden
  document was re-blessed: its contract-block digest changed, no obligation digest did.  New
  test `lemmas_part_is_never_embedded_and_never_required`.

### Fixed (security review round 1)

- Untrusted strings (file names, `vos.toml` values, document headers, argv) are quoted and escaped in every report.
- Unique temporary file names (pid and counter); `.gitignore` covers `*.vos-link.tmp`.
- Windows: the verbatim `\\?\` prefix of a canonical path is dropped and the comparison is case-insensitive there.
- Every pass prints the R6 structural-lint note; the cone rule is a taint walk (vos-manifest).
- Tests: strict `VOS_LINK_BLESS=1` (refused on CI), trailer variants, extra carried tier, `--root` disagreement, escapes, FIFO timeout guard, SKIPPED notice when root.
- Dev-dependency on `vos-sched-contract` (build-tool selection).

### Added

- VOS-033: `vos-link gen-doc` and `vos-link check-doc` (VOS02 sections 2, 6.8, 10, 10.1).
  - `gen-doc` assembles `code/specs/<impl>.warrant` from the contract package's parts and the
    implementation's `impl.part`, verifies it, and writes it (fresh temporary file plus rename)
    only when there is no finding.
  - `check-doc` verifies the document on disk (CD01-CD07) and compares it byte for byte with a
    fresh assembly (CD08).
  - Input checks GD01-GD05 (parts, `contract.vos`, tier files against `contract.vos`,
    claim-only obligations with unique names, `vos.toml` keys `contract` and `obligations`).
  - R6 as far as a document goes: kind rules, the name lint, and the structural cone rule on
    `build`.
  - Path containment, bounded reads with `O_NOFOLLOW`, exit codes 0, 1 and 2.
- Four fixture implementations and a golden document for the real `vos-sched-contract`.
- 71 tests; line coverage above 95%.
- `vos.toml` (class H), `required_capabilities.json`, BUILD and BUILD_windows.
