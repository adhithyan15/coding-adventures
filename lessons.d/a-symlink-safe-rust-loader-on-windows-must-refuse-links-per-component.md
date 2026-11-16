---
category: Rust
---

# A symlink-safe Rust loader on Windows must refuse links per component, not canonicalize, because canonicalize returns a device path that a device-never-inside containment rule rejects

The obvious way to make a Rust file loader symlink-safe is to `canonicalize` the
path and check the result is still under the repository root. On Windows
`canonicalize` returns a verbatim device path (`\\?\C:\...`). The lane runners'
shared containment rule (VER04i 9.1) deliberately says a device path is never inside
anything, so the canonicalized path would be refused even when it is perfectly
contained, and "fixing" that by exempting device paths would reopen the very
cross-drive and namespace escapes the rule exists to close.

`lane_loader::read_repo_file` therefore checks containment lexically on the
un-canonicalized absolute path, and gets symlink safety by *refusal*: it walks the
validated relative path one component at a time with `symlink_metadata`, rejects
any symbolic link (and any Windows reparse point, via `FILE_ATTRIBUTE_REPARSE_POINT`,
which covers junctions that `is_symlink` alone may not), and requires the final
component to be a regular file. The repository root itself is trusted and may be a
link. Say plainly what this does not defend against: a process that can rewrite the
tree between the check and the open.
