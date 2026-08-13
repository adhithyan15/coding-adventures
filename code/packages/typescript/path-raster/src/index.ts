// ---------------------------------------------------------------------------
// path-raster — turning a path into pixels, by hand
// ---------------------------------------------------------------------------
//
// This file answers one question that every graphics stack answers somewhere and
// almost none of them answer where you can read it: **given the outline of a
// shape, which pixels are inside it, and how much of each one?**
//
// It exists because the Human Languages books need a step-by-step picture of a
// letter being written (HL11 §6) and the owner chose PNG. The paint stack can
// already produce pixels — but only through `paint-vm-canvas`, whose own README
// says it needs "a Canvas implementation (e.g. node-canvas or Skia)". Both are
// third-party native modules. The alternative in use today is to emit SVG and
// shell out to `rsvg-convert`, which the book build already requires and already
// refuses to run without. So there was a binary dependency sitting on the
// critical path of the only gate that renders a page, and this removes it.
//
// See `code/specs/P2D08-path-raster.md`.
//
// THE ALGORITHM, IN ONE PICTURE
// -----------------------------
// Take the shape, and sweep a horizontal line down the image one row at a time.
// For each row, find every place the shape's edges cross it:
//
//       y = 7.5   ─────┬───────────┬──────────  two crossings: inside between them
//                     ╱             ╲
//       y = 8.5   ───┬───┬───────┬───┬────────  four crossings: inside between
//                   ╱     ╲     ╱     ╲          the 1st-2nd and the 3rd-4th
//
// Between which pairs is "inside" depends on the FILL RULE, and getting that
// wrong is the subtle bug this file is most careful about — see `windingInside`.
//
// WHY NOT JUST TEST EACH PIXEL'S CENTRE
// --------------------------------------
// Because a stroke in these figures is about two pixels wide when a book prints
// it. Sampling centres turns a smooth curve into a staircase, and a learner
// reads the staircase as part of the letter's shape — they are, after all,
// looking at the picture precisely because they do not yet know what the letter
// looks like. So each pixel gets a COVERAGE: how much of its area the shape
// covers, from 0 to 1. That is what `SUBSAMPLES` is for.

import type { PixelContainer } from "@coding-adventures/pixel-container";

// ---------------------------------------------------------------------------
// The path vocabulary
// ---------------------------------------------------------------------------
//
// Deliberately the same shape both callers already produce: `truetype.ts`
// returns glyph contours whose curves are quadratic (that is what TrueType
// stores), and `strokes.ts` returns hand-authored pen paths. Cubic is here
// because PostScript-flavoured fonts and SVG both speak it and converting at the
// boundary would be a second place to get a curve wrong.

export interface Point {
  x: number;
  y: number;
}

export type PathCommand =
  | { kind: "move_to"; x: number; y: number }
  | { kind: "line_to"; x: number; y: number }
  | { kind: "quad_to"; cx: number; cy: number; x: number; y: number }
  | { kind: "cubic_to"; c1x: number; c1y: number; c2x: number; c2y: number; x: number; y: number }
  | { kind: "close" };

export type FillRule = "nonzero" | "evenodd";

export interface Paint {
  /** 0-255 each. Alpha multiplies the computed coverage rather than replacing it. */
  r: number;
  g: number;
  b: number;
  a?: number;
}

export interface FillOptions {
  rule?: FillRule;
}

export interface StrokeOptions {
  width?: number;
}

/**
 * Vertical subsamples per pixel row.
 *
 * Sixteen is not arbitrary. Coverage is quantised to 8 bits on the way out, so
 * the visible steps are 1/255 apart; sampling more finely than the output can
 * express buys nothing. Sixteen rows gives 17 distinguishable coverages per
 * pixel from the vertical direction alone, and the horizontal direction is
 * computed EXACTLY (as span overlap, not by sampling), so the product is far
 * finer than the output. Raising this makes rendering slower and the pictures
 * identical.
 */
const SUBSAMPLES = 16;

/** Curve flattening tolerance, in pixels. Half a pixel is invisible after AA. */
const FLATTEN_TOLERANCE = 0.5;

