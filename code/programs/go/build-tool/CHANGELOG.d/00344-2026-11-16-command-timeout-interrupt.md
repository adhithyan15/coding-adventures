## 2026-11-16 — Interrupt handling for BUILD command process groups

- SIGINT/SIGTERM now cancel a root context (`executor.RootContext`) that every
  command's context derives from, killing each running command's process tree.
  Needed because `Setpgid` (added with `-command-timeout`) detaches children
  from the terminal's Ctrl-C. The run exits 130 with an "Interrupted" notice and
  affected packages fail with return code 130; never success.
- A command that exits cleanly but leaves a descendant holding its output
  (`ErrWaitDelay`) now also has that leftover process group killed.
- Documented the Windows `taskkill`-missing fallback (shell-only kill).
