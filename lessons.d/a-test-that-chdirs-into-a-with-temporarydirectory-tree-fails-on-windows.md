---
category: Cross-platform & Windows BUILD_windows
---

# a test that chdirs into a with-TemporaryDirectory tree fails on Windows because the block deletes the cwd before monkeypatch restores it

**What went wrong.** #18815's windows-2025 build selected
`python/programs/scaffold-generator`. Two old `TestRepoRootDetection` tests
failed with `PermissionError: [WinError 32] ... being used by another process`.
Each one ran `monkeypatch.chdir(nested)` inside
`with tempfile.TemporaryDirectory() as tmp:`. The `with` block deletes the tree
when it ends, but monkeypatch only restores the working directory at test
teardown, which comes later. Linux and macOS let a process delete its current
directory, but Windows does not, so the cleanup failed there. The tests had
been there since #848 and only ran on Windows when a PR touched the package.

**The fix.** Use pytest's `tmp_path` fixture. Pytest removes it well after
teardown has restored the working directory.

**Do differently.** Never `chdir` (or `monkeypatch.chdir`) into a directory
that a `with` block will delete. Use `tmp_path`, or restore the working
directory before the block ends, for example with `contextlib.chdir(...)`
nested inside the `with`. A package that is rarely selected can hide a
Windows-only failure for months, so read its Windows log before calling the
failure your own regression.
