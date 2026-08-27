# HL25 — Shard-native script inventories

## 1. Outcome

Japanese, Perso-Arabic, and Urdu-Nastaliq script entries are authored as one
JSON file per glyph or mark. No tracked compatibility aggregate is regenerated,
so independent glyph agents do not touch a shared JSON array.

This completes issue #13257 and the script-authoring tranche of #13193. It
builds on HL21's shard storage and security rules, HL24's owned evidence
modules, and the Japanese entry split from #13237 / PR #13241.

## 2. Canonical storage

The canonical paths are:

```text
data/scripts/japanese.d/
data/scripts/perso-arabic.d/
data/scripts/urdu-nastaliq.d/
  _meta.json
  letters/NNNN-U-<CODEPOINT>[-U-<CODEPOINT>...].json
  marks/NNNN-U-<CODEPOINT>[-U-<CODEPOINT>...].json
```

`_meta.json` owns every non-entry field. Each entry file owns one `letters`
or `marks` element. Raw Unicode glyphs do not appear in filenames because
filesystem normalization differs across APFS, ext4, and NTFS; uppercase code
point ids are the stable identity.

The three former compatibility files are forbidden:

```text
data/scripts/japanese.json
data/scripts/perso-arabic.json
data/scripts/urdu-nastaliq.json
```

`check:shards` fails if any of them is resurrected. A normal glyph edit changes
one entry shard plus any owned evidence/changelog shard, never an aggregate.

## 3. Entry and ordering invariants

The shard merge refuses data before exposing an inventory when any of these
conditions is false:

1. only `_meta.json`, `letters/*.json`, and `marks/*.json` participate;
2. every entry filename is `NNNN-U-<CODEPOINT>[-U-<CODEPOINT>...].json`;
3. the filename code-point id exactly matches `glyph` or `mark`;
4. one section cannot reuse an ordinal;
5. one glyph cannot be owned twice, including once as a letter and once as a
   mark;
6. `_meta.json` contains neither `letters` nor `marks`; and
7. sorted filename order reconstructs the pre-migration array order exactly.

Tools resolve an existing shard by code-point id, not by a hard-coded ordinal.
Ordinals are positions with insertion space; code-point ids are identities.

## 4. Consumer boundaries

### 4.1 Filesystem consumers

`human-language-data.loadScripts()` discovers both ordinary `*.json`
inventories and canonical `*.d/` inventories. A `.d/` name is converted to its
logical `.json` name only so the existing `readMaybeSharded()` boundary can
derive and validate the directory. The aggregate file need not exist.

Enumeration is deterministic and fail-closed. A symlink or non-directory at a
discovered `.d` path is an error, and two files may not silently claim the same
`script` id.

The Python font-subsetting path reads Japanese through the guarded shard helper
rather than recursively concatenating arbitrary files or depending on a
deleted aggregate.

### 4.2 Browser consumers

`@coding-adventures/script-ductus` imports one virtual module with a fixed id:

```text
virtual:script-ductus-inventories
```

Its build-time plugin reads exactly the three canonical shard directories,
merges them with the shared script-inventory rules, and emits named JavaScript
exports. This is a bounded module registry: three inventories and one virtual
id, independent of the number of glyph shards. It must not use
`import.meta.glob`, because an eager glob exposes one browser key per shard and
turns ordinary authoring growth back into eager bundle growth.

The plugin is installed in Script Ductus's Vitest config and in Language
Ladder's Vite and Vitest configs. Every contributing shard is registered with
the watch graph. An edit invalidates the virtual module; an add or unlink also
requests a full browser reload because no prior module/file association exists
for the new or removed path.

The plugin is build-time-only and is not re-exported from Script Ductus's
browser entry point. Standalone TypeScript typechecking sees a declaration for
the virtual module, while browser source never imports `node:fs`.

## 5. Filesystem trust boundary

All shard readers preserve HL21's controls:

- resolve the configured curriculum root and re-assert that each inventory
  parent remains inside it;
- reject every symlinked shard directory and shard file;
- reject non-regular JSON entries, malformed JSON, dangerous object keys, and
  empty shard sets;
- accept only fixed inventory ids at the virtual-module boundary; and
- treat add/unlink paths outside the curriculum root, or outside the three
  selected `*.d/` directories, as unrelated.

No watcher path, JSON field, registry value, or virtual-module suffix is
allowed to choose an arbitrary filesystem path.

## 6. Migration proof

Before deleting the three compatibility files, assemble each new shard set and
deep-compare it with the parsed pre-migration document. Persist deterministic
hash and entry-count pins in the test suite so later loss, duplication, or
reordering remains observable after the aggregates are gone.

The migration does not change glyph data, stroke sources, Script Ductus
behavior, Language Ladder behavior, or the Japanese font subset. It changes
only storage and the build-time boundary used to reach that storage.

## 7. Acceptance

Completion requires all of the following:

1. the three compatibility JSON files are absent and `check:shards` rejects
   their return;
2. the canonical shards reconstruct the exact pre-migration parsed documents;
3. adding or editing one glyph touches only its owned shard;
4. Human Language Data, Script Ductus, and Language Ladder suites and builds
   pass at their normal timeouts;
5. Language Ladder's eager chunk remains at or below 500 kB and its lazy-batch
   gates do not regress;
6. the affected-package build passes; and
7. an independent security review finds no unresolved filesystem or build-time
   trust-boundary issue before push.
