# engram-core-wasm

`engram-core-wasm` is the string-in / JSON-out facade over
[`engram-core`](../engram-core). It is intentionally a normal Rust library so
it can be tested without a WASM toolchain.

This crate is the shared host contract for every Engram shell:

```text
HTML / React / Electron / XAML / SwiftUI macOS+iOS / Qt
        |
        v
engram-wasm / engram-capi / platform loader
        |
        v
engram-core-wasm        string-in / JSON-out facade
        |
        v
engram-core             notes, cards, scheduling, queues, sessions
```

Target shells may own packaging and platform APIs, but they should not
reimplement scheduling, card generation, queueing, stats, or state transitions.

## API

`EngramSession` owns an in-memory `AppState` and exposes:

- `snapshot()`
- `load_snapshot(json)`
- `export_backup(exported_at)`
- `import_backup(json)`
- `dispatch(command_json)`
- `build_queue(deck_id, now)`
- `daily_limit_usage(deck_id, day_start, day_end, deck_options_json)`
- `build_queue_with_daily_limits(deck_id, now, day_start, day_end, deck_options_json)`
- `deck_stats(deck_id, now)`
- `session_progress()`
- `review_history(deck_id, reviewed_after, reviewed_before)`
- `generated_cards(note_type_id, note_id)`
- `materialized_cards(note_type_id, note_id, created_at)`
- `search_cards(query, now)`
- `export_cards_csv(deck_id)`
- `export_anki_basic_tsv(deck_id, deck_name, note_type_name, html)`
- `parse_cards_csv(csv)`
- `parse_basic_cards_csv(csv, deck_id, id_prefix, created_at)`
- `parse_anki_basic_tsv(tsv, deck_id, id_prefix, created_at)`
- `engram_app_props(deck_id, now)`
- `handle_engram_app_event(event, deck_id, now)`

All JSON uses camelCase field names to match the existing TypeScript Engram app
and keep generated bindings idiomatic.
`dispatch` accepts note-first `upsertNoteType`, `deleteNoteType`, `upsertNote`,
and `deleteNote` commands; pass `materializeCardsAt` with `upsertNoteType` or
`upsertNote` to sync generated cards from the note type through the shared core.
It also accepts `setDeckOptions` to insert or replace the durable scheduler
options for a deck, using the same camelCase `DeckOptions` shape accepted by
`rateCard`.
`build_queue`, `build_queue_with_daily_limits`, `daily_limit_usage`, and
`deck_stats` use the full loaded `AppState` to include Anki-style child decks
named with `Parent::Child` when the selected deck is a parent. HTML, Electron,
Qt, SwiftUI, XAML, and other hosts therefore share the same parent-deck study
scope without implementing hierarchy rules themselves.

`rateCard` remains backward-compatible with the original command shape. Hosts
may also include a `deckOptions` object to drive the Rust scheduler with custom
learning steps, relearning steps, daily limits, graduation intervals, and lapse
behavior. `deckOptions` can also tune Anki-style maximum interval, review
interval modifier, hard interval multiplier, easy bonus multiplier, and
same-note sibling-bury defaults for new, review, and interday-learning cards.
When `deckOptions` is omitted, the core applies `DeckOptions::default()`.
Hosts may also include `burySiblingsUntil` to rate the current card and bury
same-note siblings in the same reducer transition; the review log records enough
snapshots for `undoLastReview` to restore the sibling state and active queue.

The facade also exposes review-control commands:

- `suspendCard` / `unsuspendCard`
- `buryCard` / `unburyCard`
- `buryCardSiblings`
- `setCardFlag`
- `markCard` / `unmarkCard`
- `undoLastReview`
- `upsertMediaAsset`
- `deleteMediaAsset` / `deleteMediaAssets`

Those commands update the Rust state snapshot directly, including active-session
queues, so host shells do not need their own suspend or bury reducers.
`buryCardSiblings` uses optional `card.lineage.noteId` data to hide same-note
siblings until the host-provided `buriedUntil` timestamp. `undoLastReview`
restores previous review, sibling, and active-session snapshots and rewinds
session counters through the same shared reducer.
Flags and marks live on `CardProgress` as optional metadata so collection
browsers can filter them without changing scheduling behavior.
Media commands update `AppState.mediaAssets` through the same facade, letting
web, Electron, SwiftUI, XAML, Qt, and other native hosts copy attached media,
replace imported payloads, or prune unreferenced media IDs without a separate
platform reducer.

