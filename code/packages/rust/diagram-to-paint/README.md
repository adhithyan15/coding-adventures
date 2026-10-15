# diagram-to-paint

Railroad paints rule labels as bold `name =` and specials as `? text ?`.
Real font resolution requests the bold rule font and caches the distinct normal
leaf font. Rule-name boxes retain max(28, fontSize * 1.15), centered on the
explicit root port, and all labels retain no-wrap behavior. Declared marker
points drive ellipse centers. The new 192 four-dialect configurations compare
complete glyph vectors, ordered font queries, layout/marker coordinates and all
twelve metadata entries. Separate retained style controls cover wider primitive,
path and drawing-order behavior. Full affected fourteen-package validation
passed 902 tests over 38 targets and strict lint. Miri is DEFERRED/PENDING.

Railroad scenes retain `railroad.compactMode` as normalized true/false metadata.
All 196 previous complete metadata cells keep their eleven original fields and
add the false default key. Forty compacttrue controls retain bounded primitive,
style, glyph, path, marker, order and actual font-query expectations; some
geometry expectations use actual layout fields. They are not independent
absolute-layout goldens or direct whole-scene mode-equivalence tests. Separately,
the compact flag adds no layout branch. Subsequent circular-corner changes and
new display labels intentionally update the documented path and label geometry.

Railroad paints resolved portable styles and scales declared leaf/rule-name
text boxes with font size. Portable controls preserve 40 explicit-style scenes,
40 theme-fallback scenes and 40 sanitized-style scenes with complete ordered
glyph/path/shape/marker traces, real font queries and all twelve metadata keys.
Arc radius zero retains straight polylines; positive radius lowers orthogonal
Railroad corners to cubic approximations of circular arcs, clamped to half the
adjacent lengths. Numeric controls cover 24 orientation/radius cases and 65
radial samples each; eight authored choice/bypass/repetition scenes have
absolute coordinate/control/command goldens and full 12-key metadata across
both compact values. Five public short/collinear/reversal cases retain their
commands. Four old positive corner cells intentionally change quadratic to
cubic; the other six arc cells remain. Display-label reconciliation updates
only documented label, font-query and marker semantics in prior scene oracles.
Swimlane and other diagram lowering are unchanged. This is
bounded portable evidence, not full Mermaid geometry/backend visual parity
or arbitrary nonfinite/extreme/oblique caller-forged layout safety.
Explicit marker centers now meet the root port in authored controls and name
columns reserve the display-label estimate. Marker/stroke enclosure, reversed
connectors for large radii, general empty/large-font fitting, arbitrary custom-
font and long-label fitting remain open. Apple raster execution is a separate gate.

Railroad leaves with zero/small padding keep their declared text box inside the
leaf before glyph shaping. Complete ordered default-font glyph traces and
actual metric bounds are checked through all four source dialects, along with
exact spacing metadata. Custom-font/long-label sizing and arbitrary forged
layout IR remain separate limitations; the checked semantic-layout boundary
is `diagram_layout_hierarchy::try_layout_railroad`.

DG03 — Converts layouted graph, chart, structural, temporal, sequence, packet,
and geometric diagrams into a `PaintScene` for rendering by `paint-metal` and other
backends. Text rendering is **delegated to `layout-to-paint`**
so every paint backend receives real glyph IDs from the TXT00 shaping pipeline.

Swimlane cubic and Step endpoint markers use the first distinct control or
waypoint in endpoint-relative order. Aligned Bump/Step routes retain visible
directed triangles without changing their authored curve commands or metadata.
Authored parser/layout/paint geometry tests run on every platform, checking
exact triangle vertices, area and curve commands. Entirely coincident routes
keep their existing directionless marker behavior.

Swimlane line hops query every eligible orthogonal route segment before inserting
any segment from the current edge, preserving the exclusion of self-crossings.
The geometry index retains prior edge/segment identities and duplicate drawing
multiplicity; same-axis cohorts are excluded before candidate traversal. Exact
whole-scene controls cover all 36 prior/current combinations of None, Linear,
Rounded, Step, StepBefore and StepAfter in both arc and gap modes (72 scenes),
including intermediate segment boundaries, endpoint markers, styles and metadata.
The existing duplicate and same-axis work bounds remain required. Arbitrary
distinct oblique routes have separate, still unaccepted candidate-work costs.

