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

XAML is worth singling out, and this spec originally understated it. XAML does
not merely write `HorizontalScrollBarVisibility="Disabled"` explicitly — it had
already implemented **the entire three-way axis**, vertical/horizontal/both,
reading a backend-private prop named `direction`:

```rust
let (v_vis, h_vis) = match find_prop_keyword(node, "direction") {
    Some("horizontal") => ("Disabled", "Auto"),
    Some("both") => ("Auto", "Auto"),
    _ => ("Auto", "Disabled"), // default: vertical
};
```

No `.mll` in the repo ever wrote `direction`, and the other seven backends
could not see it. So the feature this spec proposes existed, worked, was
unreachable, and was invisible — which is a sharper statement of the defect
than the one this section opened with. It is also the strongest evidence that
`vertical` is the intended default: the one emitter that thought about it
chose vertical.

The implementation therefore *deletes* `direction` rather than keeping it as an
alias. Nothing authored it, so nothing breaks, and keeping it would preserve
exactly the backend-private vocabulary this spec exists to end.

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
place either is reliable.

**Correction.** This section originally claimed VisiCalc "picked the one that
reaches no native backend at all". That is wrong: VisiCalc authors **both** —
a `HostScroll [sheet-frame]` node *and* `overflow: auto` on the same part. The
`HostScroll` does reach every backend. What it lacked was an axis, so it
scrolled vertically and clipped the sheet horizontally.

The consequence was #14842, and that issue's own premise turned out to be a
measurement artifact too. It reported columns Q–Z as `0 x 24` at origin
`(0, 0)`, measured with `boundsInRoot` — which returns `Rect.Zero` for any node
clipped outside the viewport. Read through `size` and `positionInRoot` instead,
the columns were always laid out correctly at an 80px pitch (`P` at x=1308,
`Q` at 1388, …). They were not collapsing; they were unreachable. Giving the
sheet `axis: both` takes its horizontal scroll range from *none* to `0.0/896.0`
and closes the issue.

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

1. ~~**`fillMaxWidth` inside a Compose `horizontalScroll` is a crash.**~~
   **MEASURED FALSE, and struck.** The claim was that a horizontal scroll
   measures against an infinite max width and `fillMaxWidth` against infinity
   throws — and from that, that `axis: horizontal` would require suppressing
   the emitter's default `fillMaxWidth` throughout the scrolled subtree, across
   its eight emission sites.

   Rendered on this repo's pinned `org.jetbrains.compose` 1.6.11, it does not
   throw: Compose falls back to the minimum width when the width constraint is
   unbounded. The probe was falsified before being believed — a deliberate
   `error(..)` was planted inside the same composable and the harness reported
   `failures="1"`, so the clean run is a real absence of a throw rather than a
   swallowed exception.

   No suppression was written, and none is needed. The cost of the original
   claim would have been a subtree-wide refactor of the Compose emitter to
   avoid a crash that does not happen.

2. **A scroll needs a bounded viewport.** `.verticalScroll(..)` alone left the
   container wrapping its content in #14798, which is wrong twice: a scroller
   sized to its own content has nothing to scroll within, and whatever the
   content does not cover stays unpainted. The fix was `.fillMaxSize()` first.
   The horizontal case needs the same, and the two must compose.

3. **`height: 60vh` on VisiCalc's frame is also dropped**, so it has no bounded
   height either. #14837 mapped `100vh` to `fillMaxHeight()`; a fraction wants
   `fillMaxHeight(0.6f)` and is not covered.

## 6. Acceptance

- **[done]** A `HostScroll` with `axis: horizontal` scrolls horizontally and
  **not** vertically. Each backend asserts the axis as a *set* — the negative
  half is what has teeth, since an emitter that merely *added* a horizontal
  scrollbar to its existing vertical one passes every positive assertion.
- **[done, clarified]** The default stays `vertical`, asserted by a test that
  fails if the default changes.

  The original wording — "existing `HostScroll` output must be byte-identical"
  — was too broad, and contradicted §4 on the three backends that were scrolling
  *both* axes. It is byte-identical on the five backends already honouring the
  default (SwiftUI, Flutter, Compose, XAML, and Qt's `ScrollView` element),
  which is what protects shipped apps. On **html, react and webcomponent it is
  a deliberate change**: a bare `overflow: auto` scrolls both ways, and §4 says
  a viewport must not scroll an axis nobody asked for.
- **[done]** No Compose build throws — verified by *rendering*, not compiling.
  Trestle was temporarily authored with `axis: horizontal`, generated, compiled
  and rendered. See the struck hazard in §5: the throw this guarded against does
  not exist at 1.6.11.
- **[done, on different grounds]** VisiCalc's tail columns (#14842). The
  prediction here — that the axis alone would not be enough, because VisiCalc
  reaches its viewport through `overflow` rather than `HostScroll` — was wrong
  on its premise (see the correction in §2): VisiCalc authors a real
  `HostScroll`, and `axis: both` alone fixes it. The `height: 60vh` gap (§5
  hazard 3) is real but turned out not to block this.

  The acceptance was also restated. "Report a non-zero width" is not assertable
  through `boundsInRoot`, which is zero for anything clipped off-screen; the
  gate asserts a **horizontal scroll range** plus **non-zero layout `size` on
  the off-screen columns** instead.
- **[NOT done — out of scope here]** `overflow` appears in the drop report of
  every backend that discards it (§3.3). Compose, SwiftUI and XAML already do;
  Qt and Flutter have no style-drop reporting at all and remain gated on
  #12022.

## 7. Out of scope

Retiring `overflow: auto` on the three backends that honour it. Making it
*reported* is in scope; removing it is a migration with its own consumers.
