# DG00 — Diagram IR and Layout Pipeline

## Overview

This spec defines the repo's general architecture for **text diagrams that
render through `PaintScene` / `PaintInstruction`**.

The short answer to the design question is:

- **Yes**, we should have source-specific diagram packages.
- **No**, we should not carry source-specific layout packages all the way down.
- **Yes**, the final compiled target should stay `PaintScene` / `PaintInstruction`.
- **Yes**, we should extend paint instructions and native backends where the
  current primitive set is too weak.

The key separation is:

```text
diagram source syntax
  -> source AST
  -> shared semantic Diagram IR
  -> family-specific layout
  -> layouted diagram IR
  -> diagram-to-paint
  -> PaintScene / PaintInstruction
  -> Canvas / SVG / Metal / Direct2D / ...
```

That lets Mermaid, DOT, PlantUML subsets, WaveDrom, and future text-diagram
formats share the expensive parts:

- routing
- node sizing
- lane/timeline placement
- label placement
- paint lowering
- backend work

instead of re-implementing them per syntax.

---

## Design Rule

Organize the stack by **source language at the parse layer** and by **diagram
family at the layout layer**.

Good split:

- `mermaid-parser`
- `dot-parser`
- `plantuml-sequence-parser`
- `wavedrom-parser`
- `diagram-ir`
- `diagram-layout-graph`
- `diagram-layout-sequence`
- `diagram-layout-waveform`
- `diagram-to-paint`

Bad split:

- `mermaid-layout`
- `dot-layout`
- `plantuml-layout`
- `wavedrom-layout`

all doing their own geometry, routing, markers, labels, and paint lowering.

Mermaid flowcharts and DOT graphs are different syntaxes for mostly the same
graph-layout problem. Mermaid sequence diagrams and PlantUML sequence diagrams
are different syntaxes for mostly the same timeline-layout problem. We should
share that middle layer on purpose.

---

## Layer Position

### Standalone diagram rendering

```text
Mermaid / DOT / PlantUML / WaveDrom source
  -> source-specific parser
  -> source-specific AST
  -> diagram frontend lowerer
  -> DiagramDocument (DG00)
  -> family layout package
  -> LayoutedDiagram (DG00)
  -> diagram-to-paint
  -> PaintScene (P2D00)
  -> paint-vm backend
```

### Embedded in GFM / documents

```text
GFM source
  -> gfm-parser
  -> TE04 FencedBlockNode
  -> diagram block transform
  -> DiagramDocument (DG00)
  -> family layout package
  -> LayoutedDiagram
  -> document/layout integration
  -> PaintScene
```

TE04 is the parser seam that preserves fenced blocks.
DG00 is the rendering seam that turns claimed diagram blocks into native paint.

---

## Package Plan

### 1. Source packages

Each source language gets its own parser package and AST.

Examples:

- `mermaid-parser`
- `dot-parser`
- `plantuml-sequence-parser`
- `wavedrom-parser`

Responsibilities:

- lexical and syntax parsing
- source-level validation
- preserving dialect-specific constructs
- exposing source spans / diagnostics

Non-responsibilities:

- native rendering
- graph routing
- backend-specific paint logic

### 2. `diagram-ir`

This package defines the shared semantic diagram model.

Responsibilities:

- family-neutral document envelope
- family-specific semantic node unions
- shared style and metadata types
- no absolute geometry

### 3. Family layout packages

These are the real workhorses.

- `diagram-layout-graph`
- `diagram-layout-sequence`
- `diagram-layout-waveform`

Responsibilities:

- measure labels
- choose node sizes
- route edges/messages
- place clusters, lanes, tracks, notes, and annotations
- produce absolute diagram geometry

### 4. `diagram-to-paint`

This package lowers a layouted diagram into `PaintInstruction`.

Responsibilities:

- expand markers to paintable geometry
- expand text blocks into glyph runs or backend-compatible text instructions
- choose paint z-order
- produce one `PaintScene` or nested scene fragment

### 5. Document integration package

We likely also need a thin integration package, for example:

- `document-diagram-blocks`
- or `document-ast-diagram-transform`

Responsibilities:

- claim TE04 `FencedBlockNode`s by `name`
- dispatch to the right source parser
- produce an embedded diagram block representation
- preserve code-block fallback for unknown fences

This package is an adapter. It is not the owner of diagram semantics.

---

## Semantic Diagram IR

`diagram-ir` should define a common envelope plus family-specific payloads.

### Common envelope

