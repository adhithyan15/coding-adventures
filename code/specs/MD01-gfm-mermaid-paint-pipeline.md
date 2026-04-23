# MD01 — GFM Mermaid Diagrams Through Paint VM

## Overview

This note answers a narrow architecture question:

> Do we already have enough packages and abstractions to take a fenced
> Mermaid diagram in GFM and render it through backend-native paint APIs
> such as Metal and Direct2D via `PaintScene` / `PaintInstruction`?

Short answer:

- **Yes for the paint IR direction.** `PaintScene` is already expressive enough
  for Mermaid-style diagrams.
- **Not yet for the markdown/document seam.** The current GFM and document
  pipeline still collapses fenced blocks too early.
- **Not yet for full native backend parity.** Direct2D is already close enough
  for a first real diagram backend; Metal and GDI still have important gaps.

This analysis was done against `origin/main` at commit `6b0c65934`.

---

## What Already Exists

### 1. GFM parsing already has the raw ingredients for a general fenced-block seam

The repo already has first-party GFM packages in multiple languages, including:

- `code/packages/typescript/gfm-parser`
- `code/packages/typescript/gfm`
- `code/packages/rust/gfm-parser`
- `code/packages/rust/gfm`
- `code/packages/csharp/gfm-parser`

Today the parser already preserves the key signal we need for a fenced block:

- fenced code blocks become `CodeBlockNode`
- `CodeBlockNode.language` is populated from the fence info string

So this source:

````markdown
```mermaid
flowchart LR
  A --> B
```
````

already parses as "this is a code block whose language is `mermaid`".

That means the line-level parser is **not** the primary blocker. The missing
piece is a more general AST primitive that preserves fenced blocks before they
are forced into code rendering.

### 2. The paint IR is already expressive enough for diagram rendering

The current TypeScript `paint-instructions` package already exposes the core
shapes a Mermaid renderer wants:

- `rect`
- `ellipse`
- `path`
- `line`
- `glyph_run`
- `text`
- `group`
- `layer`
- `clip`
- `gradient`
- `image`

For Mermaid-class diagrams this is already a strong target IR:

- nodes map to `rect`, `ellipse`, or `path`
- connectors map to `line` or `path`
- labels map to `glyph_run` or `text`
- clusters/subgraphs map to `group`, `clip`, and optional `layer`

This is exactly the right direction: parse Mermaid once, lower it into
`PaintInstruction`, then let each backend do its own platform-native drawing.

### 3. Several backends already exist

Current backend picture:

| Backend | Status for diagram work |
| --- | --- |
| TypeScript `paint-vm-canvas` | Strong |
| TypeScript `paint-vm-svg` | Strong |
| Rust `paint-vm-direct2d` | Strong first native target |
| Rust `paint-metal` | Partial |
| Rust `paint-vm-gdi` | Minimal fallback only |
| Rust `paint-vm-ascii` | Debug-only / rect-only mindset |

The important point is that the project is **not** missing the backend concept.
It already has the right Paint VM shape.

### 4. There is already end-to-end markdown rendering infrastructure

The repo already has working text/document pipelines such as:

- `code/programs/typescript/markdown-canvas-demo`
- `code/programs/rust/markdown-reader`
- `code/specs/MD00-commonmark-to-metal.md`

So Mermaid support does not need to invent a brand-new rendering universe.
It needs to plug into an existing markdown -> document -> layout -> paint flow.

---

## The Real Gaps

## Gap 1 — Fenced blocks are collapsed too early

Today the GFM parser stops at:

```text
GFM source
  -> DocumentNode
  -> CodeBlockNode { language: "mermaid", value: "..." }
```

That is correct for ordinary code rendering, but it is too eager for richer
fenced-block features.

The better seam is:

```text
FencedBlockNode(name="mermaid", info="mermaid", value="...")
  -> Mermaid parser
  -> Diagram AST / Diagram layout
  -> PaintScene fragment
```

Without that primitive, the rest of the pipeline sees only "a code block" and
renders the Mermaid source literally.

## Gap 2 — `document-ast-to-layout` has no general fenced-block hook

Right now `document-ast-to-layout` renders every `code_block` as preformatted
monospace text. It has no generic "fenced block fallback plus optional
specialized transform" contract.

That means even though the parser recognizes fenced metadata, the layout layer
does not yet have a first-class place to intercept and reinterpret it.

## Gap 3 — Layout IR has no native "embedded paint fragment" leaf

This is the most important abstraction gap.

The current `layout-ir` leaf content kinds are effectively:

- text
- image

That is enough for prose and pictures, but not for "a box in document flow that
internally contains arbitrary vector paint instructions".

For Mermaid we do **not** want to rasterize to an image too early, because the
goal is native Paint VM rendering:

```text
Mermaid -> semantic diagram -> PaintInstructions -> Metal / Direct2D / ...
```

So the layout layer needs one of these:

1. A new leaf kind such as `paint_fragment`
2. A diagram container contract in `ext`
3. A post-layout composition step that can splice a nested `PaintScene`

Without one of those, Mermaid blocks can only be treated as text or as images.

## Gap 4 — There is no Mermaid parser package yet

The repo contains packages that **emit Mermaid text** from graph structures
(`directed-graph` visualization helpers), but that is the opposite direction.

We do **not** yet have:

- `mermaid-parser`
- `diagram-ast`
- `diagram-layout`
- `diagram-to-paint`

Those are the missing semantic layers.

## Gap 5 — Native backend support is uneven

### Direct2D

`paint-vm-direct2d` is already the best native candidate for a first Mermaid
backend. It dispatches:

- `rect`
- `line`
- `group`
- `clip`
- `glyph_run`
- `ellipse`
- `path`
- `layer`
- `image`

