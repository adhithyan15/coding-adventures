### Added — tests for the Electron quarantine on a dangling link and a failed copy

`tests/snapshot_quarantine.rs` now pins two cases that decide whether a
quarantine name can be used up by nothing (Unix only):

- **A dangling link at `.corrupt`** is skipped as taken. The copy lands at
  `.corrupt.1`, the link is left as it was, and nothing is written through it
  (its target stays absent).
- **A copy that fails partway** leaves no file at any quarantine name. The
  test runs Node under `ulimit -f 0` with SIGXFSZ ignored, so every file write
  fails with EFBIG. The snapshot keeps its bytes, saving is switched off, and
  the error is reported rather than "already holds 10 unreadable snapshots".

No host change was needed: `copyFileSync(…, COPYFILE_EXCL)` creates the copy
with `O_CREAT | O_EXCL` (which never follows a link) and libuv removes the
destination when the copy fails. Both tests pass against the existing
`host/electron/host.js`. They close the open questions in the backlog ("verify
libuv cleans up" and "no dangling-symlink quarantine case"). `run_node` gained
`run_node_with`, which lets a test start Node under a shell that limits it.
