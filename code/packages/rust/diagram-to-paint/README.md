# diagram-to-paint

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
