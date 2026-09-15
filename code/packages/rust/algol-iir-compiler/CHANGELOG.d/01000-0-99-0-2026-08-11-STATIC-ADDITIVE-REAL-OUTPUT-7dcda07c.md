## 0.99.0 — 2026-08-11 — static additive real output

Literal-only real addition and subtraction now evaluate at compile time and
print their finite result through the shared string path. Direct literals still
preserve source spelling, while multiplication, division, non-finite results,
and any runtime operand remain outside this bounded formatter-free step.

