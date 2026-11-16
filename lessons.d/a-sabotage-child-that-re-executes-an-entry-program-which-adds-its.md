---
category: Testing & coverage
---

# A sabotage child that re-executes an entry program which adds its own flags receives them twice and every control reads as not caught

The Go lane's `sabotage` command re-runs the running binary once per negative control with the runner's full
flag list (`run --repo-root ... --corpus ... --binding ... --package-dir ...`). The leaf package's entry
program (`cmd/warrant-evidence`) is a thin wrapper that *adds* those same flags to whatever command it gets.
The child therefore received every flag twice, the runner refused it with a usage error (exit 2), and all
eight controls printed `not ok sabotage ... (exit 2)`. The runner's own tests did not see it: they re-execute
the test binary, which has no wrapper.

The fix is that the wrapper forwards a command that already names `--repo-root` unchanged, and the entry
program has a test for it. The BUILD also runs each fault by hand (`WARRANT_EVIDENCE_FAULT=... | grep -q
'^not ok '`, `set VAR=...&&` and `findstr` on Windows), which is what exposed the difference.

Do differently: when a command re-executes "the program", test it through the real entry program, not only
through the library's test binary, and read the exit status the child reports, not just that it failed.
