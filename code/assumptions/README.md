# Assumption catalogue

`catalogue-v1.json` is the machine-readable form of the assumption inventory in
[VER13 section 3](../specs/VER13-assumption-catalogue.md): every assumption a
proof label in this repository relies on, with a stable id, layer, kind, status,
dependants and the evidence that would discharge it. Reports will pin the file by
SHA-256 and cite assumptions by id.

- Schema: `code/specs/schemas/ver13-assumption-catalogue-v1.schema.json`.
- Generated from the spec tables: `python3 code/scripts/generate_assumption_catalogue.py`
  (`--check` fails when the committed JSON is stale). Do not edit the JSON by hand;
  change the spec table or the curated overlay in the generator and regenerate.
- Checked by `python3 code/scripts/assumption_catalogue.py`, which also prints the
  digest a report pins. Run in CI under "Verify repo-wide metadata contracts".
- `catalogue-v1.json` changes need a `CHANGELOG.d/` shard.

Rules that need a report (AC-1, 2, 4..7, 9, 10, 12, 13) are enforced by
`ca-assurance-report` and are not implemented yet.
