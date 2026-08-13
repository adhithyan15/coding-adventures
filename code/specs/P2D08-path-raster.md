# P2D08 — `path-raster`: filling and stroking paths into pixels, with no dependencies

**Status:** specification, 2026-08-13
**Layer:** sits on `pixel-container`; consumed by `script-ductus` and, through it,
by the Human Languages book pipeline.
**Motivated by:** [HL11](HL11-drizzled-script-ramp.md) §6, which requires a
**step-by-step raster figure** for every writable letter, and by the owner's
choice of PNG raster over vector for those figures.

---

## 1. The gap this fills

The paint stack renders a `PaintScene` to several places and to pixels in only
one of them:

| backend | target | needs |
|---|---|---|
| `paint-vm-svg` | SVG text | nothing |
| `paint-vm-ascii` | terminal | nothing |
| `paint-vm-canvas` | `PixelContainer` | **a Canvas implementation** |

`paint-vm-canvas` can produce pixels, and its own README says how: *"server-side
runtimes that provide a Canvas implementation (e.g. `node-canvas` or Skia)."*
Both are third-party native modules, which this project does not take — the
standing rule is zero-dependency implementations, and the whole point of writing
a PNG encoder by hand was not to reach for one.

So today a book figure can be produced as SVG and then converted by
`rsvg-convert`, which `spanish/book/build.sh` already requires and already fails
without. **That is a third-party binary on the critical path of the one gate that
renders a page.** This package removes it.

## 2. What it does

Two operations, both onto a `PixelContainer`:

```
fillPath(target, path, paint)      the interior of a closed path
strokePath(target, path, paint)    a band of given width along a path
```

A `path` is the same flattened point-run vocabulary the rest of the stack speaks —
move/line/quadratic/cubic/close — because both of its callers already produce it:
`truetype.ts` returns glyph contours with quadratic control points, and
`strokes.ts` returns authored pen paths.

### 2.1 Fill rule

**Nonzero winding by default; even-odd on request.** This is not a detail to
default carelessly. A TrueType glyph with a counter — the hole in ஔ, the bowl of
अ — is two contours wound in opposite directions, and the counter is a hole only
under a winding rule that respects direction. Under even-odd, a glyph whose
contours happen to be wound the same way loses its hole and fills solid. The
letter still looks like a letter, which is what makes it a bad bug: it is wrong
in a way a reader will not report.

### 2.2 Anti-aliasing

**Coverage-based, computed per scanline, not supersampled.** A stroke in these
figures is around two pixels wide at the sizes a book prints; aliasing it turns
a smooth curve into a staircase and a learner reads the staircase as part of the
letter's shape. Coverage is exact area under the sampled span rather than a count
of hits, so a hairline stroke has a defined grey rather than dropping out.

### 2.3 Determinism

**Byte-identical output for identical input, on every platform.** The figures are
byte-gated by `core/generated-figure-hashes.json`; a rasterizer that emits a
different last-bit on a different machine turns that gate from a guarantee into
noise. So: no floating-point accumulation order that depends on iteration order
of a hash map, coverage quantised to 8 bits by a stated rounding rule, and a test
that renders the same scene twice and compares bytes.

## 3. What it does NOT do

- **No text layout.** A caption is drawn by its caller as glyph outlines, using
  the TrueType reader that already exists. This package fills paths; it does not
  know what a font is. That keeps the one hard, culturally-loaded problem —
  shaping Indic text — out of a module that has no business holding an opinion
  about it.
- **No `PaintScene` execution.** That belongs in a `paint-vm` backend built on
  this, and is P2D08's successor rather than its scope. Keeping the rasterizer
  free of the instruction set is what lets `script-ductus` use it directly.
- **No image scaling, blending modes, or gradients.** Solid fills only, until a
  figure needs more.

## 4. Verification

The hard part of testing a rasterizer is that "looks right" is not a test.

| claim | how it is checked |
|---|---|
| a filled square covers exactly its pixels | count set pixels against the analytic area |
| a glyph's counter stays a hole | fill a real font's `o`/`अ` and assert the interior pixel is background |
| winding direction matters | fill the same contours under both rules and assert they DIFFER |
| anti-aliasing is coverage, not luck | a 45° edge's boundary pixels are strictly between fill and background |
| a hairline does not drop out | stroke at width 0.5 and assert every scanline it crosses is touched |
| determinism | render twice, compare bytes; render the transpose, compare areas |
| no unbounded work on hostile input | a path with a million segments, and one with NaN coordinates, terminate |

The last row matters because a path can arrive from a font file. `truetype.ts`
is already hardened against hostile fonts; a rasterizer that loops forever on the
contours it returns would put that back.

## 5. Why it is worth writing rather than vendoring

It is roughly a scanline loop, an active-edge list, and a coverage accumulator —
the classic algorithm, and one of the few places where the literate-programming
rule pays a real dividend: a reader who works through this file understands how
every 2-D graphics stack they have ever used actually puts a curve on a screen.
It also, concretely, deletes a binary dependency from the book build.

## 6. Provenance

| claim | source |
|---|---|
| `paint-vm-canvas` needs a Canvas implementation | its own README, "Where it fits" |
| `rsvg-convert` is required by the book build | `spanish/book/build.sh`, which exits 1 without it |
| PNG for book figures | owner decision, 2026-08-12, recorded in HL11 |
| the filmstrip's contents | HL11 §6 |
| figures are byte-gated | `core/generated-figure-hashes.json` and `check:figures` |
