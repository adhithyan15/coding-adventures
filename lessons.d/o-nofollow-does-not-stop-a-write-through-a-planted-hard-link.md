---
category: Security boundaries
---

# O_NOFOLLOW does not stop a write through a planted hard link

The Go JVM backend's Unix writer opened `demo/Example.class` with `O_WRONLY|O_CREAT|O_TRUNC|O_NOFOLLOW` through an `openat` descriptor walk and treated that as link-safe. It is only symlink-safe. A hard link planted at the target is another name for the victim's inode, so the open truncated and overwrote the victim (any file on the same filesystem the planter could name; macOS allows hard links to files you do not own). The security review of #17847 flagged it as pre-existing.

Never open an attacker-reachable target name for writing. Write to a fresh sibling created with `O_CREAT|O_EXCL|O_NOFOLLOW` (it cannot open anything that already exists), then `renameat` it over the target. Rename replaces the directory entry and leaves the old inode's contents alone. Unlink the temp file on every failure. Every secure-writer test suite needs an `os.Link` sentinel case next to the symlink one.
