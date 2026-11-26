## Unreleased

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated with this lane's own `run --out` command: the shared `warrant-corpus` runner's three checks of VER04r section 16.21 (chunk `calls` sum, tag checks before `level-missing`, consumer-side type-table checks) moved `runner.src_sha256`. Nothing else changed; this is not an adopter change.
