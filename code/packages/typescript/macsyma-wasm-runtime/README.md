# @coding-adventures/macsyma-wasm-runtime

Browser TypeScript bridge for the Rust MACSYMA WASM runtime.

This package does not check in generated `wasm-pack` output. Instead, callers
build `code/packages/wasm/macsyma-runtime` with `wasm-pack build --target web`
and pass the generated module loader to `loadMacsymaWasmRuntime`.

```ts
import { loadMacsymaWasmRuntime } from "@coding-adventures/macsyma-wasm-runtime";

const runtime = await loadMacsymaWasmRuntime(() => import("./pkg/macsyma_runtime_wasm.js"));
const result = runtime.eval("x : 5$\nx + 2;");
console.log(result.visibleOutputs);
```

The bridge keeps the browser-facing API stable even though the underlying Rust
runtime crosses the WASM boundary as JSON strings.
