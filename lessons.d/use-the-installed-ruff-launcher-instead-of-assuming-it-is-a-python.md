---
category: Python
---

# Use the installed Ruff launcher instead of assuming it is a Python module

The validation command invoked `python3 -m ruff`, but the active Homebrew
Python did not have Ruff installed as an importable module even though the
standalone `/opt/homebrew/bin/ruff` executable was available. Check
`command -v ruff` before choosing the invocation form, and use `ruff check`
for repository-level scripts unless a package-local virtual environment is the
documented owner of the lint dependency. When retitling a generated lesson,
also rename its file to the exact title slug before running lesson validation.
