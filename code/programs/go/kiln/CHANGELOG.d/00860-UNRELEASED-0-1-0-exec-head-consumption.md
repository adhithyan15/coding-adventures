## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 5: the classifier believes exec heads; `kiln:recorder` 17 to 19)

- **Classifier consumption** (`internal/traced/exechead.go`, `exec.go`). A SUCCESSFUL
  exec of a program under the step's fresh tmpfs (the `go test` binary) is no longer
  always `trace-incomplete:exec-unreadable:under-tmp`: when the step ran with
  `--exec-head`, the loader chain is read from the head the supervisor took from the
  pinned inode, through the existing `loadChain`, and the loader is observed like any
  read (no `checkStability`: the head was read while the kernel held the file
  write-denied). `traced.Input.ExecHeads` carries one decoded sidecar per trace (nil
  for a step with no supervisor, which behaves exactly as before).
  - Pairing: the nth exec event of a pid with the nth line of that tgid, failed execs
    and `outside` placeholders counted on both sides, equal counts for every pid (the
    init's included); a pid is refused (`exechead-pair`) for a count that differs, a
    line with `tid != tgid`, a result-less exec, or an overload/timeout/gone/tainted
    line followed by a confirmed line whose exec did not return normally. A trace
    without an init pid (not a helper trace) believes nothing.
  - A head is used only when the pid is not refused, the line's syscall is the
    event's, the header's kernel is on the table (major.minor exactly `7.0`, else
    `exechead-kernel`), the status is `confirmed` (any other status is
    `exechead-<status>` and carries no evidence), and the line's path is non-empty
    and equals the trace's decoded path byte for byte.
  - Census: thirteen new `exec-unreadable` sub-codes (`exechead-pair`, `-kernel`,
    `-outside`, `-gone`, `-mismatch`, `-timeout`, `-unpinned`, `-fexecve`,
    `-same-image`, `-failed`, `-overload`, `-tainted`, `-notelf`); the overall census bound is
    now 20480 entries (thirty-two sub-coded kinds at 256 paths).
- **Integrity** (`internal/record/exechead_read.go`). Before the classifier sees a
  step's `.exechead`, the recorder checks it against the step sidecar (`exechead_nonce`,
  `exechead_sha256`, `exechead_bytes`, StepSchema 3): a missing, non-regular, oversize,
  malformed (strict `exechead.Decode` with the nonce, the action's digest and the step),
  digest-mismatched or UNCLAIMED file (present, but the sidecar names none) refuses the
  whole action as a parse reason (`exechead-missing|oversize|digest|malformed|
  unclaimed`). `collect` repeats the check. The census maps each phrase, and the
  supervisor's own unclean reasons, to fixed codes.
- **Differential fuzzer**: the model writes the sidecar a healthy supervisor would have
  left from its own truth and checks the honest case (the read-set oracle now reaches
  actions that exec a file under the tmpfs) and seven perturbations (a missing,
  repeated or non-leader line, a status that is not `confirmed`, a changed path, a kernel
  off the table, a rotated sequence: no refusal may disappear).
  `TestDifferentialExecHeadIsNotVacuous` pins that the case is exercised.
- **Real-kernel test** `TestSandboxRecordExecHead` (named in `ci/assert_sandbox_tests.py`):
  `kiln record step --exec-head` through RunStep with a Go-test-like step (the test
  binary, a probe the repo owns, copied under the tmpfs, run, deleted); asserts the fd 6
  pass-through via strace, the step sidecar's digest fields, the decoded file, and that
  `collect` believes the exec (and still refuses the control action run without the flag).
- Unit tests for every pairing rule and refusal, the recorder's integrity cases and the
  epilogue's wiring; a parser test pinning that killed, unfinished and result-less execs
  refuse the whole trace.

### Changed

- `RecorderVersion` 17 to 19 (the accept set grows, so every weak key changes). 18 is
  open PR #19908's; whichever lands second renumbers.
- The pending-lifetime bound no longer kills (or taints) an exec whose thread has a new
  notification in the inbox or being processed (`inbox.holds`), and re-checks after a
  50 ms grace before a kill: the step-4 review's last open item.
- `TestCollectAcceptsTheExecHeadFileAndDoesNotReadIt` became
  `TestCollectBindsTheExecHeadFileToTheStepSidecar`: `collect` now reads the file.

### Spec

- KILN04 D48: step 5 is done; the implementation status lists every departure (the order
  of the pairing checks, no anchor without a helper trace, integrity as a parse reason,
  thirteen sub-codes, `RecorderVersion` 19). Left: step 6, a fresh-home census with
  `--exec-head` turned on (no recording uses the heads until the census workflow passes
  the flag).
