# Changelog

## Unreleased

### Added — Journal on the standard Mosaic ABI (0.1.0, J3c-1 of #14416)

- **`JournalMosaicApp`** implements `MosaicApp` over `journal-core` and is
  exported with `export_mosaic_app!` as the `libmosaic_app` that generated
  hosts load.
- **Props:** the `timeline-rows` are `RecordList` rows. Each day's heading
  appears only on its first row. A title is never empty: it falls back to the
  body's opening words, then to "Untitled entry".
- **Other props:** `selected-key`, `timeline-empty`, the draft, and
  `delete-label` (empty for a new entry, so `DraftEditor` hides the button).
- **Events:** select, new, title/body change, save, delete, cancel. Unknown
  events and bad payloads are errors, and the whole event rolls back, including
  an engine rejection.
- **Ids and dates:** entry ids are minted as `entry-{n}`, skipping ids already
  in use. "Today" is the UTC date, because `StartContext` carries no time zone.
- **Snapshots** (`journal-mosaic-app/state` v1) include an unsaved draft. On
  restore the journal is validated and a stale target is repaired.
- **Hardening from the pre-push security review:**
  - Id minting uses checked arithmetic, and restore refuses a counter above
    2^53. A tampered `u64::MAX` made save spin forever.
  - Drafts are capped at the engine's limits, both as they are typed and on
    restore.
  - Rollback saves only the editor fields, not the whole journal (about 20 ms
    per keystroke at scale).
- **Tests:** 17.
