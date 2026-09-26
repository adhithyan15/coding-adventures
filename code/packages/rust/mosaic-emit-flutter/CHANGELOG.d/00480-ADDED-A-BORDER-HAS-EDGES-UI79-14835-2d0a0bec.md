### Added — a border has edges (UI79, #14835)

`border-{top,right,bottom,left}-{width,color}` now lowers. Flutter is the one
backend whose toolkit expresses this directly — `Border(top: BorderSide(..),
bottom: ..)` carries a colour *and* a width per side — so unlike Compose
(#15009) nothing has to be hand-drawn.

A part that authors no edge keeps the byte-identical `Border.all(..)` it emits
today; the per-side form appears only when an edge is authored, and unauthored
sides fall back to the `border-width`/`border-color` shorthand, which is the
CSS cascade answer (UI79 §3 rule 3).

#### Measured in the rendered widget tree, not in the Dart

Trestle's generated app, walked with `find.byType(Container)` and each
`BoxDecoration.border` inspected:

| | before | after |
| --- | --- | --- |
| containers with a uniform four-edge border | 5 | 5 |
| containers with a border on **fewer** than four edges | **0** | **2** |

The two are `right=1.0` and `top=3.0`, matching Trestle's authored
`border-right-width: 1` and `border-top-width: 3`. The uniform count is
identical either way, so existing borders are untouched. Falsified by making
the new path return `None` and confirming the count returns to zero.

#### The writer that runs is not the one that looks like it does

Worth recording, because it cost most of the work. The crate has two
`Border.all` writers. `emit_styled_box` is the one a reader finds first, and it
is gated behind `part_has_decoration`, which is only consulted for a `Box` node
lowering to a `Container`. The writer that actually runs for a styled container
is in `emit_container`. Patching the first one emitted the per-edge border
**nowhere**, and a grep for `Border.all` had hidden the second because its
`format!` sits on the previous line.

This was settled by planting a marker in the emitted string and finding it
absent from the output — not by reading the call graph, which had already been
wrong twice.

`border-<edge>-style` is not consulted: only `solid` is drawn. Flutter has no
style-drop reporting at all (#12022), so unlike Compose a non-solid style is
lost **silently** here. That is a gap in the report, not in this lowering, and
it is recorded rather than worked around.

