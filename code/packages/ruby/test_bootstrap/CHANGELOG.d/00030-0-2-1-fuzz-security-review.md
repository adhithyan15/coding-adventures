## 0.2.1 - fuzz security review (VER06i)

### Fixed
- Coverage source (M1): fallback modes `require` the target files instead of
  `load`ing them, so another tool's counters are not reset and side effects not rerun.
- Coverage source (M2): `load:` and `include:` entries are expanded to absolute paths
  at construction (a `Dir.chdir` in the target cannot hide edges); `include:` matching
  respects separators (L2).
- Coverage source (M3): a target that stops or consumes `Coverage` degrades the source
  to caller-reported edges (`degraded?`, `:fallback_degraded`) instead of aborting the
  campaign; `finish` never raises.
- `start` failure resets the mode (L3); `status`/`excluded_files` report plumbing files.
- Non-local exits from the target (`break`, `return`, `throw`) are a `non-local-exit`
  crash and the campaign returns its report (L5a).
- Budget sentinels are unforgeable by identity (`CampaignToken` table); a forged or
  copied sentinel is a crash (L5b).
- `Timeout::ExitException` is resolved at rescue time, not load time (L6).
- `inspect`/`to_s` of `Campaign`, `Bitmap`, seed and crash entries and `SplitMix64` show
  no bytes or RNG state.
