## [0.1.0] - UNRELEASED

### Added

- `traced.RawRecord.WritesComplete` (KILN04 5.5, stage 4 PR 4): the classifier
  says whether `Writes` and `TrackedWrites` list every in-repository path the
  action wrote. Fail-closed (zero value false); false for an unplaceable write,
  writing open, `rename`/`link` endpoint or Unix `bind`, a virtual write,
  `bad-path`/`vcs-write`, a write to the root, an unstable component that a
  write's resolution touched, a write-list or budget cap, a step with no exit
  line, a killed step, a process with no exit line, `daemon-left` and a recipe
  mismatch. A read-only reason (`exec-unreadable`, a read-only `unstable-path`,
  `host-read`) and a plain nonzero exit leave it true. It is `json:"-"`.
- `recfmt.CompleteOf` turns the field into the shard record's `writes_complete`.
- The differential fuzzer checks the invariant (every path the model wrote is
  under `Writes` or `TrackedWrites` whenever `WritesComplete` is true, reasons
  or not), with new model operations (writes through `/proc/self/cwd`, chmod
  through links, `..` and the root) and whole-trace perturbations. Unit tests
  give each trigger a true and a false case.
- `recmerge`: a test that a read-side `trace-incomplete` with the classifier's
  `writes_complete: true` and the switch on costs only the action, and that a
  recipe mismatch still voids the run.

### Changed

- `legacy.RecorderVersion` is now 2: the record's semantics changed, so every
  weak key changes and manifests recorded by the version-1 recorder are never
  looked up. The golden key vectors (`code/specs/fixtures/kiln/key-vectors-v1.json`)
  that embed `kiln:recorder` are regenerated; only the weak key, the strong key
  that includes it, and their preimages differ.
- The classifier's cached lookups remember a failed lookup (`node.bad`), so a
  write through a place that failed to read is unplaceable even when an earlier
  read raised the reason.
- A record with more than 65536 writes or tracked writes now carries
  `trace-incomplete:size` like the other lists (the lists were unbounded).
- `HonourWritesComplete` stays off by default; the recorder (stage 4 PR 2b)
  turns it on.
