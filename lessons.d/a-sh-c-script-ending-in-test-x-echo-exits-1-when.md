---
category: Testing & coverage
---

# A sh -c script ending in 'test X && echo' exits 1 when X is false, failing the step that runs it

**What went wrong.** The new Kiln sandbox test `TestSandboxSealedRoot` (PR #19761, D43) failed on both kiln-sandbox legs, though the sandbox behaved correctly. Its `sh -c` script ended with `test -e /run/systemd && echo run-systemd-present`. The expected case is that `/run/systemd` is ABSENT, so `test` returned 1. A `&&` list's status is that of its last executed command, so the script exited 1, and the test's `r.code != 0` guard failed before any real assertion ran. The failure printed only `exit 1:` with empty stderr, because the guard did not print stdout.

**Fix.** Use `if test …; then echo …; fi`, whose status is 0 when the condition is false. The non-zero-exit failure now also prints stdout.

**Do differently.**
- A probe script whose exit status is checked must never end in `cond && action` where `cond` may legitimately be false. Use `if`, or end with `true`.
- When a sandbox or subprocess test fails on its exit status, print stdout as well as stderr, so the first CI run shows what happened. This test could not run locally on macOS, so CI was its first real run.
