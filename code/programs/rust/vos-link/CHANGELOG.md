# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-012b0, VOS-D061)

- Representation mode: `vos.toml` `mode = "representation"` (`Loaded.mode`; `GD05` on kind any, on a
  contract package, on an unknown value, on `canonical` under it), CD07-R in `check-doc` and
  `gen-doc` (VOS02 R6 rule 2), `mode representation` in the report header, the CD07-R lint note
  in place of the R6 cone note, and `impl-own rep` lines carrying each representation claim's
  `claim_sha256` (`Outcome.rep_claims`).
- `tests/representation.rs` (31 tests; negative controls for each CD07-R finding and each `GD05`),
  the fixture `vos-fixture-rep` and its golden document.
- CD07-R hardened after the security review (17 more tests): constant or repeated `rep` arguments,
  binder refinements (walked, and refused), `_det` as a conjunction of `==` equations with op-free
  right sides, required claims checked for content (call their op on `rep(..)`; `rep_init`,
  `rep_alpha`), real-image state chains (no nullary or model-named `_img`), every `Impl`-typed
  argument checked, and the `rep` parameter-set coverage approximation.
- Existing behaviour is unchanged: canonical crates print no mode and the same note.

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
