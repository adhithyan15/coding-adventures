# Changelog

All notable changes to this package are documented here.

## [0.2.0] - Unreleased

### Added / Changed (VOS-028c3c-3b-1c, VOS-D142 and VOS-D143; VOS30 sections 8.6, 9.12 and 9.12.1)

- **`decide --replan`** prints `decide_replan`'s answer for the results file, one line each: the
  row (`row R3: Compiled lane, reduced`), `cut density K = n`, `memory: ...` and a `because:` line
  per condition. Plain `decide` is unchanged.
- **The replan block of 8.6** (VOS-D142): `table::replan_lines`, `render_replan_block`,
  `extract_replan_block` and `splice_replan`, between `<!-- vos30-replan:begin -->` and
  `<!-- vos30-replan:end -->`. `table --write` renders it in the same pass as the 9.8 block (a broken
  marker pair is a usage error); `table --check` compares it byte for byte (drift), requires it to
  sit inside 8.6, and compares the hand-written label (`**Re-plan decision (VOS-028c): R3, compiled
  lane, reduced**`, `cut density K = 2`, `at their shipped T`) with the computed row, title, K and
  memory verdict. `check` fails closed: with a results file, a missing 8.6 section, block or label is a finding; the label must occur once, `K = 2` does not match `K = 25`, each marker is alone on its line, and CRLF is read as LF. The computed answer on
  the CI recording (run 45486250172) agrees with the sentence hand-written in commit B (R3, K = 2,
  shipped T); the generated block was written into VOS30 8.6 by `table --write`.
- **`cost::slowest_seeds` and `cost::derive_cheap`** (VOS-D143): the rule for the cheap class,
  written once.
- **`cost::CHEAP` re-derived from the CI recording** (run 45486250172), 100 groups (500 rows) to 122
  groups (610 rows) of 400 groups (2,000 rows). Entered (36): the 17 B3 certified and 15 B3
  incremental `every2`/`every4` groups under 3 s, and four D5 groups (T = 16 k = 1 split index;
  T = 4 k = 4 split index and split none; T = 64 k = 1 split index). Left (14, slowest seed over
  3 s on the CI hosts): B2 25 instructions 25%; B3 certified 25/1.0/`every8`, 500/0.25/`every1`;
  B3 incremental 100/0.05/`every8`, 100/1.0/`every1`, 25/0.05/`every32`; D1 T=4 k=4; D2 T=256 k=16
  and T=64 k=16; D3 split T=16 k=4; D4 certified T=4 k=8 and T=8 k=4; D4 incremental T=16 k=2 and
  T=8 k=4. No B5 group is cheap. The figures after each entry are now the CI recording's.
  A plain `check` spends the same 60 s budget on cheap rows, so BUILD time is unchanged; the mean
  recorded cost of a cheap row falls from 0.96 s to 0.73 s, so the sample covers somewhat more rows.
  The count of decision groups a plain `check` does not re-solve is now 12 (D2 at T = 64, k = 16
  joined the 11).
- **`slowest_seeds`** also counts a `crashed` result and any non-finite time as infinitely slow.
- **Tests:** `tests/replan_label.rs` (the printed decision, check on the committed spec, an
  edited block, an edited label, `--write` on a temporary copy, a changed decision) and
  `cost_table_is_exactly_the_committed_recordings_cheap_set` in `tests/review_fixes.rs`, which
  replaces the "at most 4x the line" test of commit B with the exact derivation (plus negative
  controls). The unix-only test of a bound-killed spot child now uses D1 T=8 k=2, still cheap.

### Added (VOS-028c3c-3b-1b, VOS-D141; VOS30 sections 8.5, 9.2, 9.12.1 and 9.12.2)

- **`sign --verify-file J --expect-verify-sha256 V`**, both REQUIRED. `sign` keeps every existing
  check except the stale half of `full_refusals` (it now calls `missing_tree_refusals`, so a
  recording whose `trees` differ from the working tree can be signed), and additionally refuses,
  with exit 2 and nothing written, unless `J` is an authentic joined verify file of this recording
  that reproduces it at the working tree: its bytes (read once) hash to `V`; mode `verify`, joined,
  with the recording's `shards` and `jobs`; no structural finding and no merges; `trees` at a
  committed tree and equal to the working tree's ids; `recording.verifies_sha256` equal to the
  recording's unsigned digest; the same row identities; every identity clean under
  `results::compare_verify`; and `results::decision_differences` empty.
