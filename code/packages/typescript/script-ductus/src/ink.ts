// ---------------------------------------------------------------------------
// ink.ts — is a pen path on the letter, and does it reach all of it?
// ---------------------------------------------------------------------------
//
// Every authored stroke in this package is checked against the font rather
// than believed (see strokes.ts, "Why the path is authored, but not trusted").
// These are the measuring tools those checks use, and nothing else:
//
//   makeInInk(contours)      a point test: is (x, y) inside the glyph's ink?
//                            Non-zero winding, the rule the outline is filled
//                            with, on the contours flattened to polygons.
//   fractionOnInk(path, ..)  the share of a pen path, sampled every ~6 units,
//                            that lies on the ink. A stroke drawn in the wrong
//                            place scores low.
//   distanceToPath(x, y, ..) how far a point is from a pen path.
//   inkPoints(contours)      the ink sampled on a 16-unit grid, so "is every
//                            part of the letter traced?" becomes "is every
//                            sample near some path?".
//
// They lived in the test support code while only tests asked those questions.
// A Devanagari WORD needs them at build time too: whether one straight
// headline can be drawn across a word depends on the printed word's ink (see
// headline-word.ts), and a word that fails is refused, not drawn. So they
// moved here, unchanged, and the tests import them back.

import { boundsOf, type Contour } from "./truetype.ts";
import type { Point } from "./strokes.ts";

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
