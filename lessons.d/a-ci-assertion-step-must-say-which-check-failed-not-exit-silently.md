---
category: CI & GitHub Actions
---

# A CI assertion step must say which check failed, not exit silently from grep -q

**What went wrong.** The UI89 Android step built the APK (`BUILD SUCCESSFUL`)
and then ran a column of bare `grep -q` and `test` checks under
`set -euo pipefail` (badging lines from `aapt2 dump badging`, dex class
names, the JNA library path). One of them failed, and the log ended in
`##[error]Process completed with exit code 1.` right after the Gradle output,
with nothing saying which. One CI round was spent learning only that "a check
failed". The likely culprit is also an assumption about format: `aapt` prints
`sdkVersion:'26'`, while newer `aapt2` prints `minSdkVersion:'26'`.

**Fix.** Each expectation goes through a helper that prints
`::error::` naming the expectation and dumps what it was checked against (the
badging file, the dex directory, the lib listing) before exiting. The SDK
check accepts both spellings.

**Do differently.** In a CI step, never end a verification with a bare
`grep -q` or `test` when the thing it inspects is not already in the log. Wrap
it so failure prints what was expected and what was found; this costs three
lines and saves a whole CI round. And when asserting on a tool's text output
that cannot be run locally, match the tolerant form or print the output once.