/**
 * The most edges one path may flatten to.
 *
 * A budget rather than a guess, and it exists because the paths this fills come
 * out of FONT FILES. `truetype.ts` caps its own work — MAX_CMAP_GROUPS,
 * MAX_COMPONENT_VISITS — but not its OUTPUT: a simple glyph may declare 65,536
 * points, and composite expansion may visit 10,000 components each returning a
 * full contour set. Every one of those curves then multiplies by up to 256 here.
 * A security review measured the result: ~20,000 curve commands in one glyph is
 * about 130 seconds of CPU and 450 MB of heap, from a file.
 *
 * 200,000 is far above any real letter — the whole Devanagari face flattens to a
 * small fraction of it — and far below the point where a font can hurt anyone.
 * Overrunning it truncates the path rather than throwing, because a build that
 * draws a slightly clipped glyph and says so is better than one that dies.
 */
const MAX_EDGES = 200_000;

// ---------------------------------------------------------------------------
// Flattening: curves become short straight lines
// ---------------------------------------------------------------------------
//
// The scanline sweep only knows how to intersect a straight edge. So curves are
// chopped into segments short enough that nobody can tell — which is a decision
// about how many, and the honest way to make it is from the curve's own
// flatness rather than a fixed count. A gently bending curve gets few segments;
// a tight hook in a Malayalam letter gets many. A fixed count would either
// waste work on the first or visibly polygonise the second.

function quadSegments(p0: Point, c: Point, p1: Point): number {
  // Distance from the control point to the chord bounds how far the curve can
  // stray from a straight line between its ends.
  const dx = p0.x - 2 * c.x + p1.x;
  const dy = p0.y - 2 * c.y + p1.y;
  const deviation = Math.sqrt(dx * dx + dy * dy);
  if (!Number.isFinite(deviation) || deviation <= 0) return 1;
  return Math.min(256, Math.max(1, Math.ceil(Math.sqrt(deviation / (4 * FLATTEN_TOLERANCE)) * 4)));
}

function cubicSegments(p0: Point, c1: Point, c2: Point, p1: Point): number {
  const d1 = Math.hypot(p0.x - 2 * c1.x + c2.x, p0.y - 2 * c1.y + c2.y);
  const d2 = Math.hypot(c1.x - 2 * c2.x + p1.x, c1.y - 2 * c2.y + p1.y);
  const deviation = Math.max(d1, d2);
  if (!Number.isFinite(deviation) || deviation <= 0) return 1;
  return Math.min(256, Math.max(1, Math.ceil(Math.sqrt(deviation / (4 * FLATTEN_TOLERANCE)) * 6)));
}

/**
 * Turn a path into closed polygons — one per subpath, each a list of points.
 *
 * Every subpath comes back CLOSED whether the author closed it or not, because
 * an unclosed subpath has no interior and "fill this open path" has to mean
 * something. Every renderer resolves it the same way: join the last point back
 * to the first. Doing it here rather than in the sweep means the sweep never has
 * to ask whether an edge is real.
 */
