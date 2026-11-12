# Changelog

## Unreleased

- BUILD / BUILD_windows no longer name the `stable` toolchain (`rustup run stable cargo`, `--toolchain stable`): they use the default toolchain, which CI now pins to an exact Rust version. A pinned default is not the `stable` alias, so naming `stable` would run a second, unpinned toolchain without the targets added by `rustup target add`.

- **Windows test expectations.** Five `serial_usb_artifact` tests asserted
  Unix-formatted paths (`/`, no `.exe`) for a planner that correctly builds
  native paths, so they failed on Windows (backslash separators and `.exe`).
  The expected values are now built with `PathBuf` and `EXE_SUFFIX`; the planner
  itself is unchanged.

- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" ...` prefix
  parses as a command named `RUSTC` -- it resolves to rustup's `rustc.exe`
  proxy and fails with `unknown proxy name: 'RUSTC'`. The Windows file runs
  the same steps without the prefix (`rustup run stable cargo build ...`
  selects the toolchain; with no `RUSTC` set, the artifact tool's
  `--print-only` plan finds rustc itself).
