---
category: Repo policy / workflow reminders
---

# Use jq or a checked script instead of quote-dense inline Python for JSON report inspection

An inline `python -c` inspection embedded dictionary-key quotes inside shell
quotes and f-strings. The shell removed the key quotes, so Python evaluated
`r[commits_touching]` as a variable lookup and failed after the report itself
had generated successfully. For structured output inspection, use `jq` with a
single checked filter or put non-trivial Python in a reviewed script; do not
stack shell, Python, f-string, and dictionary quoting in one command.
