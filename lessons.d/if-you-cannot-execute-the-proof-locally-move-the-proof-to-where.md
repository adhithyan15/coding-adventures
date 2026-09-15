# If you cannot execute the proof locally, move the proof to where it runs — do not infer it

The `book.pdf` symlink guard could not be exercised on the authoring box: native symlinks
need elevation or Developer Mode, `New-Item -ItemType SymbolicLink` fails with
"Administrator privilege required", and WSL is not installed. The tempting write-up is
"same idiom as the guard above, so it is fine" — an inferred pass, and the thing this
repository has been burned by.

**Better than a local demo: make it a test that runs where the capability exists.**
`tests/test-check-book-compile-guards.sh` and the lint's `SymlinkBanTests` create a real
symlink, run the real code, and assert the real refusal. They **skip with a printed
reason** on a filesystem that cannot make one, and run on the Linux CI runner — which is
also where the guard actually matters. A one-off local observation would have proved it
once; this proves it on every run.

Two details that make the skip honest rather than a dodge:
- **Probe, don't guess.** Try `ln -s` / `Path.symlink_to` and check `[ -L ]`; do not branch
  on `$OSTYPE` or `os.name`. Git Bash reports `msys` whether or not Developer Mode is on.
- **Scope the skip to the test that needs the capability**, not the whole file. The first
  draft `exit 0`-ed the entire suite before the `book.tex`/manifest cases, which need no
  symlink — so Windows verified nothing at all. Scoped, a Windows run still executes three
  real assertions.

**And mutation-test the new test before believing it.** Removing
`[ -f "$dir/book.tex" ] || continue` turned the suite red; restoring it turned it green. A
test that has only ever been observed passing has not been observed working — the same
principle as proving a gate's red path, applied to the gate's own tests.