```text
DiagramDocument
  id: string?
  family: "graph" | "sequence" | "waveform"
  title: string?
  metadata: map<string, scalar>
  theme: DiagramTheme?
  body: GraphDiagram | SequenceDiagram | WaveformDiagram
```

### Shared helper concepts

```text
DiagramStyleRef
  id: string?
  classes: list<string>
  inline: map<string, scalar>

DiagramLabel
  text: string
  style: DiagramStyleRef?

DiagramNote
  id: string?
  label: DiagramLabel
  attachment: attachment target?
  style: DiagramStyleRef?

DiagramConstraint
  kind: family-specific symbolic constraint
```

### Graph family

```text
GraphDiagram
  direction: "lr" | "rl" | "tb" | "bt"?
  nodes: list<GraphNode>
  edges: list<GraphEdge>
  clusters: list<GraphCluster>
  lanes: list<GraphLane>
  constraints: list<DiagramConstraint>

GraphNode
  id: string
  shape: "rect" | "rounded_rect" | "ellipse" | "diamond" | "pill" | "path"
  label: DiagramLabel?
  ports: list<GraphPort>
  style: DiagramStyleRef?
  parent: cluster_id?

GraphPort
  id: string
  side: "top" | "right" | "bottom" | "left" | "auto"
  label: DiagramLabel?

GraphEdge
  id: string?
  from: endpoint
  to: endpoint
  label: DiagramLabel?
  kind: "directed" | "undirected"
  style: DiagramStyleRef?

GraphCluster
  id: string
  label: DiagramLabel?
  children: list<node_or_cluster_id>
  style: DiagramStyleRef?

GraphLane
  id: string
  label: DiagramLabel?
  children: list<node_or_cluster_id>
  style: DiagramStyleRef?
```

### Sequence family

```text
SequenceDiagram
  participants: list<Participant>
  events: list<SequenceEvent>
  groups: list<SequenceGroup>

Participant
  id: string
  label: DiagramLabel
  style: DiagramStyleRef?

SequenceEvent
  kind: "message" | "reply" | "activation" | "deactivation" | "note" | "divider"
  ...
```

The exact event union can evolve, but the semantic IR must carry concepts such
as participants, lifelines, messages, activation spans, notes, and grouping.

### Waveform family

```text
WaveformDiagram
  tracks: list<WaveTrack>
  markers: list<WaveMarker>
  spans: list<WaveSpan>

WaveTrack
  id: string
  label: DiagramLabel
  samples: waveform-specific payload
  style: DiagramStyleRef?
```

The waveform IR should talk about tracks, logical transitions, cycles, spans,
and annotations, not SVG paths.

### What does NOT belong in semantic IR

Do not put these in `diagram-ir`:

- absolute `x` / `y`
- bezier control points
- paint colors already flattened to backend strings
- glyph IDs
- backend-specific line dash objects

The semantic IR should still describe the diagram, not how Direct2D or Metal
will rasterize it.

---

## Layouted Diagram IR

After family layout runs, we need a geometry-bearing IR that is still slightly
more semantic than raw `PaintInstruction`.

```text
LayoutedDiagram
  width: float
  height: float
  items: list<LayoutedDiagramItem>
```

Recommended item types:

```text
LayoutedDiagramItem =
  | NodeBox
  | EdgeRoute
  | LabelBox
  | LaneBand
  | NoteBox
  | Decoration
  | GroupBounds

NodeBox
  id: string
  shape: node shape
  x: float
  y: float
  width: float
  height: float
  style: resolved style

EdgeRoute
  id: string?
  points: list<Point>
  curve: optional bezier/arc detail
  start_marker: Marker?
  end_marker: Marker?
  label_box: Rect?
  style: resolved style

LabelBox
  text: string
  x: float
  y: float
  width: float
  height: float
  align: "start" | "center" | "end"
  style: resolved text style
```

This is the right layer for:

- routed polylines
- arrowhead attachment points
- note box geometry
- lane and cluster rectangles
- final z-order

This is **not** yet the right layer for:

- glyph IDs
- backend handles
- COM objects
- Metal buffers

### Why not lower straight to `PaintInstruction` from layout?

Because some layout results need one more normalization step:

- markers become paths
- dashed edges may expand to backend stroke state
- text boxes become one or more glyph runs
- graph edges and sequence arrows share paint lowering logic

That shared logic belongs in `diagram-to-paint`, not duplicated across every
layout package.

---

## `diagram-to-paint`

`diagram-to-paint` consumes `LayoutedDiagram` and emits `PaintScene`.

### Responsibilities

