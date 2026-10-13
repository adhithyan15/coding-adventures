---
category: Repo policy / workflow reminders
---

# Stage new files before relying on git diff --check to catch their whitespace

`git diff --check` reported clean while every source file was still untracked,
so it did not inspect two Markdown trailing-space hard breaks. Run an explicit
whitespace scan while drafting, then stage all intended new files before the
final `git diff --cached --check`; an unstaged diff check is not evidence about
untracked content.
