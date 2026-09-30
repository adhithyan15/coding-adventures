---
category: Security boundaries
---

# Dart cannot create an owner-only file or directory: createTemp is 0777 minus umask, not mkdtemp's 0700, and there is no chmod

**What went wrong.** The Flutter platform library (UI87 §7.7) has to save a
file the way the Compose, Qt and XAML libraries do: write a temporary
owner-only (0600), give it the replaced file's rwx bits, rename it into place
-- so saving over a private file never leaves the new one readable by other
users. The plan was to lean on `Directory.createTemp`, assuming it wraps
`mkdtemp(3)` and so yields a 0700 directory in which the temporary could be
written safely. A probe on the Dart VM (3.13) showed the directory is
`0755` (0777 minus the umask), and `File.create` gives `0644`. dart:io has no
`chmod`, no `fchmod`, no create-with-mode and no `O_NOFOLLOW`/`O_NONBLOCK`.
A file created with dart:io and narrowed afterwards has a window in which
another local user can open it and keep the descriptor -- permissions are
checked at `open`, so later writes are readable through it.

**Fix.** Two non-variadic libc calls through dart:ffi: `mkdir(path, 0700)`
creates a private directory beside the target in one step (and fails if the
name exists, so a planted directory or link is refused), the bytes are
written and flushed inside it with dart:io (unreachable to anyone else,
whatever the file's own mode), `chmod` sets the final mode on a path nobody
else can swap, and the file is renamed onto the chosen path. `mode_t` is 32
bits on Linux and 16 on Apple platforms, so each signature is declared per
platform. The headless harness asserts 0600 for a new file, the replaced
file's bits for an old one, and that setuid is not carried over.

**Do differently.** Probe the actual mode a runtime's "temp" API produces
before building a permission argument on it -- `mkdtemp` semantics are not
universal (Dart's `createTemp` only guarantees a unique name). When a runtime
has no way to create with a mode, make the *container* private first; a
narrowing `chmod` after creation is always too late for a descriptor someone
already opened. Also note the neighbouring Dart traps found the same way: the
base64 decoder accepts the URL-safe alphabet and `%3D` padding (check the
alphabet first for parity with other hosts), and `File.open` on a FIFO blocks,
so check the type before opening.
