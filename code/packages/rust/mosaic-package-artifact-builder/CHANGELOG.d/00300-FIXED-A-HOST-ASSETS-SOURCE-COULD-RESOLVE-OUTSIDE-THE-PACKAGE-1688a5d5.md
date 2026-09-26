### Fixed — a `[host_assets]` source could resolve outside the package

`install_host_effects` validates its source by resolving it and requiring the
result to sit inside the package root, then refusing anything that is not a
regular file. `install_host_assets` did neither — and it is the **older and far
wider door into the same output directory**, the one every package shipping a
host file uses today.

`safe_manifest_relative_path` is lexical. It refuses `..` and absolute paths,
and a symlink sitting at an innocent-looking relative path goes straight
through. The bytes were then copied into the emitted project and, on several
backends, compiled into it.

Both guards are now applied here too, mirroring the sibling rather than
inventing a second spelling:

- **Containment.** The source is canonicalized and must start with the
  canonicalized package root. The root is resolved once outside the loop.
- **Regular files only.** A FIFO inside the package resolves *and* contains, and
  `fs::read` on one blocks forever — a build that hangs rather than fails.
  Directories and dangling links already failed loudly; this covers the one that
  did not.

The test was written failing first and drives the whole `build_package` path
rather than the helper, so it exercises the code as a build actually reaches it.
It also borrows the trap recorded in the host-effects version: the secret has to
live **genuinely outside** the package root, because a first draft of that test
put it inside and passed a different directory as the root — so the path never
resolved and the containment branch never ran, failing on a missing file while
appearing to exercise the guard.

Verified not to break the packages that use this today: `engram-app` emits for
Flutter and `venture-browser` for React, both with host assets, both still
building.

