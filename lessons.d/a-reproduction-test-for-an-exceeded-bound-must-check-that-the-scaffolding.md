---
category: Testing & coverage
---

# A reproduction test for an exceeded bound must check that the scaffolding it adds does not raise the bound

Issue #18432 reported that `iir-linker`'s rename counter panicked: its
`assert!` allowed *total functions + 1* `$N` suffixes, and module `a`
exporting `f` as `b::c$0` … `b::c$3` next to a module `b` with a private `c`
used them all up. The first "failing" test copied that example but gave `b` a
second function (`run_b`, which calls `c`) to check that calls follow the
rename. That extra function raised the bound from 3 to 4, so `$4` was allowed
and the test **passed on the unfixed linker** — it reproduced nothing.

The fix was to claim more names than any plausible bound (`$0` … `$9`) and to
run the test against the unfixed code, where it now panics at the `assert!`.

When a bug is "a counter exceeds a limit computed from the input", work out
the limit for the *exact* input the test builds, including helpers added only
to make assertions, and leave a wide margin. More generally, a test written to
fail first has to be seen failing: watch it fail before you write the fix, and
for a driver-level test, run the new test against the old source as well.
