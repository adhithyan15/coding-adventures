---
category: Repo policy / workflow reminders
---

# PowerShell inline Python with nested f-string quoting is fragile; prefer native PowerShell for graph summaries

Embedding a multiline Python graph walk and a nested f-string inside a
PowerShell `python -c` argument produced an unterminated string before the
validator ran. For small JSON state summaries, use `ConvertFrom-Json` and
native PowerShell aggregation. If Python is genuinely clearer, place the code
in an existing checked-in helper or pass a carefully bounded one-line program;
do not stack PowerShell, Python, and f-string escape layers in one command.
