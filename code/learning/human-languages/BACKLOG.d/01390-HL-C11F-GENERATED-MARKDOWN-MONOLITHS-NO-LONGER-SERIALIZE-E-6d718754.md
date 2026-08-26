## HL-C11F — Generated Markdown monoliths no longer serialize every curriculum PR

**Status:** closed structurally by HL23 / GitHub issue #12953.

HL22 made `BACKLOG.d/` and `human-language-data/CHANGELOG.d/` authoritative,
but kept their reconstructed Markdown monoliths under version control. That
left the busiest two shared files in every curriculum diff. Spanish PR #12943
then measured the practical result: five of six mainline syncs conflicted on
the two generated aggregates, repeatedly resetting a roughly half-hour CI race.

HL23 removes the generated files from version control and ignores local renders.
The canonical `_meta.md` files remain stable entry points, relative links target
the shard directory, and `npm run unshard:docs -- <path>` can still render a
single searchable file locally. The build gate now validates shard structure
without requiring a monolith, rejects tracked render targets, and rejects shard
deletions in pull-request diffs. That preserves fail-closed history coverage
without making every language branch edit one shared generated file.