- **`cli::compare_rerun`** (with `RerunLabels` and `RerunComparison`): the authentication and
  comparison core factored out of `authenticate_rerun` and `cmd_decision_diff`, now shared with
  `sign`. `decision-diff` behaviour and messages are unchanged (its tests pass unmodified).
- **The `verified` banner field** `{run_id, run_attempt, head_sha, verify_sha256, trees}` (new
  `results::Verified`), REQUIRED in any non-null `regeneration` (no file is signed yet, so nothing
  is incompatible). `run_id` and `run_attempt` come from `J`'s `recording`, `head_sha` is `J`'s
  `revisions.repo`, `verify_sha256` is the digest of `J`, `trees` are `J`'s. Parsing is strict (five
  keys, strings and an object) and `validate` checks the shapes (digits, 40-hex revision, 64-hex
  digest, `trees` passing `meta::committed_tree_refusals`). `unsigned_sha256` still ignores the
  whole banner, `verified` included.
- **The table banner names both anchors** when signed: `... Recording: run R attempt A at revision
  <12 hex>. Verified: run R2 attempt A2 at revision <12 hex>, verify file SHA-256 <16 hex>.` The
  unsigned text ("Full tier not independently regenerated.") is unchanged.
- **Guard rule 2** (`code/scripts/vos_results_guard.py`): a signed file passes when either
  `tool.trees` or `regeneration.verified.trees` equals the merge ref's ids; `verified.trees` is
  validated as strictly as `tool.trees` and a malformed one exits 2. The ratchet and rule 1 are
  unchanged.

### Changed (VOS-028c3c-3b-1b hardening, security review)

- `sign` requires the tree ids at the verify file's `revisions.repo` (via the same `trees_at` as
  VOS-D137) to equal the verify file's `trees`, so the banner's `verified.head_sha` cannot misreport
  where the verify ran; a commit missing locally is refused with the `git fetch origin <sha>` hint.
- Guard: a non-null `regeneration` needs a well-formed banner (lower-case hex digests and
  revisions, digit run numbers, a `verified` block), else exit 2, including a signed file with no
  `verified`; all id patterns use `fullmatch` so a trailing newline cannot pass.

### Added (VOS-028c3c-3b-1a, VOS-D141; VOS30 sections 9.3, 9.12.1 and 9.12.2)

- **`check --full --at-checkout`** (valid only with `--full`; refused without it before the file
  is read). A recorded input id that differs from the checkout's is printed as a `check: note:`
  line and the rows are re-run, instead of refusing. Still refused: no tree ids or only the legacy
  whole-crate names, a plan-coverage gap, an unreadable checkout, and an uncommitted (`+uncommitted`)
  input. The ok line names the count and the names of the differing inputs. Without the flag the
  behaviour and the output are unchanged. `cli::full_refusals` is now the historical-order union of
  the new `missing_tree_refusals` (always fatal; also adds the uncommitted-input refusal) and
  `stale_refusals` (the differing inputs, as note bodies); `sign` still calls `full_refusals`.
  New helpers `meta::is_comparable`, `meta::differing_names` and `meta::uncommitted_inputs`.
- **`recording.verifies_sha256`** (verify-mode files only; 64 hex digits; a record-mode file
  carrying it is refused when parsed): the SHA-256 of the recorded file rendered without its
  signing banner, from the new `Results::unsigned_sha256`, which `validate`'s banner check now also
  uses, so there is one definition. `check --full --shard --rerun-out` writes it, `join` refuses
  shards that disagree on it and carries it into the joined file; shards with none still join.
- **`results::compare_verify(rec, now)`**: the pure per-row comparison of a verification
  (`compare_rerun` followed by `compare_timing`), factored out of the in-process quick-row site
  and `spot::rerun_one_row` with no change in behaviour (pinned by `tests/compare_verify.rs`, which
  asserts equality with the old composition over every comparison class).

### Changed (VOS-028c3c-3b-1a)

- **`decision-diff`** no longer requires the two files to record the same tree ids (a verify at
  the checkout runs at another tree). It requires `--rerun`'s `recording.verifies_sha256` to equal
  the unsigned SHA-256 of `--recorded` (absent is refused), runs `compare_verify` on every
  identity, and on exit 1 prints `row differences` and `decision differences` as separate labelled
  sections; exit 0 only when both are empty. `verify_results` takes the digest as a new argument.
