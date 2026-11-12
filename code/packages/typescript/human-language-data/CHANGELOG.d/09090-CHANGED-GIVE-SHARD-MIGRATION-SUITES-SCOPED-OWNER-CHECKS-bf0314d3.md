### Changed — give shard migration suites scoped owner checks

- Check every one of the 23 real chapter plans and 23 real curriculum plans
  through the production single-plan `shard-cli --check <path>` route, naming
  the affected track in each result. Their real-corpus membership, layout,
  ordering, and negative-fixture assertions remain in place.
- Keep the complete all-plan `--check` in `shard-cli.test.ts`, as well as the
  standalone `check:shards` command used by repository verification and the
  Books workflow. This removes two redundant complete owner-tree scans under
  whole-suite load without narrowing the all-plan gate or changing source,
  shard bytes, timeouts, workers, or coverage requirements.
- The three focused test files passed 362 cases with one existing skip on
  Windows without coverage. Full-suite, covered, and required CI acceptance
  remains pending for #19242 and parent #18751.
