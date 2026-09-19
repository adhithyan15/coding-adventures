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

It remains **capability-clean** on all five backends. Style-drop reporting now
also covers all five: measured pre-existing Qt and Flutter losses are pinned by
backend/property, while every unlisted loss still fails and every stale pin is
rejected.

**Both themes are analysed, which the sibling gates do not do.** `theme: None`
selects the historical dark-wins default, so a property dropped only by the
light stylesheet is invisible to it. Measured rather than assumed: adding
`flex-wrap` to a light `.msl` left the analyzer reporting nothing, and the
identical line in the dark `.msl` produced two drops. `mosaic-pkg-toolkit`'s and
`engram-app`'s gates both pass `None` and so cover one theme each.

Mutation-tested, not assumed green: a `flex-wrap` added to a light stylesheet
now fails with `SwiftUI (light)` and `Xaml (light)` naming the property and the
reason.

The style half now covers all five native backends. Qt gained real lowering-read
recording in #15245 and Flutter in #12022; their measured pre-existing losses
are pinned by backend/property, and the inverse-ratchet test rejects stale pins
as those gaps close.


- Added a light-theme stylesheet (`CardBrowser.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).

- Added browser result metadata slots for card IDs, note IDs, template IDs,
  state labels, and selected-row values so host apps can target browser
  actions without parsing labels.
- Added the initial `CardBrowser` Mosaic component package for reusable Anki-style card browser/search surfaces.
