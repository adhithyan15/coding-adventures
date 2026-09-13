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

It emits **clean** — zero capability degradations on all five backends, and zero
style drops on the three that report them — so the allowlist is empty, and that
emptiness is the assertion rather than an omission: an empty allowlist tolerates
nothing, rather than checking nothing.

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


- Added a light-theme stylesheet (`ReviewActions.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `ReviewActions` Mosaic component package for Anki-style undo, bury,
  suspend, and mark controls.
- The component exposes label slots for localization or host-specific wording
  and emits action events expected by Engram's Rust event bridge.
