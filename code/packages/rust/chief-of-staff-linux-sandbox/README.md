# chief-of-staff-linux-sandbox

The D18S Linux applier: the kernel boundary for a compiled agent, installed
between fork and exec (D18S build step 4).

`capability-os-sandbox` lowers an agent's manifest to a `SandboxPlan`: what
the agent may do. This crate makes Linux enforce that plan from the agent's
first instruction:

| Layer | What the agent gets |
|---|---|
| `PR_SET_NO_NEW_PRIVS` | no setuid or file-capability privilege, ever |
| Landlock | can open only its own executable, the shared libraries, `/dev/null`, `/dev/urandom`, and the files the plan grants; no TCP; no abstract unix sockets or signals outside its own domain |
| seccomp | an allowlist of syscalls; anything else kills the process (`SIGSYS`) |

It sits on top of `chief-of-staff-spawn-isolation`, which `apply` runs first.
So the agent also starts with no inherited descriptors, a `/dev/null`
stderr, and no terminal.

```rust,no_run
use capability_os_sandbox::{plan_from_json, OsFamily};
use chief_of_staff_linux_sandbox::LinuxConfinement;
use std::path::Path;
use std::process::{Command, Stdio};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let manifest = r#"{"version":1,"package":"rust/agent","capabilities":[],
                   "justification":"Pure computation."}"#;
let plan = plan_from_json(manifest, OsFamily::Linux)?;
let agent = Path::new("/opt/agents/agent");

// In the parent: check the plan, build the ruleset and the program.
let confinement = LinuxConfinement::prepare(&plan, agent)?;

let mut command = Command::new(agent);
command.stdin(Stdio::piped()).stdout(Stdio::piped());
// Last: it sets stderr and registers the pre_exec hooks.
confinement.apply(&mut command)?;
let child = command.spawn()?; // a spawn error if any step failed
# drop(child);
# Ok(())
# }
```

## What is denied, and how

- **Killed by seccomp:**
  - `socket` and `socketpair`;
  - `fork`, `vfork`, and `clone` without `CLONE_THREAD`;
  - `io_uring_*`, `ptrace`, `kill` and its relatives, `bpf`, `mount`,
    `unshare`, `setns`, the `pidfd` family, SysV and POSIX IPC,
    `perf_event_open`, `userfaultfd` and `keyctl`;
  - `ioctl(TIOCSTI)` and `ioctl(TIOCLINUX)`;
  - every `prctl` except thread names;
  - `prlimit64` on another process.
- **`ENOSYS`:** `clone3`. Its flags are in memory, where the filter cannot
  see them, so libc falls back to `clone`.
- **`EACCES`:**
  - opening any path without a Landlock rule;
  - executing anything but the agent itself;
  - `readlink`, which Landlock does not mediate, and which would otherwise
    read `/proc/<supervisor>/fd`.
- **Exec, once.** The agent is started by the hook itself, with
  `execveat(fd, "", AT_EMPTY_PATH)` on the descriptor `prepare` opened.
  Seccomp filters survive exec, so the filter kills `execve`, and routes
  that `execveat` to a seccomp listener (`SECCOMP_FILTER_FLAG_NEW_LISTENER`).
  The hook sends the listener to a thread in the supervisor. That thread lets
  the first exec through, then closes the listener, so every later exec gets
  `ENOSYS` (D18S S-I4d, second option). The agent cannot exec anything at
  all: not another program, not itself, not the loader. The hook then stacks
  a second filter, the seal, that kills `sendmsg` and `seccomp`. The agent
  never has either, even with a socket for a channel.
- **Environment.** The agent gets exactly the variables set on the command
  with `env`, and nothing inherited, with or without `env_clear`. Every name
  must be in `GRANTABLE_ENVIRONMENT` (`TZ`, `LANG`, `LANGUAGE`, the `LC_*`
  categories, `NO_COLOR`). Anything else refuses at `apply`, `LD_PRELOAD`
  and the rest of S-I4a's deny-list above all. Values must be names, not
  paths (`America/New_York` yes, `/etc/passwd` or `../x` no). A refused
  `apply` poisons the command: spawning it anyway fails with `EPERM`.
