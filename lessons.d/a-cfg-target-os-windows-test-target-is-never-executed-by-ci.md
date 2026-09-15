---
category: CI & GitHub Actions
---

# A `#![cfg(target_os = "windows")]` test target is never executed by CI — the windows leg does not build Rust, on PRs or on main

Two independent reasons, both verified in `.github/workflows/ci.yml`: on a PR the "Build and test affected packages" step (the one that runs `-language all`) is guarded by `if: needs.detect.outputs.is_main != 'true' && (runner.os != 'Windows' || needs.detect.outputs.needs_swift == 'true')` (ci.yml:1529), so on Windows it runs ONLY when the PR touches Swift — a Rust-only PR skips it entirely and the leg still reports green; and on main the `detect` job builds every matrix entry as `"os": "ubuntu-latest"`, so there is no windows leg to skip in the first place. The "Full build on main merge" step has no OS guard, which makes it look like it covers Windows — it does not, because the matrix never puts it there. Consequence: do not count a Windows-gated suite as watched, and do not read a green `build (windows-latest)` as evidence that Rust compiled there. Confirm by grepping the job log for the package name — the macOS leg logs `rust/<pkg>  BUILT`, the windows leg never mentions it.
