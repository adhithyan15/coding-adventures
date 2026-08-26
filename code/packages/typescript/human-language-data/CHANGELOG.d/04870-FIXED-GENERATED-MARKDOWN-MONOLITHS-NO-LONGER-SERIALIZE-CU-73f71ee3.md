### Fixed - generated Markdown monoliths no longer serialize curriculum PRs (HL23)

- Stop tracking the reconstructed human-languages `BACKLOG.md` and package
  `CHANGELOG.md`; their `.d/` directories remain the only committed source of
  truth, while `--unshard` creates an ignored local single-file view on demand.
- Teach `check:doc-shards` to accept the normal clean-checkout state where no
  rendered monolith exists, while still rejecting a stale local render and any
  shard set whose files do not rebuild as exactly one section apiece.
- Preserve the deleted-monolith loss signal explicitly in CI: history shards
  are append-only across a pull-request diff, and the two render targets must
  remain untracked. Update repository changelog policy and documentation links
  to use the canonical shard entry points.

