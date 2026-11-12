# Changelog

## Unreleased

- **Build uses the pinned toolchain.** `BUILD` and `BUILD_windows` call
  `cargo` directly instead of `rustup run stable cargo`. CI now installs the
  exact Rust release named in the root `mise.toml` rather than the floating
  `stable` channel, so naming `stable` here would have run a different
  compiler (or none) from the rest of the build.

## 0.1.1

- Validate that the runtime compiles for `wasm32-unknown-unknown` in `BUILD`.
- Document the TypeScript browser bridge package.

## 0.1.0

- Add `wasm-bindgen` bindings for the Rust MACSYMA runtime JSON facade.
