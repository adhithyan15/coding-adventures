---
category: Testing & coverage
---

# An expect-a-crash test cannot tell a fail-closed runtime check from the wild read it replaced

`dynval_runtime.c`'s `__dyn_pair_first` read `*(word & ~7)` without checking the
tag, so Twig's `(car 5)` read address 40 on LLVM and native x86-64. The fix
checks the pair tag and aborts. The first end-to-end test was a `lang_matrix`
row with `Expect::Trap`, which passes when the process exits unsuccessfully.
It passed with the check removed: the read of address 40 segfaults, and a
segfault is also an unsuccessful exit.

**Fix:** the test also captures the tagged-word columns' stderr and requires
the runtime's own refusal text (`… is not a pair`). Only the check prints that.
In the in-process golden test, an `abort()` can't be observed directly. The
test re-runs its own binary (`std::env::current_exe()`, `--exact <name>`, plus
an environment variable that tells the child which call to make) and asserts
on the child's status and stderr.

**Do differently:** when a fix turns a crash into a controlled failure, make
the test check what only the controlled path produces (a message, a trap kind,
an exit code), not just "it failed". Then falsify it: disable the check and
watch the test fail.
