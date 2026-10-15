---
category: Workspace & package metadata
---

# A changelog shard begins with its level-two section heading

The actual doc-shard checker rejected two new Rust changelog shards because
they began with level-one headings. A shard is a section of the assembled
document: its preamble belongs in _meta.md. Match existing shards with a level-
two Unreleased heading, preserve the failed validator log, then rerun the
checker against the corrected files. This prose correction does not change
the concurrently validated Rust source, tests, manifests or build inputs.
