## Unreleased

- `BUILD` and `BUILD_windows` end with `ca-assurance-report-rs --check-lane go/ct-compare --repo-root ../../../.. --require-active` (WD-004b2, VER04i 8.3): the Rust lane loader re-checks the manifest, labels sidecar, binding, build routing and the recomputed `source_sha256` and `runner.src_sha256`, so a lane edit that leaves them stale, or a lane that stops being active, fails its own build. No source, manifest or test change.
