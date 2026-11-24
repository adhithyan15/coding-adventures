## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 4: the exec-head supervisor; no `kiln:recorder` bump, PR 5 bumps)

- `internal/exechead`, the recorder's half of D48.
  - Format (every OS): the `<step>.exechead` sidecar, JSON Lines, a header
    (`v`, `kernel`, `nonce`, `action_sha256`, `step`) and one line per execve or
    execveat notification (`seq`, `tid`, `tgid`, `syscall`, `path`, `status`, and on
    `confirmed` lines `dev`, `ino`, `head` (base64) and `sha256`). Eleven statuses:
    `confirmed`, `outside`, `gone`, `mismatch`, `timeout`, `unpinned`, `fexecve`,
    `same-image`, `failed`, `overload`, `tainted`. A `Writer` that numbers lines, hashes
    and counts every byte, and stops at a line or byte cap (`ErrOverflow`, sticky).
  - A STRICT `Decode` for PR 5's classifier: no unknown, duplicate or case-variant key,
    no missing key, statuses in the enum, the confirmed-only fields on confirmed lines
    and nowhere else, `seq` equal to the line's index, a head of at most 64 KiB whose
    `sha256` matches, per-line, per-file and line-count caps, an optional check of the
    header's nonce, action and step, and a byte-for-byte re-encoding check so spacing,
    key order and escape spellings the supervisor never writes are refused.
  - The supervisor (linux/amd64; other platforms get `Supported = false`).
    One receiver goroutine (listener hand-over with a deadline, then
    `NOTIF_RECV`), one processor, one poller per pinned exec, one watchdog.
    Per notification: tid and tgid, `NOTIF_ID_VALID` right after reading the tgid and
    again after `pidfd_open`; the path from the step's memory (`process_vm_readv`,
    page by page, at most PATH_MAX, a partial read or no NUL is `unpinned`);
    resolution inside the step's root (`openat2` with `RESOLVE_IN_ROOT` against
    `/proc/<tgid>/root` for an absolute path, base then `RESOLVE_BENEATH` for a
    relative or dirfd one, `RESOLVE_NO_MAGICLINKS`; `AT_FDCWD`, a 32-bit dirfd and
    `AT_SYMLINK_NOFOLLOW` handled); the `st_dev` test against the step's fresh tmpfs
    (`outside` otherwise); the pin (`O_RDONLY|O_NONBLOCK|O_CLOEXEC`, a regular file,
    dev and ino); the pre-exec identity of `/proc/<tgid>/exe` (`same-image` when it
    already equals the pin); then CONTINUE. The confirm waits for an OBSERVED
    transition of `exe` to the pin, reads the head from the pinned descriptor and
    checks `exe` again. `failed` only from a definite `/proc/<tid>/syscall` reading,
    read BEFORE `exe`; `gone` through the pidfd; `mismatch` for any other change.
  - Ordering: one exec per thread group at a time; a different thread's notification
    is held until the pending exec is decided; the same-thread rule (a same-tid
    notification settles the pending exec at receipt and bypasses the queue);
    taint (a per-group flag, a bounded table, a global overload-all flag on overflow)
    only on a different thread with an undecided exec, on `overload`, or past the
    deadline; the three-state `pending`/`claimed`/`written` flag so the poller and the
    settle never both write; per-group queue caps separate from the global confirm
    cap; a lingering pool for pollers past the confirm deadline.
  - Never stops answering: past a cap it keeps CONTINUE-ing and marks the step
    unclean. A processor panic or error degrades to answer-only and marks the step
    unclean. A stall watchdog (a notification unanswered past the stall deadline) and a
    pending-lifetime bound kill the step through the init's pidfd, then strace, and
    mark it unclean (`exechead-stall`, `exechead-stuck`).
- `kiln record step` creates the `AF_UNIX`/`SOCK_STREAM` socket pair (the supervisor's
  end close-on-exec), passes the child end to strace as descriptor 6, adds
  `--exec-notify-fd 6` to `sandbox-exec`'s arguments, creates `<step>.exechead`
  exclusively before the step starts, and after strace exits runs `Finish`, which syncs
  the file and reports. The listener is received with `MSG_CMSG_CLOEXEC`: exactly one
  descriptor and one data byte `L`, nothing queued behind it, else the step is unclean
  (`exechead-handover`). The receive has a deadline, because strace holds its own copy
  of the socket and end of file never comes; a lost hand-over kills the step (strace's
  pidfd-backed kill; there is no init pid yet) and marks it unclean.
