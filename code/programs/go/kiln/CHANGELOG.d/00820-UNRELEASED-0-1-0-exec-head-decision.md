## [0.1.0] - UNRELEASED

### Added (KILN04 decision D48; spec only, no production change)

- `code/specs/KILN04-traced-read-sets.md`: decision D48, "exec heads recorded at
  exec time". It fixes how a binary the step built under the fresh tmpfs and
  deleted (Go's `pkg.test`, today `trace-incomplete:exec-unreadable:under-tmp`) gets
  its head read: a second seccomp filter with a user-notification listener on
  `execve`/`execveat`, the listener handed over a dedicated unix socketpair to a
  supervisor in `kiln record step`, a pin-continue-confirm protocol per
  notification, the `<trace>.exechead` JSONL sidecar and its digest in the step
  sidecar, classifier consumption by per-tid pairing plus an exact path match,
  `exechead-*` census sub-codes, the fail-closed cases, the security preconditions
  with the probe evidence, the known v1 limit (`gone`), the cost, the
  `RecorderVersion` bump (16 to 17 when consumption lands) and the follow-up PRs.
  Classifier consumption is gated on PR #19902's valid-map `PR_SET_MM_MAP` probe.
- `code/specs/KILN04-exec-head-spike.md`: Status says the design is adopted as D48,
  and the markers the two CI runs (45342132127, 45346779343) answered are promoted
  from INFERRED to VERIFIED; the open items stay open (section 7).
- D48 security-review amendments: a pidfd per notification (the `gone` and pid-reuse
  checks), the rename-by-another-process variant of the false confirm, a `failed`
  status detected through `/proc/<tid>/syscall`, `overload` and non-blocking
  receive with bounded pollers, cap overflow that keeps answering, `timeout` versus
  the stall deadline, two-step `openat2` resolution with `RESOLVE_BENEATH` and
  `RESOLVE_NO_MAGICLINKS`, a sidecar header (kernel, nonce, step) with `seq` and a
  strict parser, non-leader execs refused, path comparison on decoded bytes,
  `PR_SET_MM` refused in the first filter (step 3, with a `RecorderVersion` bump),
  a verified-kernel-series table, and `CLOEXEC` and exactly-one-fd listener
  receipt.
- D48 re-review text fixes: the `PR_SET_MM` filter check compares the low 32 bits of
  `args[0]` only (with a high-bits test), `failed` is concluded only from a definite
  other-syscall reading (`running` or unreadable means keep polling; a wrong
  `failed` is fail-closed), the step pid's pairing has the anchor as its only rule,
  and a future `prepare`-time probe of the two kernel facts is noted as the
  alternative to the kernel-series table.
- D48 second security re-review: a per-thread-group TAINTED state (any `overload`,
  `timeout` or release with unknown exec state taints the group; tainted lines are
  never confirmed), the queue held until the exec is definitely over rather than
  released on a timer (global hold deadline kills the step), a `tainted` status,
  pairing refusals for exec events with no result, unresumed `<unfinished ...>` and
  `+++ killed`, the shebang-`#!B` attribution limit stated (argv0/`AT_EXECFN` check
  not adopted), path-read and `execveat` edge cases, `NOTIF_ID_VALID` and the pidfd
  at the first use of the `tgid`, a watchdog goroutine enforcing the stall
  deadline, exact major.minor kernel matching, and the step-4 tests for them.
- D48 third review (LOW and INFO): a release-trigger table per status (pollerless
  statuses release at once and taint, `timeout` keeps polling until decided),
  the stall clock starting at dequeue so a held notification is not a stall,
  taint set before the `overload` CONTINUE and read by the poller after its final
  `exe` re-check under one lock, the vanished-tid rule (taint only if `exe` is
  unchanged; a queued thread dying in `de_thread` is dropped without taint), the
  same-pre-exec-identity rule as defence in depth with the identity read at
  dequeue, taint keyed by `(tgid, pidfd)` in a bounded table with a global
  overload-all taint, pidfd-based kill target, per-group queue caps separate from
  the global confirm cap, and the extra step-4 tests.
- D48 same-tid rule (Fable ruling on the release table): a notification from the
  same tid as a group's last undecided exec proves that exec is over, so it settles
  a pending pin (`confirmed`, `failed` or `mismatch`) and does not taint; only a
  notification from a different tid taints. In-place exec chains (`sh -c
  ./test.bin`, `env`, `timeout`, `nice`) stay untainted, and the poller no longer
  writes `timeout` by itself (one writer wins under the group lock). The release
  table, the init-pid note and step 4's tests are updated.
- D48 fourth review (LOW/INFO): a same-tid notification bypasses the group queue
  (so a pending pin behind a different-tid one cannot deadlock the step); the
  poller and the settle use three states (`pending`, `claimed`, `written`), and the
  settle checks taint under the group lock; stale text about a release or taint at
  the deadline is rewritten (taint only on a different-tid notification, `overload`
  regardless of tid, or a vanished thread with `exe` unchanged); queued notifications
  get the taint branch at the deadline; past-deadline pollers back off to about 50 ms
  and move to a one-per-group lingering pool; an absolute pending-lifetime bound ends
  in an unclean step; tid-reuse, `ERESTARTNOINTR`, signal-interrupted and L1-bypass
  tests are added to step 4.
