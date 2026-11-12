# Changelog

## Unreleased

- **`verify_evidence` step 6b closes the converse forgery**: a CWE-208 row that cites
  the attestation `warrant-spec::all-operands-public` is now rejected
  (`EvidenceMismatch`) unless the report also carries the not-applicable exemption
  `EXEMPT-CWE-208-NO-SECRET-OPERANDS`. Before, the facts were re-derived only when
  that exemption id was present, so the attestation could be cited under another
  exemption id (e.g. `EXEMPT-CWE-208-GATE-LEVEL`) and pass. Test:
  `the_attestation_cannot_be_cited_under_a_different_exemption_id` (written first,
  failed before the fix). Security-review finding, low severity. `effect_certificate.json`
  re-emitted for the new source digest (no other change).
- **CWE-208 "not applicable" for a document with no secret operand** (first adopter
  `rust/wasm-leb128`; VER04h section 9 item 20). The matrix knew a live constant-time
  proof (gate-level exemption) and a `not-live` one ("applicable and unaddressed");
  a document labelling every operand `public` has neither and got an open row. New
  derived attestation `warrant-spec::all-operands-public` (`Evidence::attestation_ids`,
  reserved in `ATTESTATION_IDS`), present only while the spec on disk declares an
  operation and every parameter of every operation has a `labels` row and none is
  `secret` (`OpFacts::labelled_public`; an unlabelled parameter does not count as
  public). With it and no constant-time entry at all, CWE-208 is the unapproved
  exemption `EXEMPT-CWE-208-NO-SECRET-OPERANDS` (kind `not-applicable`, empty derived
  assumption list). An exemption, not an exclusion: the registry has no safe subset
  for CWE-208. It is a separate branch keyed on a new `AuditEvidence::
  no_secret_operand_ids` field whose default (empty) keeps the row open, so nothing
  that was fail-closed became permissive. `verify_evidence` refuses the exemption
  id unless both facts hold (`EvidenceMismatch`), and citing the attestation where
  it does not hold is `UnknownEvidence`. Tests: `tests/test_no_secret_operands.rs`
  (matrix branches, default fail-closed, forged gf256 reports).
- **`AuditProfile::VarintCodec` (`--profile varint-codec`)**: true CWE-326/327/407
  sentences for a LEB128 codec instead of the GF(2^8) or comparison ones.
- The ct-compare and gf256 reports regenerate byte-identically (`--generate` with
  their recorded `--commit` and profile, then `git diff`: no change).
- `effect_certificate.json` regenerated for the new source digest.

- Regenerate `effect_certificate.json` with the hardened analyzer 0.3.0: both entry points (`generate_ct_compare_report`, `generate_package_report`) now carry the declared effect set (the analyzer requires entry-point effects to equal `declared_effects`).

## 0.2.0 (2026-11-10)

VER13 AS-001a part 2: reports cite the assumption catalogue, pin it, and
`--check` enforces the report-side assumption rules. The report format changes
(breaking), hence the minor version bump.

- New module `catalogue`: loads `code/assumptions/catalogue-v1.json`,
  `evidence-kinds.json` and `non-claims.json`, found by walking up from the package
  directory. Fails closed on a missing, oversized (1 MiB / 256 KiB / 64 KiB, checked
  while reading), non-UTF-8 or malformed file: unknown or missing fields, duplicate
  keys, out-of-vocabulary values, dangling or cyclic `depends_on`/`refines`,
  inconsistent `refines`/`refined_by`, the catalogue-side AC-3/AC-8/AC-11/AC-15
  shape rules, evidence kinds named but not defined (or defined but unused), and
  evidence-id patterns that overlap or leave their manifest's namespace.
- New module `assumptions`: derives each claim's assumption list (every entry whose
  `applies_to` matches the label, claim kinds and package, closed over
  `depends_on`/`refines`/`refined_by`), the new report-level `assumptions` list, and
  each exemption's list (VER13 section 3.9); and checks AC-1..AC-14 with the spec's
  diagnostic names (`UnknownAssumption`, `EmptyAssumptions`, `UnevidencedDischarge`,
  `SilentlyDroppedAssumption`, `LabelOutranksAssumptions`, `PartialUmbrella`,
  `StackMismatch`, `FloorDischarged`, `PinMismatch`, `UnusedAssumption`,
  `StaleAttestation`, `NonClaimReported`, `ExemptionAssumptionMismatch`,
  `CatalogueDrift`). `verify_evidence` runs them before the field comparison
  (`EvidenceCheckError::Assumption`).
