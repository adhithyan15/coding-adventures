---
category: Repo policy / workflow reminders
---

# Patch evolving specs against exact current line wrapping

An OAuth spec patch quoted the right prose but assumed a different Markdown
line wrap, so exact-context verification rejected it without changing the
file. Read the current focused region immediately before patching an evolving
spec and anchor the edit to its exact present lines.
