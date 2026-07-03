# Changelog

All notable changes to the `coding-adventures-jsdoc-types-extractor` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC05 §"jsdoc-types-extractor."
- `extract_types(source: &str, anchor_cv: &str) -> Result<Sidecar, String>` — parse JSDoc body via `parse_jsdoc`, walk the resulting `GrammarASTNode` tree, recognize `@type` / `@param` / `@returns`, and emit a single `Record` at `anchor_cv`. Empty sidecar if no recognised tags are present.
- `extract_from_ast(ast: &GrammarASTNode, anchor_cv: &str) -> Sidecar` — same lowering, but takes a pre-parsed AST.
- Primitive-only type lowering: `number`/`string`/`boolean`/`null`/`undefined`/`void`/`any`/`unknown`/`never`/`bigint`/`symbol` map to their `type-sidecar::Type` variants. JSDoc convention `void ≡ undefined`. Everything else collapses to `Type::Opaque { raw }` with the reconstructed source text — the richer lowering arrives when the `Type` lattice expands.
- `@param` payloads accumulate into `attributes.extension["params"]` as a JSON array of `{ "type", "name" }` objects. `@returns` payload lands in `attributes.extension["returns"]` as a single `{ "type" }` object. Both move to proper typed slots once `type-sidecar` gains `Type::Function`/`FunctionParam`.
- Provenance: `ProducerId("jsdoc")` + `producer_version = "0.1.0"` + one `EvidenceStep { stage: "extract" }` per emitted record.
- 13 tests covering: empty source, all 4 covered primitives (number/string/boolean/void→undefined), Foo → Opaque(Foo), dotted nominal → Opaque, @param array entry, @returns object entry, multi-tag coalescing into one record, provenance content (producer/version/source_location/evidence step), unknown tag silently ignored, type + param combined record, extract_from_ast path.

### Notes
- Dependencies: `coding-adventures-jsdoc-parser` (input AST), `coding-adventures-type-sidecar` (output format), `parser` (for `GrammarASTNode`/`find_nodes`), `serde_json` (for the `extension` JSON blobs).
- No `correlation-vector` dep yet: the caller supplies `anchor_cv` as a string. The full CV-plumbed flow lands with the AST-driven version of `jsdoc-comment-extractor`.
- Closes Stage 2 of the CLOC pipeline alongside `type-sidecar`/`type-sidecar-merger`/`jsdoc-{lexer,parser,comment-extractor}`.
