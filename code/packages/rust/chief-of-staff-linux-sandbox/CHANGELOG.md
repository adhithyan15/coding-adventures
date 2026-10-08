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
  - A `Direct` grant needs ABI 3 or later.
- seccomp:
  - an arch check (x86_64 with x32 refused, and aarch64);
  - an allowlist with `SECCOMP_RET_KILL_PROCESS` as the default;
  - argument filters for `clone` (threads only), `clone3` (`ENOSYS`),
    `ioctl` (no `TIOCSTI` or `TIOCLINUX`), `prctl` (thread names) and
    `prlimit64` (own process);
  - `EACCES` for `readlink` and `readlinkat`.
- The `linux-sandbox-probe` test child, and tests that check each denial
  from inside a real confined child, each with an unconfined control.
