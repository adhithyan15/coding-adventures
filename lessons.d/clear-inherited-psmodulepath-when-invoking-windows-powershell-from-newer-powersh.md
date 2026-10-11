---
category: Cross-platform & Windows BUILD_windows
---

# Clear inherited PSModulePath when invoking Windows PowerShell from newer PowerShell

The Windows ACL regression helpers launched `powershell.exe` from Cargo running
under newer PowerShell. They inherited its `PSModulePath`, so Windows PowerShell
found an incompatible `Microsoft.PowerShell.Security` module and failed before
reading any ACL. Those failures did not reproduce the publication defect.
Remove `PSModulePath` from the child environment so Windows PowerShell selects
its own built-in modules. The corrected helpers then reached the ACL assertions:
staging inheritance protection was false and committed private policies broadened.
Keep the original harness-error logs separate from meaningful before-repair evidence.
