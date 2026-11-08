# coding_adventures_ca_assurance_report

Assurance report generator and weakness registry evaluator for the verified shipping program ([VER01](../../../specs/VER01-assurance-report.md), [VER15](../../../specs/VER15-weakness-registry.md), [VER26 Step 28](../../../specs/VER26-ct-compare-pilot.md)).

## Features

- **Weakness Registry Evaluation:** Exhaustively evaluates package sources against `weakness-registry-v1.json` (24 CWE classes).
- **Weakness Matrix Closure:** Validates that each row is closed as `excluded`, `discharged`, or `exemption`. Prohibits open rows.
- **Computed Exemptions:** Automatically derives unapproved exemptions (e.g. CWE-208 timing gate-level floor) when proof floors are not reached.
- **Rung Assessment:** Honors non-live rungs (e.g. E2 simulated host) by reporting them as `"not-live"`, never `"pass"`.
- **Tamper-Evident Verification:** Standalone `--check` command validates report integrity fail-closed.
- **Zero Third-Party Dependencies:** Implemented exclusively with standard library Rust (`std`).

## Usage

```bash
# Generate assurance report for a package
cargo run -p coding_adventures_ca_assurance_report -- --package-dir ../ct-compare --generate

# Verify an existing assurance report
cargo run -p coding_adventures_ca_assurance_report -- --check ../ct-compare/assurance_report.json
```