- convert node shapes to `rect`, `ellipse`, or `path`
- convert routed edges to `path`
- expand arrowheads and other markers into `path`
- convert label boxes to glyph runs or backend-compatible text
- emit `group`, `layer`, and `clip` where they help preserve structure

### Lowering guidelines

- Prefer `PaintPath` over inventing graph-specific paint instructions.
- Prefer expanding arrowheads into small `PaintPath` children instead of adding
  a dedicated `PaintArrowhead` instruction.
- Prefer path-based edges over `PaintLine` whenever joins, dashes, or markers
  matter.
- Keep ports, participants, tracks, and lanes in Diagram IR, not in Paint IR.

### Result

The paint layer stays generic:

```text
LayoutedDiagram
  -> diagram-to-paint
  -> PaintScene
```

not:

```text
LayoutedDiagram
  -> MermaidPaintInstruction | WaveDromPaintInstruction | ...
```

---

## What Needs To Change In PaintInstructions

Most diagram concepts do **not** require new paint instructions. They require a
better diagram middle layer.

The current paint IR is already good at:

- rectangles
- rounded rectangles
- ellipses
- arbitrary paths
- groups
- layers
- clips
- glyph runs
- images

### Must-have paint additions

The main missing primitive for serious diagram work is **dashed strokes**.

Recommended addition on strokable instructions:

```text
stroke_dash: list<float>?
stroke_dash_offset: float?
```

Applies to:

- `PaintPath`
- `PaintLine`
- `PaintRect`
- `PaintEllipse`

This covers:

- Mermaid dashed links
- UML dependency edges
- lane separators
- waveform guide lines

### Nice-to-have paint additions

These are helpful but not required for phase 1:

- `miter_limit` on stroked paths
- optional polyline convenience instruction that lowers to `PaintPath`
- path-based clip support for non-rectangular clipping

### What should NOT be added to PaintInstructions

Do not add paint instructions for:

- ports
- graph nodes
- graph edges
- participants
- lifelines
- wave tracks
- arrowheads as a first-class instruction

Those belong above paint.

---

## What Needs To Change In Layout IR

For standalone diagram rendering, DG00 can bypass document layout entirely:

```text
DiagramDocument -> family layout -> LayoutedDiagram -> PaintScene
```

For embedded document rendering, UI02 still needs a way to host a native paint
fragment in flow.

Recommended follow-up:

- add a `paint_fragment` or equivalent content kind to `layout-ir`
- or add a formally specified embedded scene-fragment contract in `ext`

That fragment needs at least:

- intrinsic width
- intrinsic height
- a paint callback or scene-fragment payload

Without that, diagrams embedded in markdown are forced to degrade into text or
images too early.

---

## Backend Work

Once the diagram pipeline targets `PaintInstruction`, backend work becomes much
more predictable.

### Direct2D

Best first native backend.

Needed for strong parity:

- finish dash-pattern support
- keep path and glyph-run support solid
- verify marker-heavy path scenes

### Metal

Biggest native gap for general diagrams.

Needed:

- robust `PaintPath` fill and stroke
- robust ellipse rendering
- dash-pattern support
- stable glyph-run rendering for labels

Without path support, diagram lowering stays artificially constrained.

### GDI

Treat as degraded fallback.

Acceptable v1 compromises:

- solid-line fallback when dash support is missing
- limited path fidelity
- simplified text handling

### Canvas / SVG

These should remain the fastest debug and parity backends:

- easy visual snapshot testing
- easy inspection of generated geometry
- fast iteration on diagram lowering

---

## Recommended Implementation Order

1. Land `diagram-ir` with graph family first.
2. Land `diagram-layout-graph`.
3. Land `diagram-to-paint`.
4. Add dashed stroke support to `paint-instructions` and first-class backends.
5. Use Canvas and SVG as reference outputs.
6. Bring Direct2D to native parity for the first subset.
7. Add document embedding via TE04 fenced-block transform plus a UI02 paint fragment.
8. Add sequence family.
9. Add waveform family.

This gets us real end-to-end value early without over-designing sequence and
waveform semantics before the graph pipeline exists.

---

## Practical Conclusion

The architecture we want is:

```text
source-specific parser packages
  -> shared family-aware Diagram IR
  -> shared family layout packages
  -> shared diagram-to-paint compiler
  -> PaintScene / PaintInstruction
  -> backend-specific execution
```

That means:

- new diagram syntaxes are mostly frontend work
- new diagram families are mostly IR + layout work
- backend growth is mostly paint parity work
- `PaintInstruction` remains the universal rendering target

This is the right scaling shape for Mermaid today and for other text-diagram
languages later.
