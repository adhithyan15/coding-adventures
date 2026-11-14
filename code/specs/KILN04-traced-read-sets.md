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
       -e signal=none \
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
   strace").

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
  it exits the kernel kills every survivor. Any process whose exit line is
  `killed by SIGKILL` after the **init child's** exit line gives
  `daemon-left`. No timer is needed.

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
sandbox's filter already blocks these (§4.3); seeing one in the trace
means the filter failed (`trace-incomplete:clone-flags`).

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
- A refused name is `refused:<name>`, an unknown one
  `unknown-syscall:<name>`.
- `clone` with `CLONE_UNTRACED`, any `CLONE_NEW*`, a missing `flags=` or
  a flag token it cannot name is `clone-flags`. `clone3` is
  `refused:clone3`.
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
`unresumed-call`. A handled call whose result is `?` (including
`ERESTARTSYS`) is `result`: restarted calls are not recognised in v1. A
final line without its newline is `unparsable-line`, since strace may have
been cut off mid-line.

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
- A successful `chdir` or `fchdir` on a cwd shared by more than one live
  process is `cwd-race`: relative paths of the sharer would change
  unobserved.
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
    `unshare(CLONE_FS)`, which the parser does not model, so avoid
    `unshare(CLONE_FS)`: threads that share a cwd make a `chdir` a
    `cwd-race`;
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
  (`Trace.StepExit`), and `Trace.InitPid` names it for the later
  `daemon-left` rule, which the parser does not implement.

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
is `size` or `arity`. A caller with a larger measured budget raises them.

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
in-sandbox socket probe and the `daemon-left` fixture. Each item of the two
"Pending for PR 2b" lists below is marked **2b-1** (decided and implemented by
`prepare`), **2b-2a** (decided and implemented by `step`), **2b-2b-i**
(decided by P13, implemented by `step` and `prepare`), **2b-2b-ii** (still
pending, owned by `collect`) or **2b-2c** (still pending, owned by the gates
and probes). A bare **2b-2b** in older text means 2b-2b-i and 2b-2b-ii.

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

- **2b-2b-ii.** `collect` treats any plain git error from `VerifyTree` (not only a
  `*TreeError`) as a void of the shard, like `tracked-changed`, and passes the
  commit id `prepare` recorded.
- **2b-2b-ii.** An `Inventory` error after the build voids the shard; it must not crash
  `collect` before the records are written.
- **2b-1.** `prepare` fails if the pre-build inventory contains any path whose
  base name starts with `BUILD`: an untracked BUILD variant would change the
  lines a package runs. `prepare` also walks the whole disk for a `BUILD*`
  file the tree does not track, because the inventory collapses an untracked
  directory to one entry and a new package's BUILD hides inside it
  (decision P5).
- **2b-1.** Global Git configuration (`$HOME/.gitconfig`, `$XDG_CONFIG_HOME`) is still
  read by Kiln's Git calls, so the recorder's `HOME`, `XDG_CONFIG_HOME` and
  `PATH` must not be writable from the sandbox.
- **2b-1** (the mask; the `git fsck` is **2b-2b-ii**). The commit pin protects object ids, not the integrity of the object store. A
  BUILD that could write `.git` could overwrite loose objects or
  `.git/config`, so `.git` must be unwritable from the sandbox (the
  `R/.git` mask is a requirement, not a convenience). Consider a post-build
  `git fsck --connectivity-only` as a second line.
