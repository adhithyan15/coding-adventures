# engram-anki-package

`engram-anki-package` is the APKG archive boundary for Engram.

It inspects `.apkg` / `.colpkg` zip archives, identifies the Anki collection
member (`collection.anki2`, `collection.anki21`, or `collection.anki21b`), and
parses the legacy JSON `media` map into archive-name to filename metadata.

It deliberately does not parse the SQLite collection yet. That next layer can
build on `inspect_apkg` and `read_collection_bytes` without pushing ZIP, media,
or package-format concerns into `engram-core`.
