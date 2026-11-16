# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

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
