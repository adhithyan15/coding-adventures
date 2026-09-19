---
category: Repo policy / workflow reminders
---

# Run repository helpers from the worktree root instead of guessing package-relative depth

Deep package directories make hand-counted `../../..` paths easy to overshoot.
Run repository-wide helpers such as `code/scripts/lessons.py` from the worktree
root, or use a previously resolved absolute path, rather than guessing the
number of parent segments.
