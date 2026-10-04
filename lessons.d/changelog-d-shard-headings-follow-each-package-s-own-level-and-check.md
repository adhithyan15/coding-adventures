---
category: CI & GitHub Actions
---

# CHANGELOG.d shard headings follow each package's own level, and check:doc-shards rejects the wrong one

**What went wrong.** PR #16655 added a `mosaic-emit-compose` changelog shard
headed `### Fixed — ...`, the style `mosaic-package-artifact-builder` uses.
`mosaic-emit-compose`'s shards are level-2, dated: `## 2026-09-27 (the
Android half of the platform seams)`. CI's "Repo-wide metadata contracts"
job runs `npm run check:doc-shards` (in
`code/packages/typescript/human-language-data`), which rebuilds every sharded
document and refused the shard: "must start with its level-2 heading; the
document preamble belongs in _meta.md".

**Fix.** Rewrite the shard as `## <date> (<title>)` and rename it so the slug
and the 8-character hash come from that heading (the hash is
`sha256(heading)[:8]` of the full heading line, `## ` included).

**Do differently.** Before writing a shard, open the newest existing shard in
that package's `CHANGELOG.d/` and copy its heading level and style; they
differ between packages. Then run the check locally, which needs only npm:
`cd code/packages/typescript/human-language-data && npm ci && npm run build
&& npm run check:doc-shards`.
