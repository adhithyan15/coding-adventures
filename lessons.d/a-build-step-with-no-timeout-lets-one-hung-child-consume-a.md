---
category: CI & GitHub Actions
---

# A build step with no timeout lets one hung child consume a whole CI job and hide which command hung

One `rust/lzw` BUILD line hung on Windows CI and the job idled for 4h47m until
GitHub's 300-minute step limit killed it. The Go build tool ran each line with
`cmd.Run()` and no deadline, and it buffers a package's output until the package
ends, so the log never showed which command hung. `Run` also waits for pipe EOF,
so even a killed shell is not enough while a grandchild holds the pipe.

Fix: every BUILD command now runs under `exec.CommandContext` with a Cancel hook
that kills the whole process tree (process group on Unix, `taskkill /T /F` on
Windows) plus `WaitDelay` to bound the pipe wait. A timeout fails the package and
names the package, command index and text. Default 90m (`-command-timeout`).

Do differently: any code that spawns a child in CI needs a deadline well under
the enclosing job limit, a kill that reaches descendants, and a failure message
that names what was running. Do not rely on the job-level timeout; it only tells
you that something hung, not what.
