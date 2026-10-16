# mermaid-parser

Railroad `compactMode` accepts case-insensitive true/false through the existing
boolean parser; absence or an empty extracted value defaults to false. Other
present values have an exact typed error. An init directive selects the whole
Railroad object before front matter, so a missing directive field uses its
default rather than merging a front-matter field. Compact state and showMarkers
remain independent. All four dialects retain the complete configuration.

All four Railroad dialects resolve themeVariables from init directives or
top-level/config YAML front matter before explicit Railroad overrides. The
selected effective font size must be finite and in (0,65536], including theme
fallbacks; a valid explicit size overrides an excessive theme size. Unparseable,
nonfinite and nonpositive theme numbers retain the producer's default fallback.
Explicit-field numeric error domains and precedence remain unchanged.

Authored color and font-family strings use the producer's trimmed lexical
allowlists. A selected theme primary/secondary/default is sanitized against the
field default, then an invalid explicit value uses that sanitized theme. An
invalid selected primary does not retry secondary. Marker fallback follows the
theme line independently of an explicit line override. These filters do not
certify complete CSS/color/font grammar or sanitization of caller-forged typed IR.

All four Railroad dialects preserve padding and vertical/horizontal separation
from init directives or YAML front matter. Every setting must be finite and in
inclusive0..65536; unsupported values return an exact field-specific parse
error. Accepted values, including zero and the cap, are retained without
clamping before checked layout and PaintScene lowering.

Versioned Mermaid compatibility dispatcher and grammar-driven Rust parsers.

Swimlane curve-only configurations share the bounded authored edge-ID index
with animation, class and style directives. Per-field updates preserve earlier
settings and exact authored order; operation-count controls distinguish edge
discovery, indexed lookups and real matching updates.

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
- `timeline`
- `mindmap`
- `block` / `block-beta`
- `packet` / `packet-beta`
- `kanban`
- `architecture` / `architecture-beta`

The Architecture subset parses groups, services, junctions, declared containment,
standard icon identifiers, titles, sibling row/column alignment hints, and undirected or directed edge syntax into the
shared structural semantic IR. Existing structural layout and PaintScene
lowering provide deterministic backend-neutral group, node, junction, edge, and shaped
text rendering. Quoted service icon text is preserved as typed metadata, reserves
layout geometry, and lowers to a backend-neutral badge plus shaped glyphs. Left,
right, and bidirectional arrowheads survive semantic and layout IR and lower to
backend-neutral path geometry. Group-edge modifiers on contained services resolve
relationship endpoints to deterministic group-boundary geometry. Explicit `L`,
`R`, `T`, and `B` ports survive semantic and layout IR and anchor relationship
geometry to the requested node or group boundary. Architecture edges resolve to
deterministic orthogonal polylines before backend-neutral PaintScene lowering.
Named service and group icons lower to canonical backend-neutral glyph geometry.
Namespaced icon identifiers survive typed IR and use a generic glyph when vendor
artwork is unavailable. Orthogonal Architecture edges deterministically avoid
unrelated service, junction, and group bounds before backend-neutral lowering. Mermaid
`init` directives and YAML front matter preserve Architecture `iconSize`,
`fontSize`, `nodeSeparation`, `padding`, and `idealEdgeLengthMultiplier` as typed
semantic configuration; layout resolves them into backend-neutral node geometry,
service typography, deterministic spacing, alignment-hint distances, group insets,
and outer canvas margins. Full vendor icon artwork,
randomized layout and the remaining fcose-specific tuning controls remain outside
the partial compatibility claim.

The initial `kanban` subset preserves indentation-defined columns and cards,
including plain labels and explicit `id[label]` forms, in typed board IR.
`diagram-layout-board` resolves deterministic column/card geometry before a
dedicated backend-neutral PaintScene lowering path. Icons, classes, shape-data
metadata, styles, links, assignments, priorities, and field configuration
remain outside the partial compatibility claim.

