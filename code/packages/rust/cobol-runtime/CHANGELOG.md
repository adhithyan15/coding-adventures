# Changelog

## 0.1.0 — COBOL runtime, execution spine (PL08)

- `run_cobol(source) -> Result<String, RuntimeError>`: parse (via cobol-parser),
  lower the CST to a typed model, build a PICTURE-typed data model, execute, and
  return the captured `DISPLAY` output. I/O is captured (pure, testable).
- **Data model**: PICTURE parsing for unsigned numeric-display (`9`/`V`) and
  character (`X`/`A`) with `(n)` repetition; the item tree from level numbers
  (`01` groups, `02+` subordinates, `77` standalone); `VALUE` initialisation;
  figurative `ZERO`/`SPACE`.
- **MOVE** with exact COBOL receiving rules — numeric: decimal-aligned,
  integer right-justified/zero-filled/high-order-truncated, fraction
  left-justified/zero-filled/low-order-truncated; character: left-justified,
  space-padded/right-truncated.
- **DISPLAY** concatenates operand images with no separator; numeric items show
  raw stored digits (no implied decimal point). **STOP RUN**; paragraph
  fall-through.
- Honest scoping: signed numerics, editing pictures, `USAGE COMP`/`COMP-3`,
  group `MOVE`, name qualification, and every verb beyond `MOVE`/`DISPLAY`/`STOP
  RUN` return a descriptive `RuntimeError`. Roadmap in PL08.
