# LANG64 — TW05-J: Multi-Module Self-Compilation via host/read_file

**Status:** Implemented
**Branch:** `feat/lang64-tw05j-multi-module-self-compilation`
**Depends on:** LANG62 (TW05-I: first self-compilation check), LANG63 (grammar-driven Twig parser)

---

## Overview

TW05-J makes two changes:

1. **Bump `MAX_DISPATCH_DEPTH`** in `twig-vm` from 4096 to 65536.
   The lex-loop in `compiler/lexer.tw` recurses once per character of input.
   The files compiled by TW05-J include `iir-builder.tw` (6278 chars) and
   `lexer.tw` (8593 chars), both of which exceed the old 4096-frame limit.
   65536 gives ample headroom for all current compiler modules.

2. **Multi-module self-compilation**: `main.tw` gains a new exported function
   `(self-compile-all dir)` that reads four real `.tw` files from disk via
   `host/read_file` and compiles each through the full lex → parse →
   `emit-program` pipeline, returning the sum of emitted function counts.

Six new integration tests in `twig-module-driver` (`tw05j_tests`) validate
the multi-module self-compilation.

---

## Motivation

TW05-I (LANG62) proved the pipeline works on a short, in-memory source string.
TW05-J proves it works on real files:

- **Real files have comments** — `lex-source` must skip `;`-to-EOL comment
  lines.  All four files have extensive literate-style comments.
- **Real files are large** — `lexer.tw` is 8593 chars, more than twice the
  old 4096-frame limit.  The `lex-loop` recurse-per-character design means
  the frame depth equals the source character count.
- **`host/read_file` exercises the host I/O path** — the VM's host call
  mechanism is used in production for the first time in a test.

---

## MAX_DISPATCH_DEPTH Bottleneck and Solution

### Problem

The Twig VM's `dispatch` function is recursive: each `Call` opcode adds a
Rust stack frame.  `lex-loop` is a tail-recursive function that calls itself
once per character:

```
(define (lex-loop src pos tokens)
  (if (>= pos (string-length src))
      (reverse tokens)
      (lex-loop src (+ pos 1) (cons (lex-one src pos) tokens))))
```

For an N-character source file, `lex-loop` requires N nested dispatch frames.

### Old limit (4096) — set in LANG62

- `span.tw` (2426 chars): OK (2426 < 4096)
- `diagnostic.tw` (2446 chars): OK (2446 < 4096)
- `iir-builder.tw` (6278 chars): **FAIL** (6278 > 4096) → `Run(DepthExceeded)`
- `lexer.tw` (8593 chars): **FAIL** (8593 > 4096) → `Run(DepthExceeded)`

### New limit (65536) — set in LANG64

All four files lex successfully.  The integration tests use a 64 MiB Rust
thread stack; 65536 frames at ~200-500 bytes each = 13-33 MB, well within
the 64 MiB budget.

---

## Module Function Counts

| File | Size (chars) | Functions | Function Names |
|---|---|---|---|
| `span.tw` | 2426 | 2 | `make-span`, `dummy-span` |
| `diagnostic.tw` | 2446 | 3 | `make-error`, `make-warning`, `make-info` |
| `iir-builder.tw` | 6278 | 8 | `iirbuilder-with-instrs`, `iirbuilder-with-reg-count`, `iirbuilder-with-label-count`, `new-builder`, `alloc-slot`, `alloc-label`, `append-instr`, `finalise-builder` |
| `lexer.tw` | 8593 | 25 | `lex-digit?`, `lex-whitespace?`, `lex-skip-whitespace`, `lex-skip-comment`, `lex-read-integer`, `lex-read-symbol`, `lex-read-string`, `lex-one`, `lex-loop`, `lex-source`, and 15 more |
| **Total** | | **38** | |

Note: `ast.tw`, `token.tw`, and `iir-types.tw` contain only `union`, `record`,
and `module` declarations — no `define` forms.  These produce 0 emitted
functions and are NOT included in the count.

---

## Files Changed

### `code/packages/rust/twig-vm/src/dispatch.rs`

- `MAX_DISPATCH_DEPTH`: `4096` → `65536`
- Updated surrounding comment to document LANG64 history entry

### `code/packages/rust/twig-vm/Cargo.toml`

- Version: `0.16.0` → `0.17.0`
- Description updated to include LANG64

### `code/packages/rust/twig-vm/CHANGELOG.md`

- New `## [0.17.0]` entry documenting the `MAX_DISPATCH_DEPTH` bump and why

### `code/twig/compiler/main.tw`

- Module header updated: `(export main self-compile-all)`
- File header comment updated to describe TW05-J and `self-compile-all`
- New function `(self-compile-all dir)` added after `(main)`

### `code/packages/rust/twig-module-driver/src/lib.rs`

- New `#[cfg(test)] mod tw05j_tests` with 6 integration tests

### `code/packages/rust/twig-module-driver/Cargo.toml`

- Version: `0.8.0` → `0.9.0`
- Description updated to include LANG64

### `code/packages/rust/twig-module-driver/CHANGELOG.md`

- New `## [0.9.0]` entry documenting all tw05j tests

---

## Tests Added (`tw05j_tests`)

| Test | Expected Result | What it verifies |
|------|----------------|-----------------|
| `self_compile_all_returns_38` | `38` | `self-compile-all` on all 4 files → 2+3+8+25 = 38 |
| `self_compile_span_from_disk` | `2` | Full `span.tw` (with comments) via `host/read_file` → 2 functions |
| `self_compile_diagnostic_from_disk` | `3` | Full `diagnostic.tw` via `host/read_file` → 3 functions |
| `self_compile_iir_builder_from_disk` | `8` | `iir-builder.tw` (6278 chars) via `host/read_file` → 8 functions |
| `self_compile_lexer_from_disk` | `25` | `lexer.tw` (8593 chars) via `host/read_file` → 25 functions |
| `existing_main_still_returns_2` | `2` | TW05-I regression: `(main)` still returns 2 after bump |

### Test architecture

Each disk-read test writes a `main-test.tw` file to a temp directory that:
1. Declares `(module compiler/main-test ...)` (matching the file path)
2. Imports only what it needs (`compiler/lexer`, `compiler/parser`, `compiler/emit`)
3. Defines `(main)` to call `host/read_file` then pipe through lex → parse → emit-program

The `self_compile_all_returns_38` test imports `compiler/main` (which re-exports
`self-compile-all`) and passes the actual compiler source directory path.

---

## Implementation Notes

1. `(main)` in `main.tw` is NOT changed — it still returns 2.
2. The `copy_all_tw_modules` helper in `tw05j_tests` copies 11 modules:
   all 10 from `tw05i_tests` plus `main` itself (needed for the `self_compile_all_returns_38` test).
3. Tests that call pipeline functions directly (not via `self-compile-all`)
   import `compiler/lexer`, `compiler/parser`, `compiler/emit` individually
   — they do NOT import `compiler/main` to keep the dependency minimal.
4. The `make_tempdir` helper in `tw05j_tests` adds a nanosecond nonce to
   prevent collisions when tests run in parallel.
