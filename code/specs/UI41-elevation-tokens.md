# UI41 — `elevation`, a typed native-shadow-intent token

**Status:** mosstyle contract and XAML implemented; Compose/Qt/Flutter
not yet — see §4.
**mosstyle surface:** one new property, `elevation`, validated in
`mosstyle-compiler::validate` — the compiler's first property with a
restricted (enum-shaped) value set instead of a freeform string.

---

## 1. The gap

Twenty real `box-shadow` declarations, across four components' light and
dark stylesheets (`TaskApp`, `ProjectNav`, `Notes`, `Calendar`), are
silently dropped on every native backend except XAML:

```
.msl:  box-shadow ×20 (non-inset)
XAML:  ThemeShadow ✓ (heuristic — see below)
Qt/Compose/Flutter/SwiftUI:  box-shadow ×0
```

XAML already ships a partial fix (`part_wants_theme_shadow`,
`mosaic-emit-xaml/src/pipeline.rs`): any part whose `box-shadow` value
doesn't contain `inset` gets a fixed-depth `ThemeShadow`. That works —
but it's a heuristic inferred from a freeform CSS property never meant
to express platform-native shadow semantics, not a real design
decision, and every other backend has nothing to infer from at all.

## 2. Why a typed property, not another per-backend heuristic

Native shadow primitives don't take CSS-shaped parameters. WinUI's
`ThemeShadow` has no blur/spread/color/opacity controls at all — it's a
system-composited shadow driven purely by a Z-*depth* value. Compose's
`Modifier.shadow`/`Surface` elevation and Qt's `MultiEffect` are
similarly parameter-light compared to CSS `box-shadow`'s six-value
syntax. Every real `box-shadow` value in this codebase already reduces
to *one visual intent* — "this part should look raised off the
surface" — expressed as two near-identical rgba pairs that differ only
by light/dark theme, never by genuinely different elevation strength.
Continuing to sniff intent out of a CSS shadow string, one backend at a
time, means every backend re-derives the same intent from a
representation that was never designed to carry it. A typed property
states the intent once, in the `.msl` source, and lets every backend
map it to its own native primitive directly.

`elevation` is deliberately **additive**, not a replacement for
`box-shadow`: a styled part keeps its existing `box-shadow:` (which
still drives correct CSS rendering on the three web backends
unmodified) and gains a new `elevation:` declaration alongside it, read
only by native backends. `box-shadow`'s other, unrelated use — an
`inset` value as a decorative shape-cutout hack (the theme-toggle
crescent-moon icon) — is untouched; `elevation` has nothing to say
about it.

## 3. The property

Legal on any styled part, in the base style or any state override.
Exactly two legal values:

| Value | Meaning | Current real usage |
|---|---|---|
| `raised` | A static element sitting slightly above the surface — a card, a selected row/tab, a static panel. | All 20 real (non-`inset`) `box-shadow` declarations in the codebase today (Kanban cards, task rows, selected nav/tab buttons, static form-row containers, the Calendar month panel). |
| `overlay` | An element floating above everything else — a modal, dialog, popover, tooltip, dropdown, toast. | None yet — `Modal`/`Dialog`/`Sheet`/`Tooltip`/`DropdownMenu`/`Toast` `.msl` files declare no shadow at all today. Included now so that future work on those components doesn't need another grammar cascade. |

```msl
part task-card {
  box-shadow: "0 1px 2px rgba(60,45,25,.05), 0 4px 14px rgba(60,45,25,.05)" ;
  elevation:  raised ;
}
```

Any other value is a hard compile error
(`ErrorKind::InvalidPropertyValue`), naming the bad value and the two
legal ones — the same diagnostic shape `validate()` already uses for
`ErrorKind::UnknownPart`/`UnknownState`. A part with no `elevation`
declared at all gets no native shadow treatment on any backend — the
same as today's behavior; there is no implicit default.

