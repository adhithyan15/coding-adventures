# Changelog

## Unreleased

- Turned the deck list into a table. `deck-names : list<text>` became
  `deck-rows : list<list<text>>`, each row carrying `[ name, due, new ]`, and
  the counts now sit in fixed-width right-aligned columns with a hairline rule
  between rows. The previous row of variable-width chips put every count in a
  different horizontal position, so the numbers could not be scanned -- which
  defeats the one question a deck list exists to answer.
- **Breaking:** `onSelectDeck` now carries `index : number` rather than
  `value : text`. A row renders three strings and MLL has no way to hand an
  emit a computed value like `row[0]`, so the click carries the row's position
  and the engine resolves it against the deck list it built the rows from.

- Added a light-theme stylesheet (`DeckStatsPanel.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `DeckStatsPanel` Mosaic component package for deck-scoped total,
  new, due, learning, and hidden review counters.
- The component exposes label/value slots so hosts can bind shared Engram core
  deck-stat JSON without target-specific layout forks.
