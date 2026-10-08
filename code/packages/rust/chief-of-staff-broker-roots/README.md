# chief-of-staff-broker-roots

Broker roots for the D18 Chief daemon (D18S S-K5, step 6 P2.6c).

The broker acts on paths that agents name, with authority the agents do not
have. That makes it the classic confused deputy: a hostile path can trick it
into opening the vault on an agent's behalf. This crate holds the two S-K5
rules that prevent that, and every brokered `fs:*` operation must go through
it.

1. **Roots are disjoint from the never-grantable set**, which is proved once,
   at start. `BrokerRoot::open` canonicalizes the root and each
   never-grantable path (the vault and its directory, the audit log, the
   runtime image, ...). It refuses the root if either contains the other.
2. **Agent names resolve strictly beneath a root**, by the kernel, in one
   step. The broker never resolves a name to a string and then opens that
   string, because a symlink can be swapped in between the two steps.

```rust,no_run
use chief_of_staff_broker_roots::{Access, BrokerRoot};
use std::io::Read;
use std::path::Path;

let root = BrokerRoot::open(
    Path::new("/home/me/agent-notes"),
    &[Path::new("/home/me/.chief/vault"), Path::new("/home/me/.chief/audit")],
)?;
let mut text = String::new();
root.open_beneath("2026/october.md", Access::Read)?
    .read_to_string(&mut text)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## The rules

| Asked for | Answer |
|---|---|
| a root inside, equal to, or containing a never-grantable path, including through a symlink, or a vault path that does not exist yet | `Overlap` |
| a never-grantable path with `..` after its last existing directory, or under a directory that exists but cannot be searched (it cannot be compared exactly) | `Overlap`: the proof fails closed |
| a root whose path changed between the proof and the open (the descriptor's own path, from the kernel, differs) | `RootMoved` |
| a name that is empty, absolute, longer than 4 KiB, contains NUL, or has a `.`, `..` or empty component | `InvalidName`, before any system call |
| a symlink anywhere on the way, even one that points inside the root | `Refused` |
| a `/proc` magic link, or a mount point (Linux) | `Refused` |
| a missing file | `Refused`. Nothing is created. |
| a directory, FIFO, socket or device | `NotRegularFile` |
| a regular file with a second hard link | `MultipleLinks` |
| a plain regular file | the file: close-on-exec, blocking, opened only for the access asked, never truncated |

Some of these choices need explaining:

- **Why refuse every symlink?** A symlink that points inside the root today
  can be repointed outside tomorrow. Refusing all of them leaves nothing to
  judge.
- **Why refuse hard links?** A hard link to the vault, made inside a root,
  *is* the vault. No path check can see it. Refusing every file with more
  than one link catches that, without the broker having to know the vault's
  inode. It is defence in depth, not a guarantee. Someone who can make the
  link can also remove it between the open and the check. Agents cannot
  link at all under their sandbox plan. Other users are stopped by the
  daemon keeping every never-grantable directory mode 0700, since linking
  a file needs search permission on its directory.
- **Why no truncation for writes?** `O_TRUNC` takes effect inside `open`,
  before the checks, so it would truncate a file the checks then refuse.
  The file opens at offset 0. A caller replacing the contents calls
  `set_len` after `open_beneath` returns.
- **Why never a directory?** A directory descriptor hands over its whole
  subtree, for good, beyond any later path check.
- **Why open non-blocking?** Opening a FIFO with no writer blocks forever.
  The broker opens non-blocking, checks with `fstat` that the file is
  regular, and only then switches back to blocking.

## Platforms

| Platform | Primitive | Mount crossing |
|---|---|---|
| Linux 5.6+ | `openat2(RESOLVE_BENEATH \| RESOLVE_NO_SYMLINKS \| RESOLVE_NO_MAGICLINKS \| RESOLVE_NO_XDEV)` | refused |
| macOS 11+ | `openat(..., O_NOFOLLOW_ANY)`, after `validate_name` refuses `..` and absolute names. `BrokerRoot::open` first proves the flag is honored: `/etc` is a symlink, so `/etc/hosts` must fail with `ELOOP`. An older kernel that ignores the flag gets `Unsupported`. | not checked: mounting inside a root needs privilege |
| others | none | `BrokerRoot::open` returns `Unsupported`, so the broker does not offer the operation (S-P3) |

## Tests

`tests/beneath.rs` builds a scratch tree for each case and checks every row
of the table above against the real filesystem. Each check in `src/lib.rs`
was mutation-tested: removing it fails at least one test. The FIFO test runs
under a deadline, so a blocking open fails the test instead of hanging CI.

```sh
cargo test -p chief-of-staff-broker-roots
```
