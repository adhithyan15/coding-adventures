---
category: Testing & coverage
---

# Never copy a coreutils binary under a new name in a test: ubuntu-26.04 coreutils is multi-call

**What went wrong.** On ubuntu-26.04 the coreutils programs are one multi-call
binary (uutils: `/usr/bin/sleep` resolves to `/usr/lib/cargo/bin/coreutils/sleep`)
that picks the applet from `argv[0]`. A test that copies `/bin/sleep` (or `true`,
`cat`, ...) to a new name such as `bin-sleep2` or `s` and runs it gets
`coreutils: unknown program 's'` and an immediate exit 1. It bit twice in Kiln:
the exec-head spike's strace run exited 1 on 26.04 only (run 45346779343), and
the D48 supervisor's lifetime test saw the "long-lived" binary exit at once
(`gone`) on 26.04, while other tests passed only because a poller won the race.

**Fix.** Keep a copied coreutils binary's base name (`dir/sleep`, not
`bin-sleep2`), or better, copy the test's own probe binary and give it an
explicit mode (`probe hold 1500`) so the behaviour does not depend on the
image's coreutils at all.

**Do differently.** Never rename a system utility in a test fixture. When a test
needs a binary with a particular lifetime or behaviour, use a probe the repo owns.
Run new sandbox and exec tests on both ubuntu-24.04 and 26.04 before trusting a
pass on one.
