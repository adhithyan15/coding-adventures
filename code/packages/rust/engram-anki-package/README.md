# engram-anki-package

`engram-anki-package` is the APKG archive boundary for Engram.

It inspects `.apkg` / `.colpkg` zip archives, identifies the Anki collection
member (`collection.anki2`, `collection.anki21`, or `collection.anki21b`), and
parses the legacy JSON `media` map into archive-name to filename metadata.
It can resolve media archive members into filename metadata plus byte payloads,
and can also write a deterministic legacy package envelope from existing
`collection.anki2` bytes plus media assets. `write_v11_collection_bytes_from_engram_state`
generates a legacy/V11 SQLite collection from `engram-core::AppState`, and
`write_legacy_apkg_from_engram_state` wraps that collection in a deterministic
APKG envelope.

It also parses legacy/V11 SQLite collection files into an owned Anki
representation. `read_v11_collection` accepts APKG bytes, extracts
`collection.anki2` or `collection.anki21`, and reads the `col`, `notes`,
`cards`, `revlog`, and `graves` tables. `parse_v11_collection_bytes` exposes
the same parser for raw SQLite collection bytes.
`read_v11_collection_as_engram_state` and `v11_collection_to_engram_state`
map that parsed representation into `engram-core::AppState` while preserving
Anki IDs as deterministic Engram IDs. Cloze note types render
`{{cloze:Field}}` templates into Engram card fronts/backs with the same
`[...]` / `[hint]` question behavior used by the core cloze generator.

`read_collection_bytes` returns the detected collection member as raw bytes for
inspection workflows. `read_v11_collection_bytes` is the import boundary for
legacy SQLite packages: it accepts `collection.anki2` and `collection.anki21`,
but rejects `collection.anki21b` until Engram has modern Anki V18 package
support.

The export path preserves numeric Anki IDs when Engram state came from Anki,
allocates deterministic numeric IDs for Engram-native rows, writes decks,
models, notes, cards, progress, and review rows, and falls back to a synthetic
Basic note type for standalone front/back cards.