- **`decision-diff` also refuses** (exit 2) a `--rerun` whose trees are not those of a committed checkout (`meta::committed_tree_refusals`: none, legacy names, `+uncommitted`, malformed) or that records merges, matching what `check --full --at-checkout` refuses to produce.
- Tests: `tests/at_checkout.rs`, `tests/compare_verify.rs`, and new cases in
  `tests/verify_fanin.rs`; the recording-literal helpers of `shard_join.rs`, `sign.rs` and
  `verify_fanin.rs` gain the new field. A spec amendment (VOS30 8.5, 9.3, 9.12.1, 9.12.2) was
  committed first: verify at the checkout, weekly cadence with no gate.

### Recorded (VOS-028c3c commit B; VOS30 sections 8.5 and 8.6)

- **The results file is now the CI recording**: run 45486250172 (attempt 1) at commit `1bb7682`,
  64 shards on GitHub `ubuntu-24.04`, `--jobs 2`, 2,225 rows, replacing the 1,625-row Mac
  recording. Section 8.3 outcome: row 1, Infeasible: re-plan X3 (unchanged). Section 8.6 re-plan
  decision: R3 (compiled lane, reduced), K = 2, memory at the shipped T. See VOS30 sections 8.5
  and 8.6 for the authoritative text. Still provisional until the named reviewer signs
  (VOS-028c3d). No source change: this entry and the README describe the recording only.

### Changed (VOS-028c3c commit A; VOS30 sections 8.6.3 and 9.2)

- **B3 `every2`/`every4`, D5 and B5 are registered in `plan(Tier::Full)`.** `EXPERIMENTS` is 16
  names (B5 after B4, D5 after D4). `knobs("B3")` now appends the 72 cut knobs after its 108, so
  B3 is one contiguous run of 180 knobs (VOS-D135); `knobs("B5")` and `knobs("D5")` are the 24-knob
  grids as one contiguous block each. The plan is 445 knobs and 2,225 rows at five seeds; at
  N = 64 no shard holds more than one B5 knob, one D5 knob or three B3 knobs. The quick tier is
  the smallest knob of each new experiment (B5 length 25 at `fuse = 1`, both divergences; D5
  `t = 4, k = 1`, both splits; no B3 cut knob), so it grows from 85 to 105 rows and
  `quick_row_cap()` follows. `FULL_PLAN_EXEMPT` stays empty. **No results are recorded here:**
  the recording on CI runners follows in commit B of this PR, and until then `check` on the
  committed file fails `full_plan_covered` (620 rows missing: 600 full and 20 quick), by design.
- **Fixed (VOS-D138): the B5/B2 twin check compares only twins present in the file.** `b5_fuse1_violations` no longer reports a missing B2 twin (a quick-only file or one record shard legitimately lacks it); a present twin with a different digest is still a violation, and removing a B2 row from a complete file is caught by `full_plan_covered`.
- Tests: `tests/common/mod.rs` `complete_plan` completes the committed file with clones of
  recorded sibling rows (B3 every8, D4, B2) so the join, sign, verify fan-in and hardening tests
  keep a plan-covering fixture; they are fixture rows, never results. `the_real_plan_keeps_b5_one_per_shard`
  is now live and also covers D5 and B3 and the 2,225 and 105 totals; `no_shard_holds_more_than_one_b5_knob`
  derives its base from the real list; the B5 refusal test uses an out-of-plan fuse; the
  cfg(unix) `run --tier quick` test expects 105 rows and the spot-check population is computed.

### Changed (VOS-D139, VOS-D140; VOS30 section 9.2)

- **The recorded `trees` are one git id per build input, not per crate.** `meta::TREE_PATHS`
  (five whole crate directories) is replaced by `meta::TREE_ENTRIES`: `src/` and `Cargo.toml` of
  the probe and the four solver crates, the probe's `Cargo.lock`, and the four files `enumx.rs`
  `include!`s / `include_bytes!`s (15 entries, named `<crate>/<relative path>`). `tests/`,
  README, CHANGELOG, BUILD and the other metadata cannot change a recorded value and are no longer
  recorded, so a doc or test edit no longer stales a recording. `TREE_NAMES` is derived from the
  list; `LEGACY_TREE_NAMES` keeps the five old names valid in a `merges[i]` record only (history,
  never compared: `stale_lines` gives one advisory line for such a record, and `check --full`
  refuses it). `working_trees` marks each entry's own path `+uncommitted`.
