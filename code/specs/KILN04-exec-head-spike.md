# KILN04 exec-head spike: recording an exec'd binary's head at exec time

## Status

ADOPTED as KILN04 decision D48 ("exec heads recorded at exec time"): the design
of section 3 (seccomp user notification, supervisor in `kiln record step`) is the
one to build, and D48 is the normative text. The spike itself remains a design
note and an experiment with no production change. It was run twice on CI's
kernels (workflow runs 45342132127 and 45346779343, ubuntu 24.04 `7.0.0-1014-aws`
and 26.04 `7.0.0-1020-azure`) and reviewed adversarially (verdict GO). Claims the
two runs answered are marked VERIFIED (run 45346779343); the ones still open are
marked INFERRED or OPEN and are listed in section 7. The experiment is
`TestExecHeadSpike` in `programs/go/kiln/exechead_spike_linux_amd64_test.go`; it
logs `PROBE: exechead` lines on CI's kernels and asserts nothing about the
mechanisms. It is gated (it skips unless `KILN_EXECHEAD_SPIKE=1`), its name has no
"Sandbox" in it so the kiln-sandbox job never runs it, and it is not in
`ci/assert_sandbox_tests.py`; the manual workflow
`.github/workflows/kiln-exechead-spike.yml` runs it on both runner images and
puts the PROBE lines in the step summary. This note says,
for every claim, whether it was VERIFIED (read in this repository's code, or
observed in a CI run, named) or INFERRED (recalled from kernel documentation and
source, not checked by a run). Where a CI log disagrees with this note, the log
wins and this note needs an edit. The one open safety question is the
`PR_SET_MM_MAP` exe re-pointing hole (section 3.3): the spike's probe passes an
all-zero map and cannot answer it; PR #19902's valid-map probe is the evidence,
and classifier consumption must not merge before it.

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

### 2.1 What the event is (INFERRED; the events seen agree: VERIFIED run 45346779343)

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

### 2.2 The permission rules, which are the problem (VERIFIED run 45346779343)

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
  not a member. Whether recent kernels (the CI's 7.0.x, 26.04) relax this was
  UNKNOWN; VERIFIED (run 45346779343): as the unprivileged runner user, in `host`
  and `sandbox`, `fanotify_init` is refused for the privileged classes
  (`notif=operation_not_permitted perm=operation_not_permitted`), only the FID
  groups are accepted, and every mount and filesystem mark is refused
  (`operation_not_permitted`); only the `userns` child, which holds capabilities
  in its own namespace, gets mount and filesystem marks, and that is not the
  sandbox's step.
- The root helper on CI is an unprivileged user, so it is in the unprivileged
  case. The sandbox's init has `CAP_SYS_ADMIN` in its own namespace until
  `dropPrivileges`, which is not enough for `capable()`.

### 2.3 Pathname and key (VERIFIED run 45346779343: events carry a `fid`, and `name=` only with DFID_NAME, never a path or bytes)

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

### 3.1 Fit with the current filter (VERIFIED from the code and run 45346779343, except fail-closed, INFERRED)

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
  (one notification), and `strace -f`'s children exec as normal (VERIFIED run
  45346779343: `xns` `strace=attached strace-exec-lines=1`, both images).
- A second filter with `NEW_LISTENER` installs on top of the first (VERIFIED:
  `notif NEW_LISTENER accepted` in `host`, `userns`, `sandbox`, and the listener
  crossed a user namespace over `SCM_RIGHTS`, `xns-super listener fd received`).

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

### 3.3 The TOCTOU story (kernel source recalled, INFERRED; the preconditions and the confirm are VERIFIED run 45346779343 where marked)

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
  binary gets (VERIFIED run 45346779343: `etxtbsy=text_file_busy` on every
  live-binary line). So once the process is alive and `exe` matches, the bytes in the
  inode are the bytes the kernel loaded, provided the process is still alive and
  `exe` still matches after the read (step 5). A head read before the exec, if one
  is made, is advisory only.
