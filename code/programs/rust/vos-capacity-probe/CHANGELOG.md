# Changelog

All notable changes to this package are documented here.

## [0.2.0] - Unreleased

### Added (VOS-028c3c-2b-i, VOS30 sections 9.2 and 9.12.2)

- **Sharding (`exp::full_knobs`, `parse_shard`, `shard_of`, `plan_shard`; `run --shard K/N`).**
  Knob `i` of the plan, in plan order, goes to shard `i mod N`; every seed of a knob stays in one
  shard; the union over `K` in `0..N` is `plan(Tier::Full)` with each row once. `parse_shard`
  takes ASCII digits only, `K < N <= 1024`. `run --shard` needs `--tier full` and refuses
  `--only`. Tests pin the partition and "no shard holds more than one B5 knob" (the B5 block
  inserted at every offset of the 445-knob plan, at N = 64 and 128, with a self-arming assertion
  on the real plan that goes live when the B5 knobs are registered).
- **The `recording` block (`results::Recording`, `Meta::recording`).** Optional, written after
  `regeneration`, omitted when absent: the schema number stays 2 and the committed file keeps its
  exact bytes (`the_committed_file_still_round_trips`). `{mode, run_id, run_attempt, shard | shards,
  jobs}`; `run_id` and `run_attempt` are 1 to 20 ASCII digits, given only by `--run-id` and
  `--run-attempt` (never the environment), both or neither; a bad value is refused on read.
- **`join` (`join::join`, `cli::cmd_join`; VOS-D125, VOS-D127 L1).** Joins the N shard files of
  one run into one file, refusing (exit 2, nothing written) unless every input is canonical and
  structurally valid, the shards are exactly `0..N`, provenance (tool, binary digest, trees,
  revisions, host triple, mode, jobs, run id, run attempt) is identical, no input has merges or a
  banner, and each input's rows are exactly `plan_shard(K, N)` (which gives disjointness and union
  = plan in one comparison). `--expect-binary-sha256`, `--expect-run-id`, `--expect-run-attempt`,
  `--expect-shards` and `--expect-jobs` refuse on a mismatch; a local run (no run id) refuses
  `--expect-run-id`. Row findings are printed as `join: finding:` and do not refuse. Unknown
  flags are refused.
- **Hardening from the 2b-i security review (VOS-D136).** `join` pins every row's `seeds` to
  `MIN_SEEDS`, refuses more than `MAX_SHARDS` input files before reading any, refuses a flag given
  more than once (`value_of` counts occurrences; `run` and `join` are its only callers) and never
  overwrites `--out` (an existing file, including an input, is refused). `rustc` and `os` are
  documented as intentionally not compared (the binary digest fixes the build; `os` may vary
  across runners of one label, VOS-D133).

### Added (VOS-028c3c-2a, VOS30 section 9.12.1)

- **Comparison classes (`results::compare_class`, VOS-D124, VOS-D128).** `compare_rerun` now
  compares a row in one of three classes. `Exact` (neither row wall-capped nor killed) is the old
  comparison, byte for byte. `WallWhole` (either row has cap `wall`, both `unknown`, the
  experiment not segmented) compares identity, `instance_sha256`, `ands`, `vars`, `clauses`,
  `entropy_bits`, `declared_bits`, `result`, `cap` and `wall_s`: a single instance is built whole
  before the solve, so its structure does not depend on where the clock ran out, and dropping it
  would let an edited instance hide behind a timeout. `Minimal` (a wall-capped segmented row, which
  hashes only the segments reached before the deadline, or a `killed`/`oom`/`crashed` row on either
  side) compares identity, `result`, `cap` and `wall_s` only. A flip between a completed verdict and
  `unknown`/`wall` is one `result differs` finding. `compare_timing` is skipped when either row is
  not `Exact` (it used to look at the recorded row only). Counted from the committed file: 64 of the
  141 wall-capped rows are segmented (D4 55, B3 7, D3 2) and none ended killed, oom or crashed.
  `spot` inherits the rule and keeps its "not verified" rule.