- **Fixed (VOS-D139):** the assertion in `tests/review_fixes.rs` that the committed file lacks
  exactly 600 plan rows (the pre-recording state) is deleted; it would fail once a recording lands.

### Fixed (VOS-028c3c-2d, VOS-D137; VOS30 section 9.12.2)

- **`sign` checks tree ids at `--head-sha`, not ancestry.** Main is squash-merged, so the commit a
  CI recording ran at never reaches main's history and "HEAD or an ancestor" would have refused
  every signature at main (VOS-028c3d). `sign::sign` now takes `trees_at: &dyn Fn(&str) ->
  Result<Vec<(String, String)>, String>` (was `is_ancestor`) and refuses unless the five tree ids
  at `--head-sha` equal the recorded `trees`, naming the first tree that differs. The CLI passes
  the new `meta::trees_at(rev)`: it validates 40-hex first, clears the `GIT_*` selection
  variables, runs no shell, requires the commit to exist locally (else the refusal says
  `git fetch origin <sha>`; GitHub keeps PR head commits), and reads each
  `git rev-parse <rev>:<path>` over `meta::TREE_PATHS`, requiring a 40-hex id. `full_refusals`
  and `head_sha == revisions.repo` are unchanged. `meta::is_ancestor_of_head` stays: `merge` still
  uses it. Tests: one tree differing (each of five), a short set, a missing commit, an equal set
  signing, `trees_at` on this checkout, and the CLI's fetch hint.

### Added (VOS-028c3c-2c, VOS-D120, VOS-D134; VOS30 sections 9.2, 9.6 and 9.12.2)

- **`sign FILE --reviewer NAME --host LABEL --run-id I --run-attempt A --head-sha S
  --expect-sha256 H --out OUT`** (`sign::sign`, pure; `cli::cmd_sign`). Writes the
  `regeneration` banner into a NEW file with `write_new` (never in place, never over an existing
  file: a signed file is evidence and a hard-link publish cannot overwrite or half-write it).
  Exit 2, nothing written, unless the input is canonical with a null banner, its `recording` is
  `record`, joined and carries the run id and attempt typed, `--head-sha` is 40-hex, equals
  `revisions.repo` and is HEAD or an ancestor (`meta::is_ancestor_of_head`), `--reviewer` and
  `--host` pass the charset rule, `validate` has no structural finding, `full_refusals` is empty
  against the working tree, the full plan is covered, the file has no merges, and
  `--expect-sha256` (required) equals the input's digest. Signing changes no row byte and no other `meta` byte.
- **`Meta.regeneration` is now `Option<Regeneration>`** (was `Option<(String, String)>`):
  `{reviewer, host, run_id, run_attempt, head_sha, unsigned_sha256}`, canonical JSON, `null` when
  absent, so the committed file keeps its bytes (`the_committed_file_still_round_trips`). Strict
  reader (exactly the six string keys). `validate` reports as STRUCTURAL findings a malformed
  field, a banner on a file that is not a joined `record` recording of the banner's run, a
  `head_sha` that is not `revisions.repo`, and an `unsigned_sha256` that differs from the digest
  of the file rendered again without its banner (a row edited after signing).
- **Banner charset** (`results::is_banner_text`): a strict whitelist, 1 to 64 characters of ASCII
  letters, digits, space and `. _ , ' @ -`, no leading, trailing or doubled space, because `table`
  splices the banner into the spec (the earlier blacklist let `[x](https://evil.example)`, `![a](x)`,
  `a*b` and `#x` through). `table` renders a signed banner as `Full tier regenerated by NAME on HOST.
  Recording: run I attempt A at revision <12 hex>.`
- **VOS-D134, the overall budget.** Plain `check`'s in-process quick re-run is bounded to
  `results::QUICK_RERUN_BUDGET_S` = 1,800 s in all (`rerun_within`, `rerun_collect_budgeted`,
  pure `budget_spent`; `rerun_collect_in` and `rerun_collect` are unchanged wrappers with no
  budget, so `check --full` is unaffected). Measured: the 85 committed quick rows reproduce in
  about 10 s on a Windows developer machine at `--jobs 4`. Once spent no further row starts and
  one finding reports `K of N rows were re-run`; a row in flight keeps its own wall. The budget is
  checked between rows, so the worst case is the budget plus one row's wall.

### Fixed (VOS-028c3c-2c security review)