- **2b-1** (kept) and **2b-2b-ii** (used by `collect`). The caller keeps `Result.Commit` from the
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
- **2b-2b-ii.** The shard writer derives `writes_complete` with `recfmt.CompleteOf` from the
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
  can write the tree with no trace at all. `prepare` runs no probe and no gate
  (gates are **2b-2c**, decision P7); the only programs it starts are Git, the
  build tool and `strace --version`, all resolved outside the checkout
  (decision P3).
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
  assume it.

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
until 2b-2b-ii only as a stub that prints "not implemented (KILN04 stage 4 PR
2b-2b-ii)" and exits 1, so the CLI surface is fixed and a workflow written
against it fails loudly, never silently skips tracing. `prepare` writes the build tool's `-exec-wrapper`
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
pre/tools.json
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
`step` and `collect` re-verifying `pre/` (2b-2a and 2b-2b-ii, list below).
`traces/` is created by `prepare` (0700, outside the sealed `pre/`), its
per-action directories by `step` (2b-2a, decision P12), and `probe/` by 2b-2c.

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
those facts. `pre/actions/<sha256(action)>.json` (`kiln-record-action/1`)
holds, per action: `action`, `language`, `weak` (hex) or `refusal` (the
`Refusal` reason), and `steps`, the lines `kiln-legacy-steps/1` gives for it
(`[]` when it has no BUILD for Linux). A package whose key is refused still
gets an environment (`Traced.RecordEnv`), because it still runs. **Gates**
(`gate`, `precondition:<gate>`) and the toolchain probes are **2b-2c**: they
need `legacy.Gate` extracted from the planner, and `prepare` runs no probe.

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
scan, as does any other error. The scan of the sandbox's own view (the in-sandbox
probe, design step 8) needs the supervisor and `strace`, so it is **2b-2c**; it is
also what covers content a host mount point shadows beneath an `ro_roots` entry
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
`kiln-record-binding/1`: `repository`, `run`, `attempt` and `revision` (the
commit `VerifyTree` returned, which `GITHUB_SHA` equalled), `shard_index`,
`shard_count`, the digests of the graph, steps, partition, host facts,
inventory and sandbox configuration, `kiln_sha256`, `build_tool_sha256`,
`strace_sha256`, `git_sha256`,
`recorder` (`legacy.RecorderVersion`), `strace_version`, `kernel`
(`/proc/sys/kernel/osrelease`), `job_start` and `prepared_at`. Every
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
- **2b-2a** (`step`) and **2b-2b-ii** (`collect`). `kiln record step` and `collect` use only the recorded git path and
  the recorded `PATH`, and never look a program up on `PATH`.
- **2b-2a.** The workflow launches the build tool with **exactly** the recorded
  `build_tool_env` (nothing inherited). The build tool hands its own environment
  to the wrapper plus `KILN_ACTION`, `KILN_STEP` and `KILN_STEP_COUNT`, so
  `record step` refuses to run unless its inherited environment, apart from those
  three, equals `build_tool_env` exactly (in particular every `GIT_*`, `LD_*` and
  `PATH` entry). The comparison is of multisets of `NAME=value` entries after
  removing `KILN_ACTION`, `KILN_STEP` and `KILN_STEP_COUNT`, and a repeated name
  (in either) is a refusal.
- **2b-2a** (`step`) and **2b-2b-ii** (`collect`). `step` and `collect` re-verify `pre/` against `SEALED` and the binding
  id before using any file in it. The `0444`/`0555` modes are cosmetic; integrity
  comes only from the files being unreachable from the sandbox.

**P12. The step supervisor `kiln record step` (stage 4 PR 2b-2a).** The build
tool runs it once per BUILD step through `-exec-wrapper` (P1). It runs
outside the sandbox and is not traced itself; it traces the step and keeps
the facts about the tracer that the trace cannot contain. It classifies the action after its final executed step (P13).

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
   125 without running; `collect` voids the shard on any `.rerun`.
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
status 125 is recorded (`exit_125`) and never retried (§4.7). strace's own
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
it. The sidecar `traces/<sha256(action)>/<step>.json` (`kiln-record-step/1`,
compact, no trailing newline, `O_EXCL`) holds: `action`, `step`,
`step_count`, `binding_id`, `line_sha256`, `line_in_plan`, `started`,
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
`collect`'s. `step_netns` is true only when strace was started with the
`kiln sandbox-exec` command above, the only way a step runs, because that
helper unshares the network namespace; it is false when the process could not
be started. `collect` copies it into `traced.Input.StepNetNS` only when the
step's own `execve` is in the trace (a refused sandbox has none).

*Obligations of `collect` (**2b-2b-ii**, except the parse half of the third, which `step` applies, P13), from the review of P12.*

