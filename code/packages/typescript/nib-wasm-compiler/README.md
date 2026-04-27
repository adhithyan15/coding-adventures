# `@coding-adventures/nib-wasm-compiler`

`@coding-adventures/nib-wasm-compiler` is the TypeScript end-to-end Nib to
WebAssembly package.

It composes the existing local packages:

```text
Nib source -> parser -> type checker -> compiler IR -> Wasm module -> bytes
```

## Usage

```ts
import { compileSource } from "@coding-adventures/nib-wasm-compiler";

const result = compileSource("fn answer() -> u4 { return 7; }");
console.log(result.binary);
```

The returned result includes each intermediate artifact so learners can inspect
the pipeline one stage at a time.
