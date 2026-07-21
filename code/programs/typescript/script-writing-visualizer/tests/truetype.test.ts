import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { parseFont, contoursToPath, boundsOf, type Contour } from "../src/truetype";

const FONT_DIR = resolve(__dirname, "../../../../learning/human-languages/_fonts");
const load = (name: string) => {
  const b = readFileSync(resolve(FONT_DIR, name));
  return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;
};
const tamil = () => parseFont(load("NotoSansTamil-Static.ttf"));

// ---------------------------------------------------------------------------
// A tiny rasteriser, used only by these tests.
//
// It exists so that shape assertions are made against WHAT THE GLYPH ACTUALLY
// LOOKS LIKE rather than against a description someone typed. Flatten the
// quadratics into line segments, then scan-convert with the non-zero winding
// rule (the same rule SVG's default fill-rule uses, so counters come out
// hollow exactly as they render).
// ---------------------------------------------------------------------------
function flatten(contours: Contour[], perCurve = 8): Array<Array<[number, number]>> {
  const polys: Array<Array<[number, number]>> = [];
  for (const pts of contours) {
    if (!pts.length) continue;
    // Re-walk the contour the same way contoursToPath does, so the polygon we
    // rasterise is the polygon the app draws.
    const poly: Array<[number, number]> = [];
    let startIndex = pts.findIndex((p) => p.on);
    let sx: number, sy: number;
    if (startIndex === -1) {
      sx = (pts[pts.length - 1].x + pts[0].x) / 2;
      sy = (pts[pts.length - 1].y + pts[0].y) / 2;
      startIndex = 0;
    } else {
      sx = pts[startIndex].x;
      sy = pts[startIndex].y;
      startIndex += 1;
    }
    poly.push([sx, sy]);
    let cx = sx;
    let cy = sy;
    let ctrl: { x: number; y: number } | null = null;
    const quad = (qx: number, qy: number, x: number, y: number) => {
      for (let i = 1; i <= perCurve; i++) {
        const t = i / perCurve;
        const mt = 1 - t;
        poly.push([mt * mt * cx + 2 * mt * t * qx + t * t * x, mt * mt * cy + 2 * mt * t * qy + t * t * y]);
      }
      cx = x;
      cy = y;
    };
    for (let k = 0; k < pts.length; k++) {
      const p = pts[(startIndex + k) % pts.length];
      if (p.on) {
        if (ctrl) {
          quad(ctrl.x, ctrl.y, p.x, p.y);
          ctrl = null;
        } else {
          poly.push([p.x, p.y]);
          cx = p.x;
          cy = p.y;
        }
      } else {
        if (ctrl) quad(ctrl.x, ctrl.y, (ctrl.x + p.x) / 2, (ctrl.y + p.y) / 2);
        ctrl = p;
      }
    }
    if (ctrl) quad(ctrl.x, ctrl.y, sx, sy);
    poly.push([sx, sy]);
    polys.push(poly);
  }
  return polys;
}

/**
 * Render contours to a boolean ink grid on a SHARED window, so two glyphs are
 * directly comparable.
 *
 * The window must be passed in and must enclose every glyph being compared.
 * An earlier version hard-coded x <= 1030 — but ண's outline runs to x=1631, so
 * it silently amputated 37% of the letter INCLUDING its final stroke, and a
 * shape description written from that picture claimed the letter ended in a
 * curve. Clipping does not look like an error; it looks like a letter.
 * `windowFor()` below derives the box from the glyphs themselves.
 */
interface Window { X0: number; X1: number; Y0: number; Y1: number }

function windowFor(...contourSets: Contour[][]): Window {
  let X0 = Infinity, X1 = -Infinity, Y0 = Infinity, Y1 = -Infinity;
  for (const cs of contourSets) {
    const b = boundsOf(cs);
    X0 = Math.min(X0, b.x0); X1 = Math.max(X1, b.x1);
    Y0 = Math.min(Y0, b.y0); Y1 = Math.max(Y1, b.y1);
  }
  const padX = (X1 - X0) * 0.04, padY = (Y1 - Y0) * 0.08;
  return { X0: X0 - padX, X1: X1 + padX, Y0: Y0 - padY, Y1: Y1 + padY };
}

