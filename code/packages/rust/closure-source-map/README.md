# coding-adventures-closure-source-map

Source-map v3 generator for the Closure Compiler clone. Companion
to [`closure-emitter`](../closure-emitter): receives per-token
mappings from the emitter and produces a source-map v3 JSON blob
the browser's devtools consume. Per
[CLOC07 Phase 2](../../../specs/CLOC07-emit-and-source-map.md).

## Why a separate crate from the emitter?

1. **Different audiences.** The emitter cares about valid JavaScript
   bytes; this crate cares about the precisely-specified JSON wire
   format.
2. **Different reuse story.** Anything that emits source text +
   position metadata can use this builder — future Lispy / Prolog
   backends in the same monorepo will produce maps the same way.
3. **Pure data transform.** No AST, no sidecar, no CV log mutation
   — just `(line, col, cv_id)` entries in, JSON blob out.

## API

```rust
let mut b = SourceMapBuilder::new();
b.set_file("out.js".into())
    .set_source_root("/src/".into())
    .add_mapping(0, 0, "node.1")
    .add_mapping(0, 5, "node.2");
let map = b.build(&cv_log);     // SourceMap
let json = map.to_json();       // String — the wire format
```

The resulting JSON is shaped per the source-map v3 spec:

```json
{
  "version": 3,
  "file": "out.js",
  "sourceRoot": "/src/",
  "sources": ["in.js"],
  "names": ["userName"],
  "mappings": "AAAA,SAASA,QAAQ;..."
}
```

## Why CV ids in the intermediate form?

Per CLOC02 the AST doesn't carry source ranges — it carries CV
ids. The CV graph maps each id back to the bytes it traces to,
even across optimization passes. Storing the CV id in the source
map's pending form lets us defer the lookup until `build()`,
which is when the VLQ encoder walks the CV graph and produces the
final `sources` / `names` index lists.

## What's here (v1)

- `SourceMapBuilder` with `new()`, `set_file`, `set_source_root`,
  `add_mapping`, `build()`, `raw_mapping_count()`. Fluent setters
  return `&mut Self` so CLI/emitter code can chain.
- `SourceMap` struct with `Serialize` derived; serializes to v3
  JSON exactly (`sourceRoot` is camelCase in the output, not
  `source_root`).
- `SourceMap::to_json()` produces the wire-format string.
- v1 `build()` produces a valid empty v3 blob:
  `version=3`, empty `sources`/`names`/`mappings` strings. The
  raw `(line, col, cv_id)` mappings are still accumulated in the
  builder (visible via `raw_mapping_count`); the VLQ encoder
  that converts them lands in v2.

## What's coming (v2)

- Walk the CV graph in `build()` to resolve each pending
  `cv_id` into the `(source_index, original_line, original_column)`
  triple the v3 spec requires.
- VLQ-encode the resolved triples into the `mappings` string.
- Build the `sources` and `names` arrays from the resolved
  triples.
- Integration with the non-identity `closure-emitter` so the
  emitter feeds real per-token mappings.

## Dependency whitelist

- `coding_adventures_correlation_vector` — `CVLog` is required
  by `build()` (the future VLQ encoder walks the graph to
  resolve `cv_id` → original `(file, line, col)`).
- `serde` + `serde_json` — the JSON wire format.

(No `javascript-ast` or `type-sidecar` dependency — this crate
is intentionally backend-agnostic.)
