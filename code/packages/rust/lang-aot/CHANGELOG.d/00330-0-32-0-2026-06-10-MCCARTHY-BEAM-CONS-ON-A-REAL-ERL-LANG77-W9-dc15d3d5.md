## 0.32.0 — 2026-06-10 — McCarthy → **BEAM cons** on a real `erl` (LANG77 / W9b, F2)

McCarthy cons runs on the BEAM (Erlang VM) — using the **native Erlang-terms**
model, NOT the boxing structural pass the managed backends use. A cons cell is a
native list cell `[H|T]`; `car`/`cdr` are `hd`/`tl`; integers are native. Two
pipeline changes in `compile_source_to_beam`:
- Run `lower_heap_builtins` so `cons`/`car`/`cdr` become `alloc ref<LispyPair>` +
  `field_store`/`field_load`, which `iir-to-beam` already maps to `put_list` /
  `get_hd` / `get_tl`.
- Generalize `concretize_scalar_any_for_beam` to concretize `any`→`i64`
  **per-instruction in every function** (BEAM is dynamically typed; `i64` is the
  universal native-term placeholder), leaving `ref<LispyPair>` cons cells for the
  list lowering — previously it skipped any heap-using function wholesale.

Verified by RUNNING on a real `erl`: `(CAR (CONS 7 9))`→7, `(CDR (CONS 7 9))`→9,
nested→2, and `(CONS 7 9)`→`[7|9]` (a genuine Erlang list cell). New
`tests/beam_cons.rs`. (`iir-to-beam` is unchanged — it already had the list ops.)

