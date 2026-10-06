# DG04 - Mermaid Compatibility Program

> Status: Draft
>
> Baseline: Mermaid 11.16.1, tag `mermaid@11.16.1`, commit
> `7ecca0cd7f1658ef74f4e7e91f925724ef403bbf`

## Goal

Reach practical compatibility with the full Mermaid language while preserving
the native rendering pipeline:

```text
Mermaid source
  -> shared family grammar
  -> source AST
  -> semantic Diagram IR
  -> family layout
  -> PaintScene / PaintInstructions
  -> Metal / Direct2D / SVG / WGPU / other Paint VM backends
```

Compatibility is versioned. "Full Mermaid" means the pinned release, not the
moving contents of Mermaid's development branch.

The machine-readable baseline is
`code/grammars/mermaid/compatibility.json`. Every language implementation and
CI report should consume the same manifest.

## Why This Is A Program, Not One Parser

Mermaid is a dispatcher over many independent languages. Flowcharts, sequence
diagrams, Gantt charts, packet diagrams, Sankey charts, and Wardley maps do not
share one useful context-free grammar or one useful semantic model.

The repository should therefore keep:

- one detector that recognizes every Mermaid family and alias;
- one shared token and parser grammar per family where a grammar is applicable;
- one source AST per family;
- a small set of reusable semantic IR families;
- specialized IRs only where the semantics are genuinely different;
- one common final lowering into PaintScene.

A monolithic Mermaid grammar would couple unrelated syntaxes and make
cross-language generation harder.

## Compatibility Levels

Each family advances independently through these levels:

| Level | Name | Requirement |
|---|---|---|
| 0 | Detected | Header, aliases, front matter, directives, and comments are recognized. |
| 1 | Parsed | The pinned syntax corpus produces a source AST with useful errors and locations. |
| 2 | Lowered | All source semantics are represented without lossy paint-specific shortcuts. |
| 3 | Layouted | The family layout package produces deterministic geometry. |
| 4 | Native | The layout lowers to PaintScene and renders on the Tier 1 native backends. |
| 5 | Compatible | Syntax fixtures and tolerant visual fixtures pass against the pinned release. |

The word `partial` in the compatibility manifest currently means Levels 1
through 4 exist for a documented subset. It does not mean full family
compatibility.

## Current Baseline

The first native subsets are:

- Flowchart and graph
- Class diagram
- Gantt
- Pie
- XY chart

Gantt core syntax is grammar-first for titles, accessibility metadata, date
formats, sections, and task declarations. Those statements lower to temporal
semantic IR, deterministic task-bar layout, Paint instructions, and a Metal PNG
fixture. The family remains partial until the remaining calendar, axis,
and pinned upstream corpus surface is represented and validated. Gantt task
`click` links and callbacks already lower through semantic IR to resolved task
bounds and backend-neutral PaintScene metadata. Calendar includes/excludes and
weekend boundaries now extend task and dependency geometry, and configured
axis formats and tick intervals lower to resolved labels rendered by Paint.
Explicit end dates honor inclusive-end mode, optional top axes complement the
standard bottom axis, and styled today markers lower to backend-neutral paths.
Multi-source `after`/`until` ranges resolve through validated dependency lists.
ID-less declarations receive pinned generated IDs, and one-field task data
preserves sequential starts across sections. Combined active/done/critical/
milestone tags remain independent, while `vert` tasks lower to full-height
markers without consuming task rows. Full date-format support plus the pinned
parser/visual corpus remain explicitly incomplete.

The XY-chart pipeline preserves Mermaid's bounded `xAxis.labelRotation` and
`yAxis.labelRotation` configuration in semantic chart IR. As in the pinned
renderer, rotation affects labels only when that axis is placed at the bottom;
layout reserves the rotated bounds before transformed Paint glyphs reach native
backends. Its pinned 11.16.1 parser acceptance corpus and native Metal render
fixtures pass, so the family is tracked at full compatibility. Its eight nested
`xyChart` axis theme variables preserve
independent label, title, tick, and spine colors for both axes through semantic
IR, layout, and backend-neutral Paint instructions. Chart background, title,
data-label, and comma-separated plot-palette colors follow the same pipeline.

The existing IR already has useful downstream capacity for the next group:

| Mermaid family | Existing semantic target | Existing layout and paint |
|---|---|---|
| Sankey | `ChartDiagram::Sankey` | Yes |
| GitGraph | `TemporalDiagram::Git` | Yes |
| ER | `StructuralDiagram::Er` | Yes |
| C4 | `StructuralDiagram::C4` | Yes |

These should be implemented before inventing new IR families.

## Required New Semantic Families

The remaining languages cluster into reusable domains:

| Domain | Mermaid families |
|---|---|
| Sequence | Sequence Diagram, ZenUML |
| State | State Diagram |
| Hierarchy | Mindmap, Treemap, TreeView |
| Board and lanes | Kanban, Swimlane, Block |
| Quantitative chart | Quadrant, Radar, Venn |
| Chronology | Timeline, Journey |
| Systems modeling | Requirement, Architecture, Event Modeling |
| Grammar | Railroad, EBNF, ABNF, PEG |
| Specialized geometry | Packet, Sankey, Ishikawa, Wardley, Cynefin |

Sharing a domain IR does not require sharing a source AST. For example,
Sequence Diagram and ZenUML should have different parsers but may lower into
the same participant/message/lifeline IR.

### State Native Slice

