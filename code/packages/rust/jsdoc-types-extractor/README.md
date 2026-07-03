# coding-adventures-jsdoc-types-extractor

Walks a parsed JSDoc document and emits
[`type-sidecar`](../type-sidecar) records per
[CLOC05 §"jsdoc-types-extractor"](../../../specs/CLOC05-jsdoc-sub-pipeline.md).

## What's here (v1)

- `extract_types(source, anchor_cv)` — parse JSDoc source, lower
  tags, return a `Sidecar` with one record at `anchor_cv` (or empty
  if there are no `@type` / `@param` / `@returns` tags).
- `extract_from_ast(ast, anchor_cv)` — same, but takes a pre-parsed
  `GrammarASTNode` (saves a re-parse for callers that already have one).
- Primitive-only type lowering: `number`/`string`/`boolean`/`null`/
  `undefined`/`void`/`any`/`unknown`/`never`/`bigint`/`symbol` map to
  their `Type` variants. JSDoc `void` ≡ `Type::Undefined`. Everything
  else (`Foo`, `Foo[]`, `?Foo`, `function(...): T`, …) becomes
  `Type::Opaque { raw }` carrying the reconstructed source text.
- `@param`/`@returns` payloads stash into
  `attributes.extension["params"]` (array of `{type, name}` objects)
  and `attributes.extension["returns"]` (single `{type}` object) as
  JSON blobs. These move to proper typed slots when `type-sidecar`
  gains `Type::Function` / `FunctionParam`.

## What's deferred

- Per-anchor flow driven by `jsdoc-comment-extractor`'s `BlockComment`
  list (v1 takes one body string + one anchor).
- Richer type lowering once the `Type` lattice grows
  (`Object`/`Function`/`Class`/`Union`/`Intersection`/`Generic`/
  `NamedRef`/literal types).
- The full tag set from CLOC05's mapping table (`@template`,
  `@typedef`, `@deprecated`, `@public`, …). Unknown tags survive
  silently — they're parsed but not emitted.

## Dependency whitelist

- `coding-adventures-jsdoc-parser` — to get the parsed JSDoc AST.
- `coding-adventures-type-sidecar` — the output format.
- `parser` — for `GrammarASTNode` / `find_nodes` walking helpers.
- `serde_json` — for the JSON values stashed into
  `attributes.extension`.

No `correlation-vector` dep yet — the caller supplies the `anchor_cv`
string directly. Full CV plumbing (where this extractor pulls anchors
from the CV log itself) lands with the AST-driven version of
`jsdoc-comment-extractor`.
