# Changelog — engram-core-wasm

## Unreleased

### Fixed -- `onSaveNote` accepted a deck or note type the collection does not contain

**Three** routes build a `Note` from partly-external input and hand it to
`UpsertNote`: the editor draft, the `onSaveNote` event payload, and the raw
`upsertNote` command. The editor route refused a note type that does not exist,
and (since the id-hardening pass) a deck that does not exist. The other two
checked **neither**.

The third is the least guarded and the most exposed: `upsertNote` carries a
whole `Note` straight from the caller, and it is a documented public surface --
the crate README describes it, the web host declares it in its `.d.ts`, and
`eg_dispatch` exports it to every native shell.

So the invariant "a note belongs to a deck that exists" held on one path and not
the other. On the unchecked path the note *and its generated cards* land in a
deck no deck list can reach, with the editor rendering a selected-deck index of
-1, so the UI cannot say where they went.

All three routes now share `validate_note_target`. The reducer would be the tempting
home for it, since every write passes through `reduce` -- but `reduce` returns
`AppState`, not `Result`, so the only thing it could do with a bad id is drop
the note silently, trading a visible wrong answer for an invisible one. The rule
lives at the two points that can still report an error, and a test pins both so
they cannot drift apart again.

### Scope, stated so it is not read as more than it is

This closes the three `UpsertNote` routes. It is **not** a global invariant on
`AppState`. `loadState`, `load_snapshot`, `import_backup`, `merge_app_states`,
and the `.apkg`/TSV importers can all still introduce a note or card naming a
deck that does not exist, and `rebuild_filtered_deck`, `createCard`, and a
per-template `deckId` can each write one directly. Those are tracked separately
(#14531, #14532, #14533); the merge paths matter most, because they fold
untrusted file content into an existing collection rather than replacing it.

An empty deck id means "inherit the existing note's deck" for the two callers
that resolve inheritance before validating, and is **refused** for the raw
`upsertNote` command, which has no such step -- the reducer stores the `Note`
verbatim, so empty there means "belongs to no deck". Sharing one rule across
callers whose empty case means different things is how a guard becomes a hole:
it let `deckId: ""` store exactly the orphan this change prevents, and blank an
existing note's real deck besides -- but the
`onSaveNote` arm now resolves the caller's deck argument the way every sibling
arm does, instead of passing it through raw. It did not before, so a **new**
note saved with no deck named reached the inherit exemption with an empty id and
was stored with `deck_id: ""`, along with the cards generated from it: absent
from every deck's queue and stats, and not reached by `DeleteDeck`'s cascade.
The exemption is for inheriting a real deck, not for having none.

### Changed -- `onSaveNote` and `upsertNote` now require the note type to exist

Previously a note naming a not-yet-created note type was stored with zero cards,
and a later `upsertNoteType` with `materializeCardsAt` would retroactively
generate them. A host that streams notes before models will now get an error
instead. Nothing in this repository does that -- the shipped UI emits
`onNoteEditorSaveNote`, and the `.apkg` and TSV importers build `AppState`
directly without going through `UpsertNote` -- but it is a public contract
change and is recorded as one.

### Fixed -- three ways a collection could name something it does not contain

Each of these is a place where an id was used verbatim where the sibling code
path checked it, so a collection or draft naming something absent wrote to a
phantom target rather than being refused.

**A note could be saved into a deck that does not exist.**
`note_from_editor_selection` validated the note type and not the deck, though
both come from the same editor draft. The editor path upheld it on its own --
`NoteEditorSelectDeck` resolves an *index* into `state.decks` -- so nothing
reached it today, but the note *and its generated cards* would have landed in a
deck no deck list can reach, with the editor rendering a selected-deck index of
-1 so the UI could not say where they went. An empty id still means "inherit the
existing note's deck" and is left alone.

**A phantom `active_session.deck_id` resolved to itself.**
`selected_deck_id_with_override` filtered its override argument against
`state.decks` but returned `active_session.deck_id` unchecked, and
`active_session` is part of `AppState` -- so a collection naming a deleted deck
resolved to that phantom, which then became the target of writes such as
`rebuild_filtered_deck`. In-app this is a no-op, since `DeleteDeck` clears
`active_session` when it targets the deleted deck; it matters for a collection
that arrives from outside. It now falls through to the first real deck, which is
what already happens when there is no active session.

**A "new" draft could adopt an existing id and overwrite it.** `draft_is_new`
and the draft's id are set together by `start_new`, which mints a fresh id, so
they cannot disagree by any route through the event surface. Set independently,
`draftIsNew` beside an id something already holds turned "create" into
"overwrite": for a note, replacing its fields and deck; for a note type, saving
the blank two-field `default_note_type_model` over a real one, discarding its
fields and templates and breaking every note built on it. A new draft now keeps
its freshly minted id when the supplied one is taken. An id nothing holds is
still honoured, so a caller can still choose its own.

### Fixed — deleting a note or note type was a silent no-op (#13933)

The user clicked delete and nothing happened: no deletion, no error, no
message. Two halves that were individually reasonable and wrong together --
the branch that minted a `deleteNote` host intent was exactly the branch that
mutated nothing, and **no adapter has ever handled that intent**. All seven
fall through a `default:` branch.

Three changes:

- The delete resolves the **currently selected** note or note type when the
  event carries no id, through the same helper `NoteEditorDeleteNote` already
  used. The Collection panel's button emits no payload, so before this it could
  never identify anything -- the click was a no-op every single time.
- When nothing can be resolved it returns an error instead of doing nothing,
  matching `NoteEditorDeleteNote` rather than being the one destructive action
  that fails quietly.
- New `collection-delete-note-disabled` / `collection-delete-note-type-disabled`
  props, so the control is withdrawn rather than offering a click that would
  fail. Computed in the engine because it is a property of the state and every
  backend renders `disabled` already -- a host-side answer would be written
  seven times and kept in step.

The intents are no longer minted: an intent alongside an error is a request to
a host that is also a failure, and restoring one belongs to UI47's `Await`
effect, which is the real confirmation round trip.

A test asserted the old behaviour -- `ok: true`, an intent, an unchanged note
count -- which is to say it pinned the bug as expected. It now pins the fix.

- `deck-rows` now exposes per-deck due and new counts as pre-formatted strings
  (`"1 due"`, `"1 new"`, or empty when the count is zero), computed with
  `get_deck_stats_for_state`. The counting stays in the engine so all five
  backends show the same numbers without reimplementing the arithmetic; a deck
  with nothing waiting renders an empty column so the decks that do have work
  are the ones that catch the eye.
- `onSelectDeck` accepts the `index` payload the deck list now emits. Selecting
  by name still works -- the event surface is also the scripting surface, and a
  name survives a reordering where a position does not.

**APKG import and export now run on wasm.** `export_anki_apkg` and
`merge_anki_apkg` returned `"Anki APKG export/import is handled by native hosts
for WASM shells"` on `wasm32`, because `Cargo.toml` excluded
`engram-anki-package` from that target entirely — it linked bundled C SQLite and
libzstd, neither of which can target `wasm32-unknown-unknown`.

The package layer is now a dependency on every target: with default features
natively, and with `default-features = false` on wasm, which drops zstd and keeps
full legacy `.apkg` support. Both stubs are gone and both methods run the real
implementation everywhere.

This closes the one documented hole in the Mosaic host contract — the generated
HTML, WebComponent, and React hosts wired up file input and download helpers and
then had to surface a delegation error.

