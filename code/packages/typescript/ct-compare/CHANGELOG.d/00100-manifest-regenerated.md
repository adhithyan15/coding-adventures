## Unreleased - manifest regenerated for the v2 runner library (CV2-S7a)

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated
  with this lane's own `run --out` command: `typescript/warrant-corpus` gained a
  `src/v2/` tree, which changes the runner source digest (`runner.src_sha256`).
  The regeneration is mechanical and is not an adopter change; nothing else
  moved, and this package still runs the v1 corpus.
- Regenerated a second time after the S7a review fixes to
  `typescript/warrant-corpus/src/v2/` (the digest covers the whole source tree).
  Again only `runner.src_sha256` and `manifest_sha256` moved, plus the matching
  digest in `domain-labels.json`.
