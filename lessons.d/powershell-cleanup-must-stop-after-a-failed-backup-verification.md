---
category: Repo policy / workflow reminders
---

# PowerShell cleanup must stop after a failed backup verification

I copied two generated, untracked Cargo lockfiles to task work before removing
them. A PowerShell expression built the destination path incorrectly, so both
`Copy-Item` operations failed as nonterminating errors. The script then reached
the later exact-path `Remove-Item` calls, and the original untracked bytes were
lost. Set `$ErrorActionPreference = 'Stop'` before the first operation, build
each destination explicitly, and compare source and backup hashes before any
cleanup. A failed backup must stop the whole sequence.
