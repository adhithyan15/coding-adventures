# Git Bash on the Windows runner has no `zip`

The Compose Desktop release job built the Windows app perfectly, then died in
the next step:

```
zip: command not found
##[error]Process completed with exit code 127
```

`zip` is on the Linux and macOS runners and not on the Windows one, so a step
written and tested against two platforms lost the third at the very last
moment — after the expensive part had already succeeded. The same shape as
`choco install poppler`, which does not exist on `windows-2025` and broke the
PDF oracle build: *assuming a Unix utility is a runner utility*.

Worse, the job is not a required check, so the PR merged green-looking while
one platform's payload was never produced. `main` then declared a Windows
Compose artifact that nothing could build, and the publish job asserts the
files on disk equal the declared set — so the next release would have failed
at the very end, after every build had run.

The fix is not a per-platform branch in the workflow. It is to do the work in
the one tool that is already installed identically on all three runners —
here Python, which the job sets up anyway — so there is a single code path
that can be tested locally. `zipfile.ZipFile.write` also carries each file's
mode into `external_attr`, so the launcher stays executable; an archiver that
loses that extracts to an app that cannot be launched, and every structural
check still passes.

Two follow-ons worth stating separately:

- **A verification that silently skips is not a verification.** The engine
  symbol check in `build-native.sh` runs `nm`, which is absent on Windows, so
  it fell through to `SHIPPED="unknown"` and passed. Windows was the one
  platform shipping unverified. Checks that cannot run everywhere belong where
  they can — the presence-and-non-empty assertion moved into the archiving
  step, which is now identical on all three.
- **Ask what a green PR did not run.** A non-required job that fails is a
  merged PR with a hole in it. Look at the job list, not the merge button.