- **MEDIUM: `table` rendered an unvalidated banner into the spec.** `table --check` and
  `table --write` now refuse (exit 2, nothing rendered or written) a results file with any
  structural finding, so a hand-edited banner with a newline, `|` or `<` never reaches the spec.
- **LOW: banner charset is a strict whitelist** (see above); `sign` and `validate` use it.
- **LOW: a banner on a file with merges.** `sign` refuses a file whose `merges` is non-empty and
  `validate` reports a banner on such a file as a structural finding.
- **LOW: `--expect-sha256` is required.** `sign` exits 2 without it, tying the sign-off to the join
  digest. `reviewer` and `host` stay self-reported and outside `unsigned_sha256` (VOS30 9.6).
- **INFO: git environment.** `meta::is_ancestor_of_head` clears `GIT_DIR`, `GIT_WORK_TREE`,
  `GIT_COMMON_DIR`, `GIT_INDEX_FILE` and `GIT_OBJECT_DIRECTORY` before asking git.

### Changed (VOS-028c3c-2c)

- `merge` and `merge --add` on a signed file clear the banner (the merged file is a new recording).
- `positionals` knows the value flags `--reviewer`, `--host`, `--head-sha` and `--expect-sha256`.

### Added (VOS-028c3c-2b-ii, VOS30 sections 9.2 and 9.12.1)

- **`check --full --shard K/N [--rerun-out F] [--run-id I --run-attempt A]`.** The whole file is
  validated as `check --full` does, then only the rows of `plan_shard(5, K, N)` are re-run (quick
  tier in process through `results::rerun_collect_in`, full tier in children through
  `spot::full_rerun_collect_in`; the old entry points are wrappers with `None`), with the same
  comparison classes. A shard with no quick-tier row is not a finding. `--shard` without `--full`,
  and `--rerun-out`, `--run-id` or `--run-attempt` without `--shard`, are usage errors; every flag
  is read by `value_of`.
- **`--rerun-out F`** writes the shard's re-run rows as a canonical file with the verifying
  binary's own `meta` and a `recording` of mode `verify` (`cli::verify_results`, pure). It is never
  overwritten (refused before any re-run) and is written even when rows differ. `join` already
  accepted mode `verify`; a test now pins it, and `decision-diff` of the joined file closes the
  fan-in.
- **`decision-diff --recorded R --rerun J [--expect-shards N]`.** Requires equal row-identity sets
  (else exit 2), runs `decision_differences` without solving, exit 1 on any difference, 0 on none.
  It first authenticates the re-run (security review, MEDIUM; each a usage refusal): `--rerun` has
  a `recording` of mode `verify` and is JOINED (`shards` set, no `shard`); `--recorded` is not a
  verify file; the two record equal, non-empty tree ids; `check_guards` is empty for both; and
  `--expect-shards`, when given, equals the joined `shards`. A file compared with itself, or with
  any file that merely holds the same rows, is refused.
- **Security review fixes (VOS-028c3c-2b-ii).** `cli::write_new` publishes with a hard link, which
  fails if the target exists, so `join --out` and `--rerun-out` can never replace a file created
  between the early `exists()` check and the publish (`write_atomic` still replaces, for
  `run --out`, `merge --out` and `table --write`). `check --rerun-out` refuses an `--exe` that is
  not the running binary (its digest goes into the verify `meta`). `check --shard K/N` refuses a
  slice with no row of the plan (exit 2) and records a finding when the rows re-run differ from the
  slice's rows in the file. `check` refuses an unknown flag (`--rerun-ot`).
- **VOS-D132.** `decision_differences` also compares `cut_density_k`, the `decide_replan` row, its
  memory verdict and its `because` lines with the counters cut. The only caller outside the tests
  is `check --full`.
- **VOS-D134.** `results::check_guards` runs first in every `check` mode: structural findings of
  `validate_classes`, a duplicate row identity, and more quick specs than the plan's 85 quick rows
  each exit 2 ("nothing was re-run"). Not done: an overall wall budget for `rerun_collect`.

### Changed (VOS-028c3c-2b-ii)

- A structural finding in `check` is now exit 2 (`Failure::Usage`), not exit 1. Two tests that
  expected exit 1 for a deleted row and for a hostile knob (`review_fixes.rs`) were updated.
- `Identity` in `results` is public, with `identity_of`, `shard_identities` and `quick_row_cap`.

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
