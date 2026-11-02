---
category: CI & GitHub Actions
---

# Changelog shard filenames must follow the document shard contract

PR #18512 used a date-prefixed changelog file inside a registered CHANGELOG.d set. Repository metadata CI rejected it: shards require a positive five-digit rank, uppercase slug and heading digest. Move a small unreleased entry into the existing _meta.md section, or use the document shard tooling to generate a valid new shard. Do not assume an arbitrary Markdown filename is accepted. Run the repository document-shard contract check before publication.
