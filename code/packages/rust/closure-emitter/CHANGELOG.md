# Changelog

All notable changes to the `coding-adventures-closure-emitter` crate will be documented in this file.

## [0.3.0] - 2026-05-31

### Added — CLOC12.07: port subset of upstream `CodePrinterTest`

Fourth port under CLOC12, first one targeting the emitter rather than
a transform pass.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/code_printer_test.rs` — 12 ported test methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.07 | **6** | **6** |

**Passing (6):** literal-position emits and bare unary that match our
current emitter output exactly:

- `test_binary_addition_with_parens_is_current_behaviour` — `2 + 3` emits as `(2 + 3);` (paren-wrapped; pins current behaviour).
- `test_string_concat_with_parens_is_current_behaviour` — `"a" + "b"` emits as `("a" + "b");`.
- `test_unary_not_emits_without_space` — `!x` emits as `!x;`.
- `test_boolean_literal_at_statement_position` — `true;` / `false;`.
- `test_integer_literals_at_statement_position` — `0;`, `42;`, `1;`.
- `test_string_literal_at_statement_position` — `"hello";`, `"a";`.

Two of those (`test_*_is_current_behaviour`) deliberately pin our
*current* paren-wrapping behaviour even though it diverges from
upstream — they serve as regression markers so when gap-024 is closed
and the wrapping comes off, the assertions can flip at the same time.

**Ignored (6):** record upstream's broader scope:

| Test | Gap | Blocker |
|------|-----|---------|
| `test_big_int` | gap-021 | `BigIntLiteral` not in Phase 1 AST |
| `test_trailing_comma_in_array_and_object_with_pretty_print` | gap-022 | array/object trailing-comma policy not modelled |
| `test_no_trailing_comma_in_empty_array_literal` | gap-023 | VariableDeclaration round-trip ports deferred |
| `test_number_formatting_shortest_form` | gap-025 | numeric exponential-form / shortest-form not implemented |
| `test_string_quote_choice_minimises_escapes` | gap-026 | quote-choice optimisation not implemented |
| `test_operator_precedence_inserts_inner_parens` | gap-027 | precedence-aware paren insertion not implemented |

Plus the meta-divergence:

- gap-024 — `ExpressionStatement` paren-wrapping is unconditional in our emitter, whereas upstream only wraps when ambiguity demands it. Not strictly "blocked" (we choose to wrap), but tracked so the eventual byte-identical match can flip the two `_is_current_behaviour` ports.

### Why the bulk of upstream is ignored

`CodePrinterTest` has 263 `@Test` methods. Most cover Phase 2+ AST
nodes (BigInt, optional chaining, template literals, classes,
spread, async/await, regex) or formatting policies (quote choice,
exponential-form numerics, precedence-aware parens) that aren't in
our emitter's v0.2.0 body. Each future emitter slice can re-port
the relevant subset and convert ignored markers into asserts.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `emit` body (first real pipeline output)

Replaces v0.1.0's identity emit with a recursive printer that walks every Phase 1 AST node and produces JavaScript text. Step 3 of 4 in the autonomous-chain real-body rollout (after constant-fold + fold-control-flow; before DCE).

- Walks every Phase 1 variant: all expressions (Identifier, literals, Binary/Logical/Unary/Assignment/Conditional/Call/Member, Array with elisions, Object with shorthand/method), all statements (Expression, Block, If, While, For, Return, Break, Continue, Empty), and Declarations (Variable/Function).
- Honors all three `EmitOptions`:
  - `pretty: false` (default) → minified single-line.
  - `pretty: true` → 2-space-indented multi-line for block bodies.
  - `ascii_only: true` → escape non-ASCII as `\uXXXX` / `\u{XXXXXX}`.
  - `source_map: true` (default) → accumulate `(line, col, cv_id)` mappings via `SourceMapBuilder`, serialize as v3 JSON in `EmitOutput.source_map`.
- Tracks line/col cursor (UTF-16 code units per source-map v3 spec).

### Always-parenthesize policy in v1

v1 always parenthesizes `BinaryExpression`, `LogicalExpression`, `ConditionalExpression`, and `AssignmentExpression`. Precedence-aware elision is Phase 1.x. `ObjectExpression` at statement position is also wrapped (else `{}` parses as a block).

### CV tracing — both modes per CLOC09

- **Traced** (`cv: Some` on nodes) → `add_mapping` called per token.
- **Untraced** (`cv: None`) → no mappings recorded; output text identical; `source_map` field still contains a valid empty-mappings v3 blob when enabled.

### Headline test — end-to-end pipeline

```rust
let prog = AST(2 + 3);
let pipeline_out = PassPipeline::new()
    .add(ConstantFoldPass::new())
    .run(prog, &sidecar, &mut cv);
