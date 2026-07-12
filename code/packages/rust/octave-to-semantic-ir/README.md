# octave-to-semantic-ir

GNU Octave source → narrow-waist [Semantic IR](../semantic-ir), by reusing
the *entire* MATLAB-to-SIR pipeline behind a thin source-compatibility
shim. Item **Stream A rollout #5** of
[`HML01`](../../../specs/HML01-math-to-semantic-ir.md).

## Where it fits in the stack

```
Octave source
   │
   ▼  coding_adventures_octave_runtime::octavify   (source-rewrite shim)
MATLAB-syntax source
   │
   ▼  matlab_to_semantic_ir::compile_source
semantic_ir::Module
```

This mirrors exactly how [`octave-runtime`](../octave-runtime) reuses
`matlab-runtime` wholesale for *evaluation* — this crate reuses
[`matlab-to-semantic-ir`](../matlab-to-semantic-ir) wholesale for
*compilation*. There is no Octave parser and no Octave-specific SIR node:
`octavify` normalizes surface syntax (`#` comments, `endif`/`endfor`/
`endwhile`/`endfunction`/`endswitch`/`end_try_catch`, `!=`/`!`) to MATLAB
*before* anything is parsed, so by the time a tree exists it already is a
MATLAB one.

## Usage

```rust
use octave_to_semantic_ir::compile_source;

let src = "x = 0; # start\nfor i = 1:3\n  x = x + i;\nendfor\n";
let module = compile_source(src, "demo").unwrap();
assert!(module.functions.iter().any(|f| f.name == "main"));
```

## Scope

Whatever [`matlab-to-semantic-ir`](../matlab-to-semantic-ir) v0.1.0
supports, minus whatever `octavify` cannot yet normalize (`++`/`--`,
`do…until` — both documented deferrals in `octave-runtime`'s own doc
comment, left untouched by the shim and reported as ordinary MATLAB
parse/lower errors here).

Unlike every other `-to-semantic-ir` frontend, this crate has **no**
`compile(tree, ...)` entry point — only `compile_source`. There is no
Octave-specific CST to hand in: the shim rewrites *text*, and the only
tree ever built is the MATLAB one `matlab_to_semantic_ir::compile_source`
constructs internally.

## Testing

```sh
cargo test -p octave-to-semantic-ir
```
