---
category: Python
---

# A scripted string replace on a test file no-ops silently when the indentation differs, so assert the old text is present

`s.replace(old, new)` in a throwaway Python script returns the string unchanged when `old` does not match, so an edit that
guessed the indentation (12 spaces for an 8-space line) did nothing and the failure surfaced only in the next full test run,
about twenty minutes later. When a script edits a test file, write `assert old in s` before the replace, or use the Edit tool,
which refuses a non-matching string. Run the one test the edit was meant to fix straight after it.
