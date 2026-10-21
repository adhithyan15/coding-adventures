---
category: Repo policy / workflow reminders
---

# Run lessons.py from the repository root instead of guessing a package-relative path

Invoking the lesson validator from a deeply nested package with a guessed
`../../..` path failed because the package depth was miscounted. Run
`python3 code/scripts/lessons.py validate` from the repository root, or resolve
the script path before changing directories, rather than embedding fragile
relative traversal in a validation chain.
