---
category: Testing & coverage
---

# A test that reads another process's /proc entries passes as root and fails on non-root CI once that process is not dumpable

**What went wrong.** A P2.6d-3 test listed `/proc/<broker>/fd` to show the
confined broker held only its pipes. It passed locally, where the session runs
as root. But the broker clears its dumpable flag (`PR_SET_DUMPABLE, 0`), and
after that the kernel lets only a reader with `CAP_SYS_PTRACE` list its
`/proc/<pid>/fd` or open `/proc/<pid>/mem`, even a reader with the same uid.
On non-root CI the `read_dir(...).unwrap()` would have panicked. The security
review caught it before the push. `/proc/<pid>/status` stays readable either
way.

**The fix.** Branch on `geteuid()`: as root, assert the exact descriptor list;
otherwise assert the listing is refused (`PermissionDenied`), which is itself
the proof that the process is not dumpable.

**Do differently.** When a test inspects another process through `/proc`,
ask whether that process changes its own dumpability or credentials, and run
the test binary once unprivileged before pushing:
`chmod 1777 target/tmp; setpriv --reuid=65534 --regid=65534 --clear-groups <test-binary>`
(then restore `target/tmp` to 755). Tests that write under
`CARGO_TARGET_TMPDIR` need that directory writable for the unprivileged run.
