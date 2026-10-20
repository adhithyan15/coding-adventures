---
category: Repo policy / workflow reminders
---

# Changelog shards must start at the configured heading level

The metadata CI rejected a new XAML changelog entry beginning with a level-one heading. This changelog is sharded at level three; copy the heading level of neighboring shards and run readDocShards validation before pushing. The document preamble belongs in _meta.md.
