---
category: Testing & coverage
---

# Limit fixture substitutions to intended tokens and preserve file endings

The octal stringize fixture was derived from the hex case. A broad `H` to `O`
replacement also changed the unrelated `BOTH` macro to `BOTO`; it still worked
but obscured the intended mixed raw/plain test. Inspection restored `BOTH`.
A PowerShell `Set-Content` rewrite then added a blank line at the Rust file's
end, which `git diff --check` reported. Normalize the edited file to one final
newline and use precise token replacements rather than global character edits.
