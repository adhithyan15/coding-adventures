---
category: Workspace & package metadata
---

# Changelog shard filenames require the digest of the exact heading

PR #15523 failed both metadata jobs because a manually added changelog shard
omitted the eight-character heading digest. Use the document-shard naming
contract: positive five-digit rank, uppercase ASCII slug (at most 60 characters),
and the first eight lowercase hex digits of SHA-256 of the exact heading line,
without its newline. Run human-language-data's build and check:doc-shards before
publishing a new shard; a valid Markdown body alone does not satisfy this gate.
