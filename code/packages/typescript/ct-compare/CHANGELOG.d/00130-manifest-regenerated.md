## Unreleased - manifest regenerated for VER04r 16.23 in the v2 lane (CV2-16-23)

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated
  with this lane's own `run --out` command: `typescript/warrant-corpus/src/v2/`
  changed (`v2-fault` row order, `env` and fault-row kit errors, bare
  `model_call`), which moves the runner source digest (`runner.src_sha256`).
  The regeneration is mechanical and is not an adopter change; nothing else
  moved.
