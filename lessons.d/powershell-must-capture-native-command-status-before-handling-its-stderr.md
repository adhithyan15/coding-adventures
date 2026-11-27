---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell must capture native command status before handling its stderr

With `$ErrorActionPreference = 'Stop'`, native stderr can be surfaced as a
PowerShell error record. If the script treats that stream before recording the
native process result, control flow may stop before `$LASTEXITCODE` is saved.
This is especially risky when a diagnostic deliberately writes stderr on both
success and failure paths. For an expected diagnostic, scope a temporary
`Continue` preference around only that invocation, redirect stderr to a
captured stream or file, and save `$LASTEXITCODE` immediately after the
process returns. Restore the prior preference in `finally`; then inspect the
captured stderr and actual exit code together. Never infer or substitute an
exit code from the text, and do not weaken error handling for later commands.
