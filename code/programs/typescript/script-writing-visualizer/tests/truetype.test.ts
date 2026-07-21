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

/** Render contours to a boolean ink grid on a FIXED em box, so two glyphs are directly comparable. */
function raster(contours: Contour[], W = 100, H = 34): boolean[][] {
  const polys = flatten(contours);
  const X0 = -30, X1 = 1030, Y0 = -320, Y1 = 880;
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
  it("ண and ன are the same shape except for their final stroke", () => {
    const f = tamil();
    const na = raster(f.glyphFor("ண")!.contours);
    const alveolar = raster(f.glyphFor("ன")!.contours);

    let firstDiff = na[0].length;
    outer: for (let r = 0; r < na.length; r++) {
      for (let c = 0; c < na[r].length; c++) {
        if (na[r][c] !== alveolar[r][c]) {
          firstDiff = Math.min(firstDiff, c);
          break outer;
        }
      }
    }
    // Identical through the left half: same top bar, same loop, same arch.
    // If this ever drops sharply, the "only the last stroke differs" claim in
    // TA-W02/TA-W04 has stopped being true and the lessons need revisiting.
    expect(firstDiff).toBeGreaterThan(45);

    // ...and they DO differ — otherwise the claim is vacuous.
    expect(firstDiff).toBeLessThan(na[0].length);

    // Control: the metric must be capable of reporting "these are different
    // letters". Two unrelated glyphs diverge almost immediately, so a passing
    // assertion above is a real property of ண/ன and not a quirk of the measure.
    const unrelated = raster(f.glyphFor("ம")!.contours);
    let controlDiff = na[0].length;
    outerControl: for (let r = 0; r < na.length; r++) {
      for (let c = 0; c < na[r].length; c++) {
        if (na[r][c] !== unrelated[r][c]) {
          controlDiff = Math.min(controlDiff, c);
          break outerControl;
        }
      }
    }
    expect(controlDiff).toBeLessThan(20);
  });

  it("ன's final stroke is a straight vertical; ண's is not", () => {
    const f = tamil();
    const straightness = (ch: string) => {
      const g = raster(f.glyphFor(ch)!.contours);
      const cols = inkColumns(g);
      const right = cols.lastIndexOf(true);
      // Walk up the rightmost stroke: for a straight vertical, the leftmost
      // inked column of the final stroke is the same on every row it occupies.
      // Look only BELOW the top bar. The bar spans the whole letter, so it
      // touches the rightmost column too and would swamp the measurement.
      const inked = g.map((row) => row.some(Boolean));
      const top = inked.indexOf(true);
      const bottom = inked.lastIndexOf(true);
      const firstBodyRow = top + Math.ceil((bottom - top) * 0.3);
      const lefts: number[] = [];
      for (let r = firstBodyRow; r <= bottom; r++) {
        if (!g[r][right]) continue;
        let c = right;
        while (c > 0 && g[r][c - 1]) c--;
        lefts.push(c);
      }
      return new Set(lefts).size; // 1 => perfectly straight
    };
    expect(straightness("ன")).toBe(1);
    expect(straightness("ண")).toBeGreaterThan(1);
  });

  it("ற has two arches (three legs at the baseline), not one", () => {
    const f = tamil();
    const g = raster(f.glyphFor("ற")!.contours);
    // Sample a row inside the letter body, above the baseline.
    const bodyRow = g[Math.floor(g.length * 0.5)];
    expect(runsInRow(bodyRow)).toBe(3);

    // Control: the measure must actually discriminate. Across the letters this
    // track teaches it returns a RANGE of values, so "3" is a measurement of ற
    // rather than a constant the metric hands back for anything.
    const counts = ["வ", "ம", "ண", "ற", "ல"].map((ch) =>
      runsInRow(raster(f.glyphFor(ch)!.contours)[Math.floor(g.length * 0.5)]),
    );
    expect(new Set(counts).size).toBeGreaterThan(1);
  });
});
