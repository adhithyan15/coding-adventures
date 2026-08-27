## Changed — shard-native script inventories

- Fold Japanese, Perso-Arabic, and Urdu-Nastaliq shards through Script Ductus's
  fixed build-time module instead of depending on generated aggregate files.
- Keep one virtual inventory module independent of glyph count, with shard
  edits watched in development and the eager bundle ceiling enforced.
