## [0.1.0] - UNRELEASED

### Added (KILN04 census PR C2)

- Per-language step wall time in the census: each `languages[]` entry has
  `step_wall_s` (`n`, `p50`, `max`, `sum`, seconds) and `step_wall_failed_s`
  (the same over steps with a non-zero sidecar `root_status`, separating a slow
  failed fetch from a slow successful compile), and the Markdown "Share by
  language" line prints it. The durations are the step sidecars' existing
  `started_at`/`ended_at` walls (the ones behind `timing.step_wall_ms`), filed
  per language; no new parse. A cargo (or NuGet, bundler, pip/uv, cabal) network
  stall now shows as one language's p50/max instead of looking like slow
  compilation.
- `collect.unknown_reasons` in the census JSON, and a line in the Markdown
  "collect" section: the shard's records with an unknown write set, counted by a
  fixed code (`serial-violated`, `interrupted`, `not-run`, `other`) chosen by the
  reason's prefix up to the colon. The prefixes are plain strings, so it works
  whether or not the collect change that emits `interrupted` and `not run` has
  merged. A byte precheck only gates the decode; the decoded top-level field
  decides, so a nested `unknown_writes` cannot fake a reason. To get the reasons the census now decodes `unknown_writes` from each
  record line of `records.jsonl` (it only counted lines before).
- Tests: `census_stepwall_test.go` (per-language aggregation with even, odd,
  one and no samples; prefix-to-code mapping including an unknown prefix; the
  line scanner).
- No workflow change: the Markdown already reaches the step summary.
