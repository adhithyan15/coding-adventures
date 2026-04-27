# wasm-module-encoder

`wasm-module-encoder` turns an in-memory `WasmModule` into raw `.wasm` bytes.

It is the final binary-emission step for code generators that already build the
typed module structure with `@coding-adventures/wasm-types`.
