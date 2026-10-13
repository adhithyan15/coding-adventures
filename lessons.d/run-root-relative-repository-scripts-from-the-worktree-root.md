---
category: Repo policy / workflow reminders
---

# Run root-relative repository scripts from the worktree root

A chained post-merge validation ran from the human-language-data package
directory but invoked `python3 code/scripts/lessons.py validate`, a path written
relative to the repository root. Python looked for a nested package-local
`code/scripts` tree and failed before the remaining checks ran. Run repository
scripts from the worktree root, and run package commands in a separate command
with the package as its working directory; do not mix relative-path anchors in
one chain.