- **2b-2b-ii.** `collect` takes an **exclusive**, non-blocking `flock` on the
  record directory (`record.LockDir(dir, true)`) and refuses if it cannot: a
  step, or an orphaned supervisor, is still running.
- **2b-2b-ii.** `collect` requires every trace to end in a newline, re-hashes it
  and requires the digest and size to equal the sidecar's `trace_sha256` and
  `trace_bytes`; a trace that does not is `unknown_writes`.
- **2b-2b-ii.** `collect` compares the `--sha256` argument of the init's traced
  `execve` (the digest the root read the configuration with) and the
  `--config-sha256` of the root's with the binding's `sandbox_config_sha256`,
  so the configuration a step ran under is the sealed one even where the
  recorder verified the file by content but the helper opened it by path.
- **2b-2b-ii.** `collect` treats any `.rerun` marker, any sidecar without a trace
  (or the reverse), any `trace-write-failed` and any `strace-pipes-held-open`
  as an unknown write set.

**P13. Records are classified at action end, and recording is serial
(2b-2b-i).** `traced.Classify` resolves paths against the disk, so a record
classified after a later action changed a link, directory or file it resolved
through places its writes and observations wrongly. M8 and M10 catch this in
the strict verdict only through M-3, not at all in the relaxed one, and not for
host state roots. So:

1. *Serial.* The traced build runs with `-jobs 1`. `step` holds an exclusive,
   non-blocking `flock` on `traces/.serial` from before its checks until it
   exits; a step that cannot take it leaves `<step>.overlap`, runs nothing and
   exits 125. Under that lock it assigns the sidecar a `seq` from
   `traces/.seq`. `collect` requires each action's `seq` values to be
   contiguous; any `.overlap` or gap makes every record of the shard
   `unknown_writes` (`serial-violated`, M1).
2. *Sealed inputs.* `prepare` also writes `pre/tree` (`kiln-record-tree/1`:
   every tracked path of the verified commit with mode, blob id and link text),
   `pre/classify.json` (the host roots and pinned ecosystems) and each action's
   `tools` (action files become `kiln-record-action/2`). Neither `step` nor
   `collect` builds a `legacy.Traced` or runs Git to classify.
3. *Classification.* After the sidecar of the action's last step, or of a step
   that did not start or exited non-zero, and before releasing the lock, `step`
   parses the action's traces, applies the parse half of G7, and runs
   `traced.Classify` with `Tree` from `pre/tree` and `FS` a live reader that
   follows no symlink (`lstat`, `readlink`; a head is read only from a regular
   file opened `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY` whose `/proc/self/fd` path
   equals the path asked). `Tree.Head` returns disk bytes only when they hash
   to the sealed blob id. The result is `traces/<sha256(action)>/record.json`
   (`O_EXCL`). A panic, a classification over 10 minutes, or traces over 2 GiB
   leaves `record.failed` instead. The step's exit status is never changed.
   The parse half of G7 is: every trace parses (with the sandbox helper as
   root); the root's exit line equals the sidecar's `root_status`; the traced
   `execve` of the init's `--sha256` and the root's `--config-sha256` equal
   `binding.sandbox_config_sha256`; and a status of 125 is a sandbox refusal
   exactly when the step's own `execve` is absent from the trace.
4. *`collect`* classifies nothing. A missing or failed `record.json` for an
   action that ran a step is `unknown_writes`. "Post-run disk" in §5.1 now reads
   "the disk at the end of the action".

*As implemented.* `pre/tree` is line-oriented, sorted by path:
`kiln-record-tree/1\n` then `<mode> <blob id> <path>\n`, with `<mode> <blob id>
<path> <link text>` for mode `120000`; paths and link text are percent-escaped
only for bytes the format must (`%`, space, control, non-ASCII is left raw).
The sidecar is `kiln-record-step/2` (adds `seq`). `record.json` is
`kiln-record-classified/1`: the raw record, `writes_complete`, the sidecar-derived
`steps_run` and `step_netns`, and the G7 parse verdict (`parse_ok`,
`parse_reasons`), so that `collect` can build a `recfmt.ShardRecord` without
reparsing.

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
  in a trace of the step means the filter failed.
