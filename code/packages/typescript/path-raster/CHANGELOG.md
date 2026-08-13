# Changelog

## 0.1.0 — the first rasterizer in the repo

`fillPath` and `strokePath`, onto a `PixelContainer`, with no dependency outside
this repository.

Written because the Human Languages books need a step-by-step picture of a letter
being written (HL11 §6) as PNG, and the only existing route to pixels —
`paint-vm-canvas` — needs a third-party Canvas implementation, while the route in
use today shells out to `rsvg-convert`. That is a binary dependency on the
critical path of the one gate that renders a page.

### Added
- **`fillPath`** — scanline fill with coverage anti-aliasing. Nonzero winding by
  default, even-odd on request.
- **`strokePath`** — a band of given width, built from per-segment quads plus a
  disc at every vertex, accumulated into one coverage mask so overlapping joints
  do not double-darken.
- **`flattenPath`** — curves to polylines, with the segment count taken from each
  curve's own flatness rather than a fixed number, so a tight hook in a Malayalam
  letter gets the segments it needs and a nearly-straight curve costs one.

### Three bugs found by the tests, all mine, all before this shipped
- **A circle enclosed more than its bounding box.** The control-point offset for a
  quarter arc drawn as one cubic is `(4/3)(√2 − 1) ≈ 0.5523`; the first draft used
  `4/3`, which is a different formula entirely. Caught by asserting the area is
  `πr²` rather than by looking at a picture, which is exactly why the test suite
  contains no stored images.
- **An unclosed subpath rendered as nothing at all.** The doc comment promised
  every subpath came back closed; the code never did it. An unclosed rectangle has
  one non-horizontal edge, so the sweep found fewer than two crossings on every
  row and drew empty space — failing silently rather than visibly.
- **A half-pixel-offset square measured 99.93 instead of 100.** Not a bug: the
  output is 8-bit, so a half-covered edge pixel lands on 128/255 and forty of them
  accumulate 0.07 of a pixel. The test tolerance was wrong, and now says so.

### Two bounds, both from the security review, both measured

The paths this fills come out of **font files**, so "how much work can one input
ask for?" is a real question rather than a theoretical one. `truetype.ts` caps its
own work but not its output — a simple glyph may declare 65,536 points, and each
curve then multiplies by up to 256 here.

- **An edge budget.** `MAX_EDGES = 200_000`, far above any real letter. Before it,
  20,000 curve commands flattened to **5,120,002 points and 438 MB of heap**; now
  they flatten to 200,002. Overrunning truncates rather than throwing: a build
  that draws a slightly clipped glyph and says so beats one that dies.
- **A y-bucketed edge index.** The sweep used to ask every edge, on every one of
  the 16 sample lines of every row, whether it crossed — so an edge confined to
  two scanlines was re-tested down the whole image. Each edge is now filed once
  under the rows it touches. Measured:

  | input | before | after |
  |---|---:|---:|
  | 20,000 edges, 64x1024 | 480 ms | **13 ms** |
  | 20,000 edges, 64x4096 | 1,937 ms | **27 ms** |
  | 40,000 edges, 64x1024 | 924 ms | **81 ms** |
  | 1,000 `quad_to`, 512x1024 | 6,612 ms | **1,092 ms** |

  The second row is the point: four times the canvas height on the *same shape*
  used to cost four times as much. The height factor is gone.

- **A stroke-width clamp**, to the canvas diagonal — and it is worth being blunt
  that this one does **not** work as a mitigation. Measured properly:

  | 300 vertices, 256x256 | time |
  |---|---:|
  | width 4 | 14 ms |
  | width 1e6 | 631 ms |
  | width 362 (the clamp) | **622 ms** |

  Nine milliseconds of six hundred. The cost is not driven by the width being
  absurd but by its being *comparable to the canvas*: past that, every join
  disc's box covers the whole image and each of the 2V sweeps every pixel. A
  width of exactly the diagonal costs what a width of a million costs. The clamp
  is kept as arithmetic hygiene; the cost is **recorded as known**, not fixed.
  Stroke width here is caller-supplied and every real caller asks for two or
  three pixels.

The first two are pinned by regression tests, including one that asserts render
time does **not** scale with canvas height — the property, not the number. The
clamp is not, and that is deliberate: the test that claimed to check it passed
with the clamp deleted, because both widths paint the canvas solid. A test that
cannot fail was worse than no test, so it now says what it actually pins.

The rewrite is behaviour-preserving in the way that matters: வ, अ, త and ക
rasterize to byte-identical ink counts before and after.

### Verified end to end
Real outlines from the shipped Noto fonts, through `script-ductus`'s TrueType
reader and out through `image-codec-png`: வ, अ, త and ക all render with their
counters intact, which is the nonzero winding rule working on real font data
rather than on synthetic contours.

Specified in `code/specs/P2D08-path-raster.md`.
