### Changed — Japanese script inventory has entry-owned shards

- Made `data/scripts/japanese.d/` the canonical inventory, with one stable
  Unicode-code-point-named file per letter or mark and a generated
  `japanese.json` browser compatibility view.
- Made the loader fail closed on malformed entry filenames, mismatched ids,
  duplicate ordinals, duplicate glyph ownership, and unknown shard kinds.
- Moved Japanese glyph-specific writing provenance out of the shared script
  README's running evidence diary; each entry shard now owns its evidence.
