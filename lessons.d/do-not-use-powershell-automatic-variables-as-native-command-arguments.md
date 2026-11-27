---
category: Cross-platform & Windows BUILD_windows
---

# Do not use PowerShell automatic variables as native command arguments

I named a PowerShell helper parameter `$args`, an automatic variable, and the
native compiler received no source files. The wrapper failed before compiling,
even though the command looked populated at the call site. Use a distinct name
such as `$ArgumentList`, record the executable and argument list that are
actually invoked, and capture `$LASTEXITCODE` immediately after each native
command. A later successful command must not hide an earlier failure.