export function flattenPath(path: readonly PathCommand[]): Point[][] {
  const polygons: Point[][] = [];
  let current: Point[] = [];
  let cursor: Point = { x: 0, y: 0 };
  let start: Point = { x: 0, y: 0 };
  let emitted = 0;
  const spent = (): boolean => emitted >= MAX_EDGES;

  const finish = (): void => {
    if (current.length >= 2) {
      // Close it if the author did not. This is what the doc comment above
      // promises, and it has to happen HERE rather than in the sweep: an
      // unclosed rectangle has only one non-horizontal edge, the sweep finds
      // fewer than two crossings on every row, and the shape renders as
      // nothing at all rather than as something visibly wrong.
      const first = current[0]!;
      const last = current[current.length - 1]!;
      if (first.x !== last.x || first.y !== last.y) current.push(first);
      polygons.push(current);
    }
    current = [];
  };

  for (const command of path) {
    switch (command.kind) {
      case "move_to":
        finish();
        cursor = { x: command.x, y: command.y };
        start = cursor;
        current = [cursor];
        break;
      case "line_to":
        if (spent()) break;
        cursor = { x: command.x, y: command.y };
        current.push(cursor);
        emitted += 1;
        break;
      case "quad_to": {
        const c = { x: command.cx, y: command.cy };
        const end = { x: command.x, y: command.y };
        const n = Math.min(quadSegments(cursor, c, end), MAX_EDGES - emitted);
        for (let i = 1; i <= n; i += 1) {
          emitted += 1;
          const t = i / n;
          const u = 1 - t;
          current.push({
            x: u * u * cursor.x + 2 * u * t * c.x + t * t * end.x,
            y: u * u * cursor.y + 2 * u * t * c.y + t * t * end.y,
          });
        }
        cursor = end;
        break;
      }
      case "cubic_to": {
        const c1 = { x: command.c1x, y: command.c1y };
        const c2 = { x: command.c2x, y: command.c2y };
        const end = { x: command.x, y: command.y };
        const n = Math.min(cubicSegments(cursor, c1, c2, end), MAX_EDGES - emitted);
        for (let i = 1; i <= n; i += 1) {
          emitted += 1;
          const t = i / n;
          const u = 1 - t;
          current.push({
            x: u * u * u * cursor.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t * t * t * end.x,
            y: u * u * u * cursor.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t * t * t * end.y,
          });
        }
        cursor = end;
        break;
      }
      case "close":
        if (current.length > 0) current.push(start);
        finish();
        cursor = start;
        break;
    }
  }
  finish();
  return polygons;
}

// ---------------------------------------------------------------------------
// The fill rule, which is where the interesting bug lives
// ---------------------------------------------------------------------------
//
// A letter with a hole in it — the counter of an `o`, the bowl of अ — is TWO
// contours, and the hole is a hole only because they are wound in opposite
// directions. The rules disagree about exactly this:
//
//   NONZERO   count +1 for each edge crossed going down, -1 going up.
//             Inside where the running total is not zero.
//             Two contours wound the same way -> total 2 -> still inside ->
//             no hole. Wound oppositely -> total 0 -> hole.
//
//   EVENODD   count crossings. Inside where the count is odd.
//             Two contours -> 2 -> outside -> hole, REGARDLESS of direction.
//
// Even-odd sounds more forgiving and that is exactly why it is the wrong
// default. A glyph whose contours happen to be wound the same way loses its
// counter and fills solid — and it still looks like a letter, which is what
// makes it a bad bug: it is wrong in a way a reader will not report. TrueType
// specifies nonzero and its fonts are wound for it, so nonzero it is.

function windingInside(winding: number, rule: FillRule): boolean {
  return rule === "evenodd" ? (winding & 1) === 1 : winding !== 0;
}

interface Crossing {
  x: number;
  /** +1 downward, -1 upward. Ignored under even-odd. */
  direction: number;
}

interface Edge {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  /** +1 downward, -1 upward. */
  direction: number;
}

/**
 * Edges, bucketed by the pixel rows they touch.
 *
 * The obvious implementation asks every edge of every polygon whether it
 * crosses the current sample line, on all 16 sample lines of every row. That is
 * O(16 · H · E) — an edge confined to two scanlines is still re-tested down the
 * whole image — and a security review measured it: 20,000 edges over 4,096 rows
 * took 1.9 seconds, and doubling the canvas height doubled it, for a shape that
 * had not changed.
 *
 * Bucketing removes the H factor. Each edge is filed once under every pixel row
 * its y-range covers, and a row's sweep walks only its own bucket. The total
 * work becomes proportional to the sum of the edges' heights — which is what it
 * always should have been, because that is how much of the picture they are
 * actually in.
 */
interface EdgeIndex {
  buckets: Edge[][];
  /** First pixel row the index covers; bucket i is row `top + i`. */
  top: number;
}

function buildEdgeIndex(
  polygons: readonly Point[][],
  top: number,
  bottom: number,
): EdgeIndex {
  const rows = Math.max(0, bottom - top + 1);
  const buckets: Edge[][] = Array.from({ length: rows }, () => []);
  for (const polygon of polygons) {
    for (let i = 0; i + 1 < polygon.length; i += 1) {
      const a = polygon[i]!;
      const b = polygon[i + 1]!;
      if (!Number.isFinite(a.y) || !Number.isFinite(b.y)) continue;
      if (!Number.isFinite(a.x) || !Number.isFinite(b.x)) continue;
      // A horizontal edge crosses no scanline, so it contributes nothing and
      // filing it would only cost time.
      if (a.y === b.y) continue;
      const edge: Edge = { x0: a.x, y0: a.y, x1: b.x, y1: b.y, direction: b.y > a.y ? 1 : -1 };
      const from = Math.max(top, Math.floor(Math.min(a.y, b.y)));
      const to = Math.min(bottom, Math.ceil(Math.max(a.y, b.y)));
      for (let row = from; row <= to; row += 1) buckets[row - top]?.push(edge);
    }
  }
  return { buckets, top };
}