## Usage

```rust
use dot_parser::parse_to_diagram;
use diagram_layout_graph::layout_graph_diagram;
use diagram_to_paint::{DiagramToPaintOptions, diagram_to_paint};
use layout_ir::{Color, font_spec};
use text_native_coretext::{CoreTextResolver, CoreTextMetrics, CoreTextShaper};

let shaper   = CoreTextShaper::new();
let metrics  = CoreTextMetrics::new();
let resolver = CoreTextResolver::new();

let diagram = parse_to_diagram("digraph G { A -> B -> C }").unwrap();
let layout  = layout_graph_diagram(&diagram, None, None);

let opts = DiagramToPaintOptions {
    background:          Color { r: 255, g: 255, b: 255, a: 255 },
    device_pixel_ratio:  1.0,
    label_font:          font_spec("Helvetica", 14.0),
    title_font:          { let mut f = font_spec("Helvetica", 18.0); f.weight = 700; f },
    shaper:              &shaper,
    metrics:             &metrics,
    resolver:            &resolver,
};
let scene = diagram_to_paint(&layout, &opts);

// scene is ready for paint-metal::render(&scene)
println!("Scene: {}×{} with {} instructions",
    scene.width, scene.height, scene.instructions.len());
```

## What each element produces

| Element          | PaintInstruction(s)                              |
|------------------|--------------------------------------------------|
| Rect node        | `PaintRect` (corner_radius = 0)                  |
| RoundedRect node | `PaintRect` (corner_radius from style)           |
| Ellipse node     | `PaintEllipse`                                   |
| Diamond node     | `PaintPath` (4-vertex diamond polygon)           |
| Node label       | `PaintGlyphRun` (real glyph IDs via TXT00)       |
| Edge             | `PaintPath` (polyline)                           |
| Arrowhead        | `PaintPath` (filled triangle)                    |
| Edge label       | `PaintGlyphRun` (real glyph IDs via TXT00)       |
| Diagram title    | `PaintGlyphRun` (real glyph IDs via TXT00)       |
| Sequence lifeline/message | `PaintPath` (solid or dashed)             |
| Sequence note/activation  | `PaintRect`                               |

## Painter's algorithm order

1. Edge lines and arrowheads (behind nodes).
2. Node shapes (filled, covering edge endpoints).
3. All text via `layout-to-paint` (on top).

## Text pipeline

All text is rendered by building a `PositionedNode` tree and calling
`layout-to-paint::layout_to_paint` **once** at the end. The shaper produces
real font-specific glyph IDs — not Unicode codepoints — so Metal, Direct2D,
SVG, and Canvas backends all produce correct, readable text.

## Spec

[DG03 — Diagram to Paint](../../../specs/DG03-diagram-to-paint.md)

Two-point Swimlane Cardinal, CatmullRom and Natural controls are checked through
authored LR/TB parsing, layout and paint: exact collinear cubic controls, mode
metadata and visible marker triangles. CatmullRom aliases share the same semantic
variant. This bounded route support does not claim a general multi-point solver.

Rounded Swimlane controls verify complete line/quadratic routes and both exact
endpoint triangles for long and short LR/RL/TB/BT layout fixtures. Authored
aligned LR/TB layouts and collapsed perpendicular/zero routes preserve the
bounded marker-reference contract; genuine zero routes retain degenerate markers.

Line-hop/Qt integration controls assert exact public-layout sorted arc/gap routes
and marker triangles, mode aliases/exclusions, and Qt native output equivalence
with exact inset/unsupported shadow debt. Existing resource and geometry controls
remain required. No general renderer/device acceptance is inferred.

