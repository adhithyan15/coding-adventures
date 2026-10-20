---
category: Cross-platform & Windows BUILD_windows
---

# Go's os.Lstat reports a Windows junction as ModeIrregular, not ModeSymlink

Since Go 1.23 (the `winsymlink` GODEBUG, on by default), `os.Lstat` on a
Windows junction (mount point) returns `os.ModeIrregular`; only real symbolic
links get `os.ModeSymlink`. A Windows link check written as
`info.Mode()&os.ModeSymlink != 0` therefore lets junctions through, and a
junction is the link an unprivileged attacker can plant (`mklink /J` needs no
Developer Mode or admin, unlike `mklink /D`). Setting `winsymlink=0` flips
junctions back to `ModeSymlink`, so the mode bits depend on build settings.

Check the attribute instead, which does not depend on how the runtime classifies
the entry:

```go
data, ok := info.Sys().(*syscall.Win32FileAttributeData)
isReparse := ok && data.FileAttributes&0x0400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
```

Put it in a `//go:build windows` file with a `false` stub elsewhere, and make
directory-link tests create junctions on Windows so they need no privilege.
Hit while porting `code/packages/go/ir-to-jvm-class-file`'s secure writer to
Windows.
