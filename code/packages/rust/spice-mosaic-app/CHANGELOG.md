# Changelog

## Unreleased

- BUILD / BUILD_windows no longer name the `stable` toolchain (`rustup run stable cargo`, `--toolchain stable`): they use the default toolchain, which CI now pins to an exact Rust version. A pinned default is not the `stable` alias, so naming `stable` would run a second, unpinned toolchain without the targets added by `rustup target add`.

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
