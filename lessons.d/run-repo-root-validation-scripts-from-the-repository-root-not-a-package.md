---
category: Testing & coverage
---

# Run repo-root validation scripts from the repository root, not a package directory

I combined the package-local Vitest command with the repository-level lessons
validator while the shell was inside the package directory. The lessons command
then looked for a duplicated `code/packages/.../code/scripts/lessons.py` path and
failed before the tests ran. My first patch also assumed the lesson generator's
exact heading and TODO wording instead of reading the new shard, so that patch
failed too. Run repository scripts from the worktree root, run package binaries
separately from their package directory, and inspect generated lesson text before
patching it.