The initial Mermaid 11.16.1 state slice is grammar-backed and covers
`stateDiagram`/`stateDiagram-v2` headers, simple declarations and quoted
aliases, standalone `State: description` labels, labeled transitions, document
direction, modern or legacy title statements, and `[*]` start/end edge states.
Titles survive graph IR and layout, then use the existing shaped Paint title
pipeline on native backends. The family lowers into the shared graph IR,
graph layout, and backend-neutral
PaintScene instructions, with a Metal-to-PNG fixture. Choice pseudostates accept
both `<<choice>>` and `[[choice]]` and lower to graph-IR diamonds. The family
remains partial until the pinned upstream corpus passes without unsupported
forms or lossy semantics.
Repeated `State: description` statements accumulate as ordered multiline
semantic labels; graph layout reserves line-aware node geometry before Paint
shapes each authored line without backend soft wrapping.
Quoted state aliases may include a trailing `: description`; both the primary
label and trailing description survive as ordered multiline semantic text.
Fork and join pseudostates accept both upstream marker spellings and lower to a
compact backend-neutral graph-IR bar shape rendered by existing rectangle Paint
instructions.
Inline `style` statements preserve fill, stroke, text color, and stroke width
through graph IR, layout style resolution, and backend-neutral Paint geometry
and glyph instructions, including comma-delimited node and composite targets.
Named `classDef` declarations and comma-delimited
`class` assignments resolve the same properties into graph IR, including
assignments that precede their declaration. The `:::` shorthand applies named
classes to standalone states and either endpoint of a transition, including
start/end pseudostates. Inline and named styles can target composite groups and
survive resolved layout style into backend-neutral Paint rectangles and labels.
State `font-size` styles survive semantic IR; graph layout measures matching
node geometry before Paint shapes and centers text at the resolved size.
State `font-weight` styles accept normal, bold, and numeric CSS weights; graph
measurement and Paint glyph shaping consume the same resolved weight.
State `font-style` accepts normal and italic; graph measurement and Paint glyph
shaping consume the same resolved italic flag.
State `font-family` accepts quoted or unquoted family names; graph measurement
and Paint glyph shaping consume the same resolved family.
One `class` statement may compose multiple named classes on every target;
later classes override properties from earlier classes in authored order.
Single-line and `end note` multiline `note left of`/`note right of` statements
lower to semantic note nodes and note-association edges. Quoted `note ... as`
statements lower to standalone note nodes. Graph layout reserves line-aware note
geometry, and the Paint lowering emits folded note and dashed connector paths
for every backend.
Single-line `accTitle`/`accDescr` and braced multiline `accDescr` statements
survive graph semantic IR and layout IR, then export as backend-neutral
PaintScene accessibility metadata.
State `click` statements, including the `href` spelling and optional tooltips,
survive graph semantic and layout IR. PaintScene exports each URL, tooltip, and
resolved node bounds as backend-neutral hit-test metadata.
Nested `state Name { ... }` composites preserve parent/child containment in
graph-group semantic IR. Graph layout computes padded nested bounds, while
Paint lowering draws group outlines and shaped labels behind member geometry.
Quoted `state "Label" as Id { ... }` composites preserve distinct semantic IDs
and display labels through the same pipeline.
`--` dividers preserve ordered concurrent-region membership in graph-group IR.
Graph layout stacks direct region members into deterministic lanes and Paint
lowering emits horizontal divider paths for every backend.
Composite-local `direction` statements remain scoped to their group in semantic
IR and arrange direct region members independently of the document direction.
`scale N width` preserves the requested canvas width in graph IR. Layout scales
all geometry and resolved stroke, corner, and font sizes uniformly before the
backend-neutral Paint scene reaches Metal or another renderer.
`hide empty description` survives graph semantic and layout IR; Paint lowering
omits unlabeled state geometry and glyphs while retaining graph connectivity.
State labels, transition labels, notes, titles, and accessibility text decode
Mermaid decimal or named entities and HTML line breaks before line-aware layout
and backend-neutral Paint glyph shaping.
State descriptions and transition labels preserve additional authored colons as
text after the statement's leading delimiter.
Transitions entering or leaving a composite retain the group ID as their
semantic endpoint. Graph layout attaches those edges to the resolved group
boundary before existing Paint paths and arrowheads render them.
Pinned `#` comments are discarded by the portable state token grammar while
decimal or named entities and hexadecimal style colors remain semantic input.

### Sequence Native Slice

The first sequence vertical slice is grammar-backed and covers participant and
actor declarations, aliases, implicit participants, solid and dotted message
arrows, open/filled/cross/point arrowheads, bidirectional messages, notes,
activation/deactivation, titles, and automatic numbering. It lowers through
`diagram-layout-sequence` to existing path, rectangle, dashed-stroke, and glyph
PaintInstructions and is exercised by a Mermaid-to-Metal-to-PNG fixture.

Grammar-backed `actor` declarations retain their semantic kind through layout
and lower to backend-neutral ellipse/path instructions for UML stick figures.
Sequence layout mirrors participant and actor headers below the interaction,
matching Mermaid's default lifeline presentation through the same Paint IR.

Nested Mermaid 11.16.1 control blocks (`loop`, `opt`, `alt`/`else`, `par`/`and`,
`par_over`, `critical`/`option`, `break`, and `rect`) lower into ordered semantic
block events. Sequence layout resolves those events into nested frames and
branch dividers before existing PaintInstructions render them.
`par_over` frames retain their distinct semantics by overlaying sibling notes
at the parallel content origin while preserving the tallest content extent.
Participant `create` and `destroy` statements lower into lifecycle events;
layout uses them to place dynamic participant headers and footers and bound
lifelines. Created headers and destroyed footers are centered on their
associated message lines, those messages terminate at the participant edge,
and destroyed lifelines and open activation bars terminate on that line.
Lifecycle declarations bind to
Mermaid's required following message:
created participants must receive it, while destroyed participants must send or
receive it. Created participant IDs must be new, and an existing participant
cannot be reassigned between participant boxes. Nested message and statement
activations retain stack order in semantic
events and lower to depth-offset bars through backend-neutral Paint rectangles.
Messages entering or leaving active participants terminate at the visible edge
of the current activation bar, including a bar opened by that message. Paint
ordering keeps those message paths and arrowheads above activation rectangles.
Self-messages on active participants anchor to the outer edge of the current
activation stack rather than falling back to the lifeline center.
Their source and destination tips remain distinct through Paint lowering so
reverse and bidirectional arrowheads and central markers use the correct ends.
Explicit activation and deactivation statements update that stack without
creating synthetic event rows or vertical gaps.
Explicit and message-suffix deactivation is validated against that semantic
stack and fails when the participant is inactive, matching Mermaid 11.16.1.
Central connections use distinct grammar alternatives and reject `+` or `-`
message suffixes; their marked endpoints provide the activation semantics.
Singular and JSON-map actor-menu links lower through semantic IR and
layout into PaintScene metadata. Actor `properties` preserve arbitrary JSON
values through the same pipeline. Mermaid's built-in `@clock` and `@computer`
property icons lower to backend-neutral ellipse, rectangle, and path
instructions; external image-property resolution remains a host concern.
DOM-referenced `details` element IDs also
survive the pipeline as scene metadata; host-document resolution remains
embedding-layer compatibility work. Participant `box` declarations now lower into
semantic groups, lane-enclosing layout geometry, and backend-neutral Paint
rectangles and labels, including the supported named and `rgb`/`rgba` color
forms. Functional `hsl`/`hsla` colors normalize to backend-safe RGB while
retaining their color semantics. The family remains partial until the
pinned upstream corpus pass; unsupported forms must fail grammar validation
rather than degrade silently.

