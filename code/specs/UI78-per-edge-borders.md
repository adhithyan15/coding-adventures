# UI78 — a border has edges

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
| Flutter | `BoxDecoration(border: Border(bottom: BorderSide(..)))` | **yes** |
| XAML | `<Border BorderThickness="0,0,0,1" BorderBrush=".."/>` | **yes** |

`drawBehind` is preferred over a sibling `HorizontalDivider` on Compose: a
divider is a layout child and would take part in the parent's arrangement,
changing spacing. A border must not move anything.

## 5. Scope of the first change

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
