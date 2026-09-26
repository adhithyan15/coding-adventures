---
category: Repo policy / workflow reminders
---

# Quote short PowerShell probes without multiline escapes and validate lesson categories first

A PowerShell `py -c` probe embedded literal `\n` escape sequences in a command
that Python parsed as source characters, so the probe failed before producing
the desired base-128 value. The first lesson-recording retry also guessed an
unsupported category. Keep inline probes single-line or use a native shell
expression, and check the repository's known lesson categories before passing
`--category`.
