## Unreleased — Unix class-file writer: hard links, missing roots, absolute paths

### Fixed

- **Hard-link write-through (security).** The Unix descriptor walk opened the
  class file with `O_WRONLY|O_CREAT|O_TRUNC|O_NOFOLLOW`. `O_NOFOLLOW` only
  refuses *symlinks*; a hard link planted at `demo/Example.class` is just
  another name for the victim's inode, so the open truncated and overwrote the
  victim (any file on the same filesystem the planter could name). The walk now
  writes to `.<name>.<16 hex>.tmp`, opened with
  `O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC`, and `renameat`s it over the
  target, which replaces the directory entry and leaves the victim's contents
  alone. The temp file is unlinked on any failure. An existing target that is
  not a regular file (`fstatat` with `AT_SYMLINK_NOFOLLOW`) is still refused
  with "class_filename points at a symlinked or invalid output file". The
  path-check walk already worked this way.
- **Missing output roots on macOS.** When the output root did not exist, the
  Unix walk created it by walking down from `/` with `O_NOFOLLOW`, so it
  refused the system links `/var -> /private/var` and `/tmp -> /private/tmp`.
  For example, `WriteClassFile(a, "$TMPDIR/out/new")` failed with "symlinked or
  invalid path component". The walk now starts from the deepest existing
  ancestor of the root, found and canonicalised by the same
  `locateExistingRoot` the path-check walk uses, and runs
  `mkdirat`/`openat(O_NOFOLLOW)` only for the parts it creates. A deepest
  existing ancestor that is itself a link is still refused on both walks, so
  `/tmp/new` on macOS (where `/tmp` is the deepest existing component) still
  fails. Pass `/private/tmp/new` or create the parent first.
- **Relative return path.** With `outputDir == ""`, the Unix walk returned
  `demo/Example.class` relative to the working directory, because
  `EvalSymlinks(".")` is `"."`. It now returns an absolute, canonical path, as
  the path-check walk does.

### Changed

- A replaced class file is a new inode. It gets mode `0644` minus the umask
  and the writer's owner, instead of keeping the old file's mode and owner.
- The temp-file naming is shared by both walks (`temporaryClassFileName`).

### Tests

- New `TestWriteClassFileDoesNotWriteThroughHardLink` (both strategies):
  `os.Link`s a sentinel to `demo/Example.class` and checks that the sentinel is
  unchanged and the class file is written.
- The missing-root, relative-root and default-root tests pass the raw
  `t.TempDir()` path (under `/var` on macOS) instead of a canonicalised one.
  The default-root test compares the returned path directly. All of these
  failed on the old Unix walk.
- New `backend_unix_test.go` drives `replaceFileAtomicallyAt` directly for an
  uninspectable target (`ENAMETOOLONG`) and an unwritable directory.
