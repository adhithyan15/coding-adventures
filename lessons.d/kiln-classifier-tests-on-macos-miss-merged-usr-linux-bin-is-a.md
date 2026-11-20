---
category: Testing & coverage
---

# Kiln classifier tests on macOS miss merged-usr: Linux /bin is a symlink to usr/bin

**What went wrong.** Kiln D43 (PR #19761) made paths outside the sealed root's exists-set "sandbox-absent". Every Kiln test passed locally on macOS. Then two `internal/record` tests (`TestEpilogue…`) failed in the Linux `build` job with an extra `trace-incomplete:unstable-path`.

The fixture declares only `/bin` and `/sbin` as host roots. On Ubuntu, `/bin` is a symlink to `usr/bin` (merged /usr), so the classifier's host-side walk of `/bin/sh` landed on `/usr/bin/sh`. That path was outside the exists-set, so an exec the kernel had performed was judged against an "absent" path. On macOS `/bin` is a real directory, so the walk never left the declared root.

**Fix.** The exists-set also admits the host-side target of a declared root that is itself a symlink, plus the target's ancestors (`declaredRootLinks`). This keeps the rule's invariant that a gap in the set never changes a verdict. A host-independent regression test fakes the merged-/usr layout.

**Do differently.**
- Any classifier rule that depends on which host paths exist must have a test that fakes the merged-/usr layout (`/bin -> usr/bin`, `/lib -> usr/lib`, `/lib64 -> usr/lib64`, `/sbin -> usr/sbin`). The Mac's real `/bin` cannot surface this class of bug.
- The kiln-sandbox legs run only `-run Sandbox`. The Linux `build` job is the only place the unix `internal/record` tests run on Linux, so wait for it before calling a Kiln PR green.
