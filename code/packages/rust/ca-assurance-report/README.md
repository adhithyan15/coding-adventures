# coding_adventures_ca_assurance_report

Assurance report generator and weakness registry evaluator for the verified shipping program ([VER01](../../../specs/VER01-assurance-report.md), [VER15](../../../specs/VER15-weakness-registry.md), [VER26 Step 28](../../../specs/VER26-ct-compare-pilot.md)).

## Features

- **Weakness Registry Evaluation:** Exhaustively evaluates package sources against `weakness-registry-v1.json` (24 CWE classes).
- **Weakness Matrix Closure:** Validates that each row is closed as `excluded`, `discharged`, or `exemption`. Prohibits open rows.
- **Computed Exemptions:** Automatically derives unapproved exemptions (e.g. CWE-208 timing gate-level floor) when proof floors are not reached.
- **CWE-208 for packages with no secret:** when the Warrant document labels every operand of every operation `public` and the proof manifest holds no constant-time obligation, CWE-208 is recorded as "not applicable: no secret operands" (`EXEMPT-CWE-208-NO-SECRET-OPERANDS`, backed by the derived attestation `warrant-spec::all-operands-public`) instead of an open row. Any secret label keeps the old fail-closed rows, and `--check` re-derives both facts from the spec and the proof manifest ([VER04h](../../../specs/VER04h-warrant-derived-evidence.md) section 9, item 20).
- **Audit profiles (`--profile`):** `comparison` (ct-compare), `field-arithmetic` (gf256), `varint-codec` (wasm-leb128), `boolean-logic` (logic-gates) and `bit-arithmetic` (arithmetic) choose the true sentence the static CWE-326/327/407 rows carry.
- **Rung Assessment:** Honors non-live rungs (e.g. E2 simulated host) by reporting them as `"not-live"`, never `"pass"`.
- **Tamper-Evident Verification:** Standalone `--check` command validates report integrity fail-closed.
- **Evidence-derived claims (VER04h):** every number, label, certificate digest and context flag is read from `evidence/*-manifest.json` written by the runs that produced it; `--check` cross-checks the report against them and fails closed on unknown ids, count/label drift, bad certificate hashes, uncovered Warrant claims and stale manifests.
- **VER13 assumption catalogue (0.2.0):** every assumption a report states is an id of
  [`code/assumptions/catalogue-v1.json`](../../../assumptions/catalogue-v1.json). The report pins the
  catalogue, `evidence-kinds.json` and `non-claims.json` by SHA-256 in its `catalogue` object, and `--check`
  enforces the [VER13 section 6](../../../specs/VER13-assumption-catalogue.md) rules AC-1..AC-14 (see below).
- **Zero Third-Party Dependencies:** Implemented exclusively with standard library Rust (`std`).

## How assumptions are derived and checked

A proof label is an assume-guarantee statement, so a label is only as honest as
its assumption list. The generator writes no assumption literal: for each claim
it takes every catalogue entry whose `applies_to` matches the claim's label,
claim kinds (functional / constant-time / bound, read from the evidence) and
package, and closes the set over `depends_on`, `refines` and `refined_by`.
Entries about the weakness matrix, capability evidence and verdict go in the
report-level `assumptions` list, and each computed exemption lists the ids it
derives from (VER13 section 3.9). For ct-compare that turns the four legacy
strings into 53 ids per claim: the list grows before it shrinks.

`--check` re-derives all of it and fails closed with a stable diagnostic:

| Rule | Diagnostic | Fails when |
|---|---|---|
| AC-14 | `CatalogueDrift` / `MalformedCatalogue` | the pin differs from the files on disk / a file is missing, oversized or malformed |
| AC-1 | `UnknownAssumption` | an id is not a catalogue id (an alias is not an id), or is listed twice |
| AC-11 | `StaleAttestation` | an `unverified::` alias whose entry is no longer `attested` |
| AC-2 | `EmptyAssumptions` | a proved-family label lists nothing |
| AC-7 | `StackMismatch` | a discharge is for another stack, or the stack is unknown |
| AC-8 | `FloorDischarged` | a floor / declared / physical entry is discharged |
| AC-6 | `PartialUmbrella` | an umbrella is discharged while a refinement is not |
| AC-3 | `UnevidencedDischarge` | a discharge lacks passing evidence of a kind the entry accepts |
| AC-9 | `PinMismatch` | per-run evidence without the same pinned build tuple |
| AC-4 | `SilentlyDroppedAssumption` | an applicable or reachable entry is not listed |
| AC-5 | `LabelOutranksAssumptions` | a `proved-to-L` label with undischarged entries above `L` |
| AC-12 | `NonClaimReported` | a declared non-claim (`non-claims.json`) is shown as proved |
| AC-13 | `ExemptionAssumptionMismatch` | an exemption's list differs from its derivation |
| AC-10 | `UnusedAssumption` | (warning; error with `--strict`) an entry naming the package matches nothing |

The catalogue directory is found by walking up from the package directory. Inputs
are size-capped while reading (catalogue 1 MiB, evidence kinds 256 KiB,
non-claims 64 KiB). Every rule has a falsification test in
`tests/test_assumptions.rs`.

## Usage

```bash
# Generate assurance report for a package
cargo run -p coding_adventures_ca_assurance_report -- --package-dir ../ct-compare --generate

# (needs <package>/evidence/{test,proof,bench}-manifest.json from the package BUILD)

# Verify an existing assurance report (also cross-checks the evidence manifests
# and runs the VER13 assumption rules against code/assumptions/)
cargo run -p coding_adventures_ca_assurance_report -- --check ../ct-compare/assurance_report.json --allow-unverified

# ...and fail on unused catalogue entries (AC-10) instead of warning
cargo run -p coding_adventures_ca_assurance_report -- --check ../ct-compare/assurance_report.json --allow-unverified --strict
```

After any edit to `code/assumptions/*.json`, regenerate every report with
`--generate` (keeping its `--commit` and `--profile`); `--check` and the CI step
`python3 code/scripts/assumption_catalogue.py` fail on a stale pin.
