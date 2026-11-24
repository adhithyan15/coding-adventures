# KILN04 exec-head spike: recording an exec'd binary's head at exec time

## Status

SPIKE. A design note and an experiment, no production change. The experiment is
`TestExecHeadSpike` in `programs/go/kiln/exechead_spike_linux_amd64_test.go`; it
logs `PROBE: exechead` lines on CI's kernels and asserts nothing about the
mechanisms. It is gated (it skips unless `KILN_EXECHEAD_SPIKE=1`), its name has no
"Sandbox" in it so the kiln-sandbox job never runs it, and it is not in
`ci/assert_sandbox_tests.py`; the manual workflow
`.github/workflows/kiln-exechead-spike.yml` runs it on both runner images and
puts the PROBE lines in the step summary. This note says,
for every claim, whether it was VERIFIED (read in this repository's code) or
INFERRED (recalled from kernel documentation and source, not re-checked in this
session; the experiment is what checks it). No kernel fact below is VERIFIED by a
run: the spike was written on macOS and could only be compiled for linux/amd64,
not run. Read the CI log first; where it disagrees with this note, the log wins
and this note needs an edit.

## 1. The problem (VERIFIED, from the code)

`internal/traced/exec.go` follows the loading chain of every exec: a `#!`
interpreter, and a dynamic ELF's PT_INTERP loader. The kernel opens those files
without a system call strace can show, so `loadChain` reads the binary's first
`headLimit` (64 KiB) bytes through `readHead`:

- from the tracked tree if the file is tracked;
- else from the post-run disk;
- but NEVER under `$HOME` or `$TMPDIR` (`underTmp`): that is a fresh tmpfs and is
  gone after the run.

A binary the step itself made under the tmpfs and then deleted (Go's
`$TMPDIR/go-build*/b001/pkg.test`) therefore reaches `reasonSub(rExecUnreadable,
"under-tmp", ...)`, and the action is refused as
`trace-incomplete:exec-unreadable:under-tmp`. Every Go test action hits this. A
trace-only rule is ruled out: the loader path is a link-time choice
(`-Wl,--dynamic-linker`), and a custom loader can mimic a static start, so only
the head itself, read at exec time, is sound.

## 2. Mechanism A: fanotify FAN_OPEN_EXEC

### 2.1 What the event is (INFERRED)

- `FAN_OPEN_EXEC` and `FAN_OPEN_EXEC_PERM` exist since Linux 5.0 (fanotify(7)).
  They are raised from `fsnotify_open()` (`include/linux/fsnotify.h`) when the
  file was opened with `FMODE_EXEC`.
- `fs/exec.c`'s `do_open_execat()` opens the program with `FMODE_EXEC`.
  `load_elf_binary()` (`fs/binfmt_elf.c`) opens the PT_INTERP loader with
  `open_exec()`, and `load_script()` (`fs/binfmt_script.c`) opens the `#!`
  interpreter with `open_exec()` too. Both take the same path, so an event per
  file is expected: the program, its `#!` interpreter(s), and the ELF loader. The
  libraries `ld.so` itself maps are opened with plain `open`, so they raise no
  exec event.
- `_PERM` events block the exec until the listener answers: the ordering needed
  to pin before the kernel runs the file. Plain events are asynchronous.

### 2.2 The permission rules, which are the problem (INFERRED)

- `fanotify_init(2)` needs `CAP_SYS_ADMIN` checked with `capable()`, that is in
  the INIT user namespace. A uid 0 inside the sandbox's user namespace does not
  satisfy it.
- Since 5.13 an unprivileged caller may create a group, with limits: class
  `FAN_CLASS_NOTIF` only, `FAN_REPORT_FID` required (no event descriptors, so no
  pathname from the kernel), no permission events, and INODE marks only (no
  `FAN_MARK_MOUNT`, no `FAN_MARK_FILESYSTEM`).
- An inode mark watches one directory's children (`FAN_EVENT_ON_CHILD`), not a
  tree. A step creates `$TMPDIR/go-build123/b001/` after the run starts; no mark
  placed before the step can cover a directory that does not exist yet.
- Mount and filesystem marks are what would cover a whole tmpfs. They need
  `CAP_SYS_ADMIN` in the init namespace, or (newer kernels) in the user namespace
  that owns the mount or superblock. For the sandbox tmpfs that owner is the
  sandbox's own user namespace, of which the (unprivileged) root is the parent,
  not a member. Whether recent kernels (the CI's 7.0.x, 26.04) relax this is
  UNKNOWN here; the experiment asks.
- The root helper on CI is an unprivileged user, so it is in the unprivileged
  case. The sandbox's init has `CAP_SYS_ADMIN` in its own namespace until
  `dropPrivileges`, which is not enough for `capable()`.

### 2.3 Pathname and key (INFERRED)

An fd-reporting (privileged) group gets an open descriptor per event, and
`/proc/self/fd/N` names the file. An unprivileged `FAN_REPORT_FID` group gets a
file handle (`FAN_EVENT_INFO_TYPE_FID`), and with `FAN_REPORT_DFID_NAME` the
parent directory handle plus the entry NAME: a basename and a handle, not a path.
Resolving a handle to a file needs `open_by_handle_at`, which needs
`CAP_DAC_READ_SEARCH`. So an unprivileged listener gets a (fsid, handle) key it
cannot open: it learns THAT a file was exec'd, not what it contained, and the
file is gone by the time anyone could ask. That is not the head.

### 2.4 Assessment of A

Even where it works it yields identities, not bytes, so the head still has to be
obtained another way. It is likely refused or too weak for an unprivileged
helper. Not recommended unless the experiment shows mount or filesystem marks
accepted for the helper on CI's kernels.

## 3. Mechanism B: seccomp user notification on execve/execveat

### 3.1 Fit with the current filter (VERIFIED from the code, plus INFERRED kernel rules)

- `sandbox.installSeccomp` (`internal/sandbox/sandbox_linux_amd64.go`) installs a
  classic BPF filter with `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)` in the
  init, after `dropPrivileges` has set `no_new_privs`, just before `execStep`.
  `bpf.go` allows execve and execveat (they are not in `RefusedSyscalls`).
- A second filter can be stacked with `seccomp(SECCOMP_SET_MODE_FILTER,
  SECCOMP_FILTER_FLAG_NEW_LISTENER, ...)` (Linux 5.0). It needs `no_new_privs`,
  which is already set. At most one filter in a task's stack may have a listener
  (EBUSY otherwise): the existing filter has none.
- All filters run and the most restrictive verdict wins (seccomp(2): KILL, TRAP,
  ERRNO, USER_NOTIF, TRACE, LOG, ALLOW). The existing filter allows exec, so
  USER_NOTIF wins for exec, and its EPERM and ENOSYS verdicts still beat
  USER_NOTIF for the calls it refuses, so nothing it refuses becomes reachable.
- The listener fd comes back in the init. The supervisor must be OUTSIDE the
  sandbox (a step in the filtered process cannot supervise itself), so the fd has
  to reach the root: a `socketpair(AF_UNIX)` inherited from the root with
  `SCM_RIGHTS` (no filesystem path, no network), or `pidfd_getfd`. The root
  already passes the liveness pipe the same way. `redirectStdinAndCloseFDs` marks
  descriptors from 3 up close-on-exec, so the step does not inherit the listener
  (VERIFIED in the code).
- Fail-closed: if the supervisor dies, the listener closes, and pending and later
  notified calls fail (ENOSYS) instead of running unrecorded (INFERRED,
  seccomp_unotify(2)).
- strace as the step's wrapper is unaffected: it is itself exec'd under the filter
  (one notification), and `strace -f`'s children exec as normal (INFERRED).

### 3.2 The protocol

1. The kernel stops the thread at `execve` entry and queues a notification
   (`SECCOMP_IOCTL_NOTIF_RECV`): `pid`, and `seccomp_data` with the argument
   pointers.
2. The supervisor reads the pathname from `/proc/<pid>/mem` at `args[0]`, then
   opens the file with `openat2(dirfd of /proc/<pid>/root, path, RESOLVE_IN_ROOT)`:
   the lookup happens in the STEP's root, not the supervisor's (a plain
   `/proc/<pid>/root/<path>` string would resolve absolute symlinks in the
   supervisor's root). This is the PIN: the inode cannot be freed while we hold it.
   It records `(dev, ino)` from `fstat`.
3. `SECCOMP_IOCTL_NOTIF_ID_VALID`, then `SECCOMP_IOCTL_NOTIF_SEND` with
   `SECCOMP_USER_NOTIF_FLAG_CONTINUE` (5.5): the kernel runs the exec itself.
4. CONFIRM: poll `stat("/proc/<pid>/exe")` until its `(dev, ino)` equals the pinned
   one. Before the exec completes `/proc/<pid>/exe` is the old image, so a
   non-match means "not yet", until a bound.
5. Only then read the head from the pinned fd, and `stat` `/proc/<pid>/exe` again
   after the read: the head counts if the process was alive with that inode before
   and after the read.

### 3.3 The TOCTOU story (INFERRED, kernel source recalled)

`SECCOMP_USER_NOTIF_FLAG_CONTINUE` is documented as NOT a security mechanism: the
kernel re-reads the user pathname after the supervisor answers. Two races follow,
and the confirm step closes both; the first read does not.

- Path swap. Another thread of the step rewrites the pathname string, or swaps a
  symlink, between the supervisor's read and the kernel's lookup. The kernel then
  runs a file the supervisor did not pin. Defence: step 4. The pinned `(dev, ino)`
  is compared with the one the process is actually running (`/proc/<pid>/exe`). A
  mismatch means refuse. The supervisor needs no trust in the string it read.
- Content swap. The step rewrites the SAME inode between the supervisor's head
  read and the kernel's read. Defence: take the head from the pinned fd only AFTER
  the confirm. The kernel denies writes to a program it opened for exec
  (`deny_write_access` in `do_open_execat`, taken at the open, before the headers
  are read) for as long as the process runs: the `ETXTBSY` a write to a running
  binary gets. So once the process is alive and `exe` matches, the bytes in the
  inode are the bytes the kernel loaded, provided the process is still alive and
  `exe` still matches after the read (step 5). A head read before the exec, if one
  is made, is advisory only.
- A process that exits before the confirm leaves nothing to confirm: its binary
  could have been rewritten after the exit. REFUSED, as today. A fast-exiting
  binary is therefore not helped; only a binary alive for the poll interval is. Go
  test binaries run for far longer than the poll.
- A second exec in the same pid before the confirm makes `exe` differ from the
  pin; the first exec is refused. Conservative, and rare.
- pid reuse: the pid is stable while the thread is stopped in the notification
  (`NOTIF_ID_VALID` says so). Taking a `pidfd` at that moment and using
  `pidfd_send_signal(..., 0)` as the liveness check removes any doubt later.
- `execveat` with a relative path needs the cwd or the dirfd of the step; reading
  `/proc/<pid>/cwd` and `/proc/<pid>/fd/<n>` is the same trust as for `exe`.
- One exec at a time per thread group. `/proc/<pid>/exe` belongs to the process,
  not the thread, and a multithreaded step can have two threads in `execve` at
  once (the first to complete replaces the whole group). The supervisor must not
  answer a second notification for a thread group until the first is confirmed
  or has timed out; otherwise a confirm could be satisfied by the wrong exec.
- A failed exec looks confirmed. If the process already runs the file it names
  (a re-exec of itself) and the exec fails, `exe` still equals the pin. Success
  or failure of the exec comes from the TRACE (the strace line's result), never
  from the supervisor alone, and sidecar entries are matched to trace execs by
  per-pid ORDER (the nth exec of pid P in the trace takes the nth sidecar entry
  for P), not by path.
- `prctl(PR_SET_MM, PR_SET_MM_EXE_FILE, fd)` re-points `/proc/<pid>/exe`. It needs
  `CAP_SYS_RESOURCE` (and, since 5.9, `CAP_CHECKPOINT_RESTORE`) in the initial
  user namespace, and the step has dropped every capability and runs with
  `no_new_privs`, so it is closed (INFERRED). The experiment PROBEs it from every
  environment (`prctl PR_SET_MM_EXE_FILE=<errno>`); it must read
  `operation_not_permitted` in `sandbox`.
- `execveat(fd, "", AT_EMPTY_PATH)` (fexecve) has no path to read: the notification
  carries an empty string and a descriptor. The first version REFUSES it: no pin is made, so no sidecar entry exists, and an `under-tmp` binary exec'd that way stays refused as today. Handling it would mean opening `/proc/<pid>/fd/<n>` from the
  supervisor; the experiment logs whether that open works
  (`execveat-empty-path(... open_proc_fd=...)`).
- A path that names a FIFO or a device must not block the supervisor: the pin
  opens with `O_NONBLOCK` (or `O_PATH` and then a checked reopen), inside a
  per-notification deadline, and refuses anything that is not a regular file. A
  supervisor that can be stalled by a step stalls every exec.

### 3.4 Scope

ELF binaries only. A `#!` script under the tmpfs stays refused: the notification
is for the script's `execve` only, the interpreter's open is not notified, and
after the exec `/proc/<pid>/exe` is the interpreter, which no longer equals the
pinned script, so there is no confirmation. (A script whose interpreter is a
tracked or host file is unaffected; it is not the `under-tmp` case.) The dynamic
loader's path comes from the confirmed head's PT_INTERP exactly as
`elfInterpreter` does today; `/proc/<pid>/maps` shows the loader that really
mapped, which could cross-check it (the experiment logs it as `ld_maps`).

### 3.4a Side benefit

The sidecar records the file the kernel actually ran (`dev`, `ino`, confirmed
through `/proc/<pid>/exe`), and the notification also carried the path string the
step passed. strace prints the entry-time string too; when the two disagree about
which file was executed (a path or symlink swapped in the window of section 3.3)
the classifier can refuse, which a trace-only reading can never notice.

### 3.5 Cost (INFERRED; the experiment measures it)

About ten system calls and two context switches per exec in the supervisor (`mem`
open and read, `root` open, `openat2`, `fstat`, head `pread`, two ioctls, the
confirm `stat`s). Plausibly tens to low hundreds of microseconds against an exec
of roughly a millisecond, and far below what `strace -f` already costs per step.
The `perf per-exec baseline_us=... with_pin_us=...` line is the number.

## 4. Recommendation

B. A is unlikely to be available to an unprivileged helper (section 2.2), cannot
cover directories that appear during the run, and gives file handles rather than
bytes (section 2.3). B reads the real head, fits the existing filter stack and the
root/init split, and its races are closed by a confirm that fails closed.

Gate: do not build B until the CI log shows, in the `sandbox` and `userns`
environments, `notif NEW_LISTENER accepted` and an `elf-live-then-deleted` line
with `pin=ok head=elf` and `confirm=exe-match-after-...us`. The one design point
the experiment cannot reach is the supervisor in the ROOT (outside the sandbox's
mount and user namespaces): the spike's supervisor runs inside the namespace it
supervises. The same calls (`/proc/<pid>/mem`, `/proc/<pid>/root`, `openat2`) work
from a parent when the parent's credentials dominate the target's (same uid, the
parent owns the target's user namespace, the target is dumpable; INFERRED), but
that needs its own test once the listener is wired.

## 5. How the result feeds the classifier

A sidecar the recorder writes beside the trace, replacing the post-run disk read
for this one case.

- File: `<trace>.exechead` (JSON lines, one per CONFIRMED exec), versioned like the
  step sidecar:
  `{"v":1,"pid":N,"path":"/abs/path/in/the/step/root","dev":D,"ino":I,"head":"<base64, at most headLimit bytes>"}`.
  Written by the root's supervisor as confirms arrive, fsynced before the trace is
  released. It is size-bounded like the trace, and an unconfirmed exec simply has
  no line.
- Reader: `classifier.readHead(abs)` gains one case, before the `underTmp`
  refusal: look up `(pid, abs)` of the exec event in the sidecar; on a hit return
  `head, true`. There is no stability re-check (`checkStability`) for such a head:
  it was read from the pinned inode while the kernel held it write-denied.
  `loadChain` is unchanged: the same ELF and shebang walk runs on the sidecar's
  bytes, and `c.observe` of the loader is as today.
- A miss is exactly today's behaviour: `exec-unreadable:under-tmp`. A malformed,
  duplicated or oversize sidecar is `trace-incomplete` as a whole (fail closed),
  never "no sidecar". A sidecar line with no matching exec in the trace is
  ignored, not trusted.
- `RecorderVersion` bumps: the accept set grows, so every weak key changes (as D44a
  and D47 did).
- Recording needs the listener, so `RunExec` gains a socketpair, the init a
  `seccomp(NEW_LISTENER)` call beside `prctl(PR_SET_SECCOMP)`, and a supervisor
  loop that must outlive the step and is on the critical path of every exec: a
  stuck supervisor stalls the step, so it needs a deadline that kills the step
  rather than answering.
- Placement: the supervisor lives in `kiln record step`, OUTSIDE the trace, not in
  the root helper (`sandbox-exec`'s root). `kiln record step` already owns the
  step sidecar and the trace files, runs as the same unprivileged user, and is the
  process whose death the recorder already treats as a failed recording.
  Everything it does to the step is a cross-user-namespace access from a process
  that owns that namespace; the experiment's `xns` environment measures exactly
  that (section 6).
- A future numbered KILN04 decision (not this spike) must fix: the protocol (the
  notification, pin, continue, confirm and re-read steps of section 3.2, with the
  one-exec-per-thread-group rule), the sidecar format (section 5, with per-pid
  ordering), WHO is trusted to write it (only the supervisor, from outside the
  sandbox; the step can never reach the file), the fail-closed cases (the
  supervisor dies: pending execs fail and the recording is refused; a confirm
  times out: no entry, the exec stays refused; a malformed, duplicated or
  oversize sidecar: the whole action is refused), the pin's non-blocking open, and
  the `RecorderVersion` bump.

## 6. What the experiment logs

All lines start `PROBE: exechead <env> `, where env is `host` (unprivileged,
no namespace), `userns` (uid 0 in a user and mount namespace of its own, with a
tmpfs, no seccomp filter), `xns` / `xns-super` (a child like `userns` that hands
its seccomp listener, over a socketpair with SCM_RIGHTS, to the test process
OUTSIDE that namespace, which is the supervisor; `xns-super` lines are that
process's findings) or `sandbox` (the real step: no capabilities, the real
filter, the fresh tmpfs as `$TMPDIR`). The `xns` child drops every capability
after installing the filter, as the sandbox's init does. The test is gated, and
run only by the manual workflow; when its own process is root or holds
`CAP_SYS_ADMIN` it runs the `sandbox` child alone, and permission events and
mount and filesystem marks are tried only with `KILN_EXECHEAD_SPIKE_MARKS=1` (the
workflow sets it, on an unprivileged runner user).

The cross-namespace question the `xns-super` lines answer: from outside the
step's user namespace, can the supervisor read `/proc/<pid>/mem` and
`process_vm_readv` the path, open the binary through `/proc/<pid>/root`
(`openat2` `RESOLVE_IN_ROOT`, and the plain string form), use `pidfd_getfd`, and
stat `/proc/<pid>/exe`? Until a CI log answers it, section 4's "INFERRED" on
this stands.

| line | answers |
| --- | --- |
| `kernel=... uid=...` | which kernel |
| `id uid= CapEff= NoNewPrivs= Seccomp=` | what the env is |
| `fanotify_init refused: <group>=<errno> ...` and `accepted: ...` | which init flag sets the kernel takes in that env |
| `fanotify_mark refused: <group>/<inode, mount or fs>=<errno>` | the mark rules (EPERM, EINVAL, ...) |
| `fan <group>/<kind> marks=a/b run=ok,ok bin{n= mask= pid0= [names]} script{...}` | events for the exec'd binary, ld.so (a name ending `ld-linux...`) and the `#!` interpreter (`dash`); `pid0` counts events with pid 0 |
| `notif NEW_LISTENER refused` or `accepted` | whether B's listener installs on top of the filter |
| `notif perf per-exec baseline_us= with_pin_us= n=` | per-exec cost |
| `notif case=elf-live-then-deleted path= pin= head= interp= pin_us= confirm= etxtbsy= ld_maps= head_same=` | B for the real scenario (made, run, deleted); `etxtbsy` should be `text_file_busy` |
| `notif case=elf-fast-exit ...` | a binary that exits before the confirm: `confirm=gone:...` is the expected refusal |
| `notif case=script ...` | the script: `confirm=exe-mismatch exe="/usr/bin/dash"` expected |
| `notif case=fexecve path="" pin="execveat-empty-path(dirfd=3 flags=0x1000 open_proc_fd=ok)" ...` | whether fexecve notifies, with what, and whether `/proc/<pid>/fd/<n>` opens |
| `prctl PR_SET_MM_EXE_FILE=<errno>` | `operation_not_permitted` in `sandbox` closes the exe re-pointing hole |
| `xns listener sent`, `xns capset-empty=ok CapEff=0...` (child) and `xns-super listener fd received ...` | the listener crossed the namespace boundary; the child ran capability-free |
| `xns-super notif outside-supervisor path="..." pin="ok" head="elf" ... confirm=exe-match-after-Nus etxtbsy=text_file_busy ... process_vm_readv=<ok(NB)\|errno> open_proc_root_string=<ok\|errno> pidfd_open=... pidfd_getfd(fd0)=<ok\|errno>` | the cross-namespace accesses; `pin="mem:..."`, `"root:..."` or `"openat2:..."` would show which one the kernel denied |
| `xns-super no listener fd received`, `harness exit=...`, `watchdog fired` | the experiment itself failed to run |
