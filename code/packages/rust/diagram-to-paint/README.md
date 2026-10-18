# diagram-to-paint

TimelineConfig now carries font_size with a16.0 default. Only direct fontSize
in the first closed init root, with absent-member fallback to config.fontSize
YAML, supplies the value. Existing decoding/unit scales and ownership remain;
invalid source values fall back to16.0. Checked Timeline layout rejects invalid
typed sizes after the established canvas/padding/margin priorities, using the
new appended TimelineFontSize error. Every positive finite size, including
subnormal/MAX, is transported without changing the existing layout formulas.
This is ingress/admission only. Caller paint sizes, glyphs, metadata, font roles,
queries and current family transport remain exact; size-driven typography and
wrapping still require their own bounded contract and represented-operation tests.
At `a3172c6569863bad3c02075f3090443c94f6c8cb`, 1181 native tests passed across59 unfiltered harnesses
(0 ignored); 2 doctests passed/0 ignored across14
libraries; all14 strict all-target checks passed. New complete controls cover
132source IR cells,84positive/empty typed layouts,12invalid inputs,8priority
cases and compatibility panic, retaining all old complete Scene/query controls.
Public API migration:56 mandatory literal fields+2field/default declarations
and1appended error variant, with five true struct updates unchanged. The retained
custom root24 source changes only12 expected semantic/layout config values.
PR17288 remains unmerged with current-main/conflict/review/hosted/actual-merge
gates outstanding. Miri DEFERRED/PENDING, never passed; full Closure parity,
transformation lineage and output-byte/source mapping remain OPEN.


Timeline configuration now carries a validated font family from the owned
themeVariables boundary into every Timeline font query and effective metadata.
The default is the opaque family string `"trebuchet ms", verdana, arial, sans-serif`;
it is one query member, without CSS fallback-list parsing. Leading/trailing
whitespace is trimmed; disallowed interior characters fall back to that default.
Title/label roles, sizes, weights, italic/stretch, geometry and source ownership
remain unchanged. None-config non-Timeline callers retain their supplied families.
At `0b7687390d942b02853dfb04a778cc73b8d3d302`, all 1174 native tests passed across 57 harnesses
(0 ignored), with 2 doctests passed/0 ignored across
14 libraries and all 14 strict all-target checks passing. New complete controls
cover 84 source IR cases, 48 source products, nine direct typed-family cases and
two None-config title/label cases, alongside all retained numeric/palette controls.
This is bounded implementation progress; PR17288 remains unmerged with current
conflict, hosted checks, review and actual-merge reachability gates outstanding.
Font size/layout expansion and native font rendering acceptance remain open.
Miri DEFERRED/PENDING, never passed; full Closure parity and lineage remain OPEN.


Bare `timeline` and `TimelineDirection::default()` now select left-to-right;
explicit TD and LR keep their existing behavior. Configuration ownership,
preprocessing, checked geometry and existing sources/assertions are retained.
At `74d9f01c`, all 1168 native tests passed across 55 harnesses
(0 existing ignored), with 2 doctests passed/0 ignored
across 14 targets and all 14 strict checks passing. Complete new controls cover
37 source/empty/retained IR cells and three portable IR/layout/Scene/query cells.
This is bounded progress, unmerged pending current review/conflict/hosted gates.
Apple rendering remains unexecuted here. Miri DEFERRED/PENDING, never passed;
full Closure parity and transformation lineage remain OPEN.


Checked separate timeline event cards now retain authored event order and
per-period palette ordinals. Actual Event text frames and the unchanged painter's
rounded arrow triangles must remain finite and nondegenerate; invalid geometry
returns TimelineGeometry before a layout is published. Chronology spines retain
their single-point exception. Legacy embedded-event painter payloads remain
supported. At dc842d0c, all258 affected tests passed, including32 complete authored
source/IR/layout/Scene/query cells and four complete legacy Scenes; both strict
checks passed. Apple rendering remains separately unexecuted here.
Miri DEFERRED/PENDING; full Closure parity and transformation lineage remain OPEN.


## Normal Venn text at shared grid centers

Attached Venn text uses row-major cell centers from one shared normal/debug
grid. Exact ordered structural memberships retain authored group/text order,
including repeated/reversed/NUL members and the existing unknown-member policy.
First-radius/last-center lookup and nonempty region-label OR remain distinct.
Checked numeric admission, complete debug geometry, styles and fonts stay intact;
the renderer, dependencies and production count limits do not change.

