# diagram-ir

`LayoutedRailroadRule` now carries required `start_marker` and `end_marker`
points. External Rust struct literals must provide both fields; this is a source
compatibility change. Semantic rule names and expression labels remain intact;
the layout layer derives the `name =` and `? text ?` display labels separately.
The checked layout boundary verifies both coordinates of each marker are finite.
Marker enclosure and arbitrary forged layout safety remain separate limits.

`RailroadConfig.compact_mode` preserves authored `compactMode` as a bool,
defaulting to false. Layout retains that configuration and PaintScene exports
its normalized string in `railroad.compactMode`. This is configuration transport;
it introduces no compact-specific layout or geometry branch.

Railroad configuration carries `arc_radius` (authored `arcRadius`, default 10)
alongside font family/size, portable colors, stroke width and marker settings.
Numeric fields share the finite 65536 cap with spacing; font size is strictly
positive, and stroke width, marker radius and arc radius are nonnegative.
Parsers and checked hierarchy layout retain accepted values without clamping.

`RailroadConfig` carries padding and vertical/horizontal separation unchanged.
Each setting has a supported finite inclusive0..65536 range, exported as
`MAX_RAILROAD_SPACING` and enforced by parsers and checked hierarchy layout.
Defaults remain the producer's10/8/10 values; no silent clamping is used.

Semantic diagram intermediate representation (IR) shared across all diagram
pipelines in the coding-adventures monorepo (DG00).

## What it is

`diagram-ir` is the shared vocabulary between diagram parsers and layout engines.
Every diagram source format (DOT, Mermaid, PlantUML) produces a `GraphDiagram`.
Every layout engine (`diagram-layout-graph`, `diagram-layout-sequence`) consumes
a `GraphDiagram` and produces a `LayoutedGraphDiagram`.

```text
dot-parser / mermaid-parser
  → GraphDiagram          ← this crate
  → diagram-layout-graph
  → LayoutedGraphDiagram  ← this crate
  → diagram-to-paint
  → PaintScene
```

## Key types

| Type                     | Description                               |
|--------------------------|-------------------------------------------|
| `GraphDiagram`           | Pre-layout semantic graph (nodes + edges) |
| `GraphNode`              | Node with id, label, shape, optional style |
| `GraphEdge`              | Edge with source, target, kind, optional label |
| `DiagramDirection`       | `Tb / Lr / Rl / Bt` flow direction        |
| `DiagramShape`           | `Rect / RoundedRect / Ellipse / Diamond`  |
| `DiagramStyle`           | Optional style overrides (all `Option<_>`) |
| `ResolvedDiagramStyle`   | Fully-resolved style after applying defaults |
| `LayoutedGraphDiagram`   | Post-layout diagram with absolute geometry |
| `LayoutedGraphNode`      | Node with `x, y, width, height`           |
| `LayoutedGraphEdge`      | Edge with routed `Vec<Point>` polyline    |

## Usage

```rust
use diagram_ir::{GraphDiagram, GraphNode, GraphEdge, DiagramDirection,
                  DiagramLabel, EdgeKind, resolve_style};

let node = GraphNode {
    id: "start".to_string(),
    label: DiagramLabel::new("Start"),
    shape: None,
    style: None,
};
```

## Spec

See `code/specs/DG00-diagram-ir.md` for the full architecture and
`code/specs/DG01-dot-pipeline-rust.md` for the Rust-specific pipeline.


## Bounded spacing and transport reconciliation

MAX_SWIMLANE_FLOWCHART_SPACING defines the own IR production range 0..=65536 for nodeSpacing, rankSpacing and diagramPadding. This bounded policy is separate from unrestricted upstream parity.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.


## Global curves and checked ranks

Global Swimlane flowchart curve uses the existing own curve enum; absent configuration retains None and all previous defaults.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.


## Native validation checkpoint

Cluster background/border/title and node-text/arrowhead/stroke-width theme fields retain absent defaults. Own types and registry identities remain unchanged; this boundary expansion does not claim compiler transformation lineage.

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