- **The shim's checks** (D18S step 5), in the child before Landlock: exactly
  one thread (`/proc/self/task`), and every descriptor but 0-2
  close-on-exec (`/proc/self/fd`).
- **Launch probes** (S-P4), after the install and before the exec: a
  `readlinkat` must get seccomp's `EACCES`, and opening `/` must get
  Landlock's. `launch_verification()` lists what each launch confirms and
  what only CI does, for the audit record.

## Inherited descriptors (the broker)

A per-agent broker (D18S P2.6d-3) is a confined process like any agent,
plus its keys:

```rust,ignore
let broker = VerifiedExecutable::open(path, pinned_sha256)?;
let confinement = LinuxConfinement::prepare_verified(&deny_all_plan, &broker)?;
let mut command = Command::new("chief-of-staff-agent-broker");
command.stdin(Stdio::piped()).stdout(Stdio::piped()).env_clear();
confinement.apply_inheriting(&mut command, key_descriptors)?;
let child = command.spawn()?;
```

- `prepare_verified` never opens the path again: it re-verifies the
  binary and works from a duplicate of the verified descriptor.
- `apply_inheriting` parks the descriptors high and close-on-exec, so the
  shim's checks pass. The hook installs and probes everything first, then
  `dup2`s them onto 3..3+n, then execs. From the first `dup2` on, a
  failure exits 127: std's exec-error pipe may have been in a target slot.
- seccomp lets any confined process clear its dumpability
  (`PR_SET_DUMPABLE, 0`) and read it, because exec resets it and a broker
  holds keys. Setting it to anything else is a kill.

## What it refuses to launch

Any of these refuses the launch rather than confining "what it can" (S-P3):

- a plan for another OS;
- a plan that fails `launch_preconditions`;
- no Landlock;
- a `Direct` grant below Landlock ABI 3;
- a `Direct` create or delete grant, which Landlock can express only over a
  whole directory;
- a grant on a directory, on a path through a symlink, on a missing file,
  or on the agent's own executable;
- a write grant inside a library directory;
- an executable whose interpreter (`PT_INTERP`, chosen by the agent's
  author) does not resolve to a loader in a system library directory, or whose ELF
  headers are malformed;
- an architecture other than x86_64 or aarch64;
- at `apply`: an environment name outside the grantable set;
- at spawn: a second thread, a descriptor that would survive the exec, or a
  launch probe that does not answer as installed.

## What it does not do

- **Interpreted runtime profiles.** The shim's mechanics are here. What Deno
  or CPython (`-S -I`) also needs is a profile: more syscalls, and read
  access to the runtime image. That is D18S step 9.
- **Wiring.** `spawn_verified` does not call this yet (step 9). That is also
  where S-I6's never-grantable paths are checked: only the supervisor knows
  where the vault and the audit log live.
- **Every environment.** `SECCOMP_FILTER_FLAG_NEW_LISTENER` fails with
  `EBUSY` under an ancestor filter that already has a listener, as some
  container runtimes install, and the shim's checks need `/proc`. Either
  refuses every launch rather than running unconfined.
- **Metadata.** Landlock mediates opening, not lookup. An agent can still
  `stat` any path, and learn that a file exists, its size and its times. It
  cannot read the file.

## Tests

`tests/confinement.rs` runs `linux-sandbox-probe`, a child that makes one
syscall and reports the outcome. Each denied class is checked twice:
confined, where the probe must die with `SIGSYS` or report `EACCES`, and
unconfined, as a control that shows the probe itself works. Every probe is
harmless when its syscall succeeds, because the controls run unconfined,
and in CI possibly as root.
