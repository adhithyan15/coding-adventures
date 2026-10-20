---
category: Security boundaries
---

# Python os.open cannot open a directory on Windows, so dir_fd traversal needs a path-check fallback

The TypeScript `ir-to-jvm-class-file` writer embedded a Python script that
walked the output tree with `os.open(dir, O_RDONLY|O_DIRECTORY|O_NOFOLLOW)`
and `dir_fd=`. On Windows `os.open()` on a directory always raises
`PermissionError [Errno 13]` and `os.supports_dir_fd` does not contain
`os.open`, so every write failed. It sat latent from April (#829) until an
unrelated PR touched `typescript/lexer`, which pulled the package into the
`windows-2025` build. Its symlink-rejection tests had been "passing" on
Windows only because the writer failed before it ever looked at a link.

Gate descriptor traversal on `os.open in os.supports_dir_fd and os.mkdir in
os.supports_dir_fd`, and give the other branch a path-check walk: create, then
`lstat`, and refuse `S_ISLNK` *or* `st_file_attributes &
FILE_ATTRIBUTE_REPARSE_POINT` (Python reports junctions only through that
bit). Write to an `O_EXCL` temp sibling and `os.replace` it over the target.
Make the strategy selectable so POSIX test runs exercise the fallback too, and
make rejection tests assert the sink is untouched — an assertion that only
checks "it threw" cannot tell a working guard from a writer that always fails.
Also send relative paths to such scripts `/`-separated: `path.join` on Windows
yields backslashes, which `PurePosixPath` treats as one component.
