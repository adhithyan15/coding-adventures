---
category: Repo policy / workflow reminders
---

# Feed Markdown PR bodies through stdin instead of shell double quotes

Embedding a Markdown PR body in a double-quoted shell command still executes
backtick-delimited validation commands, even when the outer command was built
from a JSON string. That can both run unintended commands and silently remove
the validation text from the created PR. Send multiline Markdown to
`gh pr create` or `gh pr edit` with `--body-file -` and standard input, then
read the saved body back from GitHub before continuing.
