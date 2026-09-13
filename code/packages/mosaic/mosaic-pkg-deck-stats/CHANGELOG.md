# Changelog

## Unreleased

### Fixed — the SwiftUI `gap` pin was licensing a drop that never happened

`(SwiftUI, "gap")` is off `ALLOWED_STYLE_DROPS`. It was never a real drop:
`Column` and `Row` open `VStack(spacing:)` / `HStack(spacing:)` read from the
part's own style. The **report** was wrong — it scanned only the modifier chain,
and `gap` is lowered at view-construction time, where no modifier scan reaches
it.

Left in place the entry was not untidy but load-bearing in the wrong direction:
a standing licence for this package's gaps to genuinely stop being applied with
the gate still green. `no_pinned_style_drop_has_silently_been_fixed` is what
surfaced it, which is the job it was added for.

A gap a `Box`, `Stack` or `HostScroll` really does discard is still reported, so
the entry comes back if one appears here.

### Added — a native-complete gate for this package

`package_compiles.rs` proves the sources round-trip through the three IR
compilers and deliberately says nothing about what a backend can express. So a
capability this package asked for and a backend could not provide was invisible
here — it showed up, if at all, only once Engram composed the package and its
app-level gate noticed.

This package now runs the real degradation analyzer itself, across all five
native backends. A failure names one package, one backend, one property,
instead of reporting one signal for eleven packages at once.

It loses no capability on any backend. It does drop a few style properties:
SwiftUI drops `align`, `border-bottom-style`, `flex-grow` and `gap`; Compose and XAML drop `border-bottom-style`. Those are pinned by property rather than counted, and each is also
asserted to *still* be dropped, so a pin whose gap gets fixed fails the gate
instead of quietly becoming a licence for that property to be dropped again.

**Both themes are analysed, which the sibling gates do not do.** `theme: None`
selects the historical dark-wins default, so a property dropped only by the
light stylesheet is invisible to it. Measured rather than assumed: adding
`flex-wrap` to a light `.msl` left the analyzer reporting nothing, and the
identical line in the dark `.msl` produced two drops. `mosaic-pkg-toolkit`'s and
`engram-app`'s gates both pass `None` and so cover one theme each.

Mutation-tested, not assumed green: a `flex-wrap` added to a light stylesheet
now fails with `SwiftUI (light)` and `Xaml (light)` naming the property and the
reason.

**What the style half does not cover.** Only XAML, SwiftUI and Compose populate
`style_degradations`; Qt and Flutter fall through the analyzer's catch-all arm,
whose own comment says an empty list there means "nobody looked" rather than
"nothing was lost" (#12022). The capability assertion covers all five, because
`collect_native_degradations` is backend-generic. An earlier draft of this entry
said "every backend" of both halves; security review caught it.


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
