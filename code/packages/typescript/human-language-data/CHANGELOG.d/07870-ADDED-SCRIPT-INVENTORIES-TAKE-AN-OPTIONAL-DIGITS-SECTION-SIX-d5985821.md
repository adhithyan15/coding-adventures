### Added — script inventories take an optional digits section; six digit lessons print filmstrips

- **Optional `digits/` section (HL25).** The shard-native inventories
  (Japanese, Perso-Arabic, Tamil, Urdu-Nastaliq) could hold only letters and
  marks, so a cited digit had no row to live on. `mergeScriptInventoryShards`
  now also reads `digits/NNNN-U-<CODEPOINT>.json`, identified by `glyph`,
  under the same name, ordinal and single-owner rules. `MergeSection` and
  `ShardSection` gain `optional`: a section with no shards rebuilds with no
  key at all, so the Japanese and Tamil inventories are unchanged byte for
  byte. `shard-cli` shards, binds and checks the section; the owner
  declarations take an optional `digits/` directory of `kind: "digit"`
  files, and `--check` compares them with the inventory's digits as it does
  letters and marks. The Python reader (`sharded_ledger.py`) follows.
- **Data.** Perso-Arabic gains ten digit rows ۰-۹, each with its POH-Db
  citation; Urdu-Nastaliq gains ten, of which ۰-۳ are cited and ۴-۹ are
  recognition rows (Urdu ۴, ۶ and ۷ are different shapes).
- **Strips.** Persian 26 -> 30 (FA-W19 zero-one, two-three, four-five-six,
  seven-eight-nine) and Urdu 30 -> 32 (UR-W31 zero-one, two-three). Each
  strip sits in its lesson's Script block; narration and lesson text are
  unchanged, so no lesson's duration moves. UR-W31 four-five-six and
  seven-eight-nine stay undrawn.
- **Tests.** `script-shards.test.ts` (the optional section, its identity,
  ordinal and ownership rules), `script-owner-declarations.test.ts` (digit
  counts per inventory, an optional `digits/` held to the inventory, a wrong
  kind refused), the Perso-Arabic and Urdu evidence modules, and the
  filmstrip target-count pins.
