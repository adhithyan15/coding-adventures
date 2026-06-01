# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `source_map_generator_v3_test.rs`
    - upstream: `test/com/google/debugging/sourcemap/SourceMapGeneratorV3Test.java`
    - blob SHA at port time: `325943abf4dd90afe671014fe05f1195d07bc5c0`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

Fifth port under CLOC12. Same per-crate `tests/upstream/` layout
established in CLOC12.02 / CLOC12.04 / CLOC12.05 / CLOC12.07.

- Upstream tests use a `compileAndCheck(js)` helper that drives the
  full Closure-compiler pipeline (lex → parse → emit → source-map
  generate) and then asserts the resulting map JSON against a
  `TestJsonBuilder` builder. Our `closure-source-map` crate doesn't
  participate in compilation — it's a builder API:
  `SourceMapBuilder { set_file, set_source_root, add_mapping, build }
  -> SourceMap`.
- **VLQ encoding is not implemented yet** in v0.1.0 of the crate.
  The builder accumulates raw `(line, column, cv_id)` mappings; the
  finalized `SourceMap.mappings` field is always the empty string
  pending CLOC07 Phase 2 v2 work.
- Most upstream tests assert specific VLQ `mappings` strings like
  `"A,aAAAA,QAASA,UAAS,EAAG;"` — those are all blocked on VLQ
  encoding and become `#[ignore]` here.
- What we *can* cover today are the JSON-shape invariants — version
  is always 3, file/sourceRoot reflect setters, sources/names are
  arrays, mappings is the empty string. Each ported test docstring
  records the upstream method name being modelled.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for `gap-NNN` entries that gate
ignored ports.

## Skipped (intentionally not ported)

None yet.