- **`validate FILE` and `validate_classes` (VOS-D127 L3, VOS-D129, VOS-D130).**
  `results::validate_classes` returns `Validation { structural, rows }`; `validate` is the same
  findings in the same order as before (one tagged pass, so `check` and `merge` print what they
  always printed). Structural: no rows, dirty tree, tree ids, merges, closed sets (result, cap,
  experiment, plan knob, tier), `wall_s`, seed counts. Row: everything that reads a row's control,
  counters, resident set, `green`, collapse rules or the B5/B2 twin digests. The new subcommand
  runs only `validate`, re-runs nothing, and exits 2 on a structural finding or an unreadable file,
  1 on row findings only, 0 when clean.

### Changed (VOS-028c3c-2a)

- **`runner::hard_limit_for` and `cpu_limit_s` take the experiment, not the wall (VOS-D123).** The
  control stage comes from the new `exp::control_wall(experiment)`, which `b5_control` also uses,
  so a future experiment with a 1,800 s row wall does not inherit B5's 600 s of control slack. The
  values are unchanged (300, 780 and 2,550 s).
- **`rerun_collect` re-runs each quick row under its own experiment wall (VOS-D131).**
  `results::rerun_wall` is `row_wall_s`, so a chain row gets 600 s (B5 1,800 s), not a flat 120 s.
