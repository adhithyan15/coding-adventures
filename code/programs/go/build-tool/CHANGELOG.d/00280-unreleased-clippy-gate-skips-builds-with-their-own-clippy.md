## Unreleased - clippy gate leaves a BUILD that runs its own `cargo clippy` alone

### Changed

- `clippyStepFor` no longer prepends the automatic `cargo clippy --all-targets -- -D warnings` host-target step to a Rust package whose BUILD already runs `cargo clippy` itself (quoted mentions do not count). The case is `rust/os-kernel`, a `no_std` UEFI application whose binary cannot compile for the host (`unwinding panics are not supported without std`); its BUILD lints the target it really builds for and the host lib/tests explicitly, with the same `-D warnings`, so the package is still gated by its own steps.
