### Fixed — malformed Markdown shard filenames fail closed

- The shared Markdown document reader now accepts only `_meta.md` or an HL22
  positive five-digit rank, uppercase ASCII slug, and lowercase eight-hex
  heading digest. A malformed `*.md` fragment is rejected before any document
  is rendered, so filename ordering cannot silently move history.
- Preserved 85 append-only legacy fragments at their existing paths and pinned
  their full-content SHA-256 values in a closed compatibility manifest. Unknown
  legacy-shaped additions and edits to grandfathered history now fail closed.
- Same-rank parallel fragments remain legal and deterministically ordered by
  their full names.
- Added pure grammar coverage plus a check-mode regression for missing ranks,
  zero ranks, malformed slugs and digests, and reserved metadata lookalikes.