- **Security-review fixes (VOS-028c3c-2a, VOS-D134).** (1) The two identity checks of
  `bounds_violations`, `seeds` over `MAX_SEEDS` and `seed` outside `1..=seeds`, are now
  *structural* findings (the row is not a row of the plan), not row findings, so `validate FILE`
  exits 2 for them; every check that reads a measured value stays a row finding. The messages and
  their order are unchanged (the function builds one tagged list). (2) The spot check treats a
  re-run the spot bound killed as "not verified" whatever the recorded `result` is; a `killed`
  record used to vouch for it. (3) `validate` refuses more than one positional ("validate takes
  exactly one results file"). Tests: `a_bad_seed_range_is_structural`,
  `validate_takes_exactly_one_file`, and a `killed` record in the spot bound test (unix).
- The results file and every digest are unchanged.

### Added (VOS-028c3b, VOS30 sections 8.4 item 11, 8.6.3 and 9.12)

- **B5's 600 s total is the sweep's elapsed time, fail-closed.** `b5_row` records `sweep_ms`
  (instance build through the sweep's verdict, G3 control excluded) as a row metric, listed in
  `NONDETERMINISTIC_METRICS`; `route_green` requires it present, finite, non-negative and
  strictly under 600,000 ms (NaN, infinity, negative or missing is not route-green), replacing
  the `solve_ms + check_ms` reading (VOS30 8.6.3, VOS-D118). The sweep's per-solve accounting is
  now the function `account`, so the missing-proof test exercises the code `Sweeper::solve`
  runs.

- **Experiment B5, SAT sweeping (`src/sweep.rs`).** A deterministic sweeper over one AIG for both
  sides: 256 seeded simulation patterns (`Aig::simulate`) give candidate classes up to complement,
  with the lowest node as the representative. Candidates are proved in topological order, each
  lemma a fresh CNF over its cone on a reduced AIG in which already-merged nodes are substituted,
  cut at an 8-level frontier whose nodes are shared free inputs. A SAT answer at the frontier is
  possibly spurious, so the depth doubles up to the full cone; only a full-cone SAT is a real
  counterexample, and it refines the classes. Every UNSAT lemma and the final obligation (the
  outputs that do not merge) is DRAT-checked under `Limits::default()`. Row metrics `lemmas`,
  `candidates`, `refuted` and `escalations` are added.
- **`fuse` generator (`gen_b5`, `B5Shape`).** `fuse = w` starts from B2's program (`gen_b` at the
  same length, divergence and seed) and turns every accumulating register-register `add a, a, b`
  into a run of w adds into `a`, each extra add `a += c_j` drawing `c_j` from a separate seeded
  stream, so P's own stream is untouched; the result is truncated to the instance length. The
  reference right-nests each run, so equivalence points are at most w instructions apart. With
  `fuse = 1` there are no runs and the program, its encodings and the DIMACS are B2's, byte for
  byte.
- **G3 control and signature-merge negative control (`tests/sweep.rs`).** The G3 control goes
  through the same sweep; its model is checked against the unswept control CNF and replayed. The
  signature-merge negative control is a separate test of the same sweep.
- **`validate` cross-rule.** A B5 row with `fuse = 1` must carry the same `instance_sha256` as the
  matching B2 row.
- **Chain budget.** B5 joins the 600 s chain budget (VOS-D095).
- **B5 measurement walls (owner direction, 2026-11-23; VOS-D118).** `exp::B5_ROW_WALL_S` = 1,800 s
  (`row_wall_s("B5")`, recorded as `wall_s`), `B5_OBLIGATION_WALL_S` = 600 s for each lemma and the
  final obligation, and `B5_CONTROL_WALL` = 600 s with `B5_CONTROL_CONFLICT_CAP` = 1,000,000 for
  B5's G3 control. `runner::hard_limit_for(1800)` adds the longer control stage (2,550 s). These
  are measurement walls, not gates: no gate number changed, and every other experiment's walls and
  recorded rows are unchanged.
- **`route_green` enforces B5's 600 s total.** The pre-registered clause of VOS30 8.6.3 is now
  implemented: a B5 row needs `solve_ms + check_ms` strictly under `B5_ROUTE_MAX_TOTAL_MS`
  (600,000). `decide_replan` reads it through `group_route_green`. Tests: boundary and non-B5
  controls in `tests/results_logic.rs`, the row walls of B5/B3/D3/D4/D5/B2.
- **Not yet in the plan.** B5 is not registered in `plan(Tier::Full)`; registration and Unix
  recording are VOS-028c3c (queue item 9c3). `FULL_PLAN_EXEMPT` stays empty.

### Fixed (VOS-028c3b security review; VOS-D119)

- **`validate` refuses an unregistered experiment.** The closed knob set was checked only for
  experiments in `EXPERIMENTS`, so a B5 row (unregistered until VOS-028c3c) or a row with any
  made-up name passed with any knob. An experiment outside the plan is now a violation and the
  knob check applies to every row.
- **A certified sweep requires a proof.** An UNSAT answer with no DRAT text in `ProofMode::Certified`
  is no longer counted as accepted (`all_accepted = false`, with a note).
- **The B5 cross-rule is stricter.** A B5 `fuse = 1` row needs a non-empty `instance_sha256`, and it
  is compared with every B2 row of equal length, divergence and seed, not only the first.
- **B5 knob bounds.** `shape_for` refuses a length outside 1..=100000, a fuse outside 1..=64 and a
  divergence outside [0, 1] with an error (a `crashed` row), never a panic or a huge allocation.

### Added (VOS-028c3a, VOS30 sections 8.6.1, 8.6.2, 8.6.3, 8.6.4 and 9.12; VOS-D117)

- **B3 cut densities `every2` and `every4`.** `exp::b3_c3_knobs` and `exp::C3_CUTS` add the 72
  `every2` / `every4` knobs (`segments_for` and `run_row` / `structure_of` handle them directly);
  registration in `exp::knobs("B3")` stays deferred until `VOS-028c3c` records them on Unix so
  `FULL_PLAN_EXEMPT` remains empty.
- **Experiment D5 (`D5Touched`, `D5Frame`, `exp::d5_knobs`, `exp::d5_row`).** Memory as index
  split (`split = index`, `T * k` concrete-index touched-word obligations) vs symbolic index
  (`split = none`, `k` obligations), plus a separate structural frame obligation per access
  (excluded from capacity counts when collapsed, folded in as an ordinary obligation if it needs
  conflicts, reported in `frame_obligations` / `frame_collapsed` / `frame_conflicts` /
  `frame_ands` metrics), and the two-mutant G3 control (touched-word data/index mutant + stray
  extra store frame mutant); registration in `plan(Tier::Full)` stays deferred until `VOS-028c3c`.
- **Re-plan budget & decision (`results::route_green`, `cut_density_k`, `decide_replan`).**
  Implements the frozen route bounds (`ROUTE_MAX_CONFLICTS = 50,000`, `ROUTE_MAX_CHECK_WORK = 2^31`),
  cut density `K` in `{0, 1, 2, 4}`, the R1 to R4 decision table and the orthogonal D5 memory
  verdict, with negative controls for every clause in `tests/results_logic.rs`.
- **Windows test fix (`VOS-D117`).** `tests/hardening.rs` now checks the Unix-only skip messages
  under `#[cfg(not(unix))]` in `check_prints_a_non_fatal_stale_line_when_a_recorded_tree_differs`.

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
- **Security review, round 3 (VOS30 9.12 "Hardening, round 3"; tests in `tests/review_fixes3.rs`).**
  Owner decision 2026-11-20: the CI gap for solver and timing fields of expensive rows is accepted
  (no scheduled or per-PR `--full`); the c3 decision stays provisional until a `check --full` at
  one tree passes and a named reviewer signs off. 1: a red `unsat` row relabelled `collapsed` is
  refused: `validate` (`results::collapse_violations`) requires `conflicts == 0 || clauses <= 2`
  for any reason, `ands == 0` for identical-structure and constant-fold, identical-structure only
  for a chain row, and a G2 note whose numbers match the row for a reason-less collapse; the
  `structure` child reports the rebuilt instance's own collapse verdicts
  (`Instance::structural_collapse_reason`, entropy half rule, 25% AND rule) and `compare_structure`
  holds a recorded `unsat` or `collapsed` row to them; `verify_collapse_reason` is now called by
  the child. 2: the decision rows have their own structural budget (`DECISION_STRUCT_BUDGET` =
  150 s) and one not reached or not verified is a violation naming it (`check --jobs 1
  --spot-budget 1` used to pass with 50 of 85). 3: `decision-critical groups not re-solved by this
  run: N` follows what the run re-solved. 4: `structure_comparable` is false for a chain row that
  ended `sat`.
- Tests: `tests/hardening.rs` (negative controls for every clause above, on real child processes
  where the clause is about a process) and unit tests in `runner`, `row` and `cli`.

- **Security review, round 2 (VOS30 9.12 "Hardening, round 2"; tests in `tests/review_fixes2.rs`).**
  1: a plain `check` re-solves only cheap-class rows, which the spec and README now say
  (expensive rows, decision-critical ones included, are verified only by `check --full`);
  `check` rebuilds every full-tier row's instance WITHOUT solving it in a child (new hidden
  `structure` command, `exp::structure_of`) and compares `instance_sha256`, `ands`, `vars`,
  `clauses`, `entropy_bits`, `declared_bits` (decision rows first, then a seeded sample, 45 s
  budget; `cost::STRUCT_HEAVY`, the five D3 T = 256 groups, left to `--full`) and prints
  `decision-critical groups not re-solved: N`. 2: `compare_rerun` compares every deterministic
  field (vars, clauses, entropy, propagations, proof size and work, cap, note, `wall_s`, the
  metric key set both ways and every non-timing metric); `--full` also applies the timing band
  and the `green` recomputation to the in-process quick rows. 3: `--full` requires the re-run
  rows' decision to equal the recorded one. 4: `check`, `table`, `decide` and `merge` refuse a
  non-canonical file (`Results::from_canonical_text`); `seeds` and every narrowing conversion
  use `try_from`. 5: `validate` bounds times, counters, metrics and the orderings that hold by
  construction. 6: `SPOT_FLOOR` is a plain 16 and the cheap table is pinned (at least 100 groups,
  500 eligible rows). 7: the parent waits for a child's output readers with a 5 s deadline
  (`READER_WAIT`) and ends the row `killed` instead of blocking on an orphan's pipe. 8:
  `write_atomic` keeps `mode & 0o777` and cuts the temporary name's stem to 150 bytes.

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
