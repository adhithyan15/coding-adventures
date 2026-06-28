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
- review-log-aware daily limit accounting
- optional note/template lineage on durable cards
- Anki-style Cloze note generation from `{{cloze:Field}}` templates
- note-type field rename migration for templates and required fields
- card browser search/filter evaluation
- versioned Engram JSON backup snapshots
- round-trippable card CSV import/export helpers
- Anki-compatible Basic TSV card export
- note-backed Anki Basic and Basic-and-reversed TSV import/export
- active review-session progress counts
- deck-scoped review-history summaries
- pure state transitions
- derived study stats

`EngramCommand::RateCard` uses `DeckOptions::default()` and routes through the
scheduler state machine. Hosts that already expose deck-specific options can
dispatch `EngramCommand::RateCardWithOptions` to provide learning steps,
graduating/easy intervals, review limits, and lapse behavior without forking the
review logic.

Review-control commands such as `SuspendCard`, `UnsuspendCard`, `BuryCard`,
`BuryCardSiblings`, `UnburyCard`, `SetCardFlag`, `MarkCard`, and `UnmarkCard`
also live here. They hide cards from queues and active sessions or store review
metadata in the core reducer so web, Mosaic, and native shells all share the
same behavior.

Reviews carry optional previous/resulting progress snapshots. `UndoLastReview`
uses those snapshots to remove the newest snapshot-backed review in a session,
restore the card's previous progress, adjust session counters, and return an
active session to the reviewed card. Legacy reviews without snapshots are left
unchanged because there is no reliable prior progress to restore.

`search_cards` provides the first shared collection-browser query layer. It
supports plain text terms plus `deck:`, `note:`, `noteType:`, `front:`,
`back:`, `tag:`, `state:`, `is:`, `flag:`, and `marked:` filters. Terms inside
a group use implicit AND, `OR` joins groups, and leading `-` negates a term.

`materialize_generated_card` turns a note-template `GeneratedCard` into a
durable `Card` with note/template lineage. Cloze templates using
`{{cloze:Field}}` generate one card per Anki-style `{{c1::text::hint}}`
ordinal and preserve the cloze ordinal in lineage. `BuryCardSiblings` uses
lineage to bury same-note sibling cards until a host-supplied boundary,
matching the shared behavior Anki-like review screens need.
`rename_note_type_field` and `EngramCommand::RenameNoteTypeField` keep field
IDs stable while migrating template references, Cloze references, and
required-field names to the new display name.

`create_engram_snapshot` and `restore_engram_snapshot` own the portable Engram
backup shape. Backups include durable collection data and clear any live
`active_session` when restored.

`export_cards_csv` and `import_cards_csv` provide a strict card CSV round-trip
format with the header `id,deckId,front,back,createdAt`. `import_basic_cards_csv`
accepts simpler `front,back` rows and uses host-supplied deck/timestamp/ID
prefix options to create deterministic cards.
`export_cards_anki_basic_tsv` and `import_anki_basic_tsv` cover Anki Basic
front/back text files with import headers (`#separator:tab`, `#html`,
`#notetype`, `#deck`, and `#columns`) and quoted fields containing tabs,
newlines, or quotes.
`import_anki_notes_tsv` and `export_notes_anki_tsv` use the note/template model
instead: imported Basic rows produce `NoteType`, `Note`, and materialized
lineage cards, while Basic-and-reversed note types produce forward and reverse
sibling cards. The note-backed path also preserves Anki's Tags column as note
tags.

`get_active_session_progress` derives the shared review UI counters from
`AppState`: total cards, one-based current position, remaining cards, reviewed
and correct counts, reveal state, and completion. Hosts should render these
counts instead of recomputing progress differently per platform.

`get_daily_study_limit_usage` derives how many new-card introductions and
review reps have already happened inside a host-provided day window.
`build_session_queue_with_daily_limits` uses that usage to subtract from
`DeckOptions` before assembling due and new cards, so daily limits are shared
across web, Mosaic, and native shells.

`get_deck_stats` includes due, new, learning, mastered, suspended, and buried
counts so deck overviews and collection browsers can render hidden-card state
from the same core calculation.

`summarize_review_history` derives deck-scoped review history for a timestamp
range from the durable review log. It returns total reviews, correct reviews,
unique reviewed cards, per-rating counts, and first/last review timestamps so
web and native shells can render stats without reimplementing log aggregation.

This crate does not own:

- IndexedDB, SQLite, files, or cloud sync
- React, Mosaic, native widgets, or Paint VM
- ID generation or wall-clock access
- authored language content graphs

The larger language app uses the sibling `language-core` crate for scripts,
lexemes, grammar concepts, etymology edges, and lesson graphs, while this crate
remains the memory/review engine those learning items can use.
