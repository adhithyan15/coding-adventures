# Changelog

## Unreleased

- BUILD / BUILD_windows no longer name the `stable` toolchain (`rustup run stable cargo`, `--toolchain stable`): they use the default toolchain, which CI now pins to an exact Rust version. A pinned default is not the `stable` alias, so naming `stable` would run a second, unpinned toolchain without the targets added by `rustup target add`.

- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" ...` prefix
  parses as a command named `RUSTC` -- it resolves to rustup's `rustc.exe`
  proxy and fails with `unknown proxy name: 'RUSTC'`. The Windows file runs
  the same steps without the prefix (`rustup run stable cargo build ...`
  selects the toolchain).
