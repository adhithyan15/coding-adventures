# Assumption catalogue

`catalogue-v1.json` is the machine-readable form of the assumption inventory in
[VER13 section 3](../specs/VER13-assumption-catalogue.md): every assumption a
proof label in this repository relies on, with a stable id, layer, kind, status,
dependants and the evidence that would discharge it. Reports pin the file by
SHA-256 and cite assumptions by id.

- Schema: `code/specs/schemas/ver13-assumption-catalogue-v1.schema.json`.
- Generated from the spec tables: `python3 code/scripts/generate_assumption_catalogue.py`
  (`--check` fails when the committed JSON is stale). Do not edit the JSON by hand;
  change the spec table or the curated overlay in the generator and regenerate.
- Checked by `python3 code/scripts/assumption_catalogue.py`, which also prints the
  digest a report pins. Run in CI under "Verify repo-wide metadata contracts".
- `catalogue-v1.json` changes need a `CHANGELOG.d/` shard.

Two companion files:

- `evidence-kinds.json` (schema `code/specs/schemas/ver13-evidence-kinds-v1.schema.json`):
  one row per evidence kind a catalogue entry names in `discharge.evidence_kinds`,
  with the manifest that can hold it (`test`, `proof`, `bench`, or `planned` when no
  manifest produces it yet), its evidence-id patterns, and whether it is per-run
  (pinned to one build, rule AC-9). Every named kind is defined, every defined kind
  is named, and no two patterns can match the same id. Curated by hand.
- `non-claims.json` (schema `code/specs/schemas/ver13-non-claims-v1.schema.json`):
  properties a package declares it does *not* provide (rule AC-12), e.g. gf256 is not
  constant time (MA01).

Every `assurance_report.json` pins the SHA-256 of all three files in its `catalogue`
object. After editing any of them, regenerate the reports with
`ca-assurance-report --generate`; `--check` and the CI step above fail on a stale pin
(AC-14). The report-side rules AC-1..AC-13 are enforced by `ca-assurance-report
--check` (`code/packages/rust/ca-assurance-report`).
