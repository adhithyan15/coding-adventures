## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `ec516870... (combined with the #19050 TypeScript boundary sync)`.
