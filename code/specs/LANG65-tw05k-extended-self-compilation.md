# LANG65 — TW05-K: Extended Self-Compilation (parser.tw + emit.tw)

## Overview

TW05-K extends the `self-compile-all` pipeline introduced in TW05-J (LANG64) to
cover the two largest and most semantically rich compiler modules: `parser.tw`
and `emit.tw`.

After this milestone `self-compile-all` compiles **six** real `.tw` files from
disk and returns **102** total emitted function definitions.

## Context

TW05-J (LANG64) proved the self-hosted `lex → parse → emit-program` pipeline on
four data-structure modules (span, diagnostic, iir-builder, lexer) and returned
38 total functions.  The pipeline was not yet exercised on the compiler's own
*compilation* modules — the files that contain non-trivial control flow, deep
recursion, and complex pattern-dispatch logic.

TW05-K adds those two modules:

| Module | Size (chars) | New functions | Depth required |
|--------|-------------|--------------|----------------|
| `parser.tw`  | 19 708 | 29 | ≤ 19 708 |
| `emit.tw`    | 22 697 | 35 | ≤ 22 697 |

Both files are well within the `MAX_DISPATCH_DEPTH = 65536` ceiling set in
LANG64.  In debug builds, `lex-loop` frames are ~60 KiB each; lexing `emit.tw`
(the larger of the two) requires up to ~22 697 frames × 60 KiB ≈ 1.3 GiB of
native stack.  Integration tests therefore use a `run_in_xlarge_stack` helper
(3 GiB) introduced in this milestone.

## Updated function counts

| File | Size | Functions | Names (first/last) |
|------|------|----------|--------------------|
| `span.tw`        | 2 426 |  2 | `make-span`, `dummy-span` |
| `diagnostic.tw`  | 2 446 |  3 | `make-error`, `make-warning`, `make-info` |
| `iir-builder.tw` | 6 278 |  8 | `iirbuilder-with-instrs` … `finalise-builder` |
| `lexer.tw`       | 8 593 | 25 | `lex-digit?` … `lex-source` |
| `parser.tw`      | 19 708 | 29 | `parse-expr` … `parse-program` |
| `emit.tw`        | 22 697 | 35 | `emit-const` … `emit-program` |
| **Total** | | **102** | |

## Changes

### `code/twig/compiler/main.tw`

`self-compile-all` extended to read `parser.tw` and `emit.tw` and sum their
function counts alongside the existing four files.  Return value changes from
38 → **102**.

Updated export comment explains TW05-K scope and new total.

### `code/packages/rust/twig-module-driver/src/lib.rs`

#### `run_in_xlarge_stack` helper (new)

3 GiB thread stack for files up to ~35 000 chars in debug builds.
Mirrors `run_in_large_stack` (768 MiB) but sized for the larger TW05-K files.

Budget rationale:
- Empirical debug-mode frame size: 30–60 KiB.
- Largest TW05-K file: `emit.tw` at 22 697 chars.
- Worst case: 22 697 × 60 KiB = ~1.33 GiB.
- 3 GiB gives ≥ 2× headroom; physical pages are allocated lazily by the OS.

#### tw05j regression update

`self_compile_all_returns_38` renamed to `self_compile_all_returns_102` and
updated to expect 102.  Stack upgraded to `run_in_xlarge_stack` since the new
`self-compile-all` lexes `emit.tw` (22 697 chars).

#### `mod tw05k_tests` (new)

5 integration tests:

| Test | File | Expected |
|------|------|---------|
| `self_compile_all_returns_102` | all 6 files | 102 |
| `self_compile_parser_from_disk` | `parser.tw` | 29 |
| `self_compile_emit_from_disk` | `emit.tw` | 35 |
| `self_compile_all_tw05j_modules_regression` | original 4 files only | 38 |
| `existing_main_still_returns_2_after_tw05k` | `(main)` | 2 |

The regression test re-invokes the individual `lex → parse → emit-program`
pipeline on the four original TW05-J files to confirm they still produce correct
counts after the stack and depth changes.

### `code/packages/rust/twig-module-driver/Cargo.toml`

Version: 0.9.0 → 0.10.0

### `code/packages/rust/twig-module-driver/CHANGELOG.md`

Prepend `## [0.10.0]` entry.

## Instruction budget

`self-compile-all` compiles six files sequentially.  Budget estimate:
- `lex-loop` executes ≈ 30 IIR instructions per source character.
- Total chars: 2 426 + 2 446 + 6 278 + 8 593 + 19 708 + 22 697 = 62 148.
- Lex instructions: 62 148 × 30 ≈ 1.86 M.
- Parse + emit overhead ≈ 0.5 M.
- Total: ≈ 2.4 M instructions.
- `MAX_INSTRUCTIONS_PER_RUN = 2²³ = 8 M` — 3.3× headroom.  No change needed.

## Acceptance criteria

- `(self-compile-all dir)` returns 102.
- `cargo test -p twig-module-driver --lib -- tw05k` — all 5 tests pass.
- `cargo test -p twig-module-driver --lib -- tw05j` — updated test passes (expects 102).
- `cargo build --workspace` — clean build.

## Roadmap: what remains after LANG65

| Milestone | Scope | Remaining modules |
|-----------|-------|-------------------|
| LANG66 (TW05-L) | Add cst-parser.tw (69 fns, 29 122 chars) to self-compile-all | 1 |
| LANG67 (TW05-M) | All 11 modules including token/ast/iir-types (0-fn union modules) | main.tw |
| TW05-G | Fixed-point: stage1 IIR = stage2 IIR for all modules | — |
| TW05-H | Strict mode: all modules `(typed strict)` | — |
