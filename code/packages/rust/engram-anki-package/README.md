# engram-anki-package

`engram-anki-package` is the APKG archive boundary for Engram.

It inspects `.apkg` / `.colpkg` zip archives, identifies the Anki collection
member (`collection.anki2`, `collection.anki21`, or `collection.anki21b`), and
parses the legacy JSON `media` map into archive-name to filename metadata.
It can resolve media archive members into filename metadata plus byte payloads,
and can also write a deterministic legacy package envelope from existing
`collection.anki2` bytes plus media assets. `write_modern_apkg` writes the
same SQLite payload in a modern `collection.anki21b` envelope with Anki `meta`
protobuf metadata, zstd-compressed payloads, and protobuf media entries.
`write_v11_collection_bytes_from_engram_state` generates a legacy/V11 SQLite
collection from `engram-core::AppState`, while
`write_legacy_apkg_from_engram_state` and `write_modern_apkg_from_engram_state`
wrap that collection in legacy or modern APKG envelopes. The V11 collection
byte helpers serialize and deserialize SQLite databases in memory, keeping the
same import/export path usable from native host bridges and sidecars without
temporary files.

It also parses legacy/V11 SQLite collection files into an owned Anki
representation. `read_v11_collection` accepts APKG bytes, extracts
`collection.anki2` or `collection.anki21`, and reads the `col`, `notes`,
`cards`, `revlog`, and `graves` tables through the repo `sqlite-file`
byte-reader rather than opening SQLite through `rusqlite`.
`parse_v11_collection_bytes` exposes the same parser for raw SQLite collection
bytes.
`read_v11_collection_as_engram_state` and `v11_collection_to_engram_state`
map that parsed representation into `engram-core::AppState` while preserving
Anki IDs as deterministic Engram IDs. Cloze note types render
`{{cloze:Field}}` and filtered cloze templates into Engram card fronts/backs
with the same `[...]` / `[hint]`, section, and `FrontSide` behavior used by the
core cloze generator. Imported card rendering also fills Anki's special
template fields for tags, note type, deck, subdeck, card template, and card
flag/card ID metadata, and model-level `req` rows import/export as Engram's
shared template requirement mode. Template-level deck overrides (`did` in Anki
model JSON) import into `CardTemplate::deck_id`, drive regenerated sibling card
decks, and export back to Anki model JSON.

`read_collection_bytes` returns the detected collection member as decoded raw
SQLite bytes for inspection workflows. `read_v11_collection_bytes` accepts
legacy `collection.anki2` / `collection.anki21` members and modern
`collection.anki21b` package envelopes by honoring Anki's `meta` protobuf and
zstd-decompressing the collection payload before parsing.
Modern media manifest inspection exposes each protobuf entry's SHA-1 digest as
lowercase hex and preserves `legacyZipFilename` when Anki includes it, so native
hosts can verify imported payloads without re-parsing the protobuf map.
Imported APKG media assets also receive `ExternalSourceTarget::Media`
provenance records with the original archive member and logical filename, so
merge/import flows can preserve where audio and image payloads came from even
when Engram has to rename an internal media asset ID.

The export path preserves numeric Anki IDs when Engram state came from Anki,
allocates deterministic numeric IDs for Engram-native rows, writes decks,
models, notes, cards, progress, and review rows, and falls back to a synthetic
Basic note type for standalone front/back cards. Deck option import/export
includes daily limits, learning/relearning steps, initial ease factor, interval
multipliers, lapse multiplier, maximum interval, leech threshold/action, and
Anki-style sibling bury booleans. It also imports and exports Anki FSRS preset
fields including desired retention, `fsrsParams6`/`fsrsParams5`/legacy
`fsrsWeights`, weight-search queries, ignored-history dates, historical
retention, and easy-day percentages. Anki's
special `marked` note tag imports as Engram marked-card progress for each card
generated from that note, and Engram marked cards export the canonical `marked`
tag so the mark survives APKG round-trips. Imported suspended cards and Anki's
user-buried / scheduler-buried queue distinction (`-2` vs `-3`) are preserved
when exporting back to APKG, along with the imported collection creation day
that anchors Anki's due-day offsets and the collection's modification/schema
metadata. Imported new-card due positions remain available to the shared queue
builder so study order matches Anki's new-card queue. Learning and relearning
cards translate Anki's packed `left`
remaining-step field into Engram's internal step index and back again on export.
Imported filtered decks normalize Anki's `dyn` and `resched` flags into deck
external-source metadata so the shared reducer can honor non-rescheduling
filtered/custom-study sessions without APKG-specific logic in host shells.
Anki model CSS imports into shared `NoteType::stylesheet`; Engram-native
stylesheets export back to Anki model `css` so note styling does not depend on
preserved raw model JSON alone.
Deleted imported decks, notes, and cards that the shared core records as
external-source tombstones export as Anki `graves` rows, preserving sync-visible
deletions across APKG round-trips. Imported revlog rows can still export after
their card has been deleted by falling back to the preserved Anki `cardId`.
Imported deck and note-type modification metadata is also retained
when Engram re-emits the stored Anki JSON, and imported note/card row
modification timestamps, model sort-field selection, note checksums, and
revlog answer-time metadata are preserved on export until the shared reducer
locally changes the card's scheduling or flag state. Engram-native reviews can
also export their optional shared-core `Review::answer_time_ms` value into
Anki's `revlog.time` column, so non-web Mosaic/native shells do not need their
own Anki-specific review-duration path.

`tests/fixtures/golden-v11-filtered-media.apkg` pins a deterministic V11 package
with filtered-deck metadata and media references through `include_bytes!`. Run
the ignored `regenerate_checked_in_golden_v11_apkg_fixture` test when the
fixture shape intentionally changes.