function raster(contours: Contour[], win: Window, W = 110, H = 26): boolean[][] {
  const polys = flatten(contours);
  const { X0, X1, Y0, Y1 } = win;
  // Guard: the caller's window must actually contain this glyph. Without this
  // the whole harness silently measures the clip boundary instead of the letter.
  const b = boundsOf(contours);
  if (b.x0 < X0 - 1 || b.x1 > X1 + 1 || b.y0 < Y0 - 1 || b.y1 > Y1 + 1) {
    throw new Error(
      `raster window [${X0.toFixed(0)},${X1.toFixed(0)}]x[${Y0.toFixed(0)},${Y1.toFixed(0)}] ` +
        `clips a glyph with bounds [${b.x0},${b.x1}]x[${b.y0},${b.y1}]`,
    );
  }
  const grid: boolean[][] = [];
  for (let r = 0; r < H; r++) {
    const y = Y1 - ((r + 0.5) * (Y1 - Y0)) / H;
    const row: boolean[] = [];
    for (let c = 0; c < W; c++) {
      const x = X0 + ((c + 0.5) * (X1 - X0)) / W;
      let winding = 0;
      for (const p of polys) {
        for (let i = 0; i + 1 < p.length; i++) {
          const [ax, ay] = p[i];
          const [bx, by] = p[i + 1];
          if (ay <= y !== by <= y) {
            const t = (y - ay) / (by - ay);
            if (ax + t * (bx - ax) > x) winding += by > ay ? 1 : -1;
          }
        }
      }
      row.push(winding !== 0);
    }
    grid.push(row);
  }
  return grid;
}

const inkColumns = (g: boolean[][]) => g[0].map((_, c) => g.some((row) => row[c]));
/** Count runs of ink along a row — i.e. how many separate strokes it crosses. */
const runsInRow = (row: boolean[]) => row.reduce((n, v, i) => n + (v && !row[i - 1] ? 1 : 0), 0);

describe("truetype: reading the vendored fonts", () => {
  it("parses Noto Sans Tamil and finds a Unicode cmap", () => {
    const f = tamil();
    expect(f.unitsPerEm).toBe(1000);
    expect(f.numGlyphs).toBeGreaterThan(100);
    expect([4, 12]).toContain(f.cmapFormat);
    expect(f.mappedCharacters).toBeGreaterThan(100);
  });

  it("parses every vendored font without throwing", () => {
    const fonts = readdirSync(FONT_DIR).filter((f) => f.endsWith(".ttf"));
    expect(fonts.length).toBeGreaterThan(5);
    for (const name of fonts) {
      const f = parseFont(load(name));
      expect(f.unitsPerEm, name).toBeGreaterThan(0);
      expect(f.mappedCharacters, name).toBeGreaterThan(0);
    }
  });

  it("resolves each Tamil letter the writing track teaches to a non-empty outline", () => {
    const f = tamil();
    for (const ch of ["க", "ம", "ண", "ன", "ந", "ற", "வ", "அ", "இ", "ல"]) {
      const g = f.glyphFor(ch);
      expect(g, ch).toBeDefined();
      expect(g!.contours.length, ch).toBeGreaterThan(0);
      expect(g!.path, ch).toMatch(/^M/);
      expect(g!.path, ch).toMatch(/Z$/);
      // Only the four commands we claim to emit.
      expect(g!.path.replace(/[-\d.\s]/g, "").split("").every((c) => "MLQZ".includes(c)), ch).toBe(true);
    }
  });

  it("returns undefined for a character the font does not cover", () => {
    expect(tamil().glyphFor("漢")).toBeUndefined();
  });

  it("puts descenders below the baseline and keeps the body above it", () => {
    const f = tamil();
    // ற has a long descender; ம sits on the baseline.
    expect(boundsOf(f.glyphFor("ற")!.contours).y0).toBeLessThan(-50);
    expect(boundsOf(f.glyphFor("ம")!.contours).y0).toBeGreaterThan(-50);
  });
});

