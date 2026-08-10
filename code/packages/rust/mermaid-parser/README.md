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
