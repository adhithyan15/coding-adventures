# Book and app illustration assets

This directory holds the **Class C** assets of the human-languages visual system:
model-generated raster art for scenes, objects, and cultural context — the colour the
books currently lack. It sits beside [`../_fonts/`](../_fonts/README.md) and follows the
same posture: the artefact is committed, its terms are written down next to it, and CI
gates on both.

- `illustrations/<track>/` — the committed image files, one directory per language track.
- A provenance sidecar JSON beside every asset.
- [`LICENSE.md`](./LICENSE.md) — the recorded licensing decision and the required
  sidecar fields. **No asset ships without a sidecar and a recorded licence.**

Nothing lives here yet; the pipeline is tracked as HL-C12 in the
[`../BACKLOG.d/`](../BACKLOG.d/) shard directory.

## The three figure classes

[HL06](../../../specs/HL06-visual-system.md) separates figures by **what makes them
trustworthy**, because that determines how each is produced and verified.

| Class | What it is | Source of truth | How it is trusted |
|---|---|---|---|
| **A — script figures** | Stroke-order build-ups: glyph outline, pen path, segment labels, lift points | `DUCTUS` in `strokes.ts`, validated against the vendored font outlines | Deterministic SVG, hash-gated; every pen point verified to land on real ink |
| **B — data diagrams** | Etymology trees, cousin webs, sound-articulation diagrams, gender maps | The canonical lesson AST | Deterministic SVG, hash-gated; every node must trace to a lesson assertion — a diagram may not introduce a claim |
| **C — illustrations** | Scenes, objects, cultural context | A model, a prompt, and a human choosing to commit the result | Not regenerable, so the committed file *is* the artefact: provenance sidecar plus a `sha256` gate, and a per-track size budget |

A and B are **generated and reproducible** — delete them and a clean checkout rebuilds
them byte-identically. C is **vendored**, in the same sense the Noto fonts are: it
cannot be reproduced, so it is committed, hashed, and documented.

## The glyph monopoly

> **Only the Class A font-derived pipeline may ever depict a letter, glyph, ligature,
> conjunct, or handwriting stroke.** No drawn, traced, or model-generated image may
> render script. Ever.

Class C illustrations are therefore restricted to **non-linguistic subjects**: no
script, no glyphs, no handwriting, no transliteration, and no claim about a language's
structure or history. Class A holds the glyph monopoly; Class B holds every factual
diagram; Class C gets the market stall, the tea glass, and the rooftop at dusk.

The reason is that a subtly wrong Tamil ண looks completely correct to precisely the
audience that cannot yet read Tamil. The error would not merely ship — it would ship
*as the lesson*, and the reader has no way to catch it. The Class A pipeline derives
its shapes from the actual font outlines and verifies every pen point against them, so
it cannot be subtly wrong in that way. A generated illustration can be, and would be
believed.

This rule is not a style preference and is not waivable per asset. If a figure needs to
show script, it is a Class A figure; if it needs to assert a fact, it is a Class B
diagram.

## Licensing in one line

The books stay CC BY-SA 4.0; generated illustrations are marked `CC0-1.0` with
`rightsAsserted: false`. The reasoning, the required sidecar fields, and the two
operational constraints on prompting and generator terms are in
[`LICENSE.md`](./LICENSE.md).
