// The hard part of testing a rasterizer is that "looks right" is not a test.
//
// So nothing here compares a picture to a stored picture. Every assertion is a
// property that can be stated without looking: an area that is analytically
// known, a hole that must stay a hole, an edge pixel that must be strictly
// between the two colours it lies between, an output that must not depend on
// the machine. A golden-image test would pass on a rasterizer that is wrong in
// exactly the way the golden was captured.

import { describe, expect, it } from "vitest";
import { createPixelContainer, pixelAt, fillPixels } from "@coding-adventures/pixel-container";
import { fillPath, strokePath, flattenPath, type PathCommand, type Paint } from "../src/index.ts";

const BLACK: Paint = { r: 0, g: 0, b: 0, a: 255 };
const WHITE = { r: 255, g: 255, b: 255, a: 255 };

function blank(w: number, h: number) {
  const c = createPixelContainer(w, h);
  fillPixels(c, WHITE.r, WHITE.g, WHITE.b, WHITE.a);
  return c;
}

/** Total ink, in "whole pixels covered", so it can be compared to an area. */
function ink(c: ReturnType<typeof blank>): number {
  let total = 0;
  for (let y = 0; y < c.height; y += 1) {
    for (let x = 0; x < c.width; x += 1) {
      total += (255 - pixelAt(c, x, y)[0]!) / 255;
    }
  }
  return total;
}

function rect(x: number, y: number, w: number, h: number): PathCommand[] {
  return [
    { kind: "move_to", x, y },
    { kind: "line_to", x: x + w, y },
    { kind: "line_to", x: x + w, y: y + h },
    { kind: "line_to", x, y: y + h },
    { kind: "close" },
  ];
}

/** A square wound the other way round, for the winding tests. */
function reversedRect(x: number, y: number, w: number, h: number): PathCommand[] {
  return [
    { kind: "move_to", x, y },
    { kind: "line_to", x, y: y + h },
    { kind: "line_to", x: x + w, y: y + h },
    { kind: "line_to", x: x + w, y },
    { kind: "close" },
  ];
}

describe("area is what the geometry says it is", () => {
  it("fills a pixel-aligned square exactly, with no bleed", () => {
    const c = blank(20, 20);
    fillPath(c, rect(4, 4, 10, 10), BLACK);
    expect(ink(c)).toBeCloseTo(100, 6);
    // Fully inside is fully black; fully outside is untouched. An off-by-half
    // in the sampling shows up as a grey ring, so both edges are checked.
    expect(pixelAt(c, 4, 4)[0]).toBe(0);
    expect(pixelAt(c, 13, 13)[0]).toBe(0);
    expect(pixelAt(c, 3, 4)[0]).toBe(255);
    expect(pixelAt(c, 14, 13)[0]).toBe(255);
  });

  it("fills a half-pixel-offset square to the same total area", () => {
    // The area is unchanged by where the square sits; only its distribution
    // over pixels changes. A rasterizer that rounds to pixel bounds instead of
    // computing coverage fails this while passing the aligned case.
    const c = blank(20, 20);
    fillPath(c, rect(4.5, 4.5, 10, 10), BLACK);
    // Within a tenth of a pixel, not within a hundredth: the output is 8-bit,
    // so a half-covered edge pixel lands on 128/255 rather than exactly 0.5,
    // and forty of them accumulate about 0.07 of a pixel of rounding. That is
    // the quantisation, not the geometry.
    expect(ink(c)).toBeCloseTo(100, 0);
  });

  it("fills a triangle to half the area of its bounding box", () => {
    const c = blank(40, 40);
    fillPath(c, [
      { kind: "move_to", x: 5, y: 5 },
      { kind: "line_to", x: 25, y: 5 },
      { kind: "line_to", x: 5, y: 25 },
      { kind: "close" },
    ], BLACK);
    expect(ink(c)).toBeCloseTo(200, 0);
  });

  it("closes an unclosed subpath, because an open path has no interior", () => {
    const c = blank(20, 20);
    fillPath(c, [
      { kind: "move_to", x: 4, y: 4 },
      { kind: "line_to", x: 14, y: 4 },
      { kind: "line_to", x: 14, y: 14 },
      { kind: "line_to", x: 4, y: 14 },
    ], BLACK);
    expect(ink(c)).toBeCloseTo(100, 6);
  });
});

