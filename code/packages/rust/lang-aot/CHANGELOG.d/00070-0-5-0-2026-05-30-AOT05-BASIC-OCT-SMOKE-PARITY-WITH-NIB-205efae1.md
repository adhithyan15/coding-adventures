## 0.5.0 — 2026-05-30 (AOT05 — BASIC + Oct smoke parity with Nib)

### Added — 6 new end-to-end smoke tests (BASIC + Oct)

Brings BASIC and Oct's lang-aot smoke coverage from 2 tests each to
5 tests each, matching Nib's breadth.  Closes task #32 from the
multi-language tooling parity work.

#### Oct — 3 new tests (was 2)

- `end_to_end_oct_if_else_exits_zero` — `if x == 0 { x = 1; } else
  { x = 2; }` compiles, links, runs successfully.  Exercises typed
  `cmp_eq` + `jmp_if_false` + `mov` + `jmp` + `label` through native
  codegen.
- `end_to_end_oct_while_loop_exits_zero` — `while n < 10 { n = n + 1; }`
  compiles and runs to completion.  Exercises backward `jmp` (the
  AOT chain's branch-distance encoding) and the typed `cmp_lt`+`add`
  loop body.
- `end_to_end_oct_cross_fn_chain_exits_zero` — `add_one(add_one(8))`
  chains two cross-fn calls through the typed-argument reloc path.

#### BASIC — 3 new tests (was 2)

- `end_to_end_basic_arith_chain_prints_42` — `A + B + C` printed.
  Exercises multiple typed `add` ops through the AOT pipeline.
- `end_to_end_basic_if_then_prints_1` — `IF A > 5 THEN 100` takes
  the then branch, prints 1.  Exercises typed `cmp_gt` +
  `jmp_if_*` with line-label resolution.
- `end_to_end_basic_goto_prints_1` — `GOTO 100` skips the
  assignment on line 30, prints A's original value.  Exercises
  forward unconditional branch resolution.

### Coverage parity

| Language | Smoke tests (before) | Smoke tests (now) |
|---|---|---|
| Twig | 1 | 1 |
| Nib | 5 | 5 |
| Brainfuck | 1 | 1 |
| **BASIC** | **2** | **5** |
| **Oct** | **2** | **5** |

### Tests

All 17 smoke tests pass on the local host platform.  Each test is
gated to its host OS (`#[cfg(target_os = ...)]`) so CI runners only
execute the tests appropriate to their platform.

