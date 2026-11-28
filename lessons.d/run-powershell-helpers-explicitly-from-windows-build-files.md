---
category: Cross-platform & Windows BUILD_windows
---

# Run PowerShell helpers explicitly from Windows BUILD files

The build tool executes each `BUILD_windows` line as its own `cmd /C` command.
Writing a script path such as `./verify-consumer.ps1` does not select PowerShell;
`cmd` may invoke the file association or reject the command. Use an explicit
`powershell -NoProfile -File helper.ps1` command. Keep the helper compatible with
the Windows PowerShell version installed on supported runners. Here the exact
same consumer scripts already ran through PowerShell 5-compatible cmdlets; the
BUILD lines were the broken part.
