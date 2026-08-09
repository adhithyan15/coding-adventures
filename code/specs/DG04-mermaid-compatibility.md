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

### Sequence Native Slice

The first sequence vertical slice is grammar-backed and covers participant and
actor declarations, aliases, implicit participants, solid and dotted message
arrows, open/filled/cross/point arrowheads, bidirectional messages, notes,
activation/deactivation, titles, and automatic numbering. It lowers through
`diagram-layout-sequence` to existing path, rectangle, dashed-stroke, and glyph
PaintInstructions and is exercised by a Mermaid-to-Metal-to-PNG fixture.

Nested Mermaid 11.16.1 control blocks (`loop`, `opt`, `alt`/`else`, `par`/`and`,
`par_over`, `critical`/`option`, `break`, and `rect`) lower into ordered semantic
block events. Sequence layout resolves those events into nested frames and
branch dividers before existing PaintInstructions render them. Participant
`create` and `destroy` statements lower into lifecycle events; layout uses them
to place dynamic participant headers, bound lifelines, and emit destruction
markers. Participant configuration metadata, links, and advanced arrow variants
remain compatibility work. The family remains partial until those forms and the
pinned upstream corpus pass; unsupported forms must fail grammar validation
rather than degrade silently.

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
