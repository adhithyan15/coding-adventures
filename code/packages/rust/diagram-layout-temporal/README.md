# diagram-layout-temporal

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
