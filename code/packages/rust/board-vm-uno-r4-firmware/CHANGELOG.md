# Changelog

## Unreleased

- **One more Windows test expectation.** `parses_print_upload_and_smoke_options`
  still compared `arm_gxx` and `arm_ar` against Unix-formatted paths; they now
  use the same `native_tool` helper as `arm_gcc` (`\\` separators and `.exe` on
  Windows). Found when this package next ran on the Windows lane.

- **Build uses the pinned toolchain.** `BUILD` and `BUILD_windows` call
  `cargo` directly instead of `rustup run stable cargo`. CI now installs the
  exact Rust release named in the root `mise.toml` rather than the floating
  `stable` channel, so naming `stable` here would have run a different
  compiler (or none) from the rest of the build.

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