Sequence `accTitle`, single-line `accDescr`, and multiline `accDescr` blocks
preserve accessibility semantics in PaintScene metadata.
Sequence newlines and semicolons are interchangeable statement terminators at
the document level and inside control blocks.
Mermaid preprocessor directives are removed before sequence grammar parsing
without changing source line positions. The global `wrap` directive updates
default participant, message, note, and control labels in semantic IR; host
configuration from `init` remains outside diagram semantics.
Leading YAML front matter is likewise removed without changing source line
positions; interpreting its title and configuration values remains a host-level
preprocessing concern.
Both modern `title Text` and legacy `title: Text` sequence title forms lower
through the same title semantics and native text pipeline.
Sequence text decodes Mermaid decimal and HTML named entity codes to Unicode
before layout and Paint glyph shaping.
Message, note, participant, and control labels reconstruct skipped whitespace
from token source columns so punctuation, angle text, embedded arrows, and
keyword-shaped words retain their authored spelling without synthetic spaces.
Sequence `#` comments are discarded by the grammar-driven lexer while numeric
and named `#...;` entities remain semantic label text.
Message and note `<br>`, `<br/>`, and `<br />` tags become semantic newlines;
sequence layout reserves line-aware geometry before Paint glyph shaping.
Message and note `wrap:` and `nowrap:` directives lower to explicit semantic
wrap intent. Forced wrapping is resolved into deterministic lines during
sequence layout, before Paint glyph shaping and native backend rendering.
Control-block and branch labels carry the same explicit wrap intent; layout
reserves line-aware frame headers and branch bands before Paint lowering.
Whitespace-separated multiword actor IDs are grammar-backed and retain one
semantic identity across declarations, messages, notes, lifecycle events, and
participant metadata before layout and Paint lowering.
Hyphenated actor IDs are likewise grammar-backed; parsing distinguishes an
interior identifier hyphen from Mermaid's post-arrow deactivation marker.

Inline participant configuration now carries `type` and `alias` into semantic
IR. Boundary, control, entity, database, collections, and queue kinds lower to
backend-neutral path, ellipse, and rectangle symbols and have Metal PNG coverage.
Quoted configuration aliases retain embedded commas instead of being split into
spurious fields. Double-quoted JSON escapes and doubled single quotes decode
with Mermaid's YAML JSON-schema configuration semantics before semantic IR.
Mermaid 11.16.1 half arrows are grammar-backed across every solid/dotted,
normal/reverse, filled/stick, and top/bottom form. Their endpoint semantics
survive layout and lower to backend-neutral Paint paths.
Central connection syntax (`()->>`, `->>()`, and `()->>()`) lowers to explicit
source/destination endpoint semantics. Each marked endpoint opens its own
validated activation stack entry before layout emits the activation bars and
Paint ellipse markers.
Automatic numbering preserves Mermaid 11.15+ decimal start and increment
values through semantic IR, layout, and shaped Paint labels. Re-enabling a
paused counter without arguments resumes its current value and increment;
layout rounds every increment to Mermaid's two-decimal sequence precision.
Semantic validation rejects contiguous number tokens so thousandths cannot be
misread as a valid start/increment pair, matching the pinned lexer boundary.
Nested `rect` background highlights carry RGB/RGBA fills and normalized HSL/HSLA
fills through semantic block events, layout frames, and Paint. Empty `rect`
headers preserve Mermaid's theme-default background intent, and CSS named
colors remain backend-neutral Paint values.
Sequence headers, statement keywords, placements, and control words match
case-insensitively as required by Mermaid 11.16.1's Jison lexer, while actor IDs
and user-authored text retain their original case through semantic IR and Paint.

### Timeline Native Slice

The initial Mermaid 11.16.1 timeline slice is grammar-backed and covers
`timeline`, `timeline LR`, and `timeline TD` headers, titles, accessibility
metadata, sections, periods, and ordered events attached to their preceding
period. These statements lower into timeline-specific temporal semantic IR.
Deterministic layout emits dedicated section, spine, and period-card geometry,
which lowers to backend-neutral PaintScene rectangles, paths, and shaped text
and is exercised by a Metal-to-PNG fixture. The family remains partial until
the complete pinned syntax and tolerant visual corpus passes.

### Mindmap Native Slice

The initial Mermaid 11.16.1 mindmap slice is grammar-backed and preserves
indentation as parent-child semantic relationships. It supports plain, square,
rounded, circular, hexagonal, cloud, and bang nodes, deterministic generated
IDs, quoted delimiter-rich descriptions, semantic HTML and escaped line breaks,
whole-line and trailing comments, depth styles, shared graph layout, and
backend-neutral PaintScene lowering. Multiline Markdown strings preserve their
authored source and typed bold/italic spans in semantic IR, participate in
layout, and lower to independently shaped backend-neutral glyph runs. Icon
identifiers and authored class names remain available as Paint instruction
metadata. Integrator-supplied class styles resolve in authored class order
before layout, so typography participates in node measurement and the resolved
fill, stroke, and text styling lower through backend-neutral PaintInstructions.
Integrator-supplied icon glyphs resolve from semantic icon identifiers, reserve
layout space, and lower through the same backend-neutral shaped-text path. A
native Metal-to-PNG fixture validates this documented subset; automatic loading
of external icon packs remains explicitly outside the partial compatibility
level.

### Block Native Slice

