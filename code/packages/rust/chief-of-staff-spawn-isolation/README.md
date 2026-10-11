# chief-of-staff-spawn-isolation

Descriptor isolation at the D18 agent spawn site (D18S S-I2, S-I3).

An agent process should start holding exactly three descriptors: its channel
on stdin and stdout, and a stderr that goes nowhere useful to it. Any other
descriptor it inherits is an escape that needs no syscall the sandbox denies,
because the process already holds the object. A terminal on a channel
descriptor grants `ioctl(TIOCSTI)`, which is command execution as the
supervisor's user.

`isolate(&mut command)` arranges this on a `std::process::Command`. On Unix,
the child also starts its own session (`setsid`), so it has no controlling
terminal it could open as `/dev/tty`:

| fd  | the child gets             | why                                                |
|-----|----------------------------|----------------------------------------------------|
| 0,1 | the caller's pipes         | the channel (S-I2); the spawn is refused if either is a terminal |
| 2   | `/dev/null`                | runtime noise stays out of the protocol stream and away from the supervisor's own stderr |
| 3+  | nothing: close-on-exec     | a leaked descriptor is a complete escape (S-I3)    |

```rust
use std::process::{Command, Stdio};

let mut command = Command::new("agent-runtime");
command.stdin(Stdio::piped()).stdout(Stdio::piped());
// Last, after stdin and stdout: it sets stderr itself.
chief_of_staff_spawn_isolation::isolate(&mut command);
let child = command.spawn()?;
# Ok::<(), std::io::Error>(())
```

## How it works

Between fork and exec, a `pre_exec` hook:

1. refuses the spawn (`PermissionDenied`) if fd 0, 1 or 2 is a terminal,
   using `tcgetattr`, which is async-signal-safe where `isatty` is not;
2. calls `setsid`, leaving the supervisor's session and its controlling
   terminal;
3. marks every descriptor above 2 close-on-exec:
   - on Linux, with `close_range(3, ~0U, CLOSE_RANGE_CLOEXEC)`, or, where
     that is unavailable, by listing `/proc/self/fd` with `getdents64`. If
     neither works, the spawn is refused;
   - elsewhere, with `fcntl` up to the larger of the soft and hard
     descriptor limits, which are read in the parent.

It marks rather than closes. `std` keeps a close-on-exec pipe in the child
to report a failed `exec`. Closing it early would make a failed exec look
like a child that started and exited. The crate's tests check that a
missing program is still a spawn error. Switching the flag to a plain close
makes that test fail.

On Windows only stderr is set. There is no terminal check and no new session,
and inheritable handles still pass to the child, because std spawns with
`bInheritHandles=TRUE`. The explicit `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`
that S-I3 asks for cannot be passed through stable `std`; that is D18S build
step 8.

## A verified executable

The per-agent broker (D18S P2.6d) runs from a binary pinned by its SHA-256
(S-K1: what runs is exactly what was hashed). `VerifiedExecutable` opens the
binary once and hashes it through that descriptor; `verify` hashes it again;
`descriptor` hands out a duplicate of that same descriptor to execute.

It launches nothing itself. `chief-of-staff-linux-sandbox`'s
`LinuxConfinement::prepare_verified` and `apply_inheriting` re-verify it and
exec its descriptor, confined, with the keys at 3..3+n (P2.6d-3). P2.6d-2a's
unconfined `isolate_and_exec` was removed then: there is one way to start a
broker.

Linux only. Other platforms have no `VerifiedExecutable` yet.

## Where it is used

- `chief-of-staff-process-supervisor`: `ProcessHostSupervisor::spawn_verified`,
  the production agent spawn.
- `chief-of-staff-agent-stdio-host`: `StdioAgentSession::spawn` (Level 4).
- `chief-of-staff-host-runtime`: the Deno agent path.

This crate holds the one `unsafe` hook, so those crates keep
`#![forbid(unsafe_code)]`.

## Testing

`tests/isolation.rs` runs the `spawn-isolation-probe` child. It checks that:
- a descriptor deliberately leaked without `FD_CLOEXEC` reaches the child
  without isolation (the control), and does not with it;
- fd 2 is `/dev/null`;
- the child leads its own session;
- a terminal on stdin refuses the spawn;
- a missing program is still a spawn error.
