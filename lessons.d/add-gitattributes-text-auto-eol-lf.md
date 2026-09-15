---
category: Cross-platform & Windows BUILD_windows
---

# Add `.gitattributes` `* text=auto eol=lf`

to force LF line endings everywhere. Otherwise Elixir heredoc tests, Python doctests, Ruby tests fail on Windows checkouts because `\r\n` ≠ `\n`.
