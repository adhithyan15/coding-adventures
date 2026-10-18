---
category: CI & GitHub Actions
---

# A CI gate's description in ci-gates.json is capped at 240 Unicode scalars; run the build tool's -emit-plan locally after editing a gate

P2.8b-2 (#17756) added three crates to `build-windows-os-suites` and grew
the gate's `description` in `code/specs/data/ci-gates.json` to name them.
The registry loader caps a description at 240 Unicode scalars, so
"Detect CodeQL languages" failed at once, before any build ran:
`gate "build-windows-os-suites" description exceeds 240 Unicode scalars`.
Every job that plans from the registry fails the same way.

Fix: shorten the description to 214 scalars. The detail belongs in
`code/specs/ci-gate-registry.md`, which has no cap.

Do differently: after any edit to `ci-gates.json`, build the Go build tool
and run `build-tool -diff-base origin/main -emit-plan <file>` from the repo
root. It loads the registry exactly as CI does, so it catches this in
seconds, and it shows whether the gate fires
(`run_build_windows_os_suites=true`).
