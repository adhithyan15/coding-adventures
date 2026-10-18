# diagram-layout-temporal

## Checked Timeline period-card estimates

Period cards now estimate vertical space from Unicode scalar counts, owned
width180/190 and positive configured size s. Capacity is floor((width-20)/
(s*0.44)), at least1, with defined saturating usize conversion for positive
quotient overflow/subnormal denominator underflow. Rust hard-line semantics,
integer div_ceil and checked accumulation precede ordered finite checks of
(lines*s)*1.2+12, then max with the existing max(3s,48) card minimum. Overflow
returns TimelineGeometry; no arbitrary cap or wrapped-string allocation.

Title/section/event minima, raw error order, eager empty admission, event
heights/pitch/indexing, grouped LR origin, widths, palette ordinals, authored
strings/IR, metadata/fonts and represented frame/arrow guards remain. Existing
short-label products stay exact. Taller cards update current midpoint and row
dependencies. The painter still uses fixed period text height22: complete
FakeShaper controls retain an oversized unbroken word and three hard-line
glyph runs; neither shaping fit nor containment/raster output is certified.
Event-height estimates and cumulative event stacks remain a separate slice.

Executed source `a0a5681f4295bd8083403f2cb412b9234f95e4bf`: 289 native passes across3 unfiltered harnesses
(Windows e2e0), no failures/ignores/filters. Two doctest invocations contain
0 tests; two strict all-target checks and metadata passed:8 actual commands0.
Fresh1293manifest/1292node/436registry graph gives affected2/forward50.
All407 previous completion identities plus53 new typed and6 complete
source/IR/layout/Scene/query products pass. Compiled semantic red at
`144b1df697f782aef3c7ccec7d82b03a15c973ea` was283P6F before production; both executables and raw streams
are immutable. Exact-source and actual-terminal security reviews PASS/RELEASED.
The EOF whitespace failure (shell1/nativegit2), separate diagnostic assertion
failure and one-LF repair/nativegit0 remain documented in filled lesson shards;
none is relabeled a compiler failure, semantic red or green acceptance.

PR17288 remains unmerged; whole-main/conflicts/current required checks/review
and actual merge remain OPEN. Miri DEFERRED/PENDING, never passed; frozen
run/checkout/inputs/receipts/target untouched and unpolled. Full Closure parity,
stable identities/spans, output-byte/source joins and fold/rename/inline/motion/
merge/delete/pass-sweep lineage remain OPEN. Genuine final-source Miri follows
implementation delivery. No new compiler-wide or Apple rendering run is claimed.
Green receipt `d559f2382dbb398795cc9bb006fe199f45ce9cc4359741cda47093fb36694afa`; immutable manifest `79cb663f73faae8c2e07c12a6ef42bf53aebfa4ad465a9310c93315e08311b5f`.

## Historical checked Timeline size-driven minima

Timeline title/section/period/event heights now use positive finite size s
with minima max(2.5s,40), max(1.875s,30), max(3s,48), max(1.875s,30).
Every planned product is checked before max and section work, including empty
or untitled input; overflow returns existing TimelineGeometry. Raw Canvas,
Padding, LeftMargin and FontSize priorities and full represented-frame/arrow
certification remain. Ingress/IR still transports every positive finite size.
Indexed event pitch and dependent vertical advances retain the specified f64
association, with an accepted bit-exact default-title regression witness.
Widths, grouping, palette ordinals, authored strings/IR, font roles, metadata,
ordered font identities and dependencies retain their existing contracts.
Wrapping, containment/nonoverlap and arbitrary backend/raster guarantees are
separate work; fixed period text height22 and LR row anchors remain.

Executed source `ed6526c30e3c1f35b79dbae244a069eea35c31b6`: 283 native tests passed, zero failures,
ignores or filters, across three unfiltered harness invocations (Windows e2e
contains zero tests). Two doctest invocations contain 0 tests; two strict
all-target lint checks and metadata passed: eight actual commands0. Fresh
1293manifest/1292node/436registry graph gives reverse2/forward50. No new
compiler-wide or Apple rendering execution is claimed. All177 old temporal
cells (including104 typed size cells) and210 old painter cells are retained;
17 new complete typed cells plus3 whole source/IR/layout/Scene/query products
pass. Original literal bodies are retained; old MAX Results and12 custom
root24 products have explicit closed expected migrations. Actual compiled
semantic red at `e4d9e38d3ec6741b6de7017cfd9fe44de8c4c14c` was275P8F, before production.
Both red executables and all raw streams are retained separately. Five filled
lessons record preparatory static risks/refusals, positive-verdict guard repairs
and a consumed mutation-session ordering mistake; none is mislabeled a compiled semantic red or passed test.
Exact source and actual-terminal independent security reviews PASS/RELEASED.

