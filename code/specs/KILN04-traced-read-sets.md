# KILN04 — Traced Read-Sets for Legacy BUILDs

## Status

Accepted (2026-11-12), after an adversarial design review (7 blockers and
13 should-fix items) and a security review (2 HIGH, 6 MEDIUM, 3 LOW), all
folded in. This spec replaces the static scans
of [KILN03](KILN03-legacy-keys.md) §6 as the soundness mechanism for
reusing a legacy BUILD's result. It builds on:

- the [KILN02](KILN02-key-encoding.md) key encoding;
- the ledger and trust rules of [KILN00](KILN00-kiln-build-system.md) §10;
- the security boundary of KILN00 §9.6.

It is **shadow only** until the M3 gate (§13): nothing is skipped because
of this spec alone. Implementation is staged in five PRs (§16).

## 1. Why tracing

A legacy BUILD runs in place and can read anything. KILN03 first tried to
bound what it reads by **reading the BUILD's files**, scanning for `../`,
`dirname`, `$0`, `Path(__file__)`, `CARGO_MANIFEST_DIR` and so on. On the
real graph (5,555 packages) those rules left 89% of packages uncacheable,
and each security review still found idioms they missed. A static scan
cannot win: what files a program touches cannot be decided from its text,
and the list of idioms has no end.

Tracing asks the question the other way round. Instead of predicting what
a BUILD *might* read, it **watches what the BUILD did read** on a real run
and records those reads with the pass. A later run may reuse the pass only
if every one of those reads would see exactly the same thing again.

```
static scan:   text of the BUILD  ──guess──▶  what it might read   (unbounded)
tracing:       one real run       ──watch──▶  what it did read     (exact, per run)
```

Every pass carries its own evidence. Static scans stay as **advisory
hints** (KILN03 §6) for people and for the M4 translator. They make no
soundness or security claim.

## 2. The soundness argument

Take a program that is deterministic given what it observes: the bytes it
reads, the files it finds missing, the directory listings, the
environment, the clock. Suppose two runs start from the same program, and
**every observation the first run made returns the same answer in the
second**. Then the second run takes the same path, step by step, and
reaches the same result. It cannot read anything the first run did not,
because its first divergent read would need a divergent earlier
observation.

So a recorded pass may be reused when both of these hold:

1. everything the tracer **cannot** see is held fixed by the **weak key**
   (§3): the BUILD text, the platform, the runner image, the toolchains,
   the environment, the executor and the recorder;
2. everything the tracer **can** see is listed in the **manifest** (§5),
   and each listed entry has the same digest now as when it was recorded.

The argument holds only if the premises below hold. Each one has a rule.
Where a premise cannot be checked, §14 names it.

| Premise | Rule |
|---|---|
| The trace is complete: every file access, probe, listing, exec and connection by every descendant process is seen. | §4.3, §4.4, §6 (`trace-incomplete`) |
| Absences count: "not found" and directory listings are observations. | §5.2 |
| Nothing unseen varies: the environment is scrubbed and keyed. Time and randomness are the flaky-test problem that KILN00 §10.5 already handles. | §4.2 |
| No other action's leftovers were read. | §7 (`reads-product`) |
| Reads outside the repository come only from what the weak key pins, unmodified since the image booted. | §5.3 (`host-read`) |
| The network is not used, or is used only to fetch pinned dependencies. | §5.4 (`network`) |
| The action cannot forge its own trace. | §4.3 |

## 3. Two keys

A traced legacy action has two keys. Both are ordinary KILN02 keys, schema
1, with **semantics version 2**. Version 0 is the static legacy key of
KILN03, and version 1 is reserved for KILN01.

### 3.1 The weak key

The weak key selects which manifests to try. It covers exactly what the
tracer cannot see:

| KILN02 section | Value |
|---|---|
| `platform` | OS, arch |
| `host` | `Host.ImageID()`, always present. An image ID containing `unknown` (a self-hosted runner) means no recording and no lookup. |
| `action` | target = P's `rel_path`; phase = `test` |
| `recipe` | `LegacyRecipeFiles` of the single BUILD variant this OS runs (KILN03 §3.4) |
| `inputs`, `reads` | empty |
| `consumed` | `tool:build-tool` → the executor's source digest (KILN03 §3.4); `policy:env.json` and `policy:host-roots.json` → their content digests; `policy:git` → the digest of every `.gitattributes` and `.gitmodules` file in the repository, since Git reads them while checking out and no trace sees that |
| `tools` | the pins (KILN03 §4) of the languages in P's **static closure**, plus the always-included set (`sh`, `cc`, `ld`, `strace`) |
| `env` | the whole environment of the traced run (§4.2); plus `kiln:root` → the absolute checkout path; `kiln:recorder` → the recorder version (an integer bumped by hand on any change to §4–§7); and `kiln:executor-args` → the executor's canonical flags |

`toolchains.json` itself is deliberately not in the weak key, so bumping
one language's pin does not invalidate every other language. A read under
a `tool` root whose tool is not in `tools` gives
`host-read:tool-not-keyed:<tool>`. Names containing `:` are reserved for
Kiln, and `env.json` may not list one.

### 3.2 The strong key

The pass ledger stores strong keys. A strong key is the weak key with the
manifest's entries added, each paired with its digest **as it is now**:

| KILN02 section | Value |
|---|---|
| all sections except `consumed` | identical to the weak key |
| `consumed` | `weak:` → the 32-byte weak key, plus one entry per manifest entry (§5.2) |

Only strong keys contain a `weak:` entry, so a weak key can never equal a
strong key. The `tool:` and `policy:` entries reach the strong key through
the weak key, so they are not repeated.

### 3.3 Lookup

At plan time, for package P (§9):

```
W = weak_key(P)
for M in manifests[W], newest first (at most K = 4 distinct ones):
    S = strong_key(W, entries(M) with current digests)
    if ledger has a pass for S:  predicted hit; stop
otherwise: miss
```

Lookup runs no BUILD. It reads only the tree, the manifest store and the
pass ledger. Trying several manifests matters because one BUILD can take
different paths: a feature probe that flips, or a test filtered by a
changed file. Each path that passed is recorded once.

**Trust boundary on PR runs.** A PR can change Kiln, its policy files and
the old build tool. So:

- Any run whose result can skip a test (`kiln-gate`, M3) runs the Kiln
  binary **built from the base revision** (the merge base with the target
  branch). It reads `env.json`, `host-roots.json`, `never.json`,
  `pinned.json`, `toolchains.json` and the key vectors from that same
  revision, never from the working tree.
- The weak key's `tool:` and `policy:` digests, however, are computed
  from **the files that will actually run or apply in this job**. If the
  PR changes any of them, the weak key changes and lookup misses.
- The graph export and static closure come from the PR tree. They choose
  which tool pins enter `tools`, which is safe because a different choice
  only changes the weak key.

## 4. Recording

### 4.1 Who records

Only **trusted** runs (KILN00 §10.2) record, only on **Linux x64**, and
only on a **forced full build**, where every package runs. A PR run never
traces and never writes; it only looks up.

In M2, recording runs as dedicated, non-gating jobs (`kiln-record` shards
plus one `kiln-record-merge`, §7) on the scheduled full build. A BUILD
that behaves differently under the tracer or the scrubbed environment
therefore cannot break `main`.

macOS and Windows record nothing in v1 (open question 1); their packages
always run.

### 4.2 The run is scrubbed

The tracer cannot see `getenv`. So every traced step runs with its
environment **replaced** by exactly:

- the variables `env.json` allows (KILN03 §5), with their values;
- `HOME`, and `TMPDIR` if set;
- `PATH`, **constructed** from the roots of exactly the tools in the weak
  key's `tools` (in a fixed order), never inherited, so recording and PR
  jobs produce the same value and a pin bump for an unrelated language
  does not change it. `PATH` stays in the key;
- `LANG=C.UTF-8` and `TZ=UTC`, which override `env.json`.

That exact environment is the weak key's `env` section. Two more inputs
are fixed: standard input is `/dev/null`, and the working directory is the
package directory, as the executor sets it today.

### 4.3 The tracer and the sandbox

**Wiring.** The old build tool gains two flags:

- `-exec-wrapper <argv>`: a prefix for each `sh -c <line>` it runs, with
  `KILN_ACTION` and `KILN_STEP` set in the wrapper's environment (not the
  action's);
- `-emit-results <json>`: writes `kiln-legacy-results/1`, the status and
  step count for each package.

Recording runs the executor with `-prune-leaf-targets` and `-clippy` off;
both are recorded in `kiln:executor-args`. Each step produces one trace,
and each package produces one manifest. An action **passed** if and only
if its status is `built` and every step has a complete trace whose
top-level process exited with 0.

**The command.** v1 uses **`strace`**, whose exact version comes from the
runner image and is pinned and probed like any tool. It is a CI tool like
`git`, not a Go dependency. The command for one step:

```
strace -f --quiet=attach -y -s 4096 --kill-on-exit --pidns-translation \
       -e signal=all \
       -e trace=%file,%process,%network,getdents,getdents64,fchdir,\
io_uring_setup,open_by_handle_at,bpf,setns,unshare,fsopen,fsmount,\
fsconfig,fspick,open_tree,move_mount,mount_setattr,ptrace,\
process_vm_readv,pidfd_getfd,inotify_add_watch,fanotify_mark,\
fchmod,fchown,fsetxattr,fremovexattr,\
name_to_handle_at,mount,umount2,pivot_root,chroot,clone3,\
io_uring_enter,io_uring_register,process_vm_writev,userfaultfd,\
kexec_load,kexec_file_load,perf_event_open,add_key,request_key,keyctl \
       -o <trace-file> -- kiln sandbox-exec --config <cfg> \
       --stdout-fd <N> --stderr-fd <N> -- sh -c <line>
```

The authority for this argv is `sandbox.StraceArgs(traceFile)` (§4.7, "The
strace argv"); the block above is its output with the file name left open. The
last ten names of the `trace=` set are the calls the sandbox filter refuses
beyond the first twenty of §4.4 (§4.7): they are named explicitly so that a
step's attempt to make one shows up in the trace (and is refused by the
parser) instead of depending on which class strace puts it in. The six before
them are in the classes already; they are listed too so that the set names
**every** refused call, which a test checks.

- `--pidns-translation` (strace 5.9 or later) makes strace print the pid a
  tracee gets back from `fork`, `vfork`, `clone` and `clone3` **inside the
  sandbox's PID namespace** with its host pid:
  `= 2 /* 5030 in strace's PID NS */`, and in `clone3`'s output struct
  `parent_tid=[2 /* 5030 in strace's PID NS */]`. Every line is prefixed with
  the host pid, so without the translation the local number cannot be matched
  to the child's lines (§4.5).
- `--kill-on-exit` makes strace kill its tracees if it dies. Without it a
  strace that is signalled detaches them and the step runs on untraced. The
  sandbox also puts the step in a session and process group of its own, so
  `kill(0, sig)` from the step cannot reach strace (§4.7).

- `-f` follows every `fork`, `vfork`, `clone` and `clone3`.
- `--quiet=attach` hides attach and detach chatter but keeps the
  `+++ exited` / `+++ killed` lines and personality-change messages.
  (`-qq` would remove the exit lines.)
- `-y` annotates every file descriptor with the kernel's resolved path,
  including the one an `open` returns.
- Output goes to one file, not one per process, so events stay in the
  order the tracer observed them. Every `<unfinished …>` line must have a
  matching `<… resumed>` line.
- **No `-v`.** Without it, `execve` prints `/* N vars */` instead of the
  environment.
- `-s 4096`. Any string whose printed form is truncated (ends in `...`)
  gives `trace-incomplete`.
- No `--seccomp-bpf` until it has been reviewed (open question 2).
- **Disk.** (Amended by stage 4 PR 2b-2a, decision P12; G8 of the design
  review.) strace does **not** run with `RLIMIT_FSIZE`: an rlimit is
  inherited by the step, so a BUILD that wrote a large file would die of
  `SIGXFSZ`, and a trace is too large for a tmpfs. The trace goes to a file
  in the record directory on disk, written by the supervisor from a pipe
  strace writes to (P12), so the cap is exact (1 GiB: the supervisor keeps the
  bytes up to it and kills strace) and a failed write (`ENOSPC`, `EIO`) is
  seen at once and kills strace too; the supervisor refuses to start a step
  when free space on the file system is under 4 GiB. Each gives a truncated
  trace, `trace-incomplete:size` (an unknown write set). The cap is revised
  from the M2 measurements.
- **Completeness of the run.** An action passed only if:
  - (Amended by decision P12; G7 of the design review.) strace was not
    killed by a signal, its exit status equals the traced root's, and it
    wrote nothing to its own standard output or standard error (the step's
    descriptors 1 and 2 are separate pipes, §4.7). A non-zero status that
    equals the root's is the step's own failure (`failed`), not an unclean
    tracer;
  - the number of traced steps equals the step count in
    `kiln-legacy-results`;
  - the `sh -c` line of every traced step equals, byte for byte, the
    matching step line of the BUILD variant from `Source` that the weak
    key's `recipe` covers. The wrapper passes the expected line index, and
    the parser compares each step's `argv` with it. A mismatch gives
    `trace-incomplete:recipe-mismatch`.

  The executor reads BUILD files from the writable disk, outside any
  trace. This check stops an earlier BUILD from rewriting a later
  package's steps.

**The sandbox helper.** `kiln sandbox-exec` prepares the sandbox in six
steps, then `execve`s the step:

1. **Namespaces.** It enters new user, mount, PID, **network, IPC and UTS**
   namespaces and a session of its own. It maps the caller's uid and gid to a
   **non-zero** id and writes `deny` to `setgroups`. The new network namespace
   has only a loopback, brought up: **network is isolated for every action in
   v1** (§4.7). §5.4 gives network use no record until stage 5's fetchers, so
   the isolation costs no soundness.
2. **An allowlist root.** The new root is an **empty tmpfs**, and only what
   is named is put into it. It is not a copy of `/` with some paths masked: a
   pathname Unix socket or FIFO belongs to the filesystem, not to the network
   namespace, and a read-only bind does not stop `connect()` or an open for
   writing. The runner's .NET diagnostic socket in `/tmp`, for one, lets a
   step make `Runner.Worker` load a library. What the step sees:
   - the `image` and `tool` roots of `host-roots.json` (`/usr`, `/etc`,
     `/lib*`, `/bin`, `/sbin`, the pinned toolchains), **read-only**, passed in
     the configuration as `ro_roots` (a symlink such as `/bin` is recreated);
   - the repository, which is writable and stays at **the same absolute
     path**;
   - the `state` roots (§5.3), writable;
   - a fresh, empty tmpfs for `$HOME`, `TMPDIR`, `/tmp`, `/var/tmp` and `/run`
     (the host's service sockets, `docker.sock` and D-Bus, live in `/run`);
     the missing parent directories of the checkout are created in the tmpfs,
     so a checkout under `$HOME` sits under the `home` tmpfs;
   - `/dev`, a minimal tmpfs (`null`, `zero`, `full`, `random`, `urandom`,
     `tty`, and the `fd`/`stdin`/`stdout`/`stderr` links);
   - `/proc` (step 3).

   Everything else is **absent**, not masked: `.git` aside, the trace
   directory, the raw-records and results directories, the ledger and Kiln's
   state, the runner's `_temp` and file-command directories (`$GITHUB_ENV`,
   `$GITHUB_PATH`) and the Kiln and `strace` binaries are not in the root.
   The recorder must not list a root that contains any of them. The only
   masks are the checkout's own direct children (`.git`), made empty. After
   the pivot the helper checks that `/run/docker.sock`, `/var/run/docker.sock`,
   `/run/user` and `/run/dbus` do not exist, and refuses to run (exit 125)
   otherwise: a `ro_roots` entry such as `/var` must not bring them back.3. **`/proc`.** It mounts a fresh `/proc` for the new PID namespace with
   `hidepid=2`.
4. **No privileges.** It sets `PR_SET_NO_NEW_PRIVS` and clears the
   effective, permitted, inheritable, ambient and bounding capability
   sets. It sets `RLIMIT_CORE` to 1 (soft and hard), so a crash never invokes the host's
   `core_pattern` handler.
5. **A seccomp filter.** It installs one that returns `EPERM` for:
   - every refused call (§4.4);
   - `clone` with any of `CLONE_UNTRACED`, `CLONE_NEWUSER`,
     `CLONE_NEWNS`, `CLONE_NEWPID`, `CLONE_NEWNET`, `CLONE_NEWCGROUP` or
     `CLONE_NEWIPC`;

   and `ENOSYS` for `clone3`, whose flags a filter cannot inspect, so
   libc falls back to `clone`, and for every system call number above the
   highest one the pinned strace knows (§4.7, "Calls newer than the pinned
   strace"). strace still prints these failed calls, and the parser
   tolerates exactly the filter's own answer to a refused call (§4.4,
   §4.5): `-1 EPERM`, or `-1 ENOSYS` for `clone3`.

   The filter's first instructions check two things: that
   `seccomp_data.arch` is `AUDIT_ARCH_X86_64`, and that the syscall number
   lacks `__X32_SYSCALL_BIT`. Any other architecture or ABI kills the
   process (`SECCOMP_RET_KILL_PROCESS`). Without this check, a 32-bit or
   x32 `clone` would match none of the x86-64 rules. A non-native ABI is
   therefore impossible, not merely detected. A change to the filter bumps
   `kiln:recorder`. Enforcement must not rest on refusing a
   trace after the fact: a `CLONE_UNTRACED` child is never attached by
   `strace -f`, so it would leave no lines at all.
6. **No inherited descriptors.** It closes every file descriptor ≥ 3. The
   step's standard output and error are the two descriptors the recorder names
   with `--stdout-fd` and `--stderr-fd` (§4.7), which arrive as descriptors 1
   and 2 and are the only ones that cross.

This filter is the sandbox's own; it is independent of strace's
`--seccomp-bpf` option.

Two consequences:

- **Trace boundary.** The traced tree has a fixed shape. The Kiln helper
  is the Go `kiln` binary, so it is multi-threaded. It (the root, with its
  `CLONE_THREAD` threads) spawns **exactly one** non-thread child, the init
  of the new PID namespace. That child does the setup, re-executes the same
  binary (`/proc/self/exe sandbox-init ...`) as needed, and then `execve`s
  `sh -c <line>`. **The first successful exec of a non-Kiln path is the
  step.** The parser discards every event of the root group, and every
  event of the init child before the step, except failed execs of other
  paths (§4.5); the caller first checks that the root's `/proc/<pid>/exe`
  is the Kiln binary. So the namespace setup (`unshare`, `mount`,
  `pivot_root`, the seccomp installation) never reaches the refusal list.
  The step's exit status is the init child's, and its working directory
  must be the one the executor names (§4.5). The stage 3 recipe comparison
  uses the step's exec event, not the first exec of the init child.
- **Daemons.** The init child is PID 1 of the new PID namespace, so when
  its thread group exits the kernel kills every survivor. A process that
  is not proven to be outside the init child's tree (an exit is excluded only
  when its parent chain provably ends at a parentless process without passing
  the init child; a missing entry, a cycle or any depth is undecidable and
  counts), is not provably in the init child's thread group, and has ANY
  exit line (killed by any signal, or exited: a daemon may quit on EOF
  before the namespace's SIGKILL arrives) **after the init child's
  `exit_group` call line** gives `daemon-left` (D19). A child the step
  waited for has its exit line printed when strace reaps it, before the
  parent's `wait4` returns, so it precedes the marker. The boundary is the
  call line, not the init child's `+++ exited +++` line: strace prints
  `exit_group` when the group starts to die. **Observed (F2, the first
  `kiln-sandbox` CI run on ubuntu-24.04, kernel 7.0):** the survivor's
  `+++ killed by SIGKILL +++` line comes just after the init child's
  `exit_group` start line and BEFORE the init child's own `+++ exited +++`
  line (log: survivor pid 5150 exit line 369 is before init's exit line;
  init `exit_group` at line 368), which is the order `zap_pid_ns_processes`
  gives. The rule stays order-agnostic: it keys on the call line, so it holds
  whichever side of the init child's exit line a survivor's line falls. With no `exit_group` seen
  (`InitExitLine == 0`: the init group made no exit call at all and died by a
  signal, so the step is already `failed` and incomplete) every such
  `SIGKILL` death counts, which fails closed (other exits cannot be placed
  relative to a marker that is not there). A failed `exit_group` (a result
  other than `?`, for instance EPERM from a seccomp filter the step installed)
  does not set the marker. A child the step SIGKILLed and
  did not wait for just before exiting may be flagged too; that race is
  accepted. No timer is needed.

**Integrity.**
- **The claim.** The tracer runs outside the namespaces, unprivileged: no
  `sudo`, no `--privileged`, no added `SYS_PTRACE`.
- **Why it holds.** Inside the namespaces the action has no capabilities
  and cannot unmount anything. Every path it could use to reach the
  recorder is absent or read-only, and the tracer's process is invisible
  to it. So it can neither write to nor forge its own trace, the results
  file, or the job's later steps.
- **Parse after exit.** Trace parsing runs in a **later step** of the
  shard job, after every sandbox has exited. PID-namespace teardown
  guarantees nothing from the BUILD survives into that step.
- **Its limit.** An unprivileged user namespace cannot provide a second
  uid on the host, so KILN00 §9.6's "dedicated user" is not met.
  Integrity rests on the mount and PID namespaces, the dropped
  capabilities and the seccomp filter. Yama's `ptrace_scope` is not
  relied on.
- **Runners.** Hosted Ubuntu runners restrict unprivileged user
  namespaces through AppArmor. A setup step lifts this with `sudo sysctl -w
  kernel.apparmor_restrict_unprivileged_userns=0` before any BUILD runs, as
  `ci.yml` already does for the Forme sandbox.
- **No namespaces, no recording.**

**Secrets and hygiene.**
- **Job.** `kiln-record` jobs run with `permissions: contents: read`,
  check out with `persist-credentials: false`, and get no repository or
  organisation secrets. `.git` is absent inside the sandbox, so no
  persisted token is reachable.
- **Policy names.** `env.json` may not allow a secret-like name (KILN03
  §5).
- **Output.** Kiln's outputs (`explain`, plan JSON, the run report) print
  env **names and value digests**, never values. Reason strings have
  `$HOME` and `TMPDIR` prefixes redacted.
- **Trace files.** They hold command lines and paths. They are never
  uploaded as artifacts and are deleted after processing; only derived
  records leave the runner.

### 4.4 The parser's closed table

The `-e trace=` filter decides what strace prints. The parser then sorts
every printed system call into exactly one of three classes. A name in
none of them, or a line it cannot parse, makes the trace incomplete. The
strace version is pinned exactly, so a call newly added to `%file` shows
up as an unknown name rather than being silently skipped.

**Handled** calls are observations or writes:

| Kind | System calls | Meaning |
|---|---|---|
| Open | `open`, `openat`, `openat2`, `creat` | a read if the access mode includes reading; a write if it includes writing or `O_CREAT`/`O_TRUNC` |
| Metadata | `stat`, `lstat`, `newfstatat`, `statx`, `access`, `faccessat`, `faccessat2`, `readlink`, `readlinkat`, `getxattr`, `lgetxattr`, `listxattr`, `llistxattr`, `statfs`, `inotify_add_watch`, `fanotify_mark` | an observation of the path |
| Listing | `getdents`, `getdents64` | a listing of the directory the descriptor names |
| Directory | `chdir`, `fchdir` | changes the process's working directory (§5.1) |
| Write | `mkdir`, `mkdirat`, `rmdir`, `unlink`, `unlinkat`, `rename`, `renameat`, `renameat2`, `link`, `linkat`, `symlink`, `symlinkat`, `chmod`, `fchmodat`, `fchmodat2`, `chown`, `lchown`, `fchownat`, `truncate`, `utime`, `utimes`, `utimensat`, `futimesat`, `setxattr`, `lsetxattr`, `removexattr`, `lremovexattr`, `mknod`, `mknodat`, and through a descriptor `fchmod`, `fchown`, `fsetxattr`, `fremovexattr`, and `utimensat(fd, NULL, ...)` (the object is the `-y` path; glibc's `futimens` is that call, since x86-64 has no `futimens` system call) | a write to the path. The **source** of `rename`, `link` and `linkat` is an observation as well as a write. A **successful** call on a path the action did not create is an observation as well: it only worked because something was there. |
| Process | `execve`, `execveat`, `fork`, `vfork`, `clone`, `clone3`, exit lines | the process tree; an exec is a read of the program (§5.1) |
| Network | `socket`, `connect`, `bind`, `listen`, `sendto`, `sendmsg`, `sendmmsg` | §5.4 |

**Ignored** calls carry no path: `wait4`, `waitid`, `exit`, `exit_group`,
`kill`, `tgkill`, `getsockname`, `getpeername`, `recvfrom`, `recvmsg`,
`accept`, `accept4`, `shutdown`, `setsockopt`, `getsockopt`, `socketpair`,
`getcwd`, `pipe`, `pipe2`, `close`, `dup*`, and the rest of `%process`
and `%network`, enumerated in the implementation.

**Refused** calls make the trace incomplete. They read files without a
path the tracer sees, or change what paths mean: `io_uring_setup`,
`open_by_handle_at`, `name_to_handle_at`, `mount`, `umount2`, `fsopen`,
`fsmount`, `fsconfig`, `fspick`, `open_tree`, `move_mount`,
`mount_setattr`, `pivot_root`, `chroot`, `unshare`, `setns`, `ptrace`,
`process_vm_readv`, `pidfd_getfd`, `bpf`, `io_uring_enter`, `io_uring_register`,
`process_vm_writev`, `userfaultfd`, `kexec_load`, `kexec_file_load`,
`perf_event_open`, `add_key`, `request_key`, `keyctl`. Also refused: a `clone` whose
flags include `CLONE_UNTRACED` or any `CLONE_NEW*`, and any `clone3`. The
sandbox's filter already blocks these (§4.3), and strace prints the blocked
call with the filter's answer. A refused-class call is **tolerated** (dropped,
no event) exactly when its result is that answer: `-1 EPERM` (for `clone3`,
`-1 ENOSYS`; likewise a `clone` with only forbidden flags that returned
`-1 EPERM`). A refused call with any other result (a success, another errno,
`= ?`) means the filter failed and is refused; one that never resumed is an
`unresumed-call`. The `<unfinished ...>` half of a refused name is not judged;
its resumed half is.

**Failures.** A path call that fails is an observation of that path
(`ENOENT`: absent; `EEXIST`: present), never a write. A personality-change
line (a non-x86-64 system-call ABI) makes the trace incomplete.

Calls on an already-open descriptor (`read`, `mmap`, `fstat`, `sendfile`,
`copy_file_range`) need no tracing, because the `open` that produced the
descriptor was already observed.

### 4.5 Parser decisions (stage 1a)

Stage 1a (`internal/trace`) is pure: it reads strace text and returns
ordered events plus the process tree. It never resolves a path. These are
the decisions it settled where §4.3 to §5.1 left room. The code is
authoritative for details: `table.go` for the table, `errors.go` for the
reason strings.

**Reasons.** Every refusal is `trace-incomplete:<detail>`; a detail never
contains path text.
- A refused name whose result is not the filter's own answer (§4.4) is
  `refused:<name>`, an unknown one `unknown-syscall:<name>`.
- `clone` with `CLONE_UNTRACED`, any `CLONE_NEW*`, a missing `flags=` or
  a flag token it cannot name is `clone-flags`, unless the call failed with the
  filter's `-1 EPERM` and the flags are all nameable. A `clone3` that did not
  fail with `-1 ENOSYS` is `refused:clone3`.
- A process-tree fault is `pid-reuse` (a pid returned twice, or by its
  own process, or the root's pid), `orphan-pid` (a pid nobody spawned),
  `process-cycle` (or a parent chain over 4096 deep), `missing-exit`,
  `duplicate-exit` or `event-after-exit`.
- Other reasons: `unresumed-call`, `unmatched-resume`, `interleaved-call`,
  `personality`, `unparsable-line`, `ambiguous-path`, `truncated`,
  `non-utf8-path`, `deleted-path`, `arity`, `sockaddr`, `result`, `size`.

**The `-y` annotation.** The parser assumes strace 6.x escapes the
annotation body like a string and also escapes `<` and `>` as `\<` and
`\>`, so `3</a<b>` is printed `3</a\<b\>>` and round-trips. A raw `<`
inside an annotation, or text left after its closing `>`, is
`ambiguous-path`. The return-value annotation must end the line. The stage 3
real-strace fixture must pin this assumption against the exact strace
version; until then the canned fixtures are hand-written.

**Order and split calls.** Events are ordered by when the call
**completes**, so a split call sits at its `resumed` line. Its working
directory is the one current at its `<unfinished ...>` line. An unfinished
call may end at the process's exit line only if it carries no path
(`wait4`, `exit_group`); an unfinished path call that never resumes is
`unresumed-call`, with two exceptions (the phantom `execve`, D33, and a task
that died inside a call because its thread group was exiting, D35, both below). Another shape is accepted: a thread killed at syscall
entry. When a fatal signal is already pending (a sibling's `exit_group`, a
SIGKILL, an exec by another thread), strace cannot name the call and prints
`???`, in exactly one of three spellings, each followed by that thread's exit
line: `P  ???( <unfinished ...>` (S1); the same followed by
`P  <... ??? resumed>) = ?` (S2); or the single line `P  ???()` padded to
`= ?` (S3). The kernel's `ptrace_report_syscall` returns
`fatal_signal_pending`, so `syscall_trace_enter` returns before seccomp and
dispatch: the call never ran and the thread makes no further syscall, so the
line is a death marker, not an observation, and produces no record. The
spelling depends on a stale per-thread "filtered" flag from the previous
syscall, which is why all three occur. Every other `???` shape stays
`unparsable-line` (a result other than `= ?`, arguments, near-miss spellings),
a call, second `???(` or signal line from the same pid before its exit is
`interleaved-call`, a resume of another name or a second `??? resumed` is
`unmatched-resume`, and one never followed by the exit is `unresumed-call`.
This covers the superseded-leader exec case too, which is still refused. A
handled call whose result is `?` (including `ERESTARTSYS`) is `result`:
restarted calls are not recognised in v1, except the one shape of D50 rule 2b
(a `fork`, `vfork` or `clone` that ended `= ? ERESTARTNOINTR`). A final line without its newline is `unparsable-line`, since strace may have
been cut off mid-line.

**A phantom `execve` (D33).** The same race has a fourth spelling. A thread T
that was at a syscall-ENTRY stop when another thread L's `execve` made
`de_thread` kill it can have its stale stop dispatched by strace anyway.
`PTRACE_GET_SYSCALL_INFO` then returns `op=NONE` with only its header written;
strace's `syscall.c` tests only for `op == EXIT`, so its static entry buffer
still holds L's `execve`, and it prints a byte-identical line under T's pid,
then T's exit:

```
4288  execve("/usr/bin/sh", ["sh", "-c", "..."], 0x24e1f3f4ab70 /* 5 vars */ <unfinished ...>
4289  execve("/usr/bin/sh", ["sh", "-c", "..."], 0x24e1f3f4ab70 /* 5 vars */ <unfinished ...>
4289  +++ exited with 0 +++
4288  <... execve resumed>) = 0
```

(Captured from `kiln record prepare`: the sandbox-init leader execs once, on
a locked thread, and 4289 was a Go runtime thread.) It cannot be avoided in
Kiln, since any multithreaded Go or JVM program has the race, so it is a
parser rule. The unfinished `execve` is accepted, and produces no record, only
when ALL of these hold; otherwise it stays `unresumed-call` at T's line:

1. T's pending call is `execve` (not `execveat`), and T's NEXT line is exactly
   `+++ exited with 0 +++`: no signal line or anything else between, not
   killed, code 0.
2. Another pid L has a pending `execve` with an EARLIER line and a
   byte-identical text before `<unfinished ...>` (the earliest, if several).
3. The path is absolute: the line starts `execve("/`.
4. L's `execve` later resumes with `= 0`. If the owner's `execve` resumes with
   a failure, the refusal is at the ghost's line; if the owner never resumes,
   or exits first, the refusal is the owner's own `unresumed-call` (at the
   owner's line).
5. In pass 2, T and L are in the same thread group (the spawns with
   `CLONE_THREAD` lead from one to the other's leader, within the depth
   bound). Otherwise the refusal is at the ghost's line.

Soundness: even if the line were real, L's byte-identical `execve` succeeded,
so T was torn down before its image changed; its only possible effect is
reading the same absolute path L's recorded exec reads, and it writes nothing.
T's exit record stands; the phantom yields no event. Both readers share one
helper (`exitInsideCall`), so they cannot drift. The thread-group walk of
rule 5 is memoised and bounded like the other group walks; past the depth bound
it can only refuse a real same-group pair, never accept two groups as one.

Known residual: a step that races two `execve`s of the same absolute path
against a swap of a symlink on that path could, in principle, have a killed
REAL sibling's open of the old target dropped as a phantom, so that open is not
recorded. That binary never runs (the sibling was torn down before its image
changed), and the owner's open of the path is recorded. We judge this
acceptable: it needs a deliberate race inside one step, and the reviewers
judged it theoretical.

**A task that dies inside a call (D35).** When a thread group exits (Go's
`exit_group`, a fatal signal), the kernel kills the group's other threads
wherever they are, and every threaded runtime makes some of them die inside a
system call. strace prints three shapes for that, and every real-build census
run hit them (Go actions were unparsed before this rule):

```
2  tgkill(1, 1, SIGURG <unfinished ...>
2  <... tgkill resumed>) = ? <unavailable>                          (1)

2  clone(child_stack=0x7f..., flags=CLONE_VM|... <unfinished ...>
2  <... clone resumed> <unfinished ...>) = ?                       (2)

2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY|O_CLOEXEC <unfinished ...>
2  +++ exited with 0 +++                                           (3)
```

In (1) strace could not fetch the result because the tracee was already gone.
In (2) it printed at `PTRACE_EVENT_EXIT` with the call still open, so the
`<unfinished ...>` marker follows the resume. In (3) the task was killed out of
its syscall-entry stop, so strace could no longer read its memory and the path
prints as an address. In all three the call's result is unknown.

*Syntax (the only part that differs between the readers; both accept
identically).* The result `= ? <unavailable>` is a token of its own
(`UNAVAILABLE`, `strace.tokens`, before `ERR_LT`) and the `tail` rule allows
it; a resumed line may carry the marker (`RESUMED inner UNFINISHED RPAREN
tail`). The grammar is looser than the rule, on purpose, as for every result:
the shared result parser accepts exactly `= ? <unavailable>` (one space, nothing
after it, no errno, no value), and exactly ` <unfinished ...>)` followed by
`= ?` after a resume (no other argument text, no other result). The stricter-only
row for "an `ERR` token in the text after the close" is NOT widened: the text
`<unavailable>` simply is not an `ERR_LT` any more.

*Policy (shared code, so the readers cannot drift).* Shapes (1) and (2) leave
the pid with a *dying* call: its NEXT line must be its own exit
(`+++ exited ...` or `+++ killed ...`); a signal line, a second resume or a new
call is `interleaved-call` or `unmatched-resume`. Shape (3) is an unfinished
call followed by the pid's exit. At that exit:

| the call | verdict |
|---|---|
| ignored class (`tgkill`, `wait4`, ...) | dropped |
| `fork`, `vfork` | dropped, and **no spawn is recorded**. A child that did start has a first line with no spawn record: `orphan-pid`, which fails closed. |
| `clone` whose flags are readable and allowed: `flags=` is present, every token is a name strace prints (the closed set of `CLONE_*` and signal names), and none is `CLONE_UNTRACED` or `CLONE_NEW*` (the `analyzeClone` check) | dropped, no spawn recorded, as for `fork` |
| a `clone` with any other flags (`CLONE_UNTRACED`, `CLONE_NEW*`, a hex leftover, an unknown token, no `flags=`); `clone3` (a refused name; a dying `clone3` at entry stays `unresumed-call`) | `unresumed-call`, as before |
| everything else: a call that READS (a metadata call, a listing, an `open`, `openat`), a write, a network call, `chdir`, `fchdir`, `execve`, an unknown or refused name | `unresumed-call`, as before |

Both readers share one table (`dyingPolicy`) and one exit handler
(`exitInsideCall`), and pass 2 has no part in it.

*Why a call that reads is not dropped.* The model infers what a step read only
from the `open` and `stat` records: `read` is not traced. So dropping one removes
an input from the read set, and the call's result is lost, not absent. A live
sibling thread can use the descriptor or the buffer, a sibling's `fork` in the
window copies it, a sibling's `exec` keeps the descriptor (no `CLOEXEC`), and a
stat buffer lives in memory every thread shares. No guard short of "shares
nothing with any live task" is sound, and that excludes every thread. (An
earlier design dropped such a call when its thread group was ending; it was
withdrawn for exactly this reason.) A dropped spawn records nothing, and a child
it may have made cannot be traced without being refused. Writes, network calls
and exec are not dropped because their effect, or their target (which the lost
memory read hid), may have happened. `RecorderVersion` is 5 since this rule.

Consequence: a Go (or any threaded) action whose worker is inside `openat` or
`newfstatat` when the group exits is still `unresumed-call`. Only the other two
shapes, which Go also produces, are accepted now. Follow-up idea, not built:
record such a call as an OBSERVED READ with an unknown result (the path is lost,
so the observation would have to be "something under the working directory may
have been read"), which a later stage could judge.

**Real-strace refusal test oracle.** The Linux `TestSandboxStraceDyingThreads`
integration test may classify one additional observation as an expected
*refusal*, never as an accepted trace: the exact `openat` resumed line whose
result is bare `= ?`. It is a known dying-thread shape only when the full trace
later contains a parsed terminal exit for that same PID before any known spawn
record reuses the PID. The test helper must require the existing refusal reason,
the exact refusal line and PID, and that later terminal record; a different
syscall, trailing text, missing or earlier exit, another PID's exit, or an
ambiguous PID generation fails the test. This oracle does not change the D35
parser policy: the production readers still refuse the incomplete trace. Keep
the 20 completed probe iterations, empty stderr and reader agreement checks.
The predicate and its negative controls must be testable on every host OS even
though only Linux runs the real-strace driver.


**Process model.**
- A first pass collects every child pid a `fork`, `vfork` or `clone`
  returned, because the child's lines may precede the parent's return line.
- A child is created **when the parent's call succeeds**, which snapshots
  the parent's working directory then, or shares it under `CLONE_FS`. A
  later `chdir` by the parent never reaches the child. The only earlier
  creation is a child whose lines were printed while its parent was still
  blocked in the call.
- The working directory is an **unresolved expression**: after
  `chdir("../b")` from `/a` it is `/a/../b`. Only `fchdir` gives a resolved
  path (its `-y` annotation). Stage 1b walks the cwd expression like any
  other path.
- A successful `chdir` or `fchdir` on a cwd shared with another process
  (`CLONE_FS`, threads) is `cwd-race` only when the call's line interval
  intersects that of a sharer's cwd-dependent call or of another successful
  chdir/fchdir on the same cwd; otherwise the order is unambiguous and the
  change is applied at the chdir's completion line (D46).
- Pids with leading zeros are not pids.

**The helper.** With `Options.HelperRoot` and `Options.HelperExe` (the
resolved path of the Kiln binary, required), the tree is: helper root and
its threads, one init child and its threads, then the step (§4.3). **The
first successful exec of a non-Kiln path is the step.**
- **The root group.** None of its events reaches `Events`. If it execs
  successfully, the first exec must be of `HelperExe` (`helper-exe`), and a
  later one is `helper-activity`. Threads (`CLONE_THREAD`) of the root, or
  of its threads, belong to the group. The group makes exactly one
  non-thread spawn, the init child; a second is `helper-spawn`. Before it,
  its calls are setup and are discarded, and it may clone into new
  namespaces. After it, only ignored-class calls (`wait4`, `waitid`,
  `kill`, `exit*`) are allowed; anything else is `helper-activity`. A root
  that never spawns is `helper-no-init`.
- **The init child's window.** It lasts until the init child's first
  successful exec of a path other than `HelperExe`. Inside it, events are
  discarded and calls the table would refuse or not know (`unshare`,
  `mount`, `pivot_root`, `seccomp`, ...) are tolerated, because pass 1
  cannot yet tell whose call it is and judges these names in pass 2.
  Successful execs of `HelperExe` (the re-exec; `/proc/self/exe` counts as
  the same binary) are allowed. `CLONE_THREAD` threads belong to the init
  group; a non-thread spawn is `helper-spawn`. **Exception:** a failed
  `execve` or `execveat` of any other path is kept as an observation, since
  it is a real `ENOENT` probe. A thread made in the window that is still
  calling after the step's exec is `helper-activity`; threads the step
  itself makes later are part of the step. Without a step exec the trace is
  `helper-no-exec`.
- **Working directory.** At the step's exec the init child's tracked cwd
  must equal the `initialCwd` given to `Parse`, for every path form,
  absolute included; otherwise `helper-cwd`. A setup `chdir` is fine only
  if it ends back there.
- **Requirements on the stage 3 helper.** The parser models the shape
  above and cannot enforce how the helper reaches it, so the helper must:
  - exec the step from the init **thread-group leader** (the Go helper
    locks the main goroutine's thread with `runtime.LockOSThread` before the
    exec), because an exec by another thread changes the pid strace prints;
  - do its setup `chdir`s before any thread starts, or after
    `unshare(CLONE_FS)`: threads that share a cwd make a `chdir` that
    overlaps one of their cwd-dependent calls a `cwd-race` (D46);
  - make **no traced call in the root group after the spawn**. Go runtime
    thread creation is a `clone`, so the helper starts all the threads it
    will need before it spawns the init child;
  - live at a `HelperExe` path **outside the repository tree**. The parser
    only compares paths; the caller verifies this.
- **PID namespaces.** The init and everything under it run in a PID namespace
  of their own, so a pid returned by their `fork`, `vfork`, `clone` or `clone3`
  is namespace-local, while the trace's line prefixes, exit lines and the
  root's own returns are host pids. With `--pidns-translation` (§4.3) the
  returned pid is followed by `/* N in strace's PID NS */`, and the spawn is
  recorded under that **host** pid `N`, never under the local number. The note
  is parsed exactly; any other comment after a result is `result`. **A spawn
  whose caller is not in the root group (the init, its threads, the step and
  everything it spawns) must carry the note**, otherwise the trace is
  `pidns-untranslated`: without it the spawn cannot be matched to its child's
  lines, and trusting the local number could attribute them to the wrong
  process. The root group lives in strace's own namespace and prints none.
  Exit lines use host pids; `wait4`, `kill` and `tgkill` are ignored-class.
- **Flags.** `AT_EMPTY_PATH` is read from a numeric or hex flags value too
  (bit `0x1000`); a flags field that cannot be decoded is `flags`.
- **Exit status and daemons.** The step's status is the init child's
  (`Trace.StepExit`), and `Trace.InitPid` names it for the `daemon-left`
  rule. The parser also records `Trace.ExitLines` (the line of each exit,
  parallel to `Trace.Exits`) and `Trace.InitExitLine` (the marker, after the
  step's exec, over the init child and its `CLONE_THREAD` threads: the
  earliest `exit_group` that did not return, one candidate per thread, a
  failed call and a split call whose resumed half shows a result being
  withdrawn; else the start line of the LAST `exit` that did not return
  (a single-threaded init may end with a raw `exit`, and every pthread ends
  with one, so only the last counts); else 0); `traced.DaemonLeft` applies the
  rule (D19). `--- SIG ---` signal lines are skipped; `+++ killed by SIG
  +++` is an exit. A `--- SIG ---` line on a thread with a `???(` entry
  pending is still `interleaved` (a SIGKILL has no delivery stop, so a real
  trace should not show one); a test pins it so a relaxation is deliberate.
  **Known precision limit.** A harness that SIGKILLs a busy child (a test
  timeout, say) can leave a handled call unresumed, which is an unknown write
  set and a void run (`unresumed`). It cannot be relaxed safely: a call killed
  at entry and a call killed while blocked print identically. It is strictly
  better than before D19, when every kill was `missing-exit`.

**A failed action's traces are a prefix of the recipe (D36).** `CheckRecipe`
used to demand one step trace per recipe line. A multi-step BUILD that fails
before its last step leaves fewer traces than lines, and every such action was
`trace-incomplete:recipe-mismatch` (W10): an unknown write set, so a run-void
under the strict merge. In real-build census runs that was about 33 actions per
shard, about half of all unknown write sets. Fewer traces are now accepted when
ALL of these hold, and the verdict is then only `failed` (raised by
`stepStatus`), with `writes_complete` left to the traces:

1. every trace matches its recipe line, in order, exactly as before (a shell
   `-c` argv whose line is byte-equal): the traces are a prefix of the recipe;
2. every trace but the last has an exit line and exited 0;
3. the last trace has an exit line, was not killed by a signal, and exited
   non-zero.

Everything else stays `trace-incomplete:recipe-mismatch`: more traces than
lines; a prefix whose last step succeeded; a step that differs from its line; a
reordered or gapped recipe; an earlier step that failed or was killed; and an
EMPTY trace set beside a non-empty recipe (no step ran, so there is no failed
step to excuse the missing ones; `Classify` refuses an empty set earlier anyway,
and a step that never started is the executor's `failed to start step` case,
which `collect` handles from the step counts).

*Why it is safe.* (a) The build tool runs an action's steps in order and stops
at the first that fails: `runCommands` in
`build-tool/internal/executor/executor.go` (currently near line 474) loops over
the step commands and returns the `failed` result (currently near line 531) on
the first step error, and no later step of that action is started. So the writes of a failed action are exactly those of the traced steps;
nothing runs after the failure. (b) A failed action emits no manifest, so the
prefix rule never produces a cache entry. (c) A rewritten BUILD that drops
trailing steps ends on a step that SUCCEEDED, which condition 3 rejects; to pass,
a rewrite would have to end on a failing step, which is the honest failed case.
(d) A step stopped for a reason the traces do not show (it could not be started,
a descendant held the output pipe, a timeout or interrupt) either has exit 0 or
no clean non-zero exit, and stays a mismatch. The exit status is the one
`stepStatus` already reads (`Trace.StepExit`); no new source. `RecorderVersion`
5 to 6, because the verdict for these actions changes.

*What D36 does NOT change: the strict void.* Before this rule these actions were
`trace-incomplete:recipe-mismatch`, which the merge treats as a HARD unknown
(`unkHard`, recmerge `unknown()`): a run-void in BOTH the strict and the relaxed
verdicts. After it they carry `failed`, which is `unkTruncated`: the strict
verdict still voids the run (M-1: the build on main would have run the rest),
and the relaxed one only loses that action (`voidStrict`, `relaxLost`). The
change is hard-void to strict-only. It is correct that the strict void stays:
D36 makes the write set known, it does not make a failed action a built one.

**Addresses.** `socket` needs a symbolic `AF_*` domain, and `connect` and
`bind` need a `sockaddr`; otherwise `sockaddr`. Only `sendto` may have a
`NULL` address. `sendmsg` and `sendmmsg` give one event per distinct
`sockaddr` found outside string literals (none: one event with no address).
A bare `...` element in their arrays is `sockaddr`, since it means a
truncated list. A pathname `AF_UNIX` address is not copied into `Path`;
§5.4 decides what it means. With `AT_EMPTY_PATH` and an empty path, `Path`
is the descriptor's `-y` path.

**The ignored list.** §4.4 defers the enumeration to the implementation,
and `table.go` is authoritative: `wait4`, `waitid`, `waitpid`, `exit`,
`exit_group`, `kill`, `tgkill`, `tkill`, `arch_prctl`, `prctl`,
`pidfd_open`, `pidfd_send_signal`, `getsockname`, `getpeername`,
`recvfrom`, `recvmsg`, `recvmmsg`, `accept`, `accept4`, `shutdown`,
`setsockopt`, `getsockopt`, `socketpair`, `getcwd`, `pipe`, `pipe2`,
`close`, `close_range`, `dup`, `dup2`, `dup3`. Anything else absent from the
table is unknown.

**Caps.** The parser's caps bound **resident memory**, not the recorder's
2 GiB disk cap (§4.3): it keeps every non-ignored record until the second
pass, and each record keeps its source line. The defaults are 1 MiB per
line, 64 MiB in total and 1,000,000 records, and every record counts
(calls, start markers and exit lines). Argument counts are bounded while
scanning: a handled call's arity, or 16 for any other name. Exceeding a cap
is `size` or `arity`. Limits may be lowered freely, but raised only by a
spec revision with a DoS review and a worst-case benchmark (§4.9).

**D19: signals in the trace.** `print_signalled` in strace 6.8
(`strace.c`) prints `+++ killed by %s %s+++` only when
`is_number_in_set(WTERMSIG(status), signal_set)`. With `-e signal=none`, a
process killed by ANY signal (SIGPIPE in `yes | head`, a test harness killing
its child, Go's `CommandContext`) therefore has no exit line, the parser
reports `missing-exit`, and the run is voided. So the recorder runs
`-e signal=all`; the `--- SIG ---` delivery lines it adds are skipped by the
parser, and `RecorderVersion` is 3. A missing exit line is still
`missing-exit`, and `(core dumped)` is still incomplete.

**Yield note (D19).** Two accepted patterns can be flagged `daemon-left`
nondeterministically, depending on the order in which the kernel and strace
report exits near the init child's own: an init that exits without waiting for
a child that is exiting at the same moment (read the child's output to EOF,
then exit without `wait`), and a helper that exits on its parent's EOF (Python
multiprocessing's resource tracker). Since `daemon-left` is run-void under M-1,
the first recording runs must COUNT it per action before anyone relies on the
rule.

### 4.6 Wiring decisions (stage 3a)

Stage 3a added `-exec-wrapper` and `-emit-results` to the old Go build tool
(`code/programs/go/build-tool/`). These are the choices it made where §4.3
left room; the code is authoritative.

- **Argument syntax.** `-exec-wrapper` takes a JSON array of non-empty
  strings with no NUL. An empty, invalid or trailing-data value is a usage
  error. The step runs as `wrapper... sh -c <line>` (`wrapper... cmd /C
  <line>` on Windows, with Go's standard argument quoting).
- **Environment.** The wrapper receives `KILN_ACTION` (the package's
  repo-relative path, forward slashes), `KILN_STEP` (0-based) and
  `KILN_STEP_COUNT`. §4.3 named only the first two; the count lets the
  wrapper report a missing step. Caller values of these three names are
  replaced. They are for the wrapper: `kiln sandbox-exec` scrubs them before
  the step runs.
- **Mutual exclusion.** `-exec-wrapper` with `-clippy` or
  `-prune-leaf-targets` exits 1 before any work.
- **Results.** `status` is the executor's own status, unchanged: `built`,
  `failed`, `skipped`, `dep-skipped`, `unsupported`, `dep-unsupported`,
  `would-build`. `steps_run` counts steps started, so a failing step counts
  as run and `steps_run < steps_total` marks a mid-package failure; a package
  that never ran has `steps_run` 0 and its BUILD's step count as
  `steps_total`. Packages are keyed by repo-relative path and sorted. The file
  is written after the run, even when a package failed.
- **Freshness.** `-emit-results` removes any existing file at its path
  before doing any work, so early exits (no packages, hash failure, usage
  errors after the check) leave no file. Consumers must require exit 0 **and**
  a file written by this run.
- **The wrapper program.** `wrapper[0]` is resolved at startup: a relative
  path containing a separator is rejected, a bare name goes through
  `exec.LookPath` and must resolve to an absolute path, and the absolute path
  is what runs. A BUILD can therefore never supply the wrapper through the
  package working directory. A wrapper that cannot start reports `failed` with
  `steps_run` equal to the step's index (it never started).
- **The line the wrapper sees.** The executor rewrites some lines
  deterministically before running them (the Swift native build-system pin;
  on Windows the inline-env rewrite), so the wrapper's `sh -c` argument can
  differ from the BUILD text. The stage 3b recipe check must compare with the
  line after those rewrites; `executor.EffectiveStepLines(pkg)` returns exactly
  those lines. The rewrites depend only on the line and the build-tool source,
  which is in the weak key.
- **Output path.** A symlink or non-regular `-emit-results` path is refused
  before the build starts (and again by the atomic writer).
- **The expected step lines (`-emit-kiln-steps`).** Kiln cannot derive the
  lines a package runs: they depend on the OS, on the Swift and Windows
  rewrites, and, for a Starlark BUILD, on evaluation. `-emit-kiln-steps
  <path>` writes, per package `rel_path`, the lines
  `executor.EffectiveStepLines` returns for this OS (schema
  `kiln-legacy-steps/1`, see §4.8). It is valid only with `-emit-kiln-graph`,
  is written in the same invocation from the same discovery pass, and so
  exists before any BUILD runs.

### 4.8 Recording decisions (stage 4a)

Implemented by `internal/record` (`VerifyTree`, `Inventory`) and
`internal/legacy` (the `kiln-legacy-steps/1` decoder, `RecordTmpdir`), plus
the build tool's `-emit-kiln-steps`. The rest of the shard recorder (host
scan, sandbox configuration, supervisor, `collect`, the `kiln record`
commands) is a later PR, because it needs the sandbox. The design review
found these gaps in the text above; each decision below replaces it. G7 to
G9 (the tracer completeness rule, trace storage, the strace install) are
pending that PR and are not decided here.

Stage 4 PR 2b is split in five. **2b-1** is `kiln record prepare`, everything
that runs before any BUILD (the section "Stage 4b-1" at the end of this
section). **2b-2a** is the supervisor, `kiln record step` (decision P12).
**2b-2b-i** makes `step` classify (decision P13): the sealed classifier
inputs in `pre/`, the serial lock and `seq`, and `record.json` written at the
end of each action. **2b-2b-ii** is `kiln record collect`: assembly, the
post-build `VerifyTree`, `git fsck` and `Inventory`, the header counts, the
`seq` contiguity check, the re-hash and final-newline checks, the shard writer
and the full end-to-end test. **2b-2c** is the gates and toolchain probes, the
in-sandbox structure probe (decision P14, done in 2b-2c-2) and the `daemon-left`
fixture. Each item of the two
"Pending for PR 2b" lists below is marked **2b-1** (decided and implemented by
`prepare`), **2b-2a** (decided and implemented by `step`), **2b-2b-i**
(decided by P13, implemented by `step` and `prepare`), **2b-2b-ii** (decided
and implemented by `collect`; all done, see "As implemented by `collect`"
after the obligations list of P12) or **2b-2c** (still pending, owned by the
gates and probes). A bare **2b-2b** in older text means 2b-2b-i and 2b-2b-ii.

**G5. The tree must equal `HEAD` before any digest is taken.** `Source` reads
the working tree, and nothing proved the tree equals the commit; CI's default
checkout is also sparse, which §5.2 forbids. `record.VerifyTree(root,
expectedCommit)` runs in the shard before any BUILD, after the build (the §5.2
backstop) and in the merge job before any digest. It returns the commit id it
verified. The first call passes `""`; every later call in the same run passes
that id, so a BUILD that ran `git commit -a` (a tree that equals its new HEAD)
fails the backstop. It requires, in order:

0. `root` is the top level of its repository (`git rev-parse --show-prefix`
   is empty); `HEAD^{commit}` is resolved once to a full id, with replace refs
   disabled (`--no-replace-objects`, `GIT_NO_REPLACE_OBJECTS=1`) and lazy fetch
   disabled (`GIT_NO_LAZY_FETCH=1`) in every Git call Kiln makes; and when
   `expectedCommit` is given, HEAD must still be it;
1. `git ls-tree -r -z --full-tree <that id>` (object ids from the object
   store, not the index) equals `git ls-files --stage -z` path for path, mode
   for mode and id for id, with stage 0 only. A tree path with an empty, `.`
   or `..` component, a `.git` component (any case), a leading `/`, a
   backslash or a NUL is refused;
2. `git ls-files -v` shows only the plain tag `H` (no skip-worktree, no
   assume-unchanged, no unmerged entry) and `core.sparseCheckout` is not
   `true`;
3. every entry is `lstat`ed, every ancestor directory is a real directory (not
   a symlink), and then: mode `120000` must be a symlink whose target hashes to
   the blob id; `100644` and `100755` must be regular files, opened
   `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY` (so a FIFO cannot hang the check), hashed
   as a Git blob (`"blob <n>\0"` plus the bytes; SHA-1 or SHA-256 by the id's
   length) with the executable bit equal to the mode; anything else is
   `non-regular`. A gitlink (`160000`) is an error, `submodule-unsupported`,
   until submodules are designed: nothing inside one is covered by this check
   (the repository has none today);
4. a file over 1 GiB, a check over 20 minutes, or a file whose size changed
   while it was read is an error; 8 workers hash in parallel;
5. an error lists at most 16 paths and says how many more there were.

Smudge filters, `ident`, `autocrlf` and LFS pointers are caught by their
effect on the bytes. On Windows there are no executable bits and no
`O_NOFOLLOW`; the check compares bytes and symlinks only, and the recorder
itself is Linux-only (stage 4 refuses other platforms).

**Inventory.** `record.Inventory(root)` lists everything on the disk the
tracked tree does not hold, as `kiln-record-inventory/1`: sorted, repo-relative
and forward-slashed. A directory holding no tracked file, however deep, is one
entry (the granularity of a write root), an empty one included. A symlink is an
entry and is never followed. The root `.git` (a directory, or a linked
worktree's `.git` file) is skipped. It uses its own `lstat` walk, not Git, so
ignored files are listed too (Git supplies only the tracked paths). It fails,
rather than skipping or listing, when: the root is not the repository's top
level; the tree has a gitlink (`submodule-unsupported`); a tracked path is a
directory on disk; a name the record formats refuse (not UTF-8, trailing
slash, too long); or the list passes the format's maximum (the walk stops
there instead of collecting everything first). Nothing is recorded from a tree
whose inventory cannot be written.

**G6. Kiln cannot compute the expected step lines.** See the
`-emit-kiln-steps` bullet of §4.6. The file is
`{"schema":"kiln-legacy-steps/1","os":"<goos>","packages":[{"package":
"<rel_path>","steps":["<line>",...]}]}`, sorted by package, every list
non-null, written atomically like the graph. Only packages that have a BUILD
file for that OS are listed (a package whose only file is `BUILD_windows` has
no recipe on Linux). The build tool refuses a line that is empty, not valid
UTF-8 or contains U+FFFD (the JSON encoder would map different bad bytes to the
same text), a package directory outside the root, a steps path equal to the
graph path, and a symlink or non-regular output path. Kiln's decoder
(`legacy.DecodeSteps`) refuses unknown, duplicate and case-variant keys,
trailing data, null anywhere, invalid UTF-8, U+FFFD, empty lines, NUL, an `os`
other than the three known ones, a repeated or unsorted package, a path
`key.ValidatePath` refuses, and a document over 16 MiB or over 2^20 values;
`ReadStepsFile` opens with `O_NOFOLLOW|O_NONBLOCK` and refuses a non-regular
file. The shard uses the lines as `traced.Input.Recipe`; the merge regenerates
them from its own checkout and compares `steps_sha256` (rule B6).

**G13. `TMPDIR`.** The sandbox requires a `tmpdir` and hosted runners do not
set `TMPDIR`. The recorder sets `TMPDIR=/tmp/kiln-tmp`
(`legacy.RecordTmpdir`) in the environment it derives keys from. The value is
keyed, so stage 5b lookups must set the same one.

**Pending for PR 2b** (the review of 4a found these; they belong to the code
that calls `VerifyTree` and `Inventory`, which does not exist yet):

- **2b-2b-ii. Done.** `collect` treats any plain git error from `VerifyTree` (not only a
  `*TreeError`) as a void of the shard, like `tracked-changed`, and passes the
  commit id `prepare` recorded (`collect_post.go`, `postBuild`; it also counts the
  error as one unexplained tracked change, see "As implemented by `collect`").
- **2b-2b-ii. Done.** An `Inventory` error after the build voids the shard; it must not crash
  `collect` before the records are written (likewise, and it counts as one
  unexplained path).
- **2b-1.** `prepare` fails if the pre-build inventory contains any path whose
  base name starts with `BUILD`: an untracked BUILD variant would change the
  lines a package runs. `prepare` also walks the whole disk for a `BUILD*`
  file the tree does not track, because the inventory collapses an untracked
  directory to one entry and a new package's BUILD hides inside it
  (decision P5).
- **2b-1.** Global Git configuration (`$HOME/.gitconfig`, `$XDG_CONFIG_HOME`) is still
  read by Kiln's Git calls, so the recorder's `HOME`, `XDG_CONFIG_HOME` and
  `PATH` must not be writable from the sandbox.
- **2b-1** (the mask; the `git fsck` is **2b-2b-ii, done**: `record.Fsck`, `git fsck
  --connectivity-only --no-dangling --no-progress`). The commit pin protects object ids, not the integrity of the object store. A
  BUILD that could write `.git` could overwrite loose objects or
  `.git/config`, so `.git` must be unwritable from the sandbox (the
  `R/.git` mask is a requirement, not a convenience). Consider a post-build
  `git fsck --connectivity-only` as a second line.
- **2b-1** (kept) and **2b-2b-ii** (used by `collect`, done: `collect` passes
  `binding.revision` and requires the commit `VerifyTree` returns to equal it). The caller keeps `Result.Commit` from the
  pre-build `VerifyTree` and passes it as `expectedCommit` to the post-build
  call and to the merge job's call. `prepare` writes it as the binding's
  `revision` (decision P10).
- **Sandbox follow-ups (S1 to S3) the recorder must honour** (§4.7):
  - **2b-2a.** the recorder passes `--stdout-fd` and `--stderr-fd` **together**,
    both pipes it reads, and checks that strace's own standard error is empty
    (the sidecar records it; `step` applies the parse half of G7 at the action's
    end, P13, and `collect` the rest);
  - **2b-2a** (never retried) and **2b-2b-i** (told apart, by `step`). exit status 125 is a
    failure of the action and is **never retried unsandboxed**: `step` records
    it (`exit_125`) and mirrors it. `step`'s epilogue tells "the sandbox refused" from
    "the step exited 125" by whether the step's `execve` is in the trace;
  - **2b-1.** the anchor of a tool root beneath `home` is sealed read-only as a
    whole, so PR 2b's configuration generation must declare as `state_roots` everything
    the toolchain writes beneath the same first directory: `~/.cargo/registry`,
    `~/.cargo/git` and whatever else cargo needs (`config.toml`, `env` and
    `.package-cache` are otherwise absent or read-only), and it must not anchor
    something broad such as `~/.local` (a tool root `~/.local/bin` would seal
    all of `~/.local`);
  - **2b-1.** `home` must be outside `/tmp`, `/var/tmp`, `tmpdir`, the checkout
    and every state root whenever any `ro_roots` entry lies beneath it, and a
    tool root beneath `home` must be a real directory or file (not a symlink).

**Pending for PR 2b** (the review of `writes_complete`, stage 4 PR 4):

- **2b-1.** The pre-build inventory voids the shard if any in-repository regular file has
  `nlink > 1`: a pre-run hard-link alias is the one write path no trace names
  (§5.5, Aliases).
- **2b-2b-i.** A step whose trace does not parse is `unknown_writes` and never complete; the
  recorder must not substitute an empty record. (`step`'s epilogue leaves
  `record.failed`, or a record with the trace's reason, never an empty record;
  `collect` turns a missing or failed `record.json` into `unknown_writes`.)
- **2b-2b-ii. Done** (`collect.go` `encodeShard`, `collect_actions.go` `assembleAction`). The shard writer derives `writes_complete` with `recfmt.CompleteOf` from the
  raw record, and calls `recfmt.CheckComplete` (`EncodeRecord` does). The key
  stays required on the wire for every record. A record that ran nothing
  (`steps_run == 0` and not built: refused or gated) was never classified, so
  the writer sets an explicit `false` for it, which `CheckComplete` accepts
  without a raw record; the merge charges such a record as a truncated
  execution (M-1: strict void, relaxed charges the action). Any record that ran
  a step, failed or built, was classified: its key must agree with the
  classifier, and a `false` or missing one is an unknown write set (§7.1, G1).
- **2b-2a.** A recorder test pins that `StepNetNS` is true only when the step really ran
  under `kiln sandbox-exec`, and false on every other path.
- **2b-1** (confirmed for `prepare`) and **2b-2a** (confirmed for `step`: it starts only the
  pinned `strace`, which starts the pinned Kiln; gates and probes are
  **2b-2c**, where this is to be confirmed again). Confirm that no gate or probe (preconditions, toolchain probes, `kiln`'s own
  checks) runs repository code outside a traced step: untraced repository code
  can write the tree with no trace at all. `prepare` runs the gates (decision P7, **2b-2c-3**), which read the tracked tree
  and run only the toolchain probes, and those only inside the sandbox
  (`kiln sandbox-exec`, no strace, on an empty checkout: P7); the host never runs a
  probe. Since **2b-2c-2** it also runs the in-sandbox structure probe (decision
  P14), as `strace ... kiln sandbox-exec -- <fixed argv>`, every program of it
  inside the sandbox and traced; the checkout's code is never run. The programs
  it starts are Git, the build tool, `strace --version`, the four structure probe
  runs and one `kiln sandbox-exec` per toolchain probe, all resolved outside the
  checkout (decision P3).
- **Done** (#19324). `recfmt.ShardRecord.Validate` refuses a record with `steps_run == 0` whose raw
  record has any content: a record that ran nothing was never classified.
- **2b-2a** (the sidecar's `step_netns`) and **2b-2b-i** (`step`'s epilogue copies it into
  `traced.Input.StepNetNS`). The recorder is the only setter of `traced.Input.StepNetNS`, and sets it to
  true only when the step ran under `kiln sandbox-exec` (which unshares `NET`);
  anywhere else it stays false, and an unbound loopback or abstract peer makes
  the write set unknown (W11).
- **Decided: P13** (**2b-2b-i**). Classify each action before a later action can
  change the tree: M8's cross-action `unstable-path` does not feed G1, so an
  action classified after another one rewrote links it traversed would see a
  different disk.
- **2b-2c.** The `daemon-left` ordering (a survivor's SIGKILL exit line after the init
  child's) needs a real-`strace` fixture on Linux CI; the canned traces only
  assumed it. **Done (PR 1, D19, read off the CI log in 2b-2c-2):** the rule
  keys on the init child's `exit_group` line, and `TestSandboxStraceSignals` runs
  the signal shapes against real strace. The survivor's exit line comes BEFORE
  the init child's own exit line (F2, §4.3); the rule does not depend on it.
  The `-e signal=none` wrapper really does override `signal=all` (the
  `signals` probe fails on it), so the probe is a true self-check.

**Stage 4b-1: `kiln record prepare`.** Implemented by `internal/record`
(`Prepare` and the files `place.go`, `scan.go`, `hostscan.go`, `sandboxcfg.go`,
`plan.go`, `prepare.go`) and `recordcmd.go`. It runs once per shard, before any
BUILD; any failure fails the job and records nothing. The decisions:

**P1. The command surface, and the argv contract of the wrapper.** One
command group, `kiln record`, three subcommands:

```
kiln record prepare --root <abs> --dir <abs> --build-tool <abs>
                    --shard-index I --shard-count N --job-start <unix seconds>
                    [--strace <abs>] [--git <abs>]
kiln record step    --dir <abs> -- sh -c <line>                          (2b-2a)
kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs> (2b-2b-ii)
```

Every path is absolute. `step` is implemented in 2b-2a. `collect` exists
until 2b-2b-ii only as a stub that printed "not implemented (KILN04 stage 4 PR
2b-2b-ii)" and exited 1, so the CLI surface was fixed and a workflow written
against it failed loudly, never silently skipped tracing. It is implemented
now (`internal/record/collect.go`); `--out` is the path of `records.jsonl` (the
other three files of a shard artifact are already in `pre/`). `prepare` writes the build tool's `-exec-wrapper`
value to `pre/exec-wrapper.json`: the JSON array
`["<kiln>","record","step","--dir","<dir>","--"]`, `<kiln>` being the
canonical path of the running binary. The build tool appends `sh -c <line>` and
sets `KILN_ACTION` (the package `rel_path`), `KILN_STEP` and
`KILN_STEP_COUNT` in the wrapper's environment (§4.6); `step` requires argv
after `--` to be exactly `sh -c <line>` and the working directory to be
`<root>/<action>` (2b-2a). The traced build is

```
build-tool -root <root> -plan-file <dir>/pre/build-plan.json -shard-index I \
  -jobs 1 -force -language all -exec-wrapper "$(cat <dir>/pre/exec-wrapper.json)" \
  -emit-results <dir>/results.json
```

Exit codes: 0 ok; 1 failure; 2 usage; 3 untrusted run or unsupported
platform (nothing is written).

**P2. Trust and platform come before anything else.** `prepare` refuses, with
exit 3 and nothing written, unless the process is `linux/amd64`, `host.Detect`
returns a `*host.GitHub` whose `Trusted()` is true, and the image id does not
contain `unknown`. Every path must be absolute. `GITHUB_SHA` must equal the
commit `VerifyTree` vouches for. `GITHUB_REPOSITORY`, `GITHUB_RUN_ID` and
`GITHUB_RUN_ATTEMPT` must be present and well formed (they become the
binding). The package builds, vets and tests on every platform; only the CLI
entry checks the platform, so the logic is unit-tested on developers' machines.

**Census side file (`census.json`).** The classify child (§4.8, P13) also
writes `traces/<sha256(action)>/census.json`, schema `kiln-record-census/1`, a
diagnostic that exists so a later host-path census can count every host path
that would void a recording BEFORE the 16-reasons-per-kind cap (`capReasons`)
hides all but a few, and so it can see the path behind
`trace-incomplete:proc-path`, which carries none. `traced.Input.Census` is an
optional observer that sees every path-carrying reason (`host-read`,
`writes-tracked`, `trace-visible`, `state-write`, `network`, `daemon`, and each
`proc-path`) as the classifier raises it. It only counts: the record is
byte-identical with and without it (`TestCensusDoesNotChangeTheRecord`).
Two further observations are census-only and never change a reason string:
(1) **sub-codes**: `trace-incomplete:unstable-path` and
`trace-incomplete:exec-unreadable` carry no path in the record, so at each raise
site the classifier tells the census a fixed sub-code and the redacted path,
under the kind `trace-incomplete:<reason>:<sub>` (unstable-path:
`component-changed`, `absent-changed`, `final-changed`,
`followed-final-changed`, `followed-absent-changed`, `gone-but-found`,
`removed-before-made`, `file-changed`, `own-overlap`, `through-missing`,
`dotdot`, `fd-path-differs`, `absent-before-made`; exec-unreadable: `modified-after-exec`,
`virtual-exe`, `virtual-interpreter`, `under-tmp`, `in-repo`, `host-disk`); the
sub-code is a string literal at each site, never a slice of the input.
(2) **op classes**: `census.json` has an additive `ops` map, `{kind: {class: n}}`
(split kinds: `host-read`, `trace-incomplete:proc-path` and every
`trace-incomplete:unstable-path:<sub>`), whose classes are the closed list `listing`,
`absent-probe`, `open`, `stat`, `lookup` (a link crossed on the way), `exec`,
`chdir`, `write`, `other`, chosen from the classifier's own state with
precedence listing, then absent-probe, then the call's kind. `proc-path` and the
unstable-path sub-codes raised while a call is observed take the call's class at
that moment; the sub-codes raised by the stability pass after the walk take the
class of the FIRST look that put the path into a lookup set (the classifier keeps
it per path, only when a census is attached, and reads it only to tell the
census); a path with none is `other`.
(3) **Caps and order**: the sub-coded kinds hold 256 paths each, the nine path
kinds 1024, so the caps of every kind the classifier can feed add up to at most
`MaxCensusEntries` (9*1024 + 18*256 = 13824 <= 16384; tested) and the overall
bound never decides which kind loses. The stability pass walks Go maps, so its
census notes are buffered and given sorted by kind and path: `census.json` is
deterministic even at a cap (tested). Records are untouched.
**Semantic change to `census_clean`**: a record that raised a READ-SIDE
`trace-incomplete:unstable-path` or `trace-incomplete:exec-unreadable` now has a
non-empty `census.json` (those reasons used to leave it empty), so such actions
count as `census_nonempty`, not `census_clean`. `census_clean` therefore means
"no path-carrying reason and no unstable-path or exec-unreadable", still not
"would emit" (see `emittable`). `ci/census_not_clean.py` reads only
`cross_check` and `bounds` and is unaffected. The file holds `{schema, action, total,
overflow_total, overflow: {kind: n}, entries: [{kind, path, count}]}` with
entries sorted by kind then path. Paths are printable and redacted
(`$HOME`, `$TMPDIR`) as in a reason, and sanitised once: a path the classifier
already made printable is not sanitised again. The bounds are PER KIND: at most
1024 distinct paths of each kind (and 16384 overall as a safety net), so a flood
of `host-read` cannot hide a late `proc-path`; a new pair beyond its kind's
bound adds to that kind's `overflow` (and to `total`) instead. The encoded file
is at most 512 KiB; to fit, entries are shed round-robin from the tail of every
kind that holds more than one, so every kind keeps entries, and what is shed is
counted in that kind's `overflow`. The file is **best-effort**: a failed write,
or a panic in it, changes neither the verdict nor the child's exit status. It is
written atomically (temp file, then `rename`, with NO fsync: a crash may leave it
empty or absent, which a diagnostic can bear). It is never read for any verdict:
it is not in a shard, not sealed, not hashed. One window remains: the file is
written after `record.json` is linked and before the child exits, so if the
parent's classify deadline (`ClassifyTimeout`) fires in that interval the
parent writes `record.failed` over a valid record. That fails closed and costs
only yield, and the missing fsync is what keeps the interval short. `collect`
and `step` accept a regular file named `census.json`, and a stray
`census.json.tmp-` plus 16 lowercase hex digits, in an action's directory; they
do not count either as an orphan or an unexpected entry (any other name, or a
non-regular `census.json`, is still one).

**`kiln record census` (census PR C).** The census side files are read by a
diagnostic subcommand, never during a recording:

    kiln record census --dir <abs> --out <abs> [--md <abs>] [--records <abs>]

It runs AFTER the build and after `collect`, on the record directory the
shard left, and writes a bounded, sanitised summary. The manual census
workflow (PR D) records a sample of the real build on each runner image, stores
nothing, and publishes only this summary.

*Per-language step wall and unknown-write-set codes (census PR C2).* The
summary carries two more breakdowns. `languages[].step_wall_s` is `{n, p50,
max, sum}` in seconds over that language's steps: the same sidecar `started_at` to
`ended_at` wall the blended `timing.step_wall_ms` already uses (nothing is
re-measured or re-parsed), filed under the action's language; p50 is nearest
rank, so an even count takes the lower middle sample, and steps with a
non-positive wall are in `timing.walls_dropped`, not here.
`languages[].step_wall_failed_s` is the same four numbers over the subset of
steps whose sidecar `root_status` is not zero: a stalled fetch is a long step
that then fails, and this separates it from a slow successful compile. It exists because a
stalled package fetch (cargo retrying the network for about 589 s per Rust step;
NuGet, bundler, pip/uv and cabal can do the same) looks like slow compilation in
a blended distribution. `collect.unknown_reasons` counts the shard's records
whose `unknown_writes` is non-empty, by a fixed code taken from the text before
the first colon: `serial-violated`, `interrupted` (`interrupted: a step started
and was never accounted for ...`), `not-run` (`not run: the build stopped before
this step`) and `other`. All four keys are always present; the text itself is
never copied. This is the one place the census decodes record lines, not only
counts them (a line over 1 MiB is counted, not decoded). A cheap byte search for
`"unknown_writes":"` only decides whether to decode; the top-level field of
the decoded line decides the code, so a nested key cannot fake a reason.

*Inputs, per action* (`traces/<sha256(action)>/`; the action's name comes from
its step sidecars and must hash to the directory's name): every step trace
`N.trace` and its sidecar `N.json` (`kiln-record-step/2`: `started_at`,
`ended_at`, `trace_bytes`, `root_status`); `census.json`; `record.json` or
`record.failed`; and, when `--records` names collect's `records.jsonl`, its
header line (`void`, the two unexplained counts) and the number of records.

*Every input is untrusted.* A build step chose the file names a census holds,
and a PR chose the build. Each file is opened without following a symlink and
with `O_NONBLOCK` (a FIFO cannot hang it), must be a regular file, and is read
under a size bound before it is decoded: `census.json` 512 KiB, a sidecar 1
MiB, `record.json` 32 MiB, a trace at most the classify limit (1 GiB; a longer
one is skipped and counted). Decoding is strict (`legacy.PolicyDecode`: no
unknown key, no duplicate or case-variant key, no trailing data), the schema
string and the embedded action are checked, kinds must match `[a-z0-9:_-]{1,96}`
and counts must not be negative. A file that fails any of that is skipped and
counted under a problem code; nothing panics and nothing is believed. A panic in
one trace's parse is recovered and counted (`parse_panic`). When both `record.json`
and `record.failed` exist, `record.failed` wins, as in `collect`. A `record.json`
whose `binding_id` or `sealed_sha256` is not the verified `pre/`'s is counted
(`record_binding_mismatch`) and ignored. The census takes the record directory's
lock (the flock `step` and `collect` use) exclusively and without waiting, and
refuses, exit 1, writing nothing, while a recording holds it; so it runs only
where flock exists (not on Windows).

*Per step trace.* (1) Bytes and lines are counted. (2) The wall time is
`ended_at - started_at` from the sidecar; a wall that is not positive is dropped
and counted (`walls_dropped`), never divided by. (3) ONE grammar-path parse is
timed through `parseStep`, the very call the classify child makes (same
`ClassifyLimits`, helper root, helper executable and package directory, all from
the verified `pre/`), single shot, GC on, as in production; `trace.Measure*`,
which toggle the GC, are not used. A `runtime.GC()` runs just before it,
outside the timing, so the previous trace's garbage is not charged to this one.
The report carries `parse_ms` and `parse_share` (parse over wall) of ACCEPTED
parses only. (4) ONE cross-checked parse (`trace.ParseCrossChecked`) counts
traces, accepted, mismatches and allowed stricter-only divergences. One trace is
in memory at a time.

*`ParseCrossChecked` is no longer a test instrument.* It never toggled the GC;
what kept it out of production was its process-global allowed-divergence
counter. It is now serialised by a mutex, so concurrent calls cannot charge one
trace another's hits, and `TestMeasureIsNotUsedByProductionCode` no longer lists
it. A second guard, `TestParseCrossCheckedOnlyInCensus`, is on the syntax tree
(`go/parser`): it flags ANY reference to `ParseCrossChecked` (a call, a method
value, an alias or dot import) outside its definition file and exactly one
production file, `internal/record/census_report.go`, and the census receives it
through `CensusOptions.CrossCheck`, not a package variable, because `kiln record census` is a
diagnostic that must never run inside a recording (the record child's
environment deliberately lacks `KILN_TRACE_DIFF`).

*Schema `kiln-census-summary/1`* (JSON, `encoding/json`, at most 512 KiB; the
optional Markdown rendering at most 256 KiB). Top-level keys: `schema`,
`infrastructure_failure` (text or ""), `problems` (code to count), `actions`,
`reasons`, `trace_sizes`, `timing`, `per_action`, `cross_check`, `languages`,
`collect` (null without `--records`), `bounds` and `shapes`.

- `actions`: `total`, `with_record`, `record_parse_ok`, `record_writes_complete`,
  `record_failed`, `no_verdict`, and the census states, each counted separately
  and NONE of them read as "no host reads": `census_missing` (no `census.json`:
  the classifier errored or crashed), `census_invalid`, `census_empty_parse_failed`
  (an empty census beside a `record.json` whose `parse_ok` is false: the parse
  half failed, the classifier never ran), `census_empty_no_record` and
  `census_clean` (an empty census beside a believed record: the classifier ran
  and raised nothing), `census_nonempty`.
- `reasons`: per kind, `occurrences`, `overflow` (what the classifier shed),
  `actions`, `distinct_paths`, `prefixes` (path prefixes of depth at most 4, with
  distinct paths and occurrences) and `top_paths` (the 200 most frequent full
  paths). Paths are sanitised (non-printable runes become `?`), redacted
  (`$HOME`, `$TMPDIR`, the record directory and the workspace become `$HOME`,
  `$TMPDIR`, `$RECORD_DIR`, `$WORKSPACE`; a prefix is replaced only at a
  path-component boundary, preceded by the start, whitespace, a quote, `=`, `:`,
  `,`, `(`, `[`, `<`, one of the shell metacharacters `;` `|` `&` `>` `{` `}`,
  an `@` that itself follows a boundary (a response file, `--opt=@/tmp/x`), or a
  one-letter `-X` flag, and followed by `/` or the end,
  so `/tmp` leaves `/usr/lib/tmpfiles.d`, `/var/tmp` and `/usr/include/c++/tmp`
  alone) and cut to 512 runes (`paths_truncated`). At most 100,000 distinct
  (kind, path) pairs and 32 MiB of their text are kept; the rest are counted in
  `bounds.paths_dropped`.
- `explain`: why actions are not recordable, every key from a CLOSED list in the
  source (a code is never a slice of the input; anything unmatched is `other`):
  `record_reason_kinds` (the path-free kind of each classifier reason in
  `record.json`, actions per kind: `failed`, `vcs-read`, `daemon-left`,
  `trace-incomplete:unstable-path`, `trace-incomplete:recipe-mismatch`, ...),
  `record_parse_reasons` (record.json's `parse_reasons` by code:
  `unparsable-line`, `unresumed-call`, `size`, `sandbox-refused`,
  `unclean:<reason>`, `refused:<syscall>`, ...; `refused:<name>` keeps the name
  only when it is a member of the parser's closed refused class
  (`trace.SyscallClass`), else `refused:other`), `parse_refusals` (the same codes for the traces this
  census's own parse refused), `record_parse_reasons_by_language` and
  `parse_refusals_by_language` (the same two tables as `{language: {code: n}}`,
  with the language of the share-by-language section: a validated name, at most
  64, then `other`), `parse_refusal_syscalls` (`{code: {syscall: n}}`, from the
  census's cross-checked parse: the closed-table name of the call the refusal was
  raised for, or `other` when it is tied to none, e.g. `unparsable-line`; it is
  `trace.Error.Call`, set only to a `trace.SyscallClass` name and never part of an
  error string or a record, and `refused:<name>` uses its own name),
  `parse_refusal_syscalls_by_language` (`{language: {code: {syscall: n}}}`),
  `record_failed` (the text of `record.failed`
  mapped to `timeout`, `traces-too-large`, `record-too-large`, `panic`,
  `classifier-failed`, `classifier-not-started`, `record-invalid`, `other`) and
  `sidecar` (`steps`, `truncated` by reason, `not_clean`, `unclean` by reason,
  `exit_125`, `root_status_nonzero`).
- `host`: the image id, kernel, strace version line and shard index and count
  from the verified `pre/`, and the first `model name` of `/proc/cpuinfo`, all
  sanitised.
- `trace_sizes`: `all` and `root_ok` (steps with `root_status` 0), each `n`,
  `p50`, `p90`, `p99`, `max` (nearest rank) and `log2_buckets`.
- `timing`: distributions of `step_wall_ms`, `parse_ms`, `parse_share`, and the
  counts `share_over_10pct`, `parse_over_5s` (the two limbs G2 had; both are reported, not gated), `g2_failing`
  (either limb; reported, not a gate, D49) and `walls_dropped`; `parse_mb_per_s` (throughput distribution);
  `by_size` (per log2 size bucket: n and the `parse_ms` and `parse_share`
  distributions); `g2_at_p99` (the same two limbs over the traces at or above
  the census p99 size, where G2 is defined); `g2_target` (the D49 reported target: `target_share_p50` 0.10, `share_p50`, `met`, `share_n`, `image_id` and `parse_mb_per_s_p50`; reported, not a gate); and `first_parse` (the first timed
  parse alone, whatever its outcome (`accepted` says which), its lazy
  initialisation included, which each classify child pays
  again, so the per-action child cost can be added back). Report-only.
- `per_action`: the total trace bytes of each action against the 2 GiB cap
  (`DefaultMaxTraceTotal`): a distribution, how many reach 50%, 90% and 100% of
  the cap, and the ten largest (sanitised names).
- `cross_check`: `traces`, `bytes`, `accepted`, `mismatches`, `allowed`,
  `allowed_rows` (`allowed` by the NAME of the section 4.9 stricter-only row that
  allowed each: a fixed program string of `crosscheck.go`, from
  `trace.ParseCrossCheckedDetail`, whose per-call log gives each row and line),
  `refused`,
  `skipped_too_large` and `skipped_bytes` (a trace over 256 MiB is timed but not
  cross-checked, because the cross-check buffers it whole and parses it twice; the
  workflow should set `GOMEMLIMIT`) and at most 20 `details` (an action's digest prefix, step and the mismatch's
  `Detail`, sanitised and cut; never the window of trace lines).
- `languages`: per language (the plan's, else `code/packages/<lang>/`; at most
  64, the rest `other`; shown as code spans in list items, so `www.x.example` and
  `:emoji:` cannot autolink or render): `actions`, `recordable` (`record.json` believed and
  `writes_complete`: NOT "would emit"), `clean` (recordable with an empty census;
  labelled "no-reason" in the Markdown), `recordable_share` (of all actions),
  `built_ok` (a believed record with no `failed` reason; when the parse half
  failed and there are no reasons, no started step exited non-zero; an action
  with no believed record is not built_ok), `emittable` (parse ok, writes
  complete, ZERO reasons in `record.json`, a plan entry with NO gate and a weak key
  (the merge turns a gate into `precondition:<gate>` and an unkeyed action has
  nothing to be emitted under), and the shard not void: a collect header that
  names a void reason sets `actions.shard_void` and makes every `emittable` 0;
  unknown without `--records`). `emittable` is an UPPER BOUND: it is per action
  and before the merge, so it cannot see reads-product or any cross-action rule.
  `gated` and `unkeyed` count the actions whose plan entry has a gate or no weak
  key (overall and per language), `writes_unknown` the actions with a believed
  record whose `writes_complete` is false, plus `record.failed` and every action with no verdict at all, and
  `truncated` the actions with a KNOWN write set whose execution was cut short,
  the merge's `unkTruncated` (a believed record with a `failed` or `daemon-left`
  reason, or fewer steps run than the plan lists; mirrored from recmerge
  `unknown()` as far as the census's inputs allow, and not counted again when
  already in `writes_unknown`), `relaxed_would_lose` the actions the relaxed
  merge would drop (equal to `truncated`), `relaxed_merge_voids_shard` true when
  `writes_unknown > 0` or the shard is void (a hard unknown voids both
  verdicts), and
  `actions.strict_merge_voids_shard` is true when `writes_unknown > 0`,
  `truncated > 0` or the shard is void (the Markdown line "strict merge would
  void this shard: yes/no"). Since D36 a failed action with complete writes is
  `truncated`, not `writes_unknown`. The three counts are additive JSON keys,
  also per language.
  The shares
  `recordable_share_of_built_ok`, `emittable_share` and
  `emittable_share_of_built_ok` (a share over zero is 0). `actions` carries
  `built_ok`, `emittable`, `gated`, `unkeyed`, `writes_unknown`, `shard_void` and
  `strict_merge_voids_shard` too. An empty census beside a
  record that holds a reason (an `exec-format` one, say) is `clean` but not
  `emittable`: "clean" never meant "would emit".
- `shapes`: at most 8 distinct redacted outlines of the line where the grammar
  stopped (a refusal with a line, origin `refusal:<parse code>`) or at which a
  stricter-only divergence was allowed (origin `allowed:<row>`), each with its
  count, first location (digest prefix, step, line) and its `lines`: the pid's
  pending `<unfinished ...>` half, when there is one, then the stop line. A line
  is `{role, line, tokens, facts, truncated}`. `trace.Shape` (never
  `trace.Diagnose`, whose window quotes the trace) prints ONLY token TYPE names
  from `strace.tokens` (a name off the closed list is `TOKEN?`), a character class
  for `WORD` and `ASSIGN` tokens (`digits`, `hex`, `upper`, `lower`, `mixed`), the
  call name of `SYSCALL`, `RESUMED` and `WORD` tokens only when it is in the
  closed syscall table, ASCII punctuation for `PUNCT` (at most 8 characters; a
  non-ASCII run is `\x80`), nothing of `QUOTED`, `ANNOTATED`, `ERR_*`, `PID` or
  signal text beyond the type name, at most 96 tokens a line, and the `lineFacts`
  booleans and three counts. The trace is streamed (a line is read to 1 MiB).
  Two budgets, so a flood of one refusal cannot starve the allowed-divergence
  lines: `allowed:*` outlines get 8 slots and 32 computations of their own;
  refusals get 8 slots, 32 computations and 2 per distinct refusal code. A
  (trace, line) is shaped once, and all shaping together scans at most 4 GiB (twice
  each trace's size is charged); `bounds.shapes_dropped` counts the rest. A
  refusal's line is the error's own line (the call's start for an
  `unresumed-call`), and shapes exist only for cross-checked traces, not those
  skipped as too large. `TestParseCrossCheckedOnlyInCensus` keeps `Shape`, `LineShape`,
  `ParseCrossChecked` and `ParseCrossCheckedDetail` out of every production file
  but their definitions and `census_report.go`.
- `reasons[].ops`: for `host-read`, `trace-incomplete:proc-path` and each
  `trace-incomplete:unstable-path:<sub>`, the occurrences by op class (from the
  `census.json` `ops`, validated against the closed list). The Markdown has
  "Parse reasons by language" and "Parse refusals by system call" sections, every
  name in a code span.
- *Labels.* `kiln record census` runs with the RUNNER's real HOME and temp
  directory. The classifier labels the STEP's fresh HOME and TMPDIR `$HOME` and
  `$TMPDIR`, so the invoker's are labelled `$RUNNER_HOME` and `$RUNNER_TMP`
  unless they are the very directories the sealed `host-facts.json` names (then
  they are the step's and keep `$HOME` / `$TMPDIR`; a trailing slash is ignored). The Markdown section is
  "Share by language"; a record's reasons make `recordable` and `emittable`
  differ, and the line says so.

*Bounds.* If the encoded JSON exceeds 512 KiB, the top lists are halved until it
fits (`bounds.json_lists_cut`); the Markdown is rendered the same way and then
cut at a line boundary (`bounds.md_truncated`). The summary holds no argv, no
environment value and no trace line.

*Markdown safety.* The output may be appended to `$GITHUB_STEP_SUMMARY`. Every
path is written inside a code span whose delimiter is longer than any backtick
run in the text; every other piece of text has `\` `` ` `` `|` `<` `>` `[` `]`
`&` `*` `_` `#` `~` backslash-escaped; there is no table cell that holds
attacker text (languages, kinds and every other name are code spans in list
items); no raw control character occurs; and no line starts with `::`
(every line starts with a fixed marker of ours).

*Exit codes.* 0: the census ran and the cross-check found nothing. 4: a
cross-check mismatch, any allowed divergence (strict: on a real trace there
must be none) or any `parse_panic` (a trace that crashes a reader is
soak-relevant; it wins over exit 1). 1: infrastructure failure (no traces, an unverifiable `pre/`, an
output that could not be written). 2: usage. The summary is written in every
case that got as far as reading the directory. Histograms and G2 (a reported target, D49) are report-only
and never change the status. The command refuses, writing nothing, when `--dir`
is not a record directory (a real `pre/` and `traces/`), when a path is not
absolute, when an output exists (outputs are created `O_EXCL`, mode 0600), or
when an output would lie inside `--dir`: it never writes there.

*The Markdown verdict.* The `Verdict` section states the cross-check status, and on
the next line the count of traces SKIPPED as too large to cross-check
(`skipped_too_large`, with their bytes), so an "ok" cannot be read as covering a
trace that was only timed. The cross-check status is `FAILED` also when a
`parse_panic` was counted (exit 4 says the same). The `Host` section carries the
CPU model.

**The census workflow (census PRs D and E): `.github/workflows/kiln-census.yml`.**
One job, `census`, runs on `ubuntu-24.04` and `ubuntu-26.04` (the
`kiln-sandbox` job's labels; `fail-fast: false`), `permissions: contents:
read`, a `kiln-census` concurrency group that queues and never cancels, and a
job timeout.

- *Triggers (census PR E).* `workflow_dispatch` for a deliberate run, and
  `schedule` at 04:00 UTC every Monday for the weekly one. A `schedule` event
  carries no `inputs` and the `default:` of a dispatch input applies only to a
  dispatch, so the first step falls back to the **scheduled** configuration: 15
  shards, indices `0,1,2`, `home: runner` — the PR8 gate's own configuration on
  both images. The dispatch defaults stay `15`, `0` and `fresh`, because they
  are what a person dispatching by hand starts from. A weekly run is therefore
  expected to be red until the recorder is clean: a failing `runner` census is
  the PR8 gate saying "not yet", not a broken workflow, and it is not a
  required check on any pull request.
- *Inputs.* `shard_count` (integer, 1 to 256, default 15, as `ci.yml`) and `shard_indices`
  (1 to 3 comma-separated integers, each below the count, no repeats, default
  `0` on a dispatch and `0,1,2` on the schedule). They reach the shell only
  through `env:` and are validated there
  (regular expressions, then arithmetic bounds) before anything runs; only the
  normalised values are exported to later steps. The first step also refuses a
  ref that is not the repository's default branch, because `prepare` would exit 3
  on it (P2: trusted runs only).
- *Recording.* Full checkout (no token kept), Go, the AppArmor userns sysctl
  and strace (guarded: `command -v strace`, else `code/scripts/ci/apt-install.sh`;
  the text `apt-get update` is forbidden in any workflow). `kiln` and the build
  tool are built into `$RUNNER_TEMP/bin` with the Go caches in `$RUNNER_TEMP`
  too (P3 wants the binaries outside the checkout; the only state roots are
  `~/.cargo/registry` and `~/.cargo/git`, and only when `~/.cargo/bin` exists,
  so the fresh `$HOME` makes them trivially empty). For each sampled shard, in order:
  reset and clean the checkout, a fresh `$HOME` (state roots start empty), a fresh
  record directory and `/tmp/kiln-tmp`; `kiln record prepare`; the traced build,
  exactly the command of P1, launched under `env -i` with `pre/tools.json`'s
  `build_tool_env` and wrapped in `::stop-commands::<random token>`;
  `kiln record collect`; `kiln record census --dir <rec> --records
  <rec>/records.jsonl --out census-<os>-shard<i>.json --md census-<os>-shard<i>.md`
  with `GOMEMLIMIT=8GiB` (the cross-check buffers a trace of up to 256 MiB and
  parses it twice; the hosted runner has 16 GiB). A failing BUILD step is census
  data, not a job failure; a build cut off by the per-shard timeout (90 minutes),
  a failed `prepare` or `collect` and any non-zero census exit are.
- *Output.* Each Markdown summary is appended to `$GITHUB_STEP_SUMMARY`, and
  `census-*.json` and `census-*.md` are uploaded as the artifact
  `kiln-census-<os>` (30 days). The upload runs `if: always()`, and the job's
  verdict is decided by the last step, so a census that exited 4 still publishes.
- *Exit semantics.* The job fails when any census exits non-zero: 4 (a
  cross-check mismatch, any allowed divergence or a `parse_panic`), 1
  (infrastructure) or 2 (usage), or when `prepare` or `collect` fail or a build
  times out.
- *Never uploaded.* The record directory, the traces, the sidecars,
  `records.jsonl` and the strace stderr files: they hold host paths and command
  lines (4.3). Only the sanitised summary leaves the runner.
- *Limits: coverage (`home: fresh`, the default).* This mode records with a
  fresh `$HOME` and installs Go only. Steps that need Rust, .NET, Haskell, Python, Ruby or node fail at "not
  found" and are recorded as failed steps (`record_failed`,
  `sidecar.root_status_nonzero`). Go is about 298 of 5115 packages and 0.8% of
  build time; Rust alone is about 29%. So a clean census from this workflow is
  NOT evidence for the PR8 gate, and every shard's step summary says so in its
  first line (`coverage: fresh HOME, Go toolchain only; not the PR8 gate`).
- *`home: runner`: the PR8 census.* The `home` input (`fresh`, the default, or
  `runner`; validated in shell like `shard_count`) selects the layout. `runner`:
  - installs the toolchains the main build installs, with ci.yml's pins and
    SHA-pinned actions, before the job start is stamped: Python 3.13.17 (uv
    0.13.7, fontTools 4.68.0), Node 22.23.5 (npm 11.13.0), Rust 1.100.0 and
    cargo-tarpaulin 0.37.2, GHC 9.4.8 / cabal 3.10.3.0, .NET SDK 9.0.319, Ruby
    3.4.12 (bundler 4.0.24), and the apt packages the Rust and Haskell builds
    need (via `code/scripts/ci/apt-install.sh`). A step verifies each toolchain
    resolves under a declared tool root of `host-roots.json` and fails the job
    otherwise;
  - records with the runner's own `$HOME` (the checkout sits under it, as on a
    hosted runner) instead of a fresh one. The workflow never removes it.
    (Amended: it used to empty the state roots `~/.cargo/registry` and
    `~/.cargo/git` before each shard, which `prepare` would refuse as
    `state-preloaded`. The recording sandbox has no network, so every Rust
    step's cargo then retried `index.crates.io` for about 589 seconds and
    failed, shards hit the 90-minute cap after about ten actions, and Rust never
    built in any census. The state roots are now LEFT POPULATED; see the
    census-only flag below);
  - uses a 180-minute start deadline (not 200), because setup now takes up to
    about 30 minutes: 30 + 180 + 20 + 91 + 25 is about 346 minutes, inside 360;
  - fails the job on a shard that is not clean (`census_not_clean.py --strict`,
    below), and on a census that exited 0 but left no summary JSON.

  - requires, per shard and straight after `prepare`, that every installed
    tool probed ok INSIDE the sandbox: `ci/census_probes_ok.py` reads the sealed
    `pre/probes.json` (`toolchains`: sorted `{tool, status[, detail]}`, status one
    of ok, absent, refused, mismatch, timeout; the probes run in the recorder's
    own sandbox, so they see its PATH, roots and pins) and fails the shard, before
    the build, unless python3, ruby, go, node, cargo, rustc, dotnet and ghc are
    all ok (uv, bundle and cabal are reported only). The file is parsed strictly:
    a missing or malformed file fails. Without it a run in which every non-Go
    step still failed "not found" inside the sandbox (a PATH or pin mismatch, a
    root that is a host symlink) would pass `--strict` and the host-side check
    and be the fresh-HOME census under a PR8 label. (Amended: the summary line
    no longer says "not the PR8 gate unless every installed toolchain probed OK";
    it says the shard is void by design, see the census-only flag below);
  - pins the install locations that the actions would otherwise choose:
    `GHCUP_INSTALL_BASE_PREFIX=$HOME` (the runner image keeps ghcup in
    `/usr/local/.ghcup` behind a `~/.ghcup` link, and a symlinked root is refused;
    haskell-actions/setup@6037f33 only prepends `~/.ghcup/bin`) and
    `DOTNET_INSTALL_DIR=$HOME/.dotnet` (actions/setup-dotnet@0f4a354 defaults to
    `/usr/share/dotnet` on Linux when writable, else `~/.dotnet`).

  **A `runner` run that exits 0 is the PR8 gate**, with at least three shards
  sampled on both images (ubuntu-24.04 and ubuntu-26.04). Installing before the
  job stamp makes the apt packages under `/usr`, pip fonttools in the toolcache
  Python, npm@11.13.0 in the Node root and cargo-tarpaulin in `~/.cargo/bin`
  image or tool content. That is acceptable for a census, but they are inputs no
  key names: stage-4 PR 3 (`kiln-record.yml`) must decide how to key them, or keep
  them out of the record job, before it reuses this setup. Not installed, because
  `host-roots.json` declares no tool root for them: Lua, Perl, Elixir/Erlang,
  Dart/Flutter, Java/Kotlin, OCaml, Swift, Clang and Qt; their steps fail at
  "not found" and are recorded as failed steps, not as unclean. Expect new
  `prepare` refusals in runner mode (an `ro_roots` entry overlapping the runner's
  directories); they are the point of that run.
- *Census-only `--allow-preloaded-state`.* `kiln record prepare` takes an
  optional `--allow-preloaded-state`, used ONLY by the `home: runner` census.
  It skips exactly one thing: the fail-fast refusal of a non-empty state root
  (`stateRootEmpty`, in the sandbox configuration and in the probes' "left
  nothing" check; the tmpdir half of that check still holds). It does not touch
  RunStep's seq-0 marker (`traces/.state-preloaded`) or `collect`'s
  `state-preloaded` shard-void, which are the real protection (P15), so a shard
  recorded this way is void whenever a state root is non-empty (or cannot be
  inspected) at its first step, and preloaded state can never be emitted.  In
  the census the roots are always pre-fetched, so every runner shard is void:
  the census measures the recorder, it does not publish records.  (With empty
  roots the flag changes nothing and the recording is simply clean.) Before the shards
  the workflow runs `cargo fetch --locked` for every tracked `Cargo.lock` into
  `~/.cargo/registry` and `~/.cargo/git` (failures logged and tolerated), and
  `prepare` runs with `CARGO_NET_OFFLINE=true` (`CARGO_*` is in `env.json`, so it
  reaches every step) so cargo fails fast instead of retrying the network. The
  gate for a runner shard is therefore the cross-check (census mismatches,
  refused traces, the strict not-clean checks), not `emittable`. The real path
  to Rust recordability is the stage-5 keyed fetchers, which put the registry
  content under a key instead of preloading unkeyed state; until they land the
  Rust census is a measurement of the recorder, never a source of records. The
  flag must never be used in a recording job (`kiln-record.yml`).
- *"Clean" (for PR8).* A shard is clean only when its census exits 0 AND
  `skipped_too_large` is 0 AND `bounds.actions_skipped` is 0 AND the traces were
  not all refused by both readers (`cross_check.accepted` is above 0 when
  `traces` is). The workflow flags a shard that exits 0 but misses any of these
  as "not clean" in its summary (`ci/census_not_clean.py`, which reads the JSON,
  never the Markdown); it prints a positive "clean" line otherwise, so a missing
  line is never read as clean. With `home: fresh` that only annotates. A workflow
  that gates PR8 must FAIL on "not clean", so with `home: runner` the workflow
  passes `--strict` and the checker exits 1 (also for an unreadable summary).
- *Never partly green.* The job's status is the worst of every shard. A shard
  that was not started (the start deadline: no new shard starts more than 200
  minutes after the job start stamp), a failed
  `reset`/`clean` of the checkout, a failed `prepare` or `collect`, and a build
  that timed out or was killed (timeout or OOM) each fail the job, and each
  leaves a line in the summary. A failed `collect` still gets its census (it
  needs only `pre/` and `traces/`), run without `--records`. The status file is
  created as a failure before the loop, so a missing or unreadable one fails
  the job. The job limit is 360 minutes (the hosted maximum), because the stamp
  is taken after setup and the limit counts from the job's start: setup (at most
  15) + the start deadline (200) + reset and prepare (at most 20; `VerifyTree` has
  a 20 minute budget) + one shard's build (91) + collect, census and upload (at
  most 25) is about 351 minutes. Three slow shards cannot all complete: the third
  is skipped and the job fails, which is intended.
- *Job start stamp.* The workflow stamps `JOB_START` after setup, which is more
  permissive than section 5.3 ("the job's first step"). That is acceptable for
  this census, which warns when apt installed strace. Stage-4 PR 3
  (`kiln-record.yml`) must NOT copy it: it stamps first and fails if strace is
  missing.
- *Housekeeping.* `strace` is verified (or installed, with a `::warning::`)
  BEFORE the job start is stamped, so an apt install cannot turn every action
  into `host-read:setup-modified`. Every recorder command runs with `$RUNNER_TEMP`
  as the working directory, never the checkout. Between shards the checkout is
  restored with git run under the recorder's pinned configuration
  (`core.fsmonitor=false`, `core.hooksPath=/dev/null`, `protocol.file.allow=never`,
  as `source.ConfigEnv`). The sealed `pre/` (0444 files in 0555 directories) is
  made writable before a record directory is removed. Each shard's summary is
  appended only while the step summary stays under about 900 KiB (GitHub's limit
  is 1 MiB); past that, the shard gets a pointer to the artifact.

**P3. Binary placement (D15).** The Kiln binary (the running executable,
resolved through symlinks) and the build-tool binary must be regular files
whose path equals its own resolved path, and must lie outside every place a
BUILD can write or read: the checkout, `tmpdir`, every state root and every
`ro_roots` entry. They are SHA-256 hashed into the binding. `--build-tool` is
never looked up on `PATH`. `strace` defaults to `/usr/bin/strace`, is resolved
the same way, and is run once, as `strace --version`, to put its first line in
the binding. `--dir` obeys the same rule against the checkout and `tmpdir`;
inside a state root it is refused too, because creating it makes the root
non-empty (`state-preloaded`). In addition `prepare` refuses an `ro_roots`
entry that overlaps the record directory, `RUNNER_TEMP`, either binary, or the
directory of `GITHUB_ENV`, `GITHUB_PATH` or `GITHUB_OUTPUT`.

**Decided (found by implementing P9, review of 4b-1).** `Config.CheckHost`
used to refuse a configuration file inside `home`. On a hosted runner
`RUNNER_TEMP` is `/home/runner/work/_temp`, beneath `home` (`/home/runner`), so
the design's `--dir $RUNNER_TEMP/kiln-record` was refused. The rule is now the
precise one (§4.7): the configuration, which is read host-side before the pivot,
may not overlap `repo_root`, `tmpdir`, a state root or an `ro_roots` entry, in
either direction, because after the pivot a host path is visible only inside
one of those (every bind is non-recursive, and `Validate` refuses an `ro_roots`
entry that is or contains `home`). `home` itself is a fresh tmpfs the host's
copy never appears in, so it protects nothing in this rule. `--dir
$RUNNER_TEMP/kiln-record` works.

**P4. The record directory.** `--dir` must not be a symlink and must be
canonical (`EvalSymlinks(dir) == dir`), so no component can be redirected. It
must be absent or an empty directory; it is created `0700`, and every file
under it is created `O_EXCL`. Layout of the part `prepare` writes:

```
pre/binding.json  pre/host-facts.json  pre/inventory.json  pre/graph.json
pre/steps.json    pre/build-plan.json  pre/sandbox.json     pre/exec-wrapper.json
pre/tools.json  pre/classify.json  pre/tree                        (the last two: P13)
pre/probes.json                                                     (P14, P7)
pre/actions/<sha256(action)>.json
pre/SEALED        (sha256 and path of every other pre/ file, written last)
```

An existing empty `--dir`, and an existing `tmpdir`, must be owned by the
running user and not writable by group or others, and their mode is then forced
to `0700`. Action names never become path components; their SHA-256 does. Before
`SEALED` is written, `pre/` is walked: every entry must be the directory itself,
`actions/`, or a regular file `prepare` wrote (a stray file, a symlink or another
directory is refused). `pre/` is then made read-only (files `0444`, directories
`0555`); if that pass fails, `SEALED` is removed again. The modes are cosmetic:
integrity comes only from the files being unreachable from the sandbox, and from
`step` and `collect` re-verifying `pre/` (2b-2a and 2b-2b-ii, both done, list below).
`traces/` is created by `prepare` (0700, outside the sealed `pre/`, holding
`traces/.seq` = `0\n`; `prepare` also refuses a tree file over the 128 MiB a
sealed file is read back at), its per-action directories by `step` (2b-2a, decision P12), and `probe/` (0700, the traces of the structure probe, decision P14) by `prepare`. `probe/` is never sealed, never read by `collect` and never uploaded.

**P5. Refusals that void the shard before it starts.** After `VerifyTree`
(which supplies the commit) and before any export:

- any path in the pre-build inventory whose base name starts with `BUILD`;
- any file anywhere on disk, below the root `.git`, whose base name starts
  with `BUILD` and that the tree does not track. `Inventory` collapses an
  untracked directory to one entry, so a new package's `BUILD` inside it is not
  an inventory entry of its own; the walk is `lstat`-only and follows no
  symlink;
- any socket or FIFO below the root: the checkout is bound writable into every
  step, so one would reach a process outside the sandbox;
- any regular file below the root, tracked or not, with `st_nlink > 1`: a
  pre-run hard-link alias is the one write path no trace names (§5.5,
  Aliases). Directories and symlinks are not counted. The root `.git` is not
  walked.

At most `MaxReported` paths are named. On a platform without `st_nlink` the
check is an error, not a pass.

**P6. Graph, steps and plan are exported by the out-of-tree build tool, from
the verified tree.** In this order, with a scrubbed environment (Kiln's own
`envscrub`, so no secret-like variable reaches it):
`build-tool -root R -emit-kiln-graph pre/graph.json -emit-kiln-steps
pre/steps.json`, then `build-tool -root R -force -language all -shard-count N
-shard-weights code/specs/data/ci-package-durations.json -emit-plan
pre/build-plan.json`. Each runs under a 15-minute timeout; a non-zero exit or
a missing output fails. Each export is read once, from a non-following handle,
and the digest is taken over the very bytes that are then parsed. `graph_sha256`
and `steps_sha256` of the binding are the SHA-256 of the **file bytes** (both exports are byte-stable, §4.6); the
merge compares them with the same digest of its own regeneration (B3). The
plan must hold exactly `N` shards with indices `0..N-1`, and every package
name a shard lists must be a package of the plan; `partition_sha256` is
`recfmt.PartitionDigest` over `rel_path` projections. The plan is decoded
loosely (only `packages[].name,rel_path` and
`shards[].index,assigned_packages,package_names`), because it is the build
tool's file, not a Kiln format.

**P7. Host facts and the weak keys.** `pre/host-facts.json` is written, with
`recfmt.EncodeHostFacts`, before any BUILD: `image_id`; `root`; `home`
(`$HOME`, which must be set and absolute); `tmpdir`, `legacy.RecordTmpdir`;
`executor_args`, `legacy.RecordExecutorArgs`
(`-clippy=false`, `-prune-leaf-targets=false`); `env`, the process environment
selected by `env.json` (`legacy.RecordEnviron`) plus `HOME` and
`TMPDIR=<tmpdir>`, never `PATH`. Then the weak key of every package the shard
runs (`PackageNames(I)`) is computed from `Traced` over the verified tree and
those facts. `pre/actions/<sha256(action)>.json` (`kiln-record-action/3`)
holds, per action: `action`, `language`, `weak` (hex) or `refusal` (the
`Refusal` reason), `gate`, `steps`, the lines `kiln-legacy-steps/1` gives for it
(`[]` when it has no BUILD for Linux), and `tools`, the sorted names of the tools
the key pins (P13). A package whose key is refused still
gets an environment (`Traced.RecordEnv`), because it still runs. **Gates and the
toolchain probes (stage 4 PR 2b-2c-3).** `gate` is the reason the package must
not be recorded as a measurement, computed by `Traced.Gate`: the very
`gateFacts`/`gateFrom` that `legacy-plan`'s `record()` uses, so the plan and the
recorder cannot drift. Priority: the never list, then the other hard reasons
(`symlink:`, `unknown-repo-inputs:`, `unknown-toolchain:`, `no-build-file:`), then
probes (`probe-mismatch:`, `probe-timeout:`, `probe-refused:`, `probe-absent:`); the smallest
string wins within a priority. It is printable and clipped to 200 bytes. A gated
package still RUNS (and may also be a refusal, as for an unknown toolchain); its
record carries the gate and the merge turns it into `precondition:<gate>`, so it
is never emitted. The toolchain probes (`Toolchains.Probe` over every language of
the graph, D9) run on the first `Gate` call, in a sandbox, never on the host
(`Probe` with no runner would exec on the host, so `prepare` ALWAYS passes
`toolchainProber.Run`): plain `kiln sandbox-exec`, no strace, on
`<dir>/probe/toolchains-sandbox.json`, whose `repo_root` is the empty
`<dir>/probe/root` (cwd `.../root/p`), with `home`, `ro_roots` and `kiln_exe` as
in `pre/sandbox.json`, **its own `tmpdir` `<dir>/probe/tmp`** (a probe's argv tail
comes from the PR-editable `toolchains.json`, so it must not share the steps'
writable TMPDIR: that would plant files later steps read unmeasured), no state
roots, no masks, and the probe's
environment (`PATH`, `LANG=C`, `LC_ALL=C`, `TZ=UTC`, `HOME` replaced by the
sandbox's). The process gets `KILN_ACTION=p`, the safe `PATH`, `/dev/null` as stdin
and one capped stream for stdout and stderr; 60 s per probe and 10 minutes in
all (D8), the root killed with its process group at a deadline. A probe that
mismatches, times out, is refused or whose sandbox exits 125 is a gate; so is a probe
whose binary is **absent** from the fixed PATH and whose pin is a VERSION
(`probe-absent:<tool>`, probe priority). For a versioned pin, absence means the
version in the weak key was never checked, and otherwise a PR could point the
tool's probe at a binary that does not exist and skip the check; the gate is
fail-closed and the remedy is to declare the tool's root. An image pin is exempt: it
makes no version claim (its identity is the image id, and absence is identical on every
run of that image), a step cannot use a tool that is not there, so a gate would add
nothing and only cost yield. `kiln legacy-plan` gates the same way, since both use
`gateFacts`. Any other runner error (the sandbox cannot start, the
configuration is refused, an unarmed or changed environment) fails `prepare`, and
so does a probe sandbox that is systematically broken: when probes ran and EVERY
one of them ended in exit 125 (a single 125 stays a mismatch). After the probes
`prepare` re-runs the check of the structure probes on the REAL configuration:
tmpdir and every state root must still be empty (`unscrubbed:*`).
(`kiln legacy-plan --probe`, used by lookup, still probes on the HOST
(`legacycmd.go`), unlike the recorder; both are version queries, so they are
expected to agree.) The results are sealed in
`pre/probes.json` `toolchains`.

**P8. The host scan (D15).** Every `ro_roots` entry is walked on the host, one
goroutine per root, with `lstat` only: it follows no symlink. A socket or a FIFO
is an error naming at most `MaxReported` paths, and a root's walk stops at that
many. A root that is itself a symlink (`/bin`) is not entered. A directory with a
different `st_dev` is skipped only when `/proc/self/mountinfo` lists it as a
mount point (the bind is non-recursive); if mountinfo cannot be read, or does not
list it, it is scanned like any other, so the scan fails closed. A directory the
scan cannot list is skipped only when this process also lacks search permission
on it, judged by the effective uid, gid and supplementary groups against the
mode bits and confirmed by `access(2)`, unknown meaning searchable (a step is the
same user and cannot reach anything in it); a directory with
`--x` and no `r` hides its names but not a socket at a known name, so it fails the
scan, as does any other error. The scan of the sandbox's own view is the in-sandbox
structure probe (decision P14, **2b-2c-2**); it is also what covers content a host
mount point shadows beneath an `ro_roots` entry, the checkout or a state root
(§14).

**P9. The sandbox configuration.** `pre/sandbox.json` is generated from
`code/kiln/legacy/host-roots.json` as the tracked tree has it, and must pass
`sandbox.ParseConfig` (so `Validate`) and, on the host, `Config.CheckHost`.

- `repo_root` is the checkout; `home` is `$HOME`; `tmpdir` is
  `legacy.RecordTmpdir`, created `0700` if absent and required to be an empty
  real directory (a symlink or a populated one is refused, so the step's
  `TMPDIR` is fresh); `kiln_exe` is the canonical Kiln path.
- `ro_roots` are the `image` and `tool` roots after `$HOME` expansion, those
  that do not exist dropped, sorted. A root beneath `home` must be a real
  directory or file: a symlink is refused. When any root lies beneath `home`,
  `home` must not lie inside `/tmp`, `/var/tmp`, `tmpdir`, the checkout or a
  state root (§4.7, M2): a step could rename an ancestor of `home`.
- **State roots (S1).** The anchor of a tool root deeper than a direct child of
  `home` is sealed read-only as a whole (§4.7), so everything the toolchain
  writes beneath it must be a declared state root. The recorder carries a
  closed table of anchors: `~/.cargo` with the state roots `~/.cargo/registry`
  and `~/.cargo/git`. A tool root whose anchor is not in the table (a root
  `~/.local/bin` would seal all of `~/.local`) is refused, never anchored. Roots
  that are direct children of `home` (`~/.rustup`, `~/.dotnet`, `~/.ghcup`)
  have no anchor and need no state root: they are bound read-only as they
  are. A `state` root declared in `host-roots.json` is added to the list. Every
  state root must be absent or an empty directory (`unscrubbed:state-preloaded`
  fails `prepare`), so no cache is restored into a recording.
- `masks` is `["<root>/.git"]`: the sandbox does not have a `.git` at all, so a
  BUILD can neither write the object store, `.git/config` nor a hook (§4.8 list
  above). No state root overlaps `$HOME/.gitconfig`, `$XDG_CONFIG_HOME` (or
  `$HOME/.config/git`), or a directory of the `PATH` the keys were derived
  with: Kiln's own Git calls read those, and a writable one would let a BUILD
  steer them.
- `env_by_action` maps every package the shard runs to its scrubbed
  environment (`WeakKey.Environ()`, or `Traced.RecordEnv` for a refused one).
- An `ro_roots` entry **or a state root** that overlaps, in either direction, the
  record directory, `RUNNER_TEMP`, the Kiln or build-tool binary, or the
  directory of `GITHUB_ENV`, `GITHUB_PATH`, `GITHUB_OUTPUT`,
  `GITHUB_STEP_SUMMARY` or `GITHUB_EVENT_PATH` is refused. A **state root** also
  may not overlap the `strace` or `git` binary or any directory of the recorder's
  `PATH` (a read-only root may: the image's `strace` and `git` live under `/usr`),
  because it is writable from the sandbox.
- `XDG_CONFIG_HOME`, when set, must be absolute.
- An empty shard is not recorded: a shard that runs no package has an empty
  `env_by_action`, which `Validate` refuses, so `prepare` fails. This is the
  decision, not an accident: a recording that ran nothing is a void shard.

**P10. The binding.** `pre/binding.json` is `recfmt.EncodeBinding` of
`kiln-record-binding/2`: `repository`, `run`, `attempt` and `revision` (the
commit `VerifyTree` returned, which `GITHUB_SHA` equalled), `shard_index`,
`shard_count`, the digests of the graph, steps, partition, host facts,
inventory and sandbox configuration, `kiln_sha256`, `build_tool_sha256`,
`strace_sha256`, `git_sha256`,
`recorder` (`legacy.RecorderVersion`), `strace_version`, `kernel`
(`/proc/sys/kernel/osrelease`), `job_start`, `prepared_at` (wall clock, display
and audit only) and `prepared_ticks` (D26: field 22 of `/proc/self/stat` as `prepare` read it
FIRST THING, before it starts any process, that is `prepare`'s own start time
in clock ticks since boot, 100 per second; an integer from 1 to 100 years of
ticks; a binding without it, or with it out of range, is refused, which is why
the schema went from `/1` to `/2`). It is sealed from `prepare`'s own start,
not from a later clock reading, because `prepare` starts sandboxes itself (the
structure probe and the toolchain probes, whose argv a PR can edit and whose
timeouts run to minutes) and a leftover of one of those must count as new for
S3; its own start is earlier than anything it spawns, by construction, and it
is the same kernel field, unit and epoch as every comparand (no float parse, no
`/proc/uptime`, which a container may virtualise). Every
`pre/` file is written exactly as the `recfmt` encoder produced it, with no
trailing newline, so `binding_id` is the SHA-256 of the file and the merge's
strict decoder reads it back byte for byte. Given the same inputs and the same
clock, `prepare` writes the same bytes.

**P11. The recorder's PATH and its git (review of 4b-1, H1).** Kiln, the build
tool and the programs they start look programs up on the PATH they were started
with, and `ci.yml` puts `$GITHUB_WORKSPACE/.lua/bin` first. The checkout is
writable from every step, so a BUILD could plant `<checkout>/.lua/bin/git`, and
the next host-side `git` call would run it unsandboxed; a tracked file there
would be run by `prepare` itself. So:

- `git` is resolved **once**, from `--git` (default `/usr/bin/git`), with the same
  rules as the other binaries (absolute, canonical, a regular file, hashed into
  the binding as `git_sha256`), and `strace` likewise (`strace_sha256`). Both
  must lie outside the checkout, `tmpdir`, the record directory and every state
  root; being under a read-only root (`/usr`) is fine, because a step cannot
  write it.
- Every Git call Kiln makes in `prepare` goes through `source.NewGitAt(dir, git,
  path)`, which runs that file by path with an explicit PATH; the default
  `source.NewGit` is not used by the recorder.
- The process `PATH` must be non-empty, with no empty and no relative entry, and
  no entry (spelled out or after `EvalSymlinks`) may overlap the checkout,
  `tmpdir` or the record directory; it also stays out of the state roots (P9).
  The build tool is started with a scrubbed environment whose `PATH` is exactly
  that validated value. The build tool's own code is not changed.
- **The environment is an allowlist (M3, M4).** The process environment can steer
  the dynamic loader into a library planted in the checkout (`LD_PRELOAD`,
  `LD_AUDIT`, an `LD_LIBRARY_PATH` with an empty entry, which means the working
  directory, `GCONV_PATH`, `LOCPATH`, `GLIBC_TUNABLES`, `TZDIR`...), and the build
  tool starts `git` with whatever it inherited, so a `GIT_CONFIG_GLOBAL` file in
  the checkout naming a program as `core.fsmonitor` would run it unsandboxed.
  Naming the dangerous variables one at a time keeps leaving gaps, so both
  children get an allowlist and nothing else of the process environment:
  - the pinned `git`: `HOME`, the validated `PATH`, the validated
    `XDG_CONFIG_HOME`, and `source.PinnedEnv()`: `GIT_CONFIG_NOSYSTEM=1`,
    `GIT_CONFIG_SYSTEM` and `GIT_CONFIG_GLOBAL` both the null device (no global
    configuration is read because nothing Kiln asks git needs one, and a git too
    old for the variable still reads files the recorder keeps unwritable, P9),
    `GIT_OPTIONAL_LOCKS=0`, `GIT_TERMINAL_PROMPT=0`, `GIT_NO_REPLACE_OBJECTS=1`,
    `GIT_NO_LAZY_FETCH=1`, `LC_ALL=C`; it starts in `/`;
  - the build tool: `HOME` (git and the Go runtime want a home), the validated
    `PATH` (it runs `git`, `sh` and toolchain probes through it), the validated
    `LD_LIBRARY_PATH` when one was set, `source.PinnedEnv()`, and
    `source.ConfigEnv()`: `GIT_CONFIG_COUNT=3` with `core.fsmonitor=false`,
    `core.hooksPath=` the null device and `protocol.file.allow=never`, the same
    values Kiln's own git passes as `-c` (git 2.31 and later; an older git
    ignores them). The build tool reads no other variable for
    `-emit-kiln-graph`, `-emit-kiln-steps` and `-emit-plan`: its only
    `os.Getenv` is `GITHUB_OUTPUT` (paths these flags do not take) and its
    executor's `os.Environ` is used only when it RUNS a BUILD, which `prepare`
    never does. So no `GITHUB_*` value, no `TMPDIR` and no `GOFLAGS`-style variable
    is passed, and none needs validating. It starts in the record directory.
  - `LD_PRELOAD` or `LD_AUDIT` set and non-empty refuses `prepare`;
    `LD_LIBRARY_PATH` is validated like `PATH` (no empty or relative entry, none
    overlapping the checkout, `tmpdir` or record directory, symlinks resolved
    through the deepest existing ancestor and dangling links followed by hand, at
    most 40 hops, a loop refused); every other `LD_*` and `GCONV_PATH`, `LOCPATH`
    (colon lists) are not passed on. The kept entries are also kept out of the
    state roots.
  - Every host-side subprocess starts in a directory a step cannot write (the
    build tool in the record directory, `git` and `strace` in `/`), with absolute
    paths throughout; the build tool takes `-root` and does not depend on its
    working directory.
- `pre/tools.json` (`kiln-record-tools/1`) records the absolute `git`, `strace`,
  the `PATH` string and the build tool's exact environment (`build_tool_env`).
- **2b-2a** (`step`) and **2b-2b-ii** (`collect`, done). `kiln record step` and `collect` use only the recorded git path and
  the recorded `PATH`, and never look a program up on `PATH`.
- **2b-2a.** The workflow launches the build tool with **exactly** the recorded
  `build_tool_env` (nothing inherited). The build tool hands its own environment
  to the wrapper plus `KILN_ACTION`, `KILN_STEP` and `KILN_STEP_COUNT`, so
  `record step` refuses to run unless its inherited environment, apart from those
  three, equals `build_tool_env` exactly (in particular every `GIT_*`, `LD_*` and
  `PATH` entry). The comparison is of multisets of `NAME=value` entries after
  removing `KILN_ACTION`, `KILN_STEP` and `KILN_STEP_COUNT`, and a repeated name
  (in either) is a refusal.
- **2b-2a** (`step`) and **2b-2b-ii** (`collect`, done). `step` and `collect` re-verify `pre/` against `SEALED` and the binding
  id before using any file in it. The `0444`/`0555` modes are cosmetic; integrity
  comes only from the files being unreachable from the sandbox.

**P12. The step supervisor `kiln record step` (stage 4 PR 2b-2a).** The build
tool runs it once per BUILD step through `-exec-wrapper` (P1). It runs
outside the sandbox and is not traced itself; it traces the step and keeps
the facts about the tracer that the trace cannot contain. It classifies the action after its final executed step (P13).
(Amended by P15: the step that takes sequence number 0 first re-checks the state roots and, if one is not empty, leaves `traces/.state-preloaded` and still runs.)

*Before the step* (any failure here runs nothing, writes nothing but a
diagnostic on standard error, and exits 125):

1. `pre/` is re-verified (decision P11): `SEALED` is read strictly (one
   `<sha256>` and path line per file, sorted, no duplicate); every entry of
   `pre/` is a regular file `SEALED` lists, `SEALED` itself, or the `actions/`
   directory (nothing missing, nothing extra, nothing a symlink); every digest
   matches, taken over the bytes that are then used; `binding.json` decodes
   strictly and its host facts, inventory, sandbox configuration, graph and
   steps digests equal the sealed files'. The binding id is the SHA-256 of
   `binding.json`. The running Kiln and the `strace` named in `tools.json`
   are each a canonical regular file whose SHA-256 equals the binding's
   `kiln_sha256` and `strace_sha256`, and the Kiln path equals the sandbox
   configuration's `kiln_exe`. Every action file's steps equal, line for line,
   the sealed `steps.json`'s. Nothing is looked up on `PATH`. the shared
   `flock` on the record directory is taken FIRST, before anything of the
   directory is read (below).
2. The inherited environment equals `build_tool_env` of `tools.json` as a
   multiset of `NAME=value` entries, after removing `KILN_ACTION`,
   `KILN_STEP` and `KILN_STEP_COUNT`. An entry without `=`, and a repeated
   name (in either side, the three included) is a refusal.
3. `KILN_ACTION` names an action of the plan (`pre/actions/<sha256(action)>.json`
   exists, is sealed, and is in the sandbox configuration's `env_by_action`).
   `KILN_STEP` and `KILN_STEP_COUNT` are canonical decimal integers;
   `KILN_STEP_COUNT` equals the action's step count; `KILN_STEP` is below it.
   Steps run in order: every earlier step has its sidecar, and no trace of this
   step or a later one exists. Every entry of the action's directory is
   examined and step numbers compare as numbers, so a step that is run twice
   or out of order always leaves `<step>.rerun` (also when two supervisors of
   one step race: the loser of the trace file's `O_EXCL` leaves it) and exits
   125 without running; `collect` makes the action's write set unknown on any `.rerun`
   (the merge then voids the run, G1).
4. argv after `--` is exactly `sh`, `-c`, `<line>`, and the working
   directory is `<root>/<action>`. The line is compared with the action's
   planned step and the result is recorded (`line_in_plan`); a mismatch does
   not stop the step, because `collect`'s recipe check is the judge.
5. Free space under the record directory is at least the floor (4 GiB by
   default; a field of the options, because a heavy build may want a smaller
   floor and a separate cap; it decides only whether a step may START).

*Locks.* `step` holds a shared `flock` on the record directory for its whole
run, and takes it without waiting (a failure refuses the step). It is
close-on-exec, so strace and the step never hold it, and it dies with the
supervisor, so an orphan that outlives its build tool keeps `collect` out
until it is gone. The command also sets `PR_SET_PDEATHSIG` (`SIGKILL`) on
itself (Linux) so an orphaned supervisor does not outlive the build tool; if
the build tool is already gone when it is set, the step is refused. The
command pins itself to one OS thread (`runtime.LockOSThread`) for the life of
the process before asking. The arming depends on the build tool's forking
thread staying alive (the signal fires when that thread exits); a misfire
fails closed (the step dies, no sidecar, `collect` voids the shard), and the
`flock` protects `collect` on its own whether or not the arming works.

*Running the step.* `prepare` creates `traces/` (`0700`, outside the sealed
`pre/`); `step` creates `traces/<sha256(action)>/` and, should `traces/` be
missing, `traces/` too, always by `mkdir` first and, when another supervisor
got there, by checking the directory (a real directory, owned by this user,
no group or other bits), so supervisors that start together cannot refuse each
other. Never a symlink; an action name never becomes a path component, so this
replaces `traces/<action>/` of the design review. The trace is
`<step>.trace`, created `O_EXCL`. The supervisor starts

```
<strace> <sandbox.StraceArgs("/proc/self/fd/5")> <kiln> sandbox-exec \
         --config pre/sandbox.json --config-sha256 <binding.sandbox_config_sha256> \
         --stdout-fd 3 --stderr-fd 4 -- sh -c <line>
```

where strace's descriptor 5 is a pipe. `strace -o` takes a path only, so it is
given strace's own `/proc/self/fd/5`, which it opens like a file (`O_TRUNC` means
nothing to a pipe, and a pipe opened through `/proc` is its write end); the
supervisor reads the pipe, writes `<step>.trace` and hashes the bytes as it
writes them (`trace_sha256`, `trace_bytes`). At 1 GiB it keeps the bytes up to
the cap, marks the step truncated `trace-too-large`, kills strace and keeps
draining; a write error does the same with `trace-write-failed`. Nothing polls.
`sandbox-exec` is told the digest the sealed sandbox configuration must have
(`--config-sha256`, §4.7) and refuses a file of other content.

with the working directory unchanged, standard input `/dev/null`, an
environment of exactly `KILN_ACTION`, `KILN_STEP`, `KILN_STEP_COUNT`, the
recorded `PATH` and, when `build_tool_env` has it (it is the value `prepare`
validated), `LD_LIBRARY_PATH` (a pinned strace that needs it must find it),
descriptors 3 and 4 the write ends of two fresh pipes whose
read ends the supervisor copies to its own standard output and error (so the
build log shows the step's output), and strace's own standard output and
standard error each a private pipe, kept (first 4 KiB) and counted. All five
pipes are drained with one timeout: one still open after strace is gone is held
by a survivor, whatever strace's exit status (`strace-pipes-held-open`). The
step's descriptors 1 and 2 are therefore pipes by construction, and nothing
strace or the root prints can be mistaken for step output. `Pdeathsig` is
`SIGKILL` on a locked OS thread: a supervisor that is killed takes strace with
it, and `--kill-on-exit` takes the tracees. Exit
status 125 is recorded (`exit_125`) and never retried (§4.7). strace's `-s 4096`
cuts any string longer than that, so a BUILD line over 4096 bytes has an `execve`
argv that cannot be decoded exactly and can never be recorded (P13). strace's own
standard error, when not empty, is also saved (first 4 KiB) as
`<step>.strace-stderr` (`O_EXCL`) for whoever has to find out why a run failed.
It is untrusted diagnostic text (a step can make strace print what it likes):
`collect` treats it as opaque, never interprets it, and does not upload it in
the shard.

The drain after strace exits waits for the readers that cannot block (the
trace sink and strace's own two streams) without limit, and for the two that
write to the caller's stdout and stderr only for a short grace after the
timeout, then abandons them, so a caller that stops reading cannot hold the
supervisor.

*The workflow.* The recording workflow launches the build tool under
`env -i` with exactly `build_tool_env`: a shell adds `PWD`, `SHLVL` and `_`,
and any of them makes every step refuse (the environment must be exactly the
recorded one). It must not consume step output from the build step (a step
prints through to the build log, and GitHub interprets `::` commands in a
job's output), or must wrap the build in `::stop-commands::<random token>`.

*After the step.* The supervisor exits with the traced root's status, which
strace mirrors: strace's own exit code, or 128 plus the signal that killed
it. The sidecar `traces/<sha256(action)>/<step>.json` (`kiln-record-step/2`,
compact, no trailing newline, `O_EXCL`) holds: `action`, `step`,
`step_count`, `seq` (P13), `binding_id`, `line_sha256`, `line_in_plan`, `started`,
`started_at` and `ended_at` (RFC 3339, UTC, nanoseconds), `strace_exit` (-1
if it was signalled), `strace_signal` (0 if not), `strace_stdout_empty`,
`strace_stderr_empty`, `root_status` (strace's exit code: strace exits with
the root's status, and `collect` checks it against the root's exit line in
the trace), `exit_125`, `truncated` and `truncated_reason`
(`trace-too-large` or `trace-write-failed`), `trace_bytes`,
`trace_sha256` (of the bytes written, as written), `step_netns`, `clean` and
`unclean`. `clean` is the
supervisor's half of the G7 definition: strace started, was not signalled
(by anyone, so also not by the supervisor's own kill), was not truncated, and
wrote nothing to its standard output or error. The other half (the trace
parses, the root's exit line exists, and `strace_exit` equals it) is
`collect`'s. When strace did write to its standard error, the
sidecar also carries `strace_stderr_code` (additive, omitted when strace was
quiet, so the step schema stays at 3): the recorder, ON THE RUNNER, reduces
the first 4 KiB of that text to one code of a closed list (the stream also
carries the sandbox helper's stderr, so Kiln's own prefixes are matched first),
in this order of precedence: `sandbox-exec-notify` (`kiln sandbox-init:
exec-notify:`), `sandbox-helper` (`kiln sandbox-exec:` or `kiln sandbox-init:`),
`ptrace-listen-eio` (`ptrace(PTRACE_LISTEN` with `Input/output error`, the known
race on a dying thread), `attach-failed` (`attach: ptrace(PTRACE_SEIZE`, or
`PTRACE_SEIZE` with `attach`), `oom`, `tracee-gone` (`No such process`),
`unknown-pid` (`Exit of unknown pid`), `umove-failed` (`umoven:`/`umovestr:
short read`), `ptrace-other` (any other `ptrace(` failure, a bare `Operation not
permitted` from one included), `wait-failed` (`wait4(` or `waitid(`),
`exit-killed` (strace was signalled and nothing above matched) and `unknown`
(an EPERM that is not a ptrace call lands here). The strace wordings are from
memory of strace's sources, not checked against 6.19. The text is never copied into the sidecar, a record or the census (it stays in
`<step>.strace-stderr`); the census counts the codes, and the signals that
killed strace, per shard (`explain.sidecar.strace_stderr_codes` and
`strace_signals`, and `strace_exits` for the exit status of steps with stderr text as `exit-0`, `exit-1`, `exit-2`, `exit-125`, `exit-126`, `exit-127` or `exit-other`; `unrecorded` for text with no code). The field is
informational: the reason stays `strace-stderr-not-empty`, no key reads it,
and `kiln:recorder` is unchanged. `step_netns` is true only when strace was started with the
`kiln sandbox-exec` command above, the only way a step runs, because that
helper unshares the network namespace; it is false when the process could not
be started. `collect` copies it into `traced.Input.StepNetNS` only when the
step's own `execve` is in the trace (a refused sandbox has none).

**P15. `survivor` and `state-preloaded` (stage 4 PR 2b-2c-4).** Two header `void`
tokens that the merge already knew (M3, M4) and nothing wrote until now.

- **`survivor` (run-void).** Right after the exclusive lock and `VerifyPre`,
  `collect` scans `/proc` (`collect_survivor.go`; a stub that fails closed off
  Linux). A pid other than its own is a survivor when **S1** its `ns/mnt`
  differs from `collect`'s and its mountinfo lists `repo_root` as a mount
  point; **S2** a `cmdline` element is exactly `<dir>/pre/sandbox.json` or
  `<dir>/probe/toolchains-sandbox.json` (any owner); or **S3** it is ours, it
  started at or after `prepare`, and reading it fails with anything but
  ENOENT/ESRCH (or a read hit the 4 MiB bound, whoever owns it). "Ours" is the
  `Uid:` line of `/proc/<pid>/status` (real or effective equals our euid; an
  unreadable or unparsable status is ours; the owner of `/proc/<pid>` is only a
  further reason to say ours, never a reason to skip, because for a
  non-dumpable task the kernel makes it the root of the task's user namespace,
  which a hostile step could arrange to look like root). "Started after
  `prepare`" compares field 22 of `/proc/<pid>/stat` (parsed after the last
  `)`, in clock ticks since boot, 100 per second) with the sealed
  `binding.prepared_ticks` less 200 ticks (2 s); no wall-clock time and no
  `btime` is involved (D26). An unreadable or unparsable `stat` counts as
  after. The rule exists because a real survivor is always inspectable from
  `collect`: it lives in a user namespace our user owns, so we hold ptrace
  access to it even if it made itself non-dumpable; what we cannot inspect is a
  stranger's, or an OLD process of ours (a `systemd --user` manager, an
  ssh-agent) that no build started, and counting those would void every run.
  Each scan is bounded to 5 s (a `/proc` read can block on an uninterruptible
  task); one that is not done counts as a survivor. The start-time gate is boot-relative
  (D26): `prepare` reads its own `/proc/self/stat` field 22 first thing and
  seals it as `binding.prepared_ticks`, and the gate compares it with
  `/proc/<pid>/stat` field 22 of each process, both ticks since boot parsed by
  the same function, so a step of the wall clock (NTP, `date -s`) between
  `prepare` and `collect` cannot exclude a new process from S3. (Before D26 the
  gate converted `starttime` through `btime` and compared it with the
  wall-clock `prepared_at`, and a backward step of more than 2 s could.)
  `prepare` refuses an unreadable or out-of-range own start time. `collect`
  checks the sealed value against its own start time, read the same way, which
  cannot be earlier on one boot: if it is (a reboot between the two, or the two
  reading different clock sources), or cannot be read, the bound is zero and
  every process counts (fail closed). (An absurd `starttime`, over 100 years of ticks, counts as new.) There is **no cwd test**: a shell left in the
  checkout writes nothing, and a cwd test would void every run launched from a
  terminal in the repository. A process that vanishes while being read is not
  one. The scan repeats every 200 ms for up to 10 s (`CollectOptions.SurvivorWait`;
  negative means one look, for tests); a scan that cannot be made counts as a
  survivor. If one is left, the header `void` gains `survivor` and every record
  is an unknown write set (`markUnknown(rec, "survivor")`). `collect`'s report
  prints `post-build problem: survivor: pid N: S1|S2|S3 ...`.
- **`state-preloaded` (shard-void).** `prepare` and the probes require the
  state roots empty, but the roots can be filled between `prepare` and the
  build. `RunStep` therefore re-runs the `stateRootEmpty` check at sequence
  number 0 (the first step of the shard, under the serial lock); a root that is
  not absent or an empty real directory (any failure to say so counts) writes
  `traces/.state-preloaded` (O_EXCL, containing that root's path), and the step
  still runs. `collect` lstat's the marker, adds `state-preloaded` to the header
  `void` when there is any entry of that name or `traces/` cannot be asked, and
  `globalSeqCheck` knows the name, so the serial order still holds and records
  stay believed (the merge voids the shard's records, M4). A marker that cannot
  be written refuses the step, and that is a RUN-void rather than a shard-void:
  its sequence number and trace file exist but no sidecar, so `collect` sees a
  counter mismatch (`serial-violated`) and every record is unknown.

*Divergences from the design:* the marker lives in `traces/`, not in an action's
directory, so `scanAction`'s closed list needs no entry (only `globalSeqCheck`'s
known set); `SurvivorWait` 0 means the 10 s default, and a negative value is the
no-wait setting tests use.

*Obligations of `collect` (**2b-2b-ii**, except the parse half of the third, which `step` applies, P13), from the review of P12.*

- **2b-2b-ii. Done.** `collect` takes an **exclusive**, non-blocking `flock` on the
  record directory (`record.LockDir(dir, true)`) and refuses if it cannot: a
  step, or an orphaned supervisor, is still running.
- **2b-2b-ii. Done.** `collect` requires every trace to end in a newline, re-hashes it
  and requires the digest and size to equal the sidecar's `trace_sha256` and
  `trace_bytes`; a trace that does not is `unknown_writes`.
- **2b-2b-ii. Done.** `collect` compares the `--sha256` argument of the init's traced
  `execve` (the digest the root read the configuration with) and the
  `--config-sha256` of the root's with the binding's `sandbox_config_sha256`,
  so the configuration a step ran under is the sealed one even where the
  recorder verified the file by content but the helper opened it by path.
- **2b-2b-ii. Done.** `collect` treats any `.rerun` marker, any sidecar without a trace
  (or the reverse), any `trace-write-failed` and any `strace-pipes-held-open`
  as an unknown write set.
- **2b-2b-ii. Done** (from the review of P13). `collect` requires the `seq` values of
  the whole shard to be globally distinct, each below the final value of
  `traces/.seq`, and consecutive in step order within each action; any `.overlap`
  marker voids the shard. It requires `steps_run` of a `record.json` to equal the
  number of started sidecars of the action, treats `record.failed` as overriding
  `record.json` (both present is a failure), requires the record's `binding_id`
  and `sealed_sha256` to equal the sealed directory's, and treats a `record.json`
  that does not decode (or a stray `record.json.tmp-*`) as an unknown write set,
  never as "no record". It must not read `sandbox-refused` in `parse_reasons` as
  "the step wrote nothing": a refused step ran nothing, but the sandbox may have
  failed after it had changed the disk, so the action's write set is unknown.

*As implemented by `collect` (2b-2b-ii).* The code is authoritative:
`collect.go` (the command), `collect_actions.go` (one action), `collect_post.go`
(the checkout after the build) and `results.go`. `--out` is `records.jsonl`: the
header line, then one record per action of the sealed plan, sorted by action.
The binding, the host facts and the pre-build inventory of the shard artifact
are the sealed files of `pre/`, byte for byte; `collect` decodes its own output
together with them (`recfmt.DecodeShard`, default limits) before it writes it,
atomically (`link(2)`, never replacing a file), mode `0600`. `--out` may not be
inside `pre/`, `traces/` or the checkout, its parent must be canonical, and it
must not exist. Two outcomes:

- *Refusal* (exit 1, nothing written): the directory is locked (a step or an
  orphaned supervisor holds it), `pre/` or the running Kiln or the pinned git is
  not what the binding says, `--root` is not the sealed checkout, `--results` is
  unreadable or lists a package that is not an action of the plan, or the shard
  cannot be made to decode even with every record unknown.
- *Void* (exit 0, the shard is written and the merge discards it): everything else
  that is wrong. `collect` prints the header's counts and every unknown write set.

*Per action.* The record's `status` is the build tool's (`not-run` when the
results omit the action), `steps_total` the plan's, `steps_sha256` the digest of
the sealed lines, `weak`/`refusal`/`gate` those of the action file. `steps_run` and the per-step summaries come from the started
sidecars. The raw record is `record.json`'s, with `writes_complete` from the
classifier's own answer through `recfmt.CompleteOf`. The write set is UNKNOWN
(`unknown_writes`, an empty raw record, an explicit `writes_complete: false`;
the merge voids the run) when: the directory holds an entry the supervisor does
not make, a stray `record.json.tmp-*` or a non-regular file; a `.rerun` or
`.overlap` marker; a sidecar without a trace or the reverse, or steps missing
before a later one; more steps than the plan; a sidecar that does not decode or
is of another action, step, count or binding; a step line that is not the
planned one (the digest is recomputed from the sealed plan, the sidecar's
`line_in_plan` is not trusted alone); `trace-write-failed` or
`strace-pipes-held-open`; a trace that is not a regular file of exactly
`trace_bytes`, does not hash to `trace_sha256` or does not end in a newline;
`seq` values that are not consecutive within the action; `record.failed` (which
overrides `record.json`, both present is a failure); no verdict for an action
that ran; `record.json` that is over 256 MiB, does not decode, is of another
action, binding or sealed directory, counts other started steps than the
sidecars, or has `parse_ok: false` (every parse reason, `sandbox-refused`
included, is an unknown write set); the build tool's step count or total not
equal to the traced and planned ones; `built` with a non-zero step or with no
traced step (except an empty recipe, below); or the action is in the results
with steps but a step directory is not. An action that ran nothing (no steps in
the results, no directory) is not unknown: it carries an explicit
`writes_complete: false` and the merge charges it as a truncated execution.

*An empty recipe is vacuously complete.* A package whose BUILD runs nothing (a
comment-only file: the build tool drops comments, blank lines and the `set -e`
preamble, so the plan has no step and the results say `built`, 0 of 0) has no
trace and no write set to be unknown about. When the plan has no step, the
results list it as `built` with `steps_run` and `steps_total` both 0, and the
supervisor left no directory for it, the record is the empty raw record with
`writes_complete: true` (derived with `recfmt.CompleteOf`), `steps_run` 0. Any
other fact (a directory, a started step, a different total) leaves it unknown.
Without this rule one such package, and the repository has them
(`code/programs/typescript/visicalc-deno/BUILD`), would void every run. The merge
accepts the record and does not emit the action (a manifest needs its own `pkg:`
entry). The shard-wide serial rules below still override it.

*Across the shard.* Every record becomes unknown (`serial-violated`) when any
`.overlap` marker exists, when `traces/.seq` is missing or not a counter, when a
`seq` is not below it or is used twice, when the number of sidecars is not the
counter's value, or when `traces/` holds an entry that is not `.seq`, `.serial`,
`.collected` or an action's directory. One shape of the counter mismatch has its
own reason, `interrupted: a step started and was never accounted for (the build was stopped, or its supervisor died)`: exactly one
number is missing, it is the highest (counter minus one), and exactly one step in
the shard has a trace and no sidecar (the build was killed, for instance at a census
shard's time cap, while that step ran). Every record is still unknown and the order
is still in doubt; only the words differ, and an action with no step directory at
all in such a shard says `not run: the build stopped before this step`. Any other
mismatch (two missing, a missing number below the highest, a missing highest with no
trace) stays `serial-violated`. A shard that does not decode under the format's caps is
replaced by the same shard with every record unknown and the header void
`records-too-large`.

*After the build.* `VerifyTree(root, binding.revision)` with the pinned git;
`record.Fsck`; `Inventory`. Any error of the three adds the header void
`tracked-changed` (the format's closed vocabulary has no other for "the checkout
is not what the commit says"). `tracked_changed_unexplained` counts the paths of
`TreeError.Paths` (the whole list, not the 16 named) that no record's
`tracked_writes` contains or has as an ancestor directory (compared by path
component); `post_inventory_unexplained` counts the paths of the post-build
inventory that are not in the pre-build one and that no record's `writes` equal
or have as an ancestor (by component). A write BENEATH a new path is not an
excuse for it: the trace named something under the directory, not the directory,
and whatever else is in it is nobody's (the write roots and the inventory share
one granularity, so an honest write never lies beneath a new inventory path).
Why a check failed is returned to the command (`CollectResult.PostProblems`,
printed as `post-build problem:` lines: the check, its error text, the first 16
unexplained paths; sanitised, at most 32 entries of 512 bytes), never written to
the shard.

*The arguments and the directory.* `--results` may not be inside the checkout
(a BUILD could write it), like `--out`; `--out` must not exist. Before `collect`
reads anything of `traces/` it creates `traces/.collected` (`O_EXCL`), and `step`
refuses (125, nothing run, no sequence number, no marker) once it exists: a step
that took the shared lock a moment after `collect` released the exclusive one
cannot add to a directory that has been read. A directory is collected once; the
refusals about the arguments all come BEFORE the tombstone, so one does not use
up the collection. A failure AFTER the tombstone (a shard that cannot be encoded, a
transient `--out` write error such as ENOSPC, which also takes the half-made file
back) burns the record directory: the next `collect` is refused. That fails closed,
because a missing shard is a run-void in the merge (B2), never a believed one.
`--results` needs a canonical parent, as `--out` does, and `step` refuses when it
cannot tell whether the tombstone exists (only "not there" lets it go on). Every git call `collect` makes has a 10 minute deadline and
keeps at most 1 GiB of stdout (more is an error, never a truncation) and 1 MiB of
stderr. `record.json` and the sidecars are decoded with `legacy.PolicyDecode`
(no duplicate and no case-variant key); the temporary file of the atomic writers
has a random suffix and the parent directory is synced after the link.

*Divergences from the text above, each deliberate (all stricter).*

1. A plain Git error from `VerifyTree`, a failing `fsck` and a `TreeError` with
   no path list each ALSO add one to `tracked_changed_unexplained`, and an
   `Inventory` error adds one to `post_inventory_unexplained`. The text asks for
   a shard-level void "like `tracked-changed`" (M4), which drops only this
   shard's own actions. But what a failed check means is that the paths that
   changed cannot be listed, so none can be shown explained; the counts mean "at
   least this many", and the merge turns them into a run-void (M2b, M3), which is
   what an untraced write needs (G1). Reverting is dropping the increments.
2. The init's traced `--sha256` and the root's `--config-sha256` (the third
   obligation) are compared by `step` only, as the parse half of G7 (P13): a
   mismatch is `parse_ok: false` in `record.json`, which `collect` turns into an
   unknown write set. `collect` parses no trace (it "classifies nothing"), so it
   does not compare them a second time. If a second line is wanted, `collect`
   would have to decode every trace, which costs what P13 moved out of it.
3. The count of sidecars must equal the final value of `traces/.seq` (stricter
   than "each below the final value"): a step that took a number and left no
   sidecar is otherwise visible only through its orphan trace.
4. A `.rerun` marker makes THAT action's write set unknown (the text says "voids
   the shard"); the merge turns any one unknown write set into a run-void (G1),
   so the effect is the same, and the report names the action.
5. A `TreeError` raised before the disk was compared (the index and the commit's
   tree disagreed: `TreeError.Partial`) counts one more, because the paths it
   names are not the files that changed on disk.

*A limitation, now narrowed (D36).* A failed action that stops mid-recipe has
fewer traces than recipe lines. When the traces are a byte-exact prefix whose
last step exited non-zero, the classifier raises only `failed` and the write set
is whatever the traces say (section 4.5, D36); the merge's `failed` handling
(M-1, truncated execution) applies. Any other shortfall (the last step
succeeded, a step differs) is still `trace-incomplete:recipe-mismatch` (W10) and
an unknown write set: the merge treats it as run-void in the relaxed report too.
`collect` reports what the classifier says.

**P13. Records are classified at action end, and recording is serial
(2b-2b-i).** `traced.Classify` resolves paths against the disk, so a record
classified after a later action changed a link, directory or file it resolved
through places its writes and observations wrongly. M8 and M10 catch this in
the strict verdict only through M-3, not at all in the relaxed one, and not for
host state roots. So:

1. *Serial.* The traced build runs with `-jobs 1`. `step` holds an exclusive,
   non-blocking `flock` on `traces/.serial` from before its checks until it
   exits; a step that cannot take it leaves `<step>.overlap`, runs nothing and
   exits 125 (the marker only for a lock that is HELD, `EWOULDBLOCK`; any other
   failure to take it is a refusal with no claim of overlap). Under that lock it
   assigns the sidecar a `seq` from `traces/.seq`, which `prepare` creates as
   `0\n`; the update is one `pwrite` at offset 0 and a sync (the number never
   gets shorter, so there is no truncate for a kill to land after), and an empty,
   missing or malformed counter is refused, never reset. A refused step takes no
   number. `collect` requires each action's `seq` values to be contiguous; any
   `.overlap` or gap makes every record of the shard `unknown_writes`
   (`serial-violated`, M1).
2. *Sealed inputs.* `prepare` also writes `pre/tree` (`kiln-record-tree/1`:
   every tracked path of the verified commit with mode, blob id and link text),
   `pre/classify.json` (the host roots and pinned ecosystems) and each action's
   `tools` (action files became `kiln-record-action/2`, and /3 with `gate`). Neither `step` nor
   `collect` builds a `legacy.Traced` or runs Git to classify.
3. *Classification.* After the sidecar of the action's last step, or of a step
   that did not start or exited non-zero, and before releasing the lock, `step`
   parses the action's traces, applies the parse half of G7, and runs
   `traced.Classify` with `Tree` from `pre/tree` and `FS` a live reader that
   follows no symlink (`lstat`, `readlink`; a head is read only from a regular
   file opened `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY` whose `/proc/self/fd` path
   equals the path asked). `Tree.Head` returns disk bytes only when they hash
   to the sealed blob id. The result is `traces/<sha256(action)>/record.json`,
   written atomically (to a temporary name, then `link(2)`, never replacing). The
   work runs in a **child process**, `kiln record classify-action --dir D
   --action A --step N --data-limit B`, so that running out of memory or any
   other fatal runtime error is a process that did not exit 0 and not the end of
   the supervisor: the child is given only those paths and numbers, re-verifies
   `pre/` and the sidecar itself, and writes `record.json` itself, and the parent
   decodes the file again (action, binding and `sealed_sha256` must match). Its
   memory is bounded by `GOMEMLIMIT` (soft, 5 GiB) and `RLIMIT_DATA` (hard, 8 GiB,
   set by the child on itself); `RLIMIT_AS` is not used because the Go runtime
   reserves address space it never touches. The parser's limits are 1 GiB and 8
   million events per step (the supervisor's own trace cap, so a trace the
   supervisor accepted is one the classifier reads; the parser's defaults of 64
   MiB and 1 million records are not used), and all the action's traces together
   may not exceed 2 GiB. A panic, a crash, the child killed, a memory overrun, a
   classification over 10 minutes (the parent kills the child), traces over the
   cap, or a child that exits 0 without a valid record leaves `record.failed`
   (atomically, the reason as text) instead; the epilogue can therefore add up to
   10 minutes to the last step. The step's exit status is never changed. A step
   asked to run after its action's `record.json` or `record.failed` exists is
   refused (125, nothing run). Every unclean sidecar is a parse reason
   (`step N: unclean:<reason>`), so an action with an unclean tracer has no raw
   record. A BUILD line over 4096 bytes can never be recorded: strace runs with
   `-s 4096`, the step's `execve` argv is cut, and the recipe and digest checks
   fail. Likewise any `execve` whose printed argv exceeds the parser's 1 MiB line
   cap (a huge glob expansion, an `xargs` batch) fails closed: the trace is not
   parseable, and the run is voided. The supervisor reads the child's
   `record.json` through a non-following handle and refuses a file over 256 MiB
   (`record-too-large`); the child drops a raw record whose lists reached the
   classifier's own cap (`writes-too-many`), which is incomplete anyway. A failure
   removes any `record.json` before linking `record.failed`, so one verdict
   remains (and where it cannot be removed `record.failed` overrides). The child
   is `/proc/self/exe` (the image the supervisor verified), sets
   `oom_score_adj` 1000 and `RLIMIT_DATA` to the smaller of 8 GiB and half of
   `MemTotal` (soft limit scaled 5:8), and refuses unless the step ends its action
   (the last step, or one that did not start or exited non-zero) and no verdict
   exists yet.
   The parse half of G7 is: every trace parses (with the sandbox helper as
   root); the root's exit line equals the sidecar's `root_status`; the traced
   `execve` of the init's `--sha256` and the root's `--config-sha256` equal
   `binding.sandbox_config_sha256`; and a status of 125 is a sandbox refusal
   exactly when the step's own `execve` is absent from the trace.
4. *`collect`* classifies nothing. A missing or failed `record.json` for an
   action that ran a step is `unknown_writes`. "Post-run disk" in §5.1 now reads
   "the disk at the end of the action".

*As implemented.* `pre/tree` is line-oriented, sorted bytewise by path:
`kiln-record-tree/1\n`, then `<mode> <blob id> <path>\n`, and
`<mode> <blob id> <path> <link text>\n` for mode `120000`. A path or link text
is written with every byte that is not visible ASCII, and `%`, as `%XX` in
upper-case hex, so a line splits on single spaces and each byte string has
exactly one spelling; the decoder refuses anything else (an unsorted or repeated
path, a path that is both a file and a directory, a needless or lower-case
escape, a link text on a file, mixed object-id lengths). A link text is checked
against its blob id before it is written. On this repository the file has
394 222 lines and 48 MB: written in about 0.3 s, hashed in about 20 ms (once per
step, by `VerifyPre`) and strictly decoded in about 0.2 s (once per action, by
`Pre.Tree`, only when a step classifies). `VerifyPre` hashes it and decodes
`classify.json` and the action files; the tree is decoded lazily.

The sidecar is `kiln-record-step/2` (adds `seq`, from 0). `record.json` is
`kiln-record-classified/1`: the raw record, `writes_complete` (the classifier's
own answer, which is not part of the raw record's JSON), `steps_run`,
`step_netns`, `sealed_sha256` (the digest of `pre/SEALED`), and `parse_ok` with
`parse_reasons`, so that `collect` can build a
`recfmt.ShardRecord` without parsing a trace. When any trace fails the parse half
of G7 the record carries the reasons and **no** raw record (`record: null`,
`parse_ok: false`, `writes_complete: false`); it is never an empty record.
`record.failed` holds the reason as text (`panic:`, `timeout:`, `traces-too-large:`,
`classified-twice:`, `the classifier failed: <exit status or signal>`, or the
classifier's own error). A sandbox refusal is a status
of 125 with no step `execve` in the trace, which the parser reports as
`helper-no-init` (the root died before it made the init) or `helper-no-exec` (the
init died before the step); the epilogue names both `sandbox-refused`. An
`.overlap` marker is not evidence that a step ran: `checkOrder` skips it, so a
step refused the lock can be run once the lock is free (the marker stays and
`collect` makes every record unknown).

*Why the silent case is the strong case for P13.* A link an action creates
itself is resolved from the DISK, not from the link text in the trace: that text
is used only under the fresh `$HOME` and `TMPDIR`. Only an `open` is checked
against the kernel's `-y` path. So `mkdir`, `rename`, `unlink`, `symlink` and
`chmod` through a link are placed wherever the link points when the classifier
looks, and nothing says it moved: an action that makes `L -> d1` and runs `mkdir
L/sub` is classified at its end as writing `d1/sub`, but after a later action
retargeted `L -> d2` as writing `d2/sub`, with `writes_complete` still true (a
test pins both). For a link that was already there (tracked, or left by an
earlier action) an `open` is caught late as `unstable-path` and an unknown write
set; the other calls are not. Classifying at the action's end closes both, and
host state roots as well.

**P14. The in-sandbox structure probe (stage 4 PR 2b-2c-2, design step 8).**
After `sandbox.json` is written and `Config.CheckHost` has passed, and before
any action file is written, `prepare` runs four probes in the REAL sandbox
under the real `strace`, started exactly as `kiln record step` starts a step
(`StraceArgs("/proc/self/fd/5")`, `--config-sha256`, `--stdout-fd 3 --stderr-fd
4`, the working directory `cfg.PackageDir(first action)`, the environment of
`stepProcessEnv`), but with a fixed argv and no shell:

| Name | argv | Must show |
|---|---|---|
| `pipes` | `readlink /proc/self/fd/1 /proc/self/fd/2` | exactly two lines, each `pipe:[N]` |
| `walk` | `find / ( -path /proc -o -path <each ro_root> -o -path <repo_root> ) -prune -o ( -type s -o -type p ) -print` | no output (glob characters of a root are escaped) |
| `shadow` | `find <mounts...> -xdev ( -type s -o -type p ) -print` | no output; run only when `ShadowedMounts` is not empty |
| `signals` | `sh -c 'sleep 300 & sleep 1; echo started'` | stdout `started\n`; `traced.DaemonLeft` true; finished in under 30 s |

`ShadowedMounts(cfg)` is the host's mount points (mountinfo) strictly inside a
non-symlink `ro_roots` entry, `repo_root` or a state root, sorted, keeping only
the outermost of nested ones (beneath a shadowed directory the inner mount point
may not exist in the sandbox, and the outer walk covers it; the report check
refuses a nested pair); an unreadable mountinfo, or more than 256 entries, is an
error. The `signals` run waits one
second after starting its sleeper, so the namespace's SIGKILL does not land while
the sleeper is still in `execve` or the loader (that leaves an unresumed call in
the trace and fails the probe at random). It is a
self-check of the tracer: a strace that cannot print "killed by" lines (it needs
`-e signal=all`, D19) cannot show a daemon, and `daemon-left` would never fire.

Every run is checked strictly, and any failure fails `prepare` with nothing
sealed: strace started, was not signalled, exited 0 and wrote nothing to its
stdout or stderr; the trace was not truncated (cap 64 MiB), no pipe was held
open by a survivor, and the step wrote nothing to its stderr; the trace parses in
helper mode and passes the parse half of G7 (`parseHelperTrace`, shared with
`step`: both digests are the sealed configuration's); the step's own `execve`
has exactly the probe's argv and the step exited 0; **no event of kind `write`**;
and `InitExitLine > 0`. Each probe has 60 s and all of them 10 minutes (a
deadline kills strace, and `--kill-on-exit` every tracee). After the last run,
tmpdir must be empty and every state root absent or an empty directory, or the
shard is `unscrubbed:state-preloaded`.

`pre/probes.json` is sealed, compact, with no trailing newline, and decoded
strictly (the bytes must equal what `EncodeProbes` writes for what they say):

```json
{"schema":"kiln-record-probes/1","action":"<first>","shadowed":[...],
 "runs":[{"name","argv","exit","trace_bytes","trace_sha256"}],"toolchains":[]}
```

`toolchains` (since 2b-2c-3) is the toolchain probes' results, one
`{"tool","status","detail"}` per probed tool (`detail` only on a `mismatch`),
strictly sorted by tool, with a known status (P7), and empty for a shard with no
action. `VerifyPre` decodes it and requires it to be exactly what the probe list for the sealed
configuration is (names, order, argv, exit 0, a trace size and digest), so a
dropped `pipes`, `walk` or `signals` probe is refused by every later command.
`shadowed` itself is the probe's claim (sealed, but `VerifyPre` cannot recompute
the host's mount points), so the `shadow` run is the one that can be absent
unnoticed. `<dir>/probe/<name>.trace`
(created `O_EXCL`, directory `0700`) holds the traces for whoever debugs a failed
run; the sealed digest says which file is meant. A shard with no action runs no
probe and seals the report with an empty `action` and no runs. The binding is
unchanged (D4): `SEALED` covers `probes.json`.

*The kernel may close this risk outright.* In a new user and mount namespace the
inherited mounts are locked, and a non-recursive bind (`MsBind`, which is how the
sandbox binds every root) of a directory that has a locked child mount fails
(`has_locked_children` in `fs/namespace.c`). So a host mount point inside an
`ro_roots` entry, the checkout or a state root is expected to make the sandbox
refuse to start, and `prepare` to fail on the first probe (strace exit 125, the
helper's message in the quoted step output): fail-closed, with no probe needed.
The `shadow` probe is then defence in depth. **Observed (the first
`kiln-sandbox` CI run):** the kernel refused the bind, so the sandbox failed to
start (`kiln sandbox-init: ro-root "...": invalid argument`, exit 125) and
`prepare` failed at `probe pipes`. The locked-child-mount rule therefore closes
the risk of content shadowed beneath a host mount, fail-closed: such a mount
fails every step. `TestSandboxRecordProbe` still accepts either outcome for a
FIFO hidden beneath a tmpfs. The same run showed `find /` clean inside the
sandbox, with no permission errors.

*Residue.* The probe is a sample of the host's state at `prepare`, not a lock:
a socket created later, by something outside the sandbox, in a place the sandbox
binds, is the host scan's and the runner's business (§14).

### 4.7 Sandbox decisions (stage 3b)

Stage 3b is `kiln sandbox-exec` (`internal/sandbox`). The code is
authoritative: `plan.go` for the mount order, `bpf.go` for the filter,
`privs.go` for the privilege drop. These are the decisions that §4.3 and §4.5
left open, and the places where Go's `os/exec` and the trace parser's helper
model (§4.5) forced a different shape.

**Interface.** `kiln sandbox-exec --config <abs path> [--config-sha256 <hex>]
[--stdout-fd N] [--stderr-fd N] -- <argv...>` (the optional flags are
described under "Descriptors" below; `--config-sha256` is 64 lowercase hex
digits: the root hashes the exact bytes it parses and refuses a file whose
digest differs, so a recorder that verified the file by content and handed the
helper a path cannot be fooled by a swap in between; it is optional only for
the sandbox's own tests, and `kiln record step` always passes it). The build
tool's `-exec-wrapper` is `["/abs/kiln","sandbox-exec","--config","/abs/cfg.json","--"]`
and sets `KILN_ACTION` (the package `rel_path`), `KILN_STEP` and
`KILN_STEP_COUNT`. The helper reads only `KILN_ACTION`; the other two are for
the recorder. The step never sees any of the three: its environment is the
list the configuration holds for `KILN_ACTION`, exactly, and an action the
configuration does not name is an error.

**Descriptors the caller leaves open.** Before the spawn the root marks every
descriptor from 3 up close-on-exec (`close_range`, fail closed), so the init
never holds one the caller left open (a recorder's trace pipe, say), whatever
the init's own `close_range` does; the descriptors the child needs are placed
by the spawn's explicit mapping, which clears the flag on the copy.

**The configuration** is strict JSON, written by the recorder: `repo_root`,
`home`, `tmpdir`, `ro_roots`, `state_roots`, `masks`, `env_by_action` and
`kiln_exe`. It is read with a size cap by both helper processes; the root
passes its SHA-256 to the init, which refuses a file that changed in between.
Unknown keys, differently-cased keys, duplicate keys and trailing data are
refused. Every path is absolute and clean, is not `/`, and holds no `:` (the
list separator), `,` (the mount-option separator), control character or
non-UTF-8 byte. Further rules:
- `home` and `tmpdir` are not inside `repo_root`, and differ;
- `repo_root` is not inside `tmpdir`, because `tmpdir` is where the new root
  is assembled (below), so nothing bound from the host may live under it;
- a state root overlaps neither the checkout nor `tmpdir` and does not cover
  `home`;
- `ro_roots` is not empty, has no duplicates, and no entry overlaps (equals,
  contains or lies inside) the checkout, `tmpdir` or a state root. An entry
  may lie **strictly beneath `home`** (S1, below); none equals or contains
  `home`. And no entry lies inside **or contains** `/dev`, `/run`, `/var/run`, `/proc`, `/tmp` or
  `/var/tmp`, which the sandbox builds itself. So `/var` is refused (it holds a
  real `/var/run` and `/var/tmp`), and so is `/var/lib` (service sockets): list
  specific subdirectories instead. No per-step walk for sockets is done; it
  would be too slow, and the CI test and the `kiln-record` probe cover it;
- an entry beneath `home` is allowed only if `home` does not lie strictly inside
  `/tmp`, `/var/tmp`, `tmpdir`, the checkout or a state root (its ancestors would
  be renamable, see "The ancestors of `home`" below), and it must be a real
  directory or file on the host, not a symlink;
- for an entry beneath `home`, the **anchor** (the first directory of the entry
  below `home`, which is the entry itself when it is a direct child) holds
  neither the checkout nor `tmpdir`, and no other entry lies beneath an entry
  of this kind (S1, below);
- no configured path lies inside `/dev`, `/run`, `/var/run` or `/proc`;
- **a mask is a direct child of `repo_root`**, and nothing else is a mask.
  A path outside the checkout is already absent unless it is under an allowed
  root, so a mask there (under `home`, `tmpdir`, `/run`, `/dev`) could only
  fail; the configuration refuses it with that message;
- an `env_by_action` key is a clean relative path (`.` is the checkout), and
  no entry names a `KILN_` variable;
- `kiln_exe` equals `/proc/self/exe`, and neither it nor its resolved path is
  inside `repo_root`, a state root or `tmpdir`.

Against the real filesystem the helper additionally requires that `repo_root`,
`home`, `tmpdir`, the working directory and every state root (or its nearest
existing ancestor) are canonical directories (no symlink in any component),
and that every `ro_roots` entry exists and is either a symlink (whose
**parent** must be canonical, since the link is recreated at
`<new root>/<parent>`) or a canonical directory or file,
because the mounts are made at `<new root>/<path>` and an absolute symlink in a
component would resolve against the host's root; that no mask target is a
symlink; that every mask target exists; that the configuration file does not overlap
the checkout, `tmpdir`, a state root or an `ro_roots` entry, in either direction
(not `home`: it is a fresh tmpfs the host's copy never appears in, so a
configuration beneath it that no bind exposes, such as `$RUNNER_TEMP`'s, is safe;
after the pivot a host path is visible only inside one of those, because every
bind is non-recursive and an `ro_roots` entry never is or contains `home`); and
that the working directory is exactly
`repo_root/<KILN_ACTION>`.

**Processes.** The root (`sandbox-exec`) is the process strace traces. It
starts its Go runtime threads first (a throwaway burst of goroutines blocked in
a read), then spawns exactly one child with one `clone` carrying
`CLONE_NEWUSER|CLONE_NEWNS|CLONE_NEWPID|CLONE_NEWIPC|CLONE_NEWUTS|CLONE_NEWNET`
(and a new session), the init, a re-exec of
`/proc/self/exe sandbox-init ...` with an **empty** environment, then waits.
Three things differ from the sketch in §4.3:
- **The init writes its own id maps.** Go's `SysProcAttr.UidMappings` writes
  `/proc/<pid>/uid_map` from the parent after the clone, an `openat` in the root
  after the spawn, which §4.5 forbids (`helper-activity`). A process may write
  a single-id map for its creator's effective id without privilege over the
  parent namespace, so the init does it: `setgroups` is set to `deny` first,
  then `uid_map` and `gid_map` get `1000 <caller id> 1`. The caller maps to
  uid and gid **1000** inside.
- **The init sets its own parent-death signal.** `SysProcAttr.Pdeathsig` makes
  Go's child compare `getppid()` with the parent's pid, which is 0 in a new PID
  namespace, so the child would kill itself. The init calls
  `prctl(PR_SET_PDEATHSIG, SIGKILL)`; the root's main goroutine is locked to
  the first thread, which lives as long as the root.
- **`AmbientCaps`.** The init is started with `CAP_SETPCAP` and `CAP_SYS_ADMIN`
  ambient, because the capabilities of a new user namespace do not survive the
  exec of a uid the namespace has not yet mapped.

The init runs on the thread-group leader (`runtime.LockOSThread` in an `init`
function, so `main` starts there). §4.5 asks for setup `chdir`s before any
thread starts; a Go program cannot, so the init first calls
**`unshare(CLONE_FS)`** on that thread, and the parser (`internal/trace`)
models a successful `unshare` with `CLONE_FS` in the init's window as giving
that process a private working directory. This supersedes §4.5's advice to
avoid `unshare(CLONE_FS)`. Every other `chdir` is a setup `chdir` that ends
back at the package directory.

**Mount order** (`Plan`; each step needs the ones before it). Paths in steps 2
and 3 are host paths; the host paths that are bound later (`ro_roots`,
`repo_root`, state roots) are read from the old root, and every target is placed
under the new root.
1. `mount("", "/", MS_REC|MS_PRIVATE)`: nothing propagates to the host.
2. Create each `state` root on the host.
3. Mount the staging tmpfs on the host's `tmpdir`
   (`MS_NOSUID|MS_NODEV`, `mode=0755,uid=1000,gid=1000`), create
   `tmpdir/.kiln-newroot` in it, and mount the **new root**, an empty tmpfs
   (same options), on that directory.
4. Each `ro_roots` entry, shallowest first: a symlink is recreated; a directory
   or file gets its missing parents created on the root tmpfs, is bound
   `MS_BIND` (**non-recursively**, so no mount beneath it comes along, and a
   locked one is a failure), then `mount_setattr(.., MOUNT_ATTR_RDONLY)` on
   that one mount (a plain remount would have to repeat each inherited flag,
   which a user namespace locks).
   An entry beneath `home` is **not** bound here: `home`'s tmpfs would hide it
   (step 5b).
5. Fresh tmpfs, each preceded by creating the mount point (and parents), the
   shallower first and no path twice: `home` (`0755`), `tmpdir` (`1777`),
   `/tmp` (`1777`), `/var/tmp` (`1777`) and `/run` (`0755`); all
   `MS_NOSUID|MS_NODEV`, owned by uid 1000.
5b. Each `ro_roots` entry beneath `home` (S1, below): a direct child of `home`
   is bound exactly as in step 4; a deeper one is bound inside a read-only
   tmpfs **anchor** that is mounted first and sealed last.
6. The minimal `/dev` (`MS_NOSUID|MS_NOEXEC`, `size=64k,nr_inodes=64`), with
   the six device nodes bound from the host's and the links, then remounted
   read-only.
7. Bind `repo_root` writable at the same path, then each state root.
8. Mask each configured mask (direct children of the checkout), in sorted order:
   each target is opened once with `O_PATH|O_NOFOLLOW` and mounted over through
   `/proc/self/fd/N`. A directory gets an empty read-only tmpfs
   (`MS_RDONLY|MS_NOSUID|MS_NODEV|MS_NOEXEC`, `size=4k,nr_inodes=2`); anything
   else gets `/dev/null` bound over it. A symlink and a missing target are
   refused.
9. Fresh `proc` on `/proc` (`MS_NOSUID|MS_NODEV|MS_NOEXEC`, `hidepid=2`).
10. Seal the new root read-only (`MS_REMOUNT|MS_RDONLY`), then
    `chdir(new root)`, `pivot_root(".", ".")`, `umount2(".", MNT_DETACH)`,
    `chdir("/")`, then verify that no host IPC path exists.

The scratch tmpfs precede the checkout and state roots because the checkout may
live under `$HOME` (or `/tmp`), where a tmpfs would hide a bind made earlier;
the checkout's missing parents are created in the tmpfs. A checkout under
`$HOME` therefore persists, and so do the ancestor directories the plan
creates.

**S1: tool roots beneath `home`** (`Plan`, `Config.Validate`).
`host-roots.json` declares tool roots under `$HOME` (`.cargo/bin`, `.rustup`,
`.dotnet`, `.ghcup`), and the recorder's `home` is the runner's real `$HOME`;
the sandbox mounts a fresh tmpfs over `home`, which hides anything bound there
earlier. An `ro_roots` entry strictly beneath `home` is therefore bound **after**
the scratch tmpfs (step 5b), at the same absolute path, non-recursively and
read-only exactly like any other entry (`applyRORoot`: parents created, bind,
`mount_setattr` read-only). `home` stays a writable tmpfs for the step. An
entry beneath `home` must be a **real directory or file**: a symlink is refused
(by `CheckHost` before anything is mounted, and again by `applyRORoot`),
because a recreated symlink is an ordinary inode of `home`'s tmpfs, not a mount
point, and the step could `rm` it and build a directory of its own there
(an entry outside `home`, such as `/bin`, sits on the sealed root tmpfs and may
still be a recreated symlink). The sockets-and-FIFOs rule is unchanged: the CI walk and
the `kiln-record` probe cover every `ro_roots` entry, wherever it is mounted.

*The mount point must not be swappable.* The parents of an entry under `home`
are created in `home`'s tmpfs, which the step owns, and the Linux rule that
protects a mount point is narrow: `rename`, `rmdir` and `unlink` of a directory
that **is** a mount point fail with `EBUSY`, but a directory that merely
**contains** a mount may be renamed. So for `~/.cargo/bin` the entry itself is
safe (it is a mount point), while a step could `mv ~/.cargo ~/.old` and then
build its own `~/.cargo/bin`, or move a prepared tree to `~/.cargo`, and a read
of `~/.cargo/bin/cargo` would be attributed to the declared host root while
reading the step's own file. Nothing can be mounted over or unmounted by the
step (the seccomp filter and the dropped capabilities refuse it), but that does
not stop a rename of an ancestor, so the plan removes the ability instead:

- a **direct child** of `home` (`~/.rustup`) is itself the mount point, and its
  parent is `home`, which the step may rewrite but whose entry for the mount
  cannot be renamed, replaced or removed (`EBUSY`);
- a **deeper** entry (`~/.cargo/bin`) has an **anchor**: the first directory
  below `home` on its path (`~/.cargo`). The plan mounts a fresh tmpfs on the
  anchor, creates the remaining parents and binds the entries inside it,
  creates inside it the mount points of any state root that lies beneath the
  anchor (`~/.cargo/registry` is bound writable later, in step 7, onto a
  directory that already exists), and then makes **that one mount** read-only
  with `mount_setattr` (a `KindSealDir` step). The anchor is now a mount point
  (it cannot be renamed or removed) holding only directories nobody can create,
  rename or remove an entry in, so every ancestor of every entry is fixed. A
  writable state root beneath it is itself a mount point and keeps its mount
  point;
- the checkout and `tmpdir` may not lie beneath an anchor (the anchor tmpfs
  would hide their mounts, which come earlier or later in the order), and no
  entry lies beneath another entry of this kind, so no anchor is another
  entry's mount point.

*The ancestors of `home` must be unmovable too.* Everything above is about
what lies below `home`; the directories above `home` matter as much. They are
created (`MkdirAll`) on the new root's tmpfs, which is sealed read-only before
the pivot, so a step cannot rename them, **unless** `home` lies inside a place
the step can write: a scratch tmpfs (`/tmp`, `/var/tmp`), `tmpdir`, the checkout
or a state root. With `home` at `/tmp/k/home` a step could `mv /tmp/k /tmp/.gone`
and rebuild `$HOME/.cargo/bin` itself. `Validate` therefore refuses a root
beneath `home` whenever `home` lies strictly inside any of those (and a root
inside `/tmp` or `/var/tmp` stays refused as a scratch place; there is no
exemption for roots beneath `home`). A configuration with no root beneath
`home` may keep `home` anywhere the other rules allow. The integration fixture
for S1 therefore builds its `home` under the runner's real home directory
(`/home/runner/.kiln-s1-*`), whose ancestors are on the root tmpfs, and asserts
that `mv "$(dirname "$HOME")"` fails.

Only strictly-descending entries qualify; an entry equal to `home`, or one that
contains it, is still refused, and so is one beneath `tmpdir`, the checkout or a
state root. The kiln binary and the configuration file stay out of every
writable root as before (`home` is not one of the roots `kiln_exe` is checked
against, because it is a fresh tmpfs the host's copy never appears in).

**Descriptors for the step's output (S2).** By default the step's descriptors 1
and 2 are the root's own, inherited as they are. `--stdout-fd N` and
`--stderr-fd M` make descriptors N and M of the root the step's 1 and 2 (the
two flags are given **together or not at all**, stdout first: a recorder that
names one has forgotten the other, and the remaining stream would be strace's
own; anything else is exit 125): the
root passes them to the init as its descriptors 1 and 2 (the one `clone` of the
spawn maps them; there is no `dup2` in the init, and no traced call is added to
the root), and the init's `close_range` of every descriptor from 3 up is
unchanged, so exactly 0, 1 and 2 reach the step. The recorder uses this to keep
strace's own standard error apart from the step's output, so that "strace wrote
nothing" can be checked. Each given descriptor must be 3 or more (0 to 2 are
the root's own streams, the default), open, **write-only, and a pipe**
(`O_WRONLY`; a read-write descriptor, such as a FIFO opened `O_RDWR`, could read
back what the recorder's other end holds and is refused). A terminal, a
directory, a socket, a character device, a regular file, a read end and a
read-write descriptor are refused with exit status 125 before anything is
spawned. The pipe-only rule follows D15 (the step's 1 and 2 are pipes): a
regular file would let a step seek and rewrite what it already wrote (and, in
the checkout, would be a file the recorder then hashes), and a tty would give it
a controlling terminal; a pipe is append-only and bounded by the reader. The
same descriptor may be given for both. A diagnostic of the **init** (exit 125
after the root started it) is written to the init's standard error, which is
then the step's stderr pipe; the root's own diagnostics keep going to the root's
standard error.

**Exit 125 is a failure, never a retry.** `125` means the sandbox could not be
built or refused its configuration or arguments; the recorder treats it as a
failure of the action and **never** re-runs the step unsandboxed or with a
weaker configuration. A step may also exit 125 by itself; the recorder tells
the two apart by whether the step's own `execve` appears in the trace (a
sandbox that refused has no step exec, which the parser refuses, §4.5), not by
the status.

**The strace argv (S3).** `sandbox.StraceArgs(traceFile)` returns the argv of
§4.3 up to and including `-o <traceFile> --`, so a caller appends the command to
run. `traceFile` must be an absolute, clean path (it is the only variable part).
The trace set is the three classes plus an explicit list that names every call
the filter refuses, the thirty of §4.4 and `clone3`; unit tests check that the
explicit names are exactly the parser's table's refused class and the filter's
refused list, and that every name the set spells out is known to the parser's
table. `TestSandboxRealStrace` runs the real strace with this argv.

**Privilege drop** (`PrivSteps`, then checked by reading the state back):
1. `prctl(PR_SET_NO_NEW_PRIVS, 1)`;
2. `prctl(PR_CAPBSET_DROP, c)` for `c` from 0, until `EINVAL` (at most 64
   capabilities); it needs `CAP_SETPCAP`, so it comes before step 4;
3. `prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_CLEAR_ALL)`;
4. `capset` (version 3) with empty effective, permitted and inheritable sets.

The init starts with `CAP_SETPCAP`, `CAP_NET_ADMIN` and `CAP_SYS_ADMIN`
ambient.

The init then verifies `no_new_privs`, an empty bounding set and empty `capget`
sets, and refuses to continue otherwise.

**Seccomp filter** (`BuildFilter`): a hand-built classic BPF program, installed
with `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)` on the exec thread, so the
exec carries it. Its layout:

| # | Instruction | Verdict |
|---|---|---|
| 0 | `ld arch` | |
| 1 | `jeq AUDIT_ARCH_X86_64` | else `RET KILL_PROCESS` |
| 2 | `ld nr` | |
| 3 | `jset __X32_SYSCALL_BIT` | then `RET KILL_PROCESS` |
| 4 | `jgt MaxKnownSyscall` (461, unsigned) | then `RET ERRNO\|ENOSYS` |
| 5.. | `jeq <nr>` for each refused call, in the order of `RefusedSyscalls` | then `RET ERRNO\|EPERM` |
| | `jeq clone` (56) | then the clone check |
| | `jeq clone3` (435) | then `RET ERRNO\|ENOSYS` |
| | `RET ALLOW` | |
| | clone check: `ld args[0]` (low 32 bits); `jset` any of `CLONE_UNTRACED`, `CLONE_NEWNS`, `CLONE_NEWCGROUP`, `CLONE_NEWUTS`, `CLONE_NEWIPC`, `CLONE_NEWUSER`, `CLONE_NEWPID`, `CLONE_NEWNET` | then `RET ERRNO\|EPERM`; else `RET ALLOW` |
| | `RET ERRNO\|EPERM`, `RET ERRNO\|ENOSYS`, `RET KILL_PROCESS` | the targets above |

**Calls newer than the pinned strace (L7).** A system call the pinned strace
does not know can be neither printed by `-e trace=` nor refused by the parser,
so a path-taking one would leave no line in the trace: `getxattrat`,
`listxattrat`, `setxattrat`, `removexattrat` (Linux 6.13, numbers 463 to 466),
`open_tree_attr` (6.15, 467), `file_getattr` and `file_setattr` (6.17, 468 and
469). The filter therefore answers **`ENOSYS` for every number above
`MaxKnownSyscall` = 461** (`lsm_list_modules`, the highest of Linux 6.8's x86-64
table, which strace 6.8 on the Ubuntu 24.04 runner image knows), placed right
after the x32 check so the ABI tests still come first. The threshold is 461
because nothing the Go runtime, `sh`, coreutils or the toolchains use today is
above it: glibc 2.39's newest calls are `fchmodat2` (452) and `clone3` (435, which
the filter already answers with `ENOSYS`), and the next number, 462, is `mseal`,
which none of them call. `ENOSYS` rather than `EPERM` also lets a libc that
probes for a new call fall back to the older one. The seven names above are all
beyond the threshold, so none needs an explicit rule (a unit test runs the
program over each number). When the pinned strace is bumped, **raise the
threshold** to the highest number its table knows, after adding the calls it
newly knows to the parser's table (§14).

The refused calls are the thirty §4.4 names, the first twenty being: `io_uring_setup`,
`open_by_handle_at`, `name_to_handle_at`, `mount`, `umount2`, `fsopen`,
`fsmount`, `fsconfig`, `fspick`, `open_tree`, `move_mount`, `mount_setattr`,
`pivot_root`, `chroot`, `unshare`, `setns`, `ptrace`, `process_vm_readv`,
`pidfd_getfd`, `bpf`; and, added in review, `io_uring_enter`,
`io_uring_register`, `process_vm_writev`, `userfaultfd`, `kexec_load`,
`kexec_file_load`, `perf_event_open`, `add_key`, `request_key`, `keyctl`.
On x86-64 `clone`'s flags are its **first** argument. The
filter adds `CLONE_NEWUTS` to §4.3's list, matching §4.4's "any `CLONE_NEW*`".
A change to the filter bumps `kiln:recorder`.

**Descriptors, standard input, exec.** After the filter, the init opens
`/dev/null` as descriptor 0 and marks every descriptor from 3 up
close-on-exec (`close_range(3, ~0, CLOSE_RANGE_CLOEXEC)`, or `fcntl` one by one
on a kernel without it). It does not close them outright: the Go runtime's
own epoll descriptor may still be used by its monitor thread until the exec,
and closing it is a fatal runtime error. The effect on the step is the same.
It `chdir`s to the package directory and `execve`s the step from the leader.
An `argv[0]` with no `/` is looked up in the **configured** `PATH` (absolute
entries only), one real `execve` per candidate, so the failed probes appear in
the trace as the parser expects (§4.5).

**Review decisions (second pass).** An adversarial review of the first
version of this section led to these. Each is a decision, not a sketch.

- **The step cannot signal outside.** The init is started with `Setsid`, so the
  step has its own session and process group (and a PID namespace of its own).
  `kill(0, sig)` and `kill(-1, sig)` reach only the step's own tree. Before
  this, the step shared strace's process group, and `kill(0, SIGUSR1)` killed
  strace, which detaches its tracees. The recorder also runs strace with
  `--kill-on-exit` (§4.3), and **a strace that exits by a signal or with a
  non-zero status makes the action's write set unknown, which voids the
  whole run** (§7: the earlier wording voided only the shard, see §7.1, G1).
- **No host IPC.** The step keeps the runner's supplementary groups (`setgroups`
  is `deny`), so without isolation it could `connect()` to `/run/docker.sock`,
  which is host root. The one `clone` therefore adds `CLONE_NEWIPC`,
  `CLONE_NEWUTS` and `CLONE_NEWNET`. The network namespace is empty except for
  `lo`, which the init brings up (`SIOCSIFFLAGS` on an `AF_INET` datagram
  socket, with `CAP_NET_ADMIN` as one more ambient capability): a test that
  binds and connects on `127.0.0.1` inside the step still works, and nothing
  reaches the host's loopback services or abstract sockets (those are per
  network namespace). Namespaces do not cover the filesystem, though: see the
  next bullet. `/run` is a fresh tmpfs; after the pivot `/run/docker.sock`,
  `/var/run/docker.sock`, `/run/user`, `/run/dbus` and `/var/run/dbus` must be
  absent. `/dev` is replaced by a tmpfs holding
  only `null`, `zero`, `full`, `random`, `urandom` and `tty` (bound from the
  host), and the symlinks `fd`, `stdin`, `stdout` and `stderr` (remounted
  read-only): no `/dev/kvm`, `/dev/fuse`, disks or `/dev/pts`. No configured path
  may lie under `/dev`, `/run`, `/var/run` or `/proc`. `RLIMIT_CORE` is 1: a limit of 0 would not stop a piped
  `core_pattern` handler (a pipe ignores the limit), but the kernel skips a
  piped dump when the limit is exactly 1 (soft and hard, so the step cannot raise
  it). Stage 5 revisits the network for fetchers (§14), probably through a proxy.
- **An allowlist root, not a masked copy.** A pathname Unix socket or FIFO
  belongs to the filesystem, and a read-only bind does not stop `connect()` or an
  open for writing. The runner's .NET diagnostic socket,
  `/tmp/dotnet-diagnostic-<pid>-...-socket`, lets a step make `Runner.Worker`
  load a library: code execution outside the sandbox. So the root is an empty
  tmpfs holding only the `ro_roots`, the checkout, the state roots and fresh
  scratch tmpfs (steps 3 to 7 above), and masks apply only inside the checkout.
  Test `TestSandboxNoReachableSocketsOrFifos` walks every mount of a running
  sandbox **except the `ro_roots`**, finds no socket or FIFO, and checks that
  ones the test makes in the host's `/tmp` and `/var/tmp` are not visible: it
  checks the sandbox's own structure and is fast. **The read-only roots are
  scanned on the host** (`/usr` on a runner image has millions of files, too
  slow to walk inside every sandbox) by `TestSandboxRORootsHaveNoSocketsOrFifos`,
  with `find <root> -xdev ( -type s -o -type p ) -print -quit`, one walk per root
  in parallel.
- **One spawn in the root.** `os.StartProcess` probes pidfd support with a
  throwaway `clone` (`os/pidfd_linux.go`), a second non-thread spawn in the
  root, which §4.5 refuses (`helper-spawn`). The root uses `syscall.ForkExec`
  and `syscall.Wait4` with no pidfd. Nothing else in the root clones a process
  before the spawn; its threads are `CLONE_THREAD` clones.
- **Masks cannot be renamed around.** A mask is a **direct child** of
  `repo_root` (a step may rename an ancestor of a mount point, not a mount
  point), and a mask anywhere else is refused (it is already absent). Every
  mask must exist. The mask is opened
  once with `O_PATH|O_NOFOLLOW` and mounted through `/proc/self/fd/N`, so
  what was inspected (a directory or a file, never a symlink) is what is
  mounted over, with no look-then-mount race.
- **Canonical, resolved paths.** `kiln_exe` and the configuration path are
  resolved with `EvalSymlinks` and must equal their own configured or given
  spelling. Neither, and not the resolved binary, may lie inside the checkout,
  a state root or `tmpdir`.
- **`close_range` fallback.** Only `ENOSYS` or `EINVAL` falls back, to walking
  `/proc/self/fd`; any other error is a failure. **The recorder must make the
  step's descriptors 1 and 2 pipes** (not files in the checkout or a terminal):
  they are inherited as they are.
- **More refused calls.** The filter also refuses the ten calls added to §4.4,
  and the parser's refused class lists them.
- **Liveness pipe.** The root holds the write end of a pipe and the init the
  read end (descriptor 3). After `PR_SET_PDEATHSIG` the init reads it without
  blocking and exits 125 if it sees end of file: a root that died before the
  signal was set left nobody to signal the init. Teardown does not rest on the
  parent-death signal alone.
- **`--pidns-translation` is required.** The recorder and the real-strace test
  run strace with it, and the parser refuses an untranslated spawn inside the
  namespace (§4.5), so a missing flag fails closed. `TestSandboxStraceTraceSetNamesAreValid` checks the runner's strace accepts it.
- **`clone3` in the helper's groups.** The Go runtime makes its threads with
  `clone3`, which strace 6.8 prints as a struct with an output annotation:
  `clone3({flags=A|B, child_tid=0x.., ..., tls=0x..} => {parent_tid=[N]}, 88) = N`
  (inline, or split into `<unfinished ...>` and `<... clone3 resumed> => {...},
  88) = N`). The parser decodes the struct's leading `flags=` field and records
  the returned pid as a spawn exactly as for `clone`, so a thread of the root
  group is a root-group thread and a thread of the init window shares its cwd.
  A struct it cannot decode exactly (not a struct, no leading `flags=`, a
  truncated one with `...`, numeric flags, a missing size argument, trailing
  text) is `clone-flags`. The policy is unchanged: in the root group and the
  init window a `clone3` is tolerated (threads only; a second process in the
  init window is `helper-spawn`, and `CLONE_NEW*` or `CLONE_UNTRACED` is
  `clone-flags`); anywhere else, the step's own included, it is
  `refused:clone3`, because the sandbox filter answers `ENOSYS` and a `clone3`
  that did not fail with `ENOSYS` means the filter failed. A `clone3 = -1
  ENOSYS` (what glibc's `pthread_create` and `posix_spawn` try first before
  falling back to `clone`) is the filter working: it is tolerated everywhere,
  its struct is not decoded, and it spawns nothing. `RecorderVersion` is 4 since
  this rule.
- **No late runtime threads.** Both helpers call `debug.SetGCPercent(-1)`, so
  no garbage-collector worker thread starts after the spawn.
- **CI.** `ci.yml` has a required `kiln-sandbox` job (linux/amd64,
  a matrix over `ubuntu-24.04` and `ubuntu-26.04` (D27), in `ci-gate`) that lifts the AppArmor restriction on
  unprivileged user namespaces unconditionally, installs `strace` through
  `apt-install.sh`, and runs the sandbox tests with `KILN_REQUIRE_SANDBOX=1`,
  which turns every skip into a failure. A script then asserts from the
  `go test -json` stream that every named test passed and none skipped. One of
  them runs the real `strace -f --quiet=attach -y -s 4096 --kill-on-exit`
  around `kiln sandbox-exec` (with the argv `sandbox.StraceArgs` returns) and
  parses the trace in helper mode, which pins the single-spawn root, the init's
  tolerated setup and strace's `-y` escaping. `TestSandboxROBeneathHome` (S1:
  reads, no writes, no renames of a root or its anchor, a writable state root
  beneath the anchor) and `TestSandboxOutputDescriptors` (S2: output in the two
  pipes, the root's own streams empty, refusals exit 125) are in the same set.

**Exit status.** The root exits with the init's status (128 plus the signal if
it was killed). A failure before the step runs exits 125, and a failed exec 127.
Such a trace has no step exec, which the parser refuses.

**Platforms.** linux/amd64 only (`sandbox_linux_amd64.go`). Every other
platform gets a stub that prints why and exits 125. The integration tests need
unprivileged user namespaces and skip where the kernel refuses them.

### 4.9 Syntax layer: grammar-driven (D24)

Stage 1a settled what the parser accepts and refuses. This section settles
*how the text is read*. It changes the machinery under `internal/trace`, not
its behaviour.

**The rule.** The syntax of a strace line is defined by grammar files, not by
hand-written scanning code:

- `code/grammars/strace/strace.tokens` and `strace.grammar` define tokens and
  structure;
- `code/grammars/strace/strace-string.tokens` defines the escapes inside a
  quoted string;
- the shared Go grammar lexer and parser (`code/packages/go/lexer`,
  `code/packages/go/parser`) read them, through two shared packages,
  `code/packages/go/strace-lexer` and `code/packages/go/strace-parser`
  (a line in, a syntax tree out, syntax only; the pattern is
  `json-lexer`/`json-parser`).

Kiln keeps all *policy*: the pending/resume state machine, the D18 rules,
exit markers, refusal reasons, limits, and the byte pre-checks (the
control-character alphabet scan and the `[ Process PID=` prefix check, which
are validation, not lexing). Kiln has **no hand-crafted lexers or parsers for
its own inputs**. This is a user directive (2026-11-16). It applies to KILN01
as well, and to anything Kiln reads later.

One consequence is recorded in KILN00 §17: Kiln may depend on in-repo Go
modules (wired with `replace`), and still never on third-party modules.

**Fail-closed parsing.** The shared parser does no error recovery: the first
line that does not parse is a refusal of the whole trace, never a partial
tree. It has a nesting depth limit and returns an error instead of
overflowing the stack. The failing token maps to the reason the old scanner
gave:

| Failure | Reason |
|---|---|
| a bad `<...>` annotation (`ERR_ANNOT_BAD`), or an annotation that fails strict decoding | `ambiguous-path` |
| an unterminated annotation, quote or `<`, a bad resume marker, or any structural failure | `unparsable-line` |
| a failed `result` parse (stage 2) | `result` |
| a wrong argument count after a successful parse | `arity` |
| a recovered panic: the **whole trace** is refused; no partial tree or partial trace is ever returned | `unparsable-line` |

Everything after the first pass runs the code it ran before.

Two caps are new, because the grammar tools let them be stated: nesting depth
**64** and **65,536 tokens per line**. Exceeding either is a refusal with
reason `size`, which is already in the reason list of §4.5 (the caps reuse it).
A recovered panic anywhere in the grammar path likewise refuses the whole trace. The lexer is total (the last token rules catch every remaining
byte as an `ERR_*` token), so it can never panic on hostile input.

**The divergence policy.** The old scanner and the grammar path must agree.

1. Where the old parser refused, the new one accepting is **always a
   failure**.
2. When both accept, the `Trace` must be identical.
3. `Reason` and `Line` must be exact on every existing test and fixture.
4. A divergence found by fuzzing is allowed only if it is *stricter-only*
   (accept to refuse, or refuse to refuse with a different reason), is listed
   in the table below, has been reviewed, and is pinned by a test.

| Input class | Old | New | Reason | Pinning test |
|---|---|---|---|---|
| a line nested deeper than 64 | accepted | refused, `size` | bounds stack and memory against hostile lines | `TestDepthCap` |
| a line of more than 65,536 tokens (a signal line counts too: every run of spaces in its text is a token) | accepted | refused, `size` | bounds memory and time against hostile lines | `TestTokenCap` |
| `[pid +5] ...` (a signed pid: `Atoi` accepts a sign) | accepted | refused, `unparsable-line` (`ERR_PID`) | the pid is 1 to 9 digits, no sign, no leading zero | `TestReviewDivergences` (strace-parser) |
| a resumed half that is only valid glued to its prefix, such as `<... f resumed></x>) = 0` after a prefix ending in `3` (the old parser read `3</x>`), or a lone `"` or `<` at the seam | accepted | refused, `unparsable-line` (`ERR_LT`, `ERR_QUOTE`) | the resumed half is lexed on its own, a line at a time, so a construct may not span the seam; strace never splits a token | `TestReviewDivergences` |
| a quoted string with a bad escape (`\q`, `\400`, `\x4`) in a call the old parser does not interpret | accepted (scanned leniently, decoded only in path arguments) | refused, `ambiguous-path` | every quoted string is decoded strictly (`Options.LenientQuoted` turns it off) | `TestStrictQuoted` |
| a line of more than 1 MiB | refused, `size` | refused, `size` (checked before lexing) | none: the same limit, recorded because the grammar path enforces it itself | `TestMaxLineBytes` |
| a bare `<unfinished ...>` after a call's closing `)`, such as `stat(a) = 0 <unfinished ...>`, `stat(a) <unfinished ...>`, `stat(a)<unfinished ...>`, `stat(a) = ? <unfinished ...>` | refused, `result` (or `ambiguous-path` when the marker follows an annotation) | refused, `unparsable-line` | no result can contain a bare marker, so the grammar refuses it; reason-only (`= 0<unfinished ...>`, one annotated token, is still accepted by both) | `TestMarkerAfterCloseRefused` (strace-parser) |
| a call line with trailing spaces (reported ahead of any other fault the line has) | accepted, or refused for another reason | refused, `unparsable-line` | the lexer skips spaces, so the grammar cannot see a trailing one; refusing is the safe side | `TestStricterOnlyDivergences` (strace-parser) |
| `+++ exited with +5 +++`, `-0` (a signed exit code) | accepted (`Atoi` takes a sign) | refused, `unparsable-line` | the exit code is digits only; no real strace prints a sign | `TestStricterOnlyDivergences` |
| an `ERR_*` token in the text after a call's `)` (an unterminated quote, a bad `<`, a resume marker) | refused, `result` or `ambiguous-path` | refused, `unparsable-line` (or `ambiguous-path` for a bad annotation) | the lexer cannot know a quote is "in the result"; both refuse the whole trace | `TestStricterOnlyDivergences` |
| `= ? <unavailable>` after a call, `<... NAME resumed> <unfinished ...>) = ?`, and an unfinished ignored-class call, `fork`, `vfork` or `clone` with allowed flags followed by the pid's exit (D35, the dying-call rule) | refused, `unresumed-call` or `result` | accepted, the call is dropped | none: the grammar has `UNAVAILABLE` and `RESUMED inner UNFINISHED RPAREN tail`, and the readers share `parseResult`, `dyingResumeTail`, `dyingPolicy` and `exitInsideCall`; the `ERR token` row above is not widened (`<unavailable>` is no `ERR_LT`). Both readers do the same, so this is not a divergence between them; the row records the grammar change | `TestDyingCallShapesAreAccepted`, `TestDyingCallRefused`, `TestDyingLineSeedsAgree` |
| a `<... NAME resumed>` whose NAME has a character outside `[A-Za-z0-9_?]` | refused, `unmatched-resume` | refused, `unparsable-line` | NAME is lexed as a name; anything else is a bad marker | `TestRefusals` (`ERR_RESUME`) |
| a split call whose glued text (prefix plus resumed half) is over the line byte limit, though each half is under it | accepted | refused, `size` | the grammar path reads the glued call as one line, so the limit applies to the call; a same-pid signal line between the halves does not break the glue, so the row applies across it too | `TestGluedCallOverTheByteLimit`, `TestGluedCallOverTheByteLimitAcrossSignal` |
| `= 5 /* 9 in strace's PID NS */<x>` (an annotation, or a marker, after the pid-namespace note with no word before the `<`) | accepted | refused, `unparsable-line` (`ERR_LT`) | strace prints the annotation first; a bare `<` is an `ERR_LT` | `TestPR5Divergences` |
| an exit line holding a non-ASCII Unicode space (`exited with 0` followed by U+00A0) | accepted (`strings.TrimSpace` removes it) | refused, `unparsable-line` | the exit token is ASCII only | `TestPR5Divergences` |
| a `<... NAME resumed>` line that is itself malformed AND has no matching pending call | refused, `unmatched-resume` | refused, `unparsable-line` (or `ambiguous-path`) | the grammar path reads a line before it looks at the pending call | `TestPR5Divergences` |
| an unfinished `execve(` from a pid that then exits with code 0 right after it, while an earlier pid's byte-identical `execve` is pending and later resumes `= 0` in the same thread group (D33) | accepted, no event for the ghost line | accepted, no event for the ghost line | none: both readers call the same helper, `exitInsideCall`; every other shape stays `unresumed-call` | `TestPhantomExecAccepted`, `TestPhantomExecRefused` |
| a line whose syntax is bad AND whose effect the old scanner refused first: `interleaved-call` (a call while another is pending), or the `unresumed-call` an exit line raises | refused, `interleaved-call` / `unresumed-call` | refused, `unparsable-line` or `ambiguous-path` (same line) | the old scanner checks a line's syntax before it looks at the pending calls, so these reasons mean it accepted the syntax; the grammar path refuses the syntax first | `TestSeqDiffSeeded` (reason-only; the old scanner always refused too) |
| a call with more top-level items than its arity cap AND a later syntax defect (for a split call, the cap is applied to the glued text; a same-pid signal line `--- SIG ... ---` between the halves does not break the glue, an exit line does) | refused, `arity` (stopped while scanning) | refused, `unparsable-line` or `ambiguous-path` | the grammar parses the whole line before counting | `TestLineDiffSeeded` (row "too many arguments and a later error"), `TestSignalBetweenSplitCallHalves` |
| a line with a structural error AND a later bad annotation | refused, `unparsable-line` (first problem wins) | refused, `ambiguous-path` | the parser reports a failure position only as the start of the failing rule, so a syntax error cannot be ordered against a later bad token; the bad token is reported | `TestBadTokenWinsOverEarlierSyntaxError` |

**Grammar details recorded by PR 4.** Nesting depth 64 means 64 levels of
brackets or nested calls (`parser.Options.MaxDepth` is in rule levels, 2 per
bracket level plus 8, and a test pins that exactly 64 levels parse and 65 do
not). Tokens that only begin a line's body (`+++ ... +++`, `<... NAME resumed>`,
`???( <unfinished ...>`, `???()  = ?`, `<... ??? resumed>)  = ?`, `--- `) are lexed only right after the pid, and the three `???` lines are whole-line tokens in exactly the spellings the old parser accepted (`???() = 0` and every other spelling is refused), so the same characters inside
an argument list stay plain text, as they were for the scanner. The text of a
`--- ... ---` signal line is lexed as space runs and non-space runs and is never
tokenized, so a lone quote or bracket in it is still accepted. Spaces between
tokens are skipped; `name(`, `word<...>`, the pid prefix and the exit line are
single tokens so adjacency is part of the token. Leading and trailing spaces and
newlines are refused by the `strace-parser` package.

**Migration and gates.** The change lands in two phases.

- **Phase A** parses structure only. The pending/resume machine, D18 and the
  interpreters (`pathArg`, `parseFD`, `parseArgv`, `parseResult`, sockaddr,
  `clone3`) are unchanged and read raw argument spans that are byte-identical
  to the old scanner's.
- **Phase B** moves the interpreters, one per PR, to consume the tree, and
  moves escape decoding to `strace-string.tokens`.

Old and new run side by side under a differential `crossCheck` hook, so every
existing test runs both parsers and compares the `Trace`, `Reason` and `Line`.
Beyond that: a pass-1 record diff, `FuzzParseDiff` and `FuzzLineDiff` with a
committed corpus (at least 30 minutes locally per PR, nightly in CI), and
`KILN_TRACE_DIFF=1` in the kiln-sandbox job, which cross-checks real traces and
panics on a mismatch after printing a bounded, sanitised window of the lines
around it to the job log. No artifact is uploaded (the traces hold paths a pull
request chose, so the only thing kept is that window, in the log).

**The refusal diagnostic (D32).** A kiln-sandbox run once refused a real trace
("probe walk: the trace does not parse: trace-incomplete:unparsable-line") and the
trace was deleted with the test's temp dir, so nobody could say which line was
refused or whether the old scanner would have refused it too (the children that
run `prepare` have hand-built environments without `KILN_TRACE_DIFF`, so their
parse was grammar-only). Two small changes, no artifact:

- `prepare`'s refusal now ends with the refused line, `the trace does not parse:
  <reason> (line N)`, appended to the DISPLAY text only. The reason itself is
  unchanged (it feeds the sidecar's `parse_reasons`), and a line number carries
  no path text. (`step` records only the reason, as before.)
- `trace.Diagnose` (measure.go, a test/CI instrument kept out of production code
  like `Measure`) runs pass 1 with both readers and reports each one's reason,
  error line and stop line, the `grammarPanics` delta, the line count and whether
  the data ends in a newline; with `KILN_TRACE_DIFF=1` it adds a sanitised window
  of three lines either side of the earlier stop line, the stop line's pid's
  last earlier line (a resume's pending half can be far above) and a reader's own
  Error line when it differs from its stop line (an unresumed call at the end),
  under the same quoting as the cross-check's window. Those key lines show their
  first AND last 160 bytes (the refused shape `... <unfinished ...>) = ?` sits at
  the end of long lines) and are printed before the window; neighbours keep only
  their head. The report says it is pass 1 only (a trace both readers accept can
  still be refused by pass 2, the model) and is at most 8 KB. The sandbox tests'
  shared `recStepRun` helper calls it, in the test process (which has the CI
  environment), when a child's stderr says `the trace does not parse` and names
  `(trace: <path>)` (an absolute path under the temp dir, opened once with
  `O_NOFOLLOW|O_NONBLOCK`, regular files only, read back at most 8 MiB with a
  note when cut), and logs it before the test fails. A step's or the epilogue's
  refusal is in the sidecar's `parse_reasons`, not on stderr, so the step,
  collect and gates sandbox tests call the same diagnostic on the action's step
  traces (`record.StepFile`) when they see a parse reason or an unexpected
  unknown write.

**The phantom exec (D33).** `kiln record prepare`'s "signals" probe trace was
refused `unresumed-call` by both readers: strace printed a second, byte-identical
`execve(... <unfinished ...>` under a Go runtime thread that `de_thread` had
just killed, followed by that thread's exit (section 4.5, "A phantom `execve`").
Decision: a narrow parser rule rather than a kiln change, because any
multithreaded program execing has the race. It needs a matching pending
`execve` of the same text and an earlier line, the exit directly after it with
code 0, an absolute path, the owner resuming `= 0`, and (pass 2) the same thread
group; every miss is the old `unresumed-call`. The captured trace is a test,
and both fuzzers seed it.

**Cross-checking the traces the children wrote (D34).** The soak's coverage of
real recordings was thin: `KILN_TRACE_DIFF=1` is set for the `go test` process,
but the real parses happen in CHILD processes (`kiln record prepare`'s probes,
`step`, the classify child, `collect`) whose environments are hand-built. The
variable is deliberately NOT passed to them: `recordcmd.go` seals the environment
into facts and keys, so a debugging switch could leak into sealed data. Instead
the kiln-sandbox tests that run record children (step, collect, gates, survivor
and the probe tests) register a `t.Cleanup` that reads back EVERY `*.trace` under
the record directory (`probe/*.trace` and `record.StepFile(dir, action, i,
"trace")`) and parses it in the test process with `trace.ParseCrossChecked`
(`internal/trace`, a test instrument kept out of production code by
`TestMeasureIsNotUsedByProductionCode`), using the Limits, helper-root setting,
helper executable and working directory the child used (recovered from the sealed
`pre/`). It returns a disagreement as a value instead of a panic. The test FAILS
on any mismatch (a fail-open or a divergence outside the list) and on any ALLOWED
stricter-only divergence: on a real trace the rule is that there are none. The
cleanup runs after the children finish and before the temp dir is deleted, once
per record directory, and only when the test process has `KILN_TRACE_DIFF=1`. It
logs `PROBE: crosscheck traces=N accepted=K bytes=B mismatches=M allowed=A` (all
counts, no text; with the D32 `Diagnose` report, bounded and sanitised, for any
failing trace). `accepted` counts the traces the parse accepted: two readers
that refuse every trace alike "agree", so a check whose parameters drifted (a
wrong helper executable or working directory) would otherwise read
`mismatches=0` while checking nothing. For traces under a VERIFIED `pre/` the
test also compares its verdict with the child's: a probe run listed in the
sealed `probes.json`, and every step trace of an action whose `record.json` says
`parse_ok`, must be accepted here too, and a trace the verified `pre/` does not
account for fails. The steps come from the directory listing, with no fixed
range; a directory-walk error is a failure. The check is registered BEFORE
`prepare` runs, so a probe trace that prepare refuses (the D32 incident) is
still read back; without a verified `pre/` the helper is the test's own kiln
binary. `ci/assert_sandbox_tests.py` requires, over the passing tests' lines,
`traces >= 1` and `accepted >= 1` in total, and `mismatches=0 allowed=0` on
every line, so the check cannot go vacuous. The parse reads a process-global
allowed-divergence counter, so these tests must not use `t.Parallel`. The
script's own tests (`code/scripts/tests/test_assert_kiln_sandbox_tests.py`) run
in the kiln-sandbox job.

**The default-flip gates (restated by PR 6).** The first draft gated the flip on
"at least 8 MB/s and within 3x of the old parser". That gate is replaced: a ratio
to the old scanner measures how fast the old scanner is, not whether the grammar
path is fast enough, and the old scanner is deleted in PR 8. The new gates are
tied to the deadline the recorder already enforces: an absolute floor, and a
capability floor that is a ratio to a FROZEN reference kernel. A frozen kernel
is a different kind of denominator from the old scanner: it never changes (so
the ratio moves only when the reader or the host does), and it is not a
competing implementation, so it survives PR 8.

- **G1 (blocking), in two parts.** The grammar path's **pass 1** (the line
  reader, `readRecordsAt`) must be fast enough to get a cap-sized action
  (`DefaultMaxTraceTotal`, 2 GiB) through well inside the classify deadline
  (`DefaultClassifyTimeout`, 10 minutes).
  (a) **A host-normalised capability gate** on the deterministic synthetic
  slice: the reader's rate, rescaled to a nominal host, must be at least
  **14 MB/s** (2048 MiB / 150 s, a quarter of the deadline, is 13.7 MB/s,
  rounded up to 14). It answers "did the reader get slower?" without being
  fooled by a slower runner.
  (b) **An absolute safety floor** of **8 MB/s raw**, on every gated line (the
  slice and each real trace of 32 to 128 KB). It answers "could a cap-sized
  action still finish?" whatever the host: the hard limit is the 600 s
  deadline, which is 2147 MB / 600 s = 3.6 MB/s, and 8 is more than twice that.

  *Why two parts.* GitHub's runner fleet varies about **1.75x** in CPU speed. The
  slice (best-of-7, 64 KB) read, over ten kiln-sandbox legs, 21.6, 18.7, 15.9,
  14.3, 19.2, 19.6, 19.4, 15.6, 20.2 and 12.3 MB/s: a raw 14 fails the slow
  hosts for reasons that are not the reader's. On the slowest host seen (the
  census read 10.8 MB/s there) a 2 GiB action takes about **199 s**, 33% of the
  deadline. That is a fact about the host that no code gate can change; the
  gate's job is to catch the reader getting slower, and to refuse a host on
  which a cap-sized action could not finish.

  *The normalisation.* Each synthetic measurement also times a **reference
  kernel** (`refKernel`, `internal/trace/measure.go`) in the same loop, and

      norm_mb_s = best_mb_s * refNominalMBs / ref_mb_s

  is the rate the reader would have on the nominal host (`refNominalMBs` = 72.0,
  the kernel's rate there). The kernel is **parser-shaped, allocating and
  FROZEN**: it splits the input on newlines; per line it makes a token slice of
  capacity 4 and appends heap token nodes (so the slice grows and the allocator
  runs); it finds token boundaries with a byte-class table and dispatches with a
  `switch` on the class (different work per arm); it copies each token to a
  string and looks it up in a small prefilled `map[string]int`; it keeps the last
  64 lines live in a ring; and it hashes once per token, never per byte (no
  serial multiply chain). It is NOT allocation-free: it allocates about 28 bytes
  per input byte (the grammar path about 46) and runs at about 72 MB/s, 3 to 4
  times the reader. One reference sample is 2 passes over the slice. A first
  candidate, a byte-at-a-time state machine with a serial FNV multiply chain
  ("kernel A"), was rejected: it is a clock-frequency meter, blind to branch
  prediction, out-of-order width, caches and the allocator, and its cross-leg
  spread was 1.39. `BestPass1Calibrated` times one reference sample and then one
  grammar pass in each iteration of the usual loop (the refusal-check pass
  doubles as the grammar warm-up, one warm-up sample of the kernel, collector
  off and restored for a small trace, at least 7 iterations spanning at least
  250 ms, at most 200) and keeps the minimum of each. `ref_mb_s = 2 * len /
  refBest`. The kernel, its tables and dictionary, its pass count and
  `refNominalMBs` are frozen together.

  *Calibration (done, 8 CI legs).* On each leg
  `k = ref_mb_s * 19.5 / best_mb_s` (the reference rate the leg's host would
  show if the slice read the nominal 19.5 MB/s there), and
  `norm_mb_s = best_mb_s * 72.0 / ref_mb_s`:

  | CPU | best | ref | k | norm |
  |---|---|---|---|---|
  | Xeon 6975P-C | 19.3 | 71.4 | 72.0 | 19.5 |
  | EPYC 7763 | 12.5 | 45.9 | 71.7 | 19.6 |
  | Xeon 6975P-C | 19.4 | 69.7 | 70.1 | 20.0 |
  | EPYC 9V74 | 16.8 | 65.4 | 75.9 | 18.5 |
  | EPYC 9V45 | 20.8 | 85.0 | 79.8 | 17.6 |
  | EPYC 9V74 | 16.7 | 66.2 | 77.5 | 18.2 |
  | Xeon Platinum 8370C | 13.6 | 47.9 | 68.9 | 20.4 |
  | Xeon Platinum 8573C | 17.4 | 59.4 | 66.5 | 21.1 |

  The median k is 71.85, rounded to **`refNominalMBs = 72.0`**.
  *Validity rule:* the within-leg k spread (rounds on one host) is at most 1.05,
  and the worst-leg norm is at least 1.2 x the 14 floor (16.8) with a slow host
  (raw slice at most 16) included. Observed: the worst norm is **17.6** (the
  Zen 5 EPYC 9V45, a 1.26 margin over 14), and the slow legs (EPYC 7763 at 12.5,
  Xeon 8370C at 13.6) normalise to 19.6 and 20.4. The cross-leg spread of k is
  **1.20** (66.5 to 79.8), and it is systematic by CPU model: the kernel
  over-rates Zen 4 and Zen 5 by 6 to 11% against the median (so the reader looks
  6 to 10% slower there than it is) and under-rates the Intel 8573C. That
  residual is inside the 1.39 margin and is not corrected per CPU.
  **Recalibrate** when the Go toolchain changes, or when a new CPU model shows
  norm below 16.8 on a PR that does not touch the parser; recalibration is a new
  median over at least 6 legs. **Never add a per-CPU table.**
  `refNominalCalibrated = true`: the synthetic test fails when norm is below 14,
  the slice line says `calibrated=1`, and `ci/assert_sandbox_tests.py` sets
  `REQUIRE_CALIBRATED = True`, so a line that says anything but `calibrated=1`
  fails the job and Go alone cannot switch the independent check off.

  *Margins.* Normalised, the reader clears the floor by 19.5 / 14 = **1.39x**
  (the worst leg, 17.6 / 14 = 1.26x). For the absolute floor: 2147 MB at 8 MB/s
  is **268 s** by the slice proxy, about half the 600 s deadline (3.6 MB/s is the
  hard limit).

  The CI job prints the CPU model (`/proc/cpuinfo`) and `nproc` next to
  `uname -r`, so each leg's rate can be read against its host.

  *The measurement.* The gate is on the **best** (minimum) pass-1 time over at
  least 7 passes and at least **250 ms** of wall time after one warm-up pass,
  never on a single shot: a 50 KB trace is read in about 2 ms, so one sample is
  mostly noise, and noise only ever adds time, so the minimum is the right
  statistic for a capability floor. (On an Apple M2 Max a single-shot sample
  fell to 5-7 MB/s under contention while the true rate is about 20.) For a
  trace of 128 KB or less the figure is taken **with the garbage collector off**
  (after one collection), because a stop-the-world wait on a busy machine is
  neighbour noise; production runs with the collector on, so the figure flatters
  the reader a little. The whole 4 MiB pass (collector on) reads 10 to 15%
  below the slice figure; that is why the slice, not the 4 MiB pass, is gated.
  **G1 is enforced in the dedicated kiln-sandbox job only**, selected by the
  test-only environment variable `KILN_G1_GATE=1`, set in that job in `ci.yml`
  and read nowhere in production code. Elsewhere (the general `build` job, local
  runs) the same `PROBE:` lines are logged with `gated=0` and no floor is
  applied (a zero best time or a non-finite rate still fails). The reason: the
  general build job runs kiln's tests in parallel with many other packages, and
  there the synthetic slice read 11.2 MB/s (66,600 bytes in 5.96 ms) and the
  4 MiB census 8.9: a contended runner, not a slow reader. (The dedicated job's
  slice rates are in the fleet table above; real traces read 28 to 37 MB/s
  there.) A kiln-sandbox job run without
  `KILN_G1_GATE=1` has no `gated=1` synthetic line, so
  `assert_sandbox_tests.py` fails it.
  In the job, a real trace of **32 KB to 128 KB** that falls below the 8 MB/s absolute floor fails
  it. Traces under 32 KB, and traces over 128 KB, are logged with
  `gated=0` (census only): a long pass sees every change in how the CPU is
  shared, and only about 7 passes fit, so best-of cannot see through a sustained
  shortage (a 923 KB trace read below 14 MB/s in 15 of 60 contended trials). The
  floor on larger inputs is carried by one deterministic input that always runs
  (`TestSandboxTraceGateSynthetic`): a 64 KB slice of whole repetitions of
  `testdata/build.strace`, logged as `PROBE: trace synthetic-slice ... gated=1`,
  with the same statistic, carrying both parts of G1 (its line also carries `ref_mb_s`, `norm_mb_s` and `calibrated=1`, all before `gated=` and before any refusal field). The same test reads the fixture repeated to
  at least 4 MiB (pass 1 must read all of it) and logs its best-of-7 rate on its
  own line, `synthetic-4mib-census`, `gated=0`. Each gated line carries
  `best_mb_s` (gated, raw) and real traces also carry `grammar_mb_s` (single shot,
  census only). A zero best time or a non-finite or non-positive rate FAILS.
  `ci/assert_sandbox_tests.py` independently fails the job unless a passing test
  logged a `gated=1` line, the `synthetic-slice` line is among them and comes
  from `TestSandboxTraceGateSynthetic`, and every gated `best_mb_s` is finite,
  positive and at least 8 (the slice line also needs a finite, positive `ref_mb_s`, `calibrated=1`, and a `norm_mb_s` of at least 14); a PROBE line counts only when its payload starts
  with `PROBE: trace ` after the go test `file:line` prefix, which must be
  indented exactly four spaces (in `go test -v -json`, the mode CI runs, every
  `t.Logf` line has that indent at any subtest depth; a continuation line of a
  multi-line message has eight and no `file:line`, so it cannot match). The
  script also buffers output per test and reads only newline-terminated lines,
  because `test2json` cuts a long line into events of at most 1024 bytes with no
  newline at the cut, and the second half of a long `%q` refusal must not be
  taken for a line of its own. A label ending in `:` is not a measurement
  (the tests log failures to open or measure a trace as `PROBE: trace-error`,
  with the error `%q`-quoted). The two `%q` refusal fields (`old_refusal=`,
  `grammar_refusal=`) are the LAST fields of a line; the script reads
  `key=value` facts only from the text before the first refusal field, and the
  first occurrence of a key wins, so text a trace influenced cannot forge a
  fact. A rate measured at
  about 50 KB or 64 KB is a **proxy**: it does not prove the rate at 2 GiB.
  `trace.Measure`, `MeasureBest` and `BestPass1` toggle the process-global GC
  setting, are not goroutine-safe, and are for CI and tests only
  (`TestMeasureIsNotUsedByProductionCode`).
- **G2 (a reported target, never a gate; restated by D49, 2026-11-24).**
  Target: grammar parse time at most 10% of the step's wall time, read as the
  parse share at p50. The 5 s absolute limb is dropped. G2 blocks nothing, PR 8
  included (it was "blocking before the old scanner is deleted" until D49). The
  census keeps reporting the share distribution, the `parse_mb_per_s`
  distribution and a per-image parse throughput; see D49 for the measurements
  and the reasoning. The `PROBE:` lines carry
  `step_wall_ms`, `parse_ms` and `parse_share` where the step's wall time is
  known, so the census can be read off CI logs. The wall comes from the step
  sidecar's `started_at`/`ended_at` (`kiln-record-step/2`) for the record-step
  test, from the probe runner's in-memory `ProbeRunRecord.Wall` for the probe
  path, and from a stopwatch for `TestSandboxStraceSignals` (sleep-dominated,
  so its share is not representative). A wall that is not positive (a stamp
  that does not parse, a clock that stepped back) is dropped, never divided by,
  and counted as `walls_dropped=N`, a process-global cumulative counter that
  appears on every later `PROBE: trace` line of the run. A wall time is NEVER written into a sealed
  file: `ProbeRunRecord.Wall` is `json:"-"` and `TestProbeWallIsNeverSealed`
  holds that line, because a wall differs on every run and would make
  `pre/probes.json` non-reproducible. The census was read off real CI
  runs (D49); it showed the target is missed by the trace-rate to parse-rate
  ratio, which is why G2 became a reported target rather than a gate.

Measured: on a real kiln-sandbox CI trace (51,650 bytes, 327 lines) the old
scanner reads 144.7 MB/s and the grammar path (compact `LineParser`) 22.8 MB/s;
on the committed fixtures pass 1 is 16.9 MB/s on an Apple M2 Max. Before the
compact parser (P2 PR 5) the grammar path ran at 11.6 MB/s and would have failed
G1. (An earlier draft of this section quoted the old parser at 23.7 MB/s; that
was a stale measurement and is withdrawn.) If G1 is ever missed again, the
fallback is a worker pool for the stateless per-line lex/parse with the semantic
pass kept sequential.

**What the flip means (PR 6).** The grammar path is the production default.
`parseCrossChecked` returns the grammar path's result, so for every
reason-only row of the table above the recorder now surfaces the grammar path's
`Reason`. The old scanner stays compiled as the **oracle**, reachable only
through `crossCheck` (this package's tests, and `KILN_TRACE_DIFF=1` in the
kiln-sandbox job): an old-refused/new-accepted pair is still a loud failure, and
every other comparison rule is unchanged. There is no runtime or environment
switch back to the old scanner: that would be a soundness downgrade (it is the
lenient reader). Rollback is a revert.

**Deleting the old scanner (PR 8).** Only after Phase B (PR 7) and a soak of at
least 14 days and at least 200 kiln-sandbox runs with zero mismatches, zero
allowed divergences on real traces, and clean nightly fuzzing. G2 is
not a precondition (D49): the grammar reader has been the production reader
since PR 6, so deleting the oracle changes no parse cost. Until then it is kept
as the oracle.

**What counts toward the 200 runs (soak rule).** The soak measures the
cross-check, not the throughput gate. A kiln-sandbox run counts toward the 200
if its cross-checked tests ran under `KILN_TRACE_DIFF=1` with no mismatch and no
divergence, **whatever G1 did** (a slow host failing the capability or absolute
gate says nothing about whether the two readers agree). The coverage
of REAL recordings is the D34 line: each kiln-sandbox run's `PROBE: crosscheck`
line (the children's recordings, both readers, no mismatch, no allowed
divergence) counts toward it only with traces >= 1 AND accepted >= 1 in total; a
run with `traces=0`, `accepted=0` or no line does not count. D34 coverage starts
at the merge of the PR that added it: earlier kiln-sandbox runs do not count
retroactively. A job that died before
its tests ran (a runner fault, a setup failure) counts **neither way**: it is
not a pass and it does not reset the count. Only a mismatch or a divergence
resets the count. The resource limits (1 GiB and 8M events, and the caps above) may be
lowered freely, but raised only by a spec revision that includes a DoS review
and a benchmark of the worst-case input, with the new values committed in the
spec.

**PR 5 as built.** `internal/trace` has `handleLineG` (grammar_state.go), the
`crossCheck` hook and its allow-list of the table above (crosscheck.go; the
list is `stricterRow`, one row per line shape), a seeded line differential,
`FuzzLineDiff` and `FuzzParseDiff` (linediff_test.go), and `Measure`
(measure.go). `TestNoThirdPartyDependencies` (kiln root) enforces the KILN00
section 17 rule on `go.mod` and on `go list -deps`. The kiln-sandbox job sets
`KILN_TRACE_DIFF=1`, which makes `ParseOptions` cross-check every real trace and
panic, after printing a bounded, sanitised window of the lines around the
mismatch to the job log (no artifact is uploaded: the window is all that is
kept, and it stays in that log); and it logs `PROBE: trace ...` lines with the
size, line count and the throughput of both readers for every trace the sandbox
tests produce. **Trace sizes and grammar-path throughput on real traces: to come
from the first CI run.** The default was still the old scanner then; the flip is D24 PR 6.

**On the compact `LineParser` (P2 PR 5).** `handleLineG` no longer builds an
`*ASTNode` tree per line. Each trace parse owns one `straceparser.LineParser`
(inside `lineState`, never shared between goroutines) and one reused argument
slice; the line shape, the arguments (`AppendRawArgs`), the text after the
closing parenthesis and the pending prefix come from the compact helpers. The
Options (line limit, 64 bracket levels, 65,536 tokens, memo cap, strict quoted
strings), the byte pre-checks, the pending map, the D18 and dead-resumed state
machine, the strict `parseResult` and the refuse-the-whole-trace panic guard
are unchanged, and so is every record and refusal: `grammar_legacy_test.go`
keeps the ASTNode version as a test-only reference and
`compact_equiv_test.go` (fixtures, 300k seeded lines, 600k seeded sequences,
`FuzzCompactEqualsLegacy`) demands identical output. The all-space-argument
guard stays in kiln's `trimArgs` (the parser accepts `1  f( , <unfinished ...>`
with the raw argument `" "` on every path). The old scanner was still the
default then, and `crossCheck` was unchanged. Measured on the four committed fixture
traces (Apple M2 Max, `go test ./internal/trace -bench 'LineHandler|Pass1'
-benchmem`): the old scanner 201 MB/s, 359 allocs per corpus pass; the grammar
path on the ASTNode tree 13.6 MB/s, 1693 allocs; on the compact path 17.9 MB/s,
421 allocs (1.3x faster, 4x fewer allocations; the reduction is in allocation
COUNT: bytes per pass only drop from about 360 KB to about 203 KB, against about
18 KB for the old scanner, the remainder being the lexer's per-line token
slice); production pass 1 (bufio plus
handler) 141 MB/s old against 16.9 MB/s grammar. The fixtures are short
and dense, so the ratio to the old scanner (8x to 11x here: 141/16.9 for pass 1, 201/17.9 for the handler) is a worst case
rather than the "within 3x" gate figure; the gate was restated (G1, G2 above) from
the real-trace `PROBE:` lines. Cutting the
remaining per-line token-slice allocation (a lexer `TokenizeInto`) is P2 PR 6, not to be confused
with D24 PR 6, the default flip.

**P2 PR 6 experiment: caller-owned token storage.** Add
`(*Compiled).TokenizeInto(source string, dst []Token) ([]Token, error)` as an
opt-in entry point for reuse. It must run the same standard-mode tokenizer and
grammar matcher as `Tokenize`, with identical token kinds, values, source
positions, flags, EOF token, errors, and refusal order. The returned slice may
reuse and overwrite `dst`; its contents are borrowed until the caller next
reuses that storage. `Tokenize` and `GrammarLexer.Tokenize` remain allocation-
owning APIs: later tokenization, errors, or parser calls must never mutate
previously returned owned slices. Keep pre/post-tokenize hooks and callbacks on
their existing owned path. Do not silently route indentation or layout modes
through a different algorithm; reuse may be limited to the standard mode unless
all other modes retain exact behavior.

`strace-lexer.TokenizeLineMaxInto` must preserve the existing byte limit,
newline, invalid-token and malformed-string checks. The owning
`strace-parser.ParseLineTokens` / `checkLine` path continues to request owned
tokens. `LineParser.ParseLineInto` may pass only its own previous token buffer
as scratch after invalidating the previous tree and token borrows. Do not reuse
or clear token storage supplied or returned by `PreParse` hooks; keep hook-
bearing calls on the existing owned route if their lifetime cannot be
preserved. Compact-parser refusal precedence, including the late `PostParse`
refusal, must not change. A failed parse may expose the same token results and
error as today; its storage is still borrowed only until the next parse or
`Reset`.

Clear occupied token slots before retaining/refilling owned scratch so old
source strings are not pinned through stale `Token.Value` fields. Bound the
retained token capacity and discard oversize buffers; `Reset` releases token,
tree, and source references. The zero value remains usable, separate
`LineParser` instances remain safe to use concurrently, and no mutable global
pool is allowed. Never retain or mutate caller source bytes, use unsafe
string conversion, or let a reused buffer escape through an owning API.

Acceptance requires exact token/EOF/error/position equivalence; owned-output
stability; long-to-short, invalid-to-valid, blank-line and source-retention
cases; reset and oversize-cap checks; hook/borrow lifetime checks; and race,
differential, and bounded fuzz tests. Measure the ordered 26 lines from
`build.strace` repeated 36 times with splitting outside the timer, identical
token count/checksum/EOF/error verification, and interleaved baseline/candidate
and candidate/baseline runs. Record source SHA, Go version, OS/architecture,
CPU, GC settings, fixture hash, and commands. Mac measurements are local
relative evidence only. Allocation reductions do not establish a Linux
throughput improvement; retain the 14 MB/s Linux gate and make no native Linux
speedup claim without a same-host native Linux comparison. Report candidate CI
gate results and Mac relative measurements separately. `newLexer` allocation
is outside this isolated experiment.

**The PR split.**

1. This spec, plus the KILN00 §17 and README amendment.
2. G1, shared lexer: a compiled, panic-free `Tokenize` with first-byte
   dispatch and a literal fast path (target 20 MB/s or better, committed
   benchmark).
3. G2, shared parser: `MaxDepth` (a deep input such as 200k `[` currently
   overflows the stack and kills the process, for every grammar consumer, so
   this is a denial-of-service fix for all of them), `StartRule`, a cheaper
   memo, and a reusable compiled grammar. G3 (honour `errors:` in Go) is
   optional; G4 is a Go grammar drift check as a per-package test.
4. The grammar files, `strace-lexer`, `strace-parser`, and a drift test.
5. **Done (PR 5).** Kiln wiring with the old parser still the default: `go.mod`
   `require`/`replace` for the in-repo modules, the CI sandbox-job path
   filter, `handleLineG`, `crossCheck`, the fuzzers, and a measurement of
   real trace sizes.
6. **Done (PR 6).** Flip the default to the grammar path (G1 above); the old scanner stays as the oracle.
7. Phase B conversions, one PR each. *Pending.*
8. Delete the old scanner (soak criteria above). *Pending.*

## 5. From trace to manifest

### 5.1 Resolving each observation

*(Amended by decision P13, §4.8: wherever this section says "the post-run disk" it means the disk at the end of the action, because the recorder classifies each action when its last step ends, with recording serial.)*

The parser tokenises strace output with a **real state machine**, not
regular expressions, and unescapes strings exactly. A filename containing
a quote, `<`, `>`, `)`, ` = `, a newline or a leading `+++` must either
round-trip exactly or give `trace-incomplete:ambiguous-path`. The parser
is fuzz-tested with such names, since tracked filenames come from PRs.

It looks only at **path-typed arguments** and `-y` annotations.
Other strings (a `read` buffer, for example) are ignored, even when
truncated. A non-UTF-8 path, a truncated path or a
`(deleted)` annotation makes the trace incomplete.

**Working directory.** The parser tracks it per process, starting from
the known initial directory. `chdir` and `fchdir` change it, and `clone`
copies it, or shares it under `CLONE_FS`. It never relies on an
`AT_FDCWD` annotation.

**Calls that return a descriptor.** The `-y` annotation on the return
value is the kernel's own resolved path. An open of `/proc/self/fd/N` or
`/dev/fd/N` is classified by that annotation.

**Other calls** (metadata, probes, writes). The path is resolved **component
by component**, starting from the working directory or the `-y` path of
the `dirfd`, using an `lstat` walk on the post-run tree.
- Each symlink is expanded where it occurs, with a 40-hop limit.
- A `..` is applied to the **resolved** directory, never lexically, which
  matches the kernel. `L/../x`, where `L` is a link, means `x` beside L's
  target.
- Every link traversed is itself an observation.
- A traversed component `c` is **stable** only in these cases:
  - **(a)** `c` is tracked, and no action in the run made a namespace change
    to `c` **or to any ancestor of `c`**. A namespace change is a `rename`
    source or destination (including `RENAME_EXCHANGE`), `unlink`, `rmdir`,
    the destination of `link`/`linkat`, `symlink`, `mkdir`, `mknod`, or an
    `O_CREAT` open.
  - **(b)** this action created `c` exactly once (`mkdir`, `symlink` or
    `O_CREAT|O_EXCL`), the creation's **completion line** precedes the lookup's
    **start line** (`Event.StartLine`: for a split call the `<unfinished>`
    line), no namespace change by anyone touched `c` or an ancestor
    afterwards, and `c` was never a `link`/`linkat` source or destination
    (hard links make aliases the trace cannot follow).
  - **Outside the repository**, `c` resolves on the post-run disk and is
    stable only if no traced namespace change touched it or an ancestor.
    `$HOME` and `TMPDIR` are fresh sandbox tmpfs, so the disk is **never**
    consulted under them: a path there was created by the action (rule b) or
    is absent.

  Everything else gives `trace-incomplete:unstable-path`: an untracked
  leftover of an earlier action, a component created after the lookup began
  or twice, a renamed link. There is no "did not exist yet" exception.
  Three narrow exceptions for the FINAL component of a successful open, stat
  or exec, and for `$HOME` itself, are in §5.5 ("Relaxations").
- A `..` after a component the walk could not enter (missing, or not a
  directory) cannot be placed. On a failed call, the call looked at that
  component, which is observed; on a successful call, the result is
  `unstable-path`.
- A successful open takes its final path from the `-y` annotation, and the
  walk still runs to find the links on the way. If either path is in the
  repository and they disagree, the post-run disk no longer shows what the call
  saw: `unstable-path`.
- **A followed call whose final name is absent on the post-run disk** was
  removed afterwards, or was never there. If the call found something there (it
  succeeded, or failed with an errno other than `ENOENT`/`ENOTDIR`), the action
  must have made the name, the creation completing before the lookup started;
  else `unstable-path`. And a removal of a name the action had not yet made
  (an untracked leftover) that completed after the lookup started gives
  `unstable-path` too: the lookup may have gone through a link there that the
  disk no longer shows. The removal rule applies to **any** component at which
  a walk stopped (a missing parent as in `stat("L/x")` followed by
  `unlink(L)`), not only the final one. Only a clean creation (`mkdir`,
  `symlink`, `O_CREAT|O_EXCL`) or an `O_CREAT` open keyed by a kernel path
  counts as "the action made the name". A successful open whose `-y` path is the name is exempt
  (§5.5(a)).
- **A successful call whose walk stopped before its final component**, at a
  component the disk shows missing or not a directory, contradicts the disk:
  `unstable-path`. This holds with no change of the action's own.
- **Overlap.** Calls of different processes overlap, and a change takes effect
  somewhere in `[start, completion]`. So every comparison treats anything that
  could have taken effect during the lookup's interval as "during". A path
  counts as created by the action only for calls that START after the creation
  COMPLETED, and an earlier "not found" counts only if it completed before the
  creating call started. A walk that found a name absent on the post-run disk
  is unstable if some creation of that name began before the lookup ended and
  no removal that began after that creation completed also completed before
  the lookup began.
- **The working directory.** Its expression is walked like any other path on
  every call, so a `chdir` target that later becomes unstable makes every later
  relative call from that directory unstable too.
- A component is stable only as far as the walk is bounded: the walk has a
  per-action step budget (50M) and a path-length cap; exceeding either is
  `trace-incomplete:size`.

**Exec.** `execve(path)` is a read of `path`. The kernel then loads,
invisibly, the interpreter of a `#!` script and the `PT_INTERP` loader of
an ELF file. The parser adds both as reads, following `#!` up to four
levels deep. (The kernel follows up to five; four is stricter, and a fifth
hop is `exec-depth`, so it fails closed.) The interpreter is the text up to
the first blank or tab, with **no** trimming of `\r`: the kernel looks up
that exact name. The head bytes come from the tracked tree for a tracked
file, else from the post-run disk, never under `$HOME` or `TMPDIR`. An
untracked file read from the disk must not be written, truncated, renamed,
linked onto or unlinked, nor an ancestor changed, by any action, by a change
that completed after the **first** exec of it STARTED (so an overlapping
write-open counts, and so does a rewrite between two execs); otherwise
`trace-incomplete:exec-unreadable`.

**`/proc`.** The `ephemeral` list of `host-roots.json` (§5.3) is the one
authoritative list. Its seed allows the ubiquitous entries: `/proc/self/{exe,maps,cgroup,mountinfo}`,
`/proc/{meminfo,stat,cpuinfo,filesystems}`, `/sys/devices/system/cpu/**`,
`/sys/fs/cgroup/**`, `/dev/fd/N` and `/proc/self/fd` with `/proc/self/fd/N` (D37).

`/proc/self/status` and `/proc/self/stat` reveal `TracerPid`, so a
program could behave differently when traced. A read of either gives
`trace-visible:<path>` and no record. Every other `/proc` path makes the
trace incomplete. That includes any numeric `/proc/<pid>/` path: the PIDs
in the trace are the tracer's, not the action's. `/proc/self/root/…` and
`/proc/<pid>/fd/…` could reach any file by another name.

**`/proc` magic links (D28).** `cwd`, `root`, `exe`, `fd/N`, `map_files/X` and
`ns/X` are links the kernel follows to wherever the process points them, and
`/dev/fd` is a link to `/proc/self/fd`. The path is never walked through them;
three rules keep it fail-closed. (1) A `..` after any `/proc` component is
`trace-incomplete:proc-path` unless it is exact (`/proc/..`, `/proc/self/..`,
`/proc/<pid>/..`); every `..` under `/dev/fd` is refused. (2) A path that goes
THROUGH a magic link (a further component after `cwd`, `root`, `exe`, or after
`fd/N`, `map_files/X`, `ns/X`, at any depth, `/dev/fd/N/x` included) is
`proc-path` even when an ephemeral pattern matches it: **an ephemeral glob can
never cover a path through a magic link**. (3) A successful open of a `/proc`
name is compared with the descriptor's `-y` path: equal, or the same file with
`self`/`thread-self` (and the links `/proc/mounts`, `/proc/net/…`) spelled with
pids, is fine; another non-`/proc` path is classified as the file really read
(so an open of `/proc/self/exe` is a read of the real binary), another `/proc`
path is `proc-path`; and an open of a magic link itself with an empty or
non-absolute, non-anonymous `-y` path is `proc-path`. See D28 in §5.5.

### 5.2 Mapping observations to entries

Every resolved path inside the repository is classified. Package
boundaries are the graph's (KILN03 §2).

**Record-time digests come from the clean, pre-BUILD tree via `Source`**
(KILN03 §9), never from the disk after the run. `path:` and `list:` states
are always states of the tracked tree.

**The working tree must equal `Source`.** A BUILD reads working-tree
bytes, but digests come from `Source`. So before recording, and before
any lookup that can skip, the runner checks that every tracked file on
disk is byte-identical to its `Source` digest. It hashes the working tree
itself, with no smudge filters, `core.autocrlf=false` and no sparse
checkout. Any difference means nothing is recorded or skipped. Packages
containing submodules, LFS pointers, or files with the `ident`,
`export-subst` or `filter` attributes are `precondition:` uncacheable.

| The path is… | Entry | Digest at lookup |
|---|---|---|
| inside package `q` (the deepest enclosing one), whether tracked or absent from the tracked tree | `pkg:<q.rel_path>` | `T(q)`, defined below; a package that no longer exists gets the fixed *missing* digest |
| outside every package (e.g. `code/scripts/x.py`, a root config) | `path:<rel>`; an absent path is recorded as its **first absent component** | the digest of its tracked state: *absent*, *directory*, *file* (exec bit + content) or *link* (target) |
| a directory that was **listed** | `list:<rel>` | the digest of the sorted names and kinds of its tracked children (a child directory counts if it contains any tracked file) |
| `.git` itself, by `stat` or `access` | none | — |
| anything beneath `.git/` | none: `vcs-read` (§6) | — |
| untracked and present | §7 decides | — |

`T(q)` is `D(q)` together with the names of the packages nested directly
in `q`:

```
T(q) = SHA-256( str("kiln-traced-pkg") || bytes(D(q))
              || list( str(rel_path) of each package directly nested in q, sorted ) )
```

`D(q)` (KILN03 §3.1) encodes every tracked name in `q` except those in
nested packages. So if a tracked file later appears where an action found
nothing, `D(q)` changes. If a new **package** appears there instead, the
nested-package list changes. Either way, `T(q)` changes and lookup misses.

Listings are always `list:` entries, never folded into `pkg:`, because a
listing also sees the directory names of nested packages.

**Written, then read.** A path the action itself created is its own
output, not an input. The parser treats a path as created by the action
only when the action's first touch of it provably started from empty:
- `O_EXCL`, `O_TRUNC` or `creat`;
- `mkdir` or `symlink`;
- a `link` or `rename` onto it, whose **source** is still an observation;
- an earlier probe by the same action that returned "not found".

Any other first touch is an observation of whatever was there before.

An earlier "not found" counts only if it is unambiguous: an `ENOENT` on the
final component that the walk really looked up (not a missing parent, not a
path ending in `/` or `/.`, not an exec, `rename` or `link` failure), and the
parent is in the repository or is the action's own. A failed `rename` or `link`
observes **both** names, by errno.

`O_TRUNC` without `O_CREAT` only works on a file that was there, so on a path
the action did not make it is an observation of that file.

**Creation is an absence observation.** Before the action makes a path
(`mkdir`, `symlink`, `mknod`, `bind`, an `O_CREAT` open, the destination of
`rename` or `link`), nothing of that name was there from the action's point of
view, and another action could have created it first. So a creation with no
own ancestor is recorded like an `ENOENT` probe: `pkg:q`, or
`path:<first absent>`. The raw record also lists these as **absent
observations** (in-repository paths whose first absent component is
untracked) for the merge job.

**Writes to tracked files.** An action that writes a tracked path gets
`writes-tracked:<path>` and no record. An unlocked install that rewrites
its lockfile is caught this way. Any other action that observed that path
gets `reads-product`, whether through a `pkg:`, `path:` or `list:`
entry, in any shard, and **even if the final content is unchanged**: a
write followed by a restore leaves the same hash, yet a reader in between
saw the edited bytes.

**The tainted set.** Every tracked path that any action in the run wrote
joins a run-wide *tainted* set, even if it was restored afterwards. A
package loses every record (`reads-product`) when either of these holds:
- its BUILD variants, or any other file the executor reads untraced
  (package metadata, `.gitignore`), are tainted;
- any of its `pkg:`, `path:` or `list:` entries covers a tainted path.

**The backstop.** After the job, the recorder also re-hashes every
tracked file. If any changed, the **whole shard** emits nothing.

That re-hash runs on a tree the BUILDs controlled, so it is defensive:
- each file is checked with `lstat` and then opened with
  `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY`;
- anything other than a regular file or a symlink gives
  `trace-incomplete:non-regular`;
- the bytes read per file are capped;
- the whole step runs within a wall-clock budget per shard.

The post-run `lstat` walk of §5.1 follows the same rules.

### 5.3 Reads outside the repository

Manifests never store out-of-repository paths. Instead,
`code/kiln/legacy/host-roots.json` declares what an action may read:

```json
{
  "roots": [
    {"path": "/usr",  "kind": "image"},
    {"path": "/etc",  "kind": "image"},
    {"path": "/opt/hostedtoolcache/go/1.23.4", "kind": "tool", "tool": "go"},
    {"path": "$HOME/.cargo/registry", "kind": "state", "ecosystem": "rust",
     "fetchers": ["/opt/hostedtoolcache/rust/1.99.1/bin/cargo"]}
  ],
  "ephemeral": ["/dev/null", "/dev/urandom", "/dev/fd/*",
                "/proc/self/exe", "/proc/self/fd", "/proc/self/fd/*",
                "/proc/self/maps", "/proc/self/cgroup", "/proc/self/mountinfo", "/proc/meminfo", "/proc/stat",
                "/proc/cpuinfo", "/proc/filesystems",
                "/sys/devices/system/cpu/**", "/sys/fs/cgroup/**"],
  "absent-ok": ["$HOME/.config", "$HOME/.npmrc", "/include", "/opt/rh", "/pyvenv.cfg",
                "/selinux", "/sys/fs/selinux",
                "/sys/kernel/mm/transparent_hugepage/hpage_pmd_size"]
}
```

`/bin`, `/lib` and `/lib64` are symlinks into `/usr` on the runner image,
so they resolve under it.

| Kind | Covered by | Reads allowed when |
|---|---|---|
| `image` | `Host.ImageID()` in the weak key | the file's ctime is no later than the job-start stamp; a file a setup step changed gives `host-read:setup-modified:<path>` |
| `tool` | the tool's pin and probe in the weak key | the tool is in the weak key's `tools`; the pinned version matches the path; the probe passed |
| `state` | the ecosystem's lockfile, which the action read and which is therefore in the manifest | the ecosystem is in `pinned.json` and its fetchers are enabled (stage 5, §16) |
| `ephemeral` | nothing, by design (randomness, CPU count, the process's own `/proc`) | always |

Four more rules apply:

- **State roots start empty.** Every `state` root must be empty or absent
  before the first BUILD. Otherwise the job records nothing
  (`unscrubbed:state-preloaded`), so `kiln-record` restores no caches.
- **Only fetchers write state.** Only a fetcher process may write under a
  `state` root; any other writer gives `state-write:<path>`. A tool's own
  build cache (`GOCACHE`, `~/.cache/…`) declared as `state` is KILN00 §9.5
  trust, and §14 names it.
- **Absent probes.** An absent probe outside the repository is allowed only
  under an `image`, `tool` or `state` root, or in `absent-ok`. `$HOME`
  dotfiles need explicit entries. An `absent-ok` entry covers an exact probe
  of itself on the traced result alone. A probe of a CHILD of an entry
  (`X/y`) is allowed only if the entry itself is absent: an `lstat` of `X` on
  the classifier's disk gives `ENOENT` or `ENOTDIR`. Any other answer (present,
  even as an empty directory, or `EACCES`) is `host-read`, because a present
  directory's contents are undeclared input. An entry inside the fresh `$HOME`
  or `TMPDIR` is absent by construction, and the disk is never asked beneath
  them.
- **Temporary files.** A read of the action's own temporary files (paths
  it created, §5.2) is allowed. `$HOME` and `TMPDIR` are **fresh tmpfs**, so
  a stat or listing of those two directories is allowed, and the post-run
  disk is never consulted beneath them, **except** for the checkout, its
  ancestors, and every declared host root and its ancestors (a `state` root
  such as `$HOME/.cargo/registry` persists, and hosted runners put the checkout
  under `$HOME`). The checkout may not be `$HOME` or `TMPDIR`, and neither may
  lie inside it. `ReadHead` and any other file read
  from the post-run disk open with `O_NOFOLLOW|O_NONBLOCK`.
- **Image absence.** An absent probe under an `image` root is checked against
  the ctime of the deepest directory that exists: one after the job-start
  stamp is `host-read:setup-modified`.

**Decision D27: libselinux start-up probes (ubuntu-26.04).** `ubuntu-latest`
moved to ubuntu-26.04 (libselinux 3.9). Every libselinux-linked tool
(coreutils, grep, sed) calls `statfs` on `/sys/fs/selinux` and `/selinux` and
opens `/proc/filesystems` at start-up. Unlisted, those gave `host-read:` and
`trace-incomplete:proc-path` and voided every recording. The policy now has:

- `/proc/filesystems` in `ephemeral`. It is constant for a kernel build, and
  the image id is already in the weak key (`Host.ImageID()`). It stays in the
  one authoritative `/proc` list: there is no classifier allowance and no
  hard-coded `/proc` exception (§4.5, §5.5).
- `/selinux` and `/sys/fs/selinux` in `absent-ok`, so they are allowed **only
  while absent**: a failed probe of a child such as `/sys/fs/selinux/enforce`
  counts only if the entry itself is absent (the §5.3 absent-probe rule). There is deliberately no `/sys/fs/selinux/**` entry anywhere
  (not `ephemeral`, not a root): a present SELinux filesystem is a `host-read`.

The invariant, for libselinux users: **an SELinux-enabled host records
nothing.** It holds because libselinux's start-up probe finds the present
`/sys/fs/selinux` (a `host-read`), and any child probe under a present entry is
also a `host-read`. A program that never touches those paths is unaffected, and
needs no such guarantee, since it cannot see the filesystem. SELinux state
changes outputs (`ls -Z`, `cp --preserve=context`), so it must never be
silently pinned. On such a host the probe finds the directory (`host-read`), and
libselinux goes on to read `/proc/mounts` and `/proc/self/attr/current`, which
stay `trace-incomplete:proc-path`. `kiln:recorder` needs no version bump: the
change is in `policy:host-roots.json`, which is in `consumed` and so in the
digest. The `kiln-sandbox` CI job runs on both ubuntu-24.04 and ubuntu-26.04,
so image drift is caught by a required job. That holds only because the
sandbox tests' policy fixtures derive their `ephemeral` and `absent-ok` lists
from the real `code/kiln/legacy/host-roots.json` (they once carried their own
short lists); and a change to `code/kiln/legacy/**` selects the job in
`ci-gates.json`.

**Decision D37: `/proc/self/fd` and four absent probes (first real-build
censuses).** Every C/C++ action reads `/proc/self/fd/3` thousands of times
(clang `readlink`s `/proc/self/fd/N` for each header it opens, after one
`access("/proc/self/fd")` per process), and four absent probes were flagged
`host-read`. The policy now has, and `kiln:recorder` needs no version bump
(the change is in `policy:host-roots.json`, which is in `consumed` and so in
the digest, as in D27):

- `ephemeral` gains `/proc/self/fd` and `/proc/self/fd/*`: the same kernel
  object as the `/dev/fd/*` entry already there. A single `*` matches one
  component, and `X/**` is the only form that also covers `X`, so the directory
  and its entries are two entries.
- `absent-ok` gains the exact names `/sys/kernel/mm/transparent_hugepage/hpage_pmd_size`
  (the Go runtime's probe, absent on these runners), `/pyvenv.cfg` (python),
  `/opt/rh` and `/include` (compiler search paths). An `absent-ok` entry is
  allowed only on a traced `ENOENT`. There is deliberately no
  `transparent_hugepage/**`: `enabled` and `defrag` are tunable and change
  runtime behaviour. None of the four is `ephemeral`.

What the new entries do not cover (each pinned by `internal/traced/procfd_test.go`
against the real policy file):

- (a) An OPEN through `/proc/self/fd/N` or `/dev/fd/N` is classified by the
  descriptor's own `-y` path, whatever the ephemeral list says
  (`openVirtual`, `ops.go:226-233`): an open of a host file through it is a
  `host-read` of that file. The list governs only metadata calls.
- (b) A path that continues past the magic link (`/proc/self/fd/N/x`) is refused
  as `proc-path` by `procMagicBeyond` (`host.go:150`), which `observeVirtual`
  tests before `ephemeralMatch` (`host.go:120` against `host.go:124`).
- (c) `/proc/<pid>/fd` and `/proc/<pid>/fd/N` for a numeric pid, and
  `thread-self`, are not matched by the `self` entries and stay `proc-path`:
  the pids in the trace are the tracer's.
- (d) A PRESENT read of any of the four absent-ok names stays `host-read`
  (`observeHost`, `host.go:208`: `absentOK` is consulted only when the path is
  absent).

**Decision D38: two `/proc` reads and `GOWORK=off` (second real-build
census).** Two more `/proc` opens were flagged `trace-incomplete:proc-path`, and
Go's upward `go.work` search probed `/go.work` and `/home/go.work`. As in D27 and
D37 the change is policy-only, with no `kiln:recorder` bump (`host-roots.json`
is in `consumed`; `env.json` is a keyed input):

- `ephemeral` gains the exact names `/proc/sys/vm/overcommit_memory` (allocator
  tuning, the same class as `/proc/meminfo`, already listed) and
  `/proc/version_signature` (the kernel build identity; `uname(2)` is not traced
  and gives a program the same information, so the file adds no exposure). There
  is deliberately no `/proc/sys/**`: `swappiness`, `max_map_count` and the rest
  stay `proc-path`, and `/proc/version` is not added.
- `env.json` gains `GOWORK`. The census recording step runs `kiln record
  prepare` with `GOWORK=off`, which flows into the sealed build-tool
  environment. `GOWORK=off` stops the upward search that would otherwise read
  `/go.work` and `/home/go.work` on the host. The repository tracks no `go.work`,
  so build semantics are unchanged, and the value is part of the key.

What the entries do not cover (pinned by `internal/traced/procsys_test.go`
against the real policy file): a name beneath either file
(`overcommit_memory/x`) and the sibling tunables stay `proc-path`; and a WRITE
open of either file is never allowed by the list. A writing open of a virtual
name that is not an fd link makes the write set unknown (W5, `ops.go`), so the
record carries `trace-incomplete:writes-unknown` and `writes_complete` is false.

**Decision D39: ancestor nodes and absence under the fresh tmpfs (real-build
census, `host-read` in about 45 actions; `kiln:recorder` 6 to 7).** The census
showed `host-read` raised mostly by `stat` of the ancestors of the sandbox's own
directories (`/`, `/home`, `$HOME`, `$HOME/work`, the workspace parent, `/opt`,
`/opt/hostedtoolcache`, `/opt/hostedtoolcache/go`, `/tmp`, `/dev`), which every
path walk visits, and by absent probes under the step's fresh tmpfs `$HOME` and
`TMPDIR` (`$HOME/.proverc`, `$HOME/go`, `$HOME/.cache/go-build`, test directories
the action removed and probed again). Two rules, both in `observeHost`; this
replaces the "other ancestors stay `host-read`" clause of §5.5(c).

- **(a) Ancestor nodes.** A *present, non-listing* observation of a PROPER
  ANCESTOR of an anchor is allowed. The anchors are the checkout `Root`, every
  declared host root, `Home`, `Tmpdir`, and the literal prefix of every
  `ephemeral` and `absent-ok` entry (for a pattern, the components before the
  first one containing `*` or `?`: `/dev/fd/*` gives `/dev/fd`). `/` is an
  ancestor. An anchor is not its own ancestor, and an ancestor that lies inside
  a declared root is judged by that root's rule, not this one (`prepare` builds
  the set by walking `parentOf` from each anchor).
  *Why it is safe.* Every anchor is canonical (`Root` is checked to be a real
  directory chain, §5.1; the others are declared paths), so each ancestor is a
  real directory by construction. A `stat`, `access`, `readlink`, `chdir` or
  `O_DIRECTORY` open learns only the node's type and metadata, which the model
  already treats as a non-input. A LISTING (`getdents`) still gives `host-read`:
  the sibling names are host state. An absent probe of a CHILD of an ancestor
  (`/home/go.work`, `/go.work`) is not an ancestor and stays refused, as does a
  present read of a sibling file of an ancestor; reads beneath an ancestor
  follow the existing rules.
- **(b) Absence under the fresh tmpfs.** An ABSENT probe of a path strictly
  inside the fresh `Home` or `Tmpdir` is allowed. *Why it is safe.* The tmpfs is
  empty when the step starts, so an absent name is a constant unless the action
  made it, and the trace knows when it did (`isCreated`, tested before
  `observeHost`; a name the action made, removed and probed again is the
  action's own). `underTmp` already excludes everything that could persist:
  `fresh` refuses the checkout and anything under it, every ancestor of the
  checkout (hosted runners put it under `$HOME`), and every declared root and
  ancestor of one. A PRESENT look at something the action did not make stays
  `host-read` (the state-preloaded void, §5.3, covers a pre-populated tmpfs).
  The `absent-ok` entries `$HOME/.config` and `$HOME/.npmrc` become redundant and
  are left in place.

**Hardening (review of D39).**

1. *The node must be a directory.* Only the checkout's own chain is verified to
   be real directories (§5.1, `prepare`). An ancestor of a declared root or a
   literal prefix could on some host be a symlink (`readlink` shows its target)
   or a regular file (an open and read shows its content). Rule (a) therefore
   also requires the node to be a directory on the post-run disk
   (`c.lookup(abs).kind == KindDir`); anything else falls through to the
   existing rules and gives `host-read`.
2. *No extended attributes.* `getxattr`, `lgetxattr`, `listxattr`,
   `llistxattr` (the parser tables them as metadata calls, as `stat`) are
   excluded from rule (a): an attribute value is content, not type or stat
   metadata. `classifier.xattrCall` is set per event and read only here; the
   census observer is unchanged. A walk to a path made by an xattr call
   observes its ancestors under that call, so it is refused conservatively.
3. *Rule (b) depends on the sandbox.* An absent probe under `Home` or `Tmpdir`
   is a constant only because the sandbox mounts a fresh tmpfs on both in every
   recording mode (`internal/sandbox/plan.go`; with a real home, the checkout's
   parent chain is recreated inside the tmpfs). It also needs every host path
   the sandbox binds beneath them to be a root traced declares (image, tool or
   state), because `fresh` excludes declared roots; the one other kind, a state
   root the recorder derives from an anchor, is verified empty
   (`stateRootEmpty`). `internal/record/sandboxroots_test.go` pins that the
   sandbox's read-only roots beneath `Home` and `Tmpdir` are a subset of the
   declared roots; comments at `observeHost` and `plan.go` name each other.

Rule (a) covers looks only, never write-class calls: a rename, rmdir, unlink,
chmod or utimensat of an ancestor stays `host-read` (D40 premise 4).

Not changed: a listing of any ancestor, a child probe of an ancestor, a present
uncreated name under `$HOME`/`TMPDIR`, and the rules of each declared root
(a tool root not keyed, an unpinned state root, an image root's ctime check).
`kiln:recorder` was 7 here (D40 made it 8). The differential fuzzer's oracle (`model_test.go`) has the
same two rules, so a record with no reason is still checked against the model.
Tests: `internal/traced/ancestor_test.go`.

**Decision D41: metadata writes through an anonymous descriptor (real-build
census, `fd-path` on dotnet's `fchmod(<fd>, mode) = 0`; `kiln:recorder` 8 to 9).**
`fchmod`, `fchown`, `fsetxattr` and `fremovexattr` name their object by the -y
annotation of the descriptor (`roleFdPath`), and an annotation that is not an
absolute path was refused as `fd-path`. A pipe, a socket or an anonymous inode
has the annotation `pipe:[N]`, `socket:[N]` or `anon_inode:NAME`, so csharp
actions were refused.

*The rule.* When the descriptor's annotation starts with one of those three
prefixes (the same list the classifier's `anonymousFD` uses, now
`trace.AnonymousFD`), the call yields no event and no error, whatever its result.
A bare number (no annotation), any other non-absolute annotation and AT_FDCWD
stay `fd-path`. An absolute annotation is unchanged.

*Why it is safe.* The mode, owner or extended attributes of a pipe, a socket or
an anonymous inode live outside every directory tree, so no file an action can
read by path changes. The annotation comes from the kernel (strace's -y) and is
the lexer's ANNOTATED token; a quoted string is another token kind, so a path
containing `socket:[` cannot fake it (and `/x/socket:[1]` starts with `/`). Pass 2
is shared, so both trace readers change together and the cross-check stays equal.

**Decision D44a: thread orphans stop counting as survivors (D44 follow-up; `kiln:recorder` 13 to 14).**
D44 accepts an orphan exit but leaves it without a `Procs` entry, and `DaemonLeft`
counts a missing entry as a possible survivor. So an orphan that is a THREAD of
the step's own process, whose exit line comes after that process's exit_group,
still refused the step as `daemon-left`: for the Go census case D44 only turned
`orphan-pid` into `daemon-left`.

*The rule.* Each dropped dying spawn now carries its kind on the exit record
that ended it (`record.spawnDropped`, a closed type shared by both readers
through `exitInsideCall`): `thread`, a clone whose flags argument, as strace
prints it, contains the token `CLONE_THREAD` (`analyzeClone` reads `flags=`,
`hasFlag` splits on `|`); or `process`, anything else a dropped call could have
started: fork, vfork, a clone without `CLONE_THREAD`, or a clone whose flags
cannot be read. (An unreadable or unknown-token clone is not dropped at all
today, `cloneArgsAllowed`; if that ever changed it would count as `process`.)
When EVERY dropped dying spawn in the trace is a `thread`, `findOrphans` lists
the orphans in `Trace.ThreadOrphans` and `DaemonLeft` leaves their exits out of
its survivor test. If ANY dropped spawn is a `process`, nothing changes: the
orphans stay possible survivors, as in D44. clone3 is refused-class and is never
dropped, so it never appears.

*Soundness.* No orphan is matched to a particular spawn. The claim is about all
of them: the orphans are accepted only while they number at most the dropped
spawns (D44), and every dropped spawn is a thread clone, so every orphan is
explained as a thread created by one of those clones. A thread cannot outlive
its thread group (when the last non-exited member leaves, the group is gone; a
group-wide exit kills every member). So a thread's exit line can show a
survivor only if its group survives, and a surviving group has other members
whose own exit lines carry `Procs` entries and are tested by `DaemonLeft` as
before: the thread that made the dropped clone belongs to that same group, and
the leader or any other member that survives still appears. A thread orphan
adds no process, read or write the model infers anything from, so no other
verdict changes. The accept set grows, so `RecorderVersion` is 14.

**Decision D46: `cwd-race` by overlap, not by existence of a sharer (`kiln:recorder` 15 to 16).**
The census shows `cwd-race` on `chdir` or `fchdir` in Ruby (two actions per
image) and Kotlin (one). The old rule refused any successful `chdir` or
`fchdir` while another live process shared the cwd (threads, `CLONE_FS`). That
is far stronger than the hazard: a shared cwd is only dangerous if some
sharer's cwd-dependent call could have run on either side of the change.

*The rule (`internal/trace/model.go`, `cwdOrdered`).* Every cwd cell
(`cwdCell`, shared by the processes of one `CLONE_FS` family; a fork child
copies it and is **not** a sharer) remembers `lastChange`, the completion line
of its last successful `chdir`/`fchdir`, and `lastDep`, the completion line of
its last cwd-dependent call. When a call on the cell completes at line `E`
having started at line `S` (`S == E` if uninterrupted; the split call's start
marker gives `S`):
- a call that is **cwd-dependent or a successful chdir/fchdir** is `cwd-race`
  if `lastChange > S`;
- a **successful chdir/fchdir** is `cwd-race` also if `lastDep > S`;
- otherwise the call records its `E` in `lastDep` (if dependent) and
  `lastChange` (if a successful change), and the cwd change is applied at `E`.

*Definitions.* A call is **cwd-dependent** when a path it resolves is relative
and unanchored (`path != ""`, not starting with `/`, and its dirfd path empty,
i.e. no dirfd argument or `AT_FDCWD`; `Path` and `Path2` are checked, each
against its own dirfd; a symlink target is content and never counts). Also
cwd-dependent: a successful `fork`, `vfork` or `clone`/`clone3` without
`CLONE_FS` (the child copies the cwd), and a successful helper-window
`unshare(CLONE_FS)` (it copies the cwd). Also cwd-dependent: `bind`, `connect`,
`sendto`, `sendmsg` and `sendmmsg` whose address is an `AF_UNIX` pathname that
is non-empty, not abstract (`@`) and relative (the classifier joins it with the
cwd); a `sendmsg`/`sendmmsg` counts once if any of its addresses qualifies,
and an absolute or abstract address does not. Also: **every `exec`**, whatever
its path, because an absolute script whose `#!` names a relative interpreter
resolves it against the cwd inside the same call. A relative `chdir` is dependent
whether it succeeds or fails; an absolute failed `chdir` and a failed
`fchdir` are neither dependent nor a change, so they never race. `fchdir`
takes its directory from the descriptor's absolute `-y` name, so it is a
change but not dependent. Absolute-path calls, calls under a real dirfd and
ignored-class calls never ask and never record.

*Soundness.* Two calls overlap iff their line intervals intersect; with a
later-completing call `B` and an earlier one `A` already seen, that is
`startB < endA`, which is what the two tests above ask (for `A` a change or a
dependent call, `lastChange` and `lastDep` hold the latest such `endA`; the
latest is enough because the question is "any `endA` after `startB`"). The
lines are the order in which strace handled ptrace stops. A call's effect
happens between its own entry stop and its own exit stop, and a
tracee is held at each stop until strace resumes it, so a call's effect lies
inside its interval of handling order; strace flushes a pending entry as
`<unfinished ...>` when any other line is printed, so an uninterrupted printed
call really had no printed stop of anyone inside it. Disjoint intervals are
therefore a true order, and with no overlap every sharer call is wholly before
or after every change, so a relative path resolves against the cwd the model
had at that line. Overlap is refused for all three hazards: a sharer's
relative call (which cwd did it use?), two changes (which was last?), and a
copying spawn (which cwd did the child get?). Absolute and dirfd-anchored
calls do not read the cwd, so they cannot be affected. The check is on every
cell whatever its `refs`, so a sharer that exited inside the chdir's interval
still counts (the old rule forgot it), which makes the rule slightly stricter
there. The cross-check stays equal because the rule is in the shared model.
`RecorderVersion` is bumped 15 to 16 because the verdict changes both ways.

*Split `unshare(CLONE_FS)` (`kiln:recorder` 19 to 20, D46a).* The first D46
text refused a SPLIT helper-window `unshare(CLONE_FS)` whenever another live
process shared the cwd, on the grounds that pass 1 emits no start marker for a
refused-class name, so its start line was unknown. That was a false positive,
and it made `kiln record prepare` fail at random in CI: the sandbox helper's
init thread calls `unshare(CLONE_FS)` while the Go runtime's other threads are
alive and share the cwd (`refs > 1`), and strace prints that call as
`unshare(CLONE_FS <unfinished ...>` / `<... unshare resumed>)  = 0` whenever
another thread's stop is handled between its two halves. That is a matter of
scheduling, so the `pipes` and `signals` structure probes (about 140 lines into
the trace, exactly where the helper unshares) were refused as `cwd-race` on
some runs and accepted on others. The start line was never unknown: pass 1
already holds it in the pending call. The resumed record now carries it
(`record.startLine`, set by both readers), and the unshare is checked like any
other cwd-dependent call over `[startLine, line]`: it is `cwd-race` only if a
sharer's chdir/fchdir completed after `startLine`. That is the same soundness
argument as above (disjoint intervals are a true order; the copy reads the cwd
inside its interval), so nothing is weakened: a sharer chdir completing inside
the unshare's interval is still refused, and a split record with no known start
and a sharer is still refused. The probe programs did not race their own cwd
(the init thread is `LockOSThread`ed and nothing else chdirs before the
unshare), so the rule, not the probe, was wrong, and no probe program changed.
The accept set grows, so `RecorderVersion` is bumped to 20 (17 and 19 are D48 steps 3 and 5; 18 is unused). Probe outputs are
not keyed separately: the probes run under the same recorder and
`pre/probes.json` is sealed with `pre/`, which is bound to `kiln:recorder`.
A sharer chdir that starts before the unshare's window and completes inside or
after it is refused (it overlaps), and `startLine == 0` is a defensive refusal
only. The refusal carries the call name (`Error.Call`), which `kiln record
prepare` prints for a probe refusal, and CI repeats the two probe-exercising
sandbox tests to make this flake class visible.

*AT_FDCWD with an empty path.* A dirfd slot that is `AT_FDCWD` with an EMPTY
path is cwd-dependent too: `statx(AT_FDCWD, "", AT_EMPTY_PATH, ...)`,
`newfstatat`, `fchownat`, `linkat` (either slot) and `execveat` act on the cwd
directory itself. Only an entry that has the dirfd slot counts, so a bare
empty `Path2` of an entry with no second dirfd does not; with a real dirfd the
empty path names that descriptor and is independent.

*Split helper `unshare(CLONE_FS)`.* Pass 1 emits no start record for
`unshare`, so a split one has no known start line; with another sharer on the
cell it is refused as `cwd-race`. An unsplit one is checked as a dependent
call.

*Census attribution.* The refusal is raised by whichever call completes
second, so `Error.Call` may now name the sharer's call (`openat`, `connect`,
...) under `cwd-race` rather than `chdir`/`fchdir`; the census files those
under `other`.

*Not covered (still refused if it overlaps, never accepted wrongly).* A
sharer's ignored-class call that reads the cwd for its own output (`getcwd`)
affects no path, so it is outside the rule.

**Decision D45: completed NULL-path `statx` failing with `EFAULT` (`home: runner` census run `44755244478`; `kiln:recorder` 12 to 13).**
On the first `home: runner` census run, 100% of Rust actions (8/8 on both
`ubuntu-24.04` and `ubuntu-26.04`) were refused with `path-arg` on Rust `std`'s
`statx` availability probe:
`statx(AT_FDCWD</...>, NULL, AT_STATX_SYNC_AS_STAT, STATX_ALL, ...) = -1 EFAULT (Bad address)`.

*The rule (`internal/trace/model.go`).* A completed `statx` call whose `pathname`
argument (arg 2) is printed literally as `NULL`, whose `flags` argument (arg 3)
is valid (`hasEmptyPath` returns no error) and does **not** carry
`AT_EMPTY_PATH` (by name or bit `0x1000` / `4096`), whose `dirfd` argument
(arg 1) passes `dirfdPath`, and whose result is a failure with errno `EFAULT`
(`!ret.OK && ret.Errno == "EFAULT"`), yields no event and no error.

*What stays refused.*
- `AT_EMPTY_PATH` (by name or bit `0x1000`, e.g. `0x1000` or `4096`) with a
  `NULL` path stays `path-arg`, and an unrecognised `flags` token stays
  `flags`.
- Any other outcome on `statx(..., NULL, ...)`, including `= 0`, `= -1 ENOENT`,
  or `= ?`, stays `path-arg` (or `result`).
- Any non-`NULL` non-string path argument (such as `0x1` or `0x7ffd...`), even
  when failing with `EFAULT`, stays `path-arg`.
- An unannotated or invalid `dirfd` stays `fd-path` or `dirfd`.
- Dying `statx` calls (`unresumed-call`, D35 / D44) stay unchanged (refused).

*Soundness.* In Linux (`fs/stat.c`, `SYSCALL_DEFINE5(statx, ...)` ->
`getname_uflags(filename, flags)` -> `getname_flags`), the kernel copies the
user `filename` pointer with `strncpy_from_user` before `do_statx` inspects
`dfd` or performs any path or file-descriptor lookup. A `NULL` pointer fails in
`getname_flags` with `-EFAULT` having touched neither the filesystem nor any
descriptor, and without writing to the `statx` buffer. Keeping `AT_EMPTY_PATH`
refused ensures no descriptor-based probe can slip through (fail-closed even
against non-standard kernels or crafted traces), and requiring completed
`= -1 EFAULT` ensures the kernel actually rejected the pointer before lookup.
Pass 2 (`model.call`) is shared by both trace readers, so the grammar path and
the oracle scanner stay identical with zero cross-check divergences.
`RecorderVersion` is bumped 12 to 13 because the accept set grows.

**Decision D49: G2 is a reported target, decoupled from PR 8 (user decision, 2026-11-24).**
G2 used to read: grammar parse time at most 10% of step wall time and at most 5 s
absolute, at the census p99 trace size, blocking before the old scanner is deleted
(PR 8). It is restated as a **reported target**: parse share at p50 at most 10%.
The 5 s limb is dropped, and G2 is removed from PR 8's preconditions (Phase B,
the 14-day / 200-run soak with 0 mismatches and 0 allowed divergences, and clean
nightly fuzzing all stay). Recorded here, and mirrored in the G2 bullet of 4.9.

*Why.* The grammar reader has been the production reader since PR 6, and the old
scanner is not a fallback, so deleting it changes no parse cost: G2 blocking PR 8
protected nothing. And the gate was not close. Runner census run `45345660988`
(shards 0 to 2, both images): G2 failed at p99 in all 6 shards (p99 trace about
211 MB, both limbs). The share limb fails broadly, not through outliers: parse rate
about 25.6 MB/s on ubuntu-24.04 (parse share p50 0.107) and about 41.6 MB/s on
ubuntu-26.04 (share p50 0.088); a Rust step emits about 17 MB of trace in about 6 s.
It is the ratio of trace rate to parse rate.

| trace size | parse ms p50 | parse share p50 |
|---|---|---|
| 8 MB | 626 | 0.107 |
| 16 MB | 698 | 0.104 |
| 32 MB | 1396 | 0.114 |
| 64 MB | 4212 | 0.152 |
| 128 MB | 10483 | 0.109 |

(ubuntu-24.04, shard 0.)

*What replaces it.* Parse speed is its own work item: a worker pool for the
stateless per-line lex/parse with the model pass kept sequential (the fallback
already named under G1). It is checked by an equality test against the sequential
reader, not by G2. The census keeps reporting the share p50/p90/p99, the
`parse_mb_per_s` distribution and `timing.g2_target` (`target_share_p50`, `share_p50`, `met`, `share_n`, `image_id`, `parse_mb_per_s_p50`; one census covers one image, so `image_id` labels the throughput); `g2_failing`
is annotated in the summary as reported, not a gate, and no census or CI job fails
on G2.

**Decision D50: two shapes the model refused that are provably safe (parser pass 2, `internal/trace/model.go`; `kiln:recorder` 19 to 21).**
Both rules live in the shared model, so both readers get them. Each was specified
by the reviewer and neither widens what the classifier believes beyond what is
argued here. The recorder version only goes up: 20 belongs to the open PR that
took it, and whichever change lands second renumbers.

*Rule 2b: a spawn that ended `= ? ERESTARTNOINTR`.* Accepted for `fork`, `vfork`
and `clone` (never `clone3`), whole-line or split (`clone(... <unfinished ...>`
then `<... clone resumed>...) = ? ERESTARTNOINTR (To be restarted)`). The result
must be `?` with the errno name exactly `ERESTARTNOINTR`, and a clone's flags
must pass the existing flag check (no `CLONE_UNTRACED`, no `CLONE_NEW*`, no
hex or unnamed token) and every token must be a name strace prints. It emits no
event, no spawn record, and is not a dropped spawn; it is NOT paired with the
call that follows. Soundness: `ERESTARTNOINTR` is the kernel's "restart this call
unconditionally" code, returned from `copy_process` when a signal became pending
before the new task was made visible, on a path that frees the half-built task. No
child exists and none ran. The kernel then re-executes the call itself, a
fresh line judged like any other, and a signal handler may run in between, so
pairing would hide a spawn that happened or invent one that did not. Still
`result`: any other `ERESTART*` name on these calls, `ERESTARTNOINTR` on a
call that is not a spawn, and a bare `= ?`.

*Rule 2c: a successful read-only `open` or `openat` with no descriptor
annotation.* Today `fd-path`. Accepted when the result is a plain non-negative
decimal descriptor with NO annotation at all (one that is present but does not
parse stays refused, as does a hex or pid-namespace-annotated result); the path
is a quoted string; the directory descriptor passes the existing rules; every
`|`-separated flag token is in the closed set `O_RDONLY, O_CLOEXEC, O_NONBLOCK,
O_NDELAY, O_DIRECTORY, O_NOFOLLOW, O_LARGEFILE, O_NOCTTY, O_NOATIME, O_PATH`
with no numeric residue; and `O_RDONLY` or `O_PATH` is present. The event is
emitted with an empty `ReturnFDPath`. Soundness: the annotation only lets the
classifier compare the walk with the kernel; without it the classifier walks
the named path and records the read against it under the strict stability rule,
as it already does for a descriptor that names a pipe or an anonymous inode
(`openCall` with a non-absolute return path, confirmed by `bareopen_test.go`:
a final symlink is followed as a stat follows it, a `/proc` magic link is still
`proc-path`, a path under the tmpfs is contained). The closed flag set rules out
any hidden write. Still `fd-path`: any write-capable, creating, truncating or
`O_TMPFILE` open, `openat2`, `creat`, and every LATER call on the descriptor
that lacks its own annotation (listings, `fchdir`).

*Evidence.* Both rules count their firings (`Trace.RestartedSpawns`,
`Trace.BareOpens`), summed into the census (`parse_restarted_spawns`,
`parse_bare_opens` in each action's census file, `parse_rules` in the shard
summary, a "Parse rules" section in its markdown), so the next census shows
whether they matter. Tests: `d50_*_test.go` (accept and neighbour-refuse shapes,
a biased twin differential, mutants that must be killed),
`bareopen_*_test.go` (classifier and the biased read-set oracle), and the real
kernel test `TestSandboxStraceRestartedSpawn`, which CI runs with `-count=20`
next to the signal and dying-thread tests: a refusal rule that passes once is not
evidence (the D46 flake).

*Observed on real kernels, and the pinned-strace experiment.* Both runner images
run kernel 7.0 but ship different strace: 6.8 on ubuntu-24.04, 6.19 on
ubuntu-26.04. About a quarter of Go actions per census run are refused as
`unresumed-call` on 24.04 and none on 26.04. The hypothesis is that 6.19 prints a
thread killed at its syscall-entry stop in the "never ran" form the parser
already accepts. To test it without changing the default, the census workflow
(`.github/workflows/kiln-census.yml`) has a dispatch-only input `strace`
(`system`, the default, or `6.19`). `6.19` builds strace from the official
release tarball (SHA-256 pinned in the workflow), installs it under
`$RUNNER_TEMP`, and passes it to every `kiln record prepare` as `--strace`; that
path meets prepare's placement rules (canonical, a regular file, outside the
checkout, the step TMPDIR, every state root and the record directory) for the
same reason the kiln and build-tool binaries do. Nothing in Kiln reads the
strace version except to record it in the binding and the census "Host" line;
its path is sealed in `pre/tools.json`. Such a run says so in every shard's summary and is NOT PR8
evidence; the scheduled run is always `system`.

**Decision D48: exec heads recorded at exec time (`kiln:recorder` bumps at step 3 for the filter and at step 5 for consumption; design adopted from the spike, `specs/KILN04-exec-head-spike.md`).**
*This decision is the design. Nothing in it changes what the classifier believes until step 5 of the plan below.*

*The problem.* Every Go test action is refused as
`trace-incomplete:exec-unreadable:under-tmp`. `go test` builds `pkg.test` under the
step's fresh tmpfs (`$TMPDIR/go-build*/b001/`), runs it, and deletes it. The
classifier must read the head of every exec'd file to walk the loading chain
(`loadChain`, `internal/traced/exec.go:55`: a `#!` interpreter, an ELF's
PT_INTERP), because the kernel opens those files with no call strace can show.
`readHead` (`exec.go:111`) takes the head from the tracked tree or the post-run
disk, and returns `false` first thing for `underTmp(abs)` (`resolve.go:210`: the
fresh `$HOME`, `$TMPDIR` and scratch directories), since that is a tmpfs that is
gone when `collect` reads it. `loadChain` then raises
`reasonSub(rExecUnreadable, c.headWhere(abs), abs)` (`exec.go:59`; `headWhere`
answers `"under-tmp"`, `exec.go:133`). A trace-only rule cannot replace the head:
the loader is a link-time choice (`-Wl,--dynamic-linker`) and a custom loader can
imitate a static start. Only the bytes the kernel loaded are evidence.

*The mechanism.* A second seccomp filter, installed with
`seccomp(SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_NEW_LISTENER, ...)` by the
sandbox init next to the existing one (`installSeccomp`,
`sandbox_linux_amd64.go:706`, called at `:333` after `dropPrivileges`, which has
set `no_new_privs`). It returns `SECCOMP_RET_USER_NOTIF` for `execve` and
`execveat` and `SECCOMP_RET_ALLOW` for everything else. It has its own
architecture test and KILLs a non-x86_64 call exactly as the first filter does
(`bpf.go:183-185, 200`), so an `execve` number of another ABI can never be
notified as an x86_64 one. The kernel rule (seccomp(2)) is that every filter runs
and the most restrictive verdict wins (KILL, TRAP, ERRNO, USER_NOTIF, TRACE, LOG,
ALLOW). So the first filter keeps its meaning: its EPERM list (`RefusedSyscalls`,
`bpf.go:72`, and the refused `clone` flags, `CloneRefusedSet`, `bpf.go:145`), its
`clone3` and above-ceiling ENOSYS, and its non-x86_64 and x32 KILL all still beat
USER_NOTIF, and `execve`, which it allows, is now notified. Only one filter in a
task's stack may hold a listener (EBUSY otherwise); the first has none.

*Getting the listener out.* The supervisor must be outside the sandbox (a
filtered process cannot supervise itself) and outside the trace. It is `kiln
record step`, which already owns the trace files and the step sidecar and whose
death the recorder already treats as a failed recording. The chain is `kiln record
step` -> strace -> `kiln sandbox-exec` (the root) -> `kiln sandbox-init` -> the
step. The listener descriptor reaches the supervisor over a dedicated inherited
unix socketpair, not over the liveness pipe and not through the step's output
descriptors:

1. `kiln record step` creates an `AF_UNIX`/`SOCK_STREAM` socketpair next to its
   five pipes (`runStrace`, `record/step.go:846`) and passes the child end as a
   fourth `ExtraFiles` entry (`step.go:879`; strace's descriptor 6, after the
   stdout, stderr and trace descriptors 3, 4, 5). strace leaves it inherited and
   it reaches the root. The argv gains `--exec-notify-fd 6`.
2. A new `sandbox-exec` option names that descriptor. (The brief called it "a
   config field". The per-run descriptor numbers live in the parsed arguments,
   `--stdout-fd` and `--stderr-fd` in `entry.go:92-99`, not in the sealed
   configuration whose digest the root verifies, so the option goes there; the
   number is fixed by `kiln record step`.) The root validates it before it spawns
   anything, as it validates the output descriptors (`inspectOutputFD`,
   `sandbox_linux_amd64.go:89, 204`), but NOT with `CheckOutputFD` (`entry.go:173`,
   which demands a write-only pipe and refuses a socket by name) and NOT through
   the liveness pipe (`checkPipeAlive`, `:592`, which reads EOF as "the root is
   gone"). The check is its own: the number is in the same range (3 to
   `maxOutputFD`), `fstat` says `S_IFSOCK`, `getsockopt(SO_DOMAIN)` is `AF_UNIX`,
   `SO_TYPE` is `SOCK_STREAM`, `getpeername` succeeds (it is connected), and it is
   not one of the output descriptors. A configured-but-missing or non-socket
   descriptor refuses the step before anything is spawned (`ExitFailure`, as for a
   bad output descriptor).
3. `RunExec` marks every descriptor from 3 up close-on-exec before the spawn
   (`:136`), so the descriptor reaches the init only by the spawn's explicit
   mapping, appended to `Files` (`:164`) after the liveness pipe: the init sees it
   as descriptor 4. The root closes its own copy right after `ForkExec`, as it
   does `alive[0]` (`:176`), so the init is the only holder left on this side.
4. `RunInit` installs the listener filter (after the first one, before
   `redirectStdinAndCloseFDs`, `:336`), takes the listener descriptor from the
   `seccomp` call, sends it with `sendmsg`/`SCM_RIGHTS` over descriptor 4, and
   then closes the listener and the socket. `redirectStdinAndCloseFDs` already
   marks every descriptor from 3 up close-on-exec (`:729-754`), but these are
   closed, not left to that: THE STEP HOLDS NO SOCKET AND NO LISTENER (a step
   holding either could answer its own notifications). The init's own `execve` of
   the step (`execStep`, `:343`) is the first notified call, so the supervisor
   must already be receiving when the send happens; a unix stream socket's buffer
   takes the one message without blocking.

*The supervisor, per notification.* `kiln record step` receives the listener
(`recvmsg`, `SCM_RIGHTS`) and runs a loop of `SECCOMP_IOCTL_NOTIF_RECV`. For each
notification it does, in order (all of it cross-user-namespace access by a process
that owns that namespace, on the evidence below):

1. Record `tid` (the notification's thread, in the supervisor's own pid
   namespace, which is strace's, so the number the trace prints) and `tgid` (from
   `/proc/<tid>/status`, read now; the thread may be gone later). Call
   `SECCOMP_IOCTL_NOTIF_ID_VALID` IMMEDIATELY after reading the `tgid`, before
   anything else uses it, and take the PIDFD (`pidfd_open`) from that point, so a
   `tgid` read from a recycled pid can never be used. ONE EXEC PER
   THREAD GROUP AT A TIME: a second notification for a `tgid` that has one pending
   is queued, not answered, until the first is decided (a confirm could otherwise
   be satisfied by the wrong exec; `/proc/<pid>/exe` belongs to the process).
   "Decided" means the first exec is DEFINITELY over: its confirm, or its thread
   definitely out of `execve` (the `failed` reading below), or the group exited
   (the pidfd is readable). It is never a timer that RELEASES a held notification untainted. A different-tid
   notification queued within the confirm deadline is held until the pending exec
   is decided; if the deadline passes first, the TAINT branch below is applied to
   every queued notification at that moment (the group is tainted, the pending
   exec is finalized `timeout`, and each queued notification is answered with a
   `tainted` line); the queue is not left holding. An ABSOLUTE bound on a pending
   exec's lifetime (a constant of step 4, far above the confirm deadline, measured
   from the exec's CONTINUE) ends in an unclean step (`exechead-stuck`, the step
   killed): an exec stuck in the kernel for that long is a stuck step. The
   STALL clock (below) starts when a notification is DEQUEUED for processing, not
   when it was received, so a held notification is not a stall. THE BACKSTOP IS
   TAINT. Each
   thread group has a TAINTED state and a LAST-EXEC record: the tid of its most
   recent exec that was released WITHOUT a definite outcome (`outside`,
   `unpinned`, `fexecve`, `same-image`, `overload`, and a pinned exec still being
   polled or past its deadline) and, for a pinned one, its pin, its pre-exec
   identity and its state. THE FACT THE RULE RESTS ON: a thread that is making an
   exec notification cannot still be inside an earlier `execve`, so a new
   notification from the SAME tid as the group's last undecided exec proves that
   exec is over: it failed and returned, or it succeeded and this is the new
   image. The rule, per thread group, at dequeue:

   - SAME tid as the last undecided exec: do NOT taint, and the notification
     BYPASSES the group queue (liveness: queued behind a different-tid C that is
     itself behind the pending A, a same-tid B could never settle A, because
     `/proc/<tid>/syscall` reads `execve` while the thread is held in B, and the
     step would die `exechead-stuck`). Settle A at receipt, answer B, and keep C
     queued behind B. If that exec was a pinned
     one still pending, SETTLE it now, before answering the new notification. The
     thread is held in the new notification, so the image cannot change under the
     reads. `exe` equal to the pin and different from the recorded pre-exec
     identity: read the head from the pinned descriptor, re-check `exe`, write
     `confirmed` (checking the group's taint under the group lock, exactly as
     the poller's own write does). `exe` unchanged: `failed`. `exe` changed to anything else:
     `mismatch`. (A pinned exec whose group has been tainted meanwhile was already
     finalized by the taint, and is not settled.) Then handle the new notification
     normally, taking its pre-exec identity fresh.
   - DIFFERENT tid: an overlap is possible. A pinned exec still within its
     deadline holds the new notification in the queue, as above. Otherwise (a
     pollerless exec, or a pinned one past its deadline) the group is TAINTED, the
     pending pinned exec is finalized as `timeout` (never `confirmed`), and the
     new notification gets `tainted`.

   Why this is sound: taint guards only against two execs of one group in flight
   at once, and that needs two threads, so it needs a notification from a second
   tid. A non-leader whose exec succeeds reappears with the leader's tid, which
   differs from its own, so it taints (and non-leader lines stay refused in
   pairing, so that case is covered twice). What it buys: in-place exec chains
   (`sh -c ./test.bin`, `env`, `timeout`, `nice`; dash execs the last command in
   the same pid) stay untainted, and a fast chain that the poller would have lost
   to `timeout` or `gone` still confirms, because the next same-thread
   notification is itself the synchronisation point. (Plain `go test` forks a child
   whose first exec is the tmpfs binary, so it never needed this; the rule matters
   for in-place wrappers.) The poller is kept for the case where no further
   notification comes from that tid, and it and the same-tid settle must not
   both write a line: each pinned exec has THREE states, `pending`, `claimed` and
   `written`. Whichever of the poller and the settle first moves it from
   `pending` to `claimed`, under the group lock, owns the line; the loser does not
   write. A settle that finds the exec `claimed` WAITS for it to become `written`
   (the poller's remaining work is one bounded read and one write) before it
   answers B, so the lines of one thread group stay in order and B is never
   answered ahead of A's line. (The alternative, reserving A's `seq` slot at arrival
   and filling it later, is not used: the file would hold a gap until the fill.) A
   poller that reaches the deadline writes no line by itself: it marks the exec
   past its deadline and keeps polling, so a same-tid settle can still confirm;
   the `timeout` line is written when a different-tid notification forces the
   taint, or the step ends or the pending-lifetime bound kills it. `overload`
   taints REGARDLESS of tid (a same-tid notification can hit the global cap too:
   the pending exec it would have settled is still unobserved, so the cap's
   answer, CONTINUE with no pin, is itself an unobserved exec). Taint happens
   ONLY on a different-tid notification (as above), on `overload`, or on an exec
   whose thread vanished with `exe` unchanged; there is NO release at the
   deadline. Vanished means: the notified
   thread vanished from `/proc` while the group lives AND `exe` has not changed
   (`de_thread` does that when a sibling's exec succeeds). A thread
   that vanished because ITS OWN exec succeeded (a non-leader's exec takes over
   the leader's pid; `exe` changed to the pin) is not taint. A queued notification
   that was never released and whose thread dies in `de_thread` is simply DROPPED
   when `NOTIF_ID_VALID` fails at dequeue, and does not taint (it never ran).
   Non-leader execs stay refused for pairing, as below; this only fixes the taint
   rule. Every pending and every later line of a tainted `tgid` gets
   the status `tainted` (or the triggering status, for the line that triggered it)
   and is NEVER `confirmed`, because an overlapping exec could turn `exe` into the
   pin and falsely confirm another line. Taint alone is the minimum; the held queue
   is what keeps taint rare. The
   RECEIVE loop never waits for a confirm: it only does steps 1 to 6 (bounded
   work, a few system calls) and hands the confirm to a per-notification poller, so
   one slow confirm cannot hold the next notification of ANOTHER thread group. The
   STALL deadline (see "Fail closed") is enforced by a separate watchdog goroutine
   that kills the step, because a Go system call such as `process_vm_readv` or
   `openat2` cannot be cancelled and moving the work off the receive loop would not
   make it cancellable.
   The pollers are bounded (a cap on pending confirms, which is also a cap on the
   descriptors held: one pin and one pidfd each, and on goroutines), and they poll
   with backoff (from 50 microseconds, doubling, to 5 milliseconds; a poller PAST the
   confirm deadline backs off to about 50 milliseconds, and leaves the global confirm
   cap for a separate lingering pool of at most one entry per group, so one group's
   stuck exec cannot push other groups into `overload`). Past the cap, or
   past a cap on queued same-group notifications, a notification is answered
   CONTINUE without a pin, status `overload`, AND its `tgid` is tainted (the
   answered exec overlaps the pending one). The queue caps are PER GROUP and
   separate from the global confirm cap: a group's queued notifications hold no
   poller and no descriptor, only memory under that group's own cap, so one step's
   threads cannot push other groups into `overload`; the global cap bounds only the
   ACTIVE confirm of each group. The supervisor never stops answering.
   TAINT STATE is keyed by `(tgid, pidfd)` and dropped when the pidfd becomes
   readable (the group is gone, the number may be reused). The table is bounded;
   on overflow a GLOBAL overload-all taint is set (every later line of every group
   is `tainted`) instead of evicting an entry, since an evicted entry would
   un-taint a group. An `overload` taints even when the caps are hit before the pidfd
   is open: the `tgid` is recorded first, as a pidfd-less entry. ORDERING: the taint
   is set BEFORE the `overload` CONTINUE is sent; a poller reads the taint AFTER its
   final `exe` re-check (step 8), under the same lock as the line write, so a line
   is never written `confirmed` after a taint that an overlapping exec could have
   caused.
2. Read the path string from the tracee, `process_vm_readv` at `args[0]` (`args[1]`
   for `execveat`), at most `PATH_MAX`, up to the NUL. This is the string
   recorded as `path`; it is evidence of what was asked, never trusted as what
   ran. A partial `process_vm_readv`, `EFAULT`, or no NUL within `PATH_MAX` is
   `unpinned`. For `execveat`, `dirfd` is compared as a 32-bit `int` (the kernel
   ignores the high bits), `AT_FDCWD` (-100) means the cwd, and with
   `AT_SYMLINK_NOFOLLOW` a path whose final component is a symlink is `unpinned`
   (the kernel would exec nothing the pin could follow). An `execveat` with
   `AT_EMPTY_PATH` and an empty string is `fexecve`: no
   path to read. REFUSED in v1: status `fexecve`, nothing pinned, continue. (The
   probe shows `/proc/<pid>/fd/<n>` does open from outside, `open_proc_fd=ok`, so
   a later version could pin it.)
3. Resolve the path inside the step's root. `openat2` has ONE directory
   descriptor, which is both the base and, with `RESOLVE_IN_ROOT`, the root, so the
   two roles are split in two cases. An ABSOLUTE path (or an absolute-looking
   result): `openat2(rootfd, path, RESOLVE_IN_ROOT|RESOLVE_NO_MAGICLINKS)` with
   `rootfd` = `/proc/<tgid>/root` opened as a directory. A RELATIVE path (against
   the cwd, `/proc/<tid>/cwd`, or for `execveat` the `dirfd` argument,
   `/proc/<tid>/fd/<n>`): open that base directory as a descriptor (the link
   itself is the step's real directory), then `openat2(basefd, path,
   RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS)`. `RESOLVE_BENEATH` refuses a `..` that
   climbs above the base and an absolute symlink, so a relative path that leaves
   its base is `unpinned`, never resolved with a wrong root: conservative, and a
   test binary exec'd by its absolute path is unaffected. `RESOLVE_NO_MAGICLINKS`
   makes `/proc/<n>/fd/...` and `/proc/self/exe` `unpinned`. `RESOLVE_NO_XDEV` is
   NOT used: the step's `/tmp` and `$HOME` are mounts of their own, and a legal
   path crosses into them. A plain `/proc/<pid>/root/<path>` string would resolve
   an absolute symlink in the SUPERVISOR's root, not the step's.
4. If the resolved file is not on the step's fresh tmpfs (its `st_dev` is not one
   of the devices of `$HOME`, `$TMPDIR` or the scratch directories of the sealed
   plan, which `kiln record step` measures through `/proc/<tgid>/root`; a device it
   could not measure counts as "on the tmpfs"), nothing is pinned: write an
   `outside` placeholder line and CONTINUE. The classifier needs a head only for
   the `underTmp` case, so a host binary costs what it costs today plus the round
   trip.
5. Otherwise PIN: the same `openat2` with `O_RDONLY|O_NONBLOCK|O_CLOEXEC` (the
   `O_NONBLOCK` so a FIFO or a device node cannot stall the supervisor, and the
   whole notification has a deadline), `fstat` it, REQUIRE a regular file, and
   record `(dev, ino)`. Missing, unreadable or not a regular file is `unpinned`:
   write the line and CONTINUE (the kernel will fail or run it; the trace says
   which).
6. Use the PIDFD taken in step 1, before the answer: it names
   this process for ever, so the later `gone` check (the pidfd becomes readable
   when the process exits) and any pid reuse are decided on the descriptor, never
   on a number, and every read of `/proc/<tgid>` below is bracketed by a pidfd
   check. Then record the PRE-EXEC IDENTITY: `stat("/proc/<tgid>/exe")`'s
   `(dev, ino)`, taken now, before the answer (for a notification that was
   queued, at DEQUEUE time, after the previous exec of the group is decided; a
   pre-exec identity read earlier would be the previous image). This transition rule is
   DEFENCE IN DEPTH: the basis of the argument is the order of the reads (the
   syscall first, `exe` after) and the held queue; the identity check catches what
   those miss. If it already equals the pin, the process is running
   the very file it is about to exec (a self re-exec), and the confirm below could
   not tell the exec from nothing happening: status `same-image`, write the line
   and CONTINUE without a confirm (see "the confirm needs an observed
   transition"). Then `SECCOMP_IOCTL_NOTIF_ID_VALID`, then
   `SECCOMP_IOCTL_NOTIF_SEND` with
   `SECCOMP_USER_NOTIF_FLAG_CONTINUE`. CONTINUE is explicitly NOT a security
   decision: the kernel re-reads the path string from the tracee afterwards, so
   another thread of the step may have rewritten it, or swapped a symlink, and
   the kernel then runs a file the supervisor did not pin. Nothing about the pin
   gates the exec; the confirm below is the decision.
7. CONFIRM an OBSERVED TRANSITION: poll `stat("/proc/<tgid>/exe")` (the
   thread-group id, never the tid: a non-leader that execs takes over the
   leader's pid and its own tid vanishes) until its `(dev, ino)` differs from the
   pre-exec identity. If it has changed to the pinned `(dev, ino)`, that is the
   confirm. If it has changed to anything else, that is `mismatch`, decided at
   once, without waiting for the deadline. If it never changes within the
   deadline, the exec is marked PAST-DEADLINE (no line yet: a same-tid
   notification can still settle it); `timeout` is written when the group is
   tainted or at step end. A match with no transition is never a confirm. The held
   descriptor makes the comparison meaningful: the inode cannot be freed and its
   number reused while it is open.

   *Why the transition is required (the false confirm it closes).* Suppose the
   process already runs tmpfs binary F (a Go test binary's forked child, before it
   re-execs itself). One thread calls `execve` with a path string that strace and
   the supervisor read as "F"; the supervisor pins F and CONTINUEs. If the confirm
   were "`exe` equals the pin", it would pass at once, because `exe` is already F.
   Meanwhile another thread swaps the string to G before the kernel reads it, and
   the kernel execs G. The same holds if the swapper is another PROCESS that
   renames a file Y over the path after the CONTINUE: the kernel's own lookup then
   finds Y. The line would say `confirmed` with F's head, and the trace path text
   is also "F", so the path-equality check would pass too: a false confirm. With the transition rule the pre-exec identity F equals the pin F, so
   the exec is `same-image` and refused. *Soundness.* One exec is in flight per
   thread group (step 1), and `exe` cannot be re-pointed by the step (the
   `PR_SET_MM_MAP` argument below, which rests on the valid-map probe), so the only
   thing that can change `exe` from a pre-exec identity A to the pin B, with
   `A != B`, is an exec that loaded B. A swap to G makes `exe` become G, not B,
   which is `mismatch`; the rename variant is the same case, since whatever the
   kernel resolved, the link can only become B if B was loaded. (An exec of B by a thread that was not the notified one
   cannot occur inside the window: execs of that thread group are serialised by
   step 1, and each exec's notification precedes its kernel read.)
8. Only after the confirm, read the head from the PINNED descriptor with `pread`
   (`headLimit` bytes, 64 KiB, `traced/index.go:54`), then `stat`
   `/proc/<tgid>/exe` again. The head counts only if the process still has that
   image. Write the line.

The deadlines (the confirm's, and a longer one for a notification the supervisor
has not answered at all) are constants of step 4, chosen from the probe's
`confirm=exe-match-after-12us` to `1185us` with a wide margin for a loaded runner.

*Refusal statuses.* A notification that is not `confirmed` or `outside` writes a
line with one of these, and the exec is still CONTINUEd: the supervisor never
blocks a build, it only declines to vouch.

- `gone`: the process exited (or `/proc/<tgid>` vanished) before the confirm. A
  file no process holds any more could have been rewritten after the exit, so no
  head is vouched for.
- `same-image`: the pre-exec identity of `/proc/<tgid>/exe` already equals the
  pin (step 6). The exec is not observable as a transition, so no head is vouched
  for. COST, stated plainly: a process that re-execs its own tmpfs binary stays
  refused, and Go tests that spawn themselves do this; the census sub-code
  `exechead-same-image` measures how many. A later version could confirm against
  strace's own result line for that thread instead of the `exe` link.
- `mismatch`: `exe` changed to something other than the pin (decided as soon as
  the link changes, without waiting for the deadline), or stopped being the pin by
  the re-check after the head read. A `#!` script under the tmpfs is a `mismatch`
  (or `gone`, if its process was short): the notification is for the script's
  `execve` only and `exe` becomes the interpreter, so the script's head is not
  confirmed. Scripts under the tmpfs stay refused, as today. A script whose
  interpreter is a tracked or host file is not an `under-tmp` case and is
  unaffected. THE LIMIT of that claim: if the pinned file B is an ELF and another
  thread swaps in a script whose shebang is `#!B`, `exe` becomes B and the line
  confirms with B's head, although the kernel first read the script's head. The
  loader chain recorded is still B's, which is what the classifier would walk, so
  this is not a read-set hole, only a misattribution of one head. An additional
  check that `/proc/<tgid>/cmdline` argv0 or `AT_EXECFN` equals the traced path is
  NOT adopted in v1: both live in memory the process itself can rewrite, so they
  are not evidence.
- `failed`: the thread has left `execve` and `exe` is still the pre-exec
  identity: the exec failed (EACCES, ENOEXEC, ETXTBSY, ...) and the process runs on.
  Detected without the deadline: each poll reads `/proc/<tid>/syscall` FIRST (the
  thread's current system call number; readable by the supervisor on the same
  terms as `/proc/<tid>/mem`) and `exe` AFTER. `exe` changes before a successful
  exec returns to user mode, so a thread seen outside `execve` (and
  `execveat`) whose `exe` is still the old image has failed for good. "Outside
  `execve`" is concluded ONLY from a definite reading of a different system call
  number, or of a blocked state in a call that is not `execve`/`execveat`.
  `/proc/<tid>/syscall` reports `running` whenever the task is not blocked, and
  that includes while it is inside `execve`; so `running`, or a file that cannot be
  read, means "not decided, keep polling", never `failed`. The line is written at
  once and the thread group's queue is released. A file that cannot be read
  (`ENOENT`, `EACCES`, `EPERM`, any error) is "not decided" too: with a live pidfd
  it is polled again (the exec is past-deadline after the deadline, which does not
  taint); with the pidfd readable it is `gone`. A `failed` line is never believed,
  and a wrong `failed` is caught by the same-tid settle: when the thread's next
  notification arrives, the settle compares `exe` with the failed exec's recorded
  pre-exec identity, and if `exe` has changed in between (the first exec was still
  in flight after all) that next line is `tainted`.
- `timeout`: the deadline (the CONFIRM deadline, a constant of step 4, well under
  the stall deadline) passed while the thread was still IN `execve` and `exe` had
  not changed, and no same-tid notification settled it. The line is written when
  a different-tid notification taints the group, or when the step ends or the
  pending-lifetime bound kills it (step 1); it releases nothing untainted. This is distinct from the STALL deadline, which measures the
  supervisor itself: the time from a notification's `RECV` to its `SEND`, and the
  gap between loop iterations. `SEND` (CONTINUE) happens before the confirm starts,
  so a confirm's wait never counts against the stall deadline, and a stall means
  the supervisor is stuck, not the step.
- `overload`: the pending caps of step 1 were exceeded; nothing was pinned, and
  the thread group is tainted.
- `tainted`: the thread group is tainted (step 1); the line is never confirmed.
- `unpinned`: step 5 failed.
- `fexecve`: step 2.

*Release triggers.* When each line's status lets the thread group's next queued
notification go (the group's queue is released, never by a timer):

| status | released | taints the group |
| --- | --- | --- |
| `confirmed` | at the confirm (after the step-8 re-check) | no |
| `mismatch` | when `exe` changed to something else | no |
| `failed` | when the thread is definitely out of `execve` | no |
| `gone` | when the pidfd is readable | the entry is dropped |
| `timeout` | NOT at the deadline: the poller keeps polling (about every 50 ms) until the exec is decided (one of the four above), or a same-tid notification settles it; a different-tid notification taints the group and writes it; the pending-lifetime bound kills the step | no, until a different-tid notification arrives |
| `outside`, `unpinned`, `fexecve`, `same-image`, `overload`, `tainted` | at once (they have no poller); the tid is remembered as the group's last undecided exec | `overload` and `tainted`: yes. The others: only if a DIFFERENT tid's notification follows; a notification from the SAME tid does not taint |

Releasing the pollerless statuses at once, and tainting only when a second tid
shows up, is the preferred option: those lines never observe the exec's end, so
only a notification from another thread could overlap an exec still in flight,
and the same-tid fact settles every in-place chain without a poller (holding the
queue behind an exec nobody polls would need a poller for every host-binary exec,
the common case). The cost is a refusal (`exechead-tainted`) only for a group in
which a second thread execs while another's exec is undecided. The N5 pairing refusal (unfinished, killed and result-less exec
events) is the backstop for a group whose overlapping exec dies in `de_thread`: the
trace shows that exec as `+++ killed` or unresumed, and the pid is refused.

*The sidecar `<trace>.exechead`.* Written only by the supervisor, beside the trace
(`StepFile`, `record/stepfile.go:132`), created exclusively (`writeExcl`) before
the step starts, in a directory the sandboxed step cannot see. JSON Lines. The
first line is a HEADER:

    {"v":1,"kernel":"<uname release>","nonce":"<32 hex>","action_sha256":"<hex>","step":N}

`nonce` is 16 random bytes `kiln record step` draws per step and also stores in
the step sidecar, and `action_sha256` and `step` name the step the file belongs
to: a file from another step or action does not bind. Then one line per
notification, in arrival order, with a monotonic `seq` from 0:

    {"seq":N,"tid":N,"tgid":N,"syscall":"execve"|"execveat","path":"...",
     "status":"...","dev":D,"ino":I,"head":"<base64>","sha256":"<hex of the head bytes>"}

The parser is strict: an unknown key, a duplicated key, a status outside the enum
(`confirmed`, `outside`, `gone`, `mismatch`, `timeout`, `unpinned`, `fexecve`,
`same-image`, `failed`, `overload`, `tainted`), a line longer than the per-line cap (the
`head` is capped at `headLimit`, so a line is at most about 90 KiB), or a `seq`
that is not the previous plus one refuses the whole action; duplicates and gaps
are detected by `seq`, not by comparing text. The per-line `sha256` is
SELF-ATTESTED: it detects a corrupted line, not a forged one. The binding is the
digest of the whole file in the step sidecar, together with the nonce in the
header, written by the same process after the step.

The pre-exec identity is held by the supervisor in memory only, NOT written
(`pre_dev`/`pre_ino` are not fields): the classifier cannot verify it, the
verdict it feeds (`confirmed`, `same-image`, `mismatch`) is already the line's
`status`, and a field the reader would have to trust but cannot check adds a way
to be wrong without a way to be caught.

`dev`, `ino`, `head` and `sha256` are present only on `confirmed` lines; every
other status carries the first five fields only (`outside` and the refusals are
the placeholders that keep the count honest). `path` is a JSON string; bytes that
are not UTF-8 make the line `unpinned` rather than being mangled. The file is
size-bounded (a fixed cap on lines and on bytes, the trace's own limit as the
model). When the cap is passed the supervisor does NOT stop: it keeps answering
every notification CONTINUE without pinning, writes no more lines, marks the step
unclean (`exechead-overflow`) and the action is refused. The file is fsynced before
`kiln record step` writes the step sidecar. Its SHA-256 and length are new fields
of the step sidecar (`exechead_sha256`, `exechead_bytes`, with `StepSchema`
`kiln-record-step/2` becoming `/3`, `stepfile.go:43`; compare `TraceBytes` and
`TraceSHA256`, `:106-110`), so `collect` can see that the file it reads is the one
the supervisor wrote. A malformed, duplicated (the same line twice, or a second
file), oversized or digest-mismatched file refuses the WHOLE action
(`trace-incomplete`), never "no sidecar".

*Classifier consumption.* Applies only to a SUCCESSFUL exec (the trace's result,
never the supervisor's: a failed exec of a file the process already runs looks
confirmed) whose target is `underTmp`. Everything else is as today.

- Pairing. The nth exec event of a pid in the trace pairs with the nth sidecar
  line for that `tgid` (strace prints the leader's pid for a process, which is the
  `tgid`). Failed execs count on BOTH sides (a PATH search is several
  `execve` calls that fail with ENOENT, each notified and each given a placeholder
  line), as do `outside` lines. For every pid other than the step's own the counts
  must be EQUAL, or the pairing refuses. NON-LEADER EXECS are refused outright: a
  `tgid` with any line whose `tid` differs from the `tgid` is `exechead-pair` for
  that pid, because mapping a non-leader tid to the pid strace prints would trust
  an attribution nothing here verifies, while a refusal costs only the rare
  non-leader exec (Go starts its children from a forked child, not by exec from a
  runtime thread). The step's own pid is anchored at the trace parser's
  own recognition of the step's exec: the listener is installed only just before
  the init's `execve` of the step, and the parser already drops the init's setup
  execs of the Kiln binary (`windowFilter`, `trace/model.go:1045`: a re-exec of
  the helper is setup and is dropped; a failed exec of anything else is a real
  observation and is kept; the first successful exec of anything else is the step,
  `stepExec`, `:1073`). So the init pid's events the classifier receives (the
  failed probes the step's PATH search makes, then the step's exec) pair one for
  one with that tid's lines from the first line, however many setup execs the init
  makes, and the count must be equal there too. The anchor is the only rule for
  the step's pid: nothing is skipped or tolerated, and there is no "leading
  unmatched event" allowance. The anchor itself is trusted only if the setup execs
  the parser dropped were of the recorded Kiln helper (`isHelperExe`). A refusal
  here is the `exechead-pair` sub-code. A pid whose trace has an exec event with no
  result (strace's `= ?`), an `<unfinished ...>` exec with no resumption, or an exec
  cut by `+++ killed` is refused outright (`exechead-pair`): the pairing cannot say
  whether that exec reached the kernel's loader. A pid with a non-confirmed
  `overload`, `timeout`, `gone` or `tainted` line followed by a later `confirmed`
  line is refused too, unless that later line's trace event returned normally (a
  successful result).
- Use the head only when the status is `confirmed` AND the line's `path` equals
  the trace's decoded path byte for byte. strace prints the path as a C-escaped
  string with `-s 4096` (`sandbox/strace.go:91`, which exceeds `PATH_MAX`, so a
  path is never cut); the trace parser decodes the escapes to bytes
  (`Event.Path`) and the comparison is on those bytes, not on text. A trace path
  the parser flagged as truncated or undecodable (a trailing `...`) refuses.
  Anything else refuses. A slipped count, a
  thread-versus-leader attribution difference (a non-leader exec, whose trace line
  strace may attribute to the other pid), or a path or symlink swapped in the
  window of step 6 all fail the pairing or the equality, so a counting slip fails
  closed. This is also the discrepancy check of section 3.4a of the spike note,
  which a trace-only reading can never make.
- The loader path comes from the head through the EXISTING `loadChain` walk
  (`elfInterpreter`, `shebangInterpreter`), and the loader is observed like any
  read, `c.observe(...)`. There is no `checkStability` for such a head: it was
  read from the pinned inode while the kernel held it write-denied.
- A miss keeps `exec-unreadable`, with a sub-code that names the status in place of
  `under-tmp`: `exechead-gone`, `exechead-mismatch`, `exechead-timeout`,
  `exechead-unpinned`, `exechead-fexecve`, `exechead-same-image`,
  `exechead-outside` and `exechead-pair` (no line, a count that differs, or the
  path differs). They join `censusSubCodes` (`census.go:59`), so
  the census says what is left to win.

*Fail closed.* If `kiln record step` dies, the listener closes: every pending and
every later notified call fails with `ENOSYS` instead of running unrecorded
(seccomp_unotify(2), kernel rule). A supervisor-side error (an `ioctl` failure
other than `ENOENT` for a notification that vanished, a sidecar write failure) marks
the step unclean (a new entry of `StepSidecar.Unclean`, `exechead-failed`), and the
action is not emitted. A listener that never arrives is the same hazard before the first
notification: the init's own `execve` of the step blocks in the notification with
nobody to answer it. `kiln record step` therefore has a deadline on RECEIVING the
listener (step 3's handshake); on timeout it kills the step and marks it unclean.
A notification the supervisor has not answered within the
stall deadline kills the step rather than answering it: a stuck supervisor stalls
every exec, and silence must not become permission. THE KILL TARGET is a pidfd,
never a pid number: the watchdog and the pending-lifetime bound kill the step through the
pidfd of the step's init (the pid-namespace init, whose death kills every process
in the namespace), which is the pidfd taken (step 1) for the first notification's
`tgid`, the init's own `execve` of the step; `--kill-on-exit` of strace is the
second line.

*Security preconditions, with evidence.* The probe is `TestExecHeadSpike`
(`exechead_spike_linux_amd64_test.go`), run as workflow runs 45342132127 and
45346779343 on ubuntu 24.04 (kernel `7.0.0-1014-aws`) and 26.04
(`7.0.0-1020-azure`), and reviewed adversarially (verdict GO).

- A writer that predates the exec cannot survive it. `exec-with-writer
  fd=text_file_busy shared-map=text_file_busy` in `host` and `userns`, on both
  images: the kernel refuses to exec a file with a writable descriptor OR with a
  writable `MAP_SHARED` mapping left behind by a closed one (`ETXTBSY`). So once
  the process is alive and `exe` matches, the inode's bytes are the ones the
  kernel read.
- A running binary cannot be written. `etxtbsy=text_file_busy` on every
  `elf-live-then-deleted` and tmpfs `fexecve` line, in `host`, `userns` and
  `sandbox`: the write-deny holds for as long as the process runs, so reading
  the head after the confirm, and checking `exe` again, is sound.
- An OUTSIDE supervisor works across user and pid namespaces with capabilities
  dropped. `xns` child: `CapEff=0`, `NoNewPrivs=1`, `Seccomp=2`, a pid namespace
  (`pid inside the namespace=1`), listener handed over `SCM_RIGHTS`. `xns-super`,
  the outside process: `process_vm_readv=ok(64B)`, `open_proc_root_string=ok`,
  `pidfd_getfd(fd0)=ok`, `pin="ok"`, `confirm=exe-match-after-1066us` to `1112us`
  and `etxtbsy=text_file_busy` for a live binary. The pids it saw
  (`notif-pid=4889,4891,4892,4895`) are in its own namespace, which is strace's.
- A notification and a ptrace stop coexist: `strace=attached strace-exec-lines=1`
  with the listener installed, on both runs.
- The exe re-pointing hole. `/proc/<pid>/exe` can be moved with `prctl(PR_SET_MM,
  PR_SET_MM_EXE_FILE)` or `PR_SET_MM_MAP` with `exe_fd`, and a moved `exe` would
  make a confirm lie. The step cannot do it: it holds no `CAP_SYS_RESOURCE` in the
  sandbox's user namespace (the init dropped every capability and set
  `no_new_privs`), and the filter refuses the calls that would make it a
  namespace in which it would (`unshare`, `setns`, and `clone` with
  `CLONE_NEWUSER`, `bpf.go:72-87, 145`; `clone3` is ENOSYS). The evidence so far is
  `PR_SET_MM_EXE_FILE=operation_not_permitted` in `sandbox` (uid 1000,
  `CapEff=0`, `NoNewPrivs=1`, `Seccomp=2`). **That is not yet enough.** The
  spike's `PR_SET_MM_MAP` probe passes an all-zero map and reads
  `invalid_argument` in `host` (unprivileged), `userns` AND `sandbox`: an
  unprivileged host process gets the same EINVAL, so the arguments were refused
  before a capability check could have been, and the answer says nothing about the
  capability. And `PR_SET_MM_EXE_FILE` reads `operation_not_permitted` in the
  `userns` child that HOLDS every capability, so that line does not separate the
  cases either. The decisive evidence is the VALID-map probe of PR #19902 (a map
  that passes validation, so only the capability check can refuse it), still to be
  run: the sandbox must return `EPERM`. **Classifier consumption (step 5) must not
  merge until that probe confirms it.** If the sandbox does not refuse, the confirm
  must be redesigned before any head is believed: drop `PR_SET_MM` in the filter
  (a refusal of `prctl` option 35, which a filter on the first argument can make),
  or confirm through the mapped file (`/proc/<tgid>/maps`, `map_files`) instead of
  `exe`. Steps 3 and 4 do not depend on it.
  DEFENCE IN DEPTH, not a fallback: step 3 refuses `prctl(PR_SET_MM, ...)`
  (option 35, any sub-option) with EPERM unconditionally in the FIRST filter, a
  check on `args[0]` of `prctl` (157), so the hole is closed by the filter whatever
  the capability check does. The option is an `int`, so the kernel looks only at
  the LOW 32 BITS of `args[0]`; the filter compares the low word only, as the
  `clone` flags check does, or a call with junk in the high bits would pass the
  filter and still reach `PR_SET_MM`. The valid-map probe is still the evidence required
  before step 5, because the capability argument and the filter are two layers and
  the probe is what shows the sandbox's own answer. The filter change alters what a
  step can do (a step calling `PR_SET_MM` gets EPERM; no build step has reason to),
  so it interacts with `RecorderVersion`, below.

*Kernels.* The write-deny (`ETXTBSY`) and CONTINUE behaviours are VERIFIED only on
7.0.x kernels (the two CI images). The supervisor records the kernel release in the
sidecar header; the classifier believes a head only when the header's kernel series
(major.minor) is in a table of series the real-kernel tests of step 4 have run on
(initially `7.0`), matched EXACTLY on major.minor, and otherwise refuses with
`exechead-kernel`, the behaviour of today; an empty or unparseable `uname -r` is
refused the same way. A table is needed, not a version floor: the facts are old kernel behaviour
(CONTINUE is 5.5, the write-deny far older), but a series nobody ran them on is
not evidence. A later version could drop the table: `prepare` could probe the two
facts on the recording machine itself (an exec while a writer is open is refused;
a write-open of a running binary is refused) and seal the result in `pre/`. That is
evidence about the actual kernel and needs no list of series. v1 keeps the `7.0`
table.

*Known v1 limit.* A process that exits before the confirm is `gone` and stays
refused: its binary could have been rewritten after it left. A Go test binary runs
far longer than the poll, but a fast-exiting tmpfs binary is not helped (the
probe's `elf-fast-exit`: `confirm=gone:no_such_file_or_directory`). It has its own
census sub-code, `exechead-gone`, so the census counts how much remains. A later
version could also notify on `exit_group` and take the head before the process is
reaped. A non-leader exec, `fexecve` and a script under the tmpfs are refused in v1
too. (The probe's `script` case reads `gone` rather than `mismatch` because the
script's `sh` exits at once; both refuse.)

*Cost.* Measured at about +30 to +90 microseconds per exec: `baseline_us` to
`with_pin_us` was 361 to 399 and 584 to 666 (`host`), 368 to 390 and 590 to 675
(`sandbox`), 373 to 422 and 574 to 670 (`userns`) in run 45346779343, so +22 to
+96 across the two images. Against what `strace -f` already costs per call it is
small.

*`RecorderVersion`.* It is 16 now (`internal/legacy/traced.go:148`). Step 3 changes
the sandbox's behaviour for one call (`PR_SET_MM`, EPERM) and the second filter
notifies every exec, so it bumps the constant (16 to 17: the same action could in
principle record differently); step 5 grows the accept set, so every weak key changes
again and it bumps once more (17 to 18). Renumber if another change takes a number
first. Step 4 changes nothing the classifier believes and does not bump.

*Implementation plan.* Steps 1 and 2 are the spike and this decision.

3. **Sandbox listener and socket.** The `--exec-notify-fd` option and its
   descriptor check, the second filter, the init's `seccomp` call and send,
   `RunExec`'s mapping and close. `kiln-sandbox` tests: an exec notifies; the first
   filter's refusals are unchanged; a closed listener fails execs; the step holds no
   socket; a configured-but-missing or non-socket descriptor is refused; the trace
   still parses. Also in this step: the places that read the sandbox helper's own
   command line (the parser's helper recognition and the argv comparison at
   `record/classify_step.go:568`, and the tests that spell the argv) accept the new
   `--exec-notify-fd` argument; `kiln record step` gets the deadline on
   receiving the listener (kill the step, mark it unclean, on timeout); the first
   filter refuses `PR_SET_MM` (with a filter test, including that `PR_SET_MM` with
   high bits set in `args[0]` is still refused, and the `RecorderVersion` bump
   above); the supervisor-side socket end is `CLOEXEC`, and the listener is received
   with `MSG_CMSG_CLOEXEC`, exactly one descriptor and one data byte expected, any
   extra descriptor or data failing the step.
4. **`kiln record step` supervisor.** The socketpair, the receive loop, the
   per-notification protocol, the `<trace>.exechead` writer and the digest in the
   step sidecar. Real-kernel tests for each status (`confirmed`, `outside`, `gone`,
   `mismatch`, `timeout`, `unpinned`, `fexecve`) and encoder/decoder unit tests
   (malformed, duplicate, oversize). Two more: a self re-exec (a tmpfs binary
   exec'ing itself) is refused `same-image`, and a first exec of a tmpfs binary
   from a different image confirms. And tests of the L1 bypass (a same-tid notification queued behind a different-tid
   one settles the pending exec and is answered, the other stays queued behind it),
   of the three-state poller-versus-settle race, of reuse of a tid number after a
   non-leader exec (the line confirms, and pairing refuses it), of an
   `ERESTARTNOINTR` restart, and of a signal-interrupted notification (`NOTIF_ID_VALID`
   fails, it is dropped, and a same-tid notification then arrives). And tests that an `outside`-then-tmpfs chain in one pid (a host `sh` execing a
   tmpfs binary in place) confirms, that a two-thread overlap taints, that a
   pending pin is settled at the next same-thread notification, and that the poller
   and the same-tid settle race with exactly one line written. And a race test that the taint set before an `overload` CONTINUE is seen by a
   poller finishing at that moment (no `confirmed` line after it), a test that a
   queued notification whose thread dies in `de_thread` is dropped without taint
   while its pid is refused by the pairing, and a test that a non-leader's own
   successful exec is not taint. And tests that `overload` or `timeout` taints every later line of the group, that
   an overlapping exec in the same thread group never confirms, and that a killed
   or unfinished exec (`= ?`, an unresumed `<unfinished ...>`, `+++ killed`)
   refuses that pid. And one for the rename variant: fork a child of
   X, then exec a path swapped to Y (a thread swap, and a rename by another
   process); it must not confirm. Also tests for `failed` (an exec that returns
   EACCES is decided at once), `overload` and the cap-exceeded path (the supervisor
   keeps answering), a non-leader exec (refused), and the header and `seq` checks.
5. **Classifier consumption.** The pairing, the path-equality check, the
   `exechead-*` census sub-codes, the differential fuzzer's model of the sidecar,
   and the `RecorderVersion` bump. Gated on the valid-map `PR_SET_MM_MAP` probe
   above (cleared: run 45350985752 of PR #19902, see the spike note, section 3.3).
   DONE; see the implementation status below.
6. **A fresh-home census** measuring how many Go actions become emittable and the
   remaining `exechead-gone` count.

Steps 3 and 4 may merge first.

*Implementation status.* Step 3 is merged (#19906). Step 4 is implemented
(`internal/exechead`: the format, the strict decoder PR 5 reads with, and the
supervisor; `internal/record/exechead_step.go` and `step.go`: the wiring; the
real-kernel tests are `TestSandboxExecHead*` in the root package, named in
`ci/assert_sandbox_tests.py`). Where the implementation departs from the text
above, the text is not changed and the departure is listed here:

- *The kill at a lost hand-over is strace's, not the init's.* There is no init
  pid yet when the listener has not arrived (the init's pidfd is taken from the
  first notification), so the receive-deadline kill is `cmd.Process.Kill()` on
  strace (pidfd-backed in Go) and `--kill-on-exit` takes every tracee. Every
  later kill (the stall watchdog after the first notification, the lifetime
  bound) goes through the init's pidfd first and strace second, as specified.
- *`overload` is not by itself unclean.* The per-notification `overload` (the
  confirm cap, a group's queue cap) taints and writes its line, as specified,
  and the step stays clean; the step is unclean (`exechead-overflow`) when the
  SIDECAR's line or byte cap is passed, when the thread-group table overflows
  (the global overload-all taint), and when the inbox cap is passed.
- *The settle and the queue.* A held notification is handed back to the
  processor at the FRONT of its inbox when its group's exec is decided, instead
  of being "kept queued behind B": the order of lines and answers is the same
  (B is answered first, then each held one is judged against B's outcome), and
  the processor stays single-threaded.
- *A non-leader exec is recorded, not refused here.* Its line carries a `tid`
  that differs from the `tgid`, which is what PR 5's pairing refuses on. By the
  different-tid rule the group is TAINTED as soon as a second thread notifies after
  the leader's undecided exec (CI showed it: the non-leader's line and the next exec
  of the group are both `tainted`), so a non-leader's exec is never `confirmed`
  either; the "own successful exec is not a taint" sentence of D48 step 1 applies
  only to the vanished-thread rule.
- *Line cap.* A line is at most 128 KiB (a 64 KiB head is 87384 base64 characters
  and a 4095-byte path may need six characters per byte once JSON-escaped, so
  "about 90 KiB" is not an upper bound); the file is at most 64 MiB and 131072
  notification lines.
- *Test seams.* `Timing.PollStart` (delay each poller's first look) and
  `Timing.ProcessDelay` (delay the processor per notification) exist so the
  real-kernel tests can hold an exec pending, make one `gone`, or interrupt an
  unanswered notification. Both are zero in production.
- *Not tested against a real kernel here:* the race of a taint set before an
  `overload` CONTINUE against a poller finishing at that moment (the ordering is
  by the one lock, `finishLocked` reads the taint under it), and the reuse of a
  tid number after a non-leader exec (needs a pid-reuse the kernel will not
  produce on demand); PR 5's pairing tests are where they belong.
- *Landed DEFAULT-OFF (review).* `kiln record step` runs the supervisor only with
  `--exec-head` (`StepOptions.ExecHead`); without it nothing changes (no socket, no
  second filter, the step sidecar's `exechead_*` fields empty, schema `/3` either
  way). The census and the other recorder tests stay on the old path; the census
  opts in when step 5 exists.
- *The pending-lifetime bound (changed, approved in review).* A pending exec older
  than the bound is killed (`exechead-stuck`) ONLY if the thread's last
  `/proc/<tid>/syscall` reading shows it blocked in `execve`/`execveat`. Otherwise
  the group is tainted and the line is `timeout`. Why: a failed in-place exec can
  leave its thread computing in user mode, where the file reads `running`; killing
  that would kill a healthy step. This replaces "ends in an unclean step" in step 1.
- *Vanished thread, exe unchanged (implemented).* A poller that finds the notified
  thread gone (a non-leader, whose `/proc/<tid>` no longer exists) with `exe` still
  the pre-exec image re-checks `exe` for up to 100 ms (a non-leader's own exec makes
  its tid vanish briefly BEFORE exe changes); only if it is still unchanged does the
  group taint and the line is `tainted`.
- *`WAIT_KILLABLE_RECV`.* The init asks for `SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV`
  (Linux 5.19) with `NEW_LISTENER`, so a caught signal cannot interrupt an unanswered
  notification (no EINTR, no `ERESTARTSYS` in the trace, no restart); on `EINVAL` it
  falls back to plain `NEW_LISTENER`. The hand-over byte is `K` for the former and
  `L` for the latter (`sandbox.ExecNotifyByteKillable`), the supervisor accepts
  either, and the sidecar header records it (`"wait_killable"`). The restart path
  (ERESTARTNOINTR, the first notification invalid, a second one) can therefore only
  be exercised on a kernel that refuses the flag; on the 7.0 CI kernels the signal
  test pins that no restart and no drop happens.
- *Receiver death.* If the receiver stops for any reason but the listener hanging up,
  Finish or Abort (an error, a panic), the step is killed and marked unclean
  (`exechead-receiver`), the listener is closed (later execs get ENOSYS instead of
  blocking) and the processor is stopped. A panic in a poller or the watchdog
  degrades the supervisor and kills the step.
- *Locking.* The kill path takes no `mu` (the init pidfd is atomic; unclean reasons
  have their own mutex), because `mu` can be held across a blocking sidecar write.
  The group's pidfd is read through a DUP taken under `mu`; the kill target is a dup
  of the pidfd the first notification's group was validated with, not a second
  `pidfd_open` of the number.
- *Unmeasured tmpfs devices (changed).* Where D48 said a device that could not be
  measured counts as "on the tmpfs", the supervisor now records every exec `outside`
  and never opens the file for reading (fail closed; the classifier refuses an exec
  under the tmpfs as today).
- *Start-up check.* `New` calls `seccomp(SECCOMP_GET_NOTIF_SIZES)` and refuses if the
  kernel's structure sizes differ from the layouts hard-coded here (80, 24, 64).
- *Header write is deferred* until the hand-over is decided (it carries
  `wait_killable`); the file is still created exclusively before the step starts.
- *`Decode` takes a mandatory `Expect`* (nonce, action digest, step): a zero or
  malformed one is refused, never read as "no check".
- *For step 5 (PR 5):* refuse any line with `tid != tgid`; a non-confirmed line
  carries no evidence, and an `outside` line has no transition check; an empty path
  under the tmpfs refuses (`unpinned` with path `""` is not a path); read the file
  with `exechead.Decode` and the step sidecar's nonce, the action's digest and the
  step number.
- *Not done at step 4 (the end-to-end test is DONE in step 5, see below):* a real-kernel test of `kiln record step --exec-head` end to end (the
  harness drives the supervisor with the production wiring but not `RunStep`), and
  a held thread to exercise the "blocked in execve" lifetime kill.
- *Known limits.* (a) The sidecar's blocking `Append` still runs under `mu`; the
  kill path, the stall check and the lifetime check do not depend on `mu` (atomics,
  a separate unclean mutex, `TryLock`, and a starvation bound that kills when the
  lock is unavailable for longer than the lifetime), but a stalled disk stalls the
  processor and pollers until the stall deadline kills the step. (b) The listener
  is guarded by a read-write lock so a closed descriptor number is never used.
  (c) Shutdown edge cases: a poller finishing after the processor has exited leaves
  its released notifications unprocessed (their threads are dead with the step); a
  receive that never ends is shut down after `FinishWait` and the step is marked
  unclean; `Finish` abandons a blocked `NOTIF_RECV` (the goroutine ends with the
  process). (d) The init pidfd is bound only for the first notification and only if
  its last `NSpid` entry is 1; otherwise the kill is strace's alone.
- *For PR 5, before the census opts in (DONE in step 5, see below):* add an end-to-end real-kernel
  `RunStep --exec-head` test (the fd 6 pass-through via strace and the step
  sidecar's digest fields), and close the narrow lifetime false-kill where a new
  execve from the same tid lands just as the bound is checked (skip the kill if a
  notification from that tid is in the inbox or being processed).
- *`RecorderVersion`: NO bump at step 4; PR 5 bumps.* With the supervisor
  default-off a recording is byte for byte the same apart from three empty
  sidecar fields, step sidecars are not cached across versions, and the filter
  flag change applies only with `--exec-notify-fd`. The step sidecar's schema
is `kiln-record-step/3` (`exechead_nonce`, `exechead_sha256`, `exechead_bytes`).
The paragraph "Step 4 changes nothing the classifier believes and does not
bump" above stands.

*Step 5 is implemented (classifier consumption).* The pairing and the use of a
head are `internal/traced/exechead.go` and `exec.go` (`loadChain`); the recorder
checks the file before the classifier sees it (`internal/record/exechead_read.go`,
called by the epilogue and again by `collect`); `RecorderVersion` is 19 (18 is
taken by open PR #19908; whichever of the two lands second renumbers). The gate of
the `PR_SET_MM_MAP` probe is cleared (the spike note, section 3.3). Tests: the
pairing rules and every refusal in `internal/traced/exechead_test.go`, the
recorder's integrity cases in `internal/record/exechead_read_unix_test.go`, the
exec-head case of the differential fuzzer in `exechead_model_test.go`, and the
real-kernel `TestSandboxRecordExecHead` (`kiln record step --exec-head` through
RunStep, the fd 6 pass-through, the sidecar's digest fields, and `collect`
believing the exec of a binary under the tmpfs). Where the implementation departs
from the text above, the text is not changed and the departure is listed here:

- *The pairing's order of checks.* Per exec event: the pid is not refused, the
  line's syscall equals the event's (an addition: `execve` against `execveat`),
  the kernel series is on the table, the status is `confirmed`, the path is
  non-empty and equal. So on a kernel off the table EVERY refusal is
  `exechead-kernel` (the whole file is untrusted, whatever each line says), and a
  non-confirmed line is `exechead-<status>` even if its path differs. An empty
  path (a `fexecve` line, an `unpinned` NULL) under the tmpfs is `exechead-pair`
  when the status is `confirmed`, and the status's own code otherwise.
- *No anchor without a helper trace.* A trace with no init pid (not a helper
  trace) believes nothing: every exec under the tmpfs is `exechead-pair`. The
  init pid is paired like every other pid (equal counts from the first line);
  "the setup execs the parser dropped were of the recorded Kiln helper" holds by
  construction, since `windowFilter` drops only `isHelperExe` paths and the parser
  is given the recorded Kiln binary, and the G7 digest checks read the same two
  execs' argv.
- *Killed, unfinished and result-less execs.* The trace parser refuses the WHOLE
  trace for each (`unresumed-call`, `result`; pinned by
  `TestKilledUnfinishedAndResultlessExecsRefuseTheTrace`), so none ever reaches a
  pairing. The classifier keeps its own wall (an exec event whose result is `?`
  refuses the pid) for the day the parser keeps one.
- *The file's integrity is a parse reason, not a `trace-incomplete` reason.* A
  missing, unreadable, non-regular, oversize, malformed (strict `Decode`, another
  nonce, action or step, a repeated `seq`) or digest-mismatched file, and a file
  the step sidecar does not name, add `step N: exechead-<missing|oversize|digest|
  malformed|unclaimed>: ...` to the record's `parse_reasons`: the action's write
  set is unknown, exactly as for a trace that does not parse. `collect` makes the
  same check again, so a file changed after the epilogue is not believed. The
  census files each under a fixed code of the same name, and the supervisor's own
  unclean reasons (`exechead-handover`, `-receiver`, `-failed`, `-overflow`,
  `-stuck`, `-stall`) now have codes of their own instead of `unclean:other`.
- *Sub-codes.* The census list is the thirteen of the classifier: `exechead-pair`,
  `-kernel`, `-outside`, `-gone`, `-mismatch`, `-timeout`, `-unpinned`,
  `-fexecve`, `-same-image`, and also `-failed`, `-overload`, `-tainted`, and `-notelf` (a believed head that starts with `#!`; see below) (the
  text above omits the last three and `-kernel`; a status is never believed, so it
  needs a code). Thirty-two sub-coded kinds at 256 paths no longer fit the
  census's overall bound of 16384 entries, which is now 20480.
- *The same-tid lifetime false kill is closed* (the step-4 review's last item).
  At the pending-lifetime bound the candidate is skipped (neither the kill nor
  the taint) if a notification from its thread is in the inbox or being processed
  (`inbox.holds`, one fact under one lock, covering the instant between the
  processor taking an item and starting it), and a kill is re-checked after a
  50 ms grace for a notification received but not yet pushed.
- *The differential fuzzer* writes the sidecar the supervisor would have left
  from the model's truth (the first program's bytes at the effect time) and
  checks it honest (the read-set oracle now reaches actions that exec a file
  under the tmpfs, which a plain record always refuses) and slipped (a missing,
  repeated or non-leader line, a status that is not `confirmed`, a changed path,
  a kernel off the table, a rotated sequence: no refusal of the plain record may
  disappear).
- *`RecorderVersion` is 19, not 18* (see above).
- *Two rules added in review.* (a) A believed head that starts with `#!` is refused
  (`exechead-notelf`): a `confirmed` line says exe became the pinned file, which is
  never a script, so the line contradicts itself. (b) A `confirmed` or `mismatch`
  line paired with a FAILED trace event is slip evidence and refuses the pid
  (`exechead-pair`), wherever it occurs (before, only an earlier undecided line
  did). The reverse is NOT a rule: a `failed` line beside a successful exec is
  legitimate (`sh` running a tmpfs script whose interpreter is `sh` itself leaves
  exe unchanged) and stays `exechead-failed` for that exec only. The fuzzer's
  honest sidecar writes `mismatch` for a script and gains the forward case as a
  slipped perturbation.
- *Step 6 prerequisite: DONE (biased generation).* `TestDifferentialExecHeadBiased`
  (`exechead_biased_test.go`) generates actions whose only would-be refusal is an
  exec under the step's fresh tmpfs: tmpfs ELF binaries (no loader, a loader inside
  the checkout, the host's loader; sometimes rewritten after the exec), host and
  checkout execs, failed execs, one to four pids, the init anchor, and host and
  tmpfs execs mixed in one pid. A rotation mode keeps neighbouring lines distinct so
  the ordinal slip applies. The model now records the look at an in-checkout PT_INTERP
  loader, so a classifier that forgets to walk a believed head's loader fails the
  read-set oracle. At the default 800 seeds: 800 of 800 cases honest, reason-free
  and judged by the read-set oracle (the random generator: 1 of 4000), against a
  floor of 700; every perturbation is applied to every case it can apply to and
  refused at least 700 times (the ordinal slip 250, which needs distinct neighbours:
  292 measured; the result-less exec 280: 325 measured), and the same suite adds three
  perturbations the random mode lacks (wrong syscall, unknown init pid, a confirmed
  `#!` head) and the result-less exec (`= ?`). Mutation check of the pairing rules: killed
  by the biased mode alone are the init-pid, count, non-leader, result-less,
  confirmed-vs-failed contradiction, syscall, kernel, status, path, ordinal and
  `exechead-notelf` rules, the head-from-sidecar and loader-walk rules; two mutants
  survive as equivalent (the undecided-then-confirmed clause, subsumed by the
  contradiction rule; the empty-path test, subsumed by path equality).
- *Step 6 items 1-3: DONE (the choice is sealed, the census reports it, the workflow can ask for it).*
  1. `kiln record prepare --exec-head` seals the choice in the binding (`exec_head`,
     omitted when off, so a default run's binding keeps its bytes and id) and puts
     `--exec-head` in `pre/exec-wrapper.json`, which the build tool's `-exec-wrapper` carries to every
     step. `kiln record step` takes the value from the sealed binding; its own `--exec-head`
     flag can only assert it, and one that disagrees (`--exec-head` on an off seal, `--exec-head=false` on
     an on seal) is refused before anything runs. `collect` checks every STARTED step: the sidecar has an
     exec-head nonce exactly when the seal says on, and a disagreement in either direction makes every
     record's write set unknown with the reason `exechead-seal:` (census unknown-reason code
     `exechead-seal`), the way a serial violation does (no new void token: the merge already voids on an
     unknown write set). `RecorderVersion` is unchanged: the accept set is the same, only a half-supervised
     run is newly refused.
  2. The census summary gains an `exec_head` block: `sealed` (from the binding), `kernel_trusted`, the
     supervised step count, the occurrences of each of the thirteen `exechead-*` exec-unreadable
     sub-codes, the steps unclean for each of the supervisor's six reasons, the actions refusing an
     exec-head file for each of the five parse reasons, the supervisor's kills and notifications summed
     over steps, and the emittable count of language `go`. Every key of every table is present, zero
     included; nothing in the Markdown is a path. The notification and kill counts are new step-sidecar
     fields (`exechead_notified`, `exechead_kills`, within `kiln-record-step/3`: step 3 shipped only
     in this unreleased train and nothing reads a sidecar across versions). `ci/census_not_clean.py` applies,
     only when the shard ran with exec heads, four hard criteria: no `exechead-*` unclean reason on any
     step, no supervisor kill, no `exechead-*` parse reason (and no `exechead-seal` void), and no
     `exechead-kernel` sub-code when the kernel series is on the trusted table (7.0).
  3. The census workflow's `workflow_dispatch` takes `exec_head` (default true since 2026-11-25; it was false). When true, every
     shard's summary opens with a line saying the run is an exec-head run and NOT PR8 evidence,
     `prepare` gets `--exec-head`, and a broken criterion fails the shard (`--strict-exec-head`:
     the soft passes of a fresh run stay annotations). The default flipped to true because census runs A' (45443745064/45443753967), B (45466205573/45466210497) and the classification pair (45557016876/45557181111) were all clean on the hard criteria. The scheduled run is forced to `exec_head=false` in the validate step, so the weekly PR8 evidence is unchanged.
- *Step 6 vacuity counters (census run A found a hole in items 2-3).* Run A sealed exec heads on shards
  that held only Rust actions; a fresh HOME has no cargo, so every action failed in 0.1 s and no step
  exec'd a binary under the step tmpfs. Every sub-code was 0, `go_emittable` was 0, and the strict checker
  passed: a result-shaped nothing. The `exec_head` block therefore gains three counters, per shard:
  `go_actions` (Go actions), `go_built_ok` (Go actions whose build succeeded, as `built_ok`), and
  `tmpfs_execs` (successful execs under the step tmpfs that reached the classifier's exec-head pairing,
  believed or refused with an `exechead-*` sub-code). `tmpfs_execs` is counted in `loadChain`'s tmpfs
  branch, the one place those sub-codes are raised, and travels in the action's `census.json` as the
  additive `exec_head_tmpfs_execs` (counted even when the census is otherwise empty, since a believed exec
  raises no reason); it is saturating and at least the sum of the sub-code counts. *The vacuity rule:* a
  zero in the hard-zero criteria is a result only if something that could have been nonzero happened. A
  sealed shard with `go_actions == 0` or `tmpfs_execs == 0` opens its Exec heads section with "exec heads
  ran but nothing exercised them; go_emittable is not a result", and `census_not_clean.py` prints a NOTE
  for it, never a failure (one shard without Go is legitimate). The check fails when ALL sampled shards on this image
  have `tmpfs_execs == 0` (each matrix leg checks only its own, at most three), when a summary is
  unreadable, or when fewer than `--expected-shards N` were read: `census_not_clean.py --exec-head-run
  [--expected-shards N] <summary.json>...`, called once per image by a final workflow step because a
  shard cannot see its siblings. `readCensus` refuses a `census.json` whose `exec_head_tmpfs_execs` is
  below the sum of its own `exechead-*` occurrences (entries plus overflow); the counter is diagnostic, and
  a forged value on a clean file is undetectable and never hides a hard-zero criterion. `RecorderVersion` and the step schema are
  unchanged; this is census and reporting only.
- *Not done:* runs A and B (the first dispatches with `exec_head: true`, fresh and then runner home,
  on both images), and a real-kernel run of the supervised step's classification on the ubuntu-26.04
  image (the test runs wherever the `kiln-sandbox` job does; it was not run by the author, who had no
  Linux machine). The step-5 e2e test runs ONE supervised action under a sealed prepare, and a
  control shard prepared without the flag (the same action, refused as exec-unreadable).

**Decision D47: dying unknown-number calls (D44 follow-up; `kiln:recorder` 14 to 15).**
(The decision number D46 is left to the open cwd-race work.) Go actions were refused as `unresumed-call` by a
dying `syscall_0x1c8` (456) call: the pinned strace 6.8 predates it, so it
prints the number rather than a name. D44 made the spelling recognisable
(`isUnknownNumberName`) but census-only. The spelling alone is NOT enough to
drop the call: the filter in `internal/sandbox/bpf.go` answers ENOSYS only to
numbers ABOVE `MaxKnownSyscall`; at or below it the filter is a DENYLIST
(`RefusedSyscalls` get EPERM, `clone3` ENOSYS, everything else ALLOW), so an
unknown number there reaches the kernel and may be a path-taking call.

*The rule.* `dyingPolicy` (shared by both readers, next to `nullPathDying`)
drops a dying call, with no event, whose name matches `isUnknownNumberName`
exactly and whose parsed number N satisfies either

- (a) `syscallceil.Max < N <= 0xffffffff`. The filter answers ENOSYS (or, with
  the x32 bit set, KILLs the process), so the call did nothing; sound by
  construction. The ceiling is one constant in the leaf package
  `internal/syscallceil`, read by `BuildFilter`'s `jgt` and by the parser, with
  a sandbox test pinning the filter's boundary to it. The 32-bit bound is
  needed because `seccomp_data.nr` is 32 bits: strace could print
  `syscall_0x1000001c8`, which the filter sees as nr 456, not as a number above
  the ceiling. Such a number stays refused. Equal to the ceiling stays refused
  (461, `lsm_list_modules`, reaches the kernel); so does zero.
- (b) N is in the closed set `{456}`. Source: torvalds/linux
  `arch/x86/entry/syscalls/syscall_64.tbl`, `456 common futex_requeue
  sys_futex_requeue`, signature `futex_requeue(struct futex_waitv *waiters,
  unsigned int flags, int nr_wake, int nr_requeue)`: no path argument and no
  descriptor, so dropping it removes no input from the read set. Any addition
  needs a table citation and a no-path signature. (454 futex_wake and 455
  futex_wait are known to the table but not in the set: unknown numbers below
  the ceiling are refused unless listed.)

Every other unknown number stays `unresumed-call`. Near-miss spellings are not
`isUnknownNumberName` and never reach the rule. A completed or non-dying
unknown-number call is unchanged (refused by name outside the policy). The rule
is not a spawn (`droppedSpawnOf` says `dropNone` for the unknown class), so it
accounts for no orphan pid (D44, D44a). `RecorderVersion` is 16: the accept set
grows (version 15 belongs to the cwd-race work).

*The root helper's own group.* The rule also matches a dying unknown-number call in the root helper's own process group, which is not under the step's seccomp filter, so branch (a)'s ENOSYS argument does not cover it. That group is Kiln's own binary, which issues no such call today; a helper that ever did would have to be re-reviewed against this decision.

**Decision D44: orphan exits, NULL-path dying calls, short mknod, and the
unknown-number census code (census run 8; `kiln:recorder` 11 to 12).** Three
refusals left in the census are explained by shapes that cannot hide a read,
and a fourth needs a decision first.

1. *Orphan exits (`orphan-pid`, Go, one action per image).* The child of a
   spawn whose return line died with its parent has no spawn record (D35 drops
   a dying fork, vfork or clone, so none is made), so its first line is
   unexplained. If that line is its EXIT, and the pid has no other record, it
   is accepted, but only while the number of such pids is at most the number of
   dying spawns D35 dropped in this trace (counted on the exit record that
   ended each one, `record.spawnDropped`; clone3 is refused-class and never
   dropped, so it counts for nothing). With more orphans than dropped spawns
   some orphan is unexplained, and ALL of them keep refusing as `orphan-pid`.
   *Soundness.* A pid with nothing but an exit made no recorded call, so it
   read nothing the model infers reads from (open and stat records). The count
   bounds how many such pids the dropped spawns can have produced, so a stray
   pid, a pid reused by another trace's process, or a spawn lost by any other
   route is not covered. A pid with any other record (a stat, a second exit)
   is not an orphan. The lines that leave no record (ignored-class calls,
   signals, filter-answered refusals) carry no path. The orphan's exit stays in
   `Trace.Exits` and `ExitLines` but gets no `Procs` entry, so `DaemonLeft`
   treats it as a possible survivor (a missing entry is "not proven outside the
   init's tree"): an orphan that dies after the init's exit_group still
   refuses as `daemon-left` (D44a lifts this when every dropped spawn is a
   `CLONE_THREAD` clone). *Deferred:* matching each orphan to a particular
   dropped spawn; `model.get` is unchanged.
2. *NULL-path dying calls.* A dying `stat`, `lstat`, `access` or `readlink`
   whose first argument is printed exactly `NULL` is dropped like the ignored
   class. *Soundness.* The kernel copies the path in before any lookup, so a
   NULL pointer fails with EFAULT having looked at nothing; there is no result
   a sibling could use. `newfstatat` and `statx` are NOT included: with
   `AT_EMPTY_PATH` a NULL or empty path means the descriptor itself, which is
   a read. A real path, an address (shape 3 prints one) or any other call keeps
   D35's refusal.
3. *`mknod` and `mknodat` arity.* strace prints the `dev` argument only when
   the mode's type is `S_IFCHR` or `S_IFBLK` (strace `src/mknod.c`,
   `decode_mknod`), so the minimum arity is 2 and 3, in the one table both
   readers share. The long spelling stays accepted. *Soundness.* A short call
   whose mode has `S_IFCHR`, `S_IFBLK` or any token the model does not
   recognise is still `arity`: such a line is not what strace prints.
   `S_IFREG` with the short form is what strace prints for a regular file, so
   it is accepted (it is a write of the path, exactly as with a device
   argument).
4. *Census only: unknown-number (`unresumed-call`, Go, 24.04 only, nine).* For
   a refused dying call whose name is OUTSIDE the closed table, the census now
   says whether the name is strace's unknown-number spelling, as the fixed code
   `unknown-number:yes` or `unknown-number:no` in the refusal-by-syscall
   tables (`trace.Error.UnknownNumber`; the name is never reported). The
   spelling is `syscall_` followed by the number in C's `%#lx`
   (strace `src/syscall.c`: `xsprintf(buf, "syscall_%#" PRI_klx, scno)`):
   lowercase hex with a `0x` prefix and no leading zeros, `0` for zero, at most
   16 digits. No drop rule is added: a call we cannot name may read, and that
   waits on a decision.

**Decision D43: the sealed root (real-build census, the last `host-read`s were
absent probes of places the sandbox does not contain; `kiln:recorder` 10 to 11).**
The remaining census entries were `/run/systemd/resolve/stub-resolv.conf` (317
times, node and npm's resolver, reached through `/etc/resolv.conf`), the
`package.json`, `go.mod` and `go.work` ABOVE the checkout (`$RUNNER_HOME/package.json`,
`$RUNNER_HOME/work/package.json`, `/home/package.json`, `/package.json`) and
`/var/run` (op `lookup`: the classifier walked the HOST's symlink, which does not
exist in the sandbox). The classifier asked the host's disk about places the step
runs without: the step's root is a new, sealed tmpfs holding only what the plan
mounts.

*The rule (`sealed.go`, `traced.Config.ScratchDirs`).* Inside the sandbox a path
EXISTS only if it is

- under the checkout;
- under a declared host root, or a `HostBinds` path;
- `Home`, `Tmpdir` or a scratch directory (`sandbox.ScratchDirs`: `/tmp`,
  `/var/tmp`, `/run`), or under one of them;
- a PROPER ANCESTOR of any of those (a directory the plan creates only to mount
  them: the classifier's `ancestors` set, D39, which now also anchors on the
  scratch directories, so `/var` is one);
- under `/dev`, `/proc` or `/sys` (the plan mounts `/dev` and `/proc`; `/sys` is
  admitted because the ephemeral list names it, and a policy's other literal
  ephemeral prefixes likewise).
- the target, or a proper ancestor of the target, of a declared root that is a
  SYMLINK on the host (merged `/usr`: `/bin -> usr/bin`), with up to 8 hops
  resolved once from the host's FS. The classifier's walk runs on the host's
  disk, so it follows that link even when the policy does not declare the
  target. Without this clause, an exec of `/bin/sh` walked to `/usr/bin/sh`,
  which a policy without `/usr` calls sandbox-absent, and it became
  `unstable-path` where it had been a `host-read`. Linux CI caught this in
  `internal/record`, whose fixture declares only `/bin` and `/sbin`; macOS, where
  `/bin` is a real directory, did not.

Everything else is SANDBOX-ABSENT (`sandboxHas`, `sandboxAbsent`). Lookup answers a
sandbox-absent path as absent without asking the host disk ONLY while it judges
a FAILED call; that call's absence is what this decision explains. A call that
SUCCEEDED found something the plan does not hold, and its walk (including an
exec's ELF loader, resolved within the exec) reads the host as before D43. So it
keeps exactly the pre-D43 verdict, a `host-read`, and the shortcut never invents
an `unstable-path`. Linux CI found this: with merged `/usr` and no `/lib64`
declared, the loader `/lib64/ld-linux-x86-64.so.2` of a successful exec was
answered absent. The set is a
SUPERSET of what the plan mounts (it admits every declared root whether or not
the host has it), so an error in it can only keep today's behaviour. It is active
only when `ScratchDirs` is non-empty; the recorder passes
`record.SandboxScratchDirs()`, a copy of `sandbox.ScratchDirs`, so the list the
plan mounts and the list the classifier reads cannot drift. An empty list keeps
every pre-D43 verdict.

*What changes.*

1. `lookup` returns ABSENT for a sandbox-absent path without asking the disk.
   `/etc/resolv.conf` is a link to the host's `/run/systemd/resolve/stub-resolv.conf`
   inside the image root; the walk follows it and finds nothing, as the kernel did.
2. `observeHost` allows an ABSENT probe of a sandbox-absent path. It does not
   allow a PRESENT one: a call that SUCCEEDED where the plan has nothing
   contradicts the model, and falls through to `host-read:<path>` as before. For
   a write-class call (`writePath` to `writeHost`) the same: a creation, removal,
   rename or metadata change that succeeded at such a path is `host-read` too
   (it cannot succeed on a read-only root, so the model is wrong).
3. The scratch directories are fresh tmpfs exactly as Home and Tmpdir are:
   `underTmp` and `isTmpRoot` cover them, so D39 rule 1b (an absent probe of a
   name nobody made is a constant), D40 containment (`tmpLinked`, `tmpMoved`) and
   the strict stability rule apply to them unchanged. The two directories the
   plan makes only to mount Home or Tmpdir beneath `/tmp` (ancestor-chain nodes)
   are excluded, and Home and Tmpdir themselves never count as "inside `/tmp`".
   `/tmp/x` that the action created and removed is therefore stable, where before
   it was `unstable-path` (it was judged as a host path).

   *What that means for D40.* `contained(p) = !tmpLinked && !tmpMoved &&
   underTmp(p)` now holds for a path under `/tmp`, `/var/tmp` or `/run` exactly
   as for one under Home or Tmpdir: the stability rules skip it while the action
   has made no link and moved no directory (or foreign object) in the tmpfs, and
   `tmpLinked` / `tmpMoved` taint the WHOLE action, scratch directories
   included, the moment either happens. *Why it is sound.* D40's premises are
   about the mount, not the name: each scratch directory is a fresh, empty tmpfs
   that the plan mounts per step (`sandbox.ScratchDirs`, `KindTmpfs`), a
   separate mount from the checkout, from Home, from Tmpdir and from the
   read-only roots, so hard links and renames across them fail with `EXDEV`;
   nothing is in it that the action did not put there; and a mount point is
   `EBUSY` to rename or remove, so no ancestor of it can move (D40 point 5).
   Those are the facts D40 used for Home and Tmpdir, so the argument (a path
   under it reaches the action's own file or nothing, and `..` out of it is
   exact while no directory moved) carries over unchanged. A rename ACROSS two
   scratch directories, or between one and Home, fails with `EXDEV` as before;
   the fuzzer model says so (`mountOf`).
   The differential fuzzer covers this with bursts 19 to 21 (the stale cwd
   against the scratch directories) and 22 to 27 (the `reshaped` taint, below).
4. Two new whole-action refusals, both gated on `classifier.reshaped`, set by a
   SUCCESSFUL rename or exchange anywhere (checkout included) whose source is not
   a regular file the action created, or whose names could not be placed
   (`noteMove`, `ops.go`; the same test as D40's `tmpMoved` but not limited to the
   tmpfs): `settleReshaped` runs after `checkStability` and (a) turns every ABSENT
   probe allowed by this decision, and every absent probe inside a scratch
   directory allowed by D39 rule 1b, into the `host-read` it was before; (b)
   raises `trace-incomplete:unstable-path` (sub-code `dotdot`) for a FAILED call
   whose walk stopped at a component of its text path the post-run disk lacks,
   with a `..` after it. (b) is a hole found while writing this decision and true
   without `ScratchDirs`: a cwd directory renamed away and not re-created, then a
   failed relative probe through `..` from the old text, was judged by its text
   (`observe` of the stop, absent in the fresh tmpfs, no reason) while the kernel
   started from the renamed directory. It needs a cwd the trace never walked (a
   `chdir` is itself a judged lookup, which is why the fuzzer never reached it); a
   process that inherits such a cwd would.

   *The fuzzer's reading of `reshaped`.* Bursts 22 to 27 of
   `TestDifferentialSeeded` / `FuzzDifferential` generate a rename or
   `RENAME_EXCHANGE` of an own directory, of a directory and of files the action
   did not create, of own regular files (which must not taint), absent probes at
   sandbox-absent paths before and after the move, and failed relative probes
   through `..` from a renamed-away cwd. The model sets its own `reshaped` when
   a DIRECTORY moved (the truth, read off its node kinds), and demands that a
   record with no other reason then (a) has no allowed sealed absence and (b)
   carries `unstable-path` if any failed relative probe walked, as text over the
   final disk, to a missing component followed by `..`. The classifier is allowed
   to taint MORE than the model does (it also taints on moving any object the
   action did not create, since it cannot see the kind of an object it has no
   history of); it is never allowed to taint less.

*Why it is safe.*

1. The new root is an EMPTY tmpfs (`plan.go:171`, `KindRootTmpfs`) onto which the
   plan binds only the declared roots (ro), creates the scratch tmpfs, binds the
   checkout and the state roots, and then SEALS it read-only
   (`plan.go:224`, `KindSeal`, a `MS_REMOUNT|MS_RDONLY` of the new root's own
   mount, applied by `sandbox_linux_amd64.go:411`) before the pivot (`plan.go:225`).
   The step then holds no capability and every mount-changing call is refused by
   the seccomp filter (`mount`, `umount2`, `pivot_root`, `mount_setattr`,
   `move_mount`, `chroot`, `unshare`, `setns`: `bpf.go:74-83`; the capabilities,
   `CapSysAdmin` among them, are dropped by `PrivSteps`, `privs.go:41`), so it
   cannot remount the root writable. Nothing can be created in a directory of the
   sealed root tmpfs, so `/package.json` or `/home/package.json` is absent in
   every run of the plan, and the plan's inputs are the configuration the
   classifier is given.
2. A path outside the exists-set has every ancestor on the sealed root tmpfs: if
   an ancestor lay under Home, Tmpdir, a scratch directory, the checkout or a root,
   so would the path. The writable tmpfs are all in the exists-set. This includes
   a checkout ancestor INSIDE a writable tmpfs (`$HOME/work` with Home an ancestor
   of the checkout, `/tmp/w` with the checkout at `/tmp/w/x`): its non-ancestor
   children are `underTmp` (made or absent, the trace says which) and are NOT
   judged by this rule.
3. Everything the plan mounts is in the exists-set. `internal/record/sealedroot_test.go`
   builds the plan from the recorder's own configuration for four layouts and
   requires `traced.SandboxHas` for every mount target and every directory on the
   way to it (roots, tmpfs, binds, ensure-dirs, masks, `/dev`, `/proc`, the seal
   dir), requires every tmpfs the plan mounts to be Home, Tmpdir, an anchor
   beneath Home or a `ScratchDirs` entry, requires every `ScratchDirs` entry to
   be mounted, and requires the host IPC paths, `/var/run`, `/package.json` and
   `/go.work` to be outside the set. A source check pins that `classifyInputs`
   sets the field. The real-sandbox test `TestSandboxSealedRoot` (required in
   `ci/assert_sandbox_tests.py`, linux/amd64) shows ENOENT inside a real sandbox
   for `/go.work`, `/home/x`, `/opt/rh`, `/run/x`, `/var/run`, `/package.json`, the
   resolver stub and `/var/run/docker.sock`, `mkdir` and `touch` failing on the
   sealed root, and `/run`, `/tmp` and `/var/tmp` writable and empty.

*Attacks, and why each fails.*

- (a) A stale text cwd after a directory rename, making a path look sandbox-absent
  (or a fresh scratch name) while the kernel resolves `..` from the renamed
  directory into the checkout. Three independent defences: the `chdir` into the
  directory is itself a lookup judged by the stability rules, and a rename touches
  it; every absent probe allowed here is remembered and refused at the end if the
  action moved anything but a regular file it made (`reshaped`); and a failed call
  that stopped at a component of the text the disk lacks, with a `..` after it, is
  refused under the same condition (the `dotdot` refusal above). The rename of a
  directory in the checkout, in Home, Tmpdir or a scratch directory, an exchange,
  and a rename that could not be placed all set `reshaped`; renaming a regular file
  the action created (Go's build cache, gcc) does not. Tests:
  `TestSealedRootStaleCwdAfterADirectoryRename` (scratch text that reaches the
  checkout physically, with and without re-creating the old directory; checkout
  text; exchange and unplaced renames; a move refusing every allowed absence; the
  cwd renamed away). The fuzzer's two new bursts drive the same shape, but every
  case it reaches is also refused by the stability rules, so disabling `reshaped`
  fails the unit tests and not the fuzzer: stated plainly, `reshaped` is a second
  line, proven by the unit tests alone.
- (b) A symlink the action made inside a writable root pointing at a
  sandbox-absent path. The walk follows links (a link in Home, Tmpdir or a scratch
  directory from the trace, in the checkout from the disk and tree), so what is
  judged is the TARGET, and the target really is absent in the sandbox: the probe
  is a constant. A call that SUCCEEDS through such a link contradicts the model
  and keeps `host-read`/`through-missing`. A link in a scratch directory also sets
  `tmpLinked` (D40), which disables containment for the whole action. Test:
  `TestSealedRootSymlinkToAnAbsentPath` (links in Tmpdir, Home, the checkout and
  `/tmp`; absolute and relative targets; failed and successful stat).
- (c) `..` from a scratch directory. A scratch directory is a mount root whose
  parent is `/` of the new root, and `parentOf("/tmp")` is `/` in the walk; from
  Tmpdir under `/tmp` the parent is `/tmp`. Both land on nodes the plan makes.
  `/proc/self/cwd/..` and friends never reach the rule (D28 refuses them first).
  Test: `TestSealedRootScratchDirectories` (`/run/../package.json`,
  `/tmp/kiln-1/../../package.json`).
- (d) A sandbox-absent path whose ancestor is a writable tmpfs where the action
  could create it. By point 2 above no sandbox-absent path lies under a writable
  tmpfs: the writable ones are in the exists-set, so lookup consults the trace
  (`created`, `links`, `dirs`) there, and a name the action made is its own, one
  it made and removed is absent again (and stable under D40), and a PRESENT look
  at a name nobody made is `host-read`. Tests:
  `TestSealedRootScratchDirectories`, `TestSealedRootScratchHoldingTheCheckout`
  (`/tmp/w/package.json` with the checkout at `/tmp/w/x`),
  `TestSealedRootChecksOutAncestorsChildren` (`$HOME/work/package.json`).

`GOWORK=off` (the BUILD policy), `/opt/rh`, `/include` and `/pyvenv.cfg` (the
absent-ok list) are now redundant for the classifier; they stay, because they
also keep the tools from looking for what is not there.

Tests: `internal/traced/sealed_test.go`, `internal/record/sealedroot_test.go`,
`sandbox_sealed_linux_amd64_test.go`; the fuzzer model now seals everything above
the mounts (EROFS), seeds `/var/tmp` and `/run`, treats scratch directories as
fresh tmpfs, and has two stale-cwd bursts and one scratch-absence burst aimed at
the sealed root.

**Decision D42: absent before made, metadata writes by path, and ephemeral
device nodes (real-build census, `unstable-path:final-changed` in 31 actions;
`kiln:recorder` 9 to 10).** The paths were C and C++ build outputs
(`code/packages/{c,cpp}/*/_build/{gcc,clang}-<name>-tests`, and `_build` itself
on newer images). The linker or driver `stat`s its output, gets ENOENT, creates
it with a plain `O_CREAT` open (or `mkdir` for `_build`), then stats, chmods and
runs it. A successful stat never enters the followed-final set (it is relaxed,
section 5.5); the only stat that does is one that FAILED while the post-run disk
shows the name present, and the earliest lookup then predates the creation, so
`createdOnce` fails. Separately, a shell `>/dev/null` is an `O_CREAT|O_TRUNC`
open, which logged a namespace change for `/dev/null` and raised `final-changed`
and `own-overlap` on it.

*Rule 1, absent before made (`absentBeforeMade`, `classify.go`; sub-code
`absent-before-made`).* A call that FAILED with ENOENT on its final name `p`,
where the post-run disk shows `p` present, is stable when ALL hold:

- `p` is untracked (not a repository-tracked file) and was never a
  `link()`/`linkat()` source or destination;
- `p`'s change log is not empty and EVERY entry STARTED after the latest such
  lookup COMPLETED (the interval of the lookups is the earliest start to the
  latest completion; a split call carries its `<unfinished ...>` line as its
  start, so this uses the same interval machinery as `unsafe`, never line
  numbers compared naively);
- the first entry is a creation that proves the action made the name: a clean
  creation (mkdir, symlink, `O_EXCL` open) or an `O_CREAT` open keyed by the
  descriptor's -y path;
- no removal of `p` completed before that creation (`removedBeforeMade`).

Otherwise it is judged by `createdOnce`, which is today's rule, so a verdict
that was stable stays stable. Only calls whose ENOENT speaks for ONE final name
feed the rule: `rename`, `link` (two names) and `execve` (the missing name may
be the interpreter) never do. `classifier.callErrno` carries the errno.

*Why it is safe.*

1. The ENOENT says nothing was at the name when it was looked up, and the
   absence is still recorded as an absent observation (`absentObs`, for the merge
   job) by `observeResult`, exactly as before.
2. It cannot have been a dangling symlink that the creating open then wrote
   through: an `O_CREAT` open that followed a link is keyed by its -y path, which
   is the link's TARGET, not this name, so an entry logged at `p` proves `p` was
   not a link when that open ran. A link that was removed and replaced is a
   removal before the creation, or a first entry that is not a creation.
3. Every change-log entry starts after the lookup completed, so nothing the
   action did to the name can have taken effect while the lookup ran, and the
   first entry proves the action made the name: before it the name was absent
   (or, for a plain `O_CREAT`, a regular file that is now the action's own).
4. Every directory and link the walk crossed is a traversed component and keeps
   the strict rule (`stable`), so the ENOENT cannot have come from a missing
   ancestor that was created later.
5. The lookups of calls that can write are judged again on their own interval
   (the `wr` set), so a failed `chmod`/`open` that could not be placed still
   voids the write set (W8) when the rule fails.

*Rule 2, metadata writes by path (F3).* A SUCCESSFUL `chmod`, `fchmodat`,
`fchmodat2`, `chown`, `fchownat`, `utimensat`, `utimes`, `futimesat`, `utime` or
`setxattr` that follows its final name sets `classifier.relax` exactly as a
successful stat does, so its followed final goes through the open-only rule
(section 5.5) instead of `createdOnce`. This is the linker's post-link chmod.
Under the open-only conditions (every log entry an `O_CREAT` open keyed by its
-y path, the first completed before the lookup began, at most one trailing
unlink that started after the lookup, no hard link) the name is provably the
regular file the action made, so the metadata write lands on it. The write
itself is still recorded and still observes a file the action did not make
(`writes-tracked`, untracked observation), and the no-follow forms (`lchown`,
`lsetxattr`, `AT_SYMLINK_NOFOLLOW`) are unchanged.

*Rule 3, ephemeral device nodes.* `openResolved` does not log a namespace change
for an `O_CREAT` open of a path that is under `/dev/` AND matches the host's
ephemeral list (`ephemeralDevice`, `ops.go`; `/dev/null`). Both tests are
needed: the list says the name is no input, `/dev/` says it is a device node and
not a file a build could have made. Such a name is never created, replaced or
read as content, so nothing the stability rules guard can depend on it; the write
is still recorded.

Tests: `internal/traced/absent_before_made_test.go`; the fuzzer model has four
new bursts (stat ENOENT then create, `mkdir -p`, post-link chmod, dangling link
replaced), and with the timing condition disabled `TestDifferentialSeeded` fails.

**Decision D40: tmpfs containment (real-build census, `unstable-path` in about
43 actions, all writes-incomplete; `kiln:recorder` 7 to 8).** Every high-count
path was the action's own scratch file inside the fresh tmpfs: Go's build cache
(`$HOME/.cache/go-build/HH/<hash>-a`, `-d`, and its directories),
`$TMPDIR/go-buildN/...`, gcc's `$TMPDIR/ccXXXXXX.{o,s,res}` and clang's
`name-hash.o`. Such files are re-created without `O_EXCL`, created and then
removed, or renamed into place, which breaks the "created exactly once" and
open-only rules of §5.1 and §5.5 (sub-codes `final-changed`, `own-overlap`,
`component-changed`, `absent-changed`, `followed-absent-changed`,
`removed-before-made`) and, through W8, voids the write set.

*The rule.* If the action made no symbolic link inside the fresh `Home` or
`Tmpdir`, AND made no directory rename or foreign rename there, the stability
rules do not apply to paths inside it. Two whole-action flags say otherwise:

- `classifier.tmpLinked`, set wherever a link is recorded at (or carried by
  `rename`, `RENAME_EXCHANGE` or `link` to) a path that is `underTmp` or a tmp
  root (`noteTmpLink`, `ops.go`);
- `classifier.tmpMoved`, set by a SUCCESSFUL `rename`, `renameat`, `renameat2`
  or `RENAME_EXCHANGE` with an endpoint under the tmpfs whose source is a
  directory, or an object the action did not create (`noteTmpMove`). Renaming a
  regular file the action created, into place (Go's build cache, gcc, clang),
  sets nothing; an exchange needs BOTH names to be such files.

`classifier.contained(p) = !tmpLinked && !tmpMoved && underTmp(p)`; `judge`
skips a contained path at the top of each of its eight loops (`seen`,
`seenAbsent`, `seenFinal`, `relFinal`, `relAbs`, `gone`, `seenFile`, and
`ownSeen`, which tests the looked-up path). `stable` is unchanged (its one
caller, the `seen` loop, tests `contained` first, so a version of `stable` that
also tested the flags would be dead code). `checkStability` runs once after
every event has been classified, so both flags are whole-action facts when any
`judge` runs; with either set, every verdict is today's. `exec-unreadable:under-tmp`
is unchanged. (D43 widened `underTmp` to the scratch directories `/tmp`,
`/var/tmp` and `/run`, which are fresh per-step tmpfs mounts like Home and
Tmpdir, so containment, and the two taints, apply to them too; see D43 point 3.) `fresh` also refuses the recorder's derived state roots
(`~/.cargo/registry`, `~/.cargo/git`, `stateAnchors`), which are writable HOST
binds passed as `traced.Config.HostBinds` (`record.DerivedStateRoots`); an
absent probe of one is still allowed (they start empty), a present look is
`host-read`.

*Why it is safe.*

1. Under the fresh tmpfs the classifier never asks the disk (`lookup`,
   `resolve.go`): a name there is something the action made (`links`, `dirs`,
   `created`) or it is absent. `fresh` excludes the checkout and its ancestors,
   every declared root and every `HostBinds` path, and their ancestors, so no
   host bind beneath `$HOME` is in the set.
2. A lookup can leave the tmpfs by two routes only: a symbolic link located
   inside it, or a directory that was renamed so that a path resolved physically
   (`..`, a process's cwd) no longer matches the trace's TEXT. The trace keeps
   each cwd as a text expression that a directory rename never updates, while
   the kernel resolves `..` against the real directory; a renamed and re-created
   name makes the stale text point at a different directory, with no link
   anywhere. (Example: `chdir $HOME/a/b; rename $HOME/a/b $HOME/b2;
   mkdir $HOME/a/b; unlink ../work/repo/f` reaches the bound checkout
   `$HOME/work/repo/f` while the text says the action's own `$HOME/a/work/repo/f`.)
   Hard links and renames from outside cannot exist: the scratch tmpfs are
   separate mounts (and the checkout and each state root are binds), so they fail
   with `EXDEV` (the fuzzer model says so).
3. So with no link and no directory or foreign rename there, every component of
   a path under the tmpfs is a directory the action made in place (or the tmpfs
   root), and a path reaches the action's own files or nothing: neither an input
   nor a repository write. `..` out of it is exact because no component moved.
   Renaming a regular file the action created cannot change what any path
   component or cwd is. A directory or foreign rename sets `tmpMoved` and the old
   rules (a renamed directory has two changes and fails `createdOnce`) apply to
   the whole action. Tests: `internal/traced/tmpmoved_test.go`; the fuzzer's
   burst 13 and its `unsound` flag check that a path resolved from a stale cwd is
   never recorded without a reason.
4. Nothing but the action's own files is reachable under `Tmpdir` or `Home`
   inside the sandbox, with ONE exception, the checkout's ancestor chain under
   `Home`. When the checkout lies under `Home` (hosted runners), the sandbox
   makes the chain (`$HOME/work/x`) as plain, writable directories of Home's
   tmpfs and binds the checkout on top; only the anchors of read-only roots
   are sealed. If the action could move that chain (`mv ~/work ~/y`), the
   checkout root would be reachable at a name the action "created", and a
   listing of it would count as own output. It cannot be moved unobserved:
   D39 rule (a) covers LOOKS only, so a rename, rmdir, unlink, chmod, utimensat
   or any other write-class call whose path is an ancestor is `host-read` (and
   writes-incomplete), exactly as before D39 (`observeHost`, `!c.inWrite`;
   `internal/traced/ancestor_write_test.go`; the fuzzer model now names the
   checkout's parent and checks it). The staging directory `$TMPDIR/.kiln-newroot`
   (`internal/sandbox/plan.go`, `StageDirName`, `StageRoot`) is built on the
   STAGING tmpfs mounted on the OLD root's `Tmpdir` (`KindStageTmpfs`,
   `KindStageDir`, `KindRootTmpfs`, `plan.go` in `Plan`); the step's own
   `Tmpdir` is a second, fresh tmpfs mounted inside the new root
   (`KindTmpfs`), and `pivot` detaches the old root (`MNT_DETACH`,
   `sandbox_linux_amd64.go` `pivot`). The stage directory is therefore not visible
   from inside. The read-only roots beneath `$HOME` are bound at declared paths
   (`homeRoRoots`), which `fresh` excludes; the anchor tmpfs that hold them are
   read-only, and a `$HOME/.cargo`-style anchor's missing parents are directories
   the sandbox, not the action, made (a present look there is an ancestor node
   under D39 rule (a) or a declared root's own rule).

5. *Assumption: the parents of `Home` and `Tmpdir` are immutable.* `noteTmpMove`
   ignores a rename with no endpoint under `Home` or `Tmpdir`, which is sound
   only because nothing the action can rename sits above them: the new root
   tmpfs is remounted read-only before the pivot (`KindSeal`, `plan.go` in
   `Plan`, after the `KindProc` step), so `/`, `/home`, `/tmp` and every other
   directory of it cannot be renamed, created in or removed from, and each
   mount point (`Home`, `Tmpdir`, the checkout bind, the state roots) is
   `EBUSY` to rename or remove. A future sandbox layout that made an ancestor
   of `Home` writable (a writable root, or a `Home` that is a plain directory
   of a writable tmpfs) would reopen a stale-text path, because a rename of
   that ancestor would taint nothing. Such a change must extend `noteTmpMove`
   first.

The census records, for an unstable path, the op class of the look that ENTERED
the lookup set that failed the rule (`noteLookupOp(set, abs)`), not the first
look of the path in any set; this is census-only and the record bytes are
unchanged (`TestCensusDoesNotChangeTheRecord`). `kiln:recorder` is 8. The
differential fuzzer's model treats any tmp lookup as own-or-absent while no link
was made in the tmpfs. Tests: `internal/traced/tmpcontain_test.go`.

**One explicit image label.** The record workflow and every job that consumes
its records use one explicit `ubuntu-NN.04` label, never `ubuntu-latest`: the
host's image id is part of the weak key, so a floating label would change the
key under the recorder without a commit. Which label is chosen is decided in
stage-4 PR 3.

Any other out-of-repository read or probe gives `host-read:<path>` and no
record. `$HOME` and `TMPDIR` are expanded from the scrubbed environment.
The file gates reuse, so on trusted runs it is read from the base revision
via `Source`, never from the working tree (KILN03 D6).

### 5.4 Network

Network activity is classified **by address, never by return value**:
curl, Node, Go and asyncio all get `EINPROGRESS` on a non-blocking
`connect`.

| Observation | Result |
|---|---|
| `connect`, `sendto`, `sendmsg` or `sendmmsg` naming a non-loopback address, or port 53 on any address, whatever it returned | network use: `network:<program>`. From stage 5, it is allowed when the process's program (the real path of its last `execve`, under a `tool` root) is a `fetchers` entry of a pinned ecosystem. For an interpreter-hosted fetcher such as npm, the entry names the interpreter and the script. |
| a successful loopback or Unix `connect` to an address the action's own processes bound | allowed (a test server) |
| a successful loopback or Unix `connect` to an address the action did not bind | `daemon:<address>` |
| a loopback or Unix `connect` that fails with `ECONNREFUSED` or `ENOENT` | allowed: it observes host state the image fixes (§14) |

`EINPROGRESS` and `EAGAIN` count as success. Datagrams to loopback follow
the same "bound by the action itself" rule as streams. Any socket
operation outside two families gives `network:<program>`. The allowed
families are pathname `AF_UNIX`, and `AF_INET`/`AF_INET6` under the rows
above. Examples outside them: `AF_NETLINK`, which reads routes and
interfaces that no key covers, and `AF_PACKET` and `AF_VSOCK`. An
abstract-namespace Unix socket the action did not bind gives `daemon:`.

"Bound" has three further limits. A bind to **port 0** matches nothing: the
kernel chose the port and the `bind` line does not say which, so a client of
such a server reads as a daemon (learning the port from `getsockname` is
future work). A wildcard bind is **family-scoped**: `0.0.0.0` serves IPv4
only, `::` serves both families. The **socket type** must match, read from the
`socket()` call through the socket's `-y` name (`socket:[inode]`); an unknown
or ambiguous type matches nothing.

Every package that gets
`network` is reported, which replaces the hand-curated network entries in
`never.json`.

### 5.5 Classification decisions (stage 1b-i)

Stage 1b-i (`internal/traced`) turns the parsed traces of one action into a
**raw record**. It is pure: the traces, the expected recipe lines, the package
index, a tracked-tree view, a post-run disk view, `host-roots.json`, the
pinned ecosystems, the weak key's tool set and the scrubbed `HOME` and
`TMPDIR` all arrive as arguments. It computes no digest and applies no
cross-action rule (stages 1b-ii and 4). The rules themselves are in §4.4, §5.1
to §5.4 and §7; this subsection lists what the implementation settled where
they left room. The code is authoritative for details.

**Stage 1a additions.** `Event.Argv` (an exec's decoded argument vector),
`Event.StartLine`, `Event.SockPath`, `Trace.StepEvent()`, and the descriptor
metadata writes in the table.

**The record.** `Entries`, `Writes`, `TrackedWrites`, `UntrackedObservations`,
`AbsentObservations`, `Traversed` and `Reasons`, sorted and unique. Beyond §7's
list: `TrackedWrites` holds the exact tracked paths written (the tainted-set
candidates; `Writes` holds write roots only); `AbsentObservations` and
`Traversed` exist for the merge job's cross-action checks. The traces of one
action are classified as one sequence. `Recipe` is a required input: without
it nothing shows the steps are the ones the weak key covers.

**Resolution.** The walk consults, in order, the trace's own links and
directories (only under `$HOME` and `TMPDIR`), the post-run disk, and the
tracked tree for a path the disk lost. The strict stability rule of §5.1
replaces any use of "what the trace knew at the time": the disk must vouch for
every traversed component. Simulating renames, hard links and concurrent
lookups exactly is unbounded work and every gap is a false hit, so builds that
churn temporary directories lose their records (the relaxations below recover
the common cases); the M2 reason histogram measures what is left. `/proc` and `/dev/fd` are never walked. `Config.Root` must
be a real path, and `prepare` checks that every prefix of it is a real
directory on the disk. `openat2` with a non-zero `resolve=` is
`trace-incomplete:resolve-flags`.

**Mapping.**
- A path is the action's own if it, or a directory above it, was created
  earlier in the trace. An open that cannot prove emptiness (`O_WRONLY|O_CREAT`
  alone, `O_APPEND`) is an observation.
- `rename` and `link` write and observe their source; `RENAME_EXCHANGE`
  creates nothing; a descriptor-based or `/dev/fd` path in a Write-kind call
  is `trace-incomplete:proc-path`.
- An observation of an untracked present path gives its entry **and** an
  untracked observation. The repository root records only `list:.`; `.git`
  itself records nothing; any path beneath a `.git` segment is `vcs-read`; a
  repository path KILN02 §3.5 rejects is `trace-incomplete:bad-path`; a write
  beneath `.git` is `trace-incomplete:vcs-write`.
- A write root is the shortest prefix of the written path that is not tracked.

**Exec.** An ELF's `PT_INTERP` is read from its program headers with the
`debug/elf` structures (not `elf.File`, which reads the section headers at
the end of the file). A program that cannot be read, an unknown format or a
malformed ELF is `exec-unreadable`, `exec-format` or `exec-elf`. A compiled
then deleted program (autoconf's `conftest`) therefore loses its record.

**`/proc`.** Only `/proc/self/status` and `/proc/self/stat` are
`trace-visible`; the ephemeral list comes next; every other `/proc` path,
including `/proc/self/fd` itself and the bare `/dev/fd`, is
`trace-incomplete:proc-path`. An open of `/proc/self/fd/N` or `/dev/fd/N` is
classified by its `-y` path.

**Out of the repository.** The order is: made by this action, ephemeral list,
`$HOME` or `TMPDIR` themselves, longest covering root, `absent-ok`, else
`host-read`. `X/**` in the ephemeral list also covers `X`; `absent-ok` covers
a path, and what is under it only while the entry itself is absent (§5.3), for
absent probes only. An `image` file with a
zero ctime or a failed lstat is `host-read`. A `state` root reads as
`host-read` and an absent probe under it is allowed (the fetcher check is
stage 5; `fetchersEnabled` is the switch); a write under it is `state-write`.
An absent probe under `$TMPDIR` that is not the action's own is a
`host-read`. Reasons redact `$HOME` and `TMPDIR`, replace every non-printable
rune and are capped at 200 bytes (checked before each rune is appended).

**Network.** The wildcard address counts as local for a connect. A bind
counts only if it succeeded earlier in the trace; port 0, family scoping and
the socket type are in §5.4. A pathname `AF_UNIX` bind also creates the path.
Inside the repository a Unix connect is also a path observation. Any failure
other than `ECONNREFUSED` or `ENOENT` is judged like a success. The program in
`network:<program>` is the process's last exec path, absolute and redacted.

**Reasons from the step itself.** A step whose init child (or root) did not
exit with 0 gives `failed`. A descendant outside the init child's thread
group with an exit line after the init child's `exit_group` line gives
`daemon-left` (§4.3, D19). `CheckRecipe` accepts an argv of
exactly `[<shell>, "-c", <line>]`; `argv[0]` is not compared.

**Relaxations.** The strict rule of §5.1 refuses several very common
patterns: a compiler output that is created and then stat'ed, a temporary that
is created, read and unlinked, and `stat($HOME)` when the checkout lies under
`$HOME`. Three narrow relaxations admit them. Each applies only when its
precondition provably holds; anything else stays on the strict rule.

- **(a) Opens settle the final name.** For a **successful** open whose `-y`
  path equals the walked path, the final component is neither noted as a
  followed final nor as a missing stop. The kernel's own path proves that the
  name was not a link at the moment of the open. Intermediate components, and
  every link the walk crossed, keep the full rule. A `-y` path that differs
  from the walked path, a failed open, and an open of `/proc/self/fd/N` settle
  nothing.
- **(b) Open-only change logs.** Take a followed final name of a **successful**
  `stat` (or other metadata call that follows links) or `exec` whose post-run
  disk state is a regular file. It is accepted when its **whole** change log
  is `O_CREAT` opens keyed by their `-y` path (an `O_CREAT` open through a
  link is logged at the link's target, so an entry at the name proves it was
  no link; an `O_PATH` open is never an `O_CREAT` entry, since the kernel
  ignores `O_CREAT` there, and an `O_NOFOLLOW` open does not prove the name was
  not a link, so neither is one), optionally ended by exactly **one** `unlink`/`unlinkat` that
  **started after the latest lookup of the name completed**; and
  - the first open **completed before** the earliest such lookup **started**;
  - the disk agrees with the log: the name is present if there was no unlink,
    and absent if there was one (a name absent on the disk is then not a
    "missing stop" of the lookup);
  - the name was never a `link`/`linkat` source or destination.

  Any other entry keeps the strict rule for that name: a `rename` (either end),
  `RENAME_EXCHANGE`, `link`, `symlink`, `mkdir`, `mknod`, an `unlink` before
  or overlapping a lookup, a second `unlink`, an `O_CREAT` open that is not
  keyed by a kernel path, or a lookup that began before the first open
  completed. The ancestors need no extra rule: every lookup went through them,
  so each already had to be stable by §5.1 (a created directory needs exactly
  one `mkdir` that completed before the first lookup). Other followed lookups
  (`chmod`, a failed call, `chdir`) are not relaxed.
- **(c) `$HOME` itself.** A stat or listing of the `Home` directory node is
  allowed even when the checkout lies beneath it (hosted runners). It is the
  sandbox's tmpfs mount point, the same node in every run. Listing it is
  allowed too: it is the fresh tmpfs, whose contents are the same in every
  run apart from the mounts the sandbox itself makes. Only the node: the
  checkout's other ancestors and everything under `$HOME` stay `host-read`,
  **except as D39 now allows**: a present, non-listing look at a proper
  ancestor of an anchor, and an absent probe strictly inside the fresh `$HOME`
  or `TMPDIR`. A listing of an ancestor, a child of an ancestor and a present
  uncreated name under `$HOME` still give `host-read`.

**Tests.** A differential fuzzer runs an in-memory filesystem model with the
true results and `-y` paths and gives the final state as the post-run disk. It
models two processes with overlapping calls (the effect lands at a random point
of each call's interval; events are in completion order with `StartLine`),
one or two steps, a randomised `Root` (also under `HOME`), `HOME` and `TMPDIR`,
tracked files, pre-existing untracked leftovers (file, directory, link), host
files, changing file contents, `RENAME_EXCHANGE`, `O_TRUNC` without `O_CREAT`,
dirfd-relative lookups and listings. "Bursts" issue the patterns of the
relaxations (a kept output, a temporary, an exclusive temporary, an exec of an
output, a stat of `$HOME`) on one name, each operation with its own process and
overlap, and may add a perturbation (rename onto the name, hard link, symlink,
exchange, second unlink). When the record has no reason, every path
the model really looked at must be the action's own or be covered by an entry,
an untracked observation or a `list:` entry; every absence must be in
`AbsentObservations`; and every write must lie under a write root. A mutation
check makes it fail when the overlap logic is disabled, and likewise when any
one precondition of a relaxation is dropped.

**`writes_complete` (stage 4, PR 4).** The classifier reports whether it can
vouch that `Writes` and `TrackedWrites` list *every* in-repository path the
action wrote. It is `RawRecord.WritesComplete`, a Go-only field (`json:"-"`:
the recorder copies it with `recfmt.CompleteOf` to the shard record's
top-level `writes_complete`, §7.1, so the record's own JSON is unchanged; the
key is **required** for recorder version 2, and `recfmt.EncodeRecord` refuses a
record whose key is missing or disagrees with the classifier). It is **true by
default and fails closed**: the zero value is `false`, and `Classify` leaves it `true` only for
a record in which none of the following happened. Each trigger is a way a
write could exist that the lists do not show.

- **W1. An unresolved write.** A Write-kind call (§4.4's table, including the
  descriptor writes) whose target cannot be placed: the walk refuses
  (`symlink-loop`, `size`), a `..` follows a component the walk could not enter,
  the path is empty or unresolvable. A no-follow call other than the
  parent-based create and remove calls (`lchown`, `lsetxattr`, `lremovexattr`,
  and `utimensat`, `fchownat`, `fchmodat2` with `AT_SYMLINK_NOFOLLOW`) keeps a
  trailing slash: the kernel's lookup of a final name ending in `/` follows a
  symlink and demands a directory, so the call changed the directory the link
  points to, not the link. Only `mkdir`, `mknod`, `symlink`, `unlink`, `rmdir`
  (and their `at` forms), which act on a name in its parent, drop the slash.
  The trigger holds whatever the call's result:
  a failed call that could not be placed is not proved to have changed
  nothing.
- **W2. An unresolved creating or writing open.** An `open`, `openat`,
  `openat2` or `creat` that can write (`O_WRONLY`, `O_RDWR`, `O_CREAT`,
  `O_TRUNC`, `creat`) and cannot be placed, or an `openat2` with a nonzero
  `resolve` field. A read-only open that cannot be placed is only a read-side
  reason.
- **W3. An unresolved `rename` or `link` endpoint**, either name, whatever the
  result.
- **W4. An unresolved Unix `bind`.** An `AF_UNIX` pathname `bind` whose path
  cannot be placed, or a `bind` the parser gave no socket address.
- **W5. A virtual write.** A write whose path lies under `/proc` or
  `/dev/fd`, other than the `/proc/self/fd/N` and `/dev/fd/N` opens that return
  a `-y` path (those are classified by that path), and the re-opening of a
  descriptor whose `-y` text is `pipe:[`, `socket:[` or `anon_inode:`
  (`> /dev/stderr`): such an object has no place in any directory and cannot
  name a repository file. Every other non-absolute annotation stays unknown.
  Otherwise the name says nothing about the object written. This covers Write-kind calls, writing opens,
  `rename`/`link` endpoints, a `bind` path and an `O_TMPFILE` file linked back
  by `linkat`.
- **D28. `/proc` magic links and `..`.** `/proc/self/cwd`, `/proc/self/root`,
  `/proc/<pid>/{cwd,root,exe}`, `fd/N`, `map_files/X`, `ns/X`, `thread-self`
  (a link to `/proc/<pid>/task/<tid>`) and `/proc/net` (a link to `self/net`)
  are links the kernel follows to wherever the process points them, and the
  kernel takes a `..` after one from the TARGET's parent. Read lexically,
  `open("/proc/self/cwd/../../filesystems")` is `/proc/filesystems` (an
  allowed read) while really reading a file two levels above the working
  directory, so the record would stay clean over an arbitrary host read. The
  classifier fails closed: (1) a `..` after any `/proc` component is
  `trace-incomplete:proc-path` unless exact (`/proc/..`, `/proc/self/..`,
  `/proc/<pid>/..` keep resolving lexically; every deeper `..`, and every `..`
  under `/dev/fd`, is refused); (2) a path through a magic link is `proc-path`
  even when an ephemeral pattern matches it, `/dev/fd/N/x` counted as
  `/proc/self/fd/N/x`; (3) a successful `/proc` open is checked against its
  `-y` path (equal, or `self`/`thread-self`/`mounts`/`net` spelled with pids,
  passes; another path is classified as the file really read, or `proc-path`
  if it is another `/proc` file), and an open of a magic link itself with no
  usable `-y` path is `proc-path`. Nothing relies on `/proc/thread-self/..`.
- **W6. A path the record cannot hold.** `bad-path` (a path `key.ValidatePath`
  refuses) or `vcs-write` (a write under `.git`).
- **W7. A write to the repository root itself** (`chmod .`, `utimes .`). The
  root has no write root to coarsen to; the fail-closed answer is "unknown"
  rather than a special case.
- **W8. An unstable component under a write.** The stability rule (§5.1) is
  applied twice over the same change log: to every lookup (an unstable one is
  the `unstable-path` reason) and to the lookups made by calls that can write,
  on their own intervals (an unstable one means a write may have landed
  somewhere else). A lookup only a read made costs a write nothing, and
  probing a name before creating it (`stat out/x` ENOENT, `mkdir out`, create
  `out/x`) is judged by the write's own, later lookup. The verdicts are
  monotone in the interval (a shorter span or a later first lookup is only
  safer), and what a write's placement depends on is exactly its own lookups,
  so the narrower judgement is as sound. A post-run lookup that failed
  (`unresolvable-path`) while a write was being placed, and every
  `trace-incomplete` reason raised while a write call is processed, also
  count.
- **W9. A list or budget cap.** `Writes` or `TrackedWrites` would exceed the
  65 536-entry cap (this adds the `size` reason, as the other lists do), or the
  walk or stability budget ran out (`size`): the stability pass stops early, so
  nothing vouches for the components it did not reach.
- **W10. A whole-trace problem.** A step's trace has no exit line for the step
  process or the step was killed by a signal (a truncated or interrupted
  trace), some traced process has no exit line, **any** traced process
  dumped core (the kernel writes the core file with no system call), a
  survivor outlived the sandbox's init (`daemon-left`), or the traced steps do not match the weak
  key's recipe (`recipe-mismatch`). A nonzero exit status by itself is **not**
  a trigger, and neither is a child killed by a signal other than the step
  process (`yes | head` dies of SIGPIPE): every call up to its death is traced
  and a call it never finished is refused (L1, M5): the failed action wrote what it wrote, and the merge treats the
  failure as a truncated execution (§7.1, M-1). A trace that did not parse, or
  a tracer that did not finish cleanly, never reaches `Classify`; the
  recorder reports those as `unknown_writes` and `clean: false`, which the
  merge already treats as unknown.

**W11. An untraced peer.** The write set is unknown after a connect or send
that *succeeded* to a peer the action did not bind itself and that may be a
host process, outside the trace, writing the repository on the action's
behalf:

- a pathname `AF_UNIX` socket outside the fresh `HOME` and `TMPDIR`, **inside
  or outside the repository**: every earlier step's processes are dead and a
  listener in the same step would have its bind in the trace, so an unbound
  in-repository peer is no other action's but a host process or an alias of
  one (L7);
- a loopback inet or abstract Unix peer, **unless the step ran in a fresh
  network namespace of its own** (`Input.StepNetNS`, below).

`Input.StepNetNS` is a precondition the classifier cannot check, so it is an
input that fails closed: the default `false` treats unbound inet and abstract
peers as W11. Only the recorder sets it, and only when the step ran under the
stage-3b sandbox (`kiln sandbox-exec`, which unshares `NET` and leaves only a
loopback: §4.7). Under a fresh namespace a loopback or abstract peer
nobody bound cannot be a host daemon and keeps just the plain
`daemon:<address>` reason, which the cross-action rule M13 handles; TMPDIR is
a fresh tmpfs per step. "The action bound it itself" is believed only while the
bound name is stable: the peer's lookups are judged with the write set's (L5),
so an action that swaps the bound name for a link to a host daemon's socket and
back is incomplete.

*Known precision limits (fail-closed).* A Unix-socket server that binds `P`,
connects to it, and then unlinks `P` is judged incomplete: the self-bound peer
goes through the write set's stability judge (L5), and a name with a later
removal is not stable. The planned fix is a trace-order check for the
self-bound exception instead of the stability rule: the last completed change
to `P` before the connect is the action's own bind, and no change to `P` or its
ancestors overlaps the connect. That is new soundness-critical logic and needs
its own review; until then the conservative answer costs only a cache entry.

**M3. A reason always accompanies an unknown write set.** Whenever the write
set is unknown and the record would otherwise carry no `trace-incomplete`
reason (a write to the root, a failed unplaceable call, a core dump, a daemon
peer), the classifier adds `trace-incomplete:writes-unknown`. The per-kind cap
keeps one reason of every kind, so it survives. This keeps the invariant "no
`writes_complete`, or `false`, implies a `trace-incomplete` reason" that a
merge reading only the reasons relies on.

Deliberately **not** triggers, because they cannot hide a write: any
read-only reason (`host-read`, `exec-unreadable`, a failed lookup of a name the
action only looked at), a nonzero exit, a child's death by a signal, a loopback
or abstract `daemon:` peer, `network:<program>` reasons (the
record is refused anyway and no peer is reached), and a write outside the
repository (it is `state-write` or the action's own scratch space, and cannot
name a repository path without going through a resolved, vouched component).

*Aliases.* A write through a hard link is covered case by case: an alias the
action makes during the run is tracked (the `link` source and destination are
both written, and `hard` makes neither a stable component; M10, M5, M11);
`O_TMPFILE` is refused by the parser or is W5 when linked back; a pre-run
untracked alias is excluded by the inventory (M9); tracked content changed
through any alias is caught by the post-build backstop (M-2b). The residual
risk is a metadata-only change (mode, owner, times, xattrs) through a pre-run
alias of a file the trace never names, which changes no content digest.

**The fuzzer invariant.** The differential fuzzer checks, for every record
whose `WritesComplete` is true *even when it carries reasons*, that every
in-repository path the model wrote lies under an entry of `Writes` or
`TrackedWrites`, using the classifier's own coarsening (the highest ancestor
holding no tracked file, or the tracked path itself). Perturbations aim at each
trigger: writes through links retargeted mid-run, writes after a `..` past a
missing component, writes through `/proc/self/cwd`, writes to the root, an
exchange under a written name, a write to a name another process removes
meanwhile, no-follow metadata writes with and without a trailing slash (the
model follows the kernel's rule), descriptor metadata writes, Unix `bind`,
writes under `.git` and to unrepresentable names, re-opens of pipes and sockets,
and processes that dump core. A mutation check (dropping a trigger) makes the fuzzer fail.


## 6. When nothing is recorded

A traced pass becomes a record only when every check passes. Otherwise
the action gets one or more of these reasons and **no manifest and no
pass**, and it runs again next time:

| Reason | Cause |
|---|---|
| `trace-incomplete:<detail>` | an unknown system call or unparseable line, a refused call (§4.4), a personality change, a truncated or non-UTF-8 path, an unmatched `unfinished`/`resumed` pair, a process with no exit line, an unstable or unresolvable path (§5.1), a disallowed `/proc` path, a trace over its size cap |
| `reads-product:<path>` | it read another action's or a setup step's leftovers (§7) |
| `writes-tracked:<path>` | it changed a tracked file (§5.2) |
| `host-read:<path>` | it read or probed outside the allowed host roots, or a file a setup step modified (§5.3) |
| `state-write:<path>` | a non-fetcher wrote under a `state` root |
| `network:<program>` | network use not allowed by §5.4 |
| `daemon:<address>` / `daemon-left` | it talked to, or left behind, a long-lived process (§4.3, §5.4) |
| `vcs-read` | it read beneath `.git/`; a commit-dependent answer is in no key |
| `trace-visible:<path>` | it read a file that reveals the tracer (§5.1) |
| `unscrubbed[:<detail>]` | the environment or the state roots were not as §4.2 and §5.3 require |
| `manifest-too-large` | more than 8,192 entries |
| `failed` | the BUILD did not pass |
| `precondition:<reason>` | a KILN03 hard reason: the never list, no BUILD for this OS, an unknown toolchain, a symlinked BUILD or a symlink leaving the closure |

At most 16 reasons per kind are listed for each action.

## 7. Other actions' leftovers

Legacy BUILDs share one tree. An earlier BUILD leaves `node_modules`,
`.venv` and Rust `target/` directories behind, and a later one may read
them. Such a read depends on **which actions ran before it**, and that is
in no key. KILN03 §8 called this order dependence. v1 refuses to record
it.

Shards run on separate machines, so the check cannot run per shard:

1. **Shards record raw.** Each `kiln-record` shard uploads **raw records**
   for its actions: entries, write set (coarsened to *write roots*, see
   below), observations of untracked paths, reasons and exit status. A
   shard writes no manifest and no pass.
2. **One job merges.** `kiln-record-merge` runs in the same workflow run,
   under `ledger-merge`'s trust rules.

   **Shard binding.** It recomputes the plan and the shard partition from
   the base revision. It accepts exactly one raw record per (shard,
   package) for every package the shard runs (its assigned packages and
   their prerequisites), each record carrying `{run, revision, plan
   digest, shard i/n}`. Only the assigned shard's record of an action is
   emitted; every copy is checked. Any mismatch, duplicate, missing record
   or extra record means nothing is emitted. The merge job computes every
   digest and strong key itself, from `Source`, and never trusts a digest
   from a shard.

   The merge job takes from each shard only what no other job can observe:
   the image id, the checkout path and the scrubbed environment, all
   written before the shard's first BUILD. It recomputes the weak key from
   them and from `Source`, and refuses the shard if it differs. The scrubbed
   environment's values travel in the shard's run artifact to the merge job;
   they are never printed.

   It then joins every shard's records and, for each action A, applies:
   - **Another action's write.** A observed a path that another action
     wrote, in any shard, before or after A: `reads-product`. Another job
     could run them in the other order.
   - **A changed listing.** A listed a directory in which another action
     created or removed an entry: `reads-product`.
   - **Pre-existing leftovers.** A observed an untracked path that existed
     before the first BUILD. Each shard inventories untracked and ignored
     paths, at directory granularity, before its first BUILD:
     `reads-product`.
   - **Unexplained.** A observed an untracked path that A did not create
     and nobody wrote. On a correct trace this cannot happen, so it is
     `trace-incomplete`.
   - **Overlapping write roots.** Two actions whose write roots overlap
     (one is a prefix of the other, or they are equal) both get
     `reads-product`: neither can say which one's output a later reader saw.
   - **Stability across actions.** The record's `Traversed` components and
     `AbsentObservations` are checked against the pre-existing inventory and
     against every other action's writes: a traversed component another action
     created, renamed, unlinked or re-linked is `unstable-path`, and an absent
     name another action wrote is `reads-product`.
   - **Daemons.** An action with a `daemon` reason makes every other action
     that wrote under a path it observed untracked, and every other action
     that bound a port, `reads-product:daemon-peer`: the server side depends
     on the client's bytes. The raw record's reasons are capped and
     sanitised, so the daemon's address cannot be matched against `Binds`;
     the rule is a conservative superset that needs only the reason's kind
     (§7.1, G11).
   - **Shared ports.** Two actions that bind the same inet address and port
     (`RawRecord.Binds`) both get `reads-product`: one's server may have
     answered the other's client. The kernel can also hand out the same port to
     different actions at different times, which the trace cannot tell apart.
   - **Unknown write sets.** An action's write set is *unknown* when a
     step's tracer did not finish cleanly (§4.3), a step's trace does not
     parse, the traced step count differs from `kiln-legacy-results`, or its
     raw record says `writes_complete: false`. Every cross-action rule above
     quantifies over the writes of every action in every shard, so one
     unknown write set means **the run emits nothing**. A record with
     `trace-incomplete` reasons and `writes_complete: true` loses only its
     own manifest. The classifier reports `writes_complete` (§5.5); while the
     recorder does not yet honour it (`HonourWritesComplete`, off), every
     `trace-incomplete` reason counts as unknown. (The earlier wording voided
     only the shard; that was unsound, see §7.1, G1.)
3. **All or nothing.** Only after these checks does the merge job emit
   manifests and passes. If any shard's records are missing, it emits
   nothing.

A **write root** is the highest ancestor directory of a written path that
contains no tracked files: `code/packages/rust/target`, for example, or
`code/packages/typescript/foo/node_modules`. A path written under another
action's write root counts as written by that action.

Rust packages share `code/packages/rust/target`, so under these rules
they may record nothing in v1. A v2 could union the writer's manifest into
the reader's (open question 3). The kill criterion (§13) decides whether
that is worth doing or whether Rust goes straight to M4.

### 7.1 Merge decisions (stage 4a)

Implemented by `internal/recfmt` (the record formats, strict decoding) and
`internal/recmerge` (`Merge`, pure: no I/O, no clock). The recorder is stage 4
PR 2b; the job that drives the merge is `kiln record-merge` (stage 4 PR 3a,
`internal/record/merge.go`), which verifies its own tree (B1), reads the three
build-tool exports once, decodes the shards, recomputes every key through
`legacy.Traced` (never trusting a shard's), calls `Merge`, and writes the
ledger only when the run is not void. It is a thin adapter on purpose: every
decision it could make is one `recmerge` already makes. The CI job around it
(`kiln-record.yml`, stage 4 PR 3b) is still to come, so nothing calls it yet. The design review found these
gaps in the text above; each decision below replaces it.

**G1. An unknown write set voids the run.** §7's rules are cross-shard
("another action's write, in any shard"). If action X in shard 3 has an
unknown write set and action A in shard 5 probed `p` absent, nothing shows
X does not create `p`; in a later single-tree job X may run before A, and
A's cached result would be a false hit. Shards are separate trees, so
voiding X's shard does not protect A. The merge therefore has three levels
of "no": *action* (a reason, no manifest), *shard-void* (nothing assigned
to that shard) and *run-void* (nothing at all). An unknown write set is
run-void (rule M1). `writes_complete` is modelled as an optional key of the
shard record line and a switch of `Merge`
(`Input.HonourWritesComplete`, off): with the switch off every
`trace-incomplete` reason is unknown; with it on, a record that says
`writes_complete: true` loses only its own manifest, while one that omits
the field is still unknown. The classifier now computes the field
(`RawRecord.WritesComplete`, §5.5, which defines it and lists every trigger);
the recorder copies it to the shard record's `writes_complete`. The switch
stays off until the recorder (stage 4, PR 2b) turns it on, so PR 4 changes
no merge outcome by itself. Because the record's semantics changed,
`RecorderVersion` became `2` here (it is higher now): every weak key changes and manifests recorded by
the version-1 recorder are never looked up. The read-side reasons that `writes_complete: true` may
excuse are the other `trace-incomplete` reasons (`exec-unreadable`, a
read-only `unstable-path`, and so on). **A recipe mismatch is never
excused**: when the steps that ran are not the steps the weak key's recipe
covers (B6, or the classifier's `trace-incomplete:recipe-mismatch`), the
trace says nothing about what the recipe writes. `writes_complete` is
required on the wire; a record that ran any step (`steps_run > 0`, built or
failed: it was classified) whose key is false or missing is an unknown write
set, so a hostile BUILD cannot choose a nonzero exit to downgrade it. Only a
record that ran nothing (`steps_run == 0`, refused or gated: never classified)
carries an explicit `false`, and that is a truncated execution (below), so the
relaxed verdict charges only that action. A step whose tracer did not finish cleanly
(`clean: false`) is likewise unknown, and a record must
carry exactly one step summary per step that ran (`len(steps) ==
steps_run`).

**Truncated executions (M-1).** A copy that did not run to completion has
an unknown write set: `status` other than `built` (a failed action stopped
part way even when every step ran, and the untraced build on `main`, which
does not fail here, would have run further), `steps_run` below
`steps_total`, or a `daemon-left` reason (a survivor may still write).
This is run-void.

**Unexplained tracked changes (M-2).** The shard's post-build backstop
counts the tracked paths that changed and that no record's `tracked_writes`
covers, as the header field `tracked_changed_unexplained` (a required key).
A non-zero count is a write no trace saw: run-void, like
`post_inventory_unexplained`. A `tracked-changed` void token whose changes
are all explained is only the shard-level void (M4).

**Second-order order dependence (M-3).** An action whose own record
depended on another action's recorded writes (any `reads-product` or
`unstable-path` reason derived by M5 to M14, or any reason in its own record
whose kind is not on the allow-list below) would write something else if the order changed, so its recorded write set
is one of several possible. Every cross-action rule quantifies over write
sets, so one such action makes every other action's check unreliable. The
strict verdict therefore treats it as an unknown write set: **run-void**.
The allow-list of own-record reason kinds that describe only the action's own
environment, and so do not mark it order-dependent: `failed` (its BUILD
exited non-zero), `precondition` (a gate or refusal), `network`,
`host-read`, `state-write`, `writes-tracked` (a taint source, not a
reader), `vcs-read`, `trace-visible`, `unscrubbed`, `trace-incomplete` (M1
decides the run), `daemon-left` (M-1 decides the run) and `manifest-*`.
Every other kind, a `daemon:<addr>` on the client, any spelling of
`reads-product` or `unstable-path`, and a kind the merge has never seen, is
order dependence (fail closed). That closes the class for emission. It is expected to zero the yield of
the first runs; any relaxation of it needs a lookup-time rule first (the M3
spec: an action that would have been order-dependent in the lookup job must
run), and that is the second-order item of §14.

**The relaxed verdict (report only).** `Merge` also computes, for the M2
report, what would be emitted if the M-1 and M-3 classes cost only their
action instead of the run (`Outcome.RelaxedEmittable`,
`Result.RelaxedRunVoid`). Every other cause stays run-void in it. It is
**never** turned into a manifest, a pass or a ledger record; only the
strict verdict feeds emission, and a test holds that line.

**Resource bounds.** A record path is at most 4,096 bytes and 128 segments,
holds no NUL and no U+FFFD (the same checks apply to the path part of every entry and to
the action name), a field may not be `null`, and every line is valid UTF-8 with no padding
(a BOM, CR or whitespace around the value is refused). The merge counts
every node and comparison of all its overlap walks against one work budget
(500 million units: ancestor probes weighted by prefix length, every visited
node by its owner count, descendant nodes by the query length, every reason
added by its length) and holds at
most 4 GiB of path text; exceeding either is run-void `size`. A record that
fails validation, or a shard whose index is outside the plan, voids the run
and nothing further is analysed. Caps are enforced before allocation: a line
is at most 64 MiB and a shard file 1 GiB, no list may exceed its cap and no
string 64 KiB (a token pre-pass refuses them before any struct is built), the
small files have byte caps, a record has at most 64 distinct reason kinds
(the classifier emits about a dozen), no element of a string list may be
`null` or empty, and the decoder stops once a shard holds more than its
retained-text cap. The retained-text count includes the host facts. The merge
keeps the exact number of distinct void causes but only the smallest 64 as
text. Cause text
is deterministic (validators report the first failure in a fixed order).

**G3. Shards are prerequisite-closed.** A shard runs its assigned packages
and their prerequisites (`ShardEntry.PackageNames`), so an action has a raw
record in every shard that runs it. Records are keyed (shard, action);
shard `s` must supply exactly one for every package in `PackageNames(s)`
(B4). Every copy takes part in every rule, a reason found on any copy is
the action's, and only the copy from the assigned shard becomes a manifest
and a pass, from that shard's host facts.

**G4. Weak keys are recomputed from machine facts.** `weakKey` hashes the
image id, `kiln:root` and the scrubbed environment's values, which exist
only on the shard. The shard writes `host-facts.json` before any BUILD; the
merge recomputes each weak key from those facts and its own `Source` and
requires the record's `weak` to match, or its `refusal` to be a refusal
the merge reproduces (B5). A mismatch is run-void. No digest is taken from
a shard. Image ids may differ between shards.

**G11. The daemon rule uses the reason's kind only.** `RawRecord.Reasons`
is capped (16 per kind) and sanitised, so a daemon's address cannot be
matched against `Binds`. If any copy of action C has a reason of kind
`daemon`, every other action with `overlap(u, w)` for `u` in the
untracked observations of any copy of C and `w` in its own writes, and
every other action with a non-empty `Binds`, gets
`reads-product:daemon-peer`. The reason kinds are decided explicitly:
`daemon` and `daemon-left` both count (`daemon-left` is also a truncated
execution, M-1). This is a superset of the address match only for binds the
record lists: `Binds` omits port 0 and abstract sockets. That gap is vacuous
only when the step ran in a fresh network namespace of its own (G10; the
stage-3b sandbox, §4.7; the classifier's `StepNetNS`), so no
marker is added in that case; only a pathname socket inside the checkout crosses
actions, and the untracked-overlap half catches it.

**The rules.** Notation: `anc(p, q)` is `p == q` or `p` is a *path*-prefix
ancestor of `q` (so `a/b` is not an ancestor of `a/bc`); `overlap` is `anc`
in either direction; `"."` (only in `list:.`) is an ancestor of everything.
`W(b)` and `TW(b)` are the unions of `Writes` and `TrackedWrites` over all
copies of `b`; `Tainted` is the union of `TW`; `Inv(s)` is shard `s`'s
pre-BUILD inventory.

| # | Level | Rule |
|---|---|---|
| B1 | run | The merge's own tree verifies and `HEAD` equals the workflow's sha (before `Merge`, which receives the regenerated digests). |
| B2 | run | One shard per index `0..N-1`, the index from the artifact name and equal to the header's and the binding's; `shard_count == N ==` the regenerated plan's. |
| B3 | run | `repository`, `run`, `revision`, `recorder` match; `attempt` is at most the merge's; `graph_sha256`, `steps_sha256` and `partition_sha256` equal the regenerated ones; `host_facts_sha256` and `inventory_sha256` are those of the files beside the binding. `kiln_sha256`, `build_tool_sha256`, `strace_sha256`, `git_sha256` and `sandbox_config_sha256` are **audit-only today**: the merge validates their form and compares none of them (there is no trusted value to compare with yet). |
| B4 | run | Every record line carries its shard's `binding_id` and is well-formed; the shard's actions are exactly `PackageNames(s)`, once each. |
| B5 | run | Weak keys recompute (G4). |
| B6 | run | A `built` record's `steps_sha256` equals the digest of the regenerated step lines, else `trace-incomplete:recipe-mismatch`, an unknown write set that `writes_complete` never excuses. |
| M1 | run | Any unknown write set (G1), including truncated executions (M-1) and an unclean step. |
| M2 | run | An untracked observation explained by neither `Inv(s)` nor another record's write **in the same shard**. |
| M2b | run | `tracked_changed_unexplained > 0` (M-2). |
| M3 | run | `post_inventory_unexplained > 0`, or a header `void` of `survivor`, `records-too-large` or an unknown token. Written by: `collect` (`survivor`: its /proc scan, P15; `records-too-large`: the size cap), and the post-build inventory count. |
| M4 | shard | A header `void` of `tracked-changed` or `state-preloaded`. Written by: `collect` (`tracked-changed`: the post-build checks; `state-preloaded`: the `traces/.state-preloaded` marker `step` leaves at sequence number 0, P12/P15). |
| M5 | action | An untracked observation overlaps `W(b)`, `b != a`: `reads-product`. |
| M6 | action | An absent observation overlaps `W(b)`: `reads-product`. |
| M7 | action | `list:d` with `w` in `W(b)`, `overlap(d, w)`: `reads-product`. |
| M8 | action | A traversed `t` with `w` in `W(b)` or `TW(b)`, `anc(w, t)`: `trace-incomplete:unstable-path`. |
| M9 | action | An inventory path overlapping an untracked or absent observation: `reads-product`; one that is an ancestor of a traversed path: `unstable-path`. |
| M10 | action | `overlap(w1, w2)` for `w1` in `W(a)`, `w2` in `W(b)`: `reads-product`. An action's own copies never collide. |
| M11 | action | A tainted `t` overlapping a `pkg:`, `path:` or `list:` entry: `reads-product`. |
| M12 | action | A tainted path overlapping the action's package, a `.gitignore`, the build tool or `code/kiln`: `reads-product`. |
| M13 | action | Daemons (G11). |
| M14 | action | Two actions with the same `Binds` string (type, address, port): `reads-product:shared-port`. |
| M15 | action | The record's own reasons, a status other than `built`, a gate, a refusal. |
| M-3 | run (strict) | Any action that received a reason from M5 to M14 (or carries `reads-product` or `unstable-path` itself). |

M5 to M14 are the order-dependence reasons; M15's reasons describe the
action's own environment and are not.

An action is emitted only when the run is not void, its assigned shard is
not void, no copy gave it a reason, and its assigned record's entries pass
`key.CheckEntryName`, fit the manifest caps and include `pkg:<action>` (a
manifest without it would still hit after the action's own sources
changed).

Rules are evaluated completely even after a void, so the report can name
every problem. Rule M13 reads daemon reasons per action rather than per
record: the copies of an action run the same BUILD, so a daemon seen in one
is assumed in all.

**Exec-then-rewrite by another action** needs no further data. An action
that execs its own product which another action later rewrites: the write
roots coincide or nest (M10). One that execs a tracked file another
rewrote: the file is tainted and the entry covers it (M11). One that execs
an untracked file it did not make: an untracked observation (M5, M9, M2).
A writer with an unknown write set: M1. Writes outside the repository
reach no other action (`$HOME`, `TMPDIR`, `/tmp` and `/run` are per-step
tmpfs, the `ro_roots` are read-only).

## 8. The manifest store

Manifests live beside the pass ledger (KILN00 §10), in a `manifests/`
subdirectory of the ledger directory. They use their own strict,
canonical JSON-lines segments: the same rules as pass segments except for
the caps below. Only the trusted `kiln-record-merge` job writes them, and
`ledger-merge` merges them. A manifest is a **store-only** record, not a
ledger kind: it can never cause a hit by itself, so it is not restamped,
and order is `(run, at)`.

```json
{"kind":"manifest","id":"9e0d…","weak":"3b1c…","target":"code/packages/go/parser",
 "os":"linux","arch":"x64",
 "entries":["list:code/packages/go","path:go.work","pkg:code/packages/go/lexer","pkg:code/packages/go/parser"],
 "revision":"0bf84a8…","run":"43117337685","at":"2026-11-12T03:00:00Z"}
```

- **Entries are names only**, sorted and unique. Digests are never
  stored: each is recomputed from the current tree at lookup, so a stale
  or tampered entry can only cause a miss.
- **`id`** is the SHA-256 of the canonical `entries` list. Retention keeps
  the newest K = 4 **distinct ids** per weak key, so scheduled runs that
  re-record the same paths do not crowd out the others. Records older than
  30 days are dropped.
- **Caps.** 8,192 entries per record, 512 bytes per name, 1 MiB per
  line, 64 MiB per segment and 256 MiB per store.
  - **Writers.** Every per-record cap (entry count, name length, line
    length) is checked when the record is emitted. A violation gives
    `manifest-too-large` for that action.
  - **Readers.** A reader that finds an over-cap record or segment drops
    only that record or segment and reports it. Only structural
    corruption makes it treat the **whole store** as unusable, so there
    are no hits. Structural corruption means a non-canonical line, an
    unknown field, or an over-cap store.
- **Names.** Entry names are repository paths and package paths. KILN00
  §10.1's "hashes only" becomes "hashes and repository path names": no
  file content, no out-of-repository path and no environment value is
  stored.
- **Run report.** The reason histogram (§13) is a run report, not stored
  records.

**Poisoning.** A forged manifest alone cannot produce a hit. The strong
key needs a pass record for exactly those entries' current digests, and
only a trusted run writes passes. A trusted run whose BUILD is malicious
is the KILN00 §9.6 threat, which tracing does not change. §4.3's
namespaces are what keep an action from writing the store or its own
trace.

### 8.1 Implementation decisions (stage 2)

Stage 2 lives in `internal/ledger` (`manifest.go`, `manifest_store.go`),
not a new package, because it reuses the ledger's unexported segment,
atomic-write, regular-file and `Authority` machinery. These points were
left open above:

- **`id` bytes.** The id is the SHA-256 of the compact JSON array of the
  entries (no HTML escaping), exactly the bytes after `"entries":` in the
  line. An empty list is `[]`, never `null`.
- **Entry names.** `pkg:`, `path:` and `list:` paths follow KILN02 §3.5;
  `list:.` is the only use of `.`. A name's 512-byte cap includes its
  prefix.
- **Error classes.** Structure is judged before caps. A line that is over a
  cap and also not canonical is corruption (the store is unusable), never a
  quiet drop. A line longer than 1 MiB is dropped without being parsed.
  A segment over 64 MiB is dropped without being read, but its bytes still
  count toward the 256 MiB store cap.
- **Order.** `(run, at)` compares `run` as a number when both runs are
  all digits (numeric runs sort before other runs, which compare as
  bytes), then `at`, then the ledger's segment sequence, segment name and
  line. A manifest is not restamped; the writer refuses `at` more than
  `MaxRecordSkew` after now, and `ledger-merge` refuses a delta record more
  than `MaxClockSkew` after now.
- **Retention and merge.** Expiry is applied first (`at` before now minus
  30 days), then the newest K = 4 distinct ids per weak key; a repeated id
  keeps only its newest record and does not use a slot. Merge writes each
  weak key **oldest first**, so file position (the last tie-break) agrees
  with the rank that decided the merge, and re-merging a snapshot with an
  empty delta reproduces it byte for byte. The merge reads its delta
  strictly: any structural error and any over-cap record reject it.
- **Far-future records.** A record dated after now plus `MaxClockSkew`
  would never expire and would outrank real ones, so a reader drops it and
  reports it (`ManifestReport.Future`, also listed in `Dropped`), and a
  `ledger-merge` snapshot containing one fails the merge, as in the pass
  `Merge`. `run` is capped at 128 bytes, the same as pass records.
- **Memory.** The reader applies expiry and the per-weak-key top-K
  (distinct ids) while parsing, so the heap holds at most K records per
  weak key plus the segment being parsed. The result is identical to
  sorting everything first.
- **Directories.** Every ledger read or write (pass and manifest stores,
  both merges) first `Lstat`s the ledger directory, the `manifests/`
  directory and a merge output directory, and refuses a symlink or
  anything that is not a plain directory; a missing directory is an empty
  store. Years below 0 are refused in pass and manifest records.
- **Lookup.** `OpenManifests(ledgerDir, scope).Load(now)` returns a
  `ManifestSet` and a report; `ManifestSet.Manifests(weak)` is the stage 5
  call (newest first, at most K). A load error means no hits.

## 9. Planning

`kiln legacy-plan` gains a lookup mode. For each Linux package it reports:

- the weak key, how many manifests it has, and the result: `hit`; `miss`,
  with the manifests tried and each entry's state now, for `kiln explain`
  (§9.1: the first differing entry is not computable); or `no-manifest`;
- the static class and reasons of KILN03, which are now a **diagnostic
  upper bound**: the static closure is what the old graph claims P can
  read;
- the advisory static hints (KILN03 §6), labelled as such;
- every traced `pkg:` entry that is **not** in P's static closure. Each
  one is a dependency the old build tool does not know about, and so a bug
  in the old graph (stage 5).

### 9.1 Lookup decisions (stage 5a)

Stage 5a is `kiln legacy-plan --ledger <dir> --scope shared|local`. It is
**report-only**: it skips nothing and changes no class. Without `--ledger`
the plan is byte for byte what it was. The plan schema stays
`kiln-legacy-plan/1`; everything below is an additive field, since nothing
consumes the plan yet.

- **Order.** For each package on a Linux plan (another OS gives
  `untraced-os`): gate first, then the weak key, then the manifests, then
  the strong keys, then one pass-ledger read for the whole plan.
- **Gating comes before any key or ledger read** (§13 item 5, §11.1 item 13).
  A package on the never list, with a KILN03 hard reason or precondition
  (`symlink:`, `unknown-repo-inputs:`, `unknown-toolchain:`,
  `no-build-file:`), or with a `probe-mismatch`, `probe-timeout` or
  `probe-refused` result when `--probe` ran, gets `ineligible:<reason>` and
  no lookup, even if a hit is waiting. If several apply the reported one is
  fixed: the never list, then the other hard reasons, then probes, each sorted
  bytewise. A refused weak key (`host-unknown`) is `ineligible:<refusal>`.
  A probe refusal gates but is **not** added to the record's `reasons`, so a
  plan made without `--ledger` does not change.
- **Results.** `hit` (with the manifest id and the pass record), `miss`,
  `no-manifest`, `untraced-os` and `ineligible:<reason>`. At most K = 4
  manifests are tried, newest first, and the tried list stops at the hit.
  The histogram counts each class; reasons that embed a name are grouped by
  prefix (`ineligible:never`), as for KILN03.
- **A miss cannot name the differing entry.** Manifests store names and the
  pass ledger stores only strong keys (§8), so what an entry's digest was
  when the pass was recorded is unknowable at plan time. A miss therefore
  lists, for each manifest tried, the strong key that was looked up and the
  entries in stored order, each with its **state in this tree** (`present`,
  `absent`, `directory`, `no-package`, `not-directory`; `unknown` if it could
  not be digested). That describes today's tree. It is not a comparison and
  the plan says so. This replaces §9's "the first entry whose digest
  differs".
- **Outside the static closure.** Per package with manifests, the traced
  `pkg:` entries (of any retained manifest, tried or not) that are not in its
  static closure are listed as `outside_static_closure`, and the histogram
  counts the packages that have any. A `pkg:` entry for a path that is no
  package is listed too: the trace read something the graph does not know.
- **Memory (D11).** The manifest reader takes the set of weak keys the plan
  computed and ignores every other record as it parses it, so the heap is
  bounded by the plan's packages, not the store (`ManifestReport.Ignored`
  counts them). `nil` keeps everything, which is what merge uses. Structure
  and caps are still judged for ignored records. The verified `id` is
  computed once when a record is retained and kept with it, not recomputed
  per comparison.
- **Unusable stores.** A manifest-store error makes every package that got a
  weak key `ineligible:manifest-store-unusable`; a pass-ledger error makes
  those with manifests `ineligible:pass-ledger-unusable`. The plan says which
  in `lookup.errors`. Nothing is a hit.
- **The recorder's identity is in the key.** `kiln:root` is the absolute
  `--root` and `kiln:executor-args` is the repeatable `--executor-arg`
  flags. A lookup that differs from the recorder in either gets
  `no-manifest`, never a hit.
- **Plan size.** Each tried manifest lists at most 256 entries
  (`entries`), with `entry_count` and `entries_truncated`; the strong key and
  `outside_static_closure` always use the full list. `kiln explain` shows only
  what the plan holds and says when it was truncated.
- **Scope and probes.** The plan's `lookup.scope` comes from the stores
  themselves (both must have the same scope; `NewLookupOptions` opens them
  together). `lookup.probes_ran` says whether `--probe` ran.
- **Junk manifests.** A flood of forged or junk manifests can only fill the
  four slots of a weak key and force misses; it cannot produce a hit, because
  a hit needs a pass for the strong key (§8, Poisoning).
- **Before any skip mode exists (M3 requirements from the review).** Skipping
  requires: (a) the policy files (`never.json`, `pinned.json`,
  `toolchains.json`, `env.json`, `host-roots.json`) read from the base
  revision, enforced in code, not by convention; (b) `probes_ran == true`,
  since without probes a `probe-mismatch` gate cannot fire; (c) an explicit
  decision on whether `unpinned:<lang>` gates lookup. **This is open**: 5a
  does not gate on it, and the keys carry pins only.
- **Trust boundary (§3.3).** Lookup reads the policy files (`env.json`,
  `host-roots.json`, `never.json`, `pinned.json`, `toolchains.json`) through
  the existing loaders, from whatever tree it is given. That is acceptable
  only because 5a reports. **Skipping (M3) must run the Kiln binary and read
  the policy built from the base revision**, as §3.3 says; 5a does not
  establish that and must not be wired to a skip.
- **The tree must be vouched for.** `--ledger` is refused (exit 2, nothing
  written) with `--source folder`, and the library refuses any `Source` that
  is not a `source.Cleaner`: a plain folder cannot list untracked files, so a
  build product could enter a digest and the result would describe a tree
  nobody committed. The plain plan only warns, because its keys are not a
  claim about the ledger.

## 10. What stays from KILN03

- The graph export, `D(q)`, the pins, probes and env allowlist, the
  clean-tree rule, the never list and the pinned allowlist.
- The hard preconditions of §6 (`precondition:`).
  `unknown-repo-inputs` is a static-key reason only, because tracing
  observes the reads.
- The static semantics-0 key, now diagnostic: M2 compares its predictions
  with the traced ones.
- The scans, as advisory hints only.

## 11. Encoding details

- **Entry names.** `pkg:` + the package's `rel_path`; `path:` + a
  repository-relative path; `list:` + a repository-relative path, with `.`
  for the root. Paths follow KILN02 §3.5. Manifest entries are checked
  against an allowlist: an entry that does not start with `pkg:`, `path:`
  or `list:` is refused.
- **`consumed` phase.** Every entry's phase is the nominal `build`, as
  under semantics version 0.
- **State digests for `path:`.** SHA-256 over a tag byte and a payload:
  - `00`: absent;
  - `01`: directory;
  - `02 || u8(exec) || SHA-256(content)`: file;
  - `03 || bytes(target)`: link.
- **The missing-package digest** is SHA-256 of `str("kiln-missing-package")`.
- **A listing digest** is tagged, like the state digests, so a `list:`
  entry stands on its own without a paired `path:` entry:
  - for a tracked directory (some tracked file lies under it):
    SHA-256 of `u8(0x10) || list([str(name) || u8(kind)])`, sorted by name
    bytes, where kind `1` is a file, `2` a link and `3` a directory;
  - for a path that is not a directory in the tracked tree (absent, a file
    or a link): SHA-256 of the single byte `11`.
- **`T(q)`** is as defined in §5.2.

Golden vectors are added to `key-vectors-v1.json` (KILN02 §6) for:
- a weak key;
- a strong key;
- each state digest;
- a listing digest;
- `T(q)` with and without nested packages.

### 11.1 Key decisions (stage 1b-ii)

The encoders are in `internal/key` (`traced.go`) and the composition is in
`internal/legacy` (`traced.go`). Where §3, §4.2 and §11 left a choice, this
is what the code does.

1. **Tag bytes are raw.** A state digest hashes its one tag byte, then the
   payload, exactly as listed in §11. The tag is not framed with `str` or
   `bytes`. `02` carries the SHA-256 of the content, not the content, so a
   large file is hashed once and streamed. The `exec` bit is the one the
   `Source` reports, never the disk's.
2. **Encoders refuse, they do not repair.** `ListingDigest` and `T(q)` demand
   strictly ascending input, as KILN02 §4 does for every list, so a duplicate
   or an unsorted name is an error. A listing name is a child name: non-empty,
   valid UTF-8, no `/`. A nested path in `T(q)` is a KILN02 path.
3. **The manifest allowlist is `CheckEntryName`.** The prefix must be `pkg:`,
   `path:` or `list:`; the rest is a KILN02 path, except that `list:.` is the
   root. `path:.` is refused: the root is always a directory, and only a
   `list:` entry can see into it. A `weak:` entry, a bare `weak:` and a
   `tool:` or `policy:` entry are refused. `StrongConsumed` sorts the section
   itself (a manifest keeps its own order), refuses a name given twice, and
   puts `weak:` among the rest, so the sorted order is `list:`, `path:`,
   `pkg:`, `weak:`.
4. **Tracked tree only.** `Traced.CurrentDigests` reads the tree only through
   `Source`. A `path:` entry is a file or link if `Source` lists it, a
   directory if it lists anything beneath it, and absent otherwise. A `list:`
   entry holds the immediate children, a child directory counting when
   anything tracked lies below it. A `list:` digest is tagged (`10` for a
   directory with its children, `11` for anything that is not a tracked
   directory), so a directory that later becomes a tracked file, or vanishes,
   changes its `list:` digest even when the manifest has no `path:` entry for
   it. The directory encoding requires at least one child, and its first byte
   `10` cannot be mistaken for the state tags `00` to `03`. Untracked files
   never appear. Each digest is computed once per `Traced`.
5. **"Directly nested" means the nearest enclosing package.** A package `j`
   is in `T(q)`'s list when `q` is the closest package above it. `D(q)`
   already omits everything inside any nested package.
6. **A package with no traced key is a `Refusal`, not a failure.** The weak
   key is refused with a reason: `host-unknown` (an image ID containing
   `unknown`), `unknown-toolchain:<name>`, `no-build-file:<os>` or
   `symlink:<file>`. A BUILD file named by the graph but missing from the
   tree is an error. The never list and the §6 preconditions are the
   planner's, not the key's, so the key does not look at them.
7. **`policy:git` is an inputs-list digest.** It is `FilesDigest("inputs", ...)`
   over every `.gitattributes` and `.gitmodules` in the tree, so a path, an
   executable bit or a byte change moves it, and so does the first such file
   appearing. `.git/info/attributes` is untracked and out of reach; the
   recorder's clean-checkout checks (§5.2) are what cover it.
8. **The scrubbed environment.** One function, `ScrubbedEnv`, builds it, and
   the weak key's `env` section is that result plus the three `kiln:` names, so
   what the recorder runs with is exactly what is keyed. Later steps replace
   earlier ones of the same name: the allowlist, then `HOME` and `TMPDIR`, then
   `PATH`, then `LANG=C.UTF-8` and `TZ=UTC`. So `env.json` listing `PATH` or
   `LANG` changes nothing. `HOME` and `TMPDIR` are passed when set and left out
   when not; an unset one is not invented. `PATH` is `ConstructPATH` over the
   `tools` of the weak key and `HOME`. A variable whose name contains `:` is
   refused. The `kiln:` entries are keyed but never exported to the process.
   Every origin is `0`.
9. **`kiln:recorder`** is the integer constant `RecorderVersion` (see `internal/legacy/traced.go` for the current value;
   `1` was the recorder before `writes_complete`, stage 4 PR 4) in
   `internal/legacy/traced.go`, to be bumped by hand with any change to §4 to
   §7. **`kiln:executor-args`** is each flag as one `-name=value` token,
   Go-quoted, sorted bytewise and joined by spaces, so the same flags in
   another order key the same; a flag given twice is refused. **`kiln:root`**
   must be an absolute path.
10. **Vector kinds.** (Beyond §11's list, a `weak:` manifest entry and other
    bad entries are pinned as refusals.) The weak key and the strong key are ordinary `valid`
    vectors (semantics 2). Three new arrays pin the rest: `digests` (one per
    state, missing, listing or `T(q)` digest, with its inputs),
    `strong_consumed` (a weak key and entries in, the sorted consumed section
    out) and `refused_entries`. `kiln verify-vectors` checks all of them.
11. **The graph must come from the same tree as `Source`.** `T(q)` takes its
    nested-package names from the graph, so a graph that lags the tree could
    let a new nested package leave `T(q)` unchanged. Stages 4 and 5 will
    enforce this by running the export in the same job and binding it to the
    tree (a digest of the export plus the source revision); this is a tracked
    follow-up. Until then `NewTraced`'s check, that every BUILD file the graph
    names exists in `Source`, is a partial guard. A `BUILD` file elsewhere
    (a test fixture, say) is not checked: the old tool does not treat it as a
    package, so it is an ordinary file of its enclosing package, in `D(q)`
    and not in the nested list.
12. **Returned keys are copies.** `WeakKey` returns a deep copy of the
    memoised key, and `Strong` deep-copies the sections it reuses
    (`key.Action.Clone`), so a caller cannot corrupt a cached key.
13. **Pins, not probes.** The traced keys carry the tool pins, never probe
    results. Whatever uses a key to skip a test (stage 5 lookup, M3) must
    first apply `probe-mismatch`, the never list and the §6 preconditions;
    none of them is in the key (also listed in the §13 M3 gate).
14. **Windows.** On Windows `os/exec` adds `SYSTEMROOT` to a child's
    environment. That is moot while tracing is Linux-only (§4.1), but it must
    be keyed if Windows is ever traced.
15. **`kiln:root` is not cleaned.** It is keyed as given, so two spellings of
    one checkout path are two keys. That can only cause a miss, never a hit.

## 12. Tests that must exist

- **Absence.** An action probes a missing file and is recorded. Adding
  that file inside a package, outside every package, or as a **new nested
  package** makes lookup miss.
- **Listing.** Adding or removing a sibling in a listed directory makes
  lookup miss. Changing a sibling's content does not, unless it was also
  read.
- **Written, then read.** An action that writes and then reads its own
  file records nothing for it. An action that opens a pre-existing file
  without `O_TRUNC` gets `reads-product`. Linking or renaming another
  package's file into place and reading it adds that package.
- **Products.** Action B writes `x` and action A reads `x`, in either
  order and in **different shards**: A gets `reads-product`. Each of these
  emits nothing:
  - a missing shard;
  - a shard that claims an action outside its partition;
  - a replayed shard record from another run.

  B writes a tracked file and restores it, while A reads it in between:
  A gets `reads-product`.
  A rewrites B's BUILD, with or without restoring it afterwards: B is not
  recorded, through either the recipe mismatch or the tainted set.
- **ABI.** Any 32-bit or x32 syscall, including a 32-bit
  `clone(CLONE_UNTRACED)`, kills the process.
- **Unstable links.** A resolves through a link `L` that B, in any shard,
  replaces by unlinking and re-creating it: `unstable-path`.
- **Hostile tree.** A tracked path replaced by a FIFO, a device node or a
  symlink loop gives `non-regular` or `unstable-path`, never a hang.
- **Working tree.** A `.gitattributes` change that alters checked-out
  bytes changes the weak key. A working tree that differs from `Source`
  blocks recording and lookup.
- **Parser.** Fuzz testing with filenames containing quotes, `<`, `>`,
  `)`, ` = `, newlines and `+++` either round-trips or gives
  `ambiguous-path`.
- **Failing closed.** Each of these gives `trace-incomplete`:
  - each refused call;
  - an unknown system-call name;
  - a personality change;
  - a truncated path;
  - an unmatched `resumed`;
  - a missing exit line;
  - a `/proc/self/root` path or a numeric `/proc/<pid>/` path;
  - an unstable path.
- **Host.**
  - A read under an `image` root is allowed.
  - A read of `$HOME/.config/x` gives `host-read`.
  - An `image` file with a ctime after job start gives
    `host-read:setup-modified`.
  - A preloaded `state` root gives `unscrubbed:state-preloaded`.
  - A read under the root of a tool not in the weak key's `tools` gives
    `host-read:tool-not-keyed`.
- **Network.**
  - A non-blocking `connect` returning `EINPROGRESS` to a public address
    gives `network`.
  - A loopback server the action started is allowed.
  - A connection to an existing Unix socket gives `daemon`.
  - `ECONNREFUSED` on loopback is allowed.
- **Sandbox.**
  - Setup calls before the helper's `execve` are discarded.
  - From inside the sandbox, the action **cannot**:
    - `umount` anything;
    - write to `$GITHUB_ENV`, the Kiln binary, the results or raw-records
      directories;
    - open `.git/config`;
    - see the tracer's PID;
    - create a `CLONE_UNTRACED` child.
  - A survivor killed when PID 1 exits gives `daemon-left`.
  - The action cannot open the trace file (end to end).
- **Exec.** A `#!` interpreter inside another package adds that package.
- **Keys.**
  - Weak and strong keys never collide.
  - The strong key changes when any entry's digest changes, and not
    otherwise.
  - Lookup tries up to K distinct manifests, newest first.
- **Store.**
  - Over-cap records and stores are refused.
  - Retention keeps K distinct ids per weak key.
  - Unknown fields, non-canonical lines and unsorted entries are refused.
- **Determinism.** The same trace gives a byte-identical manifest.
- **End to end** (CI, Linux). A fixture repository with a handful of shell
  BUILDs is recorded under real `strace` through `sandbox-exec`, then
  looked up after a mutation of each kind. Each strace behaviour this spec
  relies on is pinned:
  - exit lines under `--quiet=attach`;
  - `-y` annotations on returned descriptors;
  - `unfinished`/`resumed` pairing;
  - same-path annotations inside the namespace.

## 13. Rollout and gates

**M2: traced shadow (backlog item 8).** The scheduled Linux full build
also runs `kiln-record` and `kiln-record-merge` under the tracer, with the
scrubbed environment. They write manifests and passes to a **shadow
scope**. Until stage 5, all network use gives `network`. The run report
gives:

- overhead: wall time traced versus untraced;
- trace and manifest sizes;
- the reason histogram (`reads-product`, `host-read`, `network`,
  `trace-incomplete`, …) and the recordable share per language;
- the packages whose result changed under the scrubbed environment. Each
  is a missing `env.json` entry or a real dependence to fix.

**M2: comparison (backlog item 9).**

- Before each forced full build, snapshot every predicted hit. Afterwards,
  compare them with the actual results. A predicted hit that fails, and
  fails again when re-run at the same commit, is a soundness bug.
- Report traced entries outside the static closure (§9). These are bugs
  in the old graph.
- Mutation tests:
  - touching a traced file must miss;
  - touching an untraced file inside the static closure must still hit,
    and the BUILD must still pass when re-run.

**Kill criterion.** After the traced shadow, a language with under about
30% of its packages recordable (mostly `reads-product`) does not get the
bridge. It goes straight to M4 isolation. The 30% figure is a starting
point, not a measurement.

**M3 gate.** Reuse turns on per language only when all of these hold:

1. N forced full builds (KILN00 §18 Q6) show zero unexplained
   disagreements on traced predictions;
2. the scrubbed-environment shadow shows no unexplained result changes;
3. `probe-mismatch` is fatal (KILN03 §4);
4. recording runs inside the §4.3 sandbox;
5. lookup applies `probe-mismatch`, the never list and the §6
   preconditions before it can skip, because the traced keys carry pins
   only and none of these (§11.1). The recorder applies the same gate: a
   gated package is recorded with its `gate` (P7) and is never emitted, and its
   toolchain probes run in the sandbox, never on the host.
6. skipping reads policy from the base revision, enforced in code before any
   skip mode exists; it requires `probes_ran == true`; and the open question
   of whether `unpinned:<lang>` gates is decided (§9.1).

**Skipped products.** A skipped action creates no products, so another
action that relied on them might now fail. The M3 spec must settle this
before any skipping. The likely rule: do not skip an action whose write
roots a running action read in its latest trace.

**Before recording.** The `kiln-record` probe (D15) must also run that host
scan for sockets and FIFOs over the real configuration's `ro_roots`, because the
in-sandbox walk does not look inside them. (Done by `prepare`: the host scan, P8,
and the in-sandbox structure probe, P14, which is the trivial step through
`kiln sandbox-exec` the next sentences ask for, and more.) The sandbox tests skip where the kernel refuses
unprivileged namespaces, and a runner image, kernel or `strace` can change
under a green CI. The `kiln-record` job (stage 4) must therefore run the same
probe before it records anything: a trivial step through `kiln sandbox-exec`
with the real configuration, which fails the job (records nothing) when the
sandbox cannot be built. The required `kiln-sandbox` CI job also runs on every
scheduled and dispatched full build.

## 14. Residual risks

These are known and named. They are measured rather than closed.

- **A dying call that reads is refused, not dropped (D35).** A thread killed
  inside `openat`, `newfstatat` or a listing when its group exits leaves an
  `unresumed-call` refusal, so some threaded actions (Go's worker threads in a
  file-system loop) stay unrecorded. Dropping the call would remove an input from
  the read set: reads are inferred from the open and stat records alone, and a
  live sibling thread, a sibling's `fork` in the window, a sibling's `exec` (the
  descriptor survives without `CLOEXEC`) or a stat buffer in shared memory can
  still use what the call returned. A dying `clone3` at entry stays
  `unresumed-call` too. Recording such a call as an observed read with an unknown
  result is the follow-up.

- **The absent-ok child rule looks at the entry at classification time.** It
  checks the entry with `lstat` when the action ends, so an entry that existed
  during the action but was removed by something outside the action before
  classification would let an `ENOENT` child probe pass. That is not reachable
  from inside the read-only sandbox: the action cannot remove `/sys/fs/selinux`
  or `/selinux`, anything it creates is handled by `isCreated`, and libselinux
  probes the entry itself first, which is judged by the traced result.

- **`/proc/filesystems` varies with the kernel modules loaded during the job.**
  It is `ephemeral` (D27), so a read of it is not part of any key. A module
  loaded mid-job (say a filesystem driver) can change what a tool sees. This is
  accepted, like `/proc/self/mountinfo` and `/proc/cpuinfo`: the image id pins
  the kernel build, not the modules a job loads.

- **Recorded binary digests are audit-only.** The binding carries the SHA-256 of
  the Kiln, build-tool, `strace` and `git` binaries and of the sandbox
  configuration, but `recmerge` compares none of them (B3). Backlog: pin the
  expected digests (built in the merge job from the same commit) and compare.

- **Content shadowed beneath a host mount point.** The host scan skips a
  directory only when mountinfo lists it as a mount point, and the sandbox binds
  an `ro_roots` entry non-recursively, so what the mount hides on the host is
  what the step sees. That is not scanned on the host; the `shadow` run of the
  in-sandbox structure probe (decision P14, stage 4 PR 2b-2c-2: a `find -xdev`
  under every such mount point, in the sandbox's own view) covers it. The kernel's locked-mount rule makes such a mount
  fail every step, which is fail-closed. **Observed in CI:** the kernel refused
  the bind (`ro-root ...: invalid argument`, exit 125) and `prepare` failed at
  `probe pipes`, so the locked-child-mount rule closes this risk; the probe is
  defence in depth (P14).

- **The strace pin and the seccomp threshold move together.** The sandbox
  filter answers `ENOSYS` for every system call number above
  `MaxKnownSyscall` (§4.7), the highest the pinned strace's table knows, because
  a call newer than the pin can be neither traced nor refused. Bumping the pinned
  strace therefore means **raising `MaxKnownSyscall`** (after adding the calls
  the new strace knows to the parser's table, §4.4) and bumping `kiln:recorder`.
  Forgetting leaves the newer calls blocked, which fails closed, not open.

- **Serial recording is slower.** P13 runs the traced build with `-jobs 1`, so a
  shard's wall time is the sum of its packages', not the longest chain's. The
  mitigation is a higher shard count (the plan already partitions by measured
  duration); measure the slowest shard before raising it. Strace itself slows
  each step, so the effect compounds.

- **Nondeterminism.** Time, randomness, thread scheduling, CPU count and
  model, hostname, uid, and the mtimes and inode numbers that `stat`
  reveals are not keyed. A flaky test can be recorded as a pass. KILN00
  §10.5 is the answer, as for every Kiln key: a daily uncached run, and
  tombstones.
- **A new writer.** A path one action found absent may, in another job, be
  created first by a changed BUILD writing there for the first time.
  Full-build recording (§7) catches every writer that exists at recording
  time; the next item covers the rest until M4 isolation removes this
  class.
- **Out-of-repository symlinks** are resolved after the run, not at the
  moment of the call. A `-y` annotation can also race when threads reuse
  the same descriptor number.
- **Failed loopback or Unix connections** observe host state that the
  image is assumed to fix.
- **Tracer visibility.** Besides `TracerPid`, a traced program can notice
  tracing through timing. A hostile program could behave differently when
  traced. The detectors are the §13 mutation tests and the comparison
  harness, which also compares exit status between a traced and an
  untraced run of each package.
- **A first-ever write into another package.** A PR can make A write, for
  the first time, into B's directory or into a shared write root where B
  found nothing. B is then a false hit until the next forced full build.
  The M3 spec must close this before skipping. The likely rule: when an
  action misses and its latest trace, or the write-root diff in
  `legacy-plan`, touches a path another package's manifest saw as absent,
  that package runs too.
- **Bound ports are never forgotten.** The trace does not follow `close`, so a
  loopback port the action bound counts as served for the rest of the action,
  even after the server's socket is closed and a daemon owns the port. A
  connect to a port the action bound and released is then allowed. A bind to
  port 0 matches nothing. The merge job's shared-port rule (§7) is the
  partial defence.
- **Second-order order dependence.** An action that depended on another's
  output has a recorded write set that is only one of several; the other
  actions' checks quantify over write sets. The strict merge closes this for
  emission by voiding the run (§7.1, M-3), which probably zeroes the yield of
  the first runs. The relaxed verdict reports what is lost. The M3 spec must
  add a lookup-time rule (an action that would be order-dependent in the
  lookup job runs) before any relaxation.
- **A compromised shard** can under-report its own copies' `Writes` and
  choose its own host facts for its own actions. The merge cannot see either,
  because it has only the shard's records and the facts the weak key needs.
  This is inherent, and is mitigated by trusted runs only (the job `if:`,
  `Trusted()` and the cache scope). `kiln_sha256`, `build_tool_sha256` and
  `sandbox_config_sha256` in the binding are self-attested and
  report-only: they are not compared across shards, because a compromised
  shard would simply write the same value. The guards are
  `recorder == RecorderVersion` and B5/B6.
- **Tool build caches** declared as `state` are trusted the way KILN00
  §9.5 trusts tool state.
- **No network inside the sandbox.** Every traced step runs in an empty
  network namespace with only a loopback (§4.7), so a BUILD that fetches
  fails on the first connect and is recorded as such: nothing is recorded for
  it (§5.4). Stage 5 must give fetchers a way out that the sandbox can still
  account for, probably a proxy the helper reaches through a Unix socket it
  owns, and revisit this isolation then.
- **Allowed roots are trusted.** The sandbox root holds only `ro_roots`, the
  checkout and the state roots, but a socket or FIFO inside one of those is
  reachable, and a runner-image directory could hold one. The recorder must
  list roots from `host-roots.json` that hold none (stage 4 should assert it
  with the same walk as `TestSandboxNoReachableSocketsOrFifos`), and a root such
  as `/var` that might bring back a host service path is caught only by the
  post-pivot check of §4.3.
- **Shared inodes between sandboxes.** Concurrent sandboxes see the same inodes
  under `ro_roots`, so advisory locks (`flock`, `fcntl`) and timing form a
  low-bandwidth channel between cooperating actions that no trace records.
- **Stage 4 runs from outside the checkout.** Parsing and merging must run from
  binaries and scripts that live outside the checkout, never from the working
  tree after BUILDs have run: a BUILD can rewrite anything in it.
- **Linux only.** macOS and Windows get no hits from this bridge.

## 15. Open questions

1. **macOS and Windows tracing.** Each option needs special access:
   `dtruss` needs SIP off, the Endpoint Security framework needs an
   entitlement, and ETW needs administrator rights, which hosted Windows
   runners have. Is any of them worth it before M4, given those hosts are
   untrusted under KILN00 §9.6 anyway?
2. **`strace` versus an own tracer.** Is `strace` enough, and is
   `--seccomp-bpf` (much faster on stat-heavy Node and Python) safe when a
   tracee installs its own seccomp filter?
3. **Product provenance (v2).** Instead of refusing `reads-product`, union
   the writer's manifest into the reader's. Rust needs this for any hits
   under its shared `target/`.
4. **Trace volume.** A full Rust or Node build may emit gigabytes of
   trace. Measure it in M2 and set the per-step cap from the measurement.

## 16. Implementation stages

Each stage is one PR.

1. **Trace parser and manifest builder.** Pure Go, tested on canned trace
   fixtures:
   - the three-class table;
   - working-directory tracking;
   - classification;
   - `T(q)`, state and listing digests;
   - weak and strong keys;
   - golden vectors.
2. **Manifest store.** Segments, strict decoding, caps, `id`
   deduplication, retention and merge. Includes a test that the pass
   ledger ignores the `manifests/` subdirectory.
3. **Sandbox and wiring.**
   - `kiln sandbox-exec`;
   - build-tool `-exec-wrapper` and `-emit-results`;
   - the real-`strace` end-to-end fixture, on Linux CI.
4. **Recording jobs.** `kiln-record` and `kiln-record-merge`: a shadow
   scope, cross-shard checks, and the reason and overhead report, with all
   network refused.
5. **Lookup and comparison.**
   - lookup in `legacy-plan`;
   - the comparison harness;
   - fetcher and state allowances;
   - the static-closure diff report.

KILN03 item 7c-3 (pins, probes, the env allowlist, and the new
`host-roots.json`) provides the weak key. It lands before stage 3.
