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
Full date-format and implicit/sequential task syntax plus the pinned
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
