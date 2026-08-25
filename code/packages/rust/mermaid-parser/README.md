# mermaid-parser

Versioned Mermaid compatibility dispatcher and grammar-driven Rust parsers.

Compatibility is pinned to Mermaid 11.16.1 in
`code/grammars/mermaid/compatibility.json`. The manifest distinguishes family
detection from syntax and native-render compatibility so progress can be
measured without treating a recognized header as a completed implementation.

The current native pipeline supports documented subsets of:

- `flowchart` / `graph`
- `classDiagram`
- `sequenceDiagram`
- `erDiagram`
- `C4Context` / `C4Container` / `C4Component` / `C4Dynamic` / `C4Deployment`
- `gantt`
- `gitGraph`
- `pie`
- `sankey`
- `xychart`
- `quadrantChart`
- `journey`

The `xychart` subset preserves chart orientation, titles, accessibility
metadata, categorical and numeric axes, named bar and line series, and optional
labels on individual data points. Line point labels resolve into positioned,
backend-neutral text for both vertical and horizontal charts. When categories
are omitted, series receive Mermaid's inferred one-based numeric x-axis;
explicit numeric x-axis ranges distribute data points evenly across that range.
Core `xyChart` init configuration preserves authored chart dimensions, title
visibility, title size and padding, and optional inside/outside bar-value
labels. Configured data-label colors also reach backend-neutral Paint glyphs.
Nested x/y-axis configuration independently controls label and title visibility,
font sizes and padding, along with axis-line visibility and stroke width. Tick
line styling and label rotation remain pending.

Each supported family lowers into the shared Diagram IR and can continue
through its family layout package:

```text
Mermaid
  -> family grammar and parser
  -> Diagram IR
  -> family layout
  -> diagram-to-paint
  -> PaintScene
  -> Metal / SVG / Direct2D / other Paint VM backends
```

The `quadrantChart` subset covers titles, x/y endpoint labels, all four
quadrant labels, normalized points, point classes, inline point radius, fill,
and stroke styles, accessibility metadata, case-insensitive keywords, comments,
one-sided axes, extended axis arrows, Unicode labels, markdown strings, and init-configured
dimensions, axis positions, point radius, padding, independent border widths,
and title, axis, region, and point-label typography, plus all 15 quadrant theme
variables. The pinned upstream parser/style corpus and native Metal render
fixture pass, so `quadrantChart` is tracked at `full` compatibility.

The `journey` pipeline covers titles, sections, task scores in Mermaid's
documented one-to-five domain, comma-separated actors, accessibility metadata,
and multiline break-tag labels. Resolved layout assigns deterministic actor
colors, and Paint renders actor legends, task markers, and score faces.
Journey init directives also preserve diagram margins, task dimensions, and
task spacing through semantic IR and resolved temporal layout. Configured task
font size and family reach backend-neutral Paint glyph shaping.
Configured Journey title font size, family, and color follow the same resolved
layout and Paint shaping path without changing other temporal families.
Actor colors and cyclic section fill/text palettes also resolve before Paint,
matching Mermaid's Journey-specific init configuration model.
Configured `leftMargin` reserves the legend column before task rows, while
`maxLabelWidth` deterministically wraps actor labels into resolved Paint bounds.
Journey sections and tasks resolve as horizontal columns with score-ranked faces,
an activity spine, and dashed descenders before backend-neutral Paint lowering.
The pinned upstream parser corpus and native Metal render fixture pass, so
`journey` is tracked at `full` compatibility.

The sequence subset includes participants and actors, aliases, standard solid
and dotted message arrows, bidirectional/cross/point arrowheads, notes,
activations, titles, automatic numbering, and nested control blocks with branch
separators. Participant creation and destruction are lifecycle events consumed
by layout. Participant `box` declarations preserve group labels, fills, and
membership. Singular and JSON-map actor-menu links survive as PaintScene
metadata for interactive backends. Arbitrary JSON-valued actor properties are
also preserved in scene metadata. DOM-referenced `details` element IDs survive
the native pipeline; resolving host document contents remains an embedding-layer
compatibility gap.
Actor identifiers may contain multiple whitespace-separated words; the full ID
is retained consistently across declarations, messages, notes, lifecycle events,
and metadata commands.
Hyphenated actor identifiers retain their punctuation through those same
semantic references while post-arrow `-` remains the deactivation operator.
Both single-line accessibility statements and multiline `accDescr` blocks
lower to PaintScene metadata.
Newlines and semicolons are interchangeable sequence statement terminators,
including inside control blocks.
Sequence boxes and rect highlights accept `rgb`, `rgba`, `hsl`, and `hsla`;
HSL colors normalize to RGB so native backends receive portable paint values.
Sequence titles accept both `title Text` and legacy `title: Text` syntax.
Decimal (`#9829;`) and HTML named (`#infin;`) Mermaid entity codes decode to
Unicode before layout and native text shaping.
Sequence message and note `<br>` variants become semantic newlines with
deterministic multiline layout and native glyph shaping.
Message and note `wrap:` and `nowrap:` directives survive semantic lowering;
forced wrapping becomes deterministic hard lines before backend-neutral Paint shaping.
The same wrap controls apply to control-block and branch labels, with sequence
layout reserving line-aware frame and divider geometry.
All Mermaid 11.16.1 solid/dotted, normal/reverse filled and stick half-arrow
forms preserve their line style, half orientation, and endpoint through Paint.
Central connection markers before and/or after an arrow preserve source,
destination, and dual endpoint semantics through layout and Paint.
`autonumber` supports Mermaid 11.15+ decimal start and increment values with
up to two decimal places.
Nested `rect` background highlights preserve their functional-color fills
instead of being treated as labeled control frames.
Inline participant configurations support Mermaid's `type` and `alias` fields,
including boundary, control, entity, database, collections, and queue symbols.

All other Mermaid 11.16.1 family headers are recognized and return an explicit
`recognized but not implemented` error until their grammar, lowering, layout,
and native render fixtures are complete.
