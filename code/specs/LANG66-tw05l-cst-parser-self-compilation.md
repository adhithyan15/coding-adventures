# LANG66 — TW05-L: CST-Parser Self-Compilation (cst-parser.tw)

## Overview

TW05-L extends the `self-compile-all` pipeline from six files (TW05-K, 102
total functions) to **seven** files by adding `cst-parser.tw` (69 functions,
29 122 chars).

After this milestone `self-compile-all` returns **171** total emitted function
definitions.

## Context

TW05-K (LANG65) proved the pipeline on all non-generated compiler modules.
`cst-parser.tw` is the only generated module (produced by `grammar-tools`
from `twig.grammar`) and is the largest file in the compiler source tree.
Adding it is the final step before all modules are covered.

## Updated function counts

| File | Size (chars) | Functions | Names (first/last) |
|------|-------------|----------|--------------------|
| `span.tw`        |  2 426 |  2 | `make-span`, `dummy-span` |
| `diagnostic.tw`  |  2 446 |  3 | `make-error`, `make-warning`, `make-info` |
| `iir-builder.tw` |  6 278 |  8 | `iirbuilder-with-instrs` … `finalise-builder` |
| `lexer.tw`       |  8 593 | 25 | `lex-digit?` … `lex-source` |
| `parser.tw`      | 19 708 | 29 | `parse-expr` … `parse-program` |
| `emit.tw`        | 22 697 | 35 | `emit-const` … `emit-program` |
| `cst-parser.tw`  | 29 122 | 69 | `cst-match-TkLParen` … `cst-parse-program` | ← TW05-L |
| **Total**        |        | **171** | |

## Stack requirement and MAX_DISPATCH_DEPTH

`cst-parser.tw` is 29 122 chars — the largest file in the corpus.  `lex-loop`
recurses once per source character.

**Native stack budget:**
- Empirical debug-mode frame size: ~60 KiB (same as prior files).
- Peak stack: 29 122 × 60 KiB = ~1.75 GiB.
- `run_in_xlarge_stack` (3 GiB) gives ~1.7× headroom.  Physical pages are
  lazy-allocated by the OS, so physical RSS ≈ actual peak, not 3 GiB.

**VM dispatch depth:**

The self-hosted `lex-loop` is NOT tail-call optimised.  Each source character
adds ~2–3 dispatch frames: `lex-loop` + `cond-dispatch` + optional scanner
helper.  Empirically, 29 122 chars × ~2.25 frames/char ≈ 65 524 frames,
which **exceeds** the 65 536 ceiling set in LANG64.

`MAX_DISPATCH_DEPTH` must be bumped from 65 536 to 131 072 (2^17) in
`twig-vm`.  131 072 gives ~2× headroom over the 65 524 measured peak.

| Milestone | File | Chars | Est. depth | Limit |
|-----------|------|-------|-----------|-------|
| TW05-K | `emit.tw` | 22 697 | ~51 K | 65 536 ✓ |
| TW05-L | `cst-parser.tw` | 29 122 | ~66 K | 131 072 ✓ |

## Instruction budget

| Source | Chars | Lex instrs (~30/char) |
|--------|-------|-----------------------|
| TW05-K corpus (6 files) | 62 148 | ≈ 1.86 M |
| cst-parser.tw           | 29 122 | ≈ 0.87 M |
| **Total lex**           | 91 270 | ≈ **2.74 M** |
| Parse + emit overhead   |        | ≈ 0.70 M |
| **Grand total**         |        | ≈ **3.44 M** |

`MAX_INSTRUCTIONS_PER_RUN` must also be bumped: the self-hosted lex-loop
actually executes ~90 IIR instructions per character (not 30 as assumed
earlier), so 91 270 chars × 90 ≈ 8.2 M > 8 M ceiling.

`MAX_INSTRUCTIONS_PER_RUN` bumped from 2²³ (8 M) to 2²⁵ (32 M). 32 M gives
~4× headroom over the measured 8.2 M peak.

## Changes

### `code/packages/rust/twig-vm/src/dispatch.rs` + `Cargo.toml` + `CHANGELOG.md`

`MAX_DISPATCH_DEPTH` bumped from 65 536 to 131 072.  Version: 0.17.0 → 0.18.0.

### `code/twig/compiler/main.tw`

`self-compile-all` extended to read `cst-parser.tw`, compile it, and add its
69 functions to the sum.  Return value changes from 102 → **171**.

Header comment updated to document TW05-L scope.

### `code/packages/rust/twig-module-driver/src/lib.rs`

#### `tw05j_tests::self_compile_all_returns_102` rename

Renamed to `self_compile_all_returns_171` and updated to expect 171 (the new
`self-compile-all` total after TW05-L).

#### `tw05k_tests::self_compile_all_returns_102` rename

Same: renamed to `self_compile_all_returns_171`, expects 171.

#### `mod tw05l_tests` (new)

4 integration tests:

| Test | File | Expected |
|------|------|---------|
| `self_compile_cst_parser_from_disk` | `cst-parser.tw` (29 122 chars) | 69 |
| `self_compile_all_returns_171` | all 7 files via `self-compile-all` | 171 |
| `self_compile_tw05k_modules_regression` | TW05-K 6-file sum (independent) | 102 |
| `existing_main_still_returns_2_after_tw05l` | `(main)` | 2 |

The regression test invokes the pipeline directly on the six TW05-K files
(without `cst-parser.tw`) to confirm prior counts are unchanged.

### `code/packages/rust/twig-module-driver/Cargo.toml`

Version: 0.10.0 → 0.11.0

### `code/packages/rust/twig-module-driver/CHANGELOG.md`

Prepend `## [0.11.0]` entry.

## Acceptance criteria

- `(self-compile-all dir)` returns 171.
- `cargo test -p twig-module-driver --lib -- tw05l` — all 4 tests pass.
- `cargo test -p twig-module-driver --lib -- tw05k` — updated tests pass
  (both `self_compile_all_returns_171`).
- `cargo test -p twig-module-driver --lib -- tw05j` — updated test passes
  (`self_compile_all_returns_171`).
- `cargo build --workspace` — clean build.

## Roadmap: what remains after LANG66

| Milestone | Scope |
|-----------|-------|
| LANG67 (TW05-M) | All 11 modules including token/ast/iir-types (0-fn union modules) |
| TW05-G | Fixed-point: stage1 IIR = stage2 IIR for all modules |
| TW05-H | Strict mode: all modules `(typed strict)` |
