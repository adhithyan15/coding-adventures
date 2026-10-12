---
category: Repo policy / workflow reminders
---

# A changelog shard filename needs an eight-hex heading digest before the document checker will load it

The Gujarati A1 timing fix initially added a `CHANGELOG.d` filename ending in
the decimal issue number. `check:doc-shards` rejected it before reading the
entry: package changelog shards require a positive five-digit rank, an uppercase
ASCII slug, and a lowercase eight-hex digest of the level-three heading.

Choose the heading first, then derive its slug and SHA-256 heading digest with
`docSlug` and `headingDigest` in `human-language-data/src/doc-shard.ts`; use
those in the shard filename. Run `check:doc-shards` after adding the entry.
