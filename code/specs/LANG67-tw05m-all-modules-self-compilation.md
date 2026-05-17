# LANG67 — TW05-M: All-Module Self-Compilation (11 files → 173 total functions)

## Overview

TW05-M extends the `self-compile-all` pipeline from **seven** files (TW05-L,
171 total functions) to **eleven** files (TW05-M, **173** total functions)
by adding the four remaining compiler modules:

| Module | Chars | Functions | Notes |
|--------|-------|-----------|-------|
| `token.tw` | 2 782 | **0** | union-only (TokenKind + Token record) |
| `ast.tw` | 3 817 | **0** | union-only (Expr union) |
| `iir-types.tw` | 3 073 | **0** | union-only (TypeHint + IirInstr record) |
| `main.tw` | 11 699 | **2** | `main` + `self-compile-all` |

After this milestone `self-compile-all` compiles every `.tw` file in the
compiler source tree and returns **173** total emitted function definitions.

## Context

TW05-L (LANG66) proved the pipeline on all seven modules that contain explicit
`(define ...)` function forms.  The remaining four modules are either
"0-fn union modules" (token, ast, iir-types — no `define` forms at all)
or the root module itself (main.tw — 2 defines).

### Why union/record modules produce 0 functions

`emit-program` only counts `DefExpr(LambdaExpr)` top-level forms — i.e.
`(define (fn params) body)` shorthand.  Union and record declarations
(`(union ...)`, `(record ...)`) are parsed as `CallExpr` nodes by the
self-hosted parser; `emit-top-level-form` returns `nil` for those,
so `emit-program-loop` skips them.

The four new files therefore contribute:
- `token.tw`: 0 (one `(union ...)`, one `(record ...)`)
- `ast.tw`: 0 (one `(union ...)`)
- `iir-types.tw`: 0 (one `(union ...)`, one `(record ...)`)
- `main.tw`: 2 (`(define (main) ...)` + `(define (self-compile-all dir) ...)`)

## Updated function counts

| File | Size (chars) | Functions | Notes |
|------|-------------|----------|-------|
| `span.tw`        |  2 426 |  2 | make-span, dummy-span |
| `token.tw`       |  2 782 |  0 | 0-fn union module ← TW05-M |
| `diagnostic.tw`  |  2 446 |  3 | make-error, make-warning, make-info |
| `ast.tw`         |  3 817 |  0 | 0-fn union module ← TW05-M |
| `iir-types.tw`   |  3 073 |  0 | 0-fn union module ← TW05-M |
| `iir-builder.tw` |  6 278 |  8 | new-builder … finalise-builder |
| `lexer.tw`       |  8 593 | 25 | lex-digit? … lex-source |
| `cst-parser.tw`  | 29 122 | 69 | cst-match-TkLParen … cst-parse-program |
| `parser.tw`      | 19 708 | 29 | parse-expr … parse-program |
| `emit.tw`        | 22 697 | 35 | emit-const … emit-program |
| `main.tw`        | 11 699 |  2 | main, self-compile-all ← TW05-M |
| **Total**        |112 641 |**173** | |

## Stack and VM limits — no changes needed

The largest single file is still `cst-parser.tw` at 29 122 chars.
All per-file measurements from TW05-L remain valid:

| Constant | Value | TW05-L rationale |
|----------|-------|-----------------|
| `MAX_DISPATCH_DEPTH` | 131 072 | cst-parser.tw → ~65 K frames, 2× headroom |
| `MAX_INSTRUCTIONS_PER_RUN` | 2²⁵ (32 M) | TW05-L 7-file total ~8.2 M |

Additional instruction cost for TW05-M:
- New chars: 2 782 + 3 817 + 3 073 + 11 699 = 21 371
- Additional lex instructions: 21 371 × 90 ≈ 1.92 M
- New grand total: ~8.2 M + 1.92 M ≈ **10.1 M** << 32 M ✓

No VM constant changes are needed.

## Changes

### `code/twig/compiler/main.tw`

`self-compile-all` extended with four additional `host/read_file` + pipeline
calls for `token.tw`, `ast.tw`, `iir-types.tw`, and `main.tw`.  The return
value changes from 171 → **173**.

Header comment updated to document TW05-M scope.

### `code/packages/rust/twig-module-driver/src/lib.rs`

#### Existing `self_compile_all_returns_171` renames

The following tests are updated to expect **173**:
- `tw05j_tests::self_compile_all_returns_171` → `self_compile_all_returns_173`
- `tw05k_tests::self_compile_all_returns_171` → `self_compile_all_returns_173`
- `tw05l_tests::self_compile_all_returns_171` → `self_compile_all_returns_173`

#### `mod tw05m_tests` (new)

7 integration tests:

| Test | File | Expected |
|------|------|---------|
| `self_compile_token_tw_returns_0` | `token.tw` (2 782 chars) | 0 |
| `self_compile_ast_tw_returns_0` | `ast.tw` (3 817 chars) | 0 |
| `self_compile_iir_types_tw_returns_0` | `iir-types.tw` (3 073 chars) | 0 |
| `self_compile_main_tw_returns_2` | `main.tw` (11 699 chars) | 2 |
| `self_compile_all_returns_173` | all 11 files via `self-compile-all` | 173 |
| `self_compile_tw05l_modules_regression` | TW05-L 7-file sum (independent) | 171 |
| `existing_main_still_returns_2_after_tw05m` | `(main)` | 2 |

### `code/packages/rust/twig-module-driver/Cargo.toml`

Version: 0.11.0 → 0.12.0

### `code/packages/rust/twig-module-driver/CHANGELOG.md`

Prepend `## [0.12.0]` entry.

## Acceptance criteria

- `(self-compile-all dir)` returns 173.
- `cargo test -p twig-module-driver --lib -- tw05m` — all 7 tests pass.
- `cargo test -p twig-module-driver --lib -- tw05l` — updated `self_compile_all_returns_173` passes.
- `cargo test -p twig-module-driver --lib -- tw05k` — updated `self_compile_all_returns_173` passes.
- `cargo test -p twig-module-driver --lib -- tw05j` — updated `self_compile_all_returns_173` passes.
- `cargo build --workspace` — clean build.

## Roadmap: what remains after LANG67

| Milestone | Scope |
|-----------|-------|
| TW05-G | Fixed-point: stage1 IIR = stage2 IIR for all modules |
| TW05-H | Strict mode: all modules `(typed strict)` |
