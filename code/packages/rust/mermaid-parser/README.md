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

The `xychart` pipeline preserves chart orientation, titles, accessibility
metadata, categorical and numeric axes, named bar and line series, and optional
labels on individual data points. Line point labels resolve into positioned,
backend-neutral text for both vertical and horizontal charts. When categories
are omitted, series receive Mermaid's inferred one-based numeric x-axis;
explicit numeric x-axis ranges distribute data points evenly across that range.
Core `xyChart` init configuration preserves authored chart dimensions and
orientation, with an explicit syntax orientation taking precedence, plus title
visibility, title size and padding, named-series legend visibility, typography,
and padding, minimum plot-space reservation, and optional inside/outside
bar-value labels. Configured chart background, title, data-label, and plot
palette colors also reach backend-neutral Paint scenes and instructions.
Nested x/y-axis configuration independently controls label and title visibility,
font sizes, padding, and theme colors, along with axis-line visibility, stroke
width, and color. Tick visibility, length, stroke width, and color lower
independently for each axis into backend-neutral Paint paths. Bottom-axis label
rotation reserves its rotated bounds during layout and lowers through transformed
backend-neutral Paint glyphs.
The pinned Mermaid 11.16.1 parser acceptance corpus and native Metal render
fixtures pass, so `xychart` is tracked at `full` compatibility.

The Gantt subset now enters through a dedicated Mermaid 11.16.1 token and
parser grammar before semantic lowering. Core titles, date formats, sections,
task statuses, absolute and dependency-relative starts, durations, and
accessibility metadata survive through temporal layout and backend-neutral
PaintScene metadata. A native Metal fixture validates the pipeline to PNG.
Task-scoped `click` commands preserve quoted links and callback names and
arguments in semantic IR; temporal layout resolves task hit bounds, and Paint
scenes expose backend-neutral link and callback metadata. Calendar `includes`
and `excludes` controls now preserve explicit dates, weekdays, and configurable
Friday/Saturday weekend boundaries. Exclusions extend task bars and dependent
starts, while `axisFormat` and `tickInterval` resolve native time-axis labels
before Paint lowering. Explicit end dates honor `inclusiveEndDates`; a standard
bottom axis and optional `topAxis` lower independently, and `todayMarker`
stroke configuration resolves into backend-neutral path geometry. Multi-task
`after` starts choose the latest dependency end, while `until` ends choose the
earliest referenced start; unknown IDs and cyclic start graphs fail parsing.
ID-less declarations receive pinned `taskN` IDs, and one-field task data starts
after the preceding task even across sections. Repeated task tags retain active,
done, critical, milestone, and vertical-marker semantics independently. Numeric
calendar formats, short and long month names, bracketed literals, time-of-day
precision, and Unix second/millisecond formats compile into typed IR and drive
layout geometry. Millisecond, second, minute, hour, day, and week durations also
retain their authored units in typed IR before precise layout conversion. A
pinned upstream corpus now gates this supported parser surface. Semicolon- and
hash-prefixed titles, sections, and task labels follow the pinned upstream
grammar while lowering to clean semantic labels.
HTML break variants in titles, sections, and tasks now lower to semantic
newlines with multiline layout. Colon and compact timezone offsets compile to
typed parts and normalize into UTC layout geometry.
single-component second timestamps also retain sub-minute precision through
layout, and typed 12-hour clocks resolve meridiem markers before layout. Gantt
also validates numeric plus two-letter, short, and long English weekday tokens
against resolved dates.
One-, two-, and three-digit fractional-second tokens retain their authored
precision before backend-neutral temporal layout.
Ordinal calendar-day tokens validate their numeric suffix before layout.
Unpadded 24-hour, minute, and second fields retain sub-minute precision.
Quarter tokens resolve to the first month of the authored quarter.
Signed variable-width year tokens retain their sign through semantic parsing
and backend-neutral temporal layout.
Every valid pinned Gantt syntax fixture passes backend-neutral paint lowering
and Metal-to-PNG validation. The pinned Mermaid 11.16.1 Gantt bundle's English
date tokens are complete; host-specific responsive SVG sizing is intentionally
outside the native backend-neutral compatibility contract.
The pinned parser corpus includes upstream configuration, multiline
accessibility, task-tag, callback-argument, prototype-like ID, millisecond,
multi-reference dependency, and forward cross-section dependency cases.
Three-digit years under the default date format follow Mermaid's bounded
non-strict fallback without accepting malformed or oversized compact years.
Explicit calendar includes and excludes retain their authored values in typed IR
and resolve through the configured date format during temporal layout.
Duration schedules whose recurring exclusions cover every weekday fail semantic
validation instead of producing unbounded fallback geometry.
The pinned DB corpus also preserves authored task order across sections and
calendar-day geometry across month-end, inclusive-end, and daylight-saving
boundaries.
Pinned native-render fixtures cover sub-day tick intervals and typed axis time
fields through Metal-to-PNG validation.
Calendar-aligned week and month fixtures gate weekday-aware native axis layout.
Pinned styled-marker fixtures preserve Mermaid `todayMarker` opacity through paint lowering.
Pinned repeated-vertical-marker fixtures gate row-free semantic marker lowering.
Pinned date-time fixtures gate date-only exclusions and configurable weekend geometry.
YAML `displayMode: compact` is preserved as semantic Gantt layout configuration.
The upstream unpadded day-only render case closes the pinned native fixture set.

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
The first pinned Mermaid 11.16.1 Sequence corpus slice preserves equals-sign
participant IDs through semantic IR, layout, Paint lowering, and Metal rendering,
while retaining upstream rejection of malformed participant configuration.
Pinned lifecycle fixtures also gate participant and actor creation, destruction,
and create-then-destroy ordering through dynamic headers, footers, and lifelines.
Pinned note fixtures cover left, right, centered, and ordered participant-pair
placements through resolved note bounds and backend-neutral Paint geometry.
Pinned activation fixtures gate explicit, suffix-based, and nested activation
stacks, including upstream rejection of deactivation underflow.
Pinned automatic-numbering fixtures gate default visibility, integer numbering,
decimal starts and increments, and upstream rejection beyond hundredths precision.
Pinned control-block fixtures gate loops, optionals, alternate and parallel
branches, critical options, breaks, overlapping annotations, and nested rects.
Pinned message fixtures gate solid and dotted cross, point, filled,
bidirectional, and all eight half-arrow orientations through native Paint paths.
Pinned participant-group fixtures gate named, transparent, unlabeled, and RGB
boxes while preserving group membership and actor metadata through the scene.
Pinned participant-configuration fixtures gate quote variants, aliases, mixed
implicit lanes, all six stereotypes, and malformed or unterminated definitions.
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
