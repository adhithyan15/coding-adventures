## 2026-10-24 — BUILD_windows PowerShell and batch lines are rejected

- `validateWindowsBuildLines` rejects `BUILD_windows` lines that only work in
  PowerShell (`$env:X = ...`, `$ErrorActionPreference`, `Set-Location`,
  `Push-Location`/`Pop-Location`, `Write-Host`/`Write-Output`,
  `if (-not ...)`/`if ($...)`) or inside a batch file (`%~dp0`). Every
  non-comment line runs as its own `cmd /C` from the package directory, so
  such a line fails, but only when Windows CI happens to rebuild that package.
  The check reads `BUILD_windows` directly, so Linux and macOS runs catch it
  too. It sits beside `validateWindowsEnvAssignments` (#16437), which covers
  POSIX `VAR=value` assignments. Each line is checked command by command
  (split at the `&&`, `||`, `&` and `|` outside double quotes). A command
  that hands off to another shell (`powershell`, `pwsh`, `bash`, `sh`) is
  exempt, so the site BUILD files' `&& powershell -Command "..."` stays
  valid, but every other command is checked, including one after such a
  hand-off. It flags capitalised Verb-Noun cmdlets (`Remove-Item`), PowerShell
  variables braced or not (`$env:`, `${env:}`, `$LASTEXITCODE`, `$null`,
  `$global:`, `$script:`, `$PSScriptRoot`, `$true`/`$false`), a PowerShell
  `if (` or cmdlet after a `;` outside quotes (cmd has no `;` separator; a
  `$` or `exit` there is left alone because POSIX shell lines use them too),
  an unclosed `"`, a leading `[`, a `.ps1` run directly, and the batch
  self-references `%0`/`%~f0`/`%~dpnx0`. A leading UTF-8 byte-order
  mark is stripped first, and a symlinked, non-regular, oversized (over
  1 MiB) or unreadable BUILD_windows is reported instead of passing unseen.
  Harmless leftovers (`rem`, `@echo off`, `if errorlevel`) stay allowed.
