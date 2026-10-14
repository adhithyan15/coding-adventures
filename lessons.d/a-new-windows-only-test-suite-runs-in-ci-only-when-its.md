---
category: CI & GitHub Actions
---

# A new Windows-only test suite runs in CI only when its crate is in the build-windows-os-suites gate

P2.8a (#17449) added `chief-of-staff-windows-sandbox`, whose
`tests/confinement.rs` runs only on Windows, and Windows tests in
`chief-of-staff-spawn-isolation`. The `build (windows-2025)` job went green
in six minutes. Its log showed only the "Clippy Windows-only Rust crates"
step: "Build and test affected packages" is skipped on Windows unless
`build-windows-os-suites` (or a native toolchain flag) fires, and nothing
in the Rust toolchain flag makes it fire. So the confinement tests had never
run, and the green check proved nothing about them. The earlier lesson about
closurec records the same trap.

Fix: add both crates to the gate's `packages` in
`code/specs/data/ci-gates.json`, describe it in `ci-gate-registry.md`, and
check the emitted plan (`ci_jobs.build-windows-os-suites: true`) before
pushing.

Do differently: whenever a change adds or changes Windows-only tests, check
the job log for the test run itself, not the job's colour, and put the crate
in the gate in the same PR.
