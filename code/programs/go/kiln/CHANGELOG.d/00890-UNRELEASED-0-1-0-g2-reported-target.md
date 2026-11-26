## [0.1.0] - UNRELEASED

### Changed (KILN04 D49: G2 decoupled from PR 8, restated as a reported target; user decision 2026-11-24)

- **G2 no longer blocks anything**, PR 8 (deleting the old scanner) included. The
  5 s absolute limb is dropped; the share limb becomes a reported target: parse
  share at p50 at most 10%. PR 8's other preconditions (Phase B, the 14-day /
  200-run soak with 0 mismatches and 0 allowed divergences, clean nightly fuzzing)
  are unchanged. Reasoning and the measured table (runner census run
  `45345660988`, G2 failing at p99 in all 6 shards, share p50 0.107 on
  ubuntu-24.04 and 0.088 on ubuntu-26.04) are in KILN04 decision D49.
- Parse speed is its own work item: a worker pool for per-line lex/parse with the
  model pass sequential, checked by an equality test against the sequential reader.
- Census: new `timing.g2_target` (`target_share_p50`, `share_p50`, `met`,
  `share_n`, `image_id`, `parse_mb_per_s_p50`) reports the target and the per-image
  parse throughput. `g2_failing`, `g2_at_p99` and the two old limb counters stay in
  the JSON for the record and are documented as reported only. The Markdown
  heading reads "a reported target, not a gate", and the stdout line's
  `g2_failing=` is now `g2_over_limit_reported=`. Nothing in the census exit
  status, `census_not_clean.py` or `kiln-census.yml` read G2 before or after.
