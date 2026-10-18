---
category: Security boundaries
---

# A Windows directory handle does not make subsequent path-based writes safe from a directory swap

Opening each ancestor with `CreateFileW`, `FILE_FLAG_OPEN_REPARSE_POINT`, and
no `FILE_SHARE_DELETE` did not pin a path-based ledger write: a deterministic
Windows test renamed `chapters.d` successfully while its directory handle was
open. A later full-path `os.replace` could then traverse a junction substituted
at the old name and overwrite an unrelated outside file.

For an untrusted shard tree, open each child relative to the already-open
parent directory handle with `NtCreateFile` and `FILE_OPEN_REPARSE_POINT`,
reject reparse attributes on the returned handle, and create, rename, or
dispose of the leaf through those handles. Keep a privilege-free junction-swap
test that preserves an outside sentinel, plus real symlink assertions on Unix.
Do not treat path checks, post-hoc identity checks, or share flags alone as a
race-proof replacement for handle-relative operations.
