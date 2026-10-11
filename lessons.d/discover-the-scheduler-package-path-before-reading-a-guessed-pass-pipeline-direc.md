---
category: Repo policy / workflow reminders
---

# Discover the scheduler package path before reading a guessed pass-pipeline directory

The scheduler lives in `closure-pass-pipeline`, not the guessed `pass-pipeline`
directory. Use `rg --files` to discover a package before reading it. A lesson
creation attempt also used an invented category; use the categories printed by
the lessons CLI. Submit a patch only once its hunk contains an actual change;
read the current formatted lines before patching a statement rustfmt expanded.
PowerShell does not expand an `rg` directory argument such as `closure-pass-*`;
search the known parent with `-g '**/closure-pass-*/**/*.rs'` instead.

A later validation-discovery command also guessed a root `scripts` directory and `code/programs/rust/Cargo.toml`; neither exists. Discover known `code/scripts` and actual Cargo manifests first.
