## Unreleased — Windows class-file writer

### Fixed

- The package now builds on Windows. `backend.go` imported
  `golang.org/x/sys/unix` unconditionally for its `openat`-based secure writer,
  so `GOOS=windows go vet ./...` failed with `undefined: unix.Close` and any
  Windows build that pulled the package in went red. The writer is split by
  platform:
  - `backend_unix.go` (`//go:build unix`): the existing descriptor-relative
    walk, unchanged.
  - `backend_windows.go`: uses the path-check walk below, and detects
    symlinks *and* junctions through `FILE_ATTRIBUTE_REPARSE_POINT` in the
    `syscall.Win32FileAttributeData` returned by `os.Lstat`. Go 1.23+ reports a
    junction as `ModeIrregular`, not `ModeSymlink`, so the mode bits alone are
    not enough.
  - `backend_other.go` (neither): refuses with "secure class-file output is
    unsupported on this platform", as the Rust port does.
- New `backend_pathwalk.go`: the path-check walk. It finds the deepest existing
  component of the output root (which must be a plain directory) and
  canonicalises it, then creates each missing root component and package
  directory with `Mkdir` followed by `Lstat`, refusing links, junctions and
  files. An existing output that is not a plain file is refused. The bytes go
  to an `O_CREATE|O_EXCL` temp sibling that is then `os.Rename`d over the
  target, which never follows a link at the destination; the temp file is
  removed on any failure.
- Class-filename components containing `\`, `:` or NUL are now rejected on
  every platform. A hand-built artifact could otherwise use a Windows
  separator, a drive-relative name (`C:x.class`) or an NTFS alternate data
  stream (`Example.class:stream`).
- Removed the unused `rejectSymlink` helper.

### Known limitation

- On Windows the path-check walk cannot detect a directory swapped for a
  junction *between* its `Lstat` and the write; closing that window needs
  handle-relative `NtCreateFile` traversal. The Rust, Python and TypeScript
  ports accept the same residual race on Windows. The Unix walk has no such
  window.

### Tests

- Every write-safety test runs under both the `auto` strategy and the forced
  path-check strategy (through the internal `writeClassFileWithStrategy` seam),
  so POSIX runs exercise the Windows walk too. Directory links are made as
  junctions on Windows, which need no privilege. New cases cover missing root
  directories, relative and default (`""`) roots, overwriting an existing class
  file without leaving temp files, a root that is a link or a file, a package
  directory that is a file, an output path that is a directory, a symlinked
  output file (sentinel preserved), uncreatable directories, temp-file cleanup
  when the final rename fails, and the escaping-filename table.
