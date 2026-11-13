# Changelog

## Unreleased

- **Build uses the pinned toolchain.** `BUILD` and `BUILD_windows` call
  `cargo` directly instead of `rustup run stable cargo`. CI now installs the
  exact Rust release named in the root `mise.toml` rather than the floating
  `stable` channel, so naming `stable` here would have run a different
  compiler (or none) from the rest of the build.

- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" ...` prefix
  parses as a command named `RUSTC` -- it resolves to rustup's `rustc.exe`
  proxy and fails with `unknown proxy name: 'RUSTC'`. The Windows file runs
  the same steps without the prefix (`rustup run stable cargo build ...`
  selects the toolchain).
