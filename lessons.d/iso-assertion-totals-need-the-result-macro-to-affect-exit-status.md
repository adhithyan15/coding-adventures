---
category: Testing & coverage
---

# ISO assertion totals need the result macro to affect exit status

`ISO_CHECK` records failures in the harness, but the executable can still exit
successfully if `main()` returns a literal zero. Every ISO test executable must
finish with `return ISO_TEST_RESULT();` so accumulated failures produce a
nonzero process status. Verify that gate with a disposable test containing one
deliberately false assertion before relying on a green package build.
