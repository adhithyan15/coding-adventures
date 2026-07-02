# coding-adventures-closure-emitter

JavaScript code emitter for the Closure Compiler clone. The back
end: takes a finalized `Program` + sidecar and produces output
JavaScript text + a companion source-map blob. Per
[CLOC07](../../../specs/CLOC07-emit-and-source-map.md).

## Where this sits

```text
lexer → parser → AST ──┐
                       ├─► passes ──► Program' ──► emitter ──► .js
               sidecar ─┘                                       .js.map
```

The emitter runs **after** every optimization pass. It links
against `javascript-ast` (the data it reads) and `type-sidecar`
(for per-node emit hints), but **not** against any pass crate
or `closure-pass-pipeline` — the emitter doesn't know or care
what passes ran, only that the program it receives is the final
shape.

## API

```rust
pub fn emit(
    program: &Program,
    sidecar: &Sidecar,
    cv: &mut CVLog,
    opts: &EmitOptions,
) -> Result<EmitOutput, EmitError>;
```

```rust
pub struct EmitOptions {
    pub ascii_only: bool,   // default false (UTF-8 output)
    pub pretty: bool,       // default false (minified output)
    pub source_map: bool,   // default true  (companion .js.map)
}
```

```rust
pub struct EmitOutput {
    pub code: String,                       // JavaScript bytes
    pub source_map: Option<String>,         // source-map v3 blob
    pub contributions: Vec<Contribution>,   // per-token CV trail
}
```

```rust
#[non_exhaustive]
pub enum EmitError {
    UnknownCvId { id: String, site: &'static str },
    UnsupportedSidecarType { id: String, kind: String },
}
```

## What's here (v1)

- The `emit()` function signature — locked.
- `EmitOptions` struct + sensible production defaults (UTF-8,
  minified, source-map on).
- `EmitOutput` struct — `code`, `source_map`, `contributions`.
- `EmitError` enum with `Display` + `std::error::Error` impls.
- v1 body: emits empty `code`, an empty source-map placeholder
  when `source_map = true`, no contributions. Real walk lands
  once `javascript-ast` grows `Statement` / `Expression` /
  `Declaration` variants.

## What this PR locks down even as identity

1. The function signature — `emit(program, sidecar, cv, opts)`.
   Once the AST grows, the body fills in; call sites in the
   future `closurec` CLI don't change.
2. The three CLOC07 options — `ascii_only`, `pretty`,
   `source_map` — with the production-safe defaults.
3. The `Result<EmitOutput, EmitError>` shape so the CLI can
   `?` against a concrete error type.

## What's coming

- v2: emit actual JavaScript text. Walks the AST, consults the
  sidecar for per-node hints (numeric base, quote style,
  template-literal vs. concat, etc.), writes per-token
  "emitted" `Contribution`s to `cv`.
- CLOC07 Phase 2: real source-map v3 generation in the
  companion `closure-source-map` crate. This crate's
  `source_map` field becomes the actual map blob.
- CLOC08: `closurec` CLI that wires `emit()` up to file IO.

## Upstream conformance tests

`tests/upstream/` ports Google Closure Compiler `CodePrinterTest.java`
(Apache-2.0; see `ATTRIBUTION.md` and `UPSTREAM_SHA`), per the CLOC12
test-port convention. Each file isolates one printing area:

- `code_printer_test.rs` / `code_printer_declarations_test.rs` /
  `code_printer_trailing_comma_test.rs` — core expressions, `var`/`let`/`const`
  declarations, trailing-comma handling.
- `code_printer_numbers_test.rs` — numeric formatting.
- `code_printer_string_escape_test.rs` / `code_printer_ascii_escape_test.rs` —
  default-mode and `ascii_only` string escaping.
- `code_printer_object_literal_test.rs` — object-literal printing:
  identifier / string / numeric / computed keys, the key quote-stripping rules
  (including the `"__proto__"` exception), shorthand, and statement-start
  parenthesization. 13 active `#[test]`s, no `#[ignore]`. Run with
  `cargo test --test upstream_code_printer_object_literal`.
- `code_printer_function_test.rs` — function-expression printing:
  anonymous / named, params, body, IIFE, member-object and call-argument
  wrapping, generator / async prefixes. Run with
  `cargo test --test upstream_code_printer_function`.
- `code_printer_arrow_test.rs` — arrow-function printing: param-paren drop,
  concise vs block body, object-literal-body wrap, IIFE, member-object,
  call-argument, async prefix. Run with
  `cargo test --test upstream_code_printer_arrow`.
- `code_printer_template_test.rs` — template-literal printing: no-substitution
  templates (escaped backtick / `${`), a template as an unwrapped member-object
  and binary operand (it is primary), and `${…}` substitution templates
  (single / adjacent / text-interleaved / low-precedence and member-access
  bodies, and multiline quasis with literal interior newlines). 19 active
  `#[test]`s, no `#[ignore]` (gap-158 resolved in CLOC12.157 — the emitter is
  now newline-aware). Run with
  `cargo test --test upstream_code_printer_template`.

## Dependency whitelist

- `coding-adventures-javascript-ast` — `Program` input.
- `coding-adventures-type-sidecar` — per-node emit hints.
- `coding_adventures_correlation_vector` — per-token
  `Contribution` per CLOC03; receives a mutable `CVLog`.
- `serde` + `serde_json` — required for future `Contribution`
  meta payloads and source-map serialization.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
