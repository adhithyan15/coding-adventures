# Prefer PowerShell filtering over nested jq quoting in PowerShell commands

A parity-loop pre-push check embedded a jq expression containing spaces and
quoted regular expressions in one PowerShell command. PowerShell split the
expression before `gh` received it, so `gh pr list` rejected part of the jq
program as an unknown argument. The following independent `git push` still ran,
which made the noisy read-only failure easy to miss.

Lessons:

1. On PowerShell, request JSON from `gh`, pipe it through `ConvertFrom-Json`,
   and filter with `Where-Object` when the jq program needs nested quoting.
2. Keep a required precondition check separate from the external write it is
   meant to guard. A failed check must prevent the push rather than merely share
   a command invocation with it.
