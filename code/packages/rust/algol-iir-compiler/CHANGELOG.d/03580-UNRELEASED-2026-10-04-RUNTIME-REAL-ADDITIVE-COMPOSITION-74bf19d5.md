## 0.358.0 - Runtime real additive composition

Runtime-real formatter provenance now crosses addition and subtraction when
every operand is independently runtime-real or a finite static numeric
expression. Multiplication, division, and powers remain conservatively gated.
