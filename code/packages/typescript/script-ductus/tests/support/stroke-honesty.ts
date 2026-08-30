import { describe, expect, it } from "vitest";
import { boundsOf, type Contour } from "../../src/truetype";
import { SCRIPTS } from "../../src/scriptdata";
import {
  joinGaps,
  penPath,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";
import { parsedFont } from "./font-fixtures";

export const fontForDuctus = (letter: LetterDuctus) => {
  const script = SCRIPTS.find(
    (candidate) => candidate.script === letter.script,
  );
  if (!script) throw new Error(`no verified script/font owns ${letter.glyph}`);
  const letterClaim = [
    ...script.letters,
    ...(script.independentVowels ?? []),
    ...(script.finalConsonants ?? []),
  ].find(
    (entry) =>
      entry.glyph === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  const ligatureClaim = script.ligatures?.find(
    (entry) =>
      entry.displayGlyph === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  const markClaim = script.marks?.find(
    (entry) =>
      entry.mark === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  if (!letterClaim && !ligatureClaim && !markClaim)
    throw new Error(`${letter.script} does not verify ${letter.glyph}`);
  return parsedFont(script.font.split("/").pop()!);
};

// ---------------------------------------------------------------------------
// Flatten a glyph's contours to polygons and answer two questions about a
// point: is it ON the letter's ink (non-zero winding), and how FAR is it from
// a pen path. Both are what make an authored stroke checkable against the font.
// ---------------------------------------------------------------------------
function flatten(
  contours: Contour[],
  perCurve = 10,
): Array<Array<[number, number]>> {
  const polys: Array<Array<[number, number]>> = [];
  for (const pts of contours) {
    if (!pts.length) continue;
    const poly: Array<[number, number]> = [];
    let si = pts.findIndex((p) => p.on);
    let sx: number, sy: number;
    if (si === -1) {
      sx = (pts[pts.length - 1].x + pts[0].x) / 2;
      sy = (pts[pts.length - 1].y + pts[0].y) / 2;
      si = 0;
    } else {
      sx = pts[si].x;
      sy = pts[si].y;
      si += 1;
    }
    poly.push([sx, sy]);
    let cx = sx,
      cy = sy,
      ctrl: { x: number; y: number } | null = null;
    const quad = (qx: number, qy: number, x: number, y: number) => {
      for (let i = 1; i <= perCurve; i++) {
        const t = i / perCurve,
          mt = 1 - t;
        poly.push([
          mt * mt * cx + 2 * mt * t * qx + t * t * x,
          mt * mt * cy + 2 * mt * t * qy + t * t * y,
        ]);
      }
      cx = x;
      cy = y;
    };
    for (let k = 0; k < pts.length; k++) {
      const p = pts[(si + k) % pts.length];
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

export function makeInInk(contours: Contour[]) {
  const polys = flatten(contours);
  return (x: number, y: number): boolean => {
    let w = 0;
    for (const p of polys) {
      for (let i = 0; i + 1 < p.length; i++) {
        const [ax, ay] = p[i];
        const [bx, by] = p[i + 1];
        if (ay <= y !== by <= y) {
          const t = (y - ay) / (by - ay);
          if (ax + t * (bx - ax) > x) w += by > ay ? 1 : -1;
        }
      }
    }
    return w !== 0;
  };
}

/** Fraction of a polyline's sampled length that lies on the glyph's ink. */
export function fractionOnInk(
  path: Point[],
  inInk: (x: number, y: number) => boolean,
): number {
  let on = 0,
    total = 0;
  for (let i = 0; i + 1 < path.length; i++) {
    const a = path[i],
      b = path[i + 1];
    const n = Math.max(2, Math.round(Math.hypot(b.x - a.x, b.y - a.y) / 6));
    for (let s = 0; s <= n; s++) {
      const t = s / n;
      total++;
      if (inInk(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))) on++;
    }
  }
  return total === 0 ? 0 : on / total;
}

/** Distance from a point to the nearest vertex-sampled point of a pen path. */
export function distanceToPath(px: number, py: number, path: Point[]): number {
  let best = Infinity;
  for (let i = 0; i + 1 < path.length; i++) {
    const a = path[i],
      b = path[i + 1];
    const n = Math.max(2, Math.round(Math.hypot(b.x - a.x, b.y - a.y) / 10));
    for (let s = 0; s <= n; s++) {
      const t = s / n;
      const d = Math.hypot(
        px - (a.x + t * (b.x - a.x)),
        py - (a.y + t * (b.y - a.y)),
      );
      if (d < best) best = d;
    }
  }
  return best;
}

export function inkPoints(
  contours: Contour[],
  step = 16,
): Array<[number, number]> {
  const inInk = makeInInk(contours);
  const b = boundsOf(contours);
  const pts: Array<[number, number]> = [];
  for (let y = b.y0; y <= b.y1; y += step)
    for (let x = b.x0; x <= b.x1; x += step) if (inInk(x, y)) pts.push([x, y]);
  return pts;
}

export const registerStrokeHonestyTests = (
  letters: LetterDuctus[],
  minimumInkFitOverrides: Readonly<Record<string, number>> = {},
): void => {
  for (const letter of letters) {
    describe(`${letter.glyph}`, () => {
      const glyph = () => fontForDuctus(letter).glyphFor(letter.glyph)!;
      it("every stroke's pen path lies on the real letter", () => {
        const inInk = makeInInk(glyph().contours);
        for (let s = 0; s < letter.strokes.length; s++) {
          const frac = fractionOnInk(penPath(letter.strokes[s]), inInk);
          const minimumInkFit = minimumInkFitOverrides[letter.glyph] ?? 0.97;
          expect(frac, `stroke ${s} strays off the glyph`).toBeGreaterThan(
            minimumInkFit,
          );
        }
      });

      it("parts within a stroke connect — no gap, so no pen lift", () => {
        for (const stroke of letter.strokes) {
          for (const gap of joinGaps(stroke)) {
            expect(gap).toBeLessThan(2);
          }
        }
      });

      it("the strokes trace the WHOLE letter, not just part of it", () => {
        const pts = inkPoints(glyph().contours);
        const paths = letter.strokes.map((stroke) => penPath(stroke));
        const nearest = (x: number, y: number) =>
          Math.min(...paths.map((path) => distanceToPath(x, y, path)));
        const strayed = pts.filter(([x, y]) => nearest(x, y) > 100);
        expect(
          strayed.length / pts.length,
          "large parts of the letter are never traced",
        ).toBeLessThan(0.02);
      });
    });
  }
};