The initial `packet` subset uses dedicated line-preserving token and parser
grammars. Contiguous absolute inclusive bit ranges with quoted labels, titles,
and accessibility metadata lower into typed packet IR. `diagram-layout-packet`
places fields on deterministic 32-bit rows before dedicated backend-neutral
PaintScene lowering. Relative ranges, row-spanning fields, configuration, and
styling remain outside the partial compatibility claim.

The initial `block` subset uses dedicated line-preserving token and parser
grammars. Column counts, flat node grids, explicit spaces, core node shapes,
titles, accessibility metadata, and directed labeled connections lower into
typed grid IR. `diagram-layout-grid` resolves stable authored slots into shared
graph geometry, which follows the existing backend-neutral PaintScene lowering
path. Nested blocks, spans, classes, and style directives remain outside the
partial compatibility claim.

The initial `mindmap` subset uses a dedicated indentation-preserving token
grammar and parser grammar. Plain, square, rounded, and circular nodes lower to
graph semantic IR with explicit parent-child edges, deterministic IDs, and
depth-resolved styles. The shared graph layout and diagram-to-paint path then
produce backend-neutral geometry and glyphs; a native Metal fixture validates
the complete pipeline to PNG. Unsupported mindmap decorations remain outside
the partial compatibility claim.

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
Pinned actor-property fixtures and the native renderer exercise authored links,
properties, and details references through PaintScene metadata without IR injection.

Timeline parsing covers the pinned core header directions, titles,
accessibility metadata, sections, periods, and ordered events. It lowers to
timeline-specific temporal IR rather than reusing Gantt task semantics.
Architecture parsing preserves native `-[label]-` and `-[label]->` edge labels
in structural semantic IR for deterministic layout and backend-neutral Paint
text lowering. Titles plus single-line or braced accessibility metadata use
dedicated grammar tokens and survive through the native pipeline.
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
Pinned central-connection fixtures gate source, destination, and dual endpoint
markers across solid, dotted, bidirectional, and cross message families.
Pinned text fixtures gate HTML break variants, explicit wrapping intent,
punctuation next to comments, and empty message labels through native shaping.
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
Control-block and branch labels preserve punctuation adjacent to comments, and
intentionally unlabeled `loop`, `opt`, `alt`, and `par` frames remain empty.
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

## Styled Swimlane label discovery work

The partial Swimlane syntax keeps solid, thick and dotted inline labels with
original operator precedence, point markers and minimum lengths. Each fixed
label delimiter form discovers maximal ASCII runs with one monotone byte cursor
and O(1) state, including nonterminating runs and absent suffixes. Labels remain
UTF-8 slices until the selected label is copied. The bound is local to this
operator discovery; it does not certify all lexer regex costs, node lookup,
layout or drawing allocation. Exact parser predicate tests cover the admitted
work bound and semantic corner cases; the independent layout duplicate-edge
index remains unchanged. See DG04's styled-label discovery contract and #17335.

Swimlane authored edge IDs also preserve metadata after multibyte whitespace
separators; identifier slicing advances over the complete UTF-8 scalar.

Class/style edge targets use one ordered ID index, retaining all duplicate-ID
matches and node/edge-ID collisions. Node-only targets avoid repeated full-edge
scans; actual matching updates and existing node/class/style costs remain separate.


Swimlane animation configurations, classes and inline styles share one authored
edge-ID index. Repeated directives visit their actual matching edges, retaining
duplicate-ID order and exact unknown-target refusals. Node-shape metadata on
edge endpoints and literal @{...} node labels retain their original contexts.
This bounds local edge discovery without claiming whole-pipeline linearity.

Two-point Swimlane Cardinal, CatmullRom and Natural controls are checked through
authored LR/TB parsing, layout and paint: exact collinear cubic controls, mode
metadata and visible marker triangles. CatmullRom aliases share the same semantic
variant. This bounded route support does not claim a general multi-point solver.

Line-hop/Qt integration controls assert exact public-layout sorted arc/gap routes
and marker triangles, mode aliases/exclusions, and Qt native output equivalence
with exact inset/unsupported shadow debt. Existing resource and geometry controls
remain required. No general renderer/device acceptance is inferred.

