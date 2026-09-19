---
category: Repo policy / workflow reminders
---

# Markdown history shard filenames require the lowercase eight-hex heading digest

The shared `check:doc-shards` gate rejected a newly authored changelog shard
whose filename stopped after the uppercase slug. New Markdown history shards
must use the repository's full five-digit-rank, uppercase-slug, lowercase
eight-hex-heading-digest identity. Derive both the bounded slug and digest
from the exact first heading or top-level bullet with the shared `docSlug` and
`headingDigest` helpers, then run `npm run check:doc-shards` before committing.
