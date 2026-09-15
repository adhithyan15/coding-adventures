## 0.33.0 — 2026-06-10 — McCarthy → **BEAM ATOM/EQ/COND** on a real `erl` (LANG77 / W10, F3–F5)

McCarthy's predicates run on the BEAM (`iir-to-beam` 0.5.0 lowers `pair?`→
`is_nonempty_list`, `equal?`→`is_eq_exact`, `not`→`x==0`; `COND`→`jmp_if`). The
`compile_source_to_beam` pipeline is unchanged — the predicates flow through the
existing `lower_heap_builtins` + concretize path. New `tests/beam_predicates.rs`:
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(COND …)`→100/200, on a real `erl`.

