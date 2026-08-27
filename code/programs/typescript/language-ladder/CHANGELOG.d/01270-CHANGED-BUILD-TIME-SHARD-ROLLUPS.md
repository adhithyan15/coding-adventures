## Changed — bounded build-time shard rollups

- Language Ladder now folds canonical spine, curriculum, and chapter shards
  through Vite virtual modules: one eager shared spine value and one lazy module
  per track for each ledger family. Adding authored elements no longer expands
  an eager `import.meta.glob` key table or requires a generated JSON aggregate.
- Chapter capabilities still come from current authored shards, independently
  of generated book hashes, so capability edits continue to report a stale
  book. The measured largest eager chunk is 331,763 bytes under the unchanged
  500,000-byte ceiling.
- Registry ids and ledger parents are validated before build-time reads, and
  dev mode explicitly reloads after shard additions or removals so the browser
  cannot retain a stale virtual rollup.
