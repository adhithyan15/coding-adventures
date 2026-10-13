---
category: CI & GitHub Actions
---

# A CHANGELOG.d shard must match its document's entry shape, which differs per package

**What went wrong.** I added a `mosaic-emit-html/CHANGELOG.d` shard that
opened with a `### Fixed — …` heading, copying the shape of the
`mosaic-emit-xaml` shards next to it. CI failed in the human-language-books
"Build curriculum gap report" step, and also in ci.yml's "Verify sharded
documents rebuild". The error was: `doc shard "00860-…" must start with its
top-level bullet`.

**Why.** Each sharded document has its own plan in `DOC_SHARD_PLANS`
(`code/packages/typescript/human-language-data/src/doc-shard-cli.ts`). The
`mosaic-emit-xaml` plan splits on level-3 headings, so its shards start with
`###`. The `mosaic-emit-html` plan uses `entryShape: "bullet"`, so each shard
must be one top-level `- ` bullet. Neighbouring packages do not share a
shape.

**Next time.** Before writing a new shard, copy the shape of the newest
sibling shard in the same `CHANGELOG.d/`, not one from another package. Then
run the checker:
`cd code/packages/typescript/human-language-data && npm ci && npm run build && npm run check:doc-shards`.
