### Fixed — quarantine never overwrites a taken name, on any platform

Every generated host already skipped a quarantine name when anything was there.
The bugs were in what happened after that check:

- **Compose and SwiftUI could replace a quarantine in a race.**
  - Both moved with `Files.move` (without `REPLACE_EXISTING`) or
    `FileManager.moveItem`. On Linux, macOS and Android, each of those checks
    the target and then calls rename(2), which atomically *replaces* anything
    another process put at the name in between.
  - Both now do what Flutter and Qt already did. They claim the name with an
    exclusive create (`Files.createFile`, or `open(O_CREAT | O_EXCL)` from a
    file-scope helper, since the host's own `close()` shadows the C one).
    Then they rename onto that empty placeholder: `ATOMIC_MOVE` (rename(2),
    or `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`), and rename(2) on
    Apple. The only thing the rename can replace is the placeholder the host
    just made.
  - Compose deliberately does not use `REPLACE_EXISTING` without
    `ATOMIC_MOVE`: the JDK implements it as delete-then-rename, which leaves
    the name free in between.
- **A refused claim switched saving off instead of moving on.**
  - In Flutter, Compose, SwiftUI and XAML, an exclusive step that failed
    always switched saving off, even when it failed because something got to
    the name after the check.
  - Each host now looks at the name again and skips it if anything is there,
    as Qt already did. The error itself is never the test: onto a directory,
    POSIX reports "exists" but Windows reports access denied. This is the
    Engram Electron Windows bug (#18477) in another form.
- **XAML now sees every kind of link.** It counts a name as taken when
  `LinkTarget` reports a link there, so it no longer depends on whether
  `File.Exists` sees a dangling link on that platform.
- **Tests.**
  - Each effect driver now puts a directory at `.corrupt.1`. It checks that
    the directory survives empty and that the rejected file lands in
    `.corrupt.2`.
  - The source-text test pins the claim step and the re-check after it. It
    retires the check-then-rename moves.
