## Unreleased

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated with this lane's own `run --out` command: the shared `warrant-corpus` runner's checks of VER04r section 16.23 (the op row read with the table, the three static codes of `v2-fault`, `ill-typed` for an `env` type, a malformed fault row as a kit error) moved `runner.src_sha256`. Nothing else changed; this is not an adopter change.
