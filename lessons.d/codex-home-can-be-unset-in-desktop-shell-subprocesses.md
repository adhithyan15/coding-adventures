# CODEX_HOME can be unset in desktop shell subprocesses
category: Repo policy / workflow reminders

The LANG VM continuation read automation configuration through `$env:CODEX_HOME`
without verifying the variable existed. This desktop subprocess had it unset;
PowerShell emitted a file-read error while the command still exited successfully,
and parsing its output as JSON failed. Use the known configuration path when the
environment variable is absent, and set `ErrorActionPreference` to `Stop` before
reading and parsing configuration so an earlier read failure cannot masquerade
as successful structured output. Automation writes still belong to the app tool.
