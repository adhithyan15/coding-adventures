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

## Bounded Wardley configuration and checked geometry

Wardley source configuration uses direct owned mapping ancestry or a closed init
object; an init object replaces the mapping wholesale, including empty objects.
Nested/sequence/block-scalar/quoted-tail decoys cannot supply a direct field.
Width/height/radius are finite positive values at most65536; padding/label offset
are finite nonnegative values through65536; configured fonts are1..65536.
Invalid source settings use field defaults. Body size uses the same finite cap,
with a named source error for positive over-cap values.

The checked typed route `try_layout_wardley` rejects invalid configuration,
non-normalized node/evolution coordinates and unknown references before mapping,
and rejects nonfinite derived geometry. Compatibility `layout_wardley` refuses
unsupported typed values. Ordinary default/minimum/cap layouts remain literal
whole-layout controls. Stage fonts use max(axisFontSize-2,1); title fonts use
axisFontSize*1.05. Grid paths, node/anchor radii, source precedence, all glyphs,
seven metadata fields and ordered cached font queries have exact whole-scene
oracles. General font fitting, containment, area/node-count resource guarantees
and caller-forged resolved Paint layouts remain outside this bounded contract.

Spec3b6e precedes closed incoming6c merge8f43 and test-first source/typed/Paint
controls. Semantic reds parser2P3F101/geometry1P3F101/paint0P1F101 are retained.
Actual3600 full strict101/wrapper1 stopped before full tests on negative-fixture
assignment formatting;1b216e6b changes only Rust whitespace plus documentation,
preserving every assertion and independently specified numeric table.

Actual tested1b216e6b: focused5parser/8geometry/2paint PASS;94 complete IR cells,
2 exact size errors,271 geometry events and8 complete source/IR/layout/PaintScene/
glyph/metadata/ordered-query cells. Full affected14 packages981P0F0I/47targets
and strict all-target warnings/unknown-lints denial both exit0. Tests02de82c1/
strict54460559 and independent source/terminal reviews bind that exact source.
All1464 raw Rust/TOML inputs, reverse14/forward50/style827 objects, native2853,
registry376, only4 local style-version changes and resources21+3 are verified.
The13 retained Venn fixture files contain232 complete scene/query cells;
37 complete Ishikawa cells and67 causal parser IR controls remain exact.

Native execution is independently attributed to actuale3e1106c1836/78,
compiler895/fullLANG506/strict6 and146sources/876 six-backend cells. Historical
Closure36/3705+3 conservative static inputs retain316e attribution; no fresh
Closure execution or dynamic/toolchain/OS resource closure is inferred.
Integrated main6c4e is an ancestor. Later inspected main987ab434 Wardley theme
and e0724e0 human-language snapshot changes are NOT integrated/validated here.
This is local bounded acceptance and unmerged progress, with current-main/
hosted exact-head CI/conflict/review/actual merge gates and the actual macOS
phase-refusal root cause OPEN. No TS/book runtime or general visual-parity claim.

Base ESTree canonical/own types/no new crates.io dependencies remain owner
decisions. Full Closure parity/output-byte-source/full transformation lineage/
stable IDs/spans/pass-sweep/CV04b remain OPEN. Miri **DEFERRED/PENDING**, never
passed; frozen run/checkout/inputs/target/receipts untouched/unpolled, no duplicate
or termination. Delivery first, genuine final-source Miri before full completion
or disabling the ACTIVE heartbeat.
