# Changelog

## Unreleased

### Added

- `BrokerRoot::open(root, never_grantable)`, D18S S-K5's start-time proof
  (#13980 P2.6c). It canonicalizes the root and each never-grantable path,
  and refuses the root when either contains the other. A never-grantable
  path that does not exist yet is compared through its nearest existing
  ancestor. A relative path is made absolute first. A path with `..` in its
  missing tail cannot be compared exactly, so it is refused as an overlap:
  the proof fails closed.
- `BrokerRoot::open_beneath(name, Read | Write)`. The kernel resolves the
  agent-supplied name beneath the root's descriptor, in one step:
  - on Linux, `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS |
    RESOLVE_NO_MAGICLINKS | RESOLVE_NO_XDEV`;
  - on macOS, `openat` with `O_NOFOLLOW_ANY`;
  - elsewhere, `Unsupported`.

  Names are checked before any system call: relative, no `.`, `..` or empty
  component, no NUL, at most 4 KiB. Only a regular file with exactly one
  link is returned. It is close-on-exec, opened non-blocking until `fstat`
  proves it regular, and never created or truncated.
- `tests/beneath.rs` covers:
  - symlink escapes, absolute and relative, through a directory, and to
    `/proc`;
  - a symlink that stays inside the root;
  - `..` and absolute names;
  - a missing file, which is not created;
  - a directory;
  - a hard link to a file outside the root;
  - a FIFO, under a deadline;
  - the handle's access, close-on-exec and blocking flags;
  - root overlap in both directions, through symlinks, with a
    not-yet-existing vault path, with a relative one, and with a `..` the
    proof cannot resolve;
  - a sibling that only shares a name prefix with the root.
