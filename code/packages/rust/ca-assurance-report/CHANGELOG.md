# Changelog

## Unreleased

- Security review fixes (VER04h section 9, checks 8-16):
  - B1: duplicate claim ids are rejected, the claim count must equal the derived count, and every claim is compared by exact id.
  - B2: `--check` validates `exhaustive.status`, requires a ranked `proof.label`, and rejects unchecked assumptions under a `pass` verdict; rungs E3/E4 are judged per operation and E5 requires ranked labels.
  - B3: the effect certificate is parsed strictly; constructs naming `unsafe`/`ffi` (any case) are unsafe-bearing; `src/**/*.rs` is scanned for `unsafe` and a certificate without a matching exception fails.
  - B4 (partial): passing obligations must cite a certificate and carry `obligation_sha256`; claims carry `proof.certificate_kind` (`trivial-miter` / `miter`) and `--check` requires the mark. Recomputing the digest from source is WD-009.
  - B5: test and bench manifests carry and are checked against `source_sha256`.
  - B6: the spec path must be `code/specs/<package>.warrant` (relative, no `..`), identical in both manifests, and every manifest must name the spec's module.
  - B7: duplicate ids (within or across manifests), ids colliding with attestation ids or `unverified::`, and duplicate JSON keys are rejected.
  - B8: the no-unsafe contradiction check uses structured fields instead of the `reason` text.
  - B8: `--check` fails on `unverified::` ids unless `--allow-unverified` is given.
  - B9: manifests, sources and reports are capped at 16 MiB, certificate files at 64 MiB.
  - New error variants `EvidenceCheckError::{DuplicateId, TrivialCertificateUnmarked}`; `ProofEvidence` gains `certificate_kind`.

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
