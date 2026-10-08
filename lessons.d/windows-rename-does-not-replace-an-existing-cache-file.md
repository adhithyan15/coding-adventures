---
category: Cross-platform & Windows BUILD_windows
---

# Windows rename does not replace an existing cache file

A dependency-graph expansion exposed two latent Windows-only failures. Tests
for an injected filesystem compared native `path.join` output with hard-coded
POSIX strings, so the implementation was correct while the test double looked
empty. Normalize expected filesystem paths with `node:path` on every host.

Separately, `fs.rename(temp, destination)` replaces an existing destination on
POSIX but fails with `EEXIST` or `EPERM` on Windows. For a disposable cache,
serialize same-process writes per destination, remove the previous complete
entry on that Windows-only error, then rename the new complete entry. Document
the brief cache-miss window: the fallback prevents partial reads but is not an
atomic cross-process replacement.

Publication fault hooks receive canonical native paths. Windows canonicalization
adds the extended-path prefix, so a hook comparing that path with an ordinary
requested path never ran. Compare the intended component or normalized identity
and require the fault result explicitly; otherwise a passing operation does not
exercise rollback at all.