No grammar changes were needed. mosstyle's grammar already parses every
`property: value;` generically (`_grammar.rs`'s single
`property_decl` rule) — `raised`/`overlay` lex and parse as plain
`NAME` tokens exactly like `flex-start` or `monospace` already do.
`elevation`'s "typed-ness" is entirely a post-parse semantic check in
`validate()`; the `StyleProp`/`PartStyleMap` plumbing every backend
already uses to look up any other property's value needed zero changes.

## 4. Rollout

Sequenced like `Path`/`HostProgressRing`: one contract PR (this spec +
`validate()` value-checking + the real `.msl` migration — no backend
rendering change), then one PR per backend.

| Backend | Status |
|---|---|
| mosstyle-compiler | **implemented** — `elevation: raised \| overlay;` validated in `validate()`; the 8 real `.msl` files that declare a non-`inset` `box-shadow` now also declare `elevation: raised` alongside it. |
| XAML | **implemented** — replaces `part_wants_theme_shadow`'s box-shadow-value-sniffing with a direct read of the `elevation` prop (`part_elevation_tier`, an `ElevationTier` enum), reusing the existing, already-shipped `ThemeShadow`/`Translation` mechanism (`raised` keeps `Translation="0,0,4"`; `overlay` gets a deeper `Translation="0,0,16"`). `Box`/`Stack`/`Row`/`Column`/`HostButton` all apply `Translation`/`.Shadow` directly via the usual XAML attribute/property-element syntax — but `HostDraggable` cannot: a real `dotnet build` probe found that syntax fails XamlCompiler (exit code 1, no diagnostic) specifically on a custom `ContentControl` subclass like `{component}MosaicDragSource`, so its Z-depth instead flows through a new `ElevationZ` string `DependencyProperty` (the same shape `DragKey` etc. already use) whose changed-callback applies `Translation`/`Shadow` from C#. Found and fixed via real full-package verification: an initial pass only wired `Box`/`Stack`/`HostButton`, and compiling the real TaskApp package and counting rendered shadows (9 of 13) revealed `Row`/`Column` (`emit_flex_grid`) and `HostDraggable` never called the shadow helper at all. Verified against the real toolchain: a `mosaic-compile pkg --backend xaml --profile native-complete` build of the real TaskApp package confirms `nativeComplete: true` with zero `elevation`/non-inset-`box-shadow` degradations; a real `mosaic-compile pkg --backend xaml --emit-project` build of the actual TaskApp WinUI project (all 13 real `elevation` parts) passed a real `dotnet build` with zero errors. |
| Compose | not yet — `Modifier.shadow(elevation: Dp, ...)`; this crate already hardcodes `elevation = 8.dp`/`4.dp` on two unrelated components (Card, tooltip `Surface`), confirming the primitive — needs a real `gradle compileKotlin` probe against the pinned Material1 version before implementation. |
| Qt | not yet, and the one real research risk — Qt 6.5+'s `QtQuick.Effects` `MultiEffect` (`shadowEnabled`/`shadowBlur`/`shadowColor`/`shadowVerticalOffset`) is the modern non-deprecated path; needs its own real-toolchain spike (`qmllint` + a live `qml` window, then re-verified against the *actual* `mosaic-compile --backend qt` output — the exact two-pass discipline that caught two real runtime bugs during `HostProgressRing`'s Qt PR). |
| Flutter | not yet — `BoxDecoration(boxShadow: [BoxShadow(...)])`; standard, well-documented stable API, lower research risk than Qt but still verified via `dart analyze` and a live widget mount before landing. |
| SwiftUI | not yet — tracked separately in #13206 (genuinely unbuildable on this dev box, no macOS/Xcode environment for real verification, same blocker as every other primitive in this cascade). |
| react, html, webcomponent | not in scope — these backends already render the unmodified `box-shadow:` CSS correctly today; `elevation` is purely the new native-semantic-intent channel and web backends never read it. |

Until a backend implements `elevation`, a part with `elevation: raised`
(or `overlay`) but no native lowering renders with no shadow on that
host — a reported degradation once each backend's degradation arm is
wired up in its own PR (matching `Path`/`HostProgressRing`'s posture
for unimplemented backends), not silent.
