---
category: Cross-platform & Windows BUILD_windows
---

# Use body files for `gh pr` text containing Markdown backticks

Inline backticks in `--body "..."` get evaluated by zsh as command substitution. Write to a tempfile with single-quoted heredoc and pass via `--body-file`.
