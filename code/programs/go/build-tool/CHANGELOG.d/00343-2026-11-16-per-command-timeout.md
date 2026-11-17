## 2026-11-16 — Per-command timeout (`-command-timeout`)

- Every BUILD command now runs under a timeout, default 90 minutes; `0`
  disables it and a negative value is rejected. One hung `rust/lzw` line on
  Windows CI had idled a job for 4h47m until GitHub's 300-minute step limit
  killed it, and because package output is buffered the log never said which
  command hung.
- On timeout the whole process tree is killed (Unix: own process group,
  `kill(-pgid)`; Windows: `taskkill /T /F /PID`), and `cmd.WaitDelay` bounds the
  wait for the output pipes so a grandchild holding stdout cannot block `Run`.
- The package fails with return code 124. Its stderr names the package, the
  1-based command index and text, the limit and the elapsed time, after all
  output buffered so far. A command that exits cleanly but leaves a background
  process holding its output also fails, with an explanatory message, instead
  of hanging.
- No per-package override: BUILD files carry no execution settings today.
- `code/specs/12-build-system.md` documents the flag and marks it OPTIONAL for
  non-Go implementations; no other build tool or conformance fixture changes.