/** Where the edges filed under pixel row `row` cross the line y = `y`. */
function crossingsAt(index: EdgeIndex, row: number, y: number, out: Crossing[]): void {
  out.length = 0;
  const bucket = index.buckets[row - index.top];
  if (bucket === undefined) return;
  for (const edge of bucket) {
    // Half-open in y: a vertex exactly on the scanline counts for the edge
    // below it and not the one above. Without this a shape sampled exactly at
    // a vertex is counted twice and a fill leaks out sideways along that row.
    const lower = Math.min(edge.y0, edge.y1);
    const upper = Math.max(edge.y0, edge.y1);
    if (y < lower || y >= upper) continue;
    const t = (y - edge.y0) / (edge.y1 - edge.y0);
    const x = edge.x0 + t * (edge.x1 - edge.x0);
    if (!Number.isFinite(x)) continue;
    out.push({ x, direction: edge.direction });
  }
  out.sort((p, q) => p.x - q.x);
}

// ---------------------------------------------------------------------------
// Compositing
// ---------------------------------------------------------------------------

function blend(target: PixelContainer, x: number, y: number, paint: Paint, coverage: number): void {
  if (coverage <= 0) return;
  if (x < 0 || y < 0 || x >= target.width || y >= target.height) return;
  const alpha = Math.min(1, coverage) * ((paint.a ?? 255) / 255);
  if (alpha <= 0) return;
  const i = (y * target.width + x) * 4;
  const d = target.data;
  // Source-over, and the rounding is stated rather than left to the platform:
  // these images are byte-gated, so "whatever Math.round does here" has to be
  // the same everywhere or the gate measures the machine instead of the picture.
  d[i] = Math.round(paint.r * alpha + d[i]! * (1 - alpha));
  d[i + 1] = Math.round(paint.g * alpha + d[i + 1]! * (1 - alpha));
  d[i + 2] = Math.round(paint.b * alpha + d[i + 2]! * (1 - alpha));
  d[i + 3] = Math.round(255 * alpha + d[i + 3]! * (1 - alpha));
}

// ---------------------------------------------------------------------------
// fillPath
// ---------------------------------------------------------------------------

/**
 * Fill the interior of `path` into `target`.
 *
 * Coverage per pixel is accumulated over `SUBSAMPLES` horizontal sample lines
 * through the pixel's row. Along each line the inside spans are known EXACTLY —
 * they are intervals between crossings — so the horizontal contribution is the
 * overlap of that interval with the pixel, not a count of sample hits. Only the
 * vertical direction is sampled. That asymmetry is deliberate: it is where the
 * quality comes from at a cost of doing arithmetic instead of more sampling.
 */
