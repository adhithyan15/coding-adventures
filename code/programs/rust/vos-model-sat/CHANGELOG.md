# Changelog

All notable changes to this package are documented here.

## [0.2.0] - Unreleased

### Added (VOS-040b, VOS31 sections 7 to 9)

- **The solve loop** (`src/solve.rs`): `solve_cnf` (conflict-bounded, optional DRAT),
  `check_default` (`drat_check::Limits::default()`, never raised), `model_satisfies`, and the lane's
  own unit propagator `propagation_refutes` (the `propagation-only` collapse). Written again here,
  not taken from `vos-capacity-probe` (VOS31 Q3). `ROUTE_MAX_CONFLICTS = 50_000` and
  `ROUTE_MAX_CHECK_WORK = 1 << 31` are pinned by a test that READS VOS30 8.6.1
  (`spec_states_budget`).
- **The lane** (`src/lane.rs`): every lemma at N in {8, 16, 32, 64}: UNSAT with a DRAT proof the
  default checker accepts, inside the route budget of VOS30 8.6.1; a SAT control (the lemma's
  mutant) whose model replays through the CNF, the circuit and, at N = 8, the evaluator's failing
  set; a premise witness for each implication lemma, the set read from the Warrant parser's AST;
  collapses re-derived (`constant-fold` from the AIG, `propagation-only` from the CNF).
  Findings `MS01` to `MS10`.
- **The table** `warrant/lemmas.sat` (`render_table`): solver-independent fields only; conflicts,
  `check_work` and acceptance are gated and printed, never committed.
- **CLI**: `gen`, `check` (with `--emit-drat <dir>`) beside `cross-check`; `check` also runs the
  three cross-checks (`MS02`). `--allow-missing-sat-key` bridges the gap until VOS-040c teaches
  `vos-link` the `sat` key (its loader rejects an unknown `vos.toml` key).
- Labels: the "SAT-certified" strings are chosen in exactly one function, reached only after an
  accepted proof and a passing control; a test scans the sources to keep it so.
- Dependencies: `cdcl-sat`, `drat-check`, `coding_adventures_sha256` (path; none edited).
- Tests: 31 in `tests/lane.rs` (a negative control per finding, tampered tables, a wrong budget, a
  rejected proof, an unreplayable control) and 4 in `tests/cli_lane.rs`.

### Results on the real `sched` contract (as run, release, Windows)

All 40 obligations UNSAT with an accepted DRAT; worst conflicts 590 (`safe_dispatch_only_removes_the_choice`
at N = 64), worst `check_work` 2,491,407 (`top_is_the_maximum` at N = 64), against 50,000 and 2^31.
`closure_at_m`, `config_n_is_the_id_range` and `policy_admits_only_top` collapse by `constant-fold`
at every width; none needs `propagation-only`.

## [0.1.0] - Unreleased

### Added (VOS-040a, VOS31 sections 5 and 6)

- **The encoder** (`src/model.rs`): `SchedModel`, one method per model function of
  `warrant/contract.part`, the honest transcription as default methods, so a mutant that overrides
  one method is seen by every caller exactly as a spliced Warrant model is. `Encoder { n }` is
  N-parametric for N = 1 to 64 (`Encoder::new` bounds it). Types in `src/types.rs`: `Ready`
  (width N), `PickEnc` (8-bit id at every width; settles VOS31 section 16 item 2), `UpdEnc`.
- **The ten lemmas** (`src/lemmas.rs`): `lemma_circuit` / `lemma` build each claim as one edge
  `holds`, plus `premise` for the six implication lemmas (for VOS-040b's premise witness).
- **The ten mutants** (`src/mutants.rs`): `Mutant`, generalised to any N; plus four deliberately
  wrong encoders (`Fault`: `top` off by one, `<=` for `<` in `pre_insert`, `mem` ignoring thread 0,
  a `put` that cannot clear) for the negative controls.
- **The three cross-checks** (`src/crosscheck.rs`, `src/oracle.rs`, `src/probes.rs`,
  `src/reference.rs`): (a) the lemma check against the Warrant evaluator at N = 8 on the base and
  on each mutant document, comparing the failing **sets** and the counts of `lemmas.table`; (b) the
  function check, twenty probes read through the evaluator's ground-call interface; (c) the width
  check against a plain Rust reference at N = 1 to 10, and the reference against the evaluator at
  N = 8. Settles VOS31 section 16 item 3 (the ground-call interface is usable).
- **Labels** (`src/labels.rs`): the exact strings of VOS31 section 11 and the forbidden words.
- `vos-model-sat cross-check <contract-crate-dir>`: runs the three checks (exit 0, 1 or 2).
- Tests: 48; every check has a negative control (`tests/cross_checks.rs`), the oracle's refusals and
  the CLI are in `tests/oracle_and_cli.rs`, the encoder alone in `tests/encoder.rs`.

### Changed from the spec (recorded in VOS31 by a separate commit)

- The two relations `safe_dispatch` and `policy_dispatch` are probed on every `Ready`, every
  `pick_of(k)` and four candidates for the second `Ready`, not on their whole 16.8M-case domain.
- `vos-link` and `vos-manifest` are dependencies (the evaluation document and loader), and
  `cdcl-sat`, `drat-check` and `sha256` are not yet (no solving in this PR).

### Fixed (VOS-040a security review, LOW items)

- A refused or short sweep is now a disagreement. Check (c) and check (b) discarded the `TooWide`
  error of `sweep` (`let _ =`), so a failed sweep added zero comparisons and hid the shortfall.
  A new `counted_sweep` records a refusal, and any comparison count other than `2^inputs` of the
  subject's declared domain, as a disagreement; `encoder_failed` now returns a `Result`. Tests: an
  N = 64 encoder (every wide sweep refused) and an encoder with an extra input (double count).
- A mutant must break its lemma: `check_mutant_breaks` requires a nonzero failing count on both the
  evaluator and the encoder for every lemma's mutant (`closure_at_m` included: its mutant fails on
  every case). Test: the honest model used as a mutant fails this check directly, not only through
  the table.
- `label_collapsed` takes a closed `CollapseReason` enum (`constant-fold`, `propagation-only`,
  VOS31 7.5) instead of free text.