The maximal-run label scanner retains the imported point/circle/cross endpoint
operator families and positional link styles. Dispatch handles positional styles,
then complete edge chains, then standalone edge configurations; endpoint node
shape metadata therefore remains in its original context. Exact endpoint metadata
and counted label/edge-target controls remain enabled after integration.


## Bounded spacing and transport reconciliation

Swimlane flowchart spacing fields accept only finite values in 0..=65536. Existing negative/nonnumeric/nonfinite messages stay unchanged; excessive finite values report FIELD must not exceed 65536 at(1,1). Directive/front-matter precedence stays unchanged.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.


## Global curves and checked ranks

Swimlane global flowchart curve parsing retains directive/front-matter precedence, separate per-edge overrides, all existing aliases and the exact unsupported-curve refusal. Complete source fields are compared after normalizing only the proven global curve config.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.


## Native validation checkpoint

Native complete IR/layout/scene controls pin cluster and node/edge theme values, defaults, style priority and source precedence. Invalid theme stroke widths preserve defaults and zero is accepted. Producer Mac visual fixtures are retained; Windows evidence does not claim native Mac/device rendering.

The 14-package safety run passed834 tests across32 targets, explicitly
excluding only the new expected-red work test that was executed separately
(Cargo101,1PASS/1FAIL). Strict15-package lint passed. This is scoped evidence;
whole/downstream/hosted/merge acceptance remains withheld. Final-source Miri
is **DEFERRED/PENDING**.

## Bounded Venn canvas configuration

Venn carries width, height, padding, `useMaxWidth` and `useDebugLayout` through
owned semantic/layout types and eight `venn.*` scene metadata entries. Defaults
are 800/450/8/true/false. The two booleans are transport metadata in this partial
implementation. Source dimensions admit finite(0,65536], padding finite[0,65536];
an invalid field uses its own default. A closed single-line init object's direct
`venn` object replaces front-matter config as a whole. Nested objects, quoted
values and body/style text cannot supply its fields.

`diagram_layout_geometric::try_layout_venn` checks caller-owned configuration
before arithmetic and refuses nonfinite derived circle/label geometry with a
typed `VennLayoutError`. The single-argument `layout_venn` wrapper panics for
unsupported IR. Use the checked API for untrusted typed input. Placement clamps
effective padding to each canvas quarter while retaining authored admitted
padding in config/metadata. Supported placement arithmetic is unchanged.

Exact two-set/union/text geometry and complete default/custom PaintScene,
glyph vectors, ordered font queries, primitives, styles and metadata controls
are portable tests. Finite geometry does not establish label/stroke enclosure,
area-proportional overlap, theme/CSS parity, arbitrary font/style safety or a
renderer pixel-area/resource budget. See
[`CV04a-venn-config-main-reconciliation.md`](../../../specs/CV04a-venn-config-main-reconciliation.md).
Full final-source Miri is **DEFERRED/PENDING**, never passed; compiler delivery
and the full Closure parity/provenance objective remain separate gates.

## Bounded Venn theme transport

Venn reads root `themeVariables` front matter or a direct object in a closed
single-line init directive after the leading YAML header. A selected init object
replaces front matter wholesale, including an empty object. Direct mapping
fields supply values; header literals, nested maps/arrays, sequence items and
scalar sections cannot supply a theme. Active quotes preserve embedded other
quotes; malformed present values use defaults. The existing canvas/derived
bounds and unrelated readers remain intact.

Specific `vennTitleTextColor`/`vennSetTextColor` precede generic `titleColor`/
`textColor`; present rejected specific values use defaults. Defaults are
`sans-serif`, `#0f172a`, `#0f172a`. Configured family replaces caller option
families while retaining size, weight, italic and stretch. Authored label color
overrides the configured set color.

`venn1` through `venn8` independently override the eight default colors:
`#60a5fa`, `#f59e0b`, `#34d399`, `#f472b6`, `#a78bfa`, `#22d3ee`, `#fb7185`,
`#a3e635`. Rejected lexical values fall back per slot. Checked layout transports
all nine config fields. Circle fills use global circle order and cycle through
the palette; an authored fill wins without shifting later indices. Source-derived
scenes have sixteen exact metadata entries. The typed API supports arbitrary
palette lengths: metadata contains eight base entries plus every palette entry.
An explicitly empty typed palette gives every unstyled circle blue and retains
only the eight base metadata entries. This is not a universal typed memory cap
or a complete CSS validator.