The initial Mermaid 11.16.1 block slice parses `block` and `block-beta` through
dedicated portable grammars into typed grid IR. The subset preserves automatic
and positive fixed column counts, flat node ordering, explicit `space` slots, node and space
column spans, named `classDef` declarations, `class` assignments, inline
`:::class` assignments, direct `style` declarations, square, rounded,
circular, double-circle, diamond, hexagon, stadium, subroutine, cylinder,
parallelogram, trapezoid, asymmetric, and cardinal or axis-direction arrow nodes,
quoted multiword labels, titles, single-line and braced
multiline accessibility metadata, recursive anonymous and named composite blocks
with authored labels, local column counts, and parent-grid spans, and directed
and undirected solid connections, plus directed and bidirectional point-arrow
connections with solid, dotted, and thick lines and pipe-delimited optional
labels, and circle or cross markers at either endpoint. Quoted labels are
preserved on solid, dotted, and thick directed links. Link endpoints may
declare square or rounded nodes inline, including their authored labels and
quoted edge labels, rather than requiring a separate cell declaration. Block
node, composite, edge, title, and accessibility text sanitize raw HTML markup,
preserve its visible text, and then decode HTML entities into semantic Unicode
before layout. Escaped tag text remains visible rather than being reinterpreted
as markup. Top-level semicolons delimit statements without splitting quoted
label text or decoded entities, including class names that match JavaScript
prototype properties. Terminal circle and
cross markers accept Mermaid's compact `A--oB` and `A--xB` forms without
collapsing hyphenated node IDs. Three-or-more-tilde links create their endpoint
nodes and lower to open, normal-width solid edges, matching the pinned Block
renderer. `diagram-layout-grid` resolves deterministic
cell geometry before the shared backend-neutral PaintScene lowering path. A
native Metal-to-PNG fixture gates this slice. Named classes and direct styles
resolve for both cells and composites, including backend-neutral
`stroke-dasharray` patterns. `classDef default` also styles cells and composites
without an explicit class, while direct styles retain final precedence. The
complete pinned `mermaid@11.16.1` Block parser corpus now passes through grammar,
typed grid IR, deterministic layout, PaintScene lowering, and the native Metal
render fixture, so Block is recorded at the `full` compatibility level.

### Packet Native Slice

The initial Mermaid 11.16.1 packet slice parses `packet` and `packet-beta`
through dedicated portable grammars into typed packet IR. It preserves
contiguous absolute inclusive bit ranges, relative `+bits` fields, quoted
labels, titles, accessibility metadata, and empty packets. Fields spanning
configured row boundaries are split into deterministic row fragments by
`diagram-layout-packet`. Mermaid's `rowHeight`, `bitWidth`, `bitsPerRow`,
`showBits`, `paddingX`, and `paddingY` settings flow through typed semantic IR
to resolved field and bit-label geometry, and dedicated PaintScene lowering
emits backend-neutral rectangles and shaped text. A configured native
Metal-to-PNG fixture gates this slice. All ten documented packet theme variables
resolve through semantic IR and layout into backend-neutral field, bit-label,
and title paint styles. The complete pinned syntax corpus and visual fixture gate
Packet at the full compatibility level.

### Kanban Native Slice

The initial Mermaid 11.16.1 Kanban slice uses dedicated portable grammars and
preserves indentation-defined columns and cards, with plain labels and explicit
`id[label]` forms, in typed board semantic IR. Rounded, circular, hexagonal,
cloud, bang, and alternate node delimiters normalize to Mermaid's fixed Kanban semantics while
preserving their authored labels through layout and Paint lowering. Quoted labels inside
node delimiters are unquoted before entering semantic IR, while quoted and inline Markdown labels
preserve bold and italic spans through backend-neutral glyph lowering. Escaped multiline labels
expand deterministic section headers and card geometry before Paint lowering. Inline or multiline `@{...}`
card metadata preserves label overrides, ticket identifiers, assignees, and
priorities in semantic IR. Icon identifiers from either metadata or following
`::icon(...)` decorators survive the same pipeline and lower to generic,
backend-neutral shaped badge geometry without coupling the board IR to an icon
provider. High and low priority values lower to Mermaid-compatible colored edge
markers without reserving footer space. Board layout reserves a compact metadata
footer for ticket and assignee fields, which PaintScene lowering aligns to the
left and right as backend-neutral shaped text.
Following `:::class` decorators preserve ordered column or card class names
through board semantic IR and layout, then lower them as `diagram.classes`
PaintInstruction metadata without coupling native renderers to CSS.
Kanban `ticketBaseUrl` configuration from Mermaid init directives or YAML
front matter survives semantic IR; layout resolves each card ticket placeholder
and Paint lowering exposes the resulting URL as backend-neutral hit-test metadata.
Ticket metadata on sections follows the same typed IR, layout, and backend-neutral
link-metadata path as card tickets.
Positive `sectionWidth` and `padding` settings from the same configuration paths
resolve column width and outer canvas geometry before backend-neutral Paint lowering.
`diagram-layout-board` resolves
deterministic column/card geometry and dedicated PaintScene lowering emits
backend-neutral rectangles and shaped text. A native Metal-to-PNG fixture gates
this slice; external icon-pack artwork, class-driven styles, non-ticket links, description
fields, non-ticket section metadata, and configurable field arrangements remain
unsupported at the partial level.

### Architecture Native Slice

The initial Mermaid 11.16.1 Architecture slice uses dedicated portable
grammars and maps groups, services, junctions, containment, icon identifiers, titles, and
undirected and right-directed edges into structural semantic IR. Existing deterministic
structural layout lowers groups, nodes, relationships, and shaped text through
backend-neutral PaintScene instructions, with a native Metal-to-PNG fixture.
Junction declarations, including group containment, lower to compact typed
structural geometry and backend-neutral ellipse PaintInstructions.
Native `-[label]-` and `-[label]->` edge labels survive structural semantic IR,
resolve to deterministic relationship geometry, and lower through the shared
backend-neutral shaped-text PaintInstructions path.
Document titles reserve resolved layout geometry and lower through shaped-text
PaintInstructions. Single-line accessibility titles and single-line or braced
descriptions survive as PaintScene metadata.
Sibling `align row` and `align column` constraints survive semantic IR and
resolve declared member order to deterministic shared-axis layout geometry.
Quoted service icon text survives as typed semantic metadata, reserves dedicated
header geometry, and lowers to backend-neutral badge and shaped-text PaintInstructions.
Left, right, and bidirectional relationship arrowheads survive semantic and layout
IR and lower to backend-neutral path geometry. Group-edge modifiers on contained
services survive semantic IR and resolve to deterministic group-boundary endpoints.
Explicit `L`, `R`, `T`, and `B` edge ports survive semantic and layout IR and
anchor deterministic orthogonal relationship paths to the requested node or group
boundary before backend-neutral PaintScene lowering. Named service and group icons
lower to canonical backend-neutral glyph geometry. Preserved namespaced identifiers
use a generic fallback when vendor artwork is unavailable. Orthogonal relationship
layout routes around unrelated service, junction, and group bounds with deterministic
clearance before the existing backend-neutral PaintScene lowering. Full vendor icon
artwork remains unsupported at the partial level. Architecture `iconSize`, `fontSize`,
`nodeSeparation`, `padding`, and
`idealEdgeLengthMultiplier` values from Mermaid init directives or YAML front matter
survive as typed semantic configuration and resolve into backend-neutral node geometry,
service typography, deterministic spacing, alignment-hint distances, group insets,
and outer canvas margins. Bounded `edgeElasticity` values survive the same configuration
paths and deterministically tighten or loosen connected sibling spacing before relationship
routing and backend-neutral PaintScene lowering. `randomize` and signed `seed` values also
survive semantic configuration and select a repeatable seeded permutation of sibling layout
slots while preserving authored IR order, containment, routing, and backend-neutral PaintScene
lowering. Unlike Mermaid's nondeterministic `seed: 0` escape hatch, the native pipeline keeps
zero deterministic. Iteration limits and the remaining fcose-specific tuning controls remain
unsupported at the partial level.