At source `6c4b9ec8`, all261 tests across17 harnesses pass without failures,
ignores or filters; two empty doctests and both all-target strict checks pass.
This includes32 whole source/IR/layout/Scene/query cases and32 direct controls;
the old136 source/96 direct complete Scene cases retain every assertion with
only independently declared normal coordinates revised. Nine example targets
were checked on Windows; no Apple rendering execution is claimed.

See [the contract and execution checkpoint](../../../specs/CV04b-current-main-venn-normal-cell-centers.md).
Current-main/hosted/merge gates and full Closure parity/lineage remain OPEN.
Miri **DEFERRED/PENDING**, never passed; the frozen run and its inputs remain
untouched pending genuine final-source validation after implementation delivery.



At the pre-integration timeline palette checkpoint41fad2ab, all169 package tests
and strict lint pass. New48 complete typed Scene/query cells pin independently
sized0/1/2/6/8 palettes, mismatched families, MAX ordinals, role-specific empty
fallbacks, every metadata entry and glyph colors. This retains public embedded-
event period rendering; separate event-card/current-main/source-to-Scene delivery
validation remains pending. Local Miri DEFERRED/PENDING.

## Checked Ishikawa collision and hatch resources

`try_layout_ishikawa(&diagram, canvas_width)` returns an owned layout or
`IshikawaLayoutError`; `layout_ishikawa` keeps its explicit compatibility panic
for unsupported geometry. Initial finite represented geometry precedes a shared
8,388,608 pair-work budget, deterministic collision placement and certification
of both centered and actual painted rectangle extents. Failed precision checks
return `DerivedGeometry`; authored supported fonts and depths remain intact.

Final seeded outlines are built once after placement. HandDrawn hatching admits
all numeric/grid spans before plan storage: generated signed offsets stay within
2^50, with shared limits262,144 logical lines,8,388,608 two-pass edge visits and
262,144 actual emitted segments. A bounded counting pass precedes exact output
reservation and emission. Errors publish no partial layout. Classic keeps its
hatch-free path and does not acquire hatch grid limits. Existing input/layout
allocations are not a universal fallible-allocation guarantee.

Complete declared-source fixtures retain every Classic assertion and add24
HandDrawn IR/layout/scene/font-query cells, including final nested collision
positions, seeds0/17/-7, both configuration forms and two font families. A bounded
legacy one-pass oracle is checked against analytic rectangle intersections;
whole scenes retain2.5px fill hatches, transparent outlines and ordered glyphs.
At exactc27554a9, geometric81 and painter167 tests pass with zero failures,
ignores or filters; both all-target strict lint commands pass. This is scoped
package validation, not current-main/hosted/full Closure acceptance. Miri remains
DEFERRED/PENDING for genuine final-source validation after delivery.

Wardley evolution statements accept uniform stages or an explicit finite,
normalized, nondecreasing end for every stage. Empty and single-stage typed
vectors, zero/equal bands and a final end below1 are supported. Closed quoted
labels and secondary aliases preserve Unicode and interior syntax; ASCII space
and TAB dispatch keep exact physical diagnostics. Mixed boundary association is
rejected before compression. Repeated statements replace labels and ends
together. StageBoundaries follows all prior typed errors and precedes geometry;
private derived admission checks intervals and finite nonnegative band geometry.
Default labels intentionally move to centered equal-width, no-wrap bands;
custom separators use each band start and5/5 dashes. The minimum stage-font
clamp, pipeline ordering/visibility guards and literal-white force arrows stay.
This adds no general containment, font-fit or forged-Paint-payload safety claim.

At stage source `d7a4317d`, strict all-target checks for14 affected packages
and the complete unfiltered1,032-test/62-target suite passed with zero failures
or ignores. Ten test functions were added, none removed. Source38 whole-IR
cells/32 exact errors,9 legacy refusals/8 complete layouts/9 named refusals/
18 priority cells/9 derived refusals and5 accepted companions pass. Whole
source/typed IR, checked layout and82 complete Scenes (old64 intentionally
revised plus18 new) bind every glyph, all21 metadata fields and ordered cached
Georgia/CourierNew queries. Required ABI migration27files/37fields and the
intentional old64 stage oracle revision have exact whole-file inverse proofs.

