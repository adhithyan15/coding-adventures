---
category: Security boundaries
---

# Dart cannot create an owner-only file or directory (createTemp is 0777 minus umask), and a private directory does not make path-based writes safe -- open the temporary O_EXCL|O_NOFOLLOW through libc

**What went wrong.** The Flutter platform library (UI87 §7.7) has to save a
file the way the Compose, Qt and XAML libraries do: write a temporary
owner-only (0600), give it the replaced file's rwx bits, rename it into place
-- so saving over a private file never leaves the new one readable by other
users. Two mistakes, one after the other:

1. The plan assumed `Directory.createTemp` wraps `mkdtemp(3)` and so yields a
   0700 directory. A probe on the Dart VM (3.13) showed it is `0755` (0777
   minus the umask), and `File.create` gives `0644`. dart:io has no `chmod`,
   no `fchmod`, no create-with-mode, no `O_EXCL` that keeps its handle
   (`createSync(exclusive: true)` closes the file; `openSync` then reopens by
   path with `O_CREAT | O_TRUNC`), and no `O_NOFOLLOW` or `O_NONBLOCK`.
2. The first fix made the *container* private instead: libc `mkdir(…, 0700)`
   beside the target, then dart:io writes, a libc `chmod` and a recursive
   delete, all by path inside it. The security review broke it: 0700 stops
   other users entering the directory, not renaming it. In a parent folder
   another user can write to (group-writable, not sticky), they rename the
   fresh directory away and put their own in its place, with `contents` a
   symlink to the victim's `~/.bashrc`. The by-path open then truncates and
   writes it, `chmod` follows the link, and the recursive delete runs over
   the attacker's tree. The same shape was on Windows: an exclusive
   `createSync`, then a reopen by path.

**Fix.** Do what the Qt and SwiftUI libraries do, through dart:ffi. Call
`open(temp, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600)`
beside the target. Write, `fchmod` and `fsync` through that descriptor, then
`close` and `rename`, and `unlink` only if something failed. Copy the mode
from the replaced file only when `lstat` (Linux `statx`) says it is a regular
file this user owns. `open` is variadic, so declare it with
`VarArgs<(Uint32,)>`. The `O_*` values differ by OS and by architecture
(`O_NOFOLLOW` is 0x20000 on Linux x64, 0x8000 on Linux arm64 and 0x100 on
macOS), so keep one table per `Abi`, pin it in a test, and probe the running
kernel with raw `open` calls in the harness. On Windows the equivalent is
`CreateFileW(CREATE_NEW)` and `WriteFile` / `FlushFileBuffers` on that
handle, then `MoveFileExW`. Open reads go the same way:
`O_NONBLOCK | O_NOFOLLOW`, then `fstat` on the descriptor and `read` through
it, so a FIFO swapped in cannot block.

**Do differently.** Probe the actual mode a runtime's "temp" API produces
before building a permission argument on it -- `mkdtemp` semantics are not
universal. Any operation *by path* after creation (open, chmod, delete) is
only as safe as every directory on that path. A private directory does not
protect names inside it from someone who can rename the directory itself.
Operate on the handle you created, or you have not closed the race. When a
runtime's file API cannot do that, go to the OS through its FFI rather than
building a workaround out of the safe-looking calls it does have. Check any
workaround against a sibling library that already solved the problem (here
`templates/qt/MosaicPlatformEffects.cpp`) before believing it. Also note the
neighbouring Dart traps found the same way: the base64 decoder accepts the
URL-safe alphabet and `%3D` padding (check the alphabet first for parity with
other hosts), and dart:io types a device as `notFound`.