describe("the fill rule, which is where a letter loses its hole", () => {
  const outer = rect(4, 4, 12, 12);

  it("nonzero keeps a counter a hole when the inner contour is wound back", () => {
    const c = blank(24, 24);
    fillPath(c, [...outer, ...reversedRect(8, 8, 4, 4)], BLACK, { rule: "nonzero" });
    expect(pixelAt(c, 9, 9)[0]).toBe(255); // the hole is background
    expect(ink(c)).toBeCloseTo(144 - 16, 6);
  });

  it("nonzero FILLS a same-wound inner contour — which is the bug to know about", () => {
    // This is not a defect in the rasterizer, it is the rule. Stated as a test
    // because it is the exact failure a font with mis-wound contours produces,
    // and it looks like a letter, so nobody reports it.
    const c = blank(24, 24);
    fillPath(c, [...outer, ...rect(8, 8, 4, 4)], BLACK, { rule: "nonzero" });
    expect(pixelAt(c, 9, 9)[0]).toBe(0);
    expect(ink(c)).toBeCloseTo(144, 6);
  });

  it("even-odd punches the hole regardless of direction", () => {
    const c = blank(24, 24);
    fillPath(c, [...outer, ...rect(8, 8, 4, 4)], BLACK, { rule: "evenodd" });
    expect(pixelAt(c, 9, 9)[0]).toBe(255);
    expect(ink(c)).toBeCloseTo(144 - 16, 6);
  });

  it("the two rules genuinely disagree, so the default is a real choice", () => {
    const both: PathCommand[] = [...outer, ...rect(8, 8, 4, 4)];
    const a = blank(24, 24);
    const b = blank(24, 24);
    fillPath(a, both, BLACK, { rule: "nonzero" });
    fillPath(b, both, BLACK, { rule: "evenodd" });
    expect(Array.from(a.data)).not.toEqual(Array.from(b.data));
  });
});

describe("anti-aliasing is coverage, not luck", () => {
  it("puts a 45-degree edge's boundary pixels strictly between the two colours", () => {
    const c = blank(40, 40);
    fillPath(c, [
      { kind: "move_to", x: 0, y: 0 },
      { kind: "line_to", x: 40, y: 40 },
      { kind: "line_to", x: 0, y: 40 },
      { kind: "close" },
    ], BLACK);
    let partial = 0;
    for (let i = 5; i < 35; i += 1) {
      const v = pixelAt(c, i, i)[0]!;
      if (v > 0 && v < 255) partial += 1;
    }
    // Every pixel the diagonal passes through is half-covered, so every one of
    // them must be grey. A centre-sampling rasterizer makes them all 0 or 255.
    expect(partial).toBeGreaterThan(25);
  });

  it("gives a half-covered pixel about half the ink", () => {
    const c = blank(10, 10);
    fillPath(c, rect(4, 4, 0.5, 1), BLACK);
    const v = pixelAt(c, 4, 4)[0]!;
    expect(v).toBeGreaterThan(100);
    expect(v).toBeLessThan(155);
  });
});

describe("strokes", () => {
  it("draws a horizontal line of about the area width x length", () => {
    const c = blank(40, 20);
    strokePath(c, [
      { kind: "move_to", x: 5, y: 10 },
      { kind: "line_to", x: 35, y: 10 },
    ], BLACK, { width: 4 });
    // 30 long, 4 wide, plus two round caps of radius 2 -> 120 + pi*4.
    expect(ink(c)).toBeGreaterThan(115);
    expect(ink(c)).toBeLessThan(140);
  });

  it("does not drop out at hairline widths", () => {
    // A stroke thinner than a pixel must still mark every row it crosses; a
    // rasterizer that samples centres loses it entirely at some offsets.
    const c = blank(30, 30);
    strokePath(c, [
      { kind: "move_to", x: 5, y: 4.5 },
      { kind: "line_to", x: 25, y: 24.5 },
    ], BLACK, { width: 0.5 });
    let rowsTouched = 0;
    for (let y = 5; y < 24; y += 1) {
      let any = false;
      for (let x = 0; x < 30; x += 1) if (pixelAt(c, x, y)[0]! < 255) any = true;
      if (any) rowsTouched += 1;
    }
    expect(rowsTouched).toBe(19);
  });

  it("does not double-darken where segments meet", () => {
    // Two segments sharing a vertex composite twice if the stroke is drawn
    // segment-by-segment straight onto the target, leaving a bead at every
    // joint. With a coverage mask, the darkest pixel is exactly the paint.
    const c = blank(40, 40);
    strokePath(c, [
      { kind: "move_to", x: 5, y: 20 },
      { kind: "line_to", x: 20, y: 20 },
      { kind: "line_to", x: 35, y: 20 },
    ], { r: 128, g: 128, b: 128, a: 255 }, { width: 6 });
    let darkest = 255;
    for (let y = 0; y < 40; y += 1) {
      for (let x = 0; x < 40; x += 1) darkest = Math.min(darkest, pixelAt(c, x, y)[0]!);
    }
    expect(darkest).toBe(128);
  });

  it("ignores a non-positive width rather than throwing", () => {
    const c = blank(10, 10);
    strokePath(c, [{ kind: "move_to", x: 1, y: 1 }, { kind: "line_to", x: 8, y: 8 }], BLACK, {
      width: 0,
    });
    expect(ink(c)).toBe(0);
  });
});

