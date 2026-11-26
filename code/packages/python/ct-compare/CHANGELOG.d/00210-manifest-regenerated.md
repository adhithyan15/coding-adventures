## Unreleased

- `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated with this lane's own `run --out` command: the shared `warrant-corpus` runner's checks of VER04r section 16.22 (`ill-typed` for a fault case's bad table, `Int` as a model parameter, a `list` binder no longer hiding the models, an error type refused as a field type) moved `runner.src_sha256`. Nothing else changed; this is not an adopter change.
