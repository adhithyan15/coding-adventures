# Changelog

## Unreleased

- Package-agnostic generation (second package: `rust/gf256`). New
  `generate_package_report(package, version, commit, registry, evidence,
  profile)`; `generate_ct_compare_report` is now a thin wrapper and ct-compare's
  checked-in report is unchanged. `--generate` takes the package id and version
  from the package directory (`rust/<dir>`, `Cargo.toml`) and a `--profile
  comparison|field-arithmetic` that selects which true sentence the three static
  attestation rows (CWE-326/327/407) carry.
- Proof obligations may be recorded `not-live` (attempted, refused by the
  tooling, with the blocker in `note`): such a claim has no proof block and cites
  no proof id, rung E5 is `not-live` rather than `fail`, and CWE-208 / CWE-400
  become *unapproved exemptions that quote the recorded blocker* instead of an
  open row. Erasing the entries is not an acknowledgement: a spec that declares
  `secret` labels or `steps` bounds without an entry still fails closed
  (`UncoveredClaim`), and a missing proof manifest still fails to load. The bench
  manifest is optional (no manifest, no benchmark evidence to cite).
- Tests: `tests/test_package_report.rs` (gf256 report derives, verifies and
  matches the checked-in file; not-live handling; stale source, count drift and
  forged-promotion falsification; ct-compare reproduced exactly).

- VER04h / VER-P030: the report is derived from evidence manifests, never written by hand.
  - `generate_ct_compare_report` now takes an `Evidence` (loaded from `evidence/{test,proof,bench}-manifest.json`, the effect certificate and the Warrant document) and contains no evidence literal: case counts, fuzz iterations, proof labels, certificate digests, rungs and every `PackageAuditContext` flag are computed.
  - New `evidence` module with `Evidence::load`, `derive_claims` and `verify_evidence`; `--check` now also cross-checks the report against the manifests and fails closed on unknown evidence ids, failing cited evidence, count/label/digest drift, certificate files that are missing, mis-hashed or rejected by the DRAT checker, Warrant claims/examples/obligations with no evidence, stale manifests (spec or source hash), and a no-unsafe claim next to a declared `unsafe_code` exception.
  - Report claims gain `evidence` (manifest ids) and `exhaustive` fields.
  - AF-22: memory-safety CWE rows are no longer `excluded` on a false "no unsafe" basis when the certificate declares an unsafe exception: CWE-125/787 are discharged from bounds-checked proofs (bounded), CWE-416/415/476/190 become unapproved exemptions.
  - CWE-400 and CWE-208 rows cite real manifest ids; rows resting on static claims no tool checks carry `unverified::` ids and a reason suffix.
  - Falsification tests for every failure mode (`tests/test_evidence.rs`).

## 0.1.0 (2026-11-08)
- Initial release of `coding_adventures_ca_assurance_report` (VER15 / VER01 / VER-P028).
- Implementation of weakness registry parsing (`weakness-registry-v1.json`).
- Per-package weakness matrix construction and deterministic closure validation.
- Evaluation of publication rungs (E0-E5) with non-live rung isolation.
- Mechanical computation of unapproved exemptions (CWE-208 gate-level floor).
- Machine-readable assurance report generation and standalone verification (`--check`).
- Zero third-party dependencies (`#![deny(unsafe_code)]`).
