## [0.1.0] - UNRELEASED

### Added (census PR D: the manual census workflow)

- `.github/workflows/kiln-census.yml`: a `workflow_dispatch`-only workflow that
  records a sampled subset of the build's shards for real on `ubuntu-24.04` and
  `ubuntu-26.04` (`kiln record prepare`, the traced build, `kiln record collect`),
  runs `kiln record census` over each recording with `GOMEMLIMIT=8GiB`, appends
  the Markdown to the job summary and uploads only `census-*.json` and
  `census-*.md` as the artifact `kiln-census-<os>`. The record directory, the
  traces and `records.jsonl` are never uploaded. Inputs: `shard_count` (1 to 256)
  and `shard_indices` (up to 8 indices below the count); they are passed through
  `env:` and validated in shell, never interpolated into a script. The job fails
  on a non-zero census exit (4, 1 or 2), a failed `prepare` or `collect` or a
  timed-out build, but only after the summary is uploaded. Least privilege
  (`contents: read`), pinned actions, a concurrency group and a job timeout.
  The scheduled weekly run is census PR E.

### Changed

- The census Markdown `Verdict` section now states `skipped_too_large` (traces
  timed but not cross-checked, and their bytes) beside the cross-check mismatch
  line, so an "ok" cannot be read as covering a trace that was not cross-checked.
  A unit test pins it.
- KILN04 gains the census workflow section (inputs, artifact contents, exit
  semantics, what is never uploaded, limits).