Portable controls verify 104 complete scenes: 32 retained oracles with a
reversible eight-entry metadata adaptation, 48 source-to-checked-layout scenes
with complete font-query traces, and 24 typed palette/cycle/empty/override cells.
The affected fourteen-package suite passed 932 tests over 41 targets with no
failures or ignores, plus strict all-target lint, at source `55afa2b5`.
Independent source and terminal reviews passed. This establishes bounded
transport; physical-font fitting, rasterization and full style parity remain
open. Hosted checks and publication remain separate gates. Final-source Miri is
**DEFERRED/PENDING**, never passed, under delivery-first sequencing.

## Reviewed Venn property cascade

Present style fields overlay earlier values in authored order. Missing fields
retain earlier values; authored zero remains explicit. Region lookup precedes
text lookup, with the first duplicate key selected and union keys sorted.
Twelve complete IR cases cover three ingress forms and target/order controls;
192 complete scene/query cases retain176 earlier cases and add16 cascade cases.
These are bounded portable contracts, not full CSS/rendering or Mermaid parity.

Actual source `f630e6ad` passed 2,765 tests across 120 targets, zero failures,
one unchanged parent-only archive ignore, and strict all-target Clippy for all
20 affected packages. Compiler884 and full unfiltered LANG500 passed. Focused
runtime has 116 sources/696 actual cells on six available backends; all636 old
tuples and106 old sources stay exact. CLR is unavailable and uncounted.
Source/native-focused review19fd3975 and style-focused76b54bb9 passed; independent
full terminal review is pending. Hosted checks and publication remain separate.
Final-source Miri **DEFERRED/PENDING**, never passed.

## Bounded Venn source-style admission

Recognized invalid fill/stroke/text color, nonfinite or negative stroke width,
and opacity outside0..=1 reject the entire parse before target lookup. Unknown
targets and later repairing directives do not rescue invalid recognized values.
Missing properties still retain earlier cascade fields; authored zero remains
explicit. Invalid theme values keep their separate existing fallback behavior.

Color admission uses the existing lexical subset, not a CSS name dictionary,
function-arity/range validator or general renderer-safety guarantee. Existing
source tokens reject percent signs even inside quoted style values; explicit
tests preserve that boundary. Width has no new upper cap. Direct typed IR remains
its existing caller trust boundary; this is a source parser contract.

Actual source `df058580`:957 passed/zero failures/zero ignores across43 targets
in all14 affected packages, with strict all-target warnings/unknown-lints denial.
Tests assert918 exact rejection scenarios,342 complete positive IR cases,
two percent-tokenizer errors,12 original invalid-cascade errors plus12 valid
whole-IR companions, exact external corpus style and208 complete scene/query
cases (192 retained plus16 RGBA alpha/raw-stroke cases). Original invalid JSON
fills remain exact errors; title companions retain complete config/theme oracles.
Independent exact source/focused and terminal reviews passed.

Native execution remains attributed to `f630e6ad`:actual full20 run2765/120 with
one unchanged ignore, compiler884, full unfiltered LANG500 and696 runtime cells
on six available backends. Native166 package inputs differ only in four named
Markdown files;21 mapped external resources remain exact. Static resource maps
include conservative test routes and do not close arbitrary dynamic/toolchain/
OS inputs. Historical Closure36/3705 attribution is not new execution.

Required hosted checks and actual merge remain separate. Final-source Miri
**DEFERRED/PENDING**, never passed; full Closure/visual/provenance completion
remains OPEN.

## Bounded Ishikawa label boxes

Ishikawa bones retain label width clamp(7*UnicodeScalarCount+20,72,180), height28,
nested offset18 and declared-depth canvas height. Paint emits each bone and its
configured rectangle before effect box/text with6/2 insets. All37 complete
source/IR/layout/scene/query cells retain their accepted boxed-label semantics.
General fontfit, arbitrary nesting and canvas containment remain OPEN.