describe("curves", () => {
  it("flattens a quadratic finely enough that the chord error is invisible", () => {
    const polygons = flattenPath([
      { kind: "move_to", x: 0, y: 0 },
      { kind: "quad_to", cx: 50, cy: 100, x: 100, y: 0 },
    ]);
    const points = polygons[0]!;
    expect(points.length).toBeGreaterThan(8);
    // The apex of this parabola is at y = 50; a too-coarse flattening cuts it.
    const apex = Math.max(...points.map((p) => p.y));
    expect(apex).toBeGreaterThan(49);
  });

  it("flattens a straight 'curve' to a single segment rather than 256", () => {
    // Segment count comes from the curve's own flatness, so a degenerate curve
    // costs nothing. A fixed count would do the same work for both.
    const polygons = flattenPath([
      { kind: "move_to", x: 0, y: 0 },
      { kind: "quad_to", cx: 50, cy: 0, x: 100, y: 0 },
    ]);
    // Three POINTS is one segment plus the closing edge back to the start,
    // which flattenPath adds to every subpath. The claim is about segments.
    expect(polygons[0]!.length).toBe(3);
  });

  it("fills a circle to about pi r squared", () => {
    const c = blank(60, 60);
    const r = 20;
    const k = r * 0.5522847498307936;
    fillPath(c, [
      { kind: "move_to", x: 30, y: 30 - r },
      { kind: "cubic_to", c1x: 30 + k, c1y: 30 - r, c2x: 30 + r, c2y: 30 - k, x: 30 + r, y: 30 },
      { kind: "cubic_to", c1x: 30 + r, c1y: 30 + k, c2x: 30 + k, c2y: 30 + r, x: 30, y: 30 + r },
      { kind: "cubic_to", c1x: 30 - k, c1y: 30 + r, c2x: 30 - r, c2y: 30 + k, x: 30 - r, y: 30 },
      { kind: "cubic_to", c1x: 30 - r, c1y: 30 - k, c2x: 30 - k, c2y: 30 - r, x: 30, y: 30 - r },
      { kind: "close" },
    ], BLACK);
    expect(ink(c)).toBeCloseTo(Math.PI * r * r, -1);
  });
});

describe("determinism, because the figures are byte-gated", () => {
  it("renders the same scene to identical bytes", () => {
    const path: PathCommand[] = [
      { kind: "move_to", x: 3.7, y: 2.1 },
      { kind: "quad_to", cx: 19.3, cy: 30.9, x: 36.1, y: 5.5 },
      { kind: "cubic_to", c1x: 30, c1y: 1, c2x: 10, c2y: 39, x: 3.7, y: 2.1 },
      { kind: "close" },
    ];
    const a = blank(40, 40);
    const b = blank(40, 40);
    fillPath(a, path, BLACK);
    fillPath(b, path, BLACK);
    expect(Array.from(a.data)).toEqual(Array.from(b.data));
  });

  it("gives a shape and its transpose the same area", () => {
    const a = blank(40, 40);
    const b = blank(40, 40);
    fillPath(a, rect(3, 7, 20, 9), BLACK);
    fillPath(b, rect(7, 3, 9, 20), BLACK);
    expect(ink(a)).toBeCloseTo(ink(b), 6);
  });
});

describe("hostile and degenerate input terminates", () => {
  it("survives NaN and Infinity coordinates without hanging or throwing", () => {
    const c = blank(20, 20);
    expect(() =>
      fillPath(c, [
        { kind: "move_to", x: NaN, y: 0 },
        { kind: "line_to", x: 10, y: Infinity },
        { kind: "line_to", x: 5, y: 5 },
        { kind: "close" },
      ], BLACK),
    ).not.toThrow();
  });

  it("caps flattening of an absurd curve rather than subdividing forever", () => {
    const polygons = flattenPath([
      { kind: "move_to", x: 0, y: 0 },
      { kind: "quad_to", cx: 1e9, cy: 1e9, x: 10, y: 0 },
    ]);
    // 256 segments is the cap, so 257 points, plus the closing point.
    expect(polygons[0]!.length).toBeLessThanOrEqual(258);
  });

  it("draws nothing for an empty path or a single point", () => {
    const c = blank(10, 10);
    fillPath(c, [], BLACK);
    fillPath(c, [{ kind: "move_to", x: 5, y: 5 }], BLACK);
    expect(ink(c)).toBe(0);
  });

  it("clips to the canvas instead of writing out of bounds", () => {
    const c = blank(10, 10);
    expect(() => fillPath(c, rect(-50, -50, 200, 200), BLACK)).not.toThrow();
    expect(ink(c)).toBeCloseTo(100, 6);
  });
});