- Report format: new `catalogue` pin object and top-level `assumptions`; each proof
  gains `stack` (`host-rustc`), `rung` (`R0`), `discharged` and `remaining_by_layer`;
  each exemption gains `assumptions`. All are parsed strictly (absent or mistyped is
  an error, not a default). The legacy strings `compiler.preserves_timing`,
  `memory.unaliased`, `heap-bound:unchecked`, `depth-bound:unchecked` are now seeds
  resolved through the catalogue (`wir.bound.*-unchecked`).
- `Evidence` gains `package_id`, `catalogue`, `manifest_sha256` and `pinned` (an
  optional per-entry build tuple, rule AC-9). `verify_evidence` checks that the
  report is for the evidence's package; `generate_package_report` refuses a mismatch.
- The `pass`-verdict rule against unchecked rows also matches `*-unchecked` ids.
- CLI: `--strict` turns the AC-10 unused-assumption warning into a failure; unused
  entries are printed as `warning: AC-10 UnusedAssumption: <id>`.
- Tests: `tests/test_assumptions.rs` (a falsification test per rule, positive
  controls for a backed discharge and a pinned per-run discharge, loader negatives,
  strict report parsing, the CLI `--strict` path). Test sandboxes copy
  `code/assumptions/`.
- `effect_certificate.json` regenerated.
- Security review (LOW findings): the catalogue is loaded from the repository root at
  which the package's Warrant document was found, which must be a strict ancestor of
  the package directory (`CatalogueSet::load_for_package`); `CatalogueSet::locate`
  refuses a catalogue at or inside the package. Evidence-kind patterns are capped at
  4096 in total and the overlap check compares only within (length, namespace)
  buckets. Tests: `ac14_a_package_cannot_shadow_the_repository_catalogue`,
  `ac14_the_evidence_kind_pattern_count_is_capped`.

## Unreleased (before 0.2.0)

- Security fixes (review of the gf256 / package-agnostic branch):
  - M1: the CWE-400 and CWE-208 rows are derived from *every* steps-bound / constant-time obligation, not only the live ones. A manifest in which some operations pass and others are `not-live` or `unchecked` can no longer discharge CWE-400 or cite WIR proofs for CWE-208 on the strength of the passing operations (`AuditEvidence::{step,ct}_unproven`; `weakest()` and the CWE-125/787 bounds row consider every obligation; `unchecked` blocks as it did before the generalisation). `--check` re-derives the same rule (`verify_evidence` step 6): a report whose CWE-400 row is `discharged`, or whose CWE-400 / CWE-208 row cites a proof id of that kind, while any obligation of the kind is not a ranked pass, is an `EvidenceMismatch`.
  - L1: a `not-live` proof entry is an opt-in. It must carry no certificates, the label `not-live` and a non-empty `note`, and its operation must be declared `proofs_not_live=<reason>` in the package's `evidence.bindings`; `Evidence::load` (hence `--check` and `--generate`) rejects it otherwise.
  - L5: `--generate` reads `Cargo.toml` through `read_capped` and takes the version from the `[package]` table only (`evidence::package_version`); a `version` in `[dependencies.*]` or `[workspace.package]` is no longer picked up.
  - `effect_certificate.json` regenerated. Tests: `tests/test_mixed_proofs.rs`.
- Integration of package-agnostic generation with the hardened checker: E5 is `pass` only when every operation has a ranked-label pass (the hardened per-operation rule), `not-live` only when the remaining obligations are recorded `not-live`, and `fail` otherwise; an absent optional bench manifest has no source binding to go stale while a present one is still checked; `effect_certificate.json` regenerated for the new `generate_package_report` entry point.
- Round-2 security fixes:
  - N1: `has_unsafe_token` is a stricter lexer (char/byte literals are exactly `'` + escape-or-char + `'`, lifetimes/labels distinguished, `b""`/`c""`/raw strings with any `#` count, nested comments) and fails closed: an untokenisable file counts as containing `unsafe`.
  - N2: `source_sha256` binds every `src/**/*.rs` (shared `test_bootstrap::manifest` framing); editing any source file makes `--check` fail as stale.
  - N3: manifest counts must be integers in `0..=2^53` (no clamping) and per-operation totals use checked addition.
  - `--allow-unverified` prints each accepted id and states the flag is temporary (WD-008).
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
