# UI61 — a scroll viewport has an axis

Issue: [#14854](https://github.com/adhithyan15/coding-adventures/issues/14854)

Extends [UI29](UI29-primitive-kernel.md)'s primitive kernel. `HostScroll` is
defined as a "scrollable viewport" with **no axis**, and the eight backends
resolve that silence differently. A second, parallel route — `overflow` in the
`.msl` — reaches only three of them.

## 1. The defect

Measured, one minimal `HostScroll` component emitted on every backend and each
output file read:

| backend | lowers to | scrolls |
| --- | --- | --- |
| html | `overflow: auto` | both axes |
| react | `overflow: "auto"` | both axes |
| webcomponent | `overflow: auto` | both axes |
| Qt | `ScrollView` — no policy set | both, from the QML default; not measured at runtime |
| **XAML** | `ScrollViewer VerticalScrollBarVisibility="Auto" HorizontalScrollBarVisibility="Disabled"` | **vertical only, explicitly** |
| **Compose** | `.fillMaxSize().verticalScroll(..)` | **vertical only** |
| **SwiftUI** | `ScrollView { }` | **vertical only** (its default) |
| **Flutter** | `SingleChildScrollView` | **vertical only** (its default) |

Four backends scroll one axis, three scroll two, and Qt is whatever QML's
default is. Nothing in UI29 says which is correct, so no emitter is wrong — the
kernel simply does not say.

XAML is worth singling out. It does not fall into vertical-only by a toolkit
default: it writes `HorizontalScrollBarVisibility="Disabled"` explicitly.
Someone decided the axis there, once, in one emitter, and nowhere else. That is
the strongest evidence available that `vertical` is the intended meaning — and
also that the decision was made in the wrong place.

This is the same shape as UI60: a primitive whose meaning was left to each
emitter's idea of the obvious.

## 2. The second route, which is worse

`overflow` is also authorable in the `.msl`, and VisiCalc uses **that** rather
than `HostScroll`:

```
part sheet-frame { flex: 1; min-height: 0px; overflow: auto; height: 60vh; … }
```

Measured on a minimal `overflow: auto` part:

| lowers it | drops it |
| --- | --- |
| html, react, webcomponent | **SwiftUI, Qt, Flutter, XAML, Compose** |

So there are two ways to ask for a scroll viewport, they reach different sets
of backends, and their intersection — the three web backends — is the only
place either is reliable. VisiCalc picked the one that reaches no native
backend at all, which is why its sheet cannot scroll anywhere it ships.

The visible consequence is #14842: at 1280px, VisiCalc's columns Q–Z measure
`0 x 24` at origin `(0, 0)`. Ten columns present in the semantics tree,
correctly named, occupying no space, with every accessibility gate passing on
them.

## 3. What must be decided

### 3.1 Does `HostScroll` gain an axis, or does `overflow` become the answer?

They are not equivalent. `HostScroll` is a **primitive** — it changes the
element the emitter produces. `overflow` is a **style property** — it modifies
one. On Compose a scroll is a modifier either way; on SwiftUI, Flutter and XAML
it is a different widget, which a style property cannot conjure.

That asymmetry decides it: **the axis belongs on `HostScroll`**. A style
property that must sometimes replace the element it styles is not a style
property.

### 3.2 What is the default axis?

`vertical`. Four backends already behave that way, one of them (XAML) by an
explicit `Disabled` rather than a toolkit default, which is a decision already
taken. A default of `both` would silently add a horizontal scrollbar to every
existing `HostScroll` on those four — a visible change to shipped apps for no
authored reason.

### 3.3 What happens to `overflow` in the `.msl`?

It should become a **reported degradation on every backend**, including the
three that honour it today. Reporting it where it works is the unusual part and
the important one: an author who writes `overflow: auto` and sees it work on
the web has no way to learn it reaches nothing native. Silence there is what
produced VisiCalc's sheet.

Measured on the same probe, three of the five backends that drop it already
report it:

| reports the drop | silent |
| --- | --- |
| Compose, SwiftUI, XAML | **Qt, Flutter** |

Qt and Flutter have no style-drop reporting at all, which is
[#12022](https://github.com/adhithyan15/coding-adventures/issues/12022) rather
than anything specific to `overflow` — so "appears in every backend's drop
report" partly depends on that landing, and the acceptance below says so.

Retiring the three working lowerings is out of scope here and needs its own
migration; this spec only requires that the property stop being silent.

## 4. The rule

> `HostScroll` declares the axis it scrolls: `vertical` (default),
> `horizontal`, or `both`. Every backend must scroll exactly those axes — no
> more, because a scrollbar the author did not ask for is a defect, and no
> less.

## 5. Hazards already measured

These are not speculative; each was found while investigating #14842.

1. **`fillMaxWidth` inside a Compose `horizontalScroll` is a crash, not a
   layout bug.** A horizontal scroll measures its content against an infinite
   max width, and `fillMaxWidth` against infinity throws. The emitter prepends
   `fillMaxWidth` to most containers by default, so `axis: horizontal` requires
   suppressing that default throughout the scrolled subtree — not just on the
   scroll container itself.

2. **A scroll needs a bounded viewport.** `.verticalScroll(..)` alone left the
   container wrapping its content in #14798, which is wrong twice: a scroller
   sized to its own content has nothing to scroll within, and whatever the
   content does not cover stays unpainted. The fix was `.fillMaxSize()` first.
   The horizontal case needs the same, and the two must compose.

3. **`height: 60vh` on VisiCalc's frame is also dropped**, so it has no bounded
   height either. #14837 mapped `100vh` to `fillMaxHeight()`; a fraction wants
   `fillMaxHeight(0.6f)` and is not covered.

## 6. Acceptance

- A `HostScroll` with `axis: horizontal` scrolls horizontally and **not**
  vertically, asserted on the semantics tree's scroll ranges rather than on the
  emitted source.
- The default stays `vertical`, asserted by a test that would fail if the
  default changed — existing `HostScroll` output must be byte-identical.
- VisiCalc's columns Q–Z report a **non-zero width**. Asserting a width rather
  than presence is the point: `0 x 24` satisfies every existence and
  accessibility assertion in the current suite, which is how ten missing
  columns went unnoticed.
- No Compose build throws on an infinite-width measurement — verified by
  rendering, not by compiling. The generated Kotlin compiles either way.
- `overflow` appears in the drop report of every backend that discards it.
  Compose, SwiftUI and XAML already do; Qt and Flutter have no style-drop
  reporting at all, so those two are gated on #12022 rather than on this spec.

## 7. Out of scope

Retiring `overflow: auto` on the three backends that honour it. Making it
*reported* is in scope; removing it is a migration with its own consumers.
