# Use PowerShell-native exit-code handling in validation commands

POSIX `|| exit 0` is not valid PowerShell control flow, and a diagnostic command
also used a repository-relative path after the working directory had already
been changed to the package. In PowerShell, inspect `$LASTEXITCODE` explicitly
and keep paths relative to the declared command working directory.