### Radar Compatibility

The Mermaid 11.16.1 Radar family uses dedicated portable grammars and
maps labeled axes plus positional or axis-keyed curves into chart semantic IR.
The chart layout emits deterministic polygonal or circular graticules, radial
spokes, axis labels, closed series paths, and legends using backend-neutral PaintScene
instructions, with a native Metal-to-PNG fixture. Core Radar options preserve
`showLegend`, positive numeric `ticks`, numeric `min`/`max`, and circle or
polygon `graticule` choices in semantic chart IR. Layout applies the configured
scale and graticule geometry before backend-neutral Paint lowering. Multiple
curves on one statement and multiline curve bodies lower into ordered semantic
series through the same grammar-backed path. Radar width, height, four margins,
axis scale, axis-label factors, and curve tension from init directives or YAML
front matter survive typed semantic configuration and control deterministic
native geometry. Circular graticules lower Radar series through backend-neutral
cubic Paint paths. Core Radar theme variables control axis and graticule
strokes, label and legend sizing, series palettes, translucent curve fills, and
stroke widths through the same semantic layout and Paint pipeline. Axis labels
carry angle-derived horizontal anchors and vertical baselines into backend-neutral
text layout so text extends away from the chart center. The native layout also
matches the upstream 600-pixel plot, 50-pixel margins, default series palette,
top title, and vertical overlaid legend, including sparse authored `cScaleN`
overrides. The pinned syntax corpus and native Metal-to-PNG fixture pass, so
Radar is recorded at the `full` compatibility level. Backend font rasterization
and SVG overflow behavior can differ without creating a Radar semantic
compatibility gap.

### Event Modeling Native Compatibility

Mermaid 11.16.1 Event Modeling compatibility uses dedicated portable
grammars and semantic event-model IR for numbered time and reset frames,
entity-kind aliases, qualified namespaces, inferred sequence relations, and
explicit multi-source relations. Typed and untyped inline frame data in object,
single-quoted, or double-quoted form plus
reusable multiline data blocks survive in semantic IR and render as secondary
frame labels. Typed and untyped frame notes remain semantic annotations and
lower into backend-neutral PaintScene metadata without changing Mermaid's
visible frame rendering. Given/When/Then scenarios preserve their source frame
and typed entity references through semantic IR and backend-neutral PaintScene
metadata, likewise matching Mermaid's current non-visual treatment. Standalone
entity declarations preserve qualified identifiers and namespaces through the
same backend-neutral pipeline. Init directives and YAML front matter preserve
the Event Modeling `padding`, `rowHeight`, and `useMaxWidth` configuration;
padding affects native viewport geometry while the currently non-visual row
height and responsive-width settings remain available in PaintScene metadata.
All ten upstream `em*Fill` and `em*Stroke` theme variables resolve from init
directives or YAML front matter into backend-neutral frame paint styles.
Explicit frame sources enforce Mermaid's entity flow rules for UI, processor,
command, event, and read-model frames, including every source in a multi-source
declaration. Empty diagrams, multiline comments, empty metadata, separated
block delimiters, every data type, and the pinned upstream parser examples are
covered by the complete syntax corpus. Temporal
swimlane layout lowers through backend-neutral PaintScene instructions and a
representative pinned visual corpus rendered through Metal-to-PNG. Event
Modeling is therefore full at the pinned native compatibility level;
browser-only responsive resizing remains outside the backend-neutral contract.

### Treemap Native Slice

