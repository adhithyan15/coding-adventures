# engram-core

`engram-core` is the headless Rust engine for Engram-style study apps. It owns
the durable learning model, Anki-style scheduler state transitions, session queues, session reducers,
and deck statistics. It does not own UI, persistence, platform storage,
timestamps, random IDs, Mosaic components, or native shell code.

This mirrors the VisiCalc split:

```text
React / Mosaic / SwiftUI / Compose / Flutter / Qt / XAML
        |
        v
engram-wasm / engram-capi                 facade crates, future
        |
        v
engram-core                               cards, reviews, sessions, scheduling
```

The portability bar is deliberately strict: no I/O, no globals, no platform
conditionals, and no unsafe code. Frontends pass timestamps and IDs in through
commands so the core stays deterministic and easy to test.

Serialization derives live behind the optional `serde` feature. Facade crates
such as `engram-wasm` and `engram-capi` can enable that feature when they need
to exchange JSON snapshots with JavaScript or native host code.

## Boundary

This crate owns:

- decks, cards, progress, sessions, and reviews
- Anki-style review scheduling over new, learning, review, and relearning states
- due/new card queue assembly
- card browser search/filter evaluation
- pure state transitions
- derived study stats

`EngramCommand::RateCard` uses `DeckOptions::default()` and routes through the
scheduler state machine. Hosts that already expose deck-specific options can
dispatch `EngramCommand::RateCardWithOptions` to provide learning steps,
graduating/easy intervals, review limits, and lapse behavior without forking the
review logic.

Review-control commands such as `SuspendCard`, `UnsuspendCard`, `BuryCard`,
`UnburyCard`, `SetCardFlag`, `MarkCard`, and `UnmarkCard` also live here. They
hide cards from queues and active sessions or store review metadata in the core
reducer so web, Mosaic, and native shells all share the same behavior.

Reviews carry optional previous/resulting progress snapshots. `UndoLastReview`
uses those snapshots to remove the newest snapshot-backed review in a session,
restore the card's previous progress, adjust session counters, and return an
active session to the reviewed card. Legacy reviews without snapshots are left
unchanged because there is no reliable prior progress to restore.

`search_cards` provides the first shared collection-browser query layer. It
supports plain text terms plus `deck:`, `note:`, `noteType:`, `front:`,
`back:`, `tag:`, `state:`, `is:`, `flag:`, and `marked:` filters with implicit
AND semantics and leading `-` negation.

This crate does not own:

- IndexedDB, SQLite, files, or cloud sync
- React, Mosaic, native widgets, or Paint VM
- ID generation or wall-clock access
- authored language content graphs

The larger language app should add a sibling `language-core` for scripts,
lexemes, grammar concepts, etymology edges, and lesson graphs, while this crate
remains the memory/review engine those learning items can use.
