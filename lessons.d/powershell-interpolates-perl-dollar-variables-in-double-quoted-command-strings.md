---
category: Perl
---

# PowerShell interpolates Perl dollar variables in double-quoted command strings

When a Perl one-liner is passed through PowerShell, `$name` inside a
double-quoted PowerShell string is parsed as PowerShell interpolation before
Perl sees it. Use a single-quoted PowerShell literal where possible, or put the
probe in a small script file, then inspect the exact command's output before
relying on it.
