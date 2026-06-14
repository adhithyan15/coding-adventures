# Changelog

All notable changes to the `coding-adventures-closure-source-map` crate will be documented in this file.

## [0.4.0] - 2026-06-04

### Added — CLOC12.31: `SourceMapBuilder::build()` VLQ integration (gap-028 step 2/2)

Closes `gap-028` at the crate-implementation level. The
`build()` step no longer produces `mappings: String::new()` —
it now resolves each pending mapping's `cv_id` against the
supplied `CVLog` and emits the source-map v3 delta-encoded
mappings string per spec.

**Resolution policy (new, documented in module-level docs):**

1. Look up `cv_id` in the `CVLog`.
2. If the entry has its own `Origin`, use it.
3. Otherwise walk `parent_ids` depth-first (with a visited-set
   cycle guard) looking for the nearest ancestor `Origin`.
4. Parse `Origin.location` as `"line:column"` (decimal `u32`
   pair). Anything else (`"row_id:8472"`, `"byte:4096"`, etc.)
   falls through to the 1-field segment shape.
5. Index `Origin.source` into `sources` in first-seen order.

**Encoding details:**

- `;` separates lines; `,` separates segments on the same line.
- `generated_column` delta resets to 0 at each `;`.
- `source_index` / `original_line` / `original_column` deltas
  carry across lines.
- Mappings on lines beyond line 0 with no prior content get
  leading `;`s (e.g. first mapping at line 3 → `";;;..."`).
- Resolved mappings use the 4-field segment shape `[gen_col,
  src_idx, orig_line, orig_col]`. Unresolved mappings use the
  1-field shape `[gen_col]`.
- `names` is still emitted as an empty array — the 5-field
  segment shape needs a per-mapping name hint that the emitter
  doesn't currently surface. Wires in a follow-up.

**Defensive behaviour:**

- Mappings sorted by `(generated_line, generated_column)`
  before encoding; the emitter usually feeds in order but the
  builder is safe against out-of-order input.
- Cycles in `parent_ids` (which the public CV API can't
  produce but tests can synthesize) are bounded by a visited
  set — `build()` never loops.
- Empty builder → empty `mappings` and empty `sources` (no
  spurious VLQ output).

### Tests (37 inline total, +12 net new)

| Test | Pins |
|------|------|
| `add_mapping_accumulates` | 3 unresolved mappings → `"A,K;A"` |
| `build_with_single_resolved_mapping_emits_4_field_segment` | `"AAAA"` canonical |
| `build_with_two_resolved_mappings_same_line_delta_encodes` | Same-line `,` separator, `[3,0,0,5]` deltas |
| `build_with_mappings_on_different_lines_uses_semicolon_separators` | Two `;`s for line gap of 2 |
| `build_with_two_sources_indexes_them_in_first_seen_order` | Source-list order, `source_index` delta |
| `build_with_unresolvable_cv_emits_1_field_segment` | Missing-cv fallback |
| `build_mixes_resolved_and_unresolved_segments` | 4-field + 1-field on same line |
| `build_walks_parents_to_find_origin` | `derive`-style chain resolution |
| `build_sorts_out_of_order_input` | Defensive sort |
| `build_skips_unparseable_location_string` | Free-form `location` fallback |
| `build_first_mapping_on_later_line_prefixes_with_semicolons` | Leading `;` prefix |
| `build_with_empty_mappings_returns_empty_string` | Empty-builder identity |
| `build_ignores_self_referential_cycles` | Cycle guard |

### Upstream port

The 7 `#[ignore]`-d tests in
`tests/upstream/source_map_generator_v3_test.rs` stay ignored
but their ignore reasons now reflect the actual remaining
blocker: full-pipeline `compileAndCheck` harness + upstream
Closure-byte-identical golden VLQ capture, tracked under
**CLOC14.1**. The encoder itself is no longer the blocker.

### Public API change

