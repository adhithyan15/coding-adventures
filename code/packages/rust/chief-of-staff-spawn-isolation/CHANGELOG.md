# Changelog

## Unreleased

### Added

- `VerifiedExecutable::descriptor`: a duplicate of the descriptor that was
  hashed, for the confinement to execute (P2.6d-3).

### Removed

- `isolate_and_exec` (P2.6d-3). The broker is now launched only by
  `chief-of-staff-linux-sandbox`, confined; its descriptor-placement tests
  moved there.

### Added (earlier)

- `VerifiedExecutable` and `isolate_and_exec` (Linux; D18S S-K1, S-I3;
  #13980 P2.6d-2a): launch the per-agent broker, holding exactly its key
  descriptors.
  - `VerifiedExecutable::open(path, sha256)` opens the binary once and
    hashes it through that descriptor, with its size checked before and
    after. It must be a regular file, not group- or world-writable, owned
    by root or this user. `verify()` re-hashes it.
  - `isolate_and_exec` re-verifies, does what `isolate` does, places the
    given descriptors at 3..3+n, and `execveat`s the verified descriptor
    itself. A failure after the descriptors are placed exits the child
    with status 127, since std's exec-error pipe may sit in a slot.
  - argv and envp are exactly the command's; an interior NUL is refused,
    never cut.
  - The probe now also reports inodes, arguments and environment.
- `kill_session(&Child)`: `SIGKILL` to an isolated child's process group.
  An isolated child leads its own session, so this ends whatever it left
  behind as well (D18S S-K5; #13980 P2.6b). On Windows it does nothing.
  A test checks that a grandchild holding the child's stdout dies with it.
- `has_exited(&Child)`: whether a child has exited, without reaping it
  (`waitid(WNOWAIT)` on Unix), so `kill_session` can run while the pid is
  still the child's. `kill_session` now documents that a descendant which
  called `setsid` or `setpgid` itself survives it.

### Added

- `isolate(&mut Command)`: descriptor isolation for D18 agent spawns (D18S
  S-I2, S-I3; #13980 P2.2).
  - stderr goes to `/dev/null`.
  - The spawn is refused if fd 0, 1 or 2 is a terminal.
  - The child starts its own session (`setsid`), so it has no controlling
    terminal to open as `/dev/tty`.
  - Every descriptor above 2 is made close-on-exec between fork and exec.
    - Linux uses `close_range` with `CLOSE_RANGE_CLOEXEC`, or else lists
      `/proc/self/fd` with `getdents64`, and refuses the spawn if neither
      works.
    - Elsewhere, an `fcntl` loop up to the larger of the soft and hard
      limits.
  - On Windows, only stderr is set.
  - Descriptors are marked rather than closed, so std's exec-error pipe
    survives and a failed exec is still a spawn error.
- The `spawn-isolation-probe` test child, and tests that check each guarantee
  from inside a real child process.
