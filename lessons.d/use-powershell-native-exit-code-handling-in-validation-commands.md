# Use PowerShell-native exit-code handling in validation commands

POSIX `|| exit 0` is not valid PowerShell control flow, and a diagnostic command
also used a repository-relative path after the working directory had already
been changed to the package. In PowerShell, inspect `$LASTEXITCODE` explicitly
and keep paths relative to the declared command working directory.

For a command that is expected to fail, redirecting stderr is not always
enough: PowerShell can promote native stderr into a terminating error before
the script checks `$LASTEXITCODE`. When the test must inspect both streams and
the real process status, launch it with `System.Diagnostics.Process`, drain
stdout and stderr asynchronously, wait for exit, and assert the captured result.
