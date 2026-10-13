---
category: Cross-platform & Windows BUILD_windows
---

# Use rg -g for file globs in PowerShell

Passing `code/packages/rust/*/Cargo.toml` as an rg path failed because PowerShell
did not expand the wildcard for the external command. Search the directory with
`rg -g Cargo.toml PATTERN code/packages/rust` instead. Keep file filtering inside
rg so the same search works across shells and Windows paths.