- **No late runtime threads.** Both helpers call `debug.SetGCPercent(-1)`, so
  no garbage-collector worker thread starts after the spawn.
- **CI.** `ci.yml` has a required `kiln-sandbox` job (linux/amd64,
  `ubuntu-24.04`, in `ci-gate`) that lifts the AppArmor restriction on
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
`/proc/{meminfo,stat,cpuinfo}`, `/sys/devices/system/cpu/**`,
`/sys/fs/cgroup/**` and `/dev/fd/N`.

`/proc/self/status` and `/proc/self/stat` reveal `TracerPid`, so a
program could behave differently when traced. A read of either gives
`trace-visible:<path>` and no record. Every other `/proc` path makes the
trace incomplete. That includes any numeric `/proc/<pid>/` path: the PIDs
in the trace are the tracer's, not the action's. `/proc/self/root/…` and
`/proc/<pid>/fd/…` could reach any file by another name.

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
                "/proc/self/exe", "/proc/self/maps", "/proc/self/cgroup",
                "/proc/self/mountinfo", "/proc/meminfo", "/proc/stat",
                "/proc/cpuinfo", "/sys/devices/system/cpu/**", "/sys/fs/cgroup/**"],
  "absent-ok": ["$HOME/.config", "$HOME/.npmrc"]
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
  dotfiles need explicit entries.
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
a path and what is under it, for absent probes only. An `image` file with a
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
exit with 0 gives `failed`. After the init child's exit line, a process
killed by `SIGKILL` gives `daemon-left`. `CheckRecipe` accepts an argv of
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
  checkout's other ancestors and everything under `$HOME` stay `host-read`.

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
`internal/recmerge` (`Merge`, pure: no I/O, no clock). The recorder and the
`kiln record-merge` command are later PRs. The design review found these
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
`RecorderVersion` is now `2`: every weak key changes and manifests recorded by
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
| M3 | run | `post_inventory_unexplained > 0`, or a header `void` of `survivor`, `records-too-large` or an unknown token. |
| M4 | shard | A header `void` of `tracked-changed` or `state-preloaded`. |
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
9. **`kiln:recorder`** is the integer constant `RecorderVersion` (now `2`;
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
   only and none of these (§11.1).
6. skipping reads policy from the base revision, enforced in code before any
   skip mode exists; it requires `probes_ran == true`; and the open question
   of whether `unpinned:<lang>` gates is decided (§9.1).

**Skipped products.** A skipped action creates no products, so another
action that relied on them might now fail. The M3 spec must settle this
before any skipping. The likely rule: do not skip an action whose write
roots a running action read in its latest trace.

**Before recording.** The `kiln-record` probe (D15) must also run that host
scan for sockets and FIFOs over the real configuration's `ro_roots`, because the
in-sandbox walk does not look inside them. The sandbox tests skip where the kernel refuses
unprivileged namespaces, and a runner image, kernel or `strace` can change
under a green CI. The `kiln-record` job (stage 4) must therefore run the same
probe before it records anything: a trivial step through `kiln sandbox-exec`
with the real configuration, which fails the job (records nothing) when the
sandbox cannot be built. The required `kiln-sandbox` CI job also runs on every
scheduled and dispatched full build.

## 14. Residual risks

These are known and named. They are measured rather than closed.

- **Recorded binary digests are audit-only.** The binding carries the SHA-256 of
  the Kiln, build-tool, `strace` and `git` binaries and of the sandbox
  configuration, but `recmerge` compares none of them (B3). Backlog: pin the
  expected digests (built in the merge job from the same commit) and compare.

- **Content shadowed beneath a host mount point.** The host scan skips a
  directory only when mountinfo lists it as a mount point, and the sandbox binds
  an `ro_roots` entry non-recursively, so what the mount hides on the host is
  what the step sees. That is not scanned on the host; the in-sandbox probe of
  stage 4 PR 2b-2c (a walk of the sandbox's own view) covers it.

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
