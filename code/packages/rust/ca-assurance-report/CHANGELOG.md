# Changelog

## 0.1.0 (2026-11-08)
- Initial release of `coding_adventures_ca_assurance_report` (VER15 / VER01 / VER-P028).
- Implementation of weakness registry parsing (`weakness-registry-v1.json`).
- Per-package weakness matrix construction and deterministic closure validation.
- Evaluation of publication rungs (E0-E5) with non-live rung isolation.
- Mechanical computation of unapproved exemptions (CWE-208 gate-level floor).
- Machine-readable assurance report generation and standalone verification (`--check`).
- Zero third-party dependencies (`#![deny(unsafe_code)]`).