export function fillPath(
  target: PixelContainer,
  path: readonly PathCommand[],
  paint: Paint,
  options: FillOptions = {},
): void {
  const rule = options.rule ?? "nonzero";
  const polygons = flattenPath(path);
  if (polygons.length === 0) return;

  let minY = Infinity;
  let maxY = -Infinity;
  for (const polygon of polygons) {
    for (const p of polygon) {
      if (!Number.isFinite(p.y)) continue;
      if (p.y < minY) minY = p.y;
      if (p.y > maxY) maxY = p.y;
    }
  }
  if (!Number.isFinite(minY) || !Number.isFinite(maxY)) return;

  const y0 = Math.max(0, Math.floor(minY));
  const y1 = Math.min(target.height - 1, Math.ceil(maxY));
  const coverage = new Float64Array(target.width);
  const crossings: Crossing[] = [];
  const index = buildEdgeIndex(polygons, y0, y1);

  for (let py = y0; py <= y1; py += 1) {
    coverage.fill(0);
    let touched = false;
    for (let s = 0; s < SUBSAMPLES; s += 1) {
      const y = py + (s + 0.5) / SUBSAMPLES;
      crossingsAt(index, py, y, crossings);
      if (crossings.length < 2) continue;
      let winding = 0;
      for (let i = 0; i + 1 < crossings.length; i += 1) {
        winding += rule === "evenodd" ? 1 : crossings[i]!.direction;
        if (!windingInside(winding, rule)) continue;
        const spanStart = crossings[i]!.x;
        const spanEnd = crossings[i + 1]!.x;
        if (spanEnd <= spanStart) continue;
        const from = Math.max(0, Math.floor(spanStart));
        const to = Math.min(target.width - 1, Math.ceil(spanEnd) - 1);
        for (let px = from; px <= to; px += 1) {
          // Exact overlap of [spanStart, spanEnd] with this pixel's column.
          const overlap = Math.min(spanEnd, px + 1) - Math.max(spanStart, px);
          if (overlap <= 0) continue;
          coverage[px] = coverage[px]! + overlap / SUBSAMPLES;
          touched = true;
        }
      }
    }
    if (!touched) continue;
    for (let px = 0; px < target.width; px += 1) {
      const c = coverage[px]!;
      if (c > 0) blend(target, px, py, paint, c);
    }
  }
}

// ---------------------------------------------------------------------------
// strokePath
// ---------------------------------------------------------------------------

/**
 * Draw a band of width `width` along `path`.
 *
 * Implemented by turning each flattened segment into its own quad and filling
 * them — not by offsetting the whole outline. Offsetting is the "proper"
 * approach and it is a genuinely hard problem: a curve tighter than half the
 * stroke width folds its own offset inside out, which shows up as a bright
 * notch on the inside of exactly the tight hooks these Indic letters are full
 * of. Per-segment quads plus a round join at every vertex cannot fold, and at
 * these widths the two are indistinguishable.
 *
 * Each quad is filled as its own path so overlaps do not double-darken: two
 * quads meeting at a joint would otherwise composite twice and leave a visible
 * bead at every vertex.
 */
export function strokePath(
  target: PixelContainer,
  path: readonly PathCommand[],
  paint: Paint,
  options: StrokeOptions = {},
): void {
  const width = options.width ?? 1;
  if (!(width > 0)) return;
  // Clamped to the canvas diagonal, which is arithmetic hygiene and NOT a
  // performance bound. Be precise about this, because the obvious reading is
  // wrong and was believed here for a while:
  //
  //   300 vertices on 256x256, width 4       14 ms
  //   300 vertices on 256x256, width 1e6    631 ms
  //   300 vertices on 256x256, width 362    622 ms   <- the clamp
  //
  // The clamp buys nine milliseconds of six hundred. The cost is not driven by
  // the width being ABSURD, it is driven by the width being COMPARABLE TO THE
  // CANVAS: past that point every join disc's bounding box covers the whole
  // image, so each of the 2V accumulate calls sweeps every pixel, and a width
  // of exactly the diagonal costs the same as a width of a million.
  //
  // So a caller that asks for a canvas-sized stroke over many vertices gets
  // canvas-sized work per vertex, and this line does not save them. It is kept
  // because bounding `half` to a finite, sane number is worth doing anyway.
  // Left as a known cost rather than papered over: stroke width here is
  // caller-supplied, and every real caller asks for two or three pixels.
  const half = Math.min(width, Math.hypot(target.width, target.height)) / 2;
  const polygons = flattenPath(path);

  // One accumulation buffer for the whole stroke, so overlapping segments and
  // joins take the MAXIMUM coverage rather than adding up.
  const mask = createMask(target.width, target.height);

  for (const polygon of polygons) {
    for (let i = 0; i + 1 < polygon.length; i += 1) {
      const a = polygon[i]!;
      const b = polygon[i + 1]!;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len = Math.hypot(dx, dy);
      if (!Number.isFinite(len) || len === 0) continue;
      const nx = (-dy / len) * half;
      const ny = (dx / len) * half;
      accumulate(mask, target.width, target.height, [
        { kind: "move_to", x: a.x + nx, y: a.y + ny },
        { kind: "line_to", x: b.x + nx, y: b.y + ny },
        { kind: "line_to", x: b.x - nx, y: b.y - ny },
        { kind: "line_to", x: a.x - nx, y: a.y - ny },
        { kind: "close" },
      ]);
    }
    // Round joins and caps: a disc at every vertex, including the ends. Without
    // them a bend shows a wedge-shaped notch on its outside, which at two pixels
    // wide reads as a break in the stroke.
    for (const p of polygon) accumulate(mask, target.width, target.height, discPath(p, half));
  }

  for (let y = 0; y < target.height; y += 1) {
    for (let x = 0; x < target.width; x += 1) {
      const c = mask[y * target.width + x]!;
      if (c > 0) blend(target, x, y, paint, c);
    }
  }
}

