# Changelog

All notable changes to the `coding-adventures-closure-emitter` crate will be documented in this file.

## [0.7.0] - 2026-06-01

### Added — CLOC12.13: emit `LabeledStatement` (gap-009 AST partial close)

Adds emitter support for the new `LabeledStatement` variant added to
`javascript-ast` in CLOC12.13. Compact form is `label:body` with
no whitespace; pretty form is `label: body` (single space between
the colon and the body). The body's own emitter writes its trailing
`;`, so we never double-print.

Tests cover `a: foo();`, `break;`, `break a;`, and the literal
upstream-test input `a: break a;`. The `BreakStatement` emit path
was already in place from the original 0.1.0 scaffold — these tests
pin its current behaviour now that there's a label node to combine
with.

## [0.6.0] - 2026-06-01

### Added — CLOC12.12: number formatting shortest-form (closes gap-025)

`format_js_number` now computes both decimal and exponential
representations for finite non-zero numbers and returns whichever is
shorter. Ties pick decimal (canonical). Matches upstream
`CodePrinter`'s behaviour.

| Value | Old emit | New emit |
|-------|----------|----------|
| `1` | `1` | `1` |
| `100` | `100` | `100` (tie 3=3 → decimal) |
| `1_000_000_000` | `1000000000` | `1E9` |
| `5_000_000` | `5000000` | `5E6` |
| `0.5` | `0.5` | `0.5` |
| `1.5e-10` | `0.00000000015` | `1.5E-10` |
| `NaN` | `NaN` | `NaN` |
| `Infinity` | `Infinity` | `Infinity` |

Exponential form follows JS / upstream conventions: uppercase `E`,
no leading `+` for positive exponents, stripped trailing zeros in
the mantissa (`1E9`, not `1.0E+9`).

### New helper

`format_exponential_uppercase(n: f64) -> String` — wraps Rust's
`{:e}` formatter and uppercases the `E`.

### New inline tests (5)

- `number_shortest_form_small_integers_stay_decimal` — `0`, `1`, `42`, `100`, `-7`.
- `number_shortest_form_big_integers_switch_to_exponential` — `1E9`, `5E6`.
- `number_shortest_form_small_decimals_stay_decimal` — `0.5`, `3.14`.
- `number_shortest_form_tiny_floats_switch_to_exponential` — `1.5E-10`.
- `number_shortest_form_nan_and_infinity_unchanged` — sanity check.

Plus `emit_number_value(v: f64) -> String` helper.

### gap-025 → RESOLVED

### Reconciles missing version bump from CLOC12.11

