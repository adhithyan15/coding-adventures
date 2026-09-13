# Changelog

## Unreleased

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
