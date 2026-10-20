---
category: Cross-platform & Windows BUILD_windows
---

# Embedded Python helpers that open directories with os.open or dir_fd need a Windows fallback

**What went wrong.** PR #17722 (forme TOML pipelines) touched
`typescript/lexer`, which made `nib-jvm-compiler` and
`brainfuck-jvm-compiler` affected, so the Windows build also built their
dependency `typescript/ir-to-jvm-class-file` for the first time in a while.
Its `writeClassFile` shells out to an inline Python script that walks the
output tree with `os.open(dir, O_RDONLY | O_DIRECTORY | O_NOFOLLOW)` and
`dir_fd=`. Windows cannot `os.open()` a bare directory at all
(`PermissionError: [Errno 13] Permission denied: 'C:\...\Temp\ts-jvm-backend-…'`)
and `os.supports_dir_fd` is empty there, so every write failed.

The symlink-rejection tests stayed green on Windows the whole time. They
only assert that a `JvmBackendError` is thrown, and *every* write threw.
A rejection test that passes when the happy path is broken proves nothing.

**The fix.** Mirror the Python sibling package: choose the dir-fd walk only
when `os.open` and `os.mkdir` are in `os.supports_dir_fd`, and otherwise use
path checks (`islink`/`isjunction` on every component, then a temp file plus
`os.replace()`). Also, the TypeScript side built the relative path with
`node:path` `join`, which emits `demo\Example.class` on Windows. The helper
parses that with `PurePosixPath` as a single component, which would have
skipped the per-directory symlink checks. Join with `/` explicitly, and
reject `\`, `:` and NUL inside components.

**Do differently.**
- Give the helper a strategy argument, so the Windows branch runs (and is
  covered) on Linux CI too.
- In rejection tests for the fallback, pin the error message and assert that
  nothing landed in the sink, so "it threw" can't hide a broken writer.
- Never build a path for another process's POSIX parser with `node:path`
  `join`. Use `path.posix.join` or `parts.join("/")`.
