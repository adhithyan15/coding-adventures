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
- `deck_stats(deck_id, now)`
- `session_progress()`
- `generated_cards(note_type_id, note_id)`
- `search_cards(query, now)`
- `export_cards_csv(deck_id)`
- `parse_cards_csv(csv)`
- `parse_basic_cards_csv(csv, deck_id, id_prefix, created_at)`

All JSON uses camelCase field names to match the existing TypeScript Engram app
and keep generated bindings idiomatic.

`rateCard` remains backward-compatible with the original command shape. Hosts
may also include a `deckOptions` object to drive the Rust scheduler with custom
learning steps, relearning steps, daily limits, graduation intervals, and lapse
behavior. When `deckOptions` is omitted, the core applies `DeckOptions::default()`.

The facade also exposes review-control commands:

- `suspendCard` / `unsuspendCard`
- `buryCard` / `unburyCard`
- `setCardFlag`
- `markCard` / `unmarkCard`
- `undoLastReview`

Those commands update the Rust state snapshot directly, including active-session
queues, so host shells do not need their own suspend or bury reducers.
`undoLastReview` restores the previous progress snapshot recorded on the review
and rewinds session counters through the same shared reducer.
Flags and marks live on `CardProgress` as optional metadata so collection
browsers can filter them without changing scheduling behavior.

`search_cards` exposes the shared browser-query engine. It returns
`{ ok: true, results }` for valid queries and `{ ok: false, error, token }` for
parser diagnostics.

`session_progress` returns `{ ok: true, progress }`, where `progress` is null
without an active session or contains shared review counters such as
`totalCards`, `currentPosition`, `remainingCards`, `cardsReviewed`,
`cardsCorrect`, `revealed`, and `completed`.

`export_backup` and `import_backup` expose the versioned Engram JSON backup
shape. Backup import validates the app/version fields and restores only durable
collection data, clearing any live active session.

`export_cards_csv` and `parse_cards_csv` expose the shared card CSV helpers.
Hosts still own file picking, conflict handling, and whether parsed cards are
inserted, merged, or previewed.
`parse_basic_cards_csv` supports simpler `front,back` files and deterministic
ID generation from host-supplied import options.