- The step sidecar is `kiln-record-step/3`: `exechead_nonce` (the header's nonce, drawn
  per step), `exechead_sha256` and `exechead_bytes` (the whole file), all empty when no
  supervisor ran. `collect` accepts `<step>.exechead` as a regular file and does not
  read it. `StepOptions.NoExecHead` turns the supervisor off for tests whose "strace"
  is a shell script.
- `traced.HeadLimit` exports the classifier's head limit; a test holds
  `exechead.HeadLimit` equal to it.

### Review follow-up (same PR)

- DEFAULT-OFF: the supervisor runs only with `kiln record step --exec-head`
  (`StepOptions.ExecHead`); `NoExecHead` is gone and the existing tests need no
  opt-out. The sidecar's `exechead_*` fields stay empty when it is off.
- A receiver that stops (an error, a panic) kills the step and marks it
  `exechead-receiver`, closes the listener and stops the processor; a poller or
  watchdog panic degrades the supervisor and kills the step. Test seam
  `Timing.InjectReceiverError`.
- The pending-lifetime bound kills only a thread seen blocked in execve; otherwise
  the group is tainted and the line is `timeout`.
- A vanished non-leader thread with exe unchanged taints after a bounded re-check.
- The group pidfd is read through a dup under the lock; the init pidfd is a dup of
  the validated one and atomic; the kill path takes no `mu`; both goroutines recover.
- Unmeasured tmpfs devices record every exec `outside` and never open a file.
- The init asks for `SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV` (falling back to plain
  `NEW_LISTENER` on `EINVAL`); the hand-over byte is `K` or `L`, and the header's
  `wait_killable` records which. `sandbox.ExecNotifyByteKillable`.
- `exechead.Decode` takes a mandatory `Expect`; `New` checks
  `SECCOMP_GET_NOTIF_SIZES`.
- New real-kernel tests: receiver death, rename of the target by another process
  (replacing the lifetime-kill test with `...PendingLifetimeTaintsAHealthyStep`).

### Changed

- No `RecorderVersion` bump at step 4 (it stays at main's 17; #19908 takes 18, and
  PR 5, which adds classifier consumption, carries the bump). The supervisor is
  default-off, so a recording differs only by three empty sidecar fields, step
  sidecars are not cached across versions, and the filter-flag change applies only
  with `--exec-notify-fd`.
- Watchdog: the sweep and the lifetime finalise use `TryLock`, so a blocked sidecar
  write cannot stop the stall check; `receiverDied` marks and kills without `mu`.

### Tests

- Pure, every OS: the encoder, the writer (caps, sticky errors, byte accounting), the
  strict decoder (unknown and duplicate keys, case, missing keys, statuses, seq gap,
  repeat and reorder, the header, nonce, digest and step, per-line, per-file and
  line-count caps, non-canonical encodings, two files in one), the sidecar fields.
- linux/amd64, no namespace needed: the hand-over's receive (exactly one descriptor and
  one `L`, two descriptors, none, the wrong byte, two bytes, a second message, end of
  file, the deadline, nothing leaked on refusal) and the page-safe bounded string read.
- Real kernel (`kiln-sandbox`, `TestSandboxExecHead*`, all named in
  `ci/assert_sandbox_tests.py`'s `REQUIRED`), against real strace and the real sandbox:
  a confirmed first exec of a tmpfs binary from another image (dev, ino and head checked
  against the file), `same-image`, `gone`, a `#!` script, `unpinned` and `fexecve`,
  relative, `..`, symlink and execveat dirfd paths (including high bits and `AT_FDCWD`),
  an outside-then-tmpfs chain in one pid, a pending pin settled at the next same-thread
  notification, 25 poller-versus-settle races with exactly one line each, `failed`, the
  same-thread bypass of the held queue, a held exec released untainted, a held
  notification of a dead thread dropped, a signal-interrupted notification dropped, a
  two-thread overlap that taints, a non-leader exec, `overload`, the line cap, the header
  and `seq`, a lost hand-over, a listener closed mid-step, the stall deadline, the
  pending-lifetime bound, and a path flipped by another thread that never falsely
  confirms.
- Not run against a real kernel in this change (no Linux host was available to the
  author): all of the real-kernel tests above. They compile and vet for linux/amd64.
