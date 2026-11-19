# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-038c CI fix)

- `merge`: refresh a subset of the committed rows from a fresh `run` without hand-editing
  (VOS30 section 9.2), with unit tests. It keeps the file's original provenance and records each refresh in a `merges` list (checked by `validate`, named in the table banner); it refuses an invalid or dirty fresh run, a non-ancestor revision, duplicate ids, fresh rows outside the subset (unless `--ignore-outside`), an unknown `--where` field, and a `dirty_tree` that is not a JSON boolean; reads are capped and the write is atomic; used to refresh the E1 `vos-sched-fp-bitmap` rows after
  VOS-038c changed that crate's generated document (VOS-D103).

### Added (VOS-028b, VOS30 section 9)

- The solver and enumeration capacity probe: `list`, `run --tier quick|full`, `check`, `table
  [--check|--write]`, `decide`, and the hidden `row` child.
- A probe-private minimal symbolic RV32I stepper (`cpu`) over `bit-blaster`, reading instructions
  through `riscv-simulator::decode` after assembling them with `riscv-encoder`; two spellings of
  every ALU operation (`sem`), the seeded generator with the two liveness rules of guard G2 (`prog`).
- Experiments A1, A2, B1, B2, B3, B4, C1, C2, D1 to D4 (solver rows: build, G1/G2 guards, CNF,
  solve with DRAT, check under `Limits::default()`, a G3 control that must be SAT and replay on the
  concrete simulator and an independent evaluator) and E1, E2 (enumeration: the real
  `vos-sched-fp-bitmap` and the fixture `vos-probe-ready` harnesses, and the generator's budget
  refusals).
- Resource limits: a 1,000,000-conflict cap with proof logging, a 120 s wall timeout as a loop of
  20,000-conflict (and 4 x 10^8 watch-visit) slices, a process-group kill, a 2 GiB resident-set cap
  polled every 50 ms, `/usr/bin/time` peak-RSS parsing that fails the row when it cannot parse.
- The results file (`code/specs/data/vos30-capacity-results.json`, schema 2), `check` (schema and
  guard rules, quick-tier re-run), the generated table with its drift check, and the section 8.3
  decision procedure.

### Results (VOS-028b)

- The committed full-tier run: 1,625 rows (14 experiments, five seeds per knob), the generated
  table in VOS30 section 9.8 and the decision of section 8.5: **row 1, Infeasible: re-plan X3**.
  `check` re-runs its 85 quick-tier rows on every BUILD (about 14 s of the probe's own runs).
- Tests: 53 unit tests and 33 integration tests (shapes, results logic, the command line on the real
  binary and real child processes); 94% line coverage of the program.

### Fixed during VOS-028b

- `dirty_tree` is gathered before the rows run (a first full run recorded it after, while the tree
  was being edited, and was discarded); E2 compares the generator with its own numbers instead of
  a closed form that is wrong for one-byte binders; a chain whose checker accepted nothing no
  longer reports a check work of 0.
