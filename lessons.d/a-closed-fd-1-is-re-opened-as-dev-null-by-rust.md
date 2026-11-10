---
category: Rust
---

# A closed fd 1 is re-opened as /dev/null by Rust std before main, so an in-main check cannot see it

LH-003c: a test-bootstrap harness whose stdout was closed or read-only printed
nothing and exited 0, because Rust std treats `EBADF` on stdout as success. The
first fix, `fcntl(1, F_GETFL)` at the top of `main`, caught the read-only case
but the "closed fd 1 via `pre_exec`" test still exited 0: std's start-up
(`sanitize_standard_fds`) opens `/dev/null` over any closed fd 0/1/2 *before*
`main`, so by then a closed stdout is a perfectly writable `/dev/null`.

Fix: on Linux sample fd 1 from a `.init_array` entry (runs before std's start-up,
stores the flags in an atomic) and consult it from the check. Other Unix and
Windows cannot see this case and it is documented as a known limit (TF01 §8.3
item 9).

Do differently: when a defence is meant to catch a state set up by the process
environment, write the failing child-process test first (closed fd via
`pre_exec`, read-only fd via `Stdio::from(File::open)`). It showed within one run
that "check at the start of main" was too late.
