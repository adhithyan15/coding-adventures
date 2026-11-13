# Changelog

## Unreleased

- **Build uses the pinned toolchain.** `BUILD` and `BUILD_windows` call
  `cargo` directly instead of `rustup run stable cargo`. CI now installs the
  exact Rust release named in the root `mise.toml` rather than the floating
  `stable` channel, so naming `stable` here would have run a different
  compiler (or none) from the rest of the build.

- **Schematic import and export work natively.** They now use UI87's standard
  `files.open` / `files.save` kinds with `accept: ["application/json"]`. The
  retired singular `file.*` kinds were answered only by the browser executor,
  so native hosts never completed them. The same migration as VisiCalc's
  (#16927).
- **Schematic import refuses an oversized file before decoding it.** The
  check is on the base64 length, as VisiCalc's is. Native hosts read files up
  to 50 MiB, so decoding first would have held both copies of an oversized
  file in memory before refusing it (security review, LOW).
- **Windows build.** New `BUILD_windows`: the build tool runs each line under
  `cmd /C`, where BUILD's POSIX `RUSTC="$(rustup which rustc)" rustup run ...`
  prefix parses as a command named `RUSTC`, which resolves to rustup's
  `rustc.exe` proxy and fails (`unknown proxy name: 'RUSTC'`). The Windows
  file builds the wasm target with plain `rustup run stable cargo build`.
