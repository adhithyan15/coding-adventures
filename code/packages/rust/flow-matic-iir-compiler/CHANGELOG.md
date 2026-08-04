# Changelog

## 0.3.0 — READ-ITEM / END OF DATA record loops (PL09 D4)

- `READ-ITEM <handle>` reads the next record from stdin into the file's fields:
  the `input_more` builtin peeks whether a record remains (the EOF-aware read the
  plain `input_i64` lacks — PL09 D4), sets the shared `_eof` flag, and reads each
  field with `input_i64` (skipped at end-of-input, so fields keep their values).
- `IF END OF DATA GO TO OPERATION n` now compiles — a conditional jump on the
  `_eof` flag set by the most recent `READ-ITEM` (zero-initialised at entry, so a
  lone one reads a defined false).
- Completes the record-processing loop: READ → `IF END OF DATA` → process →
  `WRITE-ITEM` → `JUMP` back. Run-verified on the VM/JIT with a stdin stream
  ([5,3] → `5\n3\n`; multi-field; empty input → no output). `input_more` is a
  VM/JIT builtin for now; AOT-column EOF is a follow-up (D4).
- `TRANSFER`, `TEST`/`REWIND`/`CLOSE-OUT` remain a clean `Unsupported`.

## 0.2.0 — WRITE-ITEM → observable stdout (PL09 D4)

- `WRITE-ITEM <handle>` now writes the file's record to stdout: its fields
  (those qualified by the handle's letter, `FILE-C` → `C`), space-separated,
  then a newline. Numeric fields render through two synthesized recursive
  digit-print helpers (`__fm_print_int` → `__fm_print_mag`) over `putchar` — the
  same portable print substrate Dartmouth BASIC uses (no host print builtin).
- First observable FLOW-MATIC output. Accepted by the wasm/jvm/clr validators
  (BEAM excluded for `putchar`, as BASIC's print programs are) and run-verified
  on the JIT (`WRITE-ITEM` of a zero record prints `0`; multi-digit and negative
  covered by a direct digit-print unit test).
- `READ-ITEM` still needs the EOF-aware read (D4) and remains a clean
  `CompileError::Unsupported`.

## 0.1.0 — control-flow + scalar-field slice (PL09)

- First FLOW-MATIC → IIR compiler. `compile_source(&str, &str) -> IIRModule`
  lowers a parsed FLOW-MATIC program to a single `main` returning an `i64` exit
  code, ready for the LANG VM AOT chain.
- Lowers the control-flow + scalar-field slice: operations → `label op_n`;
  `COMPARE a WITH b` → three-way `cmp_gt`/`cmp_eq`/`cmp_lt` flags;
  `IF GREATER/EQUAL/LESS GO TO OPERATION n` → `jmp_if_true flag, op_n`;
  `OTHERWISE`/`JUMP TO OPERATION n` → `jmp op_n`; `MOVE field TO field` → `mov`;
  `STOP` → `ret 0`; `INPUT`/`OUTPUT`/`HSP` file declarations → no-op. Each
  file-qualified field (`PRODUCT-NO (A)`) is an `i64` register zeroed at entry.
- Record/file I/O (`READ-ITEM`/`WRITE-ITEM`, `TRANSFER`, `TEST`/`REWIND`/
  `CLOSE-OUT`, the `END OF DATA` loop) is a later rung — a clean
  `CompileError::Unsupported`, never wrong output.
- Verified: unit tests over the IIR shape + `module.validate()`; `backend_compat`
  proves acceptance by the wasm/jvm/clr/beam validators; `jit_e2e` runs programs
  through the generic VM/JIT (a miscompiled compare/branch would hang on an
  infinite `JUMP` loop instead of returning 0). Wired into `lang-aot` as
  `Language::FlowMatic`.
