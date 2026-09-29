---
category: CI & GitHub Actions
---

# A change to a crate everything depends on makes one unsharded PR runner rebuild the repo

Only `main` shards its build (`-shard-count 15`). A pull request gets one
runner per OS, and it builds every package the diff affects. Changing
`code/packages/rust/parser` (#16054) put ~750 packages on each leg. On Windows
that took ~140 minutes, then ~150 on the next run, and the step's 150-minute
`timeout-minutes` killed it while `rust/lang-aot`, which alone takes ~19 minutes
there, was still running normally. No test failed. The log ended at
`5173/5247 Building: rust/lang-aot` and then `has timed out after 150 minutes`.

The same wide rebuild also exposes `BUILD_windows` files that never run
otherwise. Here, `sql-codegen` and `sql-vm` were written in PowerShell, but the
build tool runs each line through `cmd /C`, where `$env:...` is a syntax error.

What to do:
- Before pushing a change to a widely shared crate, expect the Windows leg to
  take hours. Treat a timeout at the step ceiling as a capacity limit, not a
  hang: check how far the progress lines got, and whether the last package is
  one that is always slow.
- The step ceiling is now 300 minutes; the job itself stops at 360.
- A `BUILD_windows` is plain `cmd` lines, one command each. There is no
  PowerShell, no `%~dp0`, and no `if errorlevel`: a failing line already fails
  the package.