function createMask(width: number, height: number): Float64Array {
  return new Float64Array(width * height);
}

/** A circle as four quadratic arcs — close enough at these radii, and cheap. */
function discPath(centre: Point, radius: number): PathCommand[] {
  if (!(radius > 0)) return [];
  // (4/3)(sqrt(2) - 1). A quarter circle drawn as one cubic needs its control
  // points this far along the tangents; the max radial error is about 0.027%.
  // The first draft used 4/3, which is a different formula entirely and bulges
  // each quadrant so far that the "circle" enclosed MORE than its bounding box.
  const k = radius * 0.5522847498307936;
  const { x, y } = centre;
  return [
    { kind: "move_to", x, y: y - radius },
    { kind: "cubic_to", c1x: x + k, c1y: y - radius, c2x: x + radius, c2y: y - k, x: x + radius, y },
    { kind: "cubic_to", c1x: x + radius, c1y: y + k, c2x: x + k, c2y: y + radius, x, y: y + radius },
    { kind: "cubic_to", c1x: x - k, c1y: y + radius, c2x: x - radius, c2y: y + k, x: x - radius, y },
    { kind: "cubic_to", c1x: x - radius, c1y: y - k, c2x: x - k, c2y: y - radius, x, y: y - radius },
    { kind: "close" },
  ];
}

/** Fill one shape into the coverage mask, keeping the maximum at each pixel. */
function accumulate(
  mask: Float64Array,
  width: number,
  height: number,
  path: readonly PathCommand[],
): void {
  const polygons = flattenPath(path);
  if (polygons.length === 0) return;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const polygon of polygons) {
    for (const p of polygon) {
      if (!Number.isFinite(p.y)) continue;
      if (p.y < minY) minY = p.y;
      if (p.y > maxY) maxY = p.y;
    }
  }
  if (!Number.isFinite(minY) || !Number.isFinite(maxY)) return;

  const y0 = Math.max(0, Math.floor(minY));
  const y1 = Math.min(height - 1, Math.ceil(maxY));
  const row = new Float64Array(width);
  const crossings: Crossing[] = [];
  const index = buildEdgeIndex(polygons, y0, y1);

  for (let py = y0; py <= y1; py += 1) {
    row.fill(0);
    for (let s = 0; s < SUBSAMPLES; s += 1) {
      const y = py + (s + 0.5) / SUBSAMPLES;
      crossingsAt(index, py, y, crossings);
      if (crossings.length < 2) continue;
      let winding = 0;
      for (let i = 0; i + 1 < crossings.length; i += 1) {
        winding += crossings[i]!.direction;
        if (winding === 0) continue;
        const spanStart = crossings[i]!.x;
        const spanEnd = crossings[i + 1]!.x;
        if (spanEnd <= spanStart) continue;
        const from = Math.max(0, Math.floor(spanStart));
        const to = Math.min(width - 1, Math.ceil(spanEnd) - 1);
        for (let px = from; px <= to; px += 1) {
          const overlap = Math.min(spanEnd, px + 1) - Math.max(spanStart, px);
          if (overlap > 0) row[px] = row[px]! + overlap / SUBSAMPLES;
        }
      }
    }
    for (let px = 0; px < width; px += 1) {
      const c = Math.min(1, row[px]!);
      const at = py * width + px;
      if (c > mask[at]!) mask[at] = c;
    }
  }
}