PR17288 remains unmerged. Whole-main/conflicts, current review-clean exact-head
terminal acceptable required CI and actual merge reachability remain open.
Miri DEFERRED/PENDING, never passed; frozen run/checkout/inputs/receipts/target
are untouched and unpolled. Full Closure parity, stable identities/source spans,
output-byte/source joins and fold/rename/inline/motion/merge/delete/pass-sweep
lineage remain OPEN. Genuine final-source Miri follows implementation delivery.
Green receipt SHA256 `8a1ae70875d93cb07b0a8c5117a90809b110f9e4d1fbda29e1a03f2e092a0ecf`; immutable manifest `1ef0fb729dee9fb76a592540afabf534eab17bdfac70dbcbb12da256272173cb`.

## Historical Timeline size ingress a3172c65


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


Checked separate timeline event cards now retain authored event order and
per-period palette ordinals. Actual Event text frames and the unchanged painter's
rounded arrow triangles must remain finite and nondegenerate; invalid geometry
returns TimelineGeometry before a layout is published. Chronology spines retain
their single-point exception. Legacy embedded-event painter payloads remain
supported. At dc842d0c, all258 affected tests passed, including32 complete authored
source/IR/layout/Scene/query cells and four complete legacy Scenes; both strict
checks passed. Apple rendering remains separately unexecuted here.
Miri DEFERRED/PENDING; full Closure parity and transformation lineage remain OPEN.


Use `try_layout_temporal_diagram` to handle unsupported timeline geometry.
Its own `TemporalLayoutError` reports raw canvas, padding, left margin and
derived geometry in that priority. It validates actual outer and painter text
frames, including represented endpoints, without imposing a blanket finite
canvas/margin limit. The compatibility function explicitly panics on refusal.
Single-color unnamed timelines retain their chronology spine. Section detection
runs once per layout. Gantt/Git/Journey/event-model algorithms retain their
previous behavior. At22638c8b all76 package tests and strict lint passed; full
source/Scene/palette transport remains pending. Miri DEFERRED/PENDING.


Layout engine for temporal diagrams (DG04): Gantt charts and git-graphs.

## Position in pipeline

```
TemporalDiagram (diagram-ir)
  → diagram-layout-temporal
      → LayoutedTemporalDiagram (diagram-ir)
      → diagram-to-paint
      → PaintScene
```

## Usage

```rust
use diagram_layout_temporal::layout_temporal_diagram;
use mermaid_parser::parse_gantt;
use diagram_ir::{TemporalDiagram, TemporalKind, TemporalBody};

let gantt    = parse_gantt("gantt\n  dateFormat YYYY-MM-DD\n  section A\n    Task :done, t1, 2026-01-01, 5d").unwrap();
let temporal = TemporalDiagram { kind: TemporalKind::Gantt, title: None, body: TemporalBody::Gantt(gantt) };
let layout   = layout_temporal_diagram(&temporal, 800.0);
```

Timeline diagrams use dedicated section, chronology-spine, and period-card
layout items. Both top-down and left-right source directions are deterministic,
and ordered period events remain semantic text through Paint lowering.

## Algorithms

**Gantt**: Fixed-point date resolution first records typed absolute starts, then resolves
forward and cross-section `after <id...>` / `until <id...>` dependency chains.
Multi-source `after` selects the latest dependency end while multi-source `until`
selects the earliest dependency start. Time axis scaled to canvas width.
Weekend, weekday, and explicit-date exclusions extend duration geometry, while
explicit includes override exclusions. Calendar dates are interpreted with the
diagram's authored `dateFormat` before backend-neutral layout.
The parser rejects duration schedules with no recurring valid weekday before
they can enter the otherwise infallible temporal layout API.
Civil-day arithmetic remains backend-neutral across month and daylight-saving
boundaries; source section/task order is retained during row layout.
Axis ticks preserve millisecond, second, minute, hour, day, week, month, and year
interval units, with time fields formatted before Paint lowering.
Weekly ticks align to the configured weekday, while month and year ticks use
civil calendar boundaries instead of fixed-duration approximations.
Styled today markers preserve stroke opacity as backend-neutral paint semantics.
Repeated vertical-marker task IDs lower once without consuming task rows.
Date-only calendar exclusions remain valid when task dates include clock fields.
Compact display mode packs non-overlapping tasks into shared section lanes.

**Git**: Branch lanes (horizontal rows), commits as circles on lanes, merges as bezier arcs.
