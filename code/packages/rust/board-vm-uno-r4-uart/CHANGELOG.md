# Changelog

## Unreleased

- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" ...` prefix
  parses as a command named `RUSTC` -- it resolves to rustup's `rustc.exe`
  proxy and fails with `unknown proxy name: 'RUSTC'`. The Windows file runs
  the same steps without the prefix (`rustup run stable cargo build ...`
  selects the toolchain).