## Closed Wardley themes and checked notes

The Wardley parser selects one closed init or direct frontmatter theme root.
Direct wardley fields override sanitized background/primary-text root defaults;
an empty selected init root replaces frontmatter wholesale. Complete active-quote
decoding precedes the existing color whitelist. Nested mappings, sequence items,
block literals, inactive-quote prefixes and trailing junk cannot supply a value.
The nine theme fields drive background, axes/text/grid, components, links and
evolutions. This is lexical source admission; typed theme strings do not gain a
renderer-wide CSS guarantee.

Notes preserve quoted interior text, including spaces, empty strings, Unicode,
inactive quotes, brackets and percent-percent. Only the active delimiter and
backslash may be escaped. Coordinates and outside comments are read after the
closed token; malformed input has exact physical source errors. Typed finite
normalized note coordinates have a named NoteCoordinates refusal after
EvolutionTarget and before reference checks. Every derived note point is finite.
Paint notes use the retained caller family/italic/stretch with weight700/size11;
ordinary caller weights and the independent numeric/font/query tables remain.
All17 metadata fields and complete ordered cached font queries are checked.

Specd629/d70b precedes parser tests08b, closed main61 merge10e and bounded source
repairs. Genuine semantic reds4a1/53d9/2d858 remain failed receipts. Generated-tag
and removed-node-reader compile failures had zero executed tests; neither is a
semantic-red receipt. The unchanged node reader was restored whole. The ABI-only
five-file migration and old eight-scene migration reverse byte-exactly to their
original modules; no numeric/font/query oracle was inferred from output. The
new eight complete scene/query cells were committed e81 before first execution
and passed on their first render. The filtered empty-binary wrapper failure and
the v1 wrong-baseline-path preflight failure remain preserved separately.

Actual tested source33ac7884: focused8 parser/40 geometry/3 Paint tests PASS,
including old94 IR/2 body errors, new83 theme+12 note complete IR/21 exact errors,
24 legacy+24 named note refusals/4 priorities/6 forged derived-point refusals,
and16 whole source/IR/layout/scene/all-glyph/metadata/query cells (old8+new8).
Full affected14 packages989P0F0I/50 targets and strict all-target warnings and
unknown-lints denial both exit0. Tests269276ba/strict0925ef46 and independent
source/terminal reports bind that exact source. All1469 raw Rust/TOML files,
reverse14/forward50/style835 objects, registry376, four local style-version
changes, complete native2853 attribution and21+3 static resources are bound.
Retained13 Venn fixture files contain232 complete cells;37 Ishikawa cells and67
causal parser IR controls remain exact. Human snapshot changes are integrated
as pure incoming paths; no TS/book execution is claimed.

Native execution remains attributed to e3e1106c:1836P0F/1 unchanged ignore over
78 targets, compiler895/LANG506/strict6,146 programs/876 actual six-backend cells.
Historical Closure36/3705+3 retains316e attribution, not fresh execution. This
carrier integrates main61eebb03. Later main4f383fea native15 is NOT integrated
or validated; its raw shared-helper widenings remain WITHHELD under independent
incoming review5cf99164. Dedicated bounded reconciliation is the next slice.

This is unmerged progress in solePR17288. Current-head hosted CI/conflicts/
reviews/actual merge and the macOS phase-refusal root cause remain OPEN.
General fontfit, containment, area/node-count limits and forged resolved Paint
layouts remain outside this bounded contract. Base ESTree canonical serialized
boundary/own types/no new crates.io dependencies remain owner decisions. Full
Closure parity/output-byte-source/full fold/rename/inline/motion/merge/delete
lineage/stable IDs/spans/pass-sweep/CV04b remain OPEN. Final-source Miri is
**DEFERRED/PENDING**, never passed; frozen checkout/run/inputs/target/receipts
untouched/unpolled, no duplicate/termination. Delivery first, genuine Miri later;
the heartbeat remains ACTIVE until the full completion contract is met.
