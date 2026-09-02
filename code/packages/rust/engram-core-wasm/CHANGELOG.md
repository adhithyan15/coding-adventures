# Changelog — engram-core-wasm

## Unreleased

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

