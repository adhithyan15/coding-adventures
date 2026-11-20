# Changelog

All notable changes to this package are documented here.

## [0.2.0] - Unreleased

### Added (VOS-028c2, VOS30 sections 9.2, 9.5, 9.6 and 9.12; no new experiment)

- **Chain budget (VOS-D095).** A split or chain (B3, D3, D4) is one row with a 600 s row budget;
  each obligation keeps its own 120 s (`exp::obligation_deadline`), and the parent's hard kill
  moves with the budget (600 s + 30 s control + 150 s slack). The row records `wall_s` (omitted
  when 120, so unchanged rows keep their bytes); `validate` accepts only the chain budget, and only
  on a chain experiment.
- **`merge --add`.** Appends fresh rows whose `(experiment, knob, mode, seed)` identities are all
  absent from the base, records `"kind": "add"`, refuses any overlap and every refusal of a
  refresh that still applies. A refresh keeps its entry's bytes (no `kind` key).
- **Provenance by tree id (VOS-D097 item 1).** New runs and merges record `trees` (the probe and
  its four solver crates); `check` prints a non-fatal `stale:` line when one differs from the
  working tree, then runs a **seeded spot check** (seed: SHA-256 of the file's own bytes; every
  full-tier non-E row; each re-run in a child process under `run`'s limits; fixed 60 s budget;
  rows over a 20 s recorded cost skipped and counted). **`check --full`** refuses unless every
  recorded tree equals the working tree, then re-runs every full-tier row.
- **Hardening (VOS-D097 items 2 to 5).** The kill checks `/bin/kill`'s status, falls back to
  `Child::kill` and bounds the wait at 5 s; the child runs behind the fixed `ulimit -t "$1"; shift;
  exec "$@"` wrapper with the limit and every other variable part as separate argv entries;
  `validate` requires every row's `(experiment, mode, knob)` to equal one of `knobs(exp)`, values
  included; child output is read through a 1 MiB cap; `--out` is written atomically; `--jobs` is
  clamped to 1..=64. All process code is `cfg(unix)`-safe; the Windows BUILD runs `check` in
  process and skips the sampled re-run, saying so.
- **Security-review fixes (VOS-028c2; VOS30 9.12; tests in `tests/review_fixes.rs`).**
  H1: `tier` must be `quick` or `full` and the plan's for the knob. H2: `full_plan_covered` and a
  stricter `quick_plan_covered` (no missing, extra or duplicate row; `seeds` equals the seeds
  present); the reviewed `FULL_PLAN_EXEMPT` list is empty. M1: `check --full` also compares
  `solve_ms`, `worst_step_solve_ms`, `check_ms` and `peak_rss_kb` within a two-sided 4x band and
  recomputes `green` from the re-run. M2: `check` returns before any re-run when validation or
  coverage fails, and `quick_specs` admits only plan-quick rows. M3: the spec and docs no longer
  claim the deterministic sample is unpredictable; `check --full` is the gate. M4: the spot
  check's cost class is the plan's (`cost::CHEAP`), never the recorded times, and `check` fails if
  fewer than 16 rows were verified. M5: a sampled child is killed at the remaining budget plus 15
  s and counts as not verified. L1: `check`, `table` and `decide` read through the 64 MiB cap
  (`merge` already did). L2: one atomic writer (same-directory `create_new` temporary, 0600,
  `fsync`, rename; refuses a symlink target) for `run --out`, `merge --out` and `table --write`.
  L3: the wrapper is `ulimit -t "$1" || exit 126; shift; exec "$@"`; the group kill is retried
  once before `Child::kill`. New: `check --spot-budget SECS`.
- Tests: `tests/hardening.rs` (negative controls for every clause above, on real child processes
  where the clause is about a process) and unit tests in `runner`, `row` and `cli`.

### Changed

- `run_child` and `run_rows` take the row budget from the experiment; `hard_limit_for(wall_s)`
  replaces the single constant (`hard_limit()` remains the 120 s row's).
- The merge tests of `tests/results_logic.rs` use planned B4 knobs instead of an invented
  `fixture` field, because `validate` now holds every knob to the plan.

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
