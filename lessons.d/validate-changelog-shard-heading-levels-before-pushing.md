---
category: Repo policy / workflow reminders
---

# Validate changelog shard heading levels before pushing

PR #16864 passed focused Rust and generated C# checks but failed the repository doc-shard contract: three new changelog fragments used level-2 headings where their manifests require level 3. Correct the headings and run human-language-data check:doc-shards before pushing changelog fragments; package tests do not validate this repository-wide format.