describe("hostile input", () => {
  // Build a minimal font whose cmap format 12 claims one group spanning the
  // entire 32-bit range. Without clamping, the reader would try to build a
  // four-billion-entry Map and hang. With clamping it returns promptly.
  function fontWithHugeCmapRange(): ArrayBuffer {
    const tables = ["cmap", "glyf", "head", "loca", "maxp"];
    const dirSize = 12 + tables.length * 16;
    const cmapOffset = dirSize;
    const cmapSize = 16 + 12;
    const headOffset = cmapOffset + cmapSize;
    const buf = new ArrayBuffer(headOffset + 64);
    const v = new DataView(buf);
    v.setUint32(0, 0x00010000);
    v.setUint16(4, tables.length);
    const place: Record<string, [number, number]> = {
      cmap: [cmapOffset, cmapSize],
      glyf: [headOffset + 60, 0],
      head: [headOffset, 54],
      loca: [headOffset + 56, 4],
      maxp: [headOffset + 54, 6],
    };
    tables.forEach((tag, i) => {
      const at = 12 + i * 16;
      for (let k = 0; k < 4; k++) v.setUint8(at + k, tag.charCodeAt(k));
      v.setUint32(at + 8, place[tag][0]);
      v.setUint32(at + 12, place[tag][1]);
    });
    // cmap: 1 subtable, platform 3 / encoding 10, format 12
    v.setUint16(cmapOffset + 2, 1);
    v.setUint16(cmapOffset + 4, 3);
    v.setUint16(cmapOffset + 6, 10);
    v.setUint32(cmapOffset + 8, 12); // offset to the subtable, from cmap start
    const sub = cmapOffset + 12;
    v.setUint16(sub, 12); // format
    v.setUint32(sub + 12, 1); // nGroups
    v.setUint32(sub + 16, 0); // startCharCode
    v.setUint32(sub + 20, 0xffffffff); // endCharCode — the hostile part
    v.setUint32(sub + 24, 1); // startGlyphID
    v.setUint16(headOffset + 18, 1000); // unitsPerEm
    v.setInt16(headOffset + 50, 0); // indexToLocFormat
    v.setUint16(headOffset + 54 + 4, 1); // maxp numGlyphs
    return buf;
  }

  it("clamps a cmap range that spans the whole 32-bit space", () => {
    const started = Date.now();
    const f = parseFont(fontWithHugeCmapRange());
    // Unclamped this would attempt ~4.3 billion Map inserts.
    expect(f.mappedCharacters).toBeLessThanOrEqual(200_000);
    expect(Date.now() - started).toBeLessThan(10_000);
  });

  it("does not degrade to blank outlines over many lookups", () => {
    // The component budget must be per-LOOKUP. An earlier version created it
    // once per Font and decremented it forever, so ordinary letters started
    // coming back with empty contours after a few thousand renders — silent
    // blank glyphs that read as a font bug. A browse session does exactly this.
    const f = tamil();
    const first = f.glyphFor("ண")!.path;
    for (let i = 0; i < 12_000; i++) {
      const g = f.glyphFor("ண")!;
      if (g.contours.length === 0 || g.path !== first) {
        throw new Error(`glyph degraded on lookup ${i}`);
      }
    }
    expect(f.glyphFor("ண")!.path).toBe(first);
  });

  it("keeps composite glyphs intact across repeated lookups too", () => {
    const dev = parseFont(load("NotoSansDevanagari-Static.ttf"));
    const first = dev.glyphFor("आ")!;
    expect(first.contours.length).toBeGreaterThan(0);
    for (let i = 0; i < 5_000; i++) {
      expect(dev.glyphFor("आ")!.contours.length).toBe(first.contours.length);
    }
  });

  it("rejects formats it cannot read rather than guessing", () => {
    const otto = new ArrayBuffer(12);
    new DataView(otto).setUint32(0, 0x4f54544f);
    expect(() => parseFont(otto)).toThrow(/OTTO|CFF/);
    const ttc = new ArrayBuffer(12);
    new DataView(ttc).setUint32(0, 0x74746366);
    expect(() => parseFont(ttc)).toThrow(/collection/i);
  });
});

