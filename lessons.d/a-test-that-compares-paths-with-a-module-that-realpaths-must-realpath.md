---
category: Testing & coverage
---

# A test that compares paths with a module that realpaths must realpath too, or it fails only on macOS

**What went wrong.** `lesson-cache.test.ts` installed an `fs.openSync` hook
that fired only when `path === join(cacheDir, entry)`. The cache under test
resolves its directory with `realpathSync.native` and opens every entry under
that REAL path. On Linux `tmpdir()` is already real, so the two strings matched
and the test passed. On macOS `tmpdir()` is `/var/folders/…`, and `/var` is a
symlink to `/private/var`. The hook never matched and `expect(swapped).toBe(true)`
failed on every macOS run. That was easy to misread as one more load timeout,
because six real timeouts failed in the same job.

**Fix.** Compare against `join(realpathSync.native(cacheDir), entry)`, the same
spelling the code under test uses.

**Do differently.** When a module canonicalises paths, any test that intercepts
or compares paths must canonicalise the same way. Reproduce macOS on Linux by
running the test with `TMPDIR` pointing at a symlink to a real directory
(`ln -s real link; TMPDIR=$PWD/link npx vitest run <file>`). When a CI job
reports many timeouts, still read every non-timeout failure separately: an
assertion failure among timeouts is usually a different bug.
