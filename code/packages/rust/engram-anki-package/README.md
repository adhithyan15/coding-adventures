# engram-anki-package

`engram-anki-package` is the APKG archive boundary for Engram.

It inspects `.apkg` / `.colpkg` zip archives, identifies the Anki collection
member (`collection.anki2`, `collection.anki21`, or `collection.anki21b`), and
parses the legacy JSON `media` map into archive-name to filename metadata.
It can also write a deterministic legacy package envelope from existing
`collection.anki2` bytes plus media assets.

It deliberately does not parse or generate the SQLite collection yet. That next
layer can build on `inspect_apkg`, `read_collection_bytes`, and
`write_legacy_apkg` without pushing ZIP, media, or package-format concerns into
`engram-core`.
