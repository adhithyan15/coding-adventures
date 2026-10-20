---
category: Repo policy / workflow reminders
---

# Repository-root scripts must be run from the repository root

Running `python3 code/scripts/lessons.py validate` from a nested package made
Python look for a nonexistent package-local `code/scripts` tree. The failure
said nothing about the lesson files.

Run repository-relative maintenance commands with the repository root as the
working directory, or pass an absolute path when the command deliberately runs
inside a package. Keep package tests in their package working directory and
repo policy checks at the root.