New18 Scene tables preceded first rendering. First render actually failed101:
two independently declared glyph literals regrouped f64 centering. A committed
spec clarification preceded a reversible two-literal correction derived from
start_x+(width-measured_advance)/2 for all nine profiles. Second serial render
actually failed101: empty cap fixture inherited hardcoded forceCount6. Its
expected metadata now uses the independently declared force-vector cardinality.
Both failure receipts/logs remain unchanged. Third serial render passed every
18 distinct case/family pair; exact Scene equality and production arithmetic
were preserved. Independent exact-source/focused and terminal security passed.

Raw Rust/TOML1,493 and the complete14/50 style cone were frozen. Native166/
2861 objects, SwiftUI191, Closure36/3705+3 and external resources21 remain exact,
with their prior execution attribution and no new native/Closure/Apple/Swift/
C# execution. Registry376 is unchanged; only four local style versions advance.
Venn232/Ishikawa37/prior67 IR controls remain. These are source-specific local
results. Sole PR17288 remains unmerged; hosted checks and actual merge are
separate gates. Stage69adc is integrated with repairs; native17553/native186001 and
label54f66 remain separately unaccepted. Full Closure parity and CV04b actual
ingestion IDs/output-byte/source/full transformation lineage/stable IDs/spans/
pass-sweep remain OPEN. Base ESTree canonical boundary, own types, no new deps.
Miri **DEFERRED/PENDING**, NEVER passed: frozen checkout/run/inputs/receipts/
target untouched and unpolled; no duplicate or termination. Implementation
delivery first, genuine final-source Miri afterward. Heartbeat stays ACTIVE.

The following pipeline checkpoint is historical; the stage acceptance above
supersedes its then-pending stage status.

Wardley pipelines support flat, nonempty blocks with known first-authored
parents. Required parent flags and ordered pipeline vectors cross the owned IR
boundary. Closed active-quote readers preserve Unicode, percent/bracket/brace
data, declared ASCII space/TAB separators and complete physical diagnostics.
An ordered first-label index and one cached visibility per header replace
repeated prefix searches. Typed admission preserves all older error priorities,
then checks PipelineReference and PipelineMembership before geometry arithmetic;
repeated/nested/cross-block membership is explicitly unsupported. Same-block
duplicate occurrences and stable equal-x order remain. Derived checks cover
pipeline rectangles/member points and parent squares. Negative inset positions
can be valid; no general containment or font-fit guarantee is added. Paint keeps
authored block/member ordering, dashed connectors, rectangles, parent squares,
final-parent link/evolution points and the existing literal-white force arrows.

At pipeline source `80e8f09d`, strict checks for all14 affected packages and the
complete unfiltered 1,022-test/59-target suite passed with zero failures/ignores.
Thirteen test functions were added, none removed. Whole source IR, typed IR,
checked/compatibility layout and16 new complete Scenes bind every glyph, all20
metadata fields and ordered cached Georgia/CourierNew queries. New literal
oracle tables preceded the first rendering and passed unchanged; old48 complete
Scenes retain every assertion with only required ABI fields/zero pipelineCount.
Source57 whole-IR cells/44 exact physical errors,12 legacy/12 named refusals,
11 priority cells,20 derived refusals plus parent-square overflow and increasing
8/32/128 actual-key comparison profiles pass. The first strict101 test-helper
let-and-return failure and the ABI helper census preflight failure are retained;
the direct-return repair changes no production or Scene oracle. Independent
exact-source/focused and terminal security reviews passed.

These results are source-specific local acceptance. Native16 c82's1848/78/996
and SwiftUI263/6 retain their original execution attribution; current native166/
2861 carrier objects and SwiftUI191 stay exact. Closure36/3705+3 and resources21
retain their earlier attribution without new execution. Registry376 is unchanged;
only four incoming local style versions advance. All Venn232/Ishikawa37/prior67
IR controls remain. Raw Rust/TOML1488 and the complete14/50 style cone are frozen.

Sole PR17288 is still unmerged; current-main reconciliation, required hosted
checks and merge are separate gates. The bounded pipeline slice integrates c138
while later stage69adc/native17553 remain independently audited, WITHHELD and
unexecuted here. Full Closure parity/output-byte/source/full transformation
lineage/CV04b remain OPEN. Base ESTree canonical boundary, own types and no new
crates.io dependencies remain decisions. Miri **DEFERRED/PENDING**, never passed;
frozen run/checkout/inputs/target/receipts untouched and unpolled. Implementation
delivery comes first, then genuine final-source Miri. Heartbeat remains ACTIVE.

