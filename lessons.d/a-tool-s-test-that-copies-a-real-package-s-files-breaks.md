---
category: Testing & coverage
---

# A tool's test that copies a real package's files breaks when that package's layout changes in another PR

`test_release_gate.py` built its temporary repository by copying files from
the real `rust/ct-compare` package, including `CHANGELOG.md`. The next PR in the
same series (LV-007b) converted that package to `CHANGELOG.d/`, and both
pipeline tests failed in CI's "Repo-wide metadata contracts" with
`FileNotFoundError`. Nothing in the gate was wrong.

**Fix:** copy only the files whose *real contents* the test is about (the
report, the evidence manifest, the source). Write every other file the code
under test merely needs to exist (here, a changelog that names the version)
inside the test.

**Next time:** when a test for a tool borrows a real package as a fixture,
list each borrowed file and ask whether the test needs its real bytes or just
its presence. Synthesize the second kind, so ordinary edits to that package
cannot fail the tool's tests.