`SourceMapBuilder::build(self, cv: &CVLog) -> SourceMap` — the
`cv` parameter (previously named `_cv` and unused) now drives
resolution. Callers that were passing a real `CVLog` get
populated `mappings` and `sources` automatically. Callers that
were passing an empty `CVLog::new(true)` keep the same
output shape, just with the 1-field segment encoding instead
of empty (because the cv_ids don't resolve to anything).

No source-incompatible change to the type signatures.

### Version bump

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-06-04

### Added — CLOC12.29: base64-VLQ encoding primitive (gap-028 step 1/2)

Lands the encoder primitives that the source-map v3 `mappings` field
needs. Step 1 of 2 for `gap-028`:

* `src/vlq.rs` (new module):
  * `encode_vlq_int(value: i32) -> String` — encode one signed
    integer as its base64-VLQ digit sequence. Implements the
    sign-encoding (LSB carries sign, remaining bits carry
    magnitude), 5-bits-per-digit pumping loop, and continuation-bit
    marking documented in the source-map v3 spec.
  * `encode_vlq_segment(fields: &[i32]) -> String` — concatenate a
    `[generated_column]` (1 field), 4-field, or 5-field segment's
    VLQ-encoded digits.
* `src/lib.rs`: declare the new `mod vlq` and `pub use` both
  helpers so downstream callers (and debug tooling) can verify
  expectations against the canonical encoder without spelling the
  `closure_source_map::vlq::...` path.

What's NOT in this PR (deferred to gap-028 step 2):

* The `SourceMapBuilder::build()` step still produces
  `mappings: String::new()`. Resolving `cv_id` → `(source_index,
  original_line, original_column[, name_index])` quadruples /
  quintuples via the CV graph and computing per-axis deltas
  against the prior segment lives in the builder's `build()` step.
  This PR ships the encoding primitives the builder will use; the
  integration is a separate, larger PR that needs careful design
  of the CVLog-side resolution API.
* The 7 `#[ignore]`-d upstream tests in
  `source_map_generator_v3_test.rs` will flip to live assertions
  in that follow-up PR.

Implementation notes:

* The encoder works in `u32` after sign encoding to side-step
  implementation-defined signed-right-shift territory.
* `i32::MIN` is handled via `wrapping_neg` so the edge case can't
  panic. Realistic source-map values never hit this, but the helper
  shouldn't be a sharp edge.
* Cross-checked against Mozilla's `source-map` library, Google
  Closure Compiler's `Base64VLQ.java`, and the worked examples at
  https://sourcemaps.info/spec.html.

Tests: 13 new inline tests (12 → 25 total in `lib + vlq`):
zero, ±1, ±15 (last single-digit value), ±16 (first two-digit
value), ±32, ±123, ±1000, ±9999, `i32::MIN` / `i32::MAX` smoke,
plus segment-shape (1, 4, 5 fields).

No public API change to `SourceMapBuilder`, `SourceMap`, or
`PendingMapping`. Pure addition.

## [0.2.0] - 2026-06-01

### Added — CLOC12.08: port subset of upstream `SourceMapGeneratorV3Test`

Fifth port under the CLOC12 byte-identical contract. Establishes the
`tests/upstream/` layout for `closure-source-map`.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/source_map_generator_v3_test.rs` — 15 ported test
  methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.08 | **8** | **7** |

**Passing (8):** JSON-shape contract assertions that don't depend on
VLQ encoding:

- `empty_builder_emits_version_3` — version is always `3`.
- `empty_builder_emits_empty_file_field` — default `file` is empty.
- `set_file_reflects_in_json` — `set_file("out.js")` ⇒ `"file": "out.js"`.
- `set_source_root_serializes_as_camelcase_sourceroot` — serde renames `source_root` → `"sourceRoot"`.
- `empty_builder_emits_sources_as_empty_array` — `sources` is a JSON array.
- `empty_builder_emits_names_as_empty_array` — `names` is a JSON array.
- `empty_builder_emits_mappings_as_empty_string` — pins current `""` value (will flip when gap-028 closes).
- `add_mapping_accumulates_raw_count` — raw mapping count tracks `add_mapping` calls even before encoding.

**Ignored (7):** every upstream `compileAndCheck` / `checkSourceMap` test that depends on VLQ-encoded `mappings` strings. All cite gap-028.

| Test | Gap | Blocker |
|------|-----|---------|
| `test_basic_mapping_1` | gap-028 | VLQ encoder + full-pipeline harness |
| `test_basic_mapping_golden_output` | gap-028 | Golden VLQ `"A,aAAAA,QAASA,…"` string |
| `test_literal_mappings` | gap-028 | Multi-identifier VLQ |
| `test_literal_mappings_golden_output` | gap-028 | Golden VLQ |
| `test_multiline_mapping` | gap-028 | Line-delta VLQ |
| `test_multi_function_mapping` | gap-028 | Multi-function VLQ |
| `test_golden_output_0` | gap-028 | Golden JSON |

### Why the bulk is ignored

Upstream `SourceMapGeneratorV3Test` is 25 `@Test` methods, almost
all of which use the `compileAndCheck(js)` helper to drive the full
Closure-compiler pipeline (lex → parse → emit → source-map generate)
and then assert specific VLQ-encoded `mappings` strings. Our
`closure-source-map` crate is at v0.1.0 with no VLQ encoder; the
finalized `mappings` field is always `""` pending Phase 2 v2 work.
Future emitter slices will produce real mappings, and the gap-028
follow-up will land the VLQ encoder; at that point the seven
`#[ignore]`-d tests here can flip to real assertions.

### Version bump

`0.1.0` → `0.2.0`.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 Phase 2 — source-map v3 generator. Companion to `closure-emitter`: receives per-token `(generated_line, generated_column, cv_id)` mappings from the emitter and produces a source-map v3 JSON blob the browser's devtools consume.
- `SourceMapBuilder`:
  - `new() -> Self` (also `Default`).
  - `set_file(&mut self, name: String) -> &mut Self` — generated-file name.
  - `set_source_root(&mut self, root: String) -> &mut Self` — prepended to each `sources` entry.
  - `add_mapping(&mut self, generated_line: u32, generated_column: u32, cv_id: &str) -> &mut Self` — record a token's origin. Fluent (returns `&mut Self` so callers can chain).
  - `raw_mapping_count(&self) -> usize` — debug/test introspection.
  - `build(self, cv: &CVLog) -> SourceMap` — finalize into the serializable form. `cv` parameter is in v1 unused; reserved for v2's VLQ encoder which walks the CV graph to resolve each `cv_id` into `(source_index, original_line, original_column)`.
- `SourceMap` struct with `Serialize` derived: `version: u32` (always 3), `file: String`, `source_root: String` (serializes as `sourceRoot` per v3 spec — camelCase), `sources: Vec<String>`, `names: Vec<String>`, `mappings: String`.
- `SourceMap::to_json(&self) -> String` — wire-format JSON string.
- v1 body: `build()` produces a valid empty v3 blob (`version = 3`, empty arrays, empty mappings string). The raw `(line, col, cv_id)` entries are still accumulated in the builder (visible via `raw_mapping_count`); the VLQ encoder that converts them lands in v2.
- 9 tests covering: builder `new()`/`Default` produce identical state, `add_mapping` accumulates in order (verified via `raw_mapping_count`), `set_file` + `set_source_root` round-trip through `build()` and `to_json`, fluent chaining works, `to_json` output is valid JSON, the top-level object has *exactly* the six v3-spec keys (`version`, `file`, `sourceRoot`, `sources`, `names`, `mappings`) — no extras, no missing, `sourceRoot` is camelCase (not `source_root`), `SourceMap` + `SourceMapBuilder` implement `Clone` + `Debug`.

### Notes
- Dependencies: `coding_adventures_correlation_vector` (`CVLog` consumed by `build()` for v2 VLQ encoding), `serde` + `serde_json` (JSON wire format).
- **Intentionally no `javascript-ast` or `type-sidecar` dependency.** This crate is backend-agnostic: anything that emits source text + per-token CV metadata can use it. Future Lispy / Prolog backends in this monorepo will produce maps the same way.
- v1 is scaffolding. The function signatures, struct shapes, and JSON wire format are the deliverable that `closure-emitter` and the future `closurec` CLI (CLOC08) link against. The VLQ encoder + CV-graph walk lands in v2.
