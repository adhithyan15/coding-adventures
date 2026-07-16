# Changelog

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
