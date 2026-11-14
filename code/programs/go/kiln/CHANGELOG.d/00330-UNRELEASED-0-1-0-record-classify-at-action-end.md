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

- A link an action creates itself is resolved from the trace (which carries the
  link text), not the disk; the disk matters for links that were there before. The
  motivating test therefore uses a tracked link that a later action retargets.
- The tree is decoded only when an action is classified, not by `VerifyPre` on
  every step (the digest is checked every time).
- `traces/.serial` is opened (and `traces/` made when absent) before `VerifyPre`;
  the ownership and mode of `traces/` are still checked by `createTrace`, after it.
