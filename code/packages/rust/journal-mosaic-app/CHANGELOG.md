# Changelog

## Unreleased

- `conformance/compose/JournalUiTest.kt`: Journal on Compose is DRIVEN in CI,
  not only compiled. The first launch writes and saves two entries through
  the generated editor, reopens one from the timeline and deletes it; a
  second launch on the same state file (`MOSAIC_EXPECT_RESTORED=1`) must find
  the survivor, delete it and show the empty journal. It also sends a
  malformed `onTitleChange` through the binding and checks that the snapshot
  is unchanged. The Linux Compose lane now builds the distributable, checks
  that it bundles this runtime, and runs both launches.
- **Search (J4a).** `search_query` (not in the snapshot; a restart opens the
  full timeline) and `onSearchChange` / `onClearSearch`. A query over the
  engine's 1,024 characters is refused as it is typed. While searching,
  `timeline-rows` holds the hits in rank order: no day heading, the engine's
  snippet as the subtitle, and the day as a short `meta` ("24 Sep 2026"). The
  long heading form did not fit beside a title in the 300px pane on Compose,
  where it was drawn over the title. `timeline-empty` now means "no entries
  at all"; `no-matches` is new. `onSelectEntry`'s index refers to the rows last
  rendered, results included. `JournalScreenshots` adds 05-search-results and
  06-no-matches; the editor's fields are now the 2nd and 3rd text fields.
- `conformance/compose/JournalScreenshots.kt`: a screenshot harness for the
  generated Compose Journal over this runtime. Set `MOSAIC_SHOT_DIR` and run
  `gradle test --tests JournalScreenshots` in an emitted project. It checks
  only that rendering succeeds; the images are for people to look at.
- `js/wasm.test.mjs` creates every app at UTC (`utcOffsetMinutes: 0`). The
  Mosaic loader now sends the machine's own offset, and the day headings would
  otherwise depend on where the tests run.

### Changed — entries are filed under the user's local day

- "Today" is the local date at the host's `StartContext.utc_offset_minutes`,
  so an evening entry in New York no longer lands on tomorrow's UTC date. A
  host that passes no offset gets the UTC date, as before. The offset is not
  in the snapshot, and it is applied before the last-writable clamp, so no
  offset can date an entry past 9999-12-31.

### Added — Journal in the browser: the runtime (J5c-1 of #14416)

- **wasm32 build:** `mosaic_app_wasm::export_mosaic_wasm!` exports the
  standard lifecycle bridge next to the native C ABI.
- **The clock is a host import on wasm32** (`journal.now_ms() -> f64`).
  `SystemTime::now()` panics on `wasm32-unknown-unknown`. The host passes it
  through a shim that never throws.
- **Out-of-range clock readings date at the epoch.** Any non-finite or negative
  value, or one past 9999-12-31, reads as the epoch, on native and wasm alike.
  journal-core reads back only four-digit years, so a far-future entry would
  have made the whole snapshot unrestorable.
- **Bare event names** (`selectEntry`, as the generated React component
  dispatches them) are read as their emit names. An unknown event's error
  still names what was sent.
- `js/wasm.test.mjs` drives the real wasm through `mosaic-host.mjs`: write,
  save, restore; bare and prefixed names; a hostile clock; and a module loaded
  without the clock import, which must fail to instantiate.

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
