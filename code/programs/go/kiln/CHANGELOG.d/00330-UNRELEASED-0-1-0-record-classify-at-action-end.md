## [0.1.0] - UNRELEASED

### Added

- KILN04 P13 (stage 4 PR 2b-2b-i): records are classified at action end, and
  recording is serial.
  - `kiln record step` holds an exclusive non-blocking `flock` on
    `traces/.serial` (taken after the shared lock on the record directory, released
    before it). A step that cannot take it leaves `<step>.overlap`, runs nothing
    and exits 125. Under the lock the sidecar gets `seq` from `traces/.seq`
    (`kiln-record-step/2`).
  - The step that ends an action (its last step, or one that did not start or
    exited non-zero) parses the action's traces, applies the parse half of G7,
    runs `traced.Classify` and writes `traces/<sha256(action)>/record.json`
    (`kiln-record-classified/1`) or `record.failed` (panic, 10 minute limit, 2 GiB
    of traces, a second classification). The exit status is never changed.
  - `record.LiveFS`: the live no-follow disk view (`lstat`, `readlink`, and a read
    only of a regular file opened `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY` whose
    `/proc/self/fd` name equals the path asked, 64 KiB at most).
  - `prepare` seals `pre/tree` (`kiln-record-tree/1`: every tracked path with mode,
    blob id and link text; 394 222 entries, 48 MB on this repository) and
    `pre/classify.json` (`kiln-record-classify/1`: host roots and pinned
    ecosystems), and each action file lists its `tools` (`kiln-record-action/2`).
    `VerifyPre` decodes classify.json and hashes the tree; `Pre.Tree` and
    `Pre.ReadSealed` read sealed files again, re-hashing what they return.
  - `legacy.Traced.Pinned`; `trace.Trace.RootExecArgv` and `InitExecArgv` (the
    argument vectors of the two execs of Kiln that the helper-mode parser drops).

### Fixed (review of P13)

- The epilogue's parse limits are explicit and deliberate (1 GiB and 8 million
  events per step, the supervisor's own trace cap), not the parser's defaults (64
  MiB, 1 million records), which made a heavy-but-accepted trace
  `unknown_writes`. The 2 GiB per-action check is now reachable.
- Classification runs in a child process (`kiln record classify-action`), with
  `GOMEMLIMIT` and `RLIMIT_DATA`, killed at the deadline: out of memory and fatal
  runtime errors are `record.failed` and leave the step's exit status unchanged.
- `record.json` and `record.failed` are written atomically (temporary name and
  `link(2)`); `record.json` carries `sealed_sha256`.
- `traces/.seq` is created by `prepare` as `0\n` and updated with one `pwrite`; an
  empty or missing counter is refused instead of restarting at 0.
- A step is refused once its action has a `record.json` or `record.failed`.
- Every unclean sidecar is a parse reason, so an unclean tracer never sits beside
  `writes_complete: true`.
- The `.overlap` marker is written only when the serial lock is actually held
  (`SerialBusyError`).
- `prepare` refuses a tree file larger than a sealed file can be read back at;
  `readSidecar` checks the sidecar's action and step against its name.

### Fixed (round 2 of the review)

- The supervisor reads the child's `record.json` through a non-following handle,
  requires a regular file and refuses one over 256 MiB (`MaxRecordBytes`,
  `record-too-large`); the child drops a raw record whose lists hit the cap
  (`writes-too-many`).
- A failure removes any `record.json` before linking `record.failed`: one verdict.
- The child sets `oom_score_adj` 1000 and `RLIMIT_DATA` to the smaller of 8 GiB and
  half of `MemTotal` (soft limit scaled 5:8); the default child image is
  `/proc/self/exe` on Linux.
- `classify-action` refuses unless the step ends its action and no verdict exists.
- Spec: an `execve` argv over the parser's 1 MiB line cap fails closed.

### Corrected

- A link the action creates itself is resolved from the DISK, so a `mkdir` through
  it is silently misplaced if a later action retargets the link
  (`traced.TestOwnLinkIsResolvedFromTheDiskNotTheTrace`,
  `TestEpilogueSilentMisplacementOfAWriteThroughALinkTheActionMade`). That is the
  stronger case for classifying at the action's end.

### Changed

- `kiln record step`: the action and sidecar schemas are `/2`; an `.overlap`
  marker is not evidence that a step ran (`checkOrder` skips it).
- The `collect` stub's message names PR 2b-2b-ii.

### Specification

- KILN04 section 4.8: decision P13; P12 and P1 amended (`-jobs 1`; `step` classifies
  at the action's end); section 5.1 reads "the disk at the end of the action"; the
  2b-2b work is re-tagged 2b-2b-i (this), 2b-2b-ii (`collect`) and 2b-2c (gates,
  probes, the `daemon-left` fixture); section 14 records that serial recording
  slows wall time, so the shard count needs raising.

### Deviations from the P13 text

- (Withdrawn, see "Corrected" below: the claim that a link an action creates
  itself is resolved from the trace and not the disk.)
- The tree is decoded only when an action is classified, not by `VerifyPre` on
  every step (the digest is checked every time).
- `traces/.serial` is opened (and `traces/` made when absent) before `VerifyPre`;
  the ownership and mode of `traces/` are still checked by `createTrace`, after it.
