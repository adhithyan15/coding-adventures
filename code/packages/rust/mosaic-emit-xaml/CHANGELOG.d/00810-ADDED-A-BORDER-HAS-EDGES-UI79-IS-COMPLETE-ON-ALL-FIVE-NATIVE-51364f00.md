### Added — a border has edges; UI79 is complete on all five native backends (#14835)

`border-{top,right,bottom,left}-{width,color}` now lowers, completing UI79
after Compose (#15009), Flutter (#15018), SwiftUI (#15024) and Qt (#15031).

WinUI's `BorderThickness` really is per-edge, so four authored widths lower
exactly — but they collapse into **one** attribute in `left,top,right,bottom`
order, which the emitter's per-property setter table cannot express. It is
computed in a pre-pass, the same shape as `absolute_position_style_attrs`, and
its inputs are skipped in the main loop rather than falling through to the
generic drop.

Trestle now emits `BorderThickness="0,0,1,0"`, `"1,0,0,0"`, `"0,1,0,0"` and
`"0,0,0,1"` where it previously emitted nothing for those edges. Parts using
the all-four shorthand keep the byte-identical single-value form.

#### The single brush, decided by measuring rather than arguing

WinUI has **one `BorderBrush` for all four edges**, so two different edge
colours on one part cannot both be painted. Rather than guess, the corpus was
counted: **44 parts author per-edge colours and every one uses a single
colour.** The impossible case is not a case anyone has.

So the single authored colour becomes the brush, and if edges ever disagree the
first in CSS order is painted and **every other authored edge colour is
reported as a drop** — never silently repainted. Verified both ways: one colour
produces no drop, two produce exactly one report naming `BorderBrush`.
Falsified by making the report silent and watching the test fail.

Refusing the whole declaration was the alternative, and it would have discarded
44 working cases to guard against zero broken ones.

#### What could not be verified

**WinUI 3 does not build on macOS**, so unlike the other four backends there is
no way to run this. Compose and Qt were checked by sampling rendered pixels,
Flutter by walking the widget tree, SwiftUI by `swift build`. XAML is backed by
unit tests and by reading the emitted markup, which is weaker, and saying so is
the point.

