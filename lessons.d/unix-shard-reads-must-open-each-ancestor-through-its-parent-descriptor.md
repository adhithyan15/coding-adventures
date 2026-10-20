---
category: Security boundaries
---

# Unix shard reads must open each ancestor through its parent descriptor

`_directory` can reject every symlink visible during prevalidation and still
lose the race before `os.scandir(path)` or `os.open(path, O_NOFOLLOW)` runs. On
Unix, `O_NOFOLLOW` protects only the final pathname component: replacing a
checked track directory with a symlink redirects a later read through that
ancestor.

Open the trusted root once, open each descendant with `dir_fd`, `O_DIRECTORY`,
and `O_NOFOLLOW`, and enumerate the pinned leaf descriptor. Open a selected JSON
file relative to that descriptor and check its descriptor is a regular file
before parsing. Deterministic tests should swap an ancestor immediately after
pathname prevalidation for both enumerators and a shard read. This secures the
read boundary, not the separate mutation path or a snapshot of concurrent
in-root changes.
