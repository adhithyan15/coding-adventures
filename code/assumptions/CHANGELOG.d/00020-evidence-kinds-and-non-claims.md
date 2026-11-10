## 0.2.0

### Added
- `evidence-kinds.json`: the 82 evidence kinds the catalogue names, each with the
  manifest that can hold it (18 in today's `test`/`proof`/`bench` manifests, 64
  `planned`), disjoint evidence-id patterns and a `per_run` flag (AC-9). Schema
  `code/specs/schemas/ver13-evidence-kinds-v1.schema.json`.
- `non-claims.json`: declared non-claims for rule AC-12; first row gf256 CWE-208
  (MA01 "Constant-time status"). Schema `code/specs/schemas/ver13-non-claims-v1.schema.json`.
- `assumption_catalogue.py` validates both files and checks that every committed
  `assurance_report.json` pins the current digests of all three files.

### Unchanged
- `catalogue-v1.json` is byte-identical; its digest is unchanged.
