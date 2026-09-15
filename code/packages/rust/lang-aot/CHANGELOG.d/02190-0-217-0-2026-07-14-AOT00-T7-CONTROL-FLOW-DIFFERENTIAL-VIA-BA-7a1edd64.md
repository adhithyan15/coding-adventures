## 0.217.0 - 2026-07-14 (AOT00 T7 — control-flow differential via BASIC IF/THEN)

Adds `t7_differential_random_basic_conditionals_agree` to `tests/lang_matrix.rs` —
the third generative differential (AOT00 T7), the first to exercise **control
flow**. The two arithmetic slices test only straight-line evaluation; this
generates `IF <a> <relop> <b> THEN … / … / GOTO … / … / END` programs (`<a..d>` the
`+ - *` expression trees, `<relop>` over all six `= <> < > <= >=`) that branch
between two `PRINT` arms, so the printed `i64` witnesses both the comparison result
and that the right branch was taken. This exercises the **comparison ops +
conditional branch + `GOTO`** codegen — where cross-backend disagreements are most
likely (boolean representation / branch polarity, the class of the E6d-6 boxed-bool
`jmp_if_false` bug). 264 full-value agreements over 120 branch programs locally;
fast in-process engines (WASM/JIT) every program, toolchain engines sampled. Spec
§3c.

