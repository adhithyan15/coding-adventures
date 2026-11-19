## [0.1.0] - UNRELEASED

### Changed (census PR C review: `kiln record census`)

- Fixed: the redactor replaced any substring, so a `/tmp` TMPDIR turned
  `/usr/lib/tmpfiles.d/x` into `/usr/lib$TMPDIRfiles.d/x` and `/var/tmp/y` into
  `/var$TMPDIR/y`. A prefix is now replaced only at a path-component boundary.
- Added `explain`: why actions are not recordable, with fixed codes only (a code
  is a member of a closed list, never a slice of the input): the path-free
  classifier reason kinds of `record.json`, its parse reasons and this census's
  own parse refusals by code, `record.failed` categories (timeout,
  traces-too-large, panic, classifier-failed, ...) and sidecar facts (truncated
  by reason, not clean, exit 125, non-zero root status).
- Added G2 evidence by size: per log2 size bucket timing, a parse throughput
  distribution, `g2_at_p99` (G2 judged at the census p99 trace size), and the
  first (cold-start) parse reported separately.
- Added a `host` block (image id, kernel, strace version line, shard index and
  count from the verified `pre/`, and the CPU model).
- `record.failed` wins over a `record.json`, as in `collect`; a `record.json` of
  another binding or sealed directory is counted and ignored.
- The census takes the record directory's lock (exclusive, non-blocking) and
  refuses while a recording holds it.
- A trace over 256 MiB is timed but not cross-checked (counted as
  `skipped_too_large`); a panic in a parse is a counted `parse_panic` problem.
  Distinct paths are capped at 100,000 and 32 MiB of text; the per-action step
  map is bounded.
- Markdown: languages, kinds and every name are code spans in list items (no
  autolink, no emoji).
- `TestParseCrossCheckedOnlyInCensus` is syntax-tree based and flags any
  reference, not only a call; the package variable became
  `CensusOptions.CrossCheck`.
- KILN04's census section is updated to match.
