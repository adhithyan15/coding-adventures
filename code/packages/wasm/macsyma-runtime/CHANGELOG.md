# Changelog

## Unreleased

- BUILD / BUILD_windows no longer name the `stable` toolchain (`rustup run stable cargo`, `--toolchain stable`): they use the default toolchain, which CI now pins to an exact Rust version. A pinned default is not the `stable` alias, so naming `stable` would run a second, unpinned toolchain without the targets added by `rustup target add`.

- Resolve `rustup` through `PATH` in the POSIX build and install the WebAssembly
  target for the stable toolchain used by its Cargo commands.

## 0.1.1

- Validate that the runtime compiles for `wasm32-unknown-unknown` in `BUILD`.
- Document the TypeScript browser bridge package.

## 0.1.0

- Add `wasm-bindgen` bindings for the Rust MACSYMA runtime JSON facade.
