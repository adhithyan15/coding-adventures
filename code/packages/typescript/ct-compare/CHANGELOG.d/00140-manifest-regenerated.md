## Unreleased - manifest regenerated for the v2 lane's `ir.fixed` and `ir.tuple` (CV2 S10f)

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated
  with this lane's own `run --out` command: `typescript/warrant-corpus/src/v2/`
  gained `ir.fixed` and `ir.tuple` (a new `fixed.ts`, the typer, the evaluator,
  values, domains and fault typing), which moves the runner source digest
  (`runner.src_sha256`). The regeneration is mechanical and is not an adopter
  change; nothing else moved.
- The v2 lane's `eq_u64` control (`ignore-high-bit`, an `ir.fixed` fault) is live
  in the library now, but this package's manifest is still the v1 one: moving
  `not_live` to 0 for that control needs the v2 corpus reader of S7b and S7c,
  which does not exist yet, so that part of S10f is deferred to them.
