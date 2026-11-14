---
category: Cross-platform & Windows BUILD_windows
---

# Killing a service's shell leaves the real server running: stop the whole process tree, and test it with a grandchild

`benchmark-tool` starts a TCP service subject with `cmd /C go run server.go` (or `sh -c`). `Stop`
called `Process.Kill()`, which kills only that shell. The compiled server, a grandchild, kept
running. On Unix nobody noticed (an orphan process, and deleting a file that is still open is
allowed). On Windows the orphan held `service.log` open and `t.TempDir()`'s cleanup failed with
"The process cannot access the file because it is being used by another process", which is how
the first scheduled cross-OS build surfaced a real orphan-process bug as a flaky-looking test
failure.

Fix and rules for anything that supervises a child it started through a shell:

- Unix: start the child as its own process-group leader (`SysProcAttr{Setpgid: true}`) and kill
  the group (`syscall.Kill(-pid, SIGKILL)`, ignoring ESRCH).
- Windows: assign the child to a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` right after
  `Start`, and `TerminateJobObject` on stop. Keep a fallback to `Process.Kill` if the job cannot be
  set up. Check the Go struct layout of `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` (144 bytes on x64)
  in a test, because a wrong size silently fails to set the flag. Wine's `taskkill` has no `/T`,
  so a job object is also the option you can actually validate off Windows.
- Also kill the tree when the shell has already exited: it can leave its child running.
- Test it with a grandchild, not a direct child: a long-lived `sleep` started by the shell on Unix
  (check its pid is gone, or a zombie), a `ping -n 600` on Windows (check the log is deletable).
  Confirm the Unix test fails when the fix is removed. And never pipe a mutated Windows test
  through `| grep` under a timeout: the orphan keeps the pipe open and blocks the shell.
