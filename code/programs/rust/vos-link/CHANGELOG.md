# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-039a, VOS-D070, VOS-D082)

- `vos-link check-component <crate-dir>` (VOS04 section 5.6): the file set (CC02), the BUILD lines of
  section 5.2 in `BUILD` and `BUILD_windows` with a failure on a missing line (CC03) and on a missing guard
  (CC04: bail-out-aware sabotage lines, `check-component` after the generated suite), the build-tool row
  (CC05), `publish = false` for kind `any` (CC06), the coverage floor of `vos.toml` and the Linux tarpaulin
  line that enforces it (CC07), and the evidence manifest's "none ran = fail" conditions (CC08).
  New dependency: the repository's `bounded-json` (the manifest is read by it, not by a hand parser).
- `tests/component.rs` (43 tests): a passing production, reference and `-any` fixture and a negative control
  per rule, each with its own code and text.

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
