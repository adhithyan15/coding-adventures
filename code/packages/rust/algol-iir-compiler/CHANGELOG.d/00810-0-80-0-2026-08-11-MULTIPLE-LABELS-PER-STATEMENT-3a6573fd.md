## 0.80.0 — 2026-08-11 — multiple labels per statement

The compiler now pre-registers and emits every label prefix attached to one
statement. Each stable IIR label points at the same instruction position, so
forward and backward gotos may use any of the source labels while existing
block shadowing and duplicate checks remain intact.

