# Windows cmd subprocess executable paths need native backslashes

A Python validation helper passed `C:/Windows/System32/cmd.exe` as argv[0]
to `subprocess.run`. Every actual BUILD front immediately reported "The syntax
of the command is incorrect", including the trivial `npm --version` probe.
The same arguments succeeded when argv[0] used native backslashes. PowerShell
had normalized the executable path, which initially hid the difference.

Normalize the executable path with `os.path.normpath` before launching cmd from
Python. Reproduce shell setup failures with a trivial command before diagnosing
package code. Preserve the failed harness receipt, then rerun each real BUILD
line in its own shell with the package as its working directory.

A shortened PATH must also retain executables required by downstream tests,
such as Python or the installed Visual C++ development environment. Missing
tools and unavailable symlink privilege remain setup failures, not passing tests.

---
category: Cross-platform & Windows BUILD_windows
---
