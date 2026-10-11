---
category: Workspace & package metadata
---

# Sharded changelog filenames include the heading digest

Repository document shards use a strict filename identity, not only a sequence
number and readable slug. A changelog shard must end with the first eight hex
characters of the SHA-256 digest of its exact heading line. Before committing a
new shard, derive the filename with the document-shard helper (or compute that
heading digest), then run the repo's `check:doc-shards` command; a plausible
rank-and-slug filename alone will fail the metadata contract.
