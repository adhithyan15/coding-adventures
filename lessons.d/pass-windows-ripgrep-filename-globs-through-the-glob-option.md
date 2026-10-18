---
category: Cross-platform & Windows BUILD_windows
---

# Pass Windows ripgrep filename globs through the glob option

Passing `src/cv04*.rs` as a pathname to ripgrep in PowerShell returned native
exit 2 because the shell did not expand that filename glob. The enclosing
PowerShell invocation returned 1; neither result was a source or test failure.
Search the real directory with `rg -g 'cv04*.rs' PATTERN src` instead. The
corrected searches completed with exit 0. Keep filename filtering in ripgrep's
glob option instead of depending on shell expansion across platforms.
