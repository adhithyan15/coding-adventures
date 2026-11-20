---
category: Repo policy / workflow reminders
---

# Run repository validation scripts from the repository root instead of counting package-depth parents

A validation command launched from `code/packages/rust` tried to reach
`code/scripts/lessons.py` by counting `..` segments and overshot the `code`
directory. The package checks passed, but the combined validation command
reported a false tooling failure and had to be rerun.

Run repository-level validation scripts with the worktree root as the working
directory and their repository-relative path. Keep Cargo commands in the Rust
workspace root. Splitting those working-directory responsibilities avoids
fragile path arithmetic when commands are later copied or parallelized.
