# Changelog — engram-core-wasm

## Unreleased

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