`gradient` is still not implemented, but that is not a blocker for a first
Mermaid subset.

### Metal

`paint-metal` is not ready for general Mermaid output yet.

Current practical support is centered around:

- `rect`
- `line`
- `group`
- `clip`
- glyph overlay for `glyph_run`

Important Mermaid-relevant gaps still exist:

- no real `path` rendering
- no real `ellipse` rendering
- no `layer`
- no `image`
- no `gradient`

This matters because Mermaid nodes and connectors quickly need more than
axis-aligned rectangles:

- diamonds
- rounded clusters
- arrowheads
- curved or orthogonal paths

### GDI

`paint-vm-gdi` is currently only a fallback for very simple scenes:

- `rect`
- `line`
- `group`
- `clip`

That is not enough for a serious Mermaid implementation unless we accept a very
degraded subset.

## Gap 6 — Rust/native paint IR currently has `GlyphRun`, not `Text`

TypeScript has both `PaintGlyphRun` and `PaintText`.
Rust/native currently centers on `PaintGlyphRun`.

That is fine for Direct2D and a mature native text stack, but it means the
cross-language story is not yet perfectly aligned. For Mermaid labels this is
not fatal, but it is worth preserving as a design constraint:

- native backends should prefer shaped glyph runs
- web/canvas can still use `PaintText`

---

## What This Means Architecturally

## The paint abstraction is already the correct target

We do **not** need a separate "Mermaid renderer abstraction".
The existing target should stay:

```text
Mermaid source
  -> Mermaid parser
  -> DG00 Diagram IR
  -> DG00 graph layout
  -> PaintScene / PaintInstruction
  -> Paint VM backend
```

That keeps Metal, Direct2D, SVG, Canvas, and any future backend on one shared
scene model.

## The GFM parser probably needs a general fenced-block primitive, not a rewrite

For Mermaid and future fenced features, the parser is mostly doing the right
thing already. The main parser-side improvement is:

1. Preserve fenced blocks as a generic TE04 `FencedBlockNode`
2. Keep both a fast dispatch key and the full fence info string
3. Keep Mermaid parsing out of the core GFM syntax parser

In other words:

- the GFM parser should stay responsible for Markdown
- a Mermaid package should be responsible for Mermaid
- default consumers should still be able to render unknown fences like code blocks

That separation is healthy.

## The biggest missing seam is between document layout and paint

If Mermaid must participate in markdown flow like a normal block element, we
need a first-class way to say:

> "This block occupies width X and height Y in layout, but when it paints, it
> emits its own nested paint instructions instead of plain text/image content."

That is the missing abstraction.

---

## Recommended Plan

## Phase 1 — Add the general fenced-block primitive

Adopt TE04 `FencedBlockNode` in the GFM layer:

```text
fenced_block
  name
  info
  value
```

Default consumers should render unknown fenced blocks like code blocks.
Specialized transforms can claim names such as `mermaid`.

## Phase 2 — Introduce a native diagram block seam

Add a layout/content concept for embedded native paint fragments:

```text
paint_fragment
  intrinsic width / height
  paint children or paint-scene fragment
```

This is still the cleanest way to keep Mermaid native all the way down once a
fenced block has been claimed by a diagram transform.

## Phase 3 — Add a Mermaid-specific transform package

Introduce a package whose job is:

```text
DocumentNode
  -> walk blocks
  -> find FencedBlockNode(name="mermaid")
  -> parse Mermaid source
  -> lower into DG00 Diagram IR
  -> replace with diagram-aware block representation
```

Possible package names:

- `gfm-mermaid`
- `document-ast-diagram-transform`
- `mermaid-block-transform`

This should be a transform package, not part of the core parser.

## Phase 4 — Start with a focused Mermaid subset

The first supported subset should be small and backend-friendly:

- `flowchart`
- basic `stateDiagram-v2`
- straight connectors
- rectangular / rounded / diamond nodes
- text labels
- subgraphs later

Avoid starting with:

- sequence diagrams
- class diagrams
- CSS-like Mermaid theming
- animation
- icon/image embedding

The goal is to prove the pipeline, not to clone Mermaid.js in one step.

## Phase 5 — Use Direct2D as the first native reference backend

Direct2D is the best first native target because it already has working support
for most of the scene primitives a Mermaid subset needs.

Suggested backend order:

1. SVG / Canvas for fast debug output
2. Direct2D for the first native parity target
3. Metal after `PaintPath` and ellipse support land
4. GDI only as a degraded fallback

## Phase 6 — Close the Metal gap

If Metal is a must-have first-class target, the next missing backend work is
clear:

- implement `PaintPath`
- implement `PaintEllipse`
- keep `GlyphRun` path solid
- optionally add `Layer` later

Until then, Metal can render only a narrow subset of Mermaid output.

---

## Practical Conclusion

We already have enough infrastructure to justify this direction.

What we **have**:

- a good GFM parser
- a shared document AST
- a shared paint IR
- several paint backends
- native markdown rendering pipelines

What we **do not yet have**:

- a general fenced-block primitive in the GFM AST
- a Mermaid parser
- a diagram AST/layout layer
- a document-to-diagram transform seam built on fenced blocks
- a layout leaf that can carry native paint fragments
- full Metal parity for diagram primitives

So the answer is:

- **enough to begin**
- **not enough to ship full Mermaid rendering yet**

The next architectural move should not be "make GFM much more magical".
It should be:

1. add a general fenced-block primitive after GFM parse
2. add a diagram-aware transform on top of that primitive
3. add a native paint-fragment concept to the layout/paint bridge
4. target Direct2D first, then close Metal path support

That path preserves the repo's strongest design choice: one backend-neutral
scene model, many native renderers.
