---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell double-quoted Markdown turns backtick t into a tab

PowerShell treats the backtick as its escape character inside double-quoted
strings. Building a Markdown issue line as a double-quoted string therefore
turned the opening characters of `` `task-app-v0.5.4` `` into a tab plus
`ask-app-v0.5.4`, even though the GitHub API write itself succeeded.

Use single-quoted strings for literal Markdown containing code spans, or double
the PowerShell backtick when interpolation is genuinely required. After every
remote body edit, read the body back and reject ASCII control characters other
than line endings; a successful API response does not prove the text survived
shell parsing.
