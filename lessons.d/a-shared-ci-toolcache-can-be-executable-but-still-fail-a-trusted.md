---
category: Security boundaries
---

# A shared CI toolcache can be executable but still fail a trusted-runtime writer policy

The Windows hosted runner's Python distribution was readable and executable,
but its root DACL granted `S-1-5-11` (Authenticated Users) dangerous write
authority. Forme's runtime-root verifier correctly rejected it before launch.
Treating a preinstalled tool as trusted merely because the CI provider supplied
it would have widened the production policy to every authenticated local
account and invalidated the point of checking writers.

For an acceptance test that needs a writable-host runtime, first assert that
the production verifier rejects the hosted root and names the broad principal.
Then copy the quiescent, runner-controlled distribution into a private tree
under the current user's profile. Preserve rather than follow symlinks as a
second defense, ignore observed links and junctions, and verify the finished
destination before launch. Files created there inherit the trusted parent ACL.
Reuse that fixture for the suite, remove it after all launcher handles close,
and leave the shared toolcache untouched. This proves both sides of the
production boundary without adding a CI-only principal exception, mutating a
tool other jobs use, or claiming that availability implies trust.
