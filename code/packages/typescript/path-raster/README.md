# `@coding-adventures/path-raster`

**Given the outline of a shape, which pixels are inside it, and how much of each
one?**

Every graphics stack answers that somewhere, and almost none of them answer it
where you can read it. This is a scanline rasterizer with anti-aliasing, written
to be read: about 200 lines of algorithm and rather more lines of explanation.

```ts
import { fillPath, strokePath } from "@coding-adventures/path-raster";

fillPath(pixels, glyphOutline, { r: 20, g: 20, b: 30 });          // nonzero by default
strokePath(pixels, penPath, { r: 59, g: 91, b: 219 }, { width: 3 });
```

## Why it exists

The Human Languages books need a step-by-step picture of a letter being written
([HL11](../../../specs/HL11-drizzled-script-ramp.md) §6), as PNG. The paint stack
can already produce pixels — but only through `paint-vm-canvas`, whose own README
says it needs *"a Canvas implementation (e.g. node-canvas or Skia)"*. Both are
third-party native modules, which this project does not take.

The alternative in use today is to emit SVG and shell out to `rsvg-convert`,
which the book build already requires and already refuses to run without. **That
put a third-party binary on the critical path of the one gate that renders a
page.** This removes it.

## How it works

Sweep a horizontal line down the image, one row at a time, and find where the
shape's edges cross it:

```
y = 7.5   ─────┬───────────┬──────────   two crossings: inside between them
              ╱             ╲
y = 8.5   ───┬───┬───────┬───┬────────   four crossings: inside between the
            ╱     ╲     ╱     ╲           1st-2nd and the 3rd-4th
```

Two decisions do most of the work.

**The fill rule.** Which pairs count as "inside" is what decides whether a letter
keeps its hole. A counter — the bowl of `अ`, the loop of `வ` — is two contours
wound in opposite directions, and it is a hole only under a rule that respects
direction. The default is **nonzero**, which is what TrueType specifies and what
its fonts are wound for. Even-odd is available and is the wrong default: a glyph
whose contours happen to be wound the same way loses its counter and fills solid,
and *it still looks like a letter*, which is what makes it a bad bug.

**Coverage, not centre-sampling.** Each pixel gets the fraction of its area the
shape covers. A stroke in these figures is about two pixels wide when a book
prints it; sampling centres turns a smooth curve into a staircase, and a learner
reads the staircase as part of the letter's shape. Vertically the row is sampled
16 times; horizontally the inside spans are known *exactly*, as intervals between
crossings, so that direction is arithmetic rather than sampling.

## Verified end to end

Fed real outlines from the shipped Noto fonts, through `script-ductus`'s TrueType
reader and out through `image-codec-png`:

```
வ  4,148 ink pixels    अ  4,686    త  5,839    ക  3,623      (of 160x160)
```

Every counter is a hole. No dependency outside this repository was involved in
producing that PNG.

## What it does not do

- **No text layout.** A caption is drawn by its caller as glyph outlines. This
  package fills paths; it does not know what a font is — which keeps the one
  hard, culturally-loaded problem, shaping Indic text, out of a module that has
  no business holding an opinion about it.
- **No `PaintScene` execution.** That belongs in a `paint-vm` backend built on
  this. Keeping the rasterizer free of the instruction set is what lets
  `script-ductus` use it directly.
- **No scaling, blending modes, or gradients.** Solid fills only, until a figure
  needs more.

## Bounded, because the input is a font file

A path here can come from a `.ttf`, so the amount of work one input may ask for is
capped: flattening stops at 200,000 edges, and edges are bucketed by the rows
they touch rather than re-tested on every row. The second is why a taller canvas
no longer costs more for the same shape — 20,000 edges over 4,096 rows went from
1.9 s to 27 ms — and it is pinned by a test that asserts the *property* rather
than the number.

One known cost is **not** mitigated, and is documented rather than hidden: a
stroke whose width is comparable to the canvas makes every join sweep the whole
image, so many vertices at such a width is slow (300 vertices at width 362 on a
256×256 canvas: 622 ms, against 14 ms at width 4). Width is clamped to the canvas
diagonal, which bounds the arithmetic and saves about nine milliseconds of those
622 — it is hygiene, not a fix. Width is caller-supplied, and every real caller
asks for two or three pixels.


## Tests

28 tests, and none of them compares a picture to a stored picture. A golden-image
test passes on a rasterizer that is wrong in exactly the way the golden was
captured. Instead every assertion is a property that can be stated without
looking:

| claim | how it is checked |
|---|---|
| area is the geometry's area | analytic area of squares, triangles, a circle |
| a counter stays a hole | fill nested contours, assert the interior is background |
| the winding rule matters | fill the same contours both ways, assert they DIFFER |
| anti-aliasing is coverage | a 45° edge's pixels are strictly between the two colours |
| a hairline does not drop out | stroke at width 0.5, assert every crossed row is marked |
| joints do not double-darken | the darkest pixel of a bent stroke is exactly the paint |
| determinism | render twice, compare bytes; render the transpose, compare areas |
| hostile input terminates | NaN and Infinity coordinates, a curve with 1e9 control points |
| work stays bounded | 20,000 curve commands cap at 200k edges; render time does not scale with canvas height |

The determinism row is not decoration: these figures are byte-gated by
`core/generated-figure-hashes.json`, and a rasterizer whose last bit depends on
the machine turns that gate from a guarantee into noise.

Specified in [P2D08](../../../specs/P2D08-path-raster.md).
