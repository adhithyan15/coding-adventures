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
- note-backed Anki Basic, Basic-and-reversed, Cloze, and custom-field TSV import/export
- durable media asset state plus reducer commands for host-managed copy/prune flows
- active review-session progress counts
- deck-scoped review-history summaries
- pure state transitions
- derived study stats

`EngramCommand::RateCard` uses `DeckOptions::default()` and routes through the
scheduler state machine. Hosts that already expose deck-specific options can
dispatch `EngramCommand::RateCardWithOptions` to provide learning steps,
graduating/easy intervals, review limits, and lapse behavior without forking the
review logic.
Deck options also carry Anki-style maximum interval, review interval modifier,
hard interval multiplier, and easy bonus multiplier settings, so hosts can keep
deck-specific scheduler tuning in the shared Rust core.
`EngramCommand::SetDeckOptions` inserts or replaces a stored deck option preset,
letting settings screens update the same options that queue building and
`RateCard` use.

Review-control commands such as `SuspendCard`, `UnsuspendCard`, `BuryCard`,
`BuryCardSiblings`, `UnburyCard`, `SetCardFlag`, `MarkCard`, and `UnmarkCard`
also live here. They hide cards from queues and active sessions or store review
metadata in the core reducer so web, Mosaic, and native shells all share the
same behavior.
Media commands such as `UpsertMediaAsset`, `DeleteMediaAsset`, and
`DeleteMediaAssets` update `AppState.media_assets` in the same reducer, giving
every shell one deterministic place to copy newly attached media, replace
imported payloads, or prune unreferenced assets after a host-side media analysis
pass.

Reviews carry optional previous/resulting progress snapshots, sibling-progress
snapshots, and active-session snapshots. `UndoLastReview` uses those snapshots
to remove the newest snapshot-backed review in a session, restore card progress,
adjust session counters, and return the active session to its pre-review queue.
Legacy reviews without snapshots are left unchanged because there is no reliable
prior progress to restore.

`search_cards` provides the first shared collection-browser query layer. It
supports plain text terms plus Anki-style `w:`, `nc:`, `sc:`, and `re:` text
modifiers, field-scoped regex searches such as `front:re:...`, tag regexes,
`deck:`, `preset:`, `note:`, `noteType:`, `card:`, `cid:`, `nid:`, `front:`,
`back:`, `tag:`, `state:`, `is:`, `flag:`, `marked:`, `prop:`, `added:`,
`edited:`, `introduced:`, and `rated:` filters. `preset:` resolves imported
Anki deck-option preset names from preserved collection metadata, while
`prop:pos` / `prop:position` uses imported Anki new-card queue positions when
available. Imported Anki card custom data can be searched with `has-cd:key`,
numeric `prop:cdn:key>5`, and scalar string `prop:cds:key=value` filters.
Imported Anki queue metadata also powers `is:buried-manually` and
`is:buried-sibling`. Terms inside a group use implicit AND, `OR` joins groups,
parentheses group subexpressions, and leading `-` negates a term or group.

`materialize_generated_card` turns a note-template `GeneratedCard` into a
durable `Card` with note/template lineage. Cloze templates using
`{{cloze:Field}}` or filtered forms such as `{{type:cloze:Field}}` generate one
card per Anki-style `{{c1::text::hint}}` ordinal, render sections and
`FrontSide`, expose Anki-style special fields such as `Tags`, `Type`, `Deck`,
`Subdeck`, `Card`, `CardFlag`, and `CardID`, and preserve the cloze ordinal in
lineage. Templates can require all listed fields or any one listed field,
matching Anki model `req` rules for optional card generation.
`BuryCardSiblings` uses lineage to bury same-note sibling cards until a
host-supplied boundary.
`RateCardAndBurySiblings` and `RateCardWithOptionsAndBurySiblings` apply that
behavior atomically during review and record undo snapshots, matching the shared
behavior Anki-like review screens need.
`EngramCommand::UpsertNoteType` can insert or replace note types and optionally
resync generated cards for existing notes of that type, while
`EngramCommand::DeleteNoteType` removes the note type, its notes, and only their
lineaged generated cards.
`EngramCommand::UpsertNote` can optionally materialize generated cards from the
note type while preserving progress for stable generated card IDs, and
`EngramCommand::DeleteNote` cascades only the note's lineaged generated cards.
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
lineage cards, Basic-and-reversed note types produce forward and reverse
sibling cards, and Cloze rows produce cloze note models plus one generated card
per cloze ordinal. Custom note-type rows preserve arbitrary field columns and
Anki's Tags column as note data, but generate no cards until a real template is
available because Anki text exports do not carry template definitions.

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
