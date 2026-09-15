## 0.220.0 - 2026-07-14 (AOT00 T7 — loop differential via BASIC FOR/NEXT)

Adds `t7_differential_random_basic_loops_agree` to `tests/lang_matrix.rs` — the
fourth generative differential (AOT00 T7), the first to exercise a **loop
back-edge**. The three prior slices (arithmetic exit-code, arithmetic full-value,
control-flow `IF`) never touch a loop; this generates
`S := 0; FOR I = 1 TO n { S := S + <body(I)> }; PRINT S` accumulator programs
(`<body>` a `+ - *` tree over the counter `I` + small literals, trip count
`n ∈ 2..=6`, depth-capped so the `i64` sum can't overflow) and compares the printed
result across every engine — exercising the loop header/latch, counter increment +
bound test, and a mutated accumulator across iterations (a classic cross-backend
divergence source). 264 full-value agreements over 120 loop programs locally.
Spec §3d. `lang-aot` → 0.220.0 (0.219.0 is the concurrent E6d-list PR #8398).
