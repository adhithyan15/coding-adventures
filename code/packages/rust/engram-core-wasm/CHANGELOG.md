# Changelog — engram-core-wasm

## Unreleased

### Fixed -- `createCard` and `startSession` accepted a deck the collection does not contain (#14533)

Both reach `reduce` raw through `dispatch` -- a documented public surface: the
README describes it, the web host declares it, and `eg_dispatch` exports it to
every native shell. `reduce` returns `AppState` and cannot refuse anything, so
nothing checked the deck.

The session is the more serious of the two. `DeleteDeck` selects sessions by
`deck_id`, so one naming a deck that never existed can never be selected and
never removed -- the same permanently-undeletable shape as the importer bug in
#14559.

The check is written as a **match over every command variant that carries a deck
id**, not a list of the ones known to be broken. Each earlier fix in this family
patched the routes someone had already found, and each time there was another;
enumerating the surface is what stops that, and a new variant carrying a deck id
now has to be decided about rather than defaulted.

Two are deliberately exempt, and both are pinned by tests so the choice cannot
be widened by accident:

- `updateDeck` and `deleteDeck` mutate or remove only what they find, so neither
  can create a dangling reference. Refusing them would turn a harmless no-op
  into an error for a caller that deletes twice -- which a retry, or two hosts
  on one collection, will do.
- `setDeckOptions` creates *configuration*, not content. A preset for a deck
  that does not exist is inert, since options are read while scheduling that
  deck's cards and it has none. It does outlive `DeleteDeck`, which cleans
  cards, notes, sessions and reviews but not presets -- that is worth fixing by
  making the deletion complete (#14576) rather than by refusing the write.

### Fixed -- rebuilding a filtered deck that does not exist emptied the collection (#14531)

Rebuilding a filtered deck *moves cards out of the decks they are in* and into
the named one, and neither route that does it checked that the named deck
exists. The facade method had no deck guard at all; the event arm rejected an
**empty** id and nothing else, so a selected deck that had since been deleted
went straight through.

This is the most damaging member of the deck-id family and the only one that
needs no crafted input, because an empty search matches everything:

    ok        = true
    decks     = ["tamil-script", "hindi-devanagari", "kannada-script", "spanish-latin-roots"]
    before    = ["tamil-script", "tamil-script", "hindi-devanagari", ...]
    after     = ["deck-that-does-not-exist", ... x5]
    MOVED     = 5 of 5

Every card in the collection, out of its deck and into one that does not exist
-- absent from every queue and stat, and never reached by `DeleteDeck`'s
cascade, so unrecoverable from inside the app. It returned `ok: true`.

The asymmetry was that **emptying** a filtered deck already refused to restore a
card into a deck that is gone, while **rebuilding** did not. The two *rebuild*
routes now share `validate_filtered_deck_target` -- emptying still needs no
guard, since its restore loop only touches cards already carrying the named
deck and skips any whose original deck is missing, so a phantom id there either
no-ops or repairs an orphan.

The tests assert that no card moved, not merely that an error came back: a guard
that errored after mutating would satisfy the weaker check and still have
wrecked the collection.

The refusal is also enforced at the point of mutation, in `engram-core` -- see
that crate's entry. This layer exists to make it *visible*.

### Added -- the presentation cursor survives snapshot/restore (#13646)

`snapshot()` serialised `AppState` and nothing else, and `load_snapshot` reset
`selected_deck_id`, `browser`, `review`, `editor`, and `note_type_editor` to
`default()`. Reopening Engram silently put you back at the deck list: the deck
you had chosen, the screen you were on, the search you had typed, and how far
into a review you were all went away, with no error to say so.

The mechanical cause was that those types derived `Clone, Debug, PartialEq, Eq`
but not `Serialize`/`Deserialize`, so there was nothing to put in a snapshot even
if one had wanted to.

Now they do, and there is a `PresentationCursor` bundling them with the active
screen. Note the issue named five types; `active_screen` is a sixth. It was never
reset by `load_snapshot` -- but it was never *serialised* either, so it did not
survive a restore any more than the other five did, and leaving it out would have
restored the search box while losing the screen it belongs to.

Two new facade calls carry it:

- `session_snapshot()` -> `{"ok": true, "session": {"state": ..., "cursor": ...}}`
- `load_session_snapshot(json)`, where a missing `cursor` is not an error --
  that is what a pre-cursor snapshot looks like, and it loads with a fresh one.

The cursor is parsed separately from the collection, so a cursor that will not
parse costs a scroll position rather than the whole collection.
`#[serde(default)]` alone would not do this: it fills fields that are *missing*,
while a field that is present and malformed -- an unknown `activeScreen`, a
`null` cursor -- aborts the entire document and would take `state` with it.
Refusing to open someone's collection because their saved scroll position is
corrupt is the wrong trade, and the same one this crate already refuses when it
accepts a cursor-less snapshot.

`snapshot()` and `load_snapshot()` are **unchanged**, deliberately. `eg_snapshot`
and `eg_load_snapshot` are in the published C header and all five native hosts
bind them, and `load_snapshot` is a `engram-wasm` export besides; moving their
payload would have been a silent ABI break for every shell. They remain the
collection-only pair, which is also the right shape for a backup or a sync
payload, where a half-typed search box is noise.
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