describe("compositing", () => {
  it("respects paint alpha as a multiplier on coverage", () => {
    const c = blank(10, 10);
    fillPath(c, rect(2, 2, 4, 4), { r: 0, g: 0, b: 0, a: 128 });
    const v = pixelAt(c, 3, 3)[0]!;
    expect(v).toBeGreaterThan(120);
    expect(v).toBeLessThan(136);
  });

  it("paints over, so a second fill darkens the first", () => {
    const c = blank(10, 10);
    fillPath(c, rect(2, 2, 4, 4), { r: 0, g: 0, b: 0, a: 128 });
    fillPath(c, rect(2, 2, 4, 4), { r: 0, g: 0, b: 0, a: 128 });
    expect(pixelAt(c, 3, 3)[0]!).toBeLessThan(80);
  });
});

describe("bounded work on font-derived paths", () => {
  // These came out of a security review, which measured the unbounded versions
  // rather than reasoning about them: 20,000 edges over 4,096 rows took 1.9
  // seconds, doubling with canvas height for a shape that had not changed, and
  // 20,000 curve commands flattened to 5.1 million points and 438 MB of heap.
  // Paths here come from FONT FILES, so both are reachable from a file.

  it("caps flattening, so one glyph cannot expand without limit", () => {
    const path: PathCommand[] = [{ kind: "move_to", x: 0, y: 0 }];
    for (let i = 0; i < 20000; i += 1) {
      path.push({ kind: "quad_to", cx: 1e6, cy: 1e6, x: i % 2 ? 500 : 5, y: i % 900 });
    }
    const points = flattenPath(path)[0]!.length;
    expect(points).toBeLessThanOrEqual(200_002);
  });

  it("does not get slower just because the canvas is taller", () => {
    // The point of bucketing edges by row. An edge confined to two scanlines
    // used to be re-tested on every row of the image; now it is filed once.
    const edges = 8000;
    const band = (h: number): PathCommand[] => {
      const p: PathCommand[] = [{ kind: "move_to", x: 0, y: 0 }];
      for (let i = 0; i < edges; i += 1) {
        p.push({ kind: "line_to", x: i % 2 ? 60 : 2, y: (i / edges) * h });
      }
      p.push({ kind: "close" });
      return p;
    };
    const time = (h: number): number => {
      const c = blank(64, h);
      const started = Date.now();
      fillPath(c, band(h), BLACK);
      return Date.now() - started;
    };
    time(256); // warm
    const short = time(256);
    const tall = time(8192);
    // THIRTY-TWO times the rows, on the SAME shape. Without bucketing the cost
    // was linear in canvas height, so this would be ~32x; with it, the sweep
    // only visits rows the shape is actually in and the two are close.
    //
    // The ratio is deliberately far apart and the bound deliberately loose: a
    // shared CI runner is an order of magnitude slower than a laptop, and a
    // tight stopwatch bound measures the machine rather than the algorithm.
    // Between "about the same" and "thirty-two times" there is a lot of room
    // for a slow box without any room for the H factor to come back unnoticed.
    expect(tall).toBeLessThan(Math.max(400, short * 8));
  });

  it("treats a width past the canvas diagonal as the diagonal", () => {
    // Read the comment before trusting this test to catch anything.
    //
    // The first version timed the render and required under 4 seconds. It
    // passed locally at 700 ms and failed CI at 7,768 ms: a stopwatch measures
    // the machine. The second version asserted this byte-identity and was
    // WORSE — it passes with the clamp and without it, because a width of a
    // million and a width of the diagonal both paint the canvas solid. Deleting
    // the clamp did not fail it. A test that cannot fail is not a test.
    //
    // So this is kept for what it honestly is: a statement that the two widths
    // agree, which is worth pinning as behaviour. It does NOT verify the clamp,
    // and there is no cheap machine-independent test that does, because the
    // clamp saves nine milliseconds of six hundred (see the note in
    // `strokePath`). The cost that matters is a canvas-sized width over many
    // vertices, which is recorded as a known cost rather than mitigated.
    const path: PathCommand[] = [{ kind: "move_to", x: 10, y: 10 }];
    for (let i = 0; i < 40; i += 1) {
      path.push({ kind: "line_to", x: 10 + (i % 2) * 100, y: 10 + i * 2 });
    }
    const diagonal = Math.hypot(128, 128);
    const absurd = blank(128, 128);
    const clamped = blank(128, 128);
    strokePath(absurd, path, BLACK, { width: 1e6 });
    strokePath(clamped, path, BLACK, { width: diagonal });
    expect(Array.from(absurd.data)).toEqual(Array.from(clamped.data));
    // And it still draws: the clamp bounds the work, it does not skip the stroke.
    expect(ink(absurd)).toBeGreaterThan(0);
  });
});
