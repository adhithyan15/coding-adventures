---
category: Native extensions & FFI
---

# A new native Perl package must join the helper's repository-input consumers and the Windows gate

`code/scripts/check-perl-extension-windows.ps1` and the `rust/perl-bridge` crate
live outside every Perl package root, so the build tool only re-runs their XS
consumers because `repositoryInputConsumers` in
`code/programs/go/build-tool/repository_inputs.go` lists them, and the
`build-windows-os-suites` gate in `code/specs/data/ci-gates.json` names them.
Pointing a new package's `BUILD_windows` at that helper without adding it to both
lists works on the PR that creates it, then silently stops being exercised when
only the helper or the bridge changes.

When a package starts using the helper: add a consumer entry, extend the two
tests in `repository_inputs_test.go` (the consumer set and the helper-only
toolchain test), add the package to the gate's `packages`, and add the
`# needs-toolchain: rust` line to its `BUILD_windows`. The shared
`run-perl-tests-windows.ps1` helper is not registered this way; it is only
backward-compatible edits that are safe there.
