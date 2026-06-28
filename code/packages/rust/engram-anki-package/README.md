# engram-anki-package

`engram-anki-package` is the APKG archive boundary for Engram.

It inspects `.apkg` / `.colpkg` zip archives, identifies the Anki collection
member (`collection.anki2`, `collection.anki21`, or `collection.anki21b`), and
parses the legacy JSON `media` map into archive-name to filename metadata.
It can resolve media archive members into filename metadata plus byte payloads,
and can also write a deterministic legacy package envelope from existing
`collection.anki2` bytes plus media assets.

It also parses legacy/V11 SQLite collection files into an owned Anki
representation. `read_v11_collection` accepts APKG bytes, extracts
`collection.anki2` or `collection.anki21`, and reads the `col`, `notes`,
`cards`, `revlog`, and `graves` tables. `parse_v11_collection_bytes` exposes
the same parser for raw SQLite collection bytes.

`read_collection_bytes` returns the detected collection member as raw bytes for
inspection workflows. `read_v11_collection_bytes` is the import boundary for the
next SQLite milestone: it accepts `collection.anki2` and `collection.anki21`,
but rejects `collection.anki21b` until Engram has modern Anki V18 package
support.

The crate still does not map Anki collections into `engram-core::AppState` or
generate SQLite collections. Those layers can build on the parsed V11 structs
without pushing ZIP, media, or Anki package-format concerns into `engram-core`.
