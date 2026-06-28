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

All JSON uses camelCase field names to match the existing TypeScript Engram app
and keep generated bindings idiomatic.

`rateCard` remains backward-compatible with the original command shape. Hosts
may also include a `deckOptions` object to drive the Rust scheduler with custom
learning steps, relearning steps, daily limits, graduation intervals, and lapse
behavior. When `deckOptions` is omitted, the core applies `DeckOptions::default()`.
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

Those commands update the Rust state snapshot directly, including active-session
queues, so host shells do not need their own suspend or bury reducers.
`buryCardSiblings` uses optional `card.lineage.noteId` data to hide same-note
siblings until the host-provided `buriedUntil` timestamp. `undoLastReview`
restores previous review, sibling, and active-session snapshots and rewinds
session counters through the same shared reducer.
Flags and marks live on `CardProgress` as optional metadata so collection
browsers can filter them without changing scheduling behavior.

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