`handle_engram_app_event` returns updated props for generated Mosaic shells.
Events that require host APIs, including browser open/edit, Anki package
import/export, and note/note-type dialogs, include a `hostIntent` payload so
each platform can open files, save APKG bytes, or present native dialogs without
forking the generated interface.

`search_cards` exposes the shared browser-query engine. It returns
`{ ok: true, results }` for valid queries and `{ ok: false, error, token }` for
parser diagnostics.

`materialized_cards` returns generated note-template cards as durable `Card`
records with `lineage`, ready for hosts to insert into snapshots and review
queues without rebuilding note/template provenance per platform.

`session_progress` returns `{ ok: true, progress }`, where `progress` is null
without an active session or contains shared review counters such as
`totalCards`, `currentPosition`, `remainingCards`, `cardsReviewed`,
`cardsCorrect`, `revealed`, and `completed`.

`engram_app_props` includes Mosaic-slot-shaped labels and counts for collection
actions, browser rows, selected-deck scheduler options, review history
summaries, and secondary review actions such as undo, bury card, bury siblings,
suspend, and mark/unmark.
`handle_engram_app_event` accepts those generated events (`onUndo`,
`onBuryCard`, `onBurySiblings`, `onSuspendCard`, and `onToggleMark`) and routes
them through the same core reducer commands used by direct JSON dispatch. It
also accepts browser events such as `onBrowserSearch` and targeted row actions
such as `onBrowserToggleMarkSelected|card-id` or
`{"event":"onBrowserToggleSuspendSelected","cardId":"card-id"}`.
Generated `onBrowserQueryChange` events store the active browser query in the
session and reset selection to the first row. `onBrowserSelectResult` accepts an
`index` or `selectedIndex` payload, updates the shared selection, and all later
open/edit/mark/suspend browser actions use that selected row when no explicit
card ID is provided. This keeps HTML, Electron, SwiftUI, XAML, Qt, and other
Mosaic hosts on the same browser-selection contract.
Deck option controls use the generated event shape, for example
`{"type":"deckOptionsNewCardsChange","value":12}` for numeric fields or
`{"type":"deckOptionsLearningStepsChange","value":"1, 10"}` for step-list
fields. Checkbox fields use the native HostCheckbox payload, for example
`{"type":"deckOptionsBuryNewSiblingsChange","checked":false}`. All update the
selected deck through the shared `setDeckOptions` reducer path.
Collection workflow events such as `onImportAnki`, `onExportAnki`, `onAddNote`,
`onAddNoteType`, `onDeleteNote`, and `onDeleteNoteType` round-trip through the
same event parser as host intents so generated shells share one UI contract
while file picking, dialogs, and concrete note payloads stay host-owned.

`daily_limit_usage` returns `{ ok: true, usage }` with new/review counts already
seen in a host-provided day window and the remaining slots from `DeckOptions`.
`build_queue_with_daily_limits` uses the same usage calculation before returning
`{ ok: true, queue }`. Passing an empty `deck_options_json` uses default deck
options; partial JSON option objects fill omitted fields from defaults.

`deck_stats` includes `suspendedCount` and `buriedCount` alongside due/new and
learning/mastered counts.

`review_history` returns `{ ok: true, history }` with deck-scoped counts for a
half-open timestamp range: total reviews, correct reviews, unique cards,
per-rating counts, and first/last review timestamps.

`export_backup` and `import_backup` expose the versioned Engram JSON backup
shape. Backup import validates the app/version fields and restores only durable
collection data, clearing any live active session.

`export_cards_csv` and `parse_cards_csv` expose the shared card CSV helpers.
Hosts still own file picking, conflict handling, and whether parsed cards are
inserted, merged, or previewed.
`export_anki_basic_tsv` and `parse_anki_basic_tsv` expose Anki-compatible Basic
front/back text files with Anki import headers and quoted tab/newline fields.
`parse_basic_cards_csv` supports simpler `front,back` files and deterministic
ID generation from host-supplied import options.
