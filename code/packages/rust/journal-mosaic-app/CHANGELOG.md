# Changelog

## Unreleased

- **An empty journal (J4h).** A new slot, `journal-empty`, is set when a
  journal is selected, it has no entries, and there is no search. It is
  not set when no journal has any entries, which is `timeline-empty`, and it
  wins over `no-starred`, so the pane gives the more accurate reason.
  `JournalScreenshots` adds 09a-empty-journal.
- **An entry's journal (J4g).** `draft_journal` joins the draft (in the
  snapshot with `#[serde(default)]`; empty means "not chosen", and a restored
  id longer than `MAX_ID_BYTES` is refused). It is read through
  `draft_journal()`, which always names a journal that exists: the one
  chosen in the picker, else the open entry's own, else the pane's selected
  journal, or Personal under "All journals".
  - Save files a new entry there. For an existing entry whose journal
    changed, Save runs `MoveEntry`, after the date and tags are checked, so a
    refused Save moves nothing.
  - Opening an entry loads its journal. Cancel reverts the choice, and New
    entry and Delete clear it.
  - New slots: `draft-journal-options` (journal names, no "All journals"),
    `draft-journal-index`, and `has-journals`. New event:
    `onDraftJournalChange`.
  - `JournalScreenshots` adds 12-moved-to-personal.
- **Journals (J4f).** New slots `journal-options` ("All journals", then each
  journal by name in creation order, ties by id), `selected-journal-index`,
  `new-journal-name` and `journal-error`. New events `onSelectJournal`,
  `onNewJournalNameChange` and `onAddJournal`.
  - `journal_filter` (not persisted) feeds `EntryFilter::journal` to the
    timeline, search, On this day and the tag counts. A journal that no
    longer exists stops filtering.
  - *Add* runs `CreateJournal` with a minted `journal-{n}` (skipping ids in
    use) and the trimmed name, then selects the new journal. A blank name
    does nothing. A refused name (a duplicate, too long or several lines,
    too many journals) is said in `journal-error`, and the journal is
    untouched.
  - The name field is capped at `MAX_JOURNAL_NAME_CHARS` as typed.
  - A new entry is filed in the selected journal, or in Personal under All.
  - `JournalScreenshots` adds 10-work-journal and 11-journal-error.
- **An entry's day (J4e).** `draft_date` joins the draft (`#[serde(default)]`).
  Save parses it with `Date::parse_iso` BEFORE writing, then runs
  `CreateEntry` with that day, or `EditEntry` plus `SetEntryDate` when the
  day changed. A blank date is today for a new entry and the entry's own day
  for an old one.
- **Draft errors are shown, not thrown.** A bad date or tag now returns a
  normal update with `draft-error` set (and announced), and the journal is
  untouched. J4d returned an error from `dispatch`, which no host showed.
  Any other event clears the message. `JournalScreenshots` adds
  09-draft-error.
- **Tags (J4d).** `draft_tags` joins the draft (in the snapshot with
  `#[serde(default)]`, so older version-1 snapshots still load; a test pins
  it). Save checks the tags with `normalize_tags` BEFORE writing, then runs
  `CreateEntry`/`EditEntry` and `SetTags`, so a bad tag fails the whole Save
  with the journal unchanged. After Save the field shows the tags as the
  engine tidied them. Row `meta` is `#a #b`, at most 24 characters.
  `tag_filter` (not persisted) feeds `EntryFilter::tag` to the timeline,
  search and On this day. The options count every entry, and a tag that no
  longer exists stops filtering. `JournalScreenshots` now finds fields by
  tag rather than position, and adds 08-tag-filter.
- **On this day (J4c).** `on-this-day-rows` come from journal-core's
  `on_this_day` for the user's LOCAL today (the same `today(clock, offset)`
  that files entries), through the shared filter. Each year opens with a
  "N years ago · 24 Sep 2025" heading. `has-on-this-day` is false during a
  search. `onSelectOnThisDay { index }` opens a recalled entry; it has its
  own event because each `RecordList` indexes its own rows.
- **Stars (J4b).** `onToggleStar` flips `Entry::starred` for the entry in the
  editor through `Command::SetStarred`. It is an error for a new draft. It
  does NOT save the draft, so an unsaved edit stays unsaved. `starred_only`
  (not persisted, like the query) feeds `EntryFilter::starred_only` to both
  `timeline` and `search`. `no-starred` covers only the unsearched timeline,
  so each empty state says the right thing. `JournalScreenshots` adds
  07-starred-only.
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