CLOC12.11 (PR #4703) updated the CHANGELOG to `[0.5.0]` but the
`Cargo.toml` change was dropped, leaving the published crate at
`0.4.0`. This PR bumps directly `0.4.0` → `0.6.0`: the `0.5.0`
CHANGELOG entry below stays valid as the description of quote-choice
work; `0.6.0` is the first published version that actually includes
both quote-choice (CLOC12.11) AND shortest-form number rendering
(CLOC12.12).

### Version

`0.4.0` → `0.6.0` (skips `0.5.0` to absorb the missed CLOC12.11
Cargo.toml bump).

## [0.5.0] - 2026-06-01

### Added — CLOC12.11: string quote-choice optimisation (closes gap-026)

`emit_string` now picks the quote style that minimises required
escape characters. Upstream's CodePrinter does the same; matching it
brings us a step closer to byte-identical output.

**Algorithm** — count occurrences of `"` and `'` in the value. If
`"` strictly outnumbers `'`, emit with single quotes (each saved
`\"` is shorter); otherwise emit with double quotes (canonical form,
ties broken toward double).

```
value                            chosen quote   why
-----------------------------    ------------   -------------------
hello                            double         no quotes anywhere
o'malley                         double         no `"`; cheaper as `"o'malley"`
she said "hi"                    single         `"` saves one escape
"mixed 'both'"                   double         tie (2 each) → double
""x                              single         two `"`, zero `'`
```

`ascii_only` mode still always uses double quotes — switching mid-mode
would confuse downstream readers and upstream itself maintains that
invariant.

### New helpers in `lib.rs`

- `choose_quote_and_escape(value: &str) -> (&'static str, String)` —
  returns the chosen quote character plus the escaped body.
- `escape_str_sq(s: &str) -> String` — single-quoted variant of
  `escape_str_dq`. Identical control-char rules; differs only in
  which quote it escapes.

### New inline tests (6)

- `quote_choice_no_quotes_uses_double` — `"hello"`, `""`.
- `quote_choice_single_quotes_in_value_uses_double` — `"o'malley"`, `"it's"`.
- `quote_choice_double_quotes_in_value_switches_to_single` — `'she said "hi"'`.
- `quote_choice_tie_picks_double` — value `'"`, leading byte = `"`.
- `quote_choice_more_double_than_single_picks_single` — value `""x`, leading byte = `'`.
- (helper) `emit_string_value(value: &str) -> String` — emit a
  synthetic StringLiteral and return the code; used by the four
  parametric assertions.

### Side effect

The previous emit_string path used `s.raw` verbatim when present
(preserving the source-file's quote style). That's no longer used —
quote-choice now applies uniformly. The `raw` field is still
preserved in the AST for tooling but isn't consulted by emit.

### gap-026 → RESOLVED

The `test_string_quote_choice_minimises_escapes` placeholder in
`tests/upstream/code_printer_test.rs` stays `#[ignore]`-d pending a
follow-up that re-ports it with real upstream `assertPrint` cases
now that the underlying emitter behaviour is in place.

### Version bump

`0.4.0` → `0.5.0`.

## [0.4.0] - 2026-06-01

### Added — CLOC12.10: precedence-aware paren insertion (closes gap-024 + gap-027)

Replaces the previous "wrap every expression-statement body in parens"
policy with a precedence-aware emit. `emit_expression_inner(e, parent_prec)`
inspects the expression's own precedence and wraps in parens **only**
when the child binds more loosely than its parent context demands.

Precedence ladder (low → high, per ESTree §13):

```
 0   top level / statement / control-test position
 1   assignment
 2   conditional `? :`
 3   logical-or `||` / nullish `??`
 4   logical-and `&&`
 5-7 bitwise or/xor/and
 8   equality
 9   relational
10   shift
11   additive
12   multiplicative
13   exponent          right-assoc
14   prefix unary
17-18 call/member/primary       atomic — never wraps
```

Three new helper functions in `lib.rs`:

- `binary_prec(BinaryOperator) -> u8`
- `logical_prec(LogicalOperator) -> u8`
- `expr_prec(&Expression) -> u8`

`emit_binary` / `emit_logical` / `emit_conditional` no longer wrap
themselves in parens. They delegate to `emit_expression_inner` for
their children with appropriate `parent_prec` values
(`my_prec` for the left side, `my_prec + 1` for the right side of
left-associative operators).

### Truth table

| Source AST                     | Old emit               | New emit          |
|--------------------------------|------------------------|-------------------|
| `2 + 3`                        | `(2 + 3);`             | `2 + 3;`          |
| `"a" + "b"`                    | `("a" + "b");`         | `"a" + "b";`      |
| `(a + b) * c`                  | `((a + b) * c);`       | `(a + b) * c;`    |
| `a + b * c`                    | `(a + (b * c));`       | `a + b * c;`      |
| `!x`                           | `!x;` (unchanged)      | `!x;`             |
| `({a:1})`                      | `({a:1});` (unchanged) | `({a:1});`        |

### gap-024 → RESOLVED

The two `_is_current_behaviour` ports in
`tests/upstream/code_printer_test.rs` are renamed and their assertions
flipped to upstream's byte-equivalent forms:

| Old name (pinned divergence) | New name (matches upstream) | Was | Now |
|-------------------------------|------------------------------|-----|-----|
| `test_binary_addition_with_parens_is_current_behaviour` | `test_binary_addition_emits_without_outer_parens` | `(2 + 3);` | `2 + 3;` |
| `test_string_concat_with_parens_is_current_behaviour` | `test_string_concat_emits_without_outer_parens` | `("a" + "b");` | `"a" + "b";` |

The remaining whitespace difference between our `2 + 3;` (pretty-printed)
and upstream's `2+3;` (minified) is addressed by the pretty/minify
toggle work, not gap-024.

### gap-027 → RESOLVED (incidental)

The precedence ladder also closes gap-027 (precedence-aware paren
insertion) — they were two views of the same underlying problem. The
`test_operator_precedence_inserts_inner_parens` placeholder stays
`#[ignore]`-d pending a follow-up that adds the actual upstream
`a*(b+c)` test cases now that the emitter supports them.

### Updated inline tests

Three inline tests in `lib.rs`:

- `binary_addition_with_parens` renamed to `binary_addition_emits_without_outer_parens`, assertion flipped to `"2 + 3;"`.
- `string_concat_with_parens` renamed to `string_concat_emits_without_outer_parens`, assertion flipped to `"\"foo\" + \"bar\";"`.
- `untraced_program_still_emits` assertion flipped to `"2 + 3;"`.

### Version bump

`0.3.0` → `0.4.0`.

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
