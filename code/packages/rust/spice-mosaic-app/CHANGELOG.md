# Changelog

## Unreleased

- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" rustup run ...`
  prefix parses as a command named `RUSTC`, which resolves to rustup's
  `rustc.exe` proxy and fails (`unknown proxy name: 'RUSTC'`). The Windows
  file builds the wasm target with plain `rustup run stable cargo build`.
