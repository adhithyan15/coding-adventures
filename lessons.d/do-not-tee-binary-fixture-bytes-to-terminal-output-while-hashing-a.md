---
category: Repo policy / workflow reminders
---

# Do not tee binary fixture bytes to terminal output while hashing a stream

Piping a downloaded DER fixture through `tee /dev/stderr` while also hashing it
prints raw binary into command output. The bytes may be public, but the output
becomes unreadable and can corrupt terminal-oriented diagnostics. Compute a
digest directly from the stream, or transform the stream to textual hex before
printing; use `tee` only when its file target is intentional and reviewed.
