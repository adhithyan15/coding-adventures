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
strace -f --quiet=attach -y -s 4096 -e signal=none \
       -e trace=%file,%process,%network,getdents,getdents64,fchdir,\
io_uring_setup,open_by_handle_at,bpf,setns,unshare,fsopen,fsmount,\
fsconfig,fspick,open_tree,move_mount,mount_setattr,ptrace,\
process_vm_readv,pidfd_getfd,inotify_add_watch,fanotify_mark,\
fchmod,fchown,fsetxattr,fremovexattr \
       --kill-on-exit --pidns-translation \
       -o <trace-file> -- kiln sandbox-exec -- sh -c <line>
```

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
- **Disk.** strace runs with `RLIMIT_FSIZE` set, and writes to a
  size-limited tmpfs. The interim cap is 2 GiB per step; reaching it gives
  `trace-incomplete:size`. The cap is revised from the M2 measurements.
- **Completeness of the run.** An action passed only if:
  - strace exited 0 with empty standard error, and was not killed by a
    signal;
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
   libc falls back to `clone`.

   The filter's first instructions check two things: that
   `seccomp_data.arch` is `AUDIT_ARCH_X86_64`, and that the syscall number
   lacks `__X32_SYSCALL_BIT`. Any other architecture or ABI kills the
   process (`SECCOMP_RET_KILL_PROCESS`). Without this check, a 32-bit or
   x32 `clone` would match none of the x86-64 rules. A non-native ABI is
   therefore impossible, not merely detected. A change to the filter bumps
   `kiln:recorder`. Enforcement must not rest on refusing a
   trace after the fact: a `CLONE_UNTRACED` child is never attached by
   `strace -f`, so it would leave no lines at all.
6. **No inherited descriptors.** It closes every file descriptor ≥ 3.

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

### 4.7 Sandbox decisions (stage 3b)

Stage 3b is `kiln sandbox-exec` (`internal/sandbox`). The code is
authoritative: `plan.go` for the mount order, `bpf.go` for the filter,
`privs.go` for the privilege drop. These are the decisions that §4.3 and §4.5
left open, and the places where Go's `os/exec` and the trace parser's helper
model (§4.5) forced a different shape.

**Interface.** `kiln sandbox-exec --config <abs path> -- <argv...>`. The build
tool's `-exec-wrapper` is `["/abs/kiln","sandbox-exec","--config","/abs/cfg.json","--"]`
and sets `KILN_ACTION` (the package `rel_path`), `KILN_STEP` and
`KILN_STEP_COUNT`. The helper reads only `KILN_ACTION`; the other two are for
the recorder. The step never sees any of the three: its environment is the
list the configuration holds for `KILN_ACTION`, exactly, and an action the
configuration does not name is an error.

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
  contains or lies inside) the checkout, `home`, `tmpdir` or a state root, or
  lies inside **or contains** `/dev`, `/run`, `/var/run`, `/proc`, `/tmp` or
  `/var/tmp`, which the sandbox builds itself. So `/var` is refused (it holds a
  real `/var/run` and `/var/tmp`), and so is `/var/lib` (service sockets): list
  specific subdirectories instead. No per-step walk for sockets is done; it
  would be too slow, and the CI test and the `kiln-record` probe cover it;
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
symlink; that every mask target exists; that the configuration file is not inside the checkout,
`home`, `tmpdir` or a state root; and that the working directory is exactly
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
5. Fresh tmpfs, each preceded by creating the mount point (and parents), the
   shallower first and no path twice: `home` (`0755`), `tmpdir` (`1777`),
   `/tmp` (`1777`), `/var/tmp` (`1777`) and `/run` (`0755`); all
   `MS_NOSUID|MS_NODEV`, owned by uid 1000.
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
| 4.. | `jeq <nr>` for each refused call, in the order of `RefusedSyscalls` | then `RET ERRNO\|EPERM` |
| | `jeq clone` (56) | then the clone check |
| | `jeq clone3` (435) | then `RET ERRNO\|ENOSYS` |
| | `RET ALLOW` | |
| | clone check: `ld args[0]` (low 32 bits); `jset` any of `CLONE_UNTRACED`, `CLONE_NEWNS`, `CLONE_NEWCGROUP`, `CLONE_NEWUTS`, `CLONE_NEWIPC`, `CLONE_NEWUSER`, `CLONE_NEWPID`, `CLONE_NEWNET` | then `RET ERRNO\|EPERM`; else `RET ALLOW` |
| | `RET ERRNO\|EPERM`, `RET ERRNO\|ENOSYS`, `RET KILL_PROCESS` | the targets above |

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
  non-zero status voids the whole shard** (§7).
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
  sandbox, finds no socket or FIFO, and checks that ones the test makes in the
  host's `/tmp` and `/var/tmp` are not visible.
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
  around `kiln sandbox-exec` and parses the trace in helper mode, which pins
  the single-spawn root, the init's tolerated setup and strace's `-y`
  escaping.

**Exit status.** The root exits with the init's status (128 plus the signal if
it was killed). A failure before the step runs exits 125, and a failed exec 127.
Such a trace has no step exec, which the parser refuses.

**Platforms.** linux/amd64 only (`sandbox_linux_amd64.go`). Every other
platform gets a stub that prints why and exits 125. The integration tests need
unprivileged user namespaces and skip where the kernel refuses them.

## 5. From trace to manifest

### 5.1 Resolving each observation

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
   the base revision. It accepts exactly one raw record per planned
   action, from the shard the partition assigns it, each record carrying
   `{run, revision, plan digest, shard i/n}`. Any mismatch, duplicate,
   missing record or extra record means nothing is emitted. The merge job
   computes every digest and strong key itself, from `Source`, and never
   trusts a digest from a shard.

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
   - **Daemons.** An action with a `daemon:<addr>` reason voids every action
     whose `Binds`, or whose bound Unix path under its write roots, matches
     that address: the server side depends on the client's bytes.
   - **Shared ports.** Two actions that bind the same inet address and port
     (`RawRecord.Binds`) both get `reads-product`: one's server may have
     answered the other's client. The kernel can also hand out the same port to
     different actions at different times, which the trace cannot tell apart.
   - **Unknown write sets.** An action with any `trace-incomplete` reason has
     an unknown write set, so no other action's checks can be trusted: **its
     whole shard emits nothing**.
   - **A tracer that did not finish cleanly.** If the shard's strace exited by a
     signal or with a non-zero status, its tracees were killed or detached, so
     the shard's records cannot be trusted: **the whole shard emits nothing**,
     whichever action the strace was tracing.
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
9. **`kiln:recorder`** is the integer constant `RecorderVersion` (now `1`) in
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

**Before recording.** The sandbox tests skip where the kernel refuses
unprivileged namespaces, and a runner image, kernel or `strace` can change
under a green CI. The `kiln-record` job (stage 4) must therefore run the same
probe before it records anything: a trivial step through `kiln sandbox-exec`
with the real configuration, which fails the job (records nothing) when the
sandbox cannot be built. The required `kiln-sandbox` CI job also runs on every
scheduled and dispatched full build.

## 14. Residual risks

These are known and named. They are measured rather than closed.

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