Swimlane hop discovery indexes exact directed geometry and keeps every drawing
occurrence. Same-axis cohorts are excluded before candidate iteration; duplicate
geometry expands crossings back into original order/multiplicity. Counted authored
duplicate/Cartesian controls and golden repeated-crossing commands guard this
contract. Arbitrary unique oblique candidate work has no global sweep-line bound.


Distinct oblique line-hop geometry has a bounded integer cohort optimization:
all coordinates must be integral with magnitude at most2^24 and share the
same pair of x supports or y supports. Safe own AVL trees query endpoint-order
inversions before the unchanged crossing predicate, retaining directed geometry
bits and each original edge/segment occurrence. A noneligible geometry
invalidates its cohort once and retains the existing fallback. Complete scenes
and counted index work cover both axes, orientations and hop modes through8192
distinct segments, plus authored parser/layout scenes. The bound includes
geometric inversions; general arbitrary-support/floating work remains open.

An additional integer cohort allows different right supports when the left
support is common. Exact rational slope ordering and conservative bounds at
both right endpoints prune candidates; the original directed crossing
predicate still decides every returned hop. Geometry ordinals restore legacy
occurrence order. Certificate, observation, AVL maintenance, query, comparison,
rotation, drop, candidate, sort and occurrence counters describe this bounded
index; general geometry-key, cut and floating fallback costs remain separate.
Eight independent controls cover raw directed bits/order, zero slope, endpoint
equalities, extreme coordinates, minimal span, permanent invalidation, exact
candidate counts and independently recomputed AVL extrema.


Authored Swimlane edge-label theme controls verify complete font queries,
glyph text/size/fill/reference, explicit style precedence and every background
rectangle field/instruction position. The entire remaining scene equals the
same authored font/color baseline after removing only proven rectangles and
their exact metadata entry. Default textColor metadata is preserved in the
integer two-support complete-scene work controls. General arbitrary-support
and floating work remains open; final-source Miri is DEFERRED/PENDING.


## Bounded spacing and transport reconciliation

Authored spacing controls preserve complete ordinary/zero/default layout and PaintScene equality, precedence/refusal IR, finite supported boundaries and exact excessive-value refusals. Caller-owned invalid spacing and NaN-derived geometry have distinct typed refusals; the original infinite-extent and self-loop failures remain preserved. Existing work, style, glyph, classifier and scene controls remain required.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.


## Global curves and checked ranks

Authored global curve controls cover thirteen families, all seven aliases and case folding across LR/RL/TB/BT, retaining exact per-edge override, complete IR/layout/PaintScene and every remaining metadata field. Exact checked-rank controls preserve the original sixteen panic cases as immutable red evidence.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.


## Native validation checkpoint

Source baa06e72 has 192 complete node/edge theme IR/layout/scene controls, including exact directed marker fields, authored overrides, thick-edge defaults, zero/finite width behavior and invalid-width defaults. Existing cluster controls cover 128 cases. The original varied-support work receipt failed all32 cases (496/2016 predicate calls for N32/64); that failed receipt is preserved. The common-origin repair at e32c4728 passes those unchanged controls with one predicate call per case and counted index work3445..8242. All16 outside-right adversarial scene/work controls pass as well. These measured bounded families do not establish a universal output-sensitive or arbitrary-floating resource guarantee.

The current14-package cone at e32c4728 passed843 tests across32 targets with no
exclusions. Its strict run failed on a new test-only tuple annotation; a named
oracle group corrects that annotation. Independent review also found imported
padding missing from checked layout admission: the new direct red control
fails20/20 invalid cases, including NaN panics in all four directions. The
repair adds padding to the existing finite inclusive spacing guard. Final
affected suites and exact-source security acceptance are recorded separately;
whole/downstream/hosted/merge acceptance remains withheld. Final-source Miri
is **DEFERRED/PENDING**.

Subgraph title-margin controls compare complete emitted glyph runs and bounds
across36 direction/padding/margin cells, preserving the complete input layout.
They complement16128 checked-layout enclosure/default cells; metadata presence
alone is insufficient. Final3d affected14-package validation passes855/0/0 over
32targets and strict alltargets EXIT0. Miri DEFERRED/PENDING, never passed.

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
