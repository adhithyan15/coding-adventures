---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell does not expand Bash brace lists into file arguments

During the native stack-argument audit, a read command used Bash-style
`{package-a,package-b}` paths in PowerShell. PowerShell rejected the whole
command before any reads or validation ran. Retry with explicit file arguments
or a PowerShell array; never count a parser failure as test evidence.
