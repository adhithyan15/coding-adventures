# Changelog

## Unreleased

### Fixed — the SwiftUI `gap` pin was licensing a drop that never happened

`ALLOWED_STYLE_DROPS` is now empty; `(SwiftUI, "gap")` was its only entry and it
was never a real drop. `Column` and `Row` open `VStack(spacing:)` /
`HStack(spacing:)` read from the part's own style. The **report** was wrong — it
scanned only the modifier chain, and `gap` is lowered at view-construction time,
where no modifier scan reaches it.

**An empty allowlist made both existing gates one-sided**, so a third test
anchors them. The unexpected-drop check iterates the *observed* drops, so a
package with no drops passes it; the stale-pin check iterates the empty list.
Neither could tell "SwiftUI applies every gap this package authors" from "this
package authors no gap" or from "the analyzer stopped reporting".

`the_package_authors_a_gap_for_the_gate_above_to_be_about` reads the two `.msl`
sheets and requires a `gap` to be declared in each. It reads the **source**
rather than the report on purpose: a gate that asked the analyzer whether the
analyzer had work to do would be answered by the component that might have
stopped working.

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
SwiftUI drops `gap`. Those are pinned by property rather than counted, and each is also
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



- Added `delete-note-disabled` / `delete-note-type-disabled`, wired to the two
  destructive buttons. A delete with nothing selected used to be a silent
  no-op (#13933); the control is now withdrawn rather than offering a click
  that does nothing.

- Added a light-theme stylesheet (`CollectionActions.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Add `CollectionActions`, a target-neutral collection status and workflow
  action surface for Mosaic-generated study apps.