The initial Mermaid 11.16.1 Treemap slice recognizes both `treemap` and
`treemap-beta` and parses quoted parent and leaf nodes, indentation hierarchy,
colon or comma numeric values, class selectors, titles, and accessibility
metadata into dedicated hierarchy IR. Empty documents match the upstream
grammar, while semantic validation rejects a second unindented root with the
pinned Mermaid diagnostic. Deterministic alternating partitions
lower through backend-neutral rectangles and glyph runs, with native
Metal-to-PNG validation. Named `classDef` declarations resolve fill, stroke,
dash, dash offset, fill/stroke/overall opacity, text color, and font styling
through semantic IR and backend-neutral paint, including CSS percentage opacity
values. Pixel and percentage class border radii likewise reach backend-neutral
rectangle geometry, with percentages resolving against the shorter node edge,
and `currentColor` node fills and strokes resolve against the final label color
before opacity composition. Three/four/eight-digit CSS hex colors and transparent
text colors preserve authored alpha through glyph and node opacity composition;
modern space/slash `rgb()`/`rgba()` colors survive Mermaid style parsing, while
backend-neutral color lowering also accepts legacy comma forms. Modern
space/slash `hsl()`/`hsla()` styles preserve hue units and alpha through semantic
IR before backend-neutral RGB conversion; lowering also accepts legacy comma
forms. Modern `hwb()` colors preserve hue units, whiteness, blackness, and alpha
through the same semantic and backend-neutral conversion path. Perceptual
`lab()` and cylindrical `lch()` colors preserve their components and alpha through
semantic IR before conversion from D50 Lab to backend-neutral sRGB paint. The
corresponding `oklab()` and `oklch()` forms preserve their perceptual components
and alpha before direct linear-sRGB lowering. CSS `color()` supports both encoded
`srgb` and linear-light `srgb-linear` profiles through the same semantic and
backend-neutral path, while `display-p3` components convert through D65 XYZ into
backend-neutral sRGB paint. The wider-gamut `a98-rgb` profile applies its specified
transfer curve and D65 matrix before the same backend-neutral conversion. The
D50-based `prophoto-rgb` profile applies its piecewise transfer curve and chromatic
adaptation before backend-neutral sRGB lowering. The `rec2020` profile likewise
uses its standard transfer curve and D65 matrix before backend-neutral lowering.
Direct `xyz`/`xyz-d65` and chromatically adapted `xyz-d50` profiles complete the
predefined CSS `color()` spaces on the same backend-neutral path. Across modern
color functions, the CSS missing-component keyword `none` survives semantic IR
and resolves to zero when lowered to absolute backend-neutral paint; an omitted
alpha component remains fully opaque, while an explicit `/ none` resolves to
transparent alpha. CSS `color-mix()` likewise preserves authored stops through
semantic IR and supports premultiplied-alpha interpolation in the `srgb` color
space, including omitted, complementary, and normalized stop percentages. The
`srgb-linear` interpolation space decodes channels to linear light before the
same alpha-aware mixing and encodes the result back to backend-neutral sRGB.
The perceptual `oklab` interpolation space similarly converts sRGB stops into
OKLab before mixing and lowers the mixed result through linear sRGB paint.
The cylindrical `oklch` interpolation space preserves the same perceptual
lightness and chroma while applying shortest-path hue interpolation, including
powerless achromatic hues, before backend-neutral lowering.
The cylindrical `hsl` interpolation space likewise applies shortest-path hue
interpolation and achromatic hue fixup while alpha-premultiplying saturation
and lightness before conversion to backend-neutral sRGB.
The `hwb` interpolation space shares that angular behavior while mixing
whiteness and blackness with premultiplied alpha and treating near-achromatic
whiteness-plus-blackness sums as powerless hues.
The rectangular D50 `lab` interpolation space converts sRGB stops through
chromatic adaptation before premultiplied-alpha mixing and converts the result
back to backend-neutral sRGB paint.
The cylindrical D50 `lch` interpolation space adds shortest-path hue
interpolation and powerless low-chroma hue fixup to that same backend-neutral
conversion path.
The encoded `display-p3` interpolation space converts sRGB paint into the
wider D65 gamut for premultiplied-alpha mixing, then lowers the result back to
backend-neutral sRGB paint instructions.
The encoded `a98-rgb` interpolation space applies Adobe RGB's sign-preserving
gamma around D65 XYZ conversion while retaining the same alpha and
backend-neutral lowering semantics.
The encoded `prophoto-rgb` interpolation space applies its piecewise transfer
function around D50 XYZ conversion, including chromatic adaptation to and from
the backend-neutral sRGB paint space.
The encoded `rec2020` interpolation space applies its sign-preserving transfer
function around D65 XYZ conversion before backend-neutral paint lowering.
The rectangular `xyz` and `xyz-d65` interpolation aliases mix directly in D65
XYZ, while `xyz-d50` applies chromatic adaptation before interpolation; all
three retain premultiplied-alpha and backend-neutral lowering semantics.
Polar HSL, HWB, LCH, and OKLCH interpolation accepts the CSS `shorter`,
`longer`, `increasing`, and `decreasing hue` methods, with `shorter hue`
remaining the default and powerless-hue fixup applied before interpolation.
The complete CSS named-color set likewise resolves to explicit
backend-neutral RGB paint for node fills, strokes, labels, and opacity composition,
including gray/grey aliases and `rebeccapurple`.
The SVG/CSS `none` paint keyword remains distinct from text color semantics and
lowers node fills and strokes to explicit backend-neutral transparency, including
when opacity declarations are present.
Relative `bolder` and `lighter`
font weights resolve against the default weight, integer weights from 1 through
1000 survive into native variable-font matching, and `oblique` font style lowers
through the existing backend-neutral italic face selection. Positive pixel,
em, rem, and percentage font sizes retain their authored absolute or relative
semantics through label and value layout. CSS absolute-size keywords use stable
native pixel equivalents, while `smaller` and `larger` preserve relative scaling.
Nine named font-stretch widths and positive percentages
flow through nearest-width native font matching without introducing backend-specific paint behavior. Class text alignment and
none/upper/lower/capitalize/full-width/full-size-kana transforms and none/underline/overline/line-through
decoration combinations, including independently authored decoration colors,
solid/double/dotted/dashed/wavy decoration styles, and auto, from-font, pixel,
em, and percentage decoration thicknesses also survive into shaped
glyph runs. The CSS decoration shorthand composes those line, style, color, and
thickness values into the same semantic representation, and the
`text-decoration-line` longhand supports none and composable underline,
overline, and line-through values. Decoration `currentColor` remains semantic
until it resolves against the final label color. Auto, pixel, em, and percentage underline offsets follow the same
backend-neutral geometry path. Auto, from-font, and under underline positions select metric or descent-based
backend-neutral placement. Normal, unitless,
percentage, and pixel line heights flow through the same text layout path.
Pixel and percentage text indents adjust only the first backend-neutral line box
before glyph shaping while `hanging` inverts the target and `each-line` restarts
the target after forced breaks. Subsequent lines otherwise retain the authored box. Normal,
nowrap, pre, pre-wrap, pre-line,
and break-spaces white-space modes control collapsing, preservation, and native
line wrapping. Break-spaces keeps every ASCII separator and exposes each one as
a backend-neutral wrap opportunity. The white-space-collapse longhand composes
collapse, preserve, preserve-breaks, preserve-spaces, and break-spaces into the
same semantic modes. Preserve-spaces converts tabs and segment breaks to spaces,
retains authored space runs, and wraps only after a complete preserved sequence.
Auto, left/start, center, right/end, and
justify last-line alignment overrides flow into backend-neutral line positioning
and glyph spacing.
Normal, break-word, and anywhere overflow wrapping preserve ordinary Unicode
line breaks and optionally split oversized words at grapheme boundaries; the
legacy `word-wrap` alias maps into the same typed modes.
Normal, break-all, and keep-all word-breaking modes respectively preserve
Unicode opportunities, add grapheme opportunities, or suppress CJK-internal breaks.
Auto, loose, normal, strict, and anywhere line-breaking modes preserve Unicode
defaults, tailor small-kana opportunities, or add grapheme opportunities.
None and manual hyphenation modes suppress discretionary breaks or expose
authored soft hyphens as backend-neutral wrap opportunities. Manual soft
hyphens remain invisible unless selected as a line ending; dictionary-backed
automatic hyphenation remains explicitly unsupported. Auto and quoted custom
hyphenate-character values choose the glyph inserted at an authored break.
Clip and ellipsis text-overflow modes truncate oversized labels at grapheme
boundaries, with ellipsis insertion remaining backend-neutral until shaping.
Wrap and nowrap text-wrap modes independently enable or suppress soft wrapping
through semantic text metadata while retaining authored hard line breaks.
Auto, stable, and balance text-wrap styles retain greedy wrapping or redistribute text
across the existing line count toward even backend-neutral line widths.
Stable wrapping matches auto for an initial stateless render and preserves its
typed intent for future incremental layout state.
Pretty wrapping additionally moves a trailing word when that reduces final-line
raggedness without exceeding the authored line width.
The text-wrap shorthand composes wrap, nowrap, balance, pretty, and stable into the
same typed mode and style fields before backend-neutral lowering.
Justified text alignment distributes remaining line width across ASCII word
separators on non-final lines before backend-neutral glyph placement.
Auto and inter-word justification use those separator advances,
inter-character justification distributes the width across shaped glyph gaps,
and none suppresses expansion while retaining the authored alignment.
Justified last-line alignment applies that distribution to the final line while
preserving the authored alignment of earlier lines.
Normal and positive or negative pixel/em letter spacing adjusts shaped glyph
positions and line measurement through backend-neutral PaintInstructions.
Normal and positive or negative pixel/em word spacing likewise adjusts ASCII
word-separator advances, line measurement, wrapping, and alignment before paint.
Left-to-right and right-to-left class directions select the corresponding
Unicode text-flow analysis and native shaping direction before glyph lowering.
Ordered text-shadow lists with pixel offsets and optional blur lower to
backend-neutral drop-shadow filter layers around the shaped glyph instructions;
omitted colors and explicit currentColor resolve against the final label color.
Integer tab sizes from zero through 256 expand preserved tabs before shaping;
normal/default and pre-line whitespace continue to collapse tabs as CSS spaces.
All twelve `cScale`,
`cScalePeer`, and `cScaleLabel` theme-variable slots
also flow from init directives or YAML front matter through category-aware
layout into paint, with the hidden root and inherited leaf category behavior of
the upstream renderer. Common D3 value-format families lower natively, including
grouped and fixed decimals, currency, significant digits, scientific notation,
percentages, SI prefixes, radix output, signs, trimmed zeroes, alternate radix
prefixes, width, fill, and left/right/center/sign-aware alignment. Exact D3
locale-specific separators and the broader CSS property surface remain
unsupported at the partial level. Treemap layout, value visibility, font, and
border configuration also flow through semantic IR, hierarchy layout, and paint.
Explicit section and leaf fill, stroke, and stroke-width options, independent
label/value colors, and title color/size follow the same backend-neutral path;
named class styles retain precedence over those diagram-wide node options.

