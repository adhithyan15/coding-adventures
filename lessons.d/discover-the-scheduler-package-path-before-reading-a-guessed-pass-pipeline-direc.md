---
category: Repo policy / workflow reminders
---

# Discover the scheduler package path before reading a guessed pass-pipeline directory

The scheduler lives in `closure-pass-pipeline`, not the guessed `pass-pipeline`
directory. Use `rg --files` to discover a package before reading it. A lesson
creation attempt also used an invented category; use the categories printed by
the lessons CLI. Submit a patch only once its hunk contains an actual change;
read the current formatted lines before patching a statement rustfmt expanded.
