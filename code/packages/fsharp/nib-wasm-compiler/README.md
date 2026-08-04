# CodingAdventures.NibWasmCompiler.FSharp

A pure F# compiler for the portable Nib `u4` function subset. It accepts typed
function declarations whose return expressions contain `u4` literals,
parameters, function calls, and wrapping `+%` addition, then emits a validated
WebAssembly-shaped module through the native `wasm-module-encoder` package.

```fsharp
let result =
    NibWasmCompiler.compileSource
        "fn add(a: u4, b: u4) -> u4 { return a +% b; }"
```

Each Nib function is exported under its source name. Compilation is in memory;
`writeWasmFile` is the optional filesystem boundary.
