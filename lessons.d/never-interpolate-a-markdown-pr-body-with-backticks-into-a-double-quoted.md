---
category: Repo policy / workflow reminders
---

# never interpolate a Markdown PR body with backticks into a double-quoted shell argument

I passed a Markdown pull-request body containing backticked commands through a
double-quoted shell argument. The shell executed those command substitutions,
produced unrelated npm and glob errors, and created the PR with a damaged body.
For GitHub prose containing backticks, pass the entire body as a safely escaped
single-quoted argument (or a deliberately created body file) and then inspect
the remote body. Never assume JSON string quoting also protects shell syntax.