let emit_out = emit(&pipeline_out.program, ...);
assert_eq!(emit_out.code, "5;");
```

The full stack — AST → optimization → emit → text — works end-to-end for the first time.

### Tests

17 tests (up from 9 in v0.1.0): defaults + empty, basic expressions with always-paren, typeof spacing, const/function declarations (minified and pretty), `[1,,3]` array with elision, `({a:1,b:2});` ObjectExpression paren-wrap at statement start, `ascii_only` escapes Unicode (verified with "café"), `source_map` on/off, untraced still emits, **end-to-end pipeline produces `5;` from `2 + 3`**, `EmitError` `std::error::Error` compat.

### Dependencies
- Added `coding-adventures-closure-source-map` as a runtime dep.
- Added `coding-adventures-closure-pass-constant-fold` + `coding-adventures-closure-pass-pipeline` as dev-deps for the end-to-end test.

### Skipped (Phase 1.x / Phase 2+)
- Precedence-aware paren elision.
- Real source-map VLQ encoding (lives in `closure-source-map` v2; mappings accumulate now, final string is still empty).
- `FunctionExpression`, `ArrowFunctionExpression`, `ClassDeclaration` — Phase 2/3.
- JSDoc comment preservation.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 emit-and-source-map spec — the back end of the Closure Compiler clone. Takes a finalized `Program` + sidecar and produces output JavaScript text + companion source-map blob.
- `emit(program: &Program, sidecar: &Sidecar, cv: &mut CVLog, opts: &EmitOptions) -> Result<EmitOutput, EmitError>` — the canonical entry point. Signature pinned.
- `EmitOptions` struct with three knobs:
  - `ascii_only: bool` (default `false`) — when `true`, escape non-ASCII codepoints to `\uXXXX` / `\u{XXXXXX}`.
  - `pretty: bool` (default `false`) — production default is minified; switch on for human-reviewed output.
  - `source_map: bool` (default `true`) — production default is to emit a companion `.js.map`.
- `EmitOutput` struct:
  - `code: String` — JavaScript bytes (UTF-8 or ASCII-restricted).
  - `source_map: Option<String>` — source-map v3 blob; `None` when `source_map = false`.
  - `contributions: Vec<Contribution>` — per-token "emitted" CV trail per CLOC03.
- `EmitError` enum (`#[non_exhaustive]`) with `Display` + `std::error::Error` impls:
  - `UnknownCvId { id, site }` — AST referenced a CV id the log doesn't know.
  - `UnsupportedSidecarType { id, kind }` — sidecar held a type the emitter can't render.
- v1 body: emits empty `code`, an empty source-map placeholder when `source_map = true`, no contributions. `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to render. The real AST walk lands once the AST grows `Statement` / `Expression` / `Declaration` variants.
- 9 tests covering: `EmitOptions::default()` values, identity emit on empty program with default opts (code empty, source_map present-but-empty, contributions empty), `source_map = false` drops the source-map field entirely, `ascii_only` flag accepted (output trivially ASCII when empty), `pretty` flag accepted, `EmitOptions` `Clone` + `PartialEq`, `EmitError::Display` formats for both variants include the id/site/kind they carry, `EmitError` implements `std::error::Error`.

### Notes
- Dependencies: `coding-adventures-javascript-ast` (`Program`), `coding-adventures-type-sidecar` (`Sidecar` for future emit hints), `coding_adventures_correlation_vector` (`CVLog`, `Contribution`), `serde` + `serde_json` (for future source-map serialization and `Contribution.meta`). Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`.
- The emitter does **not** depend on `closure-pass-pipeline` or any pass crate. It runs after the pipeline and only consumes the final `Program` shape — keeping that decoupling means future passes can be added without touching the emit dependency graph.
- v1 is scaffolding. The function signature, options struct, output struct, and error enum are the deliverable that the future `closurec` CLI (CLOC08) and the source-map generator (`closure-source-map`, CLOC07 Phase 2) link against. The body fills in once the AST grows variants.
