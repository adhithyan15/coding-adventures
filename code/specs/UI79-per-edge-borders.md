# UI79 — a border has edges

Issue: [#14835](https://github.com/adhithyan15/coding-adventures/issues/14835)

`border-{top,right,bottom,left}-{width,color,style}` is **one mechanism, not
twelve properties**. Five backends drop all of it. This spec says what the
mechanism means and how each toolkit expresses it.

## 1. Measured, not assumed

One part authoring the three `border-bottom-*` declarations Engram writes on
`deck-list-entry`, emitted on all eight backends and each output read:

| backend | emits the edge | reports the drop |
| --- | --- | --- |
| html | yes — passes the CSS through | n/a |
| react | yes — `borderBottomWidth` etc. | n/a |
| webcomponent | yes — via the `.lattice` stylesheet | n/a |
| **Compose** | **no** | yes |
| **SwiftUI** | **no** | yes |
| **XAML** | **no** | yes |
| **Flutter** | **no** | **no** ([#12022](https://github.com/adhithyan15/coding-adventures/issues/12022)) |
| **Qt** | **no** | **no** (#12022) |

Two things worth separating, because they were conflated when this was filed.
The drop is **already loud** on Compose, SwiftUI and XAML — it lands in the
report's `styleDegradations` list, which is separate from and non-gating
against `degradations`. Reading only the latter makes it look silent, which it
is not. So this spec is about **lowering** the property, not about reporting it.

The visible consequence is Engram's deck list rendering as an undivided run of
rows.

## 2. Why this is a real gap and not an unthreaded property

Unlike the `gap`/`justify-content` family, there is no argument slot sitting
unused. `Modifier.border(width, color, shape)` draws **all four edges** and has
no per-edge form; SwiftUI's `.border` is the same. The lowering has to be built,
not connected.

## 3. The rule

> An authored edge draws exactly that edge. `border-bottom-width: 1px` draws a
> line along the bottom of the border box and nowhere else. An edge with no
> authored width draws nothing.

Three sub-rules the backends have to agree on, because each toolkit would
otherwise pick its own answer — the UI61 failure mode:

1. **The line sits on the border box**, outside padding and inside margin —
   where CSS puts it, since the authored vocabulary is CSS's.
2. **`-style` beyond `solid` is not in scope.** `dashed`/`dotted` are a
   separate capability; an authored non-`solid` style must be **reported**, not
   silently drawn solid.
3. **A per-edge rule and the all-four `border` shorthand do not compose.** If a
   part authors both, the per-edge declaration wins for its edge — the CSS
   cascade answer — and this must be asserted, not assumed.

## 4. Per-toolkit lowering

| backend | lowering | native per-edge support |
| --- | --- | --- |
| Compose | `Modifier.drawBehind { drawLine(..) }` | no — must be drawn |
| SwiftUI | `.overlay(alignment:) { Rectangle().frame(height:) }` | no — must be drawn |
| Qt | a child `Rectangle` anchored to the edge | no — must be drawn |
| Flutter | `BoxDecoration(border: Border(bottom: BorderSide(..)))` | **yes** — width *and* colour per side |
| XAML | `<Border BorderThickness="0,0,0,1" BorderBrush=".."/>` | **partly** — see below |

**Correction on XAML.** This table first said XAML has native per-edge support.
It has native per-edge *thickness* — `BorderThickness="left,top,right,bottom"`
— but only a **single `BorderBrush` for all four edges**. So
`border-top-color: X; border-bottom-color: Y` cannot both be honoured, and XAML
needs a decision this spec has not made: honour one colour and report the
others, or refuse the whole declaration. It is therefore *not* the cheap
follow-up it was described as. Its thickness aggregation is also not a 1:1
property mapping — four authored widths collapse into one attribute — which the
emitter's per-property setter table cannot express as-is.

`drawBehind` is preferred over a sibling `HorizontalDivider` on Compose: a
divider is a layout child and would take part in the parent's arrangement,
changing spacing. A border must not move anything.

## 5. Scope

**Done:** Compose (#15009, `drawBehind`), Flutter (per-side `BorderSide`),
SwiftUI (`.overlay(alignment:)`) and Qt (an anchored child `Rectangle`).

Qt was a different **shape** of change rather than a harder version of the same
one: its border is a property on a QML `Rectangle`, but a per-edge rule needs a
*child* element, emitted where children are rather than where properties are.
It also needed the wrapper's implicit size re-derived from a named content
layout — a strip anchored to `parent` and a parent sized from `childrenRect`
form a **binding loop**, which only the QML runtime reveals.

**Remaining: XAML**, which needs the single-brush decision above settled first.

### A verification gate this spec did not know it had

Qt *is* runnable on the development host: `qml` under
`QT_QPA_PLATFORM=offscreen` loads generated QML, reports binding loops, and
grabs frames via `grabToImage` for pixel sampling. Earlier notes in this work
claimed otherwise. Every backend UI79 touched now has a way to check the
rendered result rather than the emitted text — Compose and Qt by sampling
pixels, Flutter by walking the widget tree, SwiftUI by `swift build`.

Compose only, all four edges. The remaining four backends are follow-up work
and are named as such rather than left looking finished — this spec is the
shared definition they will implement against.

**Correction on the consumer.** #14835 cites Engram's `deck-list-entry`, and
this spec repeated it. Engram no longer authors that part; it moved into
`mosaic-pkg-deck-stats`. The real first consumer is **Trestle**, which authors
34 per-edge declarations, and the property is far more widely used than the
issue suggests: **92 declarations across 12 stylesheets**, including
`mosaic-pkg-calendar` (15) and Venture's chrome.

## 6. Acceptance

- Each of the four edges draws on its own edge and nowhere else, asserted as a
  **set** — an emitter that drew all four for any single authored edge passes
  every positive assertion.
- Verified by **rendering and sampling pixels**, not by grepping the emitted
  Kotlin. A `drawBehind` that compiles and draws nothing is exactly the failure
  this repo keeps finding.
- A part with no authored border draws no line — the control, rendered in the
  same frame as the positive case.
- An authored `border-bottom-style: dashed` is reported, not drawn solid.
- The per-edge `width`/`color` properties disappear from the consumer's Compose
  `styleDegradations`, with the remaining drops enumerated rather than the
  absence of a few asserted. On Trestle: every per-edge `width` and `color` is
  gone; the 17 `-style` halves and two corner radii remain, correctly.