Wardley forces retain authored order and independent labels for accelerator and
deaccelerator glyphs. Quoted labels own active escapes, percent pairs and brackets;
outside comments and ASCII space/TAB separators have exact source diagnostics.
Typed layout admits finite normalized axes after annotation coordinates and
before references, and checks every derived force point. Paint retains literal
white seven-vertex closed arrows, component stroke, and normal wrapping in the
60x20 label frame at weight700/size10. Caller font family/style/stretch survive.

At native source `c82b1d18`, strict checks for six affected packages and the full
unfiltered 1,848-test/78-target suite passed, with one unchanged ignore. Compiler
903 and LANG510 passed; 166 saved sources produced 996 actual normal-exit cells
on six available backends, retaining all old156/936. CLR was unavailable and
uncounted. SwiftUI overlap separately passed strict and263 Rust tests/6targets;
no Swift/Apple/C#/XAML execution follows. Independent exact-source and terminal
security reviews passed.

At force source `85d6df6e`, strict checks for14 affected packages and the full
unfiltered 1,009-test/56-target suite passed with zero failures/ignores. Nine
test functions were added, none removed. Old32 and new16 complete Wardley cells
bind whole IR/layout/scene, every glyph, all19 metadata fields and ordered cached
font queries. New oracle tables preceded the first force rendering and passed
unchanged, including two wrapped labels per cell with Georgia/CourierNew. Source
48IR/64errors,48legacy/48named/9priority/24derived force controls passed. All Venn,
Ishikawa and previous parser/resource controls remain. Independent exact-source/
focused and terminal security reviews passed. Two earlier test-only strict101
frames are retained; Copy/shorthand repairs changed no production or scene oracle.

This historical force checkpoint predates the pipeline acceptance above.
Its exact-source execution attribution remains unchanged.
Full Closure parity/output-byte/source/full transformation lineage/CV04b remain
open. Base ESTree canonical boundary, own types/no crates.io dependencies remain
decisions. Miri **DEFERRED/PENDING**, never passed; frozen run/checkout/inputs/
target/receipts untouched and unpolled. Deliver first, genuine final-source Miri
afterward. Heartbeat remains active until the full completion contract.

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

## Reviewed Venn debug geometry and set defaults

Debug layout groups exact ordered set vectors, including embedded NULs, in first
authored order. Preparation indexes preserve last-ID centers, first-ID radii and
any nonempty matching region label. Checked layout validates finite debug
positions and finite nonnegative radii/cell dimensions alongside every previous
numeric guard. Actual scan-visit tests cover 8/32/128 groups without a new cap.

Unstyled sets use opacity 0.1, resolved raw fill as stroke, and stroke width
`5 * width / 1600`. Authored opacity, stroke and width override independently,
including zero; an empty typed palette retains the blue fallback. Paint consumes
explicit debug vectors even when the typed debug flag is false.

Tested source `58a7862f`: 943 tests passed, zero failures/ignores, 41 targets;
strict all-target Clippy passed for all 14 affected packages. The 176 complete
scene/query cases retain 104 earlier oracles through a checked reversible
default adapter and add 56 typed plus 16 parsed-source cases. This establishes
the bounded contracts, not full Mermaid rendering or Closure parity. Required
hosted acceptance remains pending; final-source Miri **DEFERRED/PENDING**.

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

## Width-relative Venn typography

`LayoutedVennLabel.font_size` is a required resolved layout field. Generated set
and intersection labels use `48 * width / 1600`, attached labels use
`40 * width / 1600`, and paint titles use `32 * width / 1600`. Paint retains caller
weight, italic and stretch and resolves the source family/colors as before.
Checked layout also refuses nonfinite or negative label font sizes alongside
the existing coordinate checks. Width admission remains finite(0,65536]; zero
font size caused by tiny-width underflow remains supported. No source font
option, new cap, clamp, fallback or crate dependency was added.

At actual source `04712174`, the complete14-package reverse cone passed
960 tests across43 targets with zero failures/ignores, plus strict all-target
warnings/unknown-lints denial. All208 previous complete scene/query cases retain
their non-typography expectations; a narrow one-glyph adapter checks the new
font sizes and full glyph positions before reversing only the old typography.
Twenty-four additional complete scene/query cases cover three widths, two
families, titles and authored colors. Thirty-one required constructor fields
are inventoried; invalid derived fonts and width65537 still refuse.
Independent source and terminal security reviews passed.