### Venn Native Slice

The initial Mermaid 11.16.1 Venn slice parses declared sets, validated
multi-set unions, labels, numeric sizes, attached text nodes, titles, and the
documented fill, stroke, opacity, and text styling directives into dedicated
set IR. Deterministic circle geometry lowers through backend-neutral ellipses
and glyph runs, with native Metal-to-PNG validation. Exact area-proportional
overlap optimization, theme-exact styling, configuration overrides, and
interactive behavior remain unsupported at the partial level.

### Ishikawa Native Slice

The initial Mermaid 11.16.1 Ishikawa slice recognizes `ishikawa` and
`ishikawa-beta`, takes the first content line as the effect, and preserves
subsequent indentation as dedicated causal-tree IR. Deterministic alternating
fishbone layout lowers through backend-neutral paths, a rounded effect box,
and glyph runs, with native Metal-to-PNG validation. Exact upstream theme and
configuration parity, adaptive collision avoidance for very wide or deeply
nested trees, and interactive behavior remain unsupported at the partial
level.

### Wardley Native Slice

The initial Mermaid 11.16.1 Wardley slice recognizes `wardley-beta` and parses
titles, canvas sizes, custom evolution stages, anchors, components,
visibility/evolution coordinates, validated dependency links, and evolution
targets into dedicated strategic-map IR. Deterministic Cartesian mapping
honors Wardley's `[visibility, evolution]` coordinate order and lowers through
backend-neutral paths, ellipses, and glyph runs with native Metal-to-PNG
validation. Pipelines, notes, annotations, accelerators/deaccelerators,
advanced flow ports, decorators, label offsets, custom stage boundaries, and
theme-exact rendering remain unsupported at the partial level.

### Cynefin Native Slice

The initial Mermaid 11.16.1 Cynefin slice recognizes `cynefin-beta` and its
colon-terminated form, then parses the five fixed domains, quoted domain
items, titles, and labeled cross-domain transitions into dedicated domain-map
IR. Self-loop transitions are discarded to match upstream semantics. Fixed
semantic quadrant and center-ellipse layout lowers through backend-neutral
rectangles, ellipses, paths, and glyph runs with native Metal-to-PNG
validation. Theme/config overrides, organic seeded boundary waviness, exact
cliff styling, transition arrowheads, accessibility directives, and overflow
badges remain unsupported at the partial level.

### TreeView Native Slice

The initial Mermaid 11.16.1 TreeView slice recognizes `treeView-beta` and
parses indentation-based and standard/heavy box-drawing hierarchies into
dedicated tree IR. File/directory identity, quoted and bare labels, title and
accessibility metadata, `:::class`, `icon()`, and `##` descriptions survive
through deterministic row layout and backend-neutral connector, highlight,
marker, and glyph PaintInstructions with native Metal-to-PNG validation.
Configuration-driven default and filename/extension icon maps, external icon
pack artwork, custom row geometry, exact typography, and interactive behavior
remain unsupported at the partial level.

### Railroad Native Slice

The initial Mermaid 11.16.1 Railroad slice recognizes `railroad-beta` and
parses the upstream explicit constructor notation into dedicated recursive
grammar IR. Terminals, nonterminals, sequences, choices, optional elements,
zero-or-more and one-or-more repetitions, special elements, rule names, titles,
and accessibility metadata survive deterministic branch-and-loop layout and
backend-neutral path, ellipse, rectangle, and glyph PaintInstructions with
native Metal-to-PNG validation.

