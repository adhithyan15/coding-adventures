## 0.3.0 — 2026-05-20 (PL05 — Dartmouth BASIC end-to-end on LANG VM)

Dartmouth BASIC programs now compile end-to-end via the new
`dartmouth-basic-iir-compiler` crate.  `lang-aot foo.bas` produces a
native executable on Linux, Windows, and macOS — the same chain Twig,
Nib, and Brainfuck use.

**Wiring.**  The `Language::DartmouthBasic` arm in
`compile_source_to_iir` now calls
`dartmouth_basic_iir_compiler::compile_source` instead of returning
`UnsupportedLanguage`.  No other changes to the lang-aot surface —
the existing `compile_file_to_*_executable` entry points handle
BASIC transparently.

**V1 BASIC coverage.**  Integer-only programs with LET / PRINT /
INPUT / IF / GOTO / FOR / NEXT / END / REM.  GOSUB/RETURN, READ/
DATA, DIM/arrays, and DEF are deferred.  See
[`dartmouth-basic-iir-compiler/CHANGELOG.md`](../dartmouth-basic-iir-compiler/CHANGELOG.md)
for the full table.

**End-to-end smoke tests:**

- `end_to_end_basic_print_42_via_lang_aot` — `10 PRINT 42 / 20 END`
  exits cleanly and writes exactly `"42\n"`.
- `end_to_end_basic_for_loop_prints_1_2_3` — `FOR I = 1 TO 3 / PRINT
  I / NEXT I / END` writes exactly `"1\n2\n3\n"`.

Verified locally on Windows.

**Lib-test renamed.**  `dartmouth_basic_returns_clean_unsupported_error`
is gone; `dartmouth_basic_compiles_to_iir` asserts the new success
path.

