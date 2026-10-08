# Changelog

## Unreleased

### Added

- `LinuxConfinement`, the D18S Linux applier for compiled agents (build step
  4; #13980 P2.4).
  - `prepare(plan, executable)` checks the plan (`launch_preconditions`, a
    Linux plan, only exact `Direct` fs read and write grants). It then
    builds, in the parent, a Landlock ruleset and a seccomp-BPF program.
  - `apply(&mut command)` runs `chief-of-staff-spawn-isolation`, then a
    `pre_exec` hook. In the forked child, the hook sets `PR_SET_NO_NEW_PRIVS`,
    then `landlock_restrict_self`, then `seccomp(SET_MODE_FILTER)`. Any
    failure refuses the spawn.
- Landlock, ABI-negotiated. It handles every filesystem right the ABI
  knows, plus TCP (ABI 4) and scoping (ABI 6), with no rules for either.
  - Allowed: the executable and its ELF interpreter (read and execute),
    the system library directories and `/etc/ld.so.cache` (read),
    `/dev/null`, `/dev/urandom`, and the plan's grants.
  - A grant must be an existing regular file, reached through no symlink,
    and not the agent's own image.
  - The interpreter named by `PT_INTERP`, which the agent's author wrote,
    is resolved, must be named like a loader (`ld-*.so*`) inside a library
    directory, and is opened like a grant.
  - A write grant inside a library directory is refused (S-I6).
  - The ELF parse checks all of its arithmetic, so a hostile header is a
    refused launch, never a panic.
  - A `Direct` grant needs ABI 3 or later.
- The exec is the hook's own: `execveat` on the descriptor `prepare`
  opened, with `AT_EMPTY_PATH` (S-I4d). The seccomp program kills `execve`,
  and allows `execveat` only on that descriptor number with that flag. What
  runs is the file that was parsed and given its Landlock rule, even if its
  path is replaced after `prepare`.
- The agent's environment is a closed set (S-I4a): exactly the variables set
  on the command with `env`, and nothing inherited, with or without
  `env_clear`.
- seccomp:
  - an arch check (x86_64 with x32 refused, and aarch64);
  - an allowlist with `SECCOMP_RET_KILL_PROCESS` as the default;
  - argument filters for `clone` (threads only), `clone3` (`ENOSYS`),
    `ioctl` (no `TIOCSTI` or `TIOCLINUX`), `prctl` (thread names) and
    `prlimit64` (own process);
  - `EACCES` for `readlink` and `readlinkat`.
- The `linux-sandbox-probe` test child, and tests that check each denial
  from inside a real confined child, each with an unconfined control.

### Added (P2.5, D18S step 5: the shim)

- **Exec once.** The seccomp filter is installed with
  `SECCOMP_FILTER_FLAG_NEW_LISTENER`, and `execveat` on the pinned
  descriptor returns `SECCOMP_RET_USER_NOTIF`.
  - The hook sends the listener over a per-`apply` socketpair. `sendmsg` is
    allowed only on that descriptor number.
  - A supervisor thread answers the first notification with `CONTINUE`,
    then closes the listener, so every later exec gets `ENOSYS`.
  - This closes the P2.4 residual: re-running the binary, or the loader, by
    `dup2` or an absolute path.
- **The environment's closed set**: `GRANTABLE_ENVIRONMENT`, with S-I4a's
  deny-list as a redundant check. `apply` now returns
  `Result<&mut Command, ConfinementError>`, and refuses any other name with
  `ConfinementError::Environment`.
- **Checks in the child**, before Landlock:
  - exactly one thread;
  - fds 0-2 open, and every other descriptor close-on-exec.
- **Launch probes (S-P4)**:
  - `readlinkat` must return seccomp's `EACCES`;
  - opening `/` must return Landlock's.
- `launch_verification()` lists the classes each launch confirms and those
  only CI does.
- Security review round 4 fixes:
  - **The seal** (M1): after sending the listener, the hook stacks a second
    filter that kills `sendmsg` and `seccomp`. A pinned descriptor number
    had let an agent with a socket channel `dup2` it there and pass
    descriptors. The first filter allows `seccomp` only as
    `SET_MODE_FILTER` with no flags.
  - **Poisoning** (M2): a failed `apply` checks everything before touching
    the command, and poisons it with a hook that refuses every spawn.
  - The exec-once thread validates what it receives, as one descriptor that
    is a seccomp listener; closes extras; keeps serving after a bad message;
    and bounds its wait at 30 s (L1).
  - `MSG_NOSIGNAL` on the send (L2).
  - Bounds-checked `getdents64` parsing (L3).
  - Environment values must be names, not paths (L4).
  - An availability note for `EBUSY` and missing `/proc` (L5).
- Security review round 5, PASS. Its LOWs are fixed too:
  - the listener's payload carries the sender's pid, and CONTINUE is
    answered only for that pid's `execveat`;
  - exec-once start failures are `ConfinementError::ExecOnce`.