- Precondition: no writer that predates the exec. The write-deny of the content
  argument holds only if the kernel refuses to exec a file somebody already has
  open for writing (`ETXTBSY` from `deny_write_access`), and refuses a file with
  a writable shared mapping left behind by a closed descriptor. A writer that
  existed before the exec and survives it could change the bytes after they were
  loaded, and a head read from the pinned descriptor would no longer say what
  ran. The experiment's evidence is the line `exec-with-writer fd=<errno|allowed>
  shared-map=<errno|allowed>` (case A: an `O_WRONLY` descriptor held; case B: an
  `O_RDWR` descriptor mapped `MAP_SHARED` with `PROT_WRITE` and then closed), in
  `host` and `userns`. If EITHER reads `allowed`, this section's trust argument
  fails as written and the design must also refuse an exec whose file had a writer
  (a check of `/proc/<pid>/fdinfo` and the maps is not enough; it would need the
  inode's write count, which user space cannot see) before B can be built.
  VERIFIED run 45346779343, both images, `host` and `userns`: `exec-with-writer
  fd=text_file_busy shared-map=text_file_busy`. Neither reads `allowed`, so the
  argument stands as written.
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
  for P), not by path. EVERY notification gets an entry, a failed exec included
  (a PATH search is several `execve` calls that fail with ENOENT before one
  succeeds), with a placeholder that says "not pinned" where nothing was pinned;
  otherwise the nth-exec pairing slips by one for the rest of that pid. Counting
  starts at the step's OWN first exec: the helper's earlier execs (the root's
  start of the init and the init's exec of the step) appear in the trace but
  precede the listener, and have no notification.
- Confirm by thread-group id. The confirm uses the thread-group id captured when
  the notification arrived (the `pid` the kernel reports is the thread's tgid in
  the supervisor's pid namespace, to be checked by the probe's `notif-pid=` line),
  never a thread id. A non-leader thread that execs takes over the leader's pid
  and its own tid vanishes from `/proc`; a confirm keyed on the tid would find no
  process.
- `/proc/<pid>/exe` can be re-pointed with `prctl(PR_SET_MM, PR_SET_MM_EXE_FILE,
  fd)` or the map form `PR_SET_MM_MAP` with `exe_fd`. Neither is closed by a
  capability in the INITIAL user namespace: the checks are made against the
  caller's own user namespace (so a process that has `CAP_SYS_RESOURCE` and
  `CAP_CHECKPOINT_RESTORE` in a user namespace of its own can re-point its exe;
  INFERRED, and the `userns` probe lines show it). The step is closed off for a
  different reason: it holds NO capabilities in the sandbox's user namespace (the
  init dropped them all, with `no_new_privs`), and the seccomp filter refuses
  `unshare` and `clone(CLONE_NEWUSER)`, so it cannot make a namespace in which it
  would have some. The experiment PROBEs both forms: `prctl PR_SET_MM_EXE_FILE=` and
  `prctl PR_SET_MM_MAP(exe_fd)=` in `sandbox` (expect `operation_not_permitted`)
  and in the unprivileged `userns` child (the map call is all zeros but for
  `exe_fd`, so EPERM means the capability check failed and EINVAL means it passed
  and validation refused it). RESULT (run 45346779343): `PR_SET_MM_EXE_FILE` is
  `operation_not_permitted` in `host`, `userns` (which holds every capability) and
  `sandbox`; `PR_SET_MM_MAP(exe_fd)` is `invalid_argument` in all three, including
  the unprivileged `host`, so the zero map was refused before any capability check
  and the line does NOT show that the sandbox lacks the capability. ANSWERED by run
  45350985752 (the VALID-map probe of PR #19902, both images, kernels
  7.0.0-1017-azure and 7.0.0-1016-aws): `PR_SET_MM_MAP(valid,exe_fd)` is
  `operation_not_permitted` in `sandbox` and in the unprivileged `host`, with
  `exe` unchanged, and `device_or_resource_busy` in `userns` (past the capability
  check, which is therefore namespace-relative). The sandbox refuses at the
  capability check, which is D48's gate for classifier consumption; D48 step 3
  also refuses `PR_SET_MM` in the filter as a second layer.
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

### 3.5 Cost (VERIFIED run 45346779343)

About ten system calls and two context switches per exec in the supervisor (`mem`
open and read, `root` open, `openat2`, `fstat`, head `pread`, two ioctls, the
confirm `stat`s). Plausibly tens to low hundreds of microseconds against an exec
of roughly a millisecond, and far below what `strace -f` already costs per step.
The `perf per-exec baseline_us=... with_pin_us=...` line is the number:
361 to 399, 584 to 666 (`host`), 368 to 390, 590 to 675 (`sandbox`), 373 to 422,
574 to 670 (`userns`), that is +22 to +96 microseconds per exec, about +30 to +90.

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
- The real supervisor must open the pin with `RESOLVE_IN_ROOT` relative to the
  tracee's root (section 3.2 step 2) and must not treat `CONTINUE` as a security
  decision (the user-notification TOCTOU of section 3.3); the confirm is the
  decision. Its BPF arch check must also KILL a non-x86_64 architecture, where
  the spike's filter allows it (the spike only ever runs on linux/amd64).
- Removal: the `TestMain` dispatch cases (`spikeCommand`, `spikeFexecCommand`)
  added to `sandbox_linux_amd64_test.go` must be removed together with the spike
  (`exechead_spike_linux_amd64_test.go`, the workflow and this note's experiment
  sections); they dispatch to code that would no longer exist.
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
stat `/proc/<pid>/exe`? ANSWERED (VERIFIED run 45346779343, both images): `process_vm_readv=ok(64B)`,
`open_proc_root_string=ok`, `pidfd_getfd(fd0)=ok`, pin `ok`, and
`confirm=exe-match-after-1066us` to `1112us` from outside, with the child
capability-free (`CapEff=0`) in its own user and pid namespace.

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
| `exec-with-writer fd=<errno\|allowed> shared-map=<errno\|allowed>` (`host`, `userns`) | whether exec refuses a file with an existing writer (the precondition of section 3.3); `text_file_busy` expected for `fd` |
| `prctl PR_SET_MM_MAP(zero-fields,exe_fd)=<errno>` | the first map probe (all fields zero). INCONCLUSIVE: run 45346779343 showed `invalid_argument` in `host`, `userns` and `sandbox` alike, including the unprivileged `host`, so address validation runs before (or without) the capability check. Kept only for comparison |
| `prctl PR_SET_MM_MAP_SIZE=<n> sizeof=104 match=<bool>` | the kernel's `struct prctl_mm_map` size against the probe's struct (the query needs no capability); the valid probe is skipped on a mismatch |
| `prctl PR_SET_MM_MAP(valid,exe_fd)=<errno\|ok> exe_after="<readlink /proc/self/exe>"` | the real question: the map filled from the process's own `/proc/self/stat` (start_code, end_code, start_stack, start_data, end_data, start_brk, arg_start, arg_end, env_start, env_end), `brk(0)` and `/proc/self/auxv`, with `exe_fd` an `O_RDONLY` descriptor of a COPY of `/usr/bin/true` in the child's temp dir. Run last, in the short-lived re-executed child of `host`, `userns` and `sandbox` (never in the test process); `host` is the control. Outcomes in section 7 |
| `fdpath child inherited=<ok\|errno\|not-a-socket>` and `child listener-sent=<ok\|errno>` | the child under `strace -f` (or alone): did descriptor 3 arrive, and did the listener go back over it |
| `fdpath strace=<attached\|absent> inherited=<ok\|errno> listener=<received\|errno> confirm="<confirm> etxtbsy=<..> head_same=.. alive_after_reread=.."` | descriptor-path probe: the test process makes a socketpair, starts `strace -f <test binary> fdpath` with one end deliberately inherited on descriptor 3, the child installs the notify filter and sends the listener back over it, and the test process, as supervisor, pins and confirms the child's exec of a binary it made, ran and deleted. `strace=absent` means no execve in the trace file (strace missing or unable to trace) and the child ran alone |
| `fdpath sandbox-path="..."` and `fdpath-sandbox scan inherited=<ok\|bad_file_descriptor> open_3_63=<n> sockets=<n> fds="..."` | the REAL sandbox root and init, with the socketpair end as `ExtraFiles` of the `sandbox-exec` command. The first line is the static reason from the code why it cannot be passed; the second is what the step (as `probe-bin`) holds: expected `sockets=0`, hence `inherited=bad_file_descriptor`. The sandbox case is not run under strace here (that needs the `TestSandboxRealStrace` harness), so strace inheritance and sandbox inheritance are probed separately |
| `xns strace=<..> strace-exec-lines=<n> run=<errno\|ok> strace_stderr="<quoted, 512B>" tracee_exit="exited with N"\|"killed by SIG"\|none` | the strace run in the xns child, with strace's captured stderr and the tracee's own exit read from the trace |
| `xns strace-trace first_execve="..." first_stderr_write="..."` | the first `execve(` and the first `write(2,` line of that trace: what the tracee was and what it said |
| `prep true_real="..." sleep_real="..." multicall=<bool>` | whether the image's `true`/`sleep` resolve to a differently named (multi-call) executable |
| `xns-super notif-pid=<pid[,pid]> strace=<attached\|absent> strace-exec-lines=<n>` | which pid the OUTSIDE supervisor sees (the `xns` child is in a pid namespace of its own, and logs its own `pid inside the namespace=`), and whether a seccomp notification and a ptrace stop (`strace -f -o`) coexisted on the same execs |
| `notif case=fexecve path="" pin="execveat-empty-path(dirfd=3 flags=0x1000 open_proc_fd=ok)" ...` | whether fexecve notifies, with what, and whether `/proc/<pid>/fd/<n>` opens |
| `prctl PR_SET_MM_EXE_FILE=<errno>` | `operation_not_permitted` in `sandbox` closes the exe re-pointing hole |
| `xns listener sent`, `xns capset-empty=ok CapEff=0...` (child) and `xns-super listener fd received ...` | the listener crossed the namespace boundary; the child ran capability-free |
| `xns-super notif outside-supervisor path="..." pin="ok" head="elf" ... confirm=exe-match-after-Nus etxtbsy=text_file_busy ... process_vm_readv=<ok(NB)\|errno> open_proc_root_string=<ok\|errno> pidfd_open=... pidfd_getfd(fd0)=<ok\|errno>` | the cross-namespace accesses; `pin="mem:..."`, `"root:..."` or `"openat2:..."` would show which one the kernel denied |
| `xns-super no listener fd received`, `harness exit=...`, `watchdog fired` | the experiment itself failed to run |

## 7. Open questions the experiment cannot settle

- The `xns` child has a user, mount and pid namespace, a notify-only filter and no
  capabilities, but no `pivot_root` and not the sandbox's real filter. The
  `sandbox` environment has the real step but its supervisor is inside it.
  A supervisor outside the REAL sandbox needs the listener wired through
  `sandbox-exec`, which is production code. If the `xns-super` lines show
  cross-namespace access working, that wiring is the next step and its test is
  the real answer.
- ANSWERED, no longer open: `strace=attached strace-exec-lines=1` and
  `pid inside the namespace=1` in both runs (a notification and a ptrace stop
  coexist; the pid namespace was in effect, so the pids the supervisor saw,
  `notif-pid=...`, are evidence for the real topology).
- ANSWERED (run 45350985752): the `PR_SET_MM_MAP` exe re-pointing hole (section
  3.3). The valid-map probe returned `EPERM` in the sandbox on both images.
- OPEN, and amended by D48 after review: the confirm of section 3.2 step 4 as
  written ("`exe` equals the pin") passes at once for a process that already runs
  the pinned file, and a swap to another file by a second thread then gives a false
  confirm. D48 requires an observed TRANSITION instead: the pre-exec `(dev, ino)`
  of `/proc/<tgid>/exe`, taken before CONTINUE, must differ from the pin, and the
  link must then change to the pin; an equal pre-exec identity is `same-image`
  (refused; a self re-exec stays refused), a change to anything else is
  `mismatch` at once, and `timeout` is only a link that never changes. No probe
  exercises the transition rule; D48 step 4's tests do.
- OPEN: a path or symlink swap in the window between the supervisor's read and the
  kernel's (section 3.3, "Path swap") has no probe; the argument is the confirm's
  `(dev, ino)` comparison, and the real-kernel tests of D48 step 4 exercise it
  (D48 adds the rename-by-another-process variant and a pidfd for the process).
- OPEN: the write-deny and CONTINUE behaviour is VERIFIED only on the 7.0.x CI
  kernels; D48 records the kernel release in the sidecar header and refuses a series
  the real-kernel tests have not run on.
- OPEN: the fail-closed behaviour of a dead supervisor (pending and later execs
  fail with `ENOSYS`) is documented kernel behaviour, not observed here; D48 step 3
  tests it.
- OPEN: that the pid in the sidecar equals the pid strace prints (both are in the
  supervisor's pid namespace by construction); D48 step 4 tests it.
- ANSWERED by run 45350985752, superseding the OPEN items below that came from
  PR #19902: 26.04's coreutils is multi-call (`multicall=true`, `sleep` resolves
  to `/usr/lib/cargo/bin/coreutils/sleep`), and with real applet names the
  strace-attached `xns` run is `run=ok tracee_exit="exited with 0"` on both
  images, so the 26.04 `exit_status_1` was the probe's own harness; the valid-map
  `PR_SET_MM_MAP` results are as in section 3.3; and `fdpath` shows a socketpair
  inherited through `strace -f` carrying the listener back
  (`inherited=ok listener=received`, confirm `exe-match`, `head_same=true`) while
  the real sandbox step holds `sockets=0`. The text below is kept as the record
  of what each outcome would have meant.
- If `strace=` reads `absent` or `strace-exec-lines=0` in the `xns` run (strace not
  installed, or unable to trace under the filter), whether a notification and a
  ptrace stop on the same exec coexist is OPEN: the kiln-sandbox job runs the
  step under strace, so this must be answered before the design is built.
- OPEN (hypothesis, to be confirmed by the next run): the `exit_status_1` of
  `xns strace=... run=` on ubuntu-26.04 (strace 6.19, kernel 7.0.0-1020-azure)
  against `run=ok` on 24.04. The notifications still arrived and confirmed, and
  every other line agreed, so the supervisor and the filter are not implicated.
  strace exits with its tracee's status, so the 1 most likely belongs to the
  TRACEE, and the likeliest cause is the probe's own harness: the image's
  coreutils on 26.04 is the uutils multi-call binary, `/usr/bin/sleep` resolves to
  one executable that picks the applet from `argv[0]`, and the probe copied it to
  `bin-sleep2`, an unknown applet, which prints an error and exits 1 at once.
  (The other cases ignore the program's status; they only watch the pin and the
  confirm, so they did not show it. A side effect to keep in mind: on that image
  the `elf-live-then-deleted` binary may have exited at once instead of living
  0.5 s, and its `confirm` then raced the exit.) The probe now names its copies
  `true` and `sleep` (the second `sleep` in a directory of its own), prints
  `prep ... multicall=<bool>`, and splits strace's stderr from the tracee's exit
  (`strace_stderr=`, `tracee_exit=`, and the first `execve(` / `write(2,` lines of
  the trace). Confirmed if 26.04 prints `multicall=true`, and `run=ok
  tracee_exit="exited with 0"` after the rename. Alternatives the new fields would
  show instead: `strace_stderr` starting `strace:` (strace 6.19 itself failing
  under `no_new_privs` plus the notify filter, a ptrace restriction in the pid
  namespace, or `--seccomp-bpf` interplay, none of which the xns child sets), a
  `first_execve` with `ENOENT` or `EACCES` (a missing or non-executable file in
  the tmpfs), or `tracee_exit` of a signal.
- `PR_SET_MM_MAP` (the exe re-pointing hole). The first probe (`zero-fields`) is
  inconclusive: an all-zero map fails address validation with EINVAL, and run
  45346779343 saw EINVAL even in the unprivileged `host` child, so validation
  evidently comes before the capability check. The valid-map probe supplies the
  process's own addresses and auxv and a descriptor of a different regular file.
  Reading each environment (the order inside the kernel is capability, then
  validation, then the check that the old exe is no longer mapped, so each errno
  says how far the call got):
  - `host` (control, no capabilities anywhere): MUST be `operation_not_permitted`
    or, if the kernel checks the mapping first, `device_or_resource_busy`. If it
    is `ok`, the primitive needs no privilege at all and the confirm cannot be
    trusted in ANY environment: redesign.
  - `sandbox`: `operation_not_permitted` closes the hole for the real step (it has
    no capability in its user namespace and cannot make a new one). If it is
    `ok` the confirm design (`/proc/<pid>/exe` equals the pin) must be
    REDESIGNED, because a step could make `exe` name any file it can open and so
    forge the confirm of a different binary; the defences would be to drop
    `prctl(PR_SET_MM)` in the seccomp filter or to confirm through something the
    step cannot re-point (the `maps` of the executable mapping, `AT_EXECFD`/the
    mapped inode, or the pid's `exe` link cross-checked against the mapped file).
    `device_or_resource_busy` or `invalid_argument` from the sandbox means the
    capability check passed or was not reached: treat it as NOT closed and
    investigate before building, since only `operation_not_permitted` proves the
    check refused.
  - `userns` (uid 0 with a full set in a namespace of its own): the earlier text of
    this note INFERRED success. `ok` confirms that a capability in one's own user
    namespace suffices and the step's closure rests on having none; `operation_not_permitted`
    means the check is against the initial namespace (stronger for us, and
    it would make the earlier inference wrong); `device_or_resource_busy` is as good as `ok`
    for this question: the capability check runs before the "old exe still mapped"
    check, so EBUSY means the call got past the capability with capabilities held
    only in its own namespace (the Go test binary always has its old exe mapped, so
    EBUSY is the likely answer; do not chase it). `exe_after` shows whether the
    link actually moved.
  - A size mismatch (`PR_SET_MM_MAP_SIZE`) makes the valid probe `skipped`: fix
    the struct first.
- NOT COVERED: the sandbox under strace, end to end, with a listener descriptor
  passed through both. This spike probes strace inheritance and the sandbox's
  descriptor handling separately; the first real evidence for the combination
  will be the kiln-sandbox tests of the production listener (the dedicated
  inherited socket that `kiln record step` creates, validated as a connected unix
  socket, sent over by the init before the step starts and closed before it runs).
- The descriptor path (`fdpath`). Findings from the code, before any run: the real
  `sandbox-exec` root cannot pass an extra descriptor to the step. `RunExec` marks
  every descriptor of 3 and up close-on-exec with `close_range` before the spawn
  and maps only `/dev/null`, `--stdout-fd`, `--stderr-fd` and the liveness pipe
  into the init; `RunInit` marks 3 and up close-on-exec again before the step;
  and `CheckOutputFD` accepts only a write-only pipe for the two output
  descriptors, so a socket cannot ride in as one of them. Passing the listener
  socket therefore needs a production change (an explicit `--listener-fd` that
  the root maps and the init keeps open as a descriptor the step cannot see:
  closed, or `CLOEXEC`, before the exec), which this spike does not make. The probe
  splits the question: `fdpath strace=... inherited=... listener=...
  confirm=...` runs the inheritance through `strace -f` (and no sandbox) and the
  supervisor round trip on a real inherited socket; `fdpath-sandbox scan
  sockets=0` is the evidence for the sandbox half (`sockets>0` would mean the code
  reading above is wrong). Outcomes: `inherited=ok listener=received
  confirm="exe-match-after-..."` with `strace=attached` means strace -f passes
  the descriptor, a seccomp notification and a ptrace stop coexist on a real
  exec, and the listener round trip works from a process outside the step; the
  remaining work is the root/init plumbing above. `inherited=bad_file_descriptor`
  with `strace=attached` means strace itself closes or does not pass descriptor 3
  (a design problem for strace-wrapped recording: use a pathname socket or a
  descriptor number strace keeps). `listener=` an errno or a `NEW_LISTENER
  refused` in the child means the filter cannot be installed in a traced process.
  An empty `confirm` (`no-execve-notification`) with `listener=received` means
  the supervisor did not see the child's exec: ptrace and seccomp do not
  coexist the way the xns line assumed.
- If a CI log shows the pid namespace was not in effect (the child's
  `pid inside the namespace=1` is absent), the pid the supervisor sees is not
  evidence for the real topology.