The textual `railroad-ebnf-beta` dialect now has its own portable token and
parser grammars. W3C and ISO choices, sequences, groups, optional elements,
zero-or-more and one-or-more repetition, special sequences, exceptions, both
rule-assignment spellings, comments, titles, and accessibility metadata lower
to the same recursive semantic IR, deterministic layout, and backend-neutral
PaintScene path as constructor notation, with native Metal-to-PNG validation.

The textual `railroad-abnf-beta` dialect likewise uses dedicated portable
grammars and lowers string literals, numeric values and ranges, rule
references, concatenation, alternation, groups, optional groups, and exact or
bounded repetition into the recursive Railroad IR. Repetition bounds remain
semantic even though the pinned renderer draws every repeat with the same
loopback convention. A pinned syntax corpus and native Metal-to-PNG fixture
exercise the complete backend-neutral path.

The textual `railroad-peg-beta` dialect completes the four-notation parser
surface with dedicated portable grammars. Literals, identifiers, ordered
choice, sequences, groups, wildcard matches, lookahead predicates, and suffix
operators lower to the same recursive Railroad IR. Predicate labels preserve
the pinned transformer's special-node convention, and a pinned syntax corpus
plus native Metal-to-PNG fixture exercise the backend-neutral path.

This is intentionally partial. Upstream curve geometry, theme/config
overrides, and exact typography remain unsupported rather than being counted
as compatible.

### Info Full Compatibility

Mermaid 11.16.1 Info accepts exactly `info` and `info showInfo`. Both forms
lower to dedicated version IR carrying the pinned `11.16.1` release, then to
the upstream fixed 400-by-100 layout and a backend-neutral glyph run reading
`v11.16.1`. The pinned corpus covers both accepted forms and unsupported
suffixes, and the native Metal-to-PNG path validates the complete family.

### ZenUML Partial Compatibility

The native ZenUML frontend accepts the documented title, participant ordering,
aliases, Actor and Database annotators, and asynchronous `A->B: message` forms.
It lowers into the shared Sequence IR, layout, and backend-neutral PaintScene
pipeline, with a Metal-to-PNG fixture. Mermaid 11.16.1 delegates the broader
language to external `@zenuml/core`; synchronous nesting, creation and reply
messages, comments-as-notes, and control fragments remain unsupported and fail
grammar validation rather than degrading silently.

### Structural Groups

Nested containers such as C4 boundaries are semantic structural groups, not
ordinary nodes and not backend-specific paint primitives. A structural group
records its parent group, while member nodes record their immediate group.
Layout computes nested group bounds from child nodes and groups, then lowers
the result through existing rectangle, stroke, and glyph PaintInstructions.
This model is reusable for package boundaries, deployment nodes, clusters, and
future diagram families with nested visual containment.

## Grammar Source Of Truth

Shared grammar files live under:

```text
code/grammars/mermaid/
  compatibility.json
  <family>.tokens
  <family>.grammar
```

The grammar files are the portable syntax source for Rust, TypeScript, Go,
Python, Ruby, and future implementations. Language packages may contain
semantic AST builders and lowerers, but must not redefine the accepted syntax
with an unrelated handwritten parser.

Some Mermaid families use embedded encodings rather than ordinary grammars:

- Sankey contains RFC 4180-like CSV rows.
- Packet diagrams contain bit-range declarations.
- Railroad variants embed grammar dialects.
- Directives and YAML front matter are preprocessing layers.

Those layers should use their existing shared parsers or dedicated portable
grammars and then feed the family AST.

## Conformance Corpus

Each family needs three fixture tiers:

1. `smoke`: one minimal source proving detection through native paint.
2. `syntax`: focused fixtures for every documented production and alias.
3. `visual`: representative diagrams compared with tolerant geometry or image
   metrics rather than byte-identical PNGs.

The upstream Mermaid package at the pinned tag is the behavioral oracle. CI may
run it as a development-only oracle, but production rendering must not invoke
Mermaid JavaScript or round-trip through SVG.

Every upstream version bump must:

1. update `compatibility.json`;
2. report added and removed families, aliases, and productions;
3. leave newly introduced behavior explicitly `detected` until implemented;
4. never silently claim the previous compatibility level.

## Cross-Cutting Mermaid Features

Family syntax is only part of compatibility. The shared frontend also needs:

- YAML front matter and Mermaid directives;
- `title`, accessibility title, and accessibility description;
- comments and Unicode text;
- Markdown strings and entity decoding;
- theme variables and class/style declarations;
- links, callbacks, tooltips, and security-level policy;
- icon and image references;
- deterministic IDs and stable layout options.

Interactive features should lower into scene metadata or document actions.
They should not become backend-specific paint instructions.

## Paint And Backend Policy

New Mermaid concepts should become paint instructions only when they describe a
general graphics primitive. Nodes, participants, tasks, commits, and domains
remain in Diagram IR.

Likely general-purpose paint needs include:

- text decoration and multiline text metrics;
- dashed and patterned strokes;
- robust cubic and arc paths;
- gradients;
- image/icon drawing;
- link and hit-test metadata;
- clipping and nested transforms.

Backend parity is tracked separately from parser compatibility. A family is not
Level 4 until its representative scene renders without major degradation on
the target backend.

## Delivery Order

1. Complete Flowchart syntax while retaining its current graph pipeline.
2. Add Sankey, GitGraph, ER, and C4 parsers against existing IR capacity.
3. Add Sequence IR, layout, paint lowering, and both Sequence/ZenUML frontends.
4. Add State and hierarchy domains.
5. Add chart, chronology, board, and systems-modeling families.
6. Add specialized and beta families.
7. Close cross-cutting configuration, styling, accessibility, interaction, and
   visual-parity gaps.

Full compatibility is achieved when every manifest family is Level 5 for the
pinned Mermaid release and unsupported syntax fails explicitly rather than
silently degrading.

## Swimlane Partial Compatibility

The native Mermaid 11.16.1 Swimlane slice recognizes `swimlane-beta` with all
five directions, top-level `subgraph` lanes, common process-node shapes,
directed/undirected/dotted/thick chained links, link labels, titles, and
accessibility metadata. It lowers through dedicated ownership IR and stable
lane geometry before producing backend-neutral paint instructions.

This is intentionally partial. Nested subgraphs, the complete Flowchart shape
and link catalog, classes and inline styles, clicks, configuration-driven lane
ordering and line hops, and exact upstream routing or typography remain
unsupported rather than being counted as compatible.
