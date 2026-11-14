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

### Fixed (second review round)

- **M4** Only a successful connect to a pathname Unix socket outside the
  repository and outside the fresh HOME and TMPDIR makes the write set unknown
  (W11); loopback, abstract and in-repository peers keep only the `daemon:`
  reason (an httptest server on port 0 stays complete).
- **M5** A child killed by a signal (SIGPIPE in `yes | head`) no longer makes
  the write set unknown; a core dump in any process, and the step process's
  own death, still do.
- **L5** The peer of a connect is resolved as a write lookup, so the
  "bound by the action itself" exception is believed only while the name is
  stable (the bind, swap-to-docker.sock, connect, swap-back, bind spoof is
  incomplete).
- **L6** `writes_complete` stays required on the wire for every record;
  `CheckComplete` accepts an explicit `false` for a non-built record without a
  raw record, and the merge charges a non-built record as a truncated
  execution (M-1), not as an unknown write set. A built record with a false or
  missing key is still unknown. The contract is in KILN04 7.1 and 4.8.
- The fuzz model gains port-0 bind plus connect and SIGPIPE child death.

### Fixed (third review round)

- **M6** `traced.Input.StepNetNS` (default false, fail closed) carries the
  per-step network namespace precondition. Without it a successful connect to
  a loopback or abstract peer the action did not bind makes the write set
  unknown; with it only the `daemon:` reason remains. The recorder is the only
  setter. The bare references to KILN04 section 4.7 are now conditional.
- **L7** A successful connect to an in-repository pathname socket that nobody in
  the trace bound is a host process: the write set is unknown (TMPDIR peers
  stay as they are).
- **L8** Only a record that ran nothing (`steps_run == 0`, not built) may carry
  an explicit `writes_complete: false` without a raw record; any record that ran
  a step, failed or built, must agree with the classifier and a false or missing
  key is `unkHard`.
- Regression tests `TestR3` and `TestR3Parser`; a fuzz model op connects to an
  unbound loopback peer with `StepNetNS` false (must be incomplete).

### Changed (final hardening)

- `recfmt.ShardRecord.Validate` refuses a record with `steps_run == 0` whose raw
  record has any entries, writes, tracked writes, observations, binds or
  reasons: a record that ran nothing was never classified, so content is a
  recorder bug.
- KILN04: a "known precision limits" note (bind, connect, unlink of a Unix
  socket is judged incomplete; the planned trace-order exception needs its own
  review) and two more PR 2b items (the `StepNetNS` recorder test, and no gate
  or probe running repository code outside a traced step).
