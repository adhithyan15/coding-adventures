---
category: Repo policy / workflow reminders
---

# Run repository scripts from the repository root instead of guessing package-relative depth

After a package-local test passed, the command guessed a four-level relative
path to `code/scripts/lessons.py`; the package is nested differently, so Python
could not find the script. Keep npm commands package-local, but run repository
scripts from the repository root with their canonical `code/scripts/...` path.
Conversely, do not assume the repository root owns npm scripts: locate the
defining `package.json` first and run `check:shards`, `check:doc-shards`, and
`check:assessment-artifacts` from `code/packages/typescript/human-language-data`.
