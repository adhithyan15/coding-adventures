### Fixed — the package CLIs no longer truncate piped output

- Every entry point that ended with `process.exit(runX())` now sets
  `process.exitCode = runX()` and lets Node exit on its own. There are fifteen,
  among them `report-cli`, `cli validate`, `plan-cli`, `book-cli`,
  `narration-cli`, `modality-cli`, `shard-cli` and `doc-shard-cli`.
- `process.exit()` ends the process before an asynchronous stdout write has
  drained. When stdout is a pipe, a report larger than the 64 KiB pipe buffer
  was cut off mid-document:
  `node dist/report-cli.js --format json | jq` failed at byte 65,536 of an
  8 MB report. Writes to a file or a terminal are synchronous and were never
  affected.
- Exit codes are unchanged (`cli.js` still exits 2 on an unknown command).
  `root-slug-splits-cli` and `script-owner-evidence-cli` already worked this way.