Fixed text boxes, long/multiline fitting, real-font rendering, arbitrary forged
layout/fonts/styles and general renderer resource/visual parity remain open.
Native validation is separately attributed to `0cc8fef6`:1,824 passed/zero failed/
one unchanged archive ignore across78 targets, compiler887/fullLANG502 and
strict six-package lint. The126 focused sources produced756 actual backend
cells (696 prior exact+60new), on six available backends; absent CLR is uncounted.
Transfer uses exact166-package/2847-object inputs and21 mapped external static
resources before this documentation carrier. Named Markdown exceptions are
inventoried afterward; no dynamic/toolchain/OS closure or new native execution
is inferred. Historical Closure36/3705 plus three static inputs retains316e
attribution. Registry376 identities/checksums remain exact.

The actual hosted macOS compiler-phase proof is still OPEN. The separate
diagnostic change4ab4 preserves a bounded JSON-quoted child-output tail on
failure;13 portable tests and independent review passed. That is visibility,
not evidence of a successful Apple compiler/Cabal route. Required current-head
CI/review/conflict checks and actual merge remain delivery gates. Final-source
Miri **DEFERRED/PENDING**, never passed; frozen run/source/inputs/target/receipts
remain untouched/unpolled. Full Closure parity and output-source/full lineage
remain OPEN.

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

## Stable Wardley annotations and total box placement

Annotation groups retain first authored number order, every repeated coordinate,
and the first nonempty decoded text. An ordered index replaces repeated vector
scans. Active quotes preserve interior spaces, Unicode and percent-percent;
only the active delimiter and backslash may be escaped. Malformed tokens retain
exact physical source errors. Typed box and marker coordinates are finite in
[0,1], with named refusals after note coordinates and before reference checks;
every derived box and marker point is finite.

Boxes use the existing effective plot edges. Width is the smaller of desired
width and max(plot width,40); height retains all nonempty entries. Clamp upper
bounds are never below their lower bounds. Insufficient space pins the origin
to the inner left/top and permits overflow. No entry is dropped or font reduced.
This defines placement, without a general fontfit or containment guarantee.
Marker700/10, entry caller-weight/11 and note700/11 retain caller family, italic
and stretch. The checked minimum420x300 map and stage expression are unchanged.

Actual exact source1e13fc69 passes parser10, work3, geometry45 and Paint4 focused
tests:20 new complete IR/34 exact errors;48legacy+48named/6priority/24derived
annotation refusals;32 whole source/IR/layout/scene/glyph/all18metadata/ordered
cached-query cells, comprising16 old and16 new. New literal scene oracles were
committed before rendering. The real default30-entry clamp48..44 panic was
reproduced; its failed wrapper word check remains separate from actual Cargo101.
All new cells passed on first repaired render without oracle corrections. Old
scene migrations add only required empty annotation ABI and zero count metadata;
all numeric/font/query/color/stroke assertions reverse byte-exactly.

Full affected14 packages pass1000P0F0I across53 target identities; strict all-
target warnings/unknown-lints denial exits0. Tests4b007728/strict86be17a7 bind
style14reverse/50forward/844objects, raw1476 Rust/TOML, native2856 exact93bfe3d2,
registry376 and21+3 static resources. Independent source and terminal reviews
PASS. Retained13 Venn fixture files contain232 complete cells;37 Ishikawa cells
and67 prior parser IR controls remain exact. Indexed comparison bounds are
cfg(test) observers, without a production work counter/cap.

This is unmerged progress in solePR17288. The composed source integrates main
5b42502d; later native16 main459274e4 is pending bounded reconciliation, without
current-main validation acceptance. Required exact-head hostedCI, conflicts,
reviews, actual merge reachability and macOS phase-refusal rootcause remain OPEN.
Historical Closure36/3705+3 retains source316e attribution, without fresh execution.
Base ESTree canonical serialized boundary/own types/no new crates.io dependencies
remain owner decisions. Full Closure parity/output-byte-source/full folding,
renaming,inlining,motion,merging,deletion lineage/stableIDs/spans/pass-sweep/CV04b
remain OPEN. Miri **DEFERRED/PENDING**, never passed; frozen checkout/run/inputs/
target/receipts untouched and unpolled, no duplicate or termination. Delivery
first, genuine final-source Miri afterward; heartbeat remains ACTIVE.