describe("contoursToPath", () => {
  it("inserts the implied on-curve midpoint between consecutive off-curve points", () => {
    // Two off-curve points in a row: TrueType implies an on-curve point halfway
    // between them, so we must emit TWO quadratics, not one.
    const contour: Contour = [
      { x: 0, y: 0, on: true },
      { x: 10, y: 10, on: false },
      { x: 20, y: 10, on: false },
      { x: 30, y: 0, on: true },
    ];
    const d = contoursToPath([contour]);
    // Q to the implied midpoint, then Q on to (30,0). Closing back to the
    // start is a straight L because the last point walked is on-curve.
    expect((d.match(/Q/g) ?? []).length).toBe(2);
    expect(d).toContain("Q10 10 15 10"); // the implied midpoint of (10,10)-(20,10)
  });

  it("synthesises a start point when a contour has no on-curve point at all", () => {
    const d = contoursToPath([
      [
        { x: 0, y: 0, on: false },
        { x: 10, y: 0, on: false },
      ],
    ]);
    expect(d).toMatch(/^M5 0/); // midpoint of last and first
  });

  it("emits nothing for an empty contour set", () => {
    expect(contoursToPath([])).toBe("");
  });
});

// ---------------------------------------------------------------------------
// Shape regressions.
//
// These encode facts about the LETTERS, taken from the rasterised font, that
// the Tamil writing lessons state in prose. They exist because an earlier
// draft shipped "ண is ன with one extra arch" — which is false — and it
// survived a review in which the font was rendered and the claim reported
// confirmed. A description can look right next to a glyph while naming the
// wrong distinguishing feature. Only a mechanical comparison catches that.
// ---------------------------------------------------------------------------
describe("Tamil letter shapes the lessons make claims about", () => {
  // These pin, against the rasterised font, what TA-W02 / TA-W04 / tamil.json
  // say in prose. They exist because BOTH previous attempts at this claim were
  // wrong: first "ண is ன with one extra arch" was replaced by a false
  // "they differ only in the final stroke", the replacement having been written
  // from a raster window that clipped ண at 63% of its width. The lesson is that
  // the instrument needs checking as much as the memory does.

  /** Count separate ink runs along a row — how many strokes it crosses. */
  const runs = (row: boolean[]) => row.reduce((n, v, i) => n + (v && !row[i - 1] ? 1 : 0), 0);

  it("ண is ன with exactly one extra arch, and both end in a straight vertical", () => {
    const f = tamil();
    const retroflex = f.glyphFor("ண")!.contours;
    const alveolar = f.glyphFor("ன")!.contours;
    const win = windowFor(retroflex, alveolar);
    const a = raster(retroflex, win);
    const b = raster(alveolar, win);

    // Mid-body, each arch shows up as two legs. ண crosses exactly two more
    // strokes than ன: one extra arch = two extra legs.
    const midA = runs(a[Math.floor(a.length * 0.55)]);
    const midB = runs(b[Math.floor(b.length * 0.55)]);
    expect(midA - midB).toBe(2);

    // Both finish with a straight vertical. Measured as the SPREAD of the final
    // stroke's left edge down the rows it spans: a vertical holds its x
    // position, a curve wanders. Tolerance of one cell, because real typefaces
    // taper a stem slightly and the raster is coarse.
    const finalStrokeSpread = (g: boolean[][]) => {
      const inked = g.map((row) => row.some(Boolean));
      const top = inked.indexOf(true);
      const bottom = inked.lastIndexOf(true);
      const startRow = top + Math.ceil((bottom - top) * 0.35);
      // Anchor on the rightmost column inked WITHIN THE BODY ROWS. Anchoring on
      // the whole-glyph profile picks the top bar, which overhangs the final
      // vertical in both letters — so no body row has ink in that column, no
      // samples are collected, and Math.max([]) - Math.min([]) is -Infinity,
      // which satisfies any upper bound. The metric then silently measures
      // nothing while reporting agreement: the same failure as a clipped
      // window, one level down.
      let right = -1;
      for (let r = startRow; r <= bottom; r++)
        for (let c = g[r].length - 1; c > right; c--)
          if (g[r][c]) { right = c; break; }
      const lefts: number[] = [];
      for (let r = startRow; r <= bottom; r++) {
        if (right < 0 || !g[r][right]) continue;
        let c = right;
        while (c > 0 && g[r][c - 1]) c--;
        lefts.push(c);
      }
      // Report the sample count alongside the answer so callers can guard the
      // metric's INPUT, the same way raster() guards its window.
      return {
        samples: lefts.length,
        spread: lefts.length ? Math.max(...lefts) - Math.min(...lefts) : NaN,
      };
    };
    const sa = finalStrokeSpread(a);
    const sb = finalStrokeSpread(b);
    // The claim is only meaningful if the metric actually looked at the stroke.
    expect(sa.samples).toBeGreaterThan(8);
    expect(sb.samples).toBeGreaterThan(8);
    expect(sa.spread).toBeLessThanOrEqual(1);
    expect(sb.spread).toBeLessThanOrEqual(1);

    // Control: the measure must be able to report a non-vertical final stroke,
    // otherwise "<= 1" is satisfied by a metric that always returns 0. Across
    // the letters this track teaches it returns a range of values.
    // Built from letters NOT under test, so the control is independent of the
    // subjects. Deliberately wider than it needs to be: with a bare handful the
    // survivor count sits right on the guard, and a raster-resolution change
    // would flip it.
    const spreads = ["ம", "வ", "அ", "இ", "ல", "ற", "க", "ய", "ப", "ள", "எ"]
      .map((ch) => {
        const cs = f.glyphFor(ch)!.contours;
        return finalStrokeSpread(raster(cs, windowFor(cs)));
      })
      .filter((r) => r.samples > 8)
      .map((r) => r.spread);
    expect(spreads.length).toBeGreaterThan(5);
    expect(Math.max(...spreads)).toBeGreaterThan(1);
  });

  it("the two letters share their opening: identical until the extra arch", () => {
    const f = tamil();
    const retroflex = f.glyphFor("ண")!.contours;
    const alveolar = f.glyphFor("ன")!.contours;
    const win = windowFor(retroflex, alveolar);
    const a = raster(retroflex, win);
    const b = raster(alveolar, win);

    // Minimum differing column ACROSS ALL ROWS (an earlier version stopped at
    // the first differing cell in row-major order, so it only ever measured
    // the topmost differing row).
    let firstDiff = a[0].length;
    for (let r = 0; r < a.length; r++)
      for (let c = 0; c < a[r].length; c++)
        if (a[r][c] !== b[r][c]) { firstDiff = Math.min(firstDiff, c); break; }

    expect(firstDiff).toBeGreaterThan(20); // a shared opening really exists
    expect(firstDiff).toBeLessThan(a[0].length); // ...and they do differ

    // Control: unrelated letters diverge almost at once, so the number above
    // is a property of this pair rather than of the measure.
    const other = f.glyphFor("ம")!.contours;
    const win2 = windowFor(retroflex, other);
    const a2 = raster(retroflex, win2);
    const c2 = raster(other, win2);
    let controlDiff = a2[0].length;
    for (let r = 0; r < a2.length; r++)
      for (let c = 0; c < a2[r].length; c++)
        if (a2[r][c] !== c2[r][c]) { controlDiff = Math.min(controlDiff, c); break; }
    expect(controlDiff).toBeLessThan(firstDiff);
  });

  it("ற has two arches — three legs at mid-body — and a descender", () => {
    const f = tamil();
    const g = f.glyphFor("ற")!.contours;
    const win = windowFor(g);
    expect(runs(raster(g, win)[Math.floor(26 * 0.5)])).toBe(3);
    expect(boundsOf(g).y0).toBeLessThan(-200); // the long tail below the baseline
  });

  it("the raster window guard fires rather than silently clipping", () => {
    const f = tamil();
    const wide = f.glyphFor("ண")!.contours; // runs to x=1631
    const narrow = windowFor(f.glyphFor("ம")!.contours); // only to x=797
    expect(() => raster(wide, narrow)).toThrow(/clips a glyph/);
  });
});
