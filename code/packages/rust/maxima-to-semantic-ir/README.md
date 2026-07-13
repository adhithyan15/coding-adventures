# maxima-to-semantic-ir

Maxima source → narrow-waist [Semantic IR](../semantic-ir), by directly
re-exporting `macsyma-to-semantic-ir`'s public API under Maxima's own name.
The next item in [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)
Stream B's rollout, right after `macsyma-to-semantic-ir`.

## Where it fits in the stack

```
Maxima source
   │
   ▼  macsyma_to_semantic_ir::compile_source   (no shim -- same surface)
semantic_ir::Module
```

This mirrors how [`maxima-runtime`](../maxima-runtime) reuses
`macsyma-runtime` wholesale for *evaluation* — this crate reuses
`macsyma-to-semantic-ir` wholesale for *compilation*. Unlike
[`octave-to-semantic-ir`](../octave-to-semantic-ir) (which needs
`octave-runtime`'s `octavify` source-rewriting shim for a handful of real
surface departures from MATLAB — `#` comments, `endif`/`endfor`/…, `!=`/`!`),
Maxima needs **zero** surface normalization at all: per `maxima-runtime`'s
own doc comment, "a program written for one runs on the other." So where
`octave-to-semantic-ir` is *shim, then delegate*, this crate is just
*delegate* — a plain `pub use` re-export, no wrapper function in between.

## Usage

```rust
use maxima_to_semantic_ir::compile_source;

let module = compile_source("diff(x^3, x)$\n", "demo").unwrap();
assert!(module.functions.iter().any(|f| f.name == "main"));
```

Both `compile_source` (source-in) and `compile` (an already-parsed
`GrammarASTNode`-in) are re-exported, matching every other `-to-semantic-ir`
frontend's own pair — unlike `octave-to-semantic-ir`, which deliberately has
no `compile(tree, ...)` (its shim rewrites text, not a tree). Maxima has no
shim at all, so the full pair applies unchanged.

## Scope

Identical to [`macsyma-to-semantic-ir`](../macsyma-to-semantic-ir) v0.1.0's
own scope — see that crate's module doc comment for the full node-by-node
mapping and its disclosed "no pattern-matching/rewrite-rule syntax in this
cut" boundary. There is nothing Maxima-specific to add or restrict, since
the surface is unchanged.

## Testing

```sh
cargo test -p maxima-to-semantic-ir
```
