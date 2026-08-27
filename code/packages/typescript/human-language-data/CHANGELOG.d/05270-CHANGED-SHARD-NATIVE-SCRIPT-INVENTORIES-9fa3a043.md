### Changed — shard-native script inventories

- Make Japanese, Perso-Arabic, and Urdu-Nastaliq canonical as one glyph or mark
  per file and remove their generated aggregate JSON files.
- Discover script inventory directories through the guarded shared loader and
  preserve exact pre-migration ordering and values with deterministic pins.
- Give Python font subsetting the same fail-closed shard boundary.
