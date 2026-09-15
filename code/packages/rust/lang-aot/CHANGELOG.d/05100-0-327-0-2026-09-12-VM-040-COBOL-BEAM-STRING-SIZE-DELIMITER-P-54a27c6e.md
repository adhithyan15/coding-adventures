## 0.327.0 — 2026-09-12 — VM-040 COBOL BEAM STRING SIZE/delimiter probe

Reprioritized the post-reference-modification-MOVE/trap queue (remaining
COBOL BEAM rows, VM-041, VM-060b, VM-058) by re-running the backlog's
prioritization policy against current source: `git fetch origin && git merge
origin/main` reported "Already up to date" — this worktree already sat at
`4bb9aecd21` (PR #14803) with 15 commits behind `origin/main`, every one an
ALGOL `unroll ... powers` commit (separately owned, confirmed unrelated via a
path-scoped `git log` over every `lang-*`/`iir-*`/`cobol-*`/BEAM path and this
backlog's own spec files, which returned nothing). `gh pr list --state open`
showed six open PRs (a Compose table-wheel feature, three npm/yarn dependabot
bumps, one GitHub Actions dependabot bump, one stale `paint-vm-canvas`
dependabot bump) — none touching this backlog. A fresh brace-balanced parse
of `PROGRAMS` (scoped to the static array) found the array grew from 455 to
470 total entries (ALGOL's own `unroll ... powers` rows), but the non-ALGOL
total held at 210 — confirming those additions never touched this backlog's
scope. 58 `Cobol60` rows, exactly 28 already declaring `Beam` and 30 not — the
backlog's "~30 undeclared" estimate was exactly right. `VM-041`/`VM-060b`
still have no scoped design entry beyond the ranked-backlog one-liner. COBOL
BEAM rows win again on the same rung-4/bounded-pattern reasoning as every
prior round.

Also swept every other `str_*`/`array_*`/`*_byte` lowering arm in
`iir-to-beam/src/lower.rs` for the same "relies on incidental host leniency"
shape that produced VM-D032's `str_slice` gap, per this session's own
mandate. Found none: `array_get`/`array_set`/`load_byte`/`store_byte` rely on
`atomics:get/put`, which enforce a STRICTER range than the documented
contract (the opposite direction from `str_slice`'s gap); `div`/`mod` rely on
`erlang:div/rem`, which raise a real, documented `badarith` on division by
zero; `field_load`/`field_store`'s field index is validated at compile time,
never a runtime bounds question. Logged as a negative result in the backlog
rather than silently, so the next slice does not redo this sweep.

Continued the established `.skip(N).take(4)`-over-`Cobol60`-filter pattern
(rows 28–31, immediately after the twenty-eight already covered) on real
Erlang: three `STRING ... DELIMITED BY SIZE` rows (item padding/truncation
into differently-sized receivers; a short-receiver truncate paired with an
exact-fill receiver; a changed source re-read by a second `STRING` into the
same receiver) and one `STRING ... DELIMITED BY ","` row (cutting each sender
at its first delimiter, keeping an absent delimiter's field whole, preserving
the receiver's tail). The three `SIZE`-delimited rows compile entirely
through `str_concat`/`str_slice`/`const`, already proven by earlier COBOL
BEAM slices. The fourth row is the first to reach a `DELIMITED BY delim`
sending-field scan (`cobol-iir-compiler::emit_prefix_before_delim`, shared
with `UNSTRING`), which reads a field via `str_len`/`str_index`/`str_slice` —
and `str_len`/`str_index` had no BEAM lowering at all before this slice (see
`iir-to-beam` CHANGELOG 0.9.3). Added both, reusing established patterns:
`str_len` via `erlang:length/1`'s existing `gc_bif1`/`import_length`
machinery (no bounds concern — length has no failure mode); `str_index` via
`lists:nth(idx+1, source)`, whose own clause structure already traps on both
out-of-range directions with no separate guard needed (unlike `str_slice`'s
`sublist` gap) — confirmed against the documented contract
(`vm-core::dispatch::handle_str_index`) before writing the lowering, not
after.

Promoted all four rows to declare `Beam` (28 → 32 of 58 COBOL rows; 434 → 438
declared cells) and added
`portable_text_stdout_cobol_beam_string_size_and_delimiter`, mirroring the
existing COBOL BEAM probe tests. Four new direct `iir-to-beam` regressions
pin `str_len`/`str_index` directly: `test_75_real_erl_str_len`,
`test_76_real_erl_str_index`, and the same trap/boundary-control pair shape
`test_73`/`test_74` established for `str_slice` —
`test_77_real_erl_str_index_traps_at_length` (`idx == length(source)` must
trap) and `test_78_real_erl_str_index_last_valid_index_is_in_bounds`
(`idx == length(source) - 1`, the last valid index, must not trap). Updated
`feature_coverage_doc_counts_match_programs_source`'s expected COBOL-60
tuple (58, 434) → (58, 438) and confirmed it fails against the pre-fix figure
before fixing it, and updated `LANG-VM-FEATURE-COVERAGE.md`'s COBOL-60 row
and grand-total prose (1563 → 1567 declared cells) to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone confirmed the corrected
total against a live run: 210 programs, 1357 cells exercised (was 1353), 210
skipped, zero failures, 478.98s.
