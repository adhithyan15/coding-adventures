# In zsh, `status` and `path` are special parameters, not scratch variables

A merge-verification script assigned command output to `status`; zsh stopped
the script with `read-only variable: status` because that parameter exposes the
last exit status. A later inspection loop used `path` for a filename; zsh maps
that array parameter to `PATH`, so the assignment removed `git`, `rg`, and
`head` from command lookup for the rest of the loop.

Use task-specific names such as `wt_status` and `file_path` in zsh scripts.
Avoid generic shell names until `typeset -p <name>` has shown they are ordinary
parameters. A verification script that aborts before printing its evidence has
not performed the verification, even when an earlier command in the script
succeeded.
