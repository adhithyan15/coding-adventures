---
category: Cross-platform & Windows BUILD_windows
---

# Tests that build scratch dirs under the system temp dir must canonicalize it before a no-symlink path check
**What failed.** Six `vos-model-sat` CLI tests passed on Linux and failed on macOS and Windows CI with `NoVerdict("...: a component is a symlink (it resolves to ...)")`. The tests built a scratch repo under `std::env::temp_dir()` and handed it to `vos-link`, whose loader refuses any path whose canonical form differs from its spelling.

**Why.** CI temp dirs are not canonical. On macOS `/var/folders/...` is a symlink to `/private/var/folders/...`. On Windows runners `C:\Users\RUNNER~1\...` is an 8.3 short name for `C:\Users\runneradmin\...`. Linux `/tmp` is a real directory, so it passed. The loader was right; the tests were wrong.

**Fix.** Do not weaken the check. In the tests, resolve the temp dir first with `std::fs::canonicalize`, and on Windows strip a leading `\?\` verbatim prefix when a drive letter follows (never from UNC paths), in one helper (`real_temp_dir()` in `tests/common/mod.rs`).

**Rule.** Any test that builds scratch dirs under the system temp dir and feeds them to code with a no-symlink or canonical-path check must canonicalize the temp dir first. A local run on a Linux-like path proves nothing. To reproduce locally, point `TMP`/`TEMP` at an NTFS junction (`mklink /J`), which exercises the same refusal when 8.3 names are unavailable.
