import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseFont, boundsOf, type Contour } from "../src/truetype";
import { SCRIPTS, verifiedLetterFont } from "../src/data";
import {
  DUCTUS,
  ductusKey,
  penPath,
  joinGaps,
  penLifts,
  penPathD,
  penTip,
  type LetterDuctus,
  type Point,
} from "../src/strokes";
import { ductusFor } from "../src/ductusview";

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FONT_DIR = resolve(TEST_DIR, "../../../../learning/human-languages/_fonts");
const load = (name: string) => {
  const b = readFileSync(resolve(FONT_DIR, name));
  return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;
};
const tamil = () => parseFont(load("NotoSansTamil-Static.ttf"));
const HEBREW_ALEF = DUCTUS[ductusKey("hebrew", "א")];
const HEBREW_BET = DUCTUS[ductusKey("hebrew", "ב")];
const HEBREW_GIMEL = DUCTUS[ductusKey("hebrew", "ג")];
const HEBREW_DALET = DUCTUS[ductusKey("hebrew", "ד")];
const HEBREW_HEI = DUCTUS[ductusKey("hebrew", "ה")];
const HEBREW_VAV = DUCTUS[ductusKey("hebrew", "ו")];
const ARABIC_ALEF = DUCTUS[ductusKey("arabic", "ا")];
const ARABIC_BAA = DUCTUS[ductusKey("arabic", "ب")];
const ARABIC_TAA = DUCTUS[ductusKey("arabic", "ت")];
const ARABIC_JEEM = DUCTUS[ductusKey("arabic", "ج")];
const ARABIC_HAA = DUCTUS[ductusKey("arabic", "ح")];
const ARABIC_KHAA = DUCTUS[ductusKey("arabic", "خ")];
const ARABIC_DAAL = DUCTUS[ductusKey("arabic", "د")];
const ARABIC_RAA = DUCTUS[ductusKey("arabic", "ر")];
const ARABIC_SEEN = DUCTUS[ductusKey("arabic", "س")];
const ARABIC_SHIIN = DUCTUS[ductusKey("arabic", "ش")];
const ARABIC_SAAD = DUCTUS[ductusKey("arabic", "ص")];
const ARABIC_DAAD = DUCTUS[ductusKey("arabic", "ض")];
const ARABIC_AYN = DUCTUS[ductusKey("arabic", "ع")];
const ARABIC_KAF = DUCTUS[ductusKey("arabic", "ك")];
const ARABIC_LAM = DUCTUS[ductusKey("arabic", "ل")];
const ARABIC_HEH = DUCTUS[ductusKey("arabic", "ه")];
const ARABIC_WAW = DUCTUS[ductusKey("arabic", "و")];
const ARABIC_YAA = DUCTUS[ductusKey("arabic", "ي")];
const URDU_ALEF = DUCTUS[ductusKey("urdu-nastaliq", "ا")];
const URDU_JIM = DUCTUS[ductusKey("urdu-nastaliq", "ج")];
const URDU_RE = DUCTUS[ductusKey("urdu-nastaliq", "ر")];
const URDU_SIN = DUCTUS[ductusKey("urdu-nastaliq", "س")];
const URDU_SHIN = DUCTUS[ductusKey("urdu-nastaliq", "ش")];
const URDU_KAF = DUCTUS[ductusKey("urdu-nastaliq", "ک")];
const URDU_LAM = DUCTUS[ductusKey("urdu-nastaliq", "ل")];
const URDU_MIM = DUCTUS[ductusKey("urdu-nastaliq", "م")];
const URDU_NUN = DUCTUS[ductusKey("urdu-nastaliq", "ن")];
const URDU_GHUNNA = DUCTUS[ductusKey("urdu-nastaliq", "ں")];
const URDU_HE = DUCTUS[ductusKey("urdu-nastaliq", "ہ")];
const URDU_YE = DUCTUS[ductusKey("urdu-nastaliq", "ی")];
const URDU_BARI_YE = DUCTUS[ductusKey("urdu-nastaliq", "ے")];

const fontForDuctus = (letter: LetterDuctus) => {
  const script = SCRIPTS.find((candidate) => candidate.script === letter.script);
  if (!script) throw new Error(`no verified script/font owns ${letter.glyph}`);
  const claim = script.letters.find(
    (entry) => entry.glyph === letter.glyph && entry.strokeOrderSource?.url === letter.source.url,
  );
  if (!claim) throw new Error(`${letter.script} does not verify ${letter.glyph}`);
  return parseFont(load(script.font.split("/").pop()!));
};

// ---------------------------------------------------------------------------
// Flatten a glyph's contours to polygons and answer two questions about a
// point: is it ON the letter's ink (non-zero winding), and how FAR is it from
// a pen path. Both are what make an authored stroke checkable against the font.
// ---------------------------------------------------------------------------
function flatten(contours: Contour[], perCurve = 10): Array<Array<[number, number]>> {
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
    let cx = sx, cy = sy, ctrl: { x: number; y: number } | null = null;
    const quad = (qx: number, qy: number, x: number, y: number) => {
      for (let i = 1; i <= perCurve; i++) {
        const t = i / perCurve, mt = 1 - t;
        poly.push([mt * mt * cx + 2 * mt * t * qx + t * t * x, mt * mt * cy + 2 * mt * t * qy + t * t * y]);
      }
      cx = x;
      cy = y;
    };
    for (let k = 0; k < pts.length; k++) {
      const p = pts[(si + k) % pts.length];
      if (p.on) {
        if (ctrl) { quad(ctrl.x, ctrl.y, p.x, p.y); ctrl = null; }
        else { poly.push([p.x, p.y]); cx = p.x; cy = p.y; }
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

function makeInInk(contours: Contour[]) {
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
function fractionOnInk(path: Point[], inInk: (x: number, y: number) => boolean): number {
  let on = 0, total = 0;
  for (let i = 0; i + 1 < path.length; i++) {
    const a = path[i], b = path[i + 1];
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
function distanceToPath(px: number, py: number, path: Point[]): number {
  let best = Infinity;
  for (let i = 0; i + 1 < path.length; i++) {
    const a = path[i], b = path[i + 1];
    const n = Math.max(2, Math.round(Math.hypot(b.x - a.x, b.y - a.y) / 10));
    for (let s = 0; s <= n; s++) {
      const t = s / n;
      const d = Math.hypot(px - (a.x + t * (b.x - a.x)), py - (a.y + t * (b.y - a.y)));
      if (d < best) best = d;
    }
  }
  return best;
}

function inkPoints(contours: Contour[], step = 16): Array<[number, number]> {
  const inInk = makeInInk(contours);
  const b = boundsOf(contours);
  const pts: Array<[number, number]> = [];
  for (let y = b.y0; y <= b.y1; y += step)
    for (let x = b.x0; x <= b.x1; x += step) if (inInk(x, y)) pts.push([x, y]);
  return pts;
}

describe("handwriting ductus", () => {
  const letters = Object.values(DUCTUS) as LetterDuctus[];

  it("has at least one authored letter", () => {
    expect(letters.length).toBeGreaterThan(0);
  });

  for (const letter of letters) {
    describe(`${letter.glyph}`, () => {
      const glyph = () => fontForDuctus(letter).glyphFor(letter.glyph)!;

      it("every stroke's pen path lies on the real letter", () => {
        const inInk = makeInInk(glyph().contours);
        for (let s = 0; s < letter.strokes.length; s++) {
          const frac = fractionOnInk(penPath(letter.strokes[s]), inInk);
          expect(frac, `stroke ${s} strays off the glyph`).toBeGreaterThan(0.97);
        }
      });

      it("parts within a stroke connect — no gap, so no pen lift", () => {
        for (const stroke of letter.strokes) {
          for (const gap of joinGaps(stroke)) {
            // font units; the glyph is ~566 tall, so 2 units is a hard join.
            expect(gap).toBeLessThan(2);
          }
        }
      });

      it("the strokes trace the WHOLE letter, not just part of it", () => {
        const pts = inkPoints(glyph().contours);
        const paths = letter.strokes.map((s) => penPath(s));
        const nearest = (x: number, y: number) => Math.min(...paths.map((p) => distanceToPath(x, y, p)));
        // Every inked point must be within ~a pen-stroke's width of the path.
      // 100 units: the reviewer measured every real ink point within 67 of the
      // authored path, so this keeps margin while still catching a dropped feature.
        const strayed = pts.filter(([x, y]) => nearest(x, y) > 100);
        expect(strayed.length / pts.length, "large parts of the letter are never traced").toBeLessThan(0.02);
      });
    });
  }

  it("ம is written without lifting the pen (one stroke)", () => {
    expect(penLifts(DUCTUS["ம"])).toBe(0);
    expect(DUCTUS["ம"].strokes).toHaveLength(1);
  });

  it("Hebrew א uses two crossed pen-down runs with one lift", () => {
    expect(HEBREW_ALEF.script).toBe("hebrew");
    expect(penLifts(HEBREW_ALEF)).toBe(1);
    expect(HEBREW_ALEF.strokes).toHaveLength(2);
    expect(HEBREW_ALEF.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2]);
    const main = penPath(HEBREW_ALEF.strokes[0]);
    const opposing = penPath(HEBREW_ALEF.strokes[1]);
    expect(main[0].x).toBeLessThan(main.at(-1)!.x);
    expect(main[0].y).toBeGreaterThan(main.at(-1)!.y);
    expect(opposing[0].x).toBeGreaterThan(opposing.at(-1)!.x);
    expect(opposing[0].y).toBeGreaterThan(opposing.at(-1)!.y);
  });

  it("Hebrew ב joins its top and right side before the lifted baseline", () => {
    expect(HEBREW_BET.script).toBe("hebrew");
    expect(penLifts(HEBREW_BET)).toBe(1);
    expect(HEBREW_BET.strokes).toHaveLength(2);
    expect(HEBREW_BET.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const top = HEBREW_BET.strokes[0].segments[0].path;
    const down = HEBREW_BET.strokes[0].segments[1].path;
    const base = HEBREW_BET.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
  });

  it("Hebrew ה joins its top and right side before the detached left leg", () => {
    expect(HEBREW_HEI.script).toBe("hebrew");
    expect(penLifts(HEBREW_HEI)).toBe(1);
    expect(HEBREW_HEI.strokes).toHaveLength(2);
    expect(HEBREW_HEI.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const top = HEBREW_HEI.strokes[0].segments[0].path;
    const down = HEBREW_HEI.strokes[0].segments[1].path;
    const detached = HEBREW_HEI.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(detached[0].y).toBeGreaterThan(detached.at(-1)!.y);
  });

  it("Hebrew ו joins its small head directly to its descending stem", () => {
    expect(HEBREW_VAV.script).toBe("hebrew");
    expect(penLifts(HEBREW_VAV)).toBe(0);
    expect(HEBREW_VAV.strokes).toHaveLength(1);
    expect(HEBREW_VAV.strokes.map((stroke) => stroke.segments.length)).toEqual([2]);
    const head = HEBREW_VAV.strokes[0].segments[0].path;
    const stem = HEBREW_VAV.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("அ lifts once before its separate right upright (two strokes)", () => {
    expect(penLifts(DUCTUS["அ"])).toBe(1);
    expect(DUCTUS["அ"].strokes).toHaveLength(2);
  });

  it("ஆ lifts once, then joins its upright and long-vowel loop", () => {
    expect(penLifts(DUCTUS["ஆ"])).toBe(1);
    expect(DUCTUS["ஆ"].strokes).toHaveLength(2);
    expect(DUCTUS["ஆ"].strokes[1].segments).toHaveLength(2);
  });

  it("இ lifts once before its joined outer climb and arch", () => {
    expect(penLifts(DUCTUS["இ"])).toBe(1);
    expect(DUCTUS["இ"].strokes).toHaveLength(2);
    expect(DUCTUS["இ"].strokes[0].segments).toHaveLength(5);
    expect(DUCTUS["இ"].strokes[1].segments).toHaveLength(2);
  });

  it("க lifts between its upper frame and two lower bowls", () => {
    expect(penLifts(DUCTUS["க"])).toBe(2);
    expect(DUCTUS["க"].strokes).toHaveLength(3);
    expect(DUCTUS["க"].strokes.map((stroke) => stroke.segments.length)).toEqual([3, 2, 1]);
  });

  it("வ joins all five movements without lifting the pen", () => {
    expect(penLifts(DUCTUS["வ"])).toBe(0);
    expect(DUCTUS["வ"].strokes).toHaveLength(1);
    expect(DUCTUS["வ"].strokes[0].segments).toHaveLength(5);
  });

  it("ல joins all four movements without lifting the pen", () => {
    expect(penLifts(DUCTUS["ல"])).toBe(0);
    expect(DUCTUS["ல"].strokes).toHaveLength(1);
    expect(DUCTUS["ல"].strokes[0].segments).toHaveLength(4);
  });

  it("ற lifts between its three pen-down runs", () => {
    expect(penLifts(DUCTUS["ற"])).toBe(2);
    expect(DUCTUS["ற"].strokes).toHaveLength(3);
    expect(DUCTUS["ற"].strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 2]);
  });

  it("ன joins its first five movements before the right upright", () => {
    expect(penLifts(DUCTUS["ன"])).toBe(1);
    expect(DUCTUS["ன"].strokes).toHaveLength(2);
    expect(DUCTUS["ன"].strokes.map((stroke) => stroke.segments.length)).toEqual([5, 1]);
  });

  it("ண joins its first six movements before the right upright", () => {
    expect(penLifts(DUCTUS["ண"])).toBe(1);
    expect(DUCTUS["ண"].strokes).toHaveLength(2);
    expect(DUCTUS["ண"].strokes.map((stroke) => stroke.segments.length)).toEqual([6, 1]);
  });

  it("ந lifts between its three pen-down runs", () => {
    expect(penLifts(DUCTUS["ந"])).toBe(2);
    expect(DUCTUS["ந"].strokes).toHaveLength(3);
    expect(DUCTUS["ந"].strokes.map((stroke) => stroke.segments.length)).toEqual([3, 2, 1]);
  });

  it("Persian ا is one downward pen-down run", () => {
    const alef = DUCTUS["ا"];
    expect(penLifts(alef)).toBe(0);
    expect(alef.strokes).toHaveLength(1);
    expect(alef.strokes[0].segments).toHaveLength(1);
    const path = penPath(alef.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Urdu independent ا is its own one-stroke downward ductus", () => {
    expect(URDU_ALEF.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_ALEF)).toBe(0);
    expect(URDU_ALEF.strokes).toHaveLength(1);
    expect(URDU_ALEF.strokes[0].segments).toHaveLength(1);
    const path = penPath(URDU_ALEF.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Urdu independent ج places its dot, then joins the pointed head to the bowl", () => {
    expect(URDU_JIM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_JIM)).toBe(1);
    expect(URDU_JIM.strokes).toHaveLength(2);
    expect(URDU_JIM.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2]);
    expect(URDU_JIM.strokes[0].segments[0].label).toBe("place the dot below");
    const head = URDU_JIM.strokes[1].segments[0].path;
    const bowl = URDU_JIM.strokes[1].segments[1].path;
    expect(head[0].x).toBeGreaterThan(Math.min(...head.map((point) => point.x)));
    expect(bowl[0]).toEqual(head.at(-1));
    expect(bowl[0].y).toBeGreaterThan(bowl.at(-1)!.y);
  });

  it("Urdu independent ر joins its downward line directly to the leftward curve", () => {
    expect(URDU_RE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_RE)).toBe(0);
    expect(URDU_RE.strokes).toHaveLength(1);
    expect(URDU_RE.strokes[0].segments).toHaveLength(2);
    const down = URDU_RE.strokes[0].segments[0].path;
    const curve = URDU_RE.strokes[0].segments[1].path;
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(curve[0]).toEqual(down.at(-1));
    expect(curve[0].x).toBeGreaterThan(curve.at(-1)!.x);
  });

  it("Urdu independent س joins its three close teeth directly to the final bowl", () => {
    expect(URDU_SIN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_SIN)).toBe(0);
    expect(URDU_SIN.strokes).toHaveLength(1);
    expect(URDU_SIN.strokes[0].segments).toHaveLength(2);
    const teeth = URDU_SIN.strokes[0].segments[0].path;
    const bowl = URDU_SIN.strokes[0].segments[1].path;
    expect(teeth[0].x).toBeGreaterThan(teeth.at(-1)!.x);
    expect(bowl[0]).toEqual(teeth.at(-1));
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Urdu independent ش writes its س body before three separately lifted dots", () => {
    expect(URDU_SHIN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_SHIN)).toBe(3);
    expect(URDU_SHIN.strokes).toHaveLength(4);
    expect(URDU_SHIN.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1]);
    const teeth = URDU_SHIN.strokes[0].segments[0].path;
    const bowl = URDU_SHIN.strokes[0].segments[1].path;
    expect(bowl[0]).toEqual(teeth.at(-1));
    const [lowerLeft, lowerRight, upper] = URDU_SHIN.strokes.slice(1).map(
      (stroke) => stroke.segments[0].path,
    );
    expect(lowerLeft[0].x).toBeLessThan(lowerRight[0].x);
    expect(upper[0].y).toBeGreaterThan(lowerLeft[0].y);
    expect(upper[0].y).toBeGreaterThan(lowerRight[0].y);
  });

  it("Urdu independent ک writes its main-line body before the separately lifted slash", () => {
    expect(URDU_KAF.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_KAF)).toBe(1);
    expect(URDU_KAF.strokes).toHaveLength(2);
    expect(URDU_KAF.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const stem = URDU_KAF.strokes[0].segments[0].path;
    const bowl = URDU_KAF.strokes[0].segments[1].path;
    const slash = URDU_KAF.strokes[1].segments[0].path;
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(bowl[0]).toEqual(stem.at(-1));
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(slash[0].x).toBeGreaterThan(slash.at(-1)!.x);
    expect(slash[0].y).toBeGreaterThan(slash.at(-1)!.y);
  });

  it("Urdu independent ل descends through its below-baseline bowl without lifting", () => {
    expect(URDU_LAM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_LAM)).toBe(0);
    expect(URDU_LAM.strokes).toHaveLength(1);
    expect(URDU_LAM.strokes[0].segments).toHaveLength(2);
    const upright = URDU_LAM.strokes[0].segments[0].path;
    const bowl = URDU_LAM.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(bowl[0]).toEqual(upright.at(-1));
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(Math.min(...bowl.map((point) => point.y)));
  });

  it("Urdu independent م joins its round head to a below-baseline tail", () => {
    expect(URDU_MIM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_MIM)).toBe(0);
    expect(URDU_MIM.strokes).toHaveLength(1);
    expect(URDU_MIM.strokes[0].segments).toHaveLength(2);
    const head = URDU_MIM.strokes[0].segments[0].path;
    const tail = URDU_MIM.strokes[0].segments[1].path;
    expect(tail[0]).toEqual(head.at(-1));
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(head[0].y);
    expect(Math.min(...tail.map((point) => point.y))).toBeLessThan(0);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
  });

  it("Urdu independent ن draws its below-baseline bowl before the lifted dot", () => {
    expect(URDU_NUN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_NUN)).toBe(1);
    expect(URDU_NUN.strokes).toHaveLength(2);
    expect(URDU_NUN.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = URDU_NUN.strokes[0].segments[0].path;
    const dot = URDU_NUN.strokes[1].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(0);
  });

  it("Urdu independent ں reuses ن's below-baseline bowl without a dot or lift", () => {
    expect(URDU_GHUNNA.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_GHUNNA)).toBe(0);
    expect(URDU_GHUNNA.strokes).toHaveLength(1);
    expect(URDU_GHUNNA.strokes[0].segments).toHaveLength(1);
    const bowl = URDU_GHUNNA.strokes[0].segments[0].path;
    expect(bowl).toEqual(URDU_NUN.strokes[0].segments[0].path);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
  });

  it("Urdu independent ہ closes its counterclockwise teardrop without lifting", () => {
    expect(URDU_HE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_HE)).toBe(0);
    expect(URDU_HE.strokes).toHaveLength(1);
    expect(URDU_HE.strokes[0].segments).toHaveLength(1);
    const loop = URDU_HE.strokes[0].segments[0].path;
    expect(loop[1].x).toBeLessThan(loop[0].x);
    expect(loop[1].y).toBeLessThan(loop[0].y);
    expect(Math.min(...loop.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...loop.slice(9).map((point) => point.x))).toBeGreaterThan(loop[0].x);
    expect(loop.at(-1)!.y).toBeGreaterThan(loop[0].y);
  });

  it("Urdu independent ی keeps its dotless S and bowl in one unbroken stroke", () => {
    expect(URDU_YE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_YE)).toBe(0);
    expect(URDU_YE.strokes).toHaveLength(1);
    expect(URDU_YE.strokes[0].segments).toHaveLength(2);
    const upper = URDU_YE.strokes[0].segments[0].path;
    const bowl = URDU_YE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(-200);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
  });

  it("Urdu independent ے folds its broad bowl back underneath without lifting", () => {
    expect(URDU_BARI_YE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_BARI_YE)).toBe(0);
    expect(URDU_BARI_YE.strokes).toHaveLength(1);
    expect(URDU_BARI_YE.strokes[0].segments).toHaveLength(3);
    const upper = URDU_BARI_YE.strokes[0].segments[0].path;
    const curl = URDU_BARI_YE.strokes[0].segments[1].path;
    const lower = URDU_BARI_YE.strokes[0].segments[2].path;
    expect(upper.at(-1)).toEqual(curl[0]);
    expect(curl.at(-1)).toEqual(lower[0]);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
    expect(Math.min(...curl.map((point) => point.x))).toBeLessThan(upper.at(-1)!.x);
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x);
  });

  it("Arabic independent ا descends in one unbroken stroke", () => {
    expect(penLifts(ARABIC_ALEF)).toBe(0);
    expect(ARABIC_ALEF.strokes).toHaveLength(1);
    expect(ARABIC_ALEF.strokes[0].segments).toHaveLength(1);
    const path = penPath(ARABIC_ALEF.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Arabic independent ب sweeps right-to-left, then lifts once for the dot", () => {
    expect(penLifts(ARABIC_BAA)).toBe(1);
    expect(ARABIC_BAA.strokes).toHaveLength(2);
    expect(ARABIC_BAA.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = penPath(ARABIC_BAA.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Arabic independent ت uses the shared bowl, then two separately lifted dots", () => {
    expect(penLifts(ARABIC_TAA)).toBe(2);
    expect(ARABIC_TAA.strokes).toHaveLength(3);
    expect(ARABIC_TAA.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1]);
    const bowl = penPath(ARABIC_TAA.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(ARABIC_TAA.strokes[1].segments[0].path[0].x).toBeLessThan(
      ARABIC_TAA.strokes[2].segments[0].path[0].x,
    );
  });

  it("Arabic independent ج draws its body first, then lifts once for the dot", () => {
    expect(penLifts(ARABIC_JEEM)).toBe(1);
    expect(ARABIC_JEEM.strokes).toHaveLength(2);
    expect(ARABIC_JEEM.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const head = ARABIC_JEEM.strokes[0].segments[0].path;
    const bowl = ARABIC_JEEM.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(
      Math.min(...head.map((point) => point.y)),
    );
  });

  it("Arabic independent ح draws a short stem, then lifts once for its dotless bowl", () => {
    expect(penLifts(ARABIC_HAA)).toBe(1);
    expect(ARABIC_HAA.strokes).toHaveLength(2);
    expect(ARABIC_HAA.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2]);
    const stem = ARABIC_HAA.strokes[0].segments[0].path;
    const head = ARABIC_HAA.strokes[1].segments[0].path;
    const bowl = ARABIC_HAA.strokes[1].segments[1].path;
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(head[0]).toEqual(stem[0]);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(
      Math.min(...head.map((point) => point.y)),
    );
  });

  it("Arabic independent خ draws its body first, then lifts once for the upper dot", () => {
    expect(penLifts(ARABIC_KHAA)).toBe(1);
    expect(ARABIC_KHAA.strokes).toHaveLength(2);
    expect(ARABIC_KHAA.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const head = ARABIC_KHAA.strokes[0].segments[0].path;
    const bowl = ARABIC_KHAA.strokes[0].segments[1].path;
    const dot = ARABIC_KHAA.strokes[1].segments[0].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...head.map((point) => point.y)),
    );
  });

  it("Arabic independent د descends and turns left without lifting", () => {
    expect(penLifts(ARABIC_DAAL)).toBe(0);
    expect(ARABIC_DAAL.strokes).toHaveLength(1);
    expect(ARABIC_DAAL.strokes[0].segments).toHaveLength(2);
    const shoulder = ARABIC_DAAL.strokes[0].segments[0].path;
    const baseline = ARABIC_DAAL.strokes[0].segments[1].path;
    expect(shoulder[0].y).toBeGreaterThan(shoulder.at(-1)!.y);
    expect(shoulder[0].x).toBeLessThan(shoulder.at(-1)!.x);
    expect(shoulder.at(-1)).toEqual(baseline[0]);
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
  });

  it("Arabic independent ر descends and sweeps left without lifting", () => {
    expect(penLifts(ARABIC_RAA)).toBe(0);
    expect(ARABIC_RAA.strokes).toHaveLength(1);
    expect(ARABIC_RAA.strokes[0].segments).toHaveLength(2);
    const descent = ARABIC_RAA.strokes[0].segments[0].path;
    const curve = ARABIC_RAA.strokes[0].segments[1].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(curve[0]);
    expect(curve[0].x).toBeGreaterThan(curve.at(-1)!.x);
  });

  it("Arabic independent س joins its three close teeth directly to the final bowl", () => {
    expect(ARABIC_SEEN.script).toBe("arabic");
    expect(penLifts(ARABIC_SEEN)).toBe(0);
    expect(ARABIC_SEEN.strokes).toHaveLength(1);
    expect(ARABIC_SEEN.strokes[0].segments).toHaveLength(2);
    const teeth = ARABIC_SEEN.strokes[0].segments[0].path;
    const bowl = ARABIC_SEEN.strokes[0].segments[1].path;
    expect(teeth[0].x).toBeGreaterThan(teeth.at(-1)!.x);
    expect(teeth.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Arabic independent ش writes its body before three separately lifted dots", () => {
    expect(ARABIC_SHIIN.script).toBe("arabic");
    expect(penLifts(ARABIC_SHIIN)).toBe(3);
    expect(ARABIC_SHIIN.strokes).toHaveLength(4);
    expect(ARABIC_SHIIN.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1]);
    const teeth = ARABIC_SHIIN.strokes[0].segments[0].path;
    const bowl = ARABIC_SHIIN.strokes[0].segments[1].path;
    expect(teeth.at(-1)).toEqual(bowl[0]);
    const [lowerLeft, lowerRight, upper] = ARABIC_SHIIN.strokes.slice(1).map(
      (stroke) => stroke.segments[0].path,
    );
    expect(lowerLeft[0].x).toBeLessThan(lowerRight[0].x);
    expect(upper[0].y).toBeGreaterThan(lowerLeft[0].y);
    expect(upper[0].y).toBeGreaterThan(lowerRight[0].y);
  });

  it("Arabic independent ص lifts once between its closed oval and trailing bowl", () => {
    expect(ARABIC_SAAD.script).toBe("arabic");
    expect(penLifts(ARABIC_SAAD)).toBe(1);
    expect(ARABIC_SAAD.strokes).toHaveLength(2);
    expect(ARABIC_SAAD.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const oval = ARABIC_SAAD.strokes[0].segments[0].path;
    const shoulder = ARABIC_SAAD.strokes[0].segments[1].path;
    expect(oval[0]).toEqual(oval.at(-1));
    expect(oval.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)!.y).toBeGreaterThan(shoulder[0].y);
    const bowl = ARABIC_SAAD.strokes[1].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
  });

  it("Arabic independent ض repeats the ص body before a second lift places its dot", () => {
    expect(ARABIC_DAAD.script).toBe("arabic");
    expect(penLifts(ARABIC_DAAD)).toBe(2);
    expect(ARABIC_DAAD.strokes).toHaveLength(3);
    expect(ARABIC_DAAD.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    expect(ARABIC_DAAD.strokes.slice(0, 2)).toEqual(ARABIC_SAAD.strokes);
    const dot = ARABIC_DAAD.strokes[2].segments[0].path;
    const bodyTop = Math.max(
      ...ARABIC_DAAD.strokes.slice(0, 2).flatMap((stroke) =>
        stroke.segments.flatMap((segment) => segment.path.map((point) => point.y)),
      ),
    );
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(bodyTop);
    expect(dot[0]).toEqual(dot.at(-1));
  });

  it("Arabic independent ع joins its open head directly to the lower bowl", () => {
    expect(ARABIC_AYN.script).toBe("arabic");
    expect(penLifts(ARABIC_AYN)).toBe(0);
    expect(ARABIC_AYN.strokes).toHaveLength(1);
    expect(ARABIC_AYN.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_AYN.strokes[0].segments[0].path;
    const bowl = ARABIC_AYN.strokes[0].segments[1].path;
    expect(head[0].x).toBeGreaterThan(Math.min(...head.map((point) => point.x)));
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
  });

  it("Arabic independent ك turns along its base before lifting for the inner arm", () => {
    expect(ARABIC_KAF.script).toBe("arabic");
    expect(penLifts(ARABIC_KAF)).toBe(1);
    expect(ARABIC_KAF.strokes).toHaveLength(2);
    expect(ARABIC_KAF.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const upright = ARABIC_KAF.strokes[0].segments[0].path;
    const base = ARABIC_KAF.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(base[0]);
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    const inner = ARABIC_KAF.strokes[1].segments[0].path;
    expect(inner[0].x).toBeGreaterThan(inner.at(-1)!.x);
    expect(inner[0].y).toBeGreaterThan(inner.at(-1)!.y);
  });

  it("Arabic independent ل descends through its leftward bowl without lifting", () => {
    expect(ARABIC_LAM.script).toBe("arabic");
    expect(penLifts(ARABIC_LAM)).toBe(0);
    expect(ARABIC_LAM.strokes).toHaveLength(1);
    expect(ARABIC_LAM.strokes[0].segments).toHaveLength(2);
    const upright = ARABIC_LAM.strokes[0].segments[0].path;
    const bowl = ARABIC_LAM.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(Math.min(...bowl.map((point) => point.y)));
  });

  it("Arabic independent ه closes both counters before its leftward baseline sweep", () => {
    expect(ARABIC_HEH.script).toBe("arabic");
    expect(penLifts(ARABIC_HEH)).toBe(0);
    expect(ARABIC_HEH.strokes).toHaveLength(1);
    expect(ARABIC_HEH.strokes[0].segments).toHaveLength(3);
    const lower = ARABIC_HEH.strokes[0].segments[0].path;
    const upperRight = ARABIC_HEH.strokes[0].segments[1].path;
    const baseline = ARABIC_HEH.strokes[0].segments[2].path;
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(lower.at(-1)).toEqual(upperRight[0]);
    expect(Math.max(...upperRight.map((point) => point.x))).toBeGreaterThan(upperRight[0].x);
    expect(upperRight.at(-1)).toEqual(baseline[0]);
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
  });

  it("Arabic independent و closes its head before continuing through the leftward tail", () => {
    expect(ARABIC_WAW.script).toBe("arabic");
    expect(penLifts(ARABIC_WAW)).toBe(0);
    expect(ARABIC_WAW.strokes).toHaveLength(1);
    expect(ARABIC_WAW.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_WAW.strokes[0].segments[0].path;
    const tail = ARABIC_WAW.strokes[0].segments[1].path;
    expect(head[0]).toEqual(head.at(-1));
    expect(Math.min(...head.map((point) => point.x))).toBeLessThan(head[0].x);
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(head[0].y);
    expect(head.at(-1)).toEqual(tail[0]);
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("Arabic independent ي completes its bowl before the lower-left and lower-right dots", () => {
    expect(ARABIC_YAA.script).toBe("arabic");
    expect(penLifts(ARABIC_YAA)).toBe(2);
    expect(ARABIC_YAA.strokes).toHaveLength(3);
    expect(ARABIC_YAA.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    const descent = ARABIC_YAA.strokes[0].segments[0].path;
    const bowl = ARABIC_YAA.strokes[0].segments[1].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    const bodyFloor = Math.min(
      ...ARABIC_YAA.strokes[0].segments.flatMap((segment) =>
        segment.path.map((point) => point.y),
      ),
    );
    const leftDot = ARABIC_YAA.strokes[1].segments[0].path;
    const rightDot = ARABIC_YAA.strokes[2].segments[0].path;
    expect(Math.max(...leftDot.map((point) => point.y))).toBeLessThan(bodyFloor);
    expect(Math.max(...rightDot.map((point) => point.y))).toBeLessThan(bodyFloor);
    expect(Math.max(...leftDot.map((point) => point.x))).toBeLessThan(
      Math.min(...rightDot.map((point) => point.x)),
    );
  });

  it("Persian ب sweeps right-to-left, then lifts once for the dot", () => {
    const beh = DUCTUS["ب"];
    expect(penLifts(beh)).toBe(1);
    expect(beh.strokes).toHaveLength(2);
    expect(beh.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = penPath(beh.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Persian ت sweeps right-to-left, then lifts for each dot", () => {
    const teh = DUCTUS["ت"];
    expect(penLifts(teh)).toBe(2);
    expect(teh.strokes).toHaveLength(3);
    expect(teh.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1]);
    const bowl = penPath(teh.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(teh.strokes[1].segments[0].path[0].x).toBeLessThan(
      teh.strokes[2].segments[0].path[0].x,
    );
  });

  it("Persian س joins its three teeth directly to the final bowl", () => {
    const sin = DUCTUS["س"];
    expect(penLifts(sin)).toBe(0);
    expect(sin.strokes).toHaveLength(1);
    expect(sin.strokes[0].segments).toHaveLength(2);
    const path = penPath(sin.strokes[0]);
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  it("Persian ل joins its descending upright directly to the base curve", () => {
    const lam = DUCTUS["ل"];
    expect(penLifts(lam)).toBe(0);
    expect(lam.strokes).toHaveLength(1);
    expect(lam.strokes[0].segments).toHaveLength(2);
    const path = penPath(lam.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  it("Persian م joins its round head directly to the descending tail", () => {
    const mim = DUCTUS["م"];
    expect(penLifts(mim)).toBe(0);
    expect(mim.strokes).toHaveLength(1);
    expect(mim.strokes[0].segments).toHaveLength(2);
    const head = mim.strokes[0].segments[0].path;
    const tail = mim.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("Persian ن sweeps its bowl right-to-left, then lifts once for the dot", () => {
    const nun = DUCTUS["ن"];
    expect(penLifts(nun)).toBe(1);
    expect(nun.strokes).toHaveLength(2);
    expect(nun.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = penPath(nun.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Persian و joins its small head loop directly to the leftward tail", () => {
    const waw = DUCTUS["و"];
    expect(penLifts(waw)).toBe(0);
    expect(waw.strokes).toHaveLength(1);
    expect(waw.strokes[0].segments).toHaveLength(2);
    const head = waw.strokes[0].segments[0].path;
    const tail = waw.strokes[0].segments[1].path;
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(
      Math.max(...tail.map((point) => point.y)),
    );
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
  });

  it("Persian ه keeps its isolated looping body in one pen-down run", () => {
    const heh = DUCTUS["ه"];
    expect(penLifts(heh)).toBe(0);
    expect(heh.strokes).toHaveLength(1);
    expect(heh.strokes[0].segments).toHaveLength(1);
    const path = penPath(heh.strokes[0]);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(
      Math.min(...path.map((point) => point.y)),
    );
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  // The PROVENANCE GATE. A stroke's SHAPE is checked against the font above;
  // its ORDER cannot be — so it must trace to a cited source, or it does not
  // ship. This is the counterpart, for hand-authored order, of "facts enter
  // only through a source". Where no source exists, the letter is simply not
  // authored rather than invented.
  it("every letter cites a real source for its stroke order", () => {
    for (const letter of letters) {
      expect(letter.source, `${letter.glyph} has no source`).toBeDefined();
      expect(letter.source.citation.length, `${letter.glyph} citation is empty`).toBeGreaterThan(10);
      expect(letter.source.url, `${letter.glyph} source url is not a real link`).toMatch(/^https?:\/\/\S+$/);
    }
  });

  it("every verified prose claim has the same font-checked ductus and source", () => {
    const verified = SCRIPTS.flatMap((script) =>
      script.letters
        .filter((letter) => letter.penLifts !== undefined || letter.strokeOrderSource !== undefined)
        .map((letter) => ({ script: script.script, letter })),
    );
    expect(verified).toHaveLength(letters.length);
    for (const { script, letter } of verified) {
      const ductus = ductusFor(letter.glyph, script);
      expect(ductus, `${script} ${letter.glyph} claims verification without a ductus`).toBeDefined();
      if (!ductus) throw new Error(`${script} ${letter.glyph} has no ductus`);
      expect(letter.penLifts).toBe(penLifts(ductus));
      expect(letter.strokeOrderSource).toEqual(ductus.source);
    }
    for (const ductus of letters) {
      expect(
        verified.some(({ script, letter }) => script === ductus.script && letter.glyph === ductus.glyph),
        `${ductus.glyph} has a ductus but no verified prose claim`,
      ).toBe(true);
    }
  });

  it("routes each verified ductus to the owning script font", () => {
    expect(verifiedLetterFont("א", HEBREW_ALEF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ב", HEBREW_BET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ג", HEBREW_GIMEL.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ד", HEBREW_DALET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ה", HEBREW_HEI.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ו", HEBREW_VAV.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ம", DUCTUS["ம"].source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
    expect(verifiedLetterFont("و", DUCTUS["و"].source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ه", DUCTUS["ه"].source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ا", URDU_ALEF.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ا", ARABIC_ALEF.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ب", ARABIC_BAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ت", ARABIC_TAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ج", ARABIC_JEEM.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ح", ARABIC_HAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("خ", ARABIC_KHAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("د", ARABIC_DAAL.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ر", ARABIC_RAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("و", "https://example.invalid/wrong-source")).toBeUndefined();
  });

  it("Hebrew א traces its two-run order to the dedicated HebrewPod101 lesson", () => {
    const src = HEBREW_ALEF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=JBVpQzvrJ4w");
    expect(src.citation).toMatch(/Hebrew Writing #1.*Alef and Beit.*01:33.0–01:35.8.*HebrewPod101/i);
    expect(src.variation).toMatch(
      /printed block Alef.*two handwritten variants.*descending main diagonal.*01:33.0–01:34.25.*lifts once.*opposing diagonal.*upper right.*crossing.*01:34.5–01:35.8.*styles vary.*X-like.*Noto Sans Hebrew.*two-stroke/i,
    );
  });

  it("Hebrew ב traces its block-style body separately from the optional dagesh", () => {
    const src = HEBREW_BET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=JBVpQzvrJ4w");
    expect(src.citation).toMatch(/Hebrew Writing #1.*Alef and Beit.*02:25.8–02:27.7.*HebrewPod101/i);
    expect(src.variation).toMatch(
      /two handwritten Bet styles.*second.*block-style.*top bar left-to-right.*right side.*without lifting.*02:25.8–02:26.7.*lifts once.*baseline left-to-right.*02:26.9–02:27.7.*dagesh.*02:27.9–02:28.2.*not part of the base ב glyph.*one-lift body.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ג traces its printed angular form without erasing the cursive alternative", () => {
    const src = HEBREW_GIMEL.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tN6Mf7fxxS4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Gimel, Dalet and Kamats.*00:54.2–00:55.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /contrasts.*rounded cursive Gimel.*00:48.4–00:50.1.*printed form.*top bar left-to-right.*right stem.*short lower-right leg.*00:54.2–00:55.4.*lifts once.*lower junction.*longer diagonal leg down-left.*00:55.4–00:55.9.*Noto Sans Hebrew.*handwriting variation/i,
    );
  });

  it("Hebrew ד preserves its one-curve cursive order on the angular block outline", () => {
    const src = HEBREW_DALET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tN6Mf7fxxS4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Gimel, Dalet and Kamats.*03:43.8–03:45.0.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /cursive Dalet.*broad arch left-to-right.*03:43.8–03:44.1.*small lower loop.*03:44.1–03:44.7.*descending tail.*03:45.0.*without lifting.*just one curve.*03:46.6–03:47.3.*printed Dalet.*angular.*Noto Sans Hebrew.*block top-bar-and-right-downstroke.*single continuous run.*zero-lift/i,
    );
  });

  it("Hebrew ה traces its printed body separately from the detached left leg", () => {
    const src = HEBREW_HEI.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=FtCuWlS6V7g");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Hei.*00:59.6–01:01.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /curved handwritten Hei.*00:53.7–00:55.1.*printed form.*sharp angles.*top bar left-to-right.*right side.*without lifting.*00:59.6–01:00.8.*lifts once.*detached left leg top-to-bottom.*01:01.2–01:01.9.*curved in handwriting.*sharp angles in print.*01:03.5–01:09.4.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ו keeps its printed head and stem in one sourced run", () => {
    const src = HEBREW_VAV.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=kJUMyHR0zN4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Vav, Hirik, and Shuruk.*01:08.6–01:09.8.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Vav.*top to bottom.*00:57.3–00:58.2.*printed form.*01:08.6–01:09.8.*head runs left-to-right.*vertical stem without lifting.*one stroke from top to bottom.*01:00.0–01:02.5.*print version.*small difference.*01:03.6–01:10.9.*Noto Sans Hebrew.*Hirik and Shuruk.*vowel marks.*not part of base U\+05D5.*zero-lift body count/i,
    );
  });

  it("ம's stroke order traces to the UT Austin primer, and records Tamil's variation", () => {
    const src = DUCTUS["ம"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I|Frame 1/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("அ's stroke order traces to Frame 4 of the same primer", () => {
    const src = DUCTUS["அ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*அ/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ஆ's stroke order traces to the next row of Frame 4", () => {
    const src = DUCTUS["ஆ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*ஆ/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("இ's stroke order traces to Frame 4's third row", () => {
    const src = DUCTUS["இ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*இ/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("க's stroke order traces to Frame 3's final row", () => {
    const src = DUCTUS["க"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*க/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("வ's stroke order traces to Frame 9's first row", () => {
    const src = DUCTUS["வ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 9.*வ/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ல's stroke order traces to Frame 9's second row", () => {
    const src = DUCTUS["ல"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 9.*ல/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ற's stroke order traces to Frame 10", () => {
    const src = DUCTUS["ற"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 10.*ற/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ன's stroke order traces to Frame 13's first row", () => {
    const src = DUCTUS["ன"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ன/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ண's stroke order traces to Frame 13's adjacent row", () => {
    const src = DUCTUS["ண"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ண/);
    expect(src.variation, "must not present one order as the only order").toMatch(/variation|no single/i);
  });

  it("ந's stroke order traces to Frame 12 and records the Noto adaptation", () => {
    const src = DUCTUS["ந"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 12.*ந/);
    expect(src.variation).toMatch(/looped handwritten form.*Noto/i);
  });

  it("Persian ا traces to UT Austin's opening right-to-left freehand demonstration", () => {
    const src = DUCTUS["ا"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ا.*00:08–00:11/i);
    expect(src.variation).toMatch(/top-to-bottom.*right-to-left.*Noto Naskh/i);
  });

  it("Urdu independent ا traces to Zer o Zabar's top-to-bottom animation", () => {
    const src = URDU_ALEF.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(src.citation).toMatch(/Zer o Zabar.*independent ا.*Northwestern/i);
    expect(src.variation).toMatch(
      /independent.*top-to-bottom.*one continuous stroke.*final.*bottom-to-top.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["ا"].source.url);
  });

  it("Arabic independent ا traces to the University of Oregon's top-to-bottom video", () => {
    const src = ARABIC_ALEF.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%A8/",
    );
    expect(src.citation).toMatch(/Introduction to Arabic.*Alphabet ا ب.*00:05–00:07.*Oregon/i);
    expect(src.variation).toMatch(
      /one continuous top-to-bottom stroke.*no pen lift.*one-way connector.*isolated and final forms.*Noto Naskh.*Arabic provenance.*Persian and Urdu/i,
    );
    expect(src.url).not.toBe(DUCTUS["ا"].source.url);
    expect(src.url).not.toBe(URDU_ALEF.source.url);
  });

  it("Arabic independent ب traces to the University of Oregon's bowl-first video", () => {
    const src = ARABIC_BAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%A8/",
    );
    expect(src.citation).toMatch(/Introduction to Arabic.*Alphabet ا ب.*Baa.*00:02–00:04.*Oregon/i);
    expect(src.variation).toMatch(
      /upper-right tip.*right-to-left.*shallow bowl.*left tip.*lifting once.*dot below.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic provenance.*Persian/i,
    );
    expect(src.url).not.toBe(DUCTUS["ب"].source.url);
  });

  it("Arabic independent ت traces its bowl and separate dots to the University of Oregon", () => {
    const src = ARABIC_TAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/two-way-connectors-%D8%A8-%D8%AA-%D8%AB-%D9%86-%D9%8A/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ب ت ث.*Baa.*00:02–00:04.*Taa.*00:00–00:01.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Baa demonstration.*upper-right tip.*right-to-left.*turned-up left tip.*Taa demonstration opens.*complete bowl.*left dot.*00:00.45–00:00.70.*right dot.*00:00.75–00:01.00.*does not redraw.*rather than inferring.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic provenance.*Persian/i,
    );
    expect(src.url).not.toBe(DUCTUS["ت"].source.url);
  });

  it("Arabic independent ج traces its body-first order to the University of Oregon", () => {
    const src = ARABIC_JEEM.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Jeem.*00:05–00:06.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /body first.*00:05.1–00:05.8.*upper head.*left-to-right.*turns downward.*curls back left.*rounded bowl.*without lifting.*lifts once.*dot below.*00:06.3–00:06.5.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic body-first provenance.*Urdu dot-first/i,
    );
    expect(src.url).not.toBe(URDU_JIM.source.url);
  });

  it("Arabic independent ح traces its stem-first order to the page's Haa attachment", () => {
    const src = ARABIC_HAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Haa.*00:00–00:01.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Haa attachment.*two pen-down runs.*opens.*first mark already underway.*short left stem downward.*00:00.00–00:00.15.*lifts once.*restarts near the stem's upper portion.*00:00.32.*down-right and around the bowl.*without another lift.*00:00.82.*two-way connector.*contextual shapes.*no dot stroke.*stem-first order.*rather than inherited from ج.*Noto Naskh.*Arabic provenance/i,
    );
    expect(src.url).toBe(ARABIC_JEEM.source.url);
  });

  it("Arabic independent خ traces its body-first order to its own Khaa clip", () => {
    const src = ARABIC_KHAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Khaa.*00:02–00:04.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Khaa QuickTime clip.*body-first.*00:02.8–00:03.9.*upper head.*left-to-right.*same pen-down run.*turns downward.*curls around the bowl.*lifts once.*dot above.*00:04.2–00:04.4.*two-way connector.*contextual shapes.*own clip.*matches adjacent Jeem.*rather than Haa.*stem-first restart.*Noto Naskh.*Arabic provenance/i,
    );
    expect(src.url).toBe(ARABIC_JEEM.source.url);
    expect(src.url).toBe(ARABIC_HAA.source.url);
  });

  it("Arabic independent د traces its unbroken turn to the University of Oregon", () => {
    const src = ARABIC_DAAL.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/chapter-1/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: د ذ ر.*Daal.*00:07.0–00:07.6.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*00:07.0–00:07.6.*upper tip.*diagonally down and right.*curved shoulder.*turns left.*baseline.*without lifting.*one-way connector.*independent and final forms.*Noto Naskh.*scoped to Arabic.*contextual form/i,
    );
  });

  it("Arabic independent ر traces its unbroken curve to the University of Oregon", () => {
    const src = ARABIC_RAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/chapter-1/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: د ذ ر.*Raa.*00:08.8–00:09.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*00:08.8–00:09.3.*upper tip.*descends through the short stroke.*sweeps left.*lower curve.*without lifting.*one-way connector.*independent and final forms.*Noto Naskh.*scoped to Arabic.*Urdu ر source.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(URDU_RE.source.url);
  });

  it("Arabic independent س traces its continuous teeth and bowl to the University of Oregon", () => {
    const src = ARABIC_SEEN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Seen.*00:01.6–00:02.8.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-8.mov.*one continuous pen-down run.*00:01.6–00:02.8.*upper right.*three close teeth.*right to left.*final bowl.*without lifting.*two-way connector.*contextual shapes.*Noto Naskh.*scoped to Arabic.*Persian or Urdu س sources.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["س"].source.url);
    expect(src.url).not.toBe(URDU_SIN.source.url);
  });

  it("Arabic independent ش traces its body-first dots to the University of Oregon", () => {
    const src = ARABIC_SHIIN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Shiin.*00:00.7–00:03.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-7.mov.*body-first.*one continuous pen-down run.*00:00.7–00:02.2.*three close teeth.*right to left.*final bowl.*lower-left dot.*00:02.4–00:02.5.*lower-right dot.*00:02.7–00:02.8.*centered upper dot.*00:02.9–00:03.0.*two-way connector.*contextual shapes.*four-stroke.*three-lift.*Noto Naskh.*scoped to Arabic.*Urdu ش source.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(URDU_SHIN.source.url);
  });

  it("Arabic independent ص traces its lifted trailing bowl to the University of Oregon", () => {
    const src = ARABIC_SAAD.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Saad.*00:01.1–00:03.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-6.mov.*two pen-down runs.*00:01.1–00:02.4.*lower-left junction.*oval clockwise.*turns left.*short shoulder.*without lifting.*one lift.*00:02.6–00:03.3.*baseline junction.*descends.*trailing bowl.*sweeps left.*finishes above the baseline.*two-way connector.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*distinct from.*Seen and Shiin/i,
    );
  });

  it("Arabic independent ض traces its Saad skeleton and final dot to the embedded Oregon lesson", () => {
    const src = ARABIC_DAAD.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Daad.*00:43.1–00:46.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /embedded Panopto Daad lesson.*three pen-down runs.*00:43.1–00:46.3.*00:43.1–00:45.0.*lower-left junction.*oval clockwise.*short shoulder.*without lifting.*one lift.*00:45.2–00:45.4.*baseline junction.*trailing bowl.*second lift.*upper dot last.*00:46.0–00:46.3.*FullSizeRender-5.mov.*HTTP 403.*accessible embedded primary lesson.*embedded Saad lesson.*direct Saad clip.*same two body runs.*two-way connector.*contextual shapes.*three-stroke.*two-lift.*Noto Naskh.*independently evidenced.*Saad/i,
    );
  });

  it("Arabic independent ع traces its unbroken head and bowl to the Oregon MOV", () => {
    const src = ARABIC_AYN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B9-%D8%BA/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ع غ.*Ayn.*00:03.1–00:04.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked ayn.mov.*one continuous pen-down run.*00:03.1–00:04.0.*00:03.1–00:03.5.*upper-right tip.*sweeps left.*hooks downward.*open head.*without lifting.*00:03.5–00:04.0.*left side.*lower bowl.*floor.*finishes toward the right.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*distinct from.*Ghayn.*upper dot/i,
    );
  });

  it("Arabic independent ك traces its joined outer body and restarted inner arm to the Oregon MOV", () => {
    const src = ARABIC_KAF.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Kaf.*00:11.8–00:13.4.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked kaf.mov.*two pen-down runs.*00:11.8–00:12.9.*main upright.*turns left.*baseline.*without lifting.*one lift.*00:13.2–00:13.4.*upper right.*inner arm.*down-left.*two-way connector.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*Arabic-scoped ك.*distinct from Urdu ک.*different Unicode glyph.*separate source-backed fallback order/i,
    );
    expect(src.url).not.toBe(URDU_KAF.source.url);
    expect(ARABIC_KAF.glyph).not.toBe(URDU_KAF.glyph);
  });

  it("Arabic independent ل traces its unbroken upright and bowl to the Oregon MOV", () => {
    const src = ARABIC_LAM.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Lam.*00:01.9–00:02.4.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked lam.mov.*one continuous pen-down run.*00:01.9–00:02.4.*descends the tall upright.*turns left.*base bowl.*without lifting.*rises.*outer edge.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*Arabic-scoped ل.*distinct from.*Persian and Urdu.*same Unicode glyph.*own source-backed orders/i,
    );
    expect(src.url).not.toBe(DUCTUS["ل"].source.url);
    expect(src.url).not.toBe(URDU_LAM.source.url);
    expect(ARABIC_LAM.glyph).toBe(DUCTUS["ل"].glyph);
    expect(ARABIC_LAM.glyph).toBe(URDU_LAM.glyph);
  });

  it("Arabic independent ي traces its bowl and left-then-right lower dots to the Oregon MOV", () => {
    const src = ARABIC_YAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Yaa.*00:33.2–00:35.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked yaa.mov.*three pen-down runs.*upper right.*00:33.2.*descends.*sweeps left.*independent bowl.*without lifting.*00:34.4.*one lift.*lower-left dot.*00:34.5–00:34.7.*second lift.*lower-right dot.*00:34.8–00:35.0.*two-way connector.*contextual shapes.*three-stroke.*two-lift.*Noto Naskh.*U\+064A.*separate from Urdu ی.*U\+06CC.*no lower dots.*own source-backed order/i,
    );
    expect(src.url).not.toBe(URDU_YE.source.url);
    expect(ARABIC_YAA.glyph).not.toBe(URDU_YE.glyph);
  });

  it("Arabic independent ه traces its two counters and baseline sweep to the Oregon MOV", () => {
    const src = ARABIC_HEH.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%d9%87-%d9%88-%d9%8a/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ه و ي.*Heh.*00:04.9–00:06.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked letter-haa.mov.*one continuous pen-down run.*00:04.9–00:06.0.*upper right.*00:04.9–00:05.4.*down-left.*lower counter.*without lifting.*centre.*upper-right counter.*00:05.4–00:05.7.*baseline.*00:06.0.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*Arabic ه.*script-scoped provenance.*Persian.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["ه"].source.url);
    expect(ARABIC_HEH.glyph).toBe(DUCTUS["ه"].glyph);
  });

  it("Arabic independent و traces its closed head and leftward tail to the Oregon MOV", () => {
    const src = ARABIC_WAW.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%d9%87-%d9%88-%d9%8a/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ه و ي.*Waw.*00:45.7–00:46.9.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked waw.mov.*one continuous pen-down run.*00:45.7–00:46.9.*lower-right junction.*00:45.7–00:46.5.*sweeps left.*curves up and around.*small head loop.*without lifting.*00:46.5–00:46.9.*descends.*curls left.*tail.*one-way connector.*consonant w.*long-vowel ū.*one-stroke.*zero-lift.*Noto Naskh.*Arabic و.*script-scoped provenance.*Persian.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["و"].source.url);
    expect(ARABIC_WAW.glyph).toBe(DUCTUS["و"].glyph);
  });

  it("Urdu independent ج traces to Zer o Zabar's dot-first pointed-head animation", () => {
    const src = URDU_JIM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(src.citation).toMatch(/Zer o Zabar.*independent ج.*flat-head.*Northwestern/i);
    expect(src.variation).toMatch(
      /dot below first.*lifts once.*pointed hooked head.*one continuous stroke.*pointed rather than rounded.*flat-head.*purely aesthetic.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ر traces to Zer o Zabar's downward-then-leftward animation", () => {
    const src = URDU_RE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/dal-re-and-waw/",
    );
    expect(src.citation).toMatch(/Zer o Zabar.*independent ر.*Re instructions.*Northwestern/i);
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*downward line.*curve to the left.*final form.*lower left.*final re rises in Naskh.*not in Nastaliq.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent س traces to Zer o Zabar's continuous teeth-and-bowl animations", () => {
    const src = URDU_SIN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent س.*calligraphic and handwriting animations.*Sīn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*three close teeth.*right to left.*final bowl without lifting.*optional long gentle curve.*especially common in handwriting.*adjacent sīns.*standard toothed.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["س"].source.url);
  });

  it("Urdu independent ش traces to Zer o Zabar's body-first three-dot animations", () => {
    const src = URDU_SHIN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ش.*calligraphic and handwriting animations.*Shīn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /standard toothed sīn body first.*lower-left dot.*lower-right dot.*centered upper dot.*four strokes.*three pen lifts.*two below.*nestled.*optional long gentle curve.*dots stay centered.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ک traces to Zer o Zabar's body-first two-stroke animations", () => {
    const src = URDU_KAF.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/be-kaf-and-short-vowels/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ک.*calligraphic and handwriting animations.*Kāf instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /two separate pen strokes.*stem.*main line.*right to left.*flatter bowl.*pronounced final hook.*lift once.*upper right.*long downward slash.*not to write kāf in one penstroke.*flatter than be.*Noto Naskh.*rather than Arabic ك.*Nastaliq/i,
    );
  });

  it("Urdu independent ل traces to Zer o Zabar's unbroken downward-and-around animations", () => {
    const src = URDU_LAM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ل.*calligraphic and handwriting animations.*Lām instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*begin at the top.*descend the tall upright.*below the baseline.*leftward bowl.*back up.*without lifting.*connector.*final form.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent م traces to Zer o Zabar's unbroken head-and-tail animations", () => {
    const src = URDU_MIM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent م.*calligraphic and handwriting animations.*Mīm instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*calligraphic and handwritten.*ordinary constant-width pen.*counterclockwise loop.*independent or final mīm drops below the baseline.*head-to-tail.*zero-lift.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["م"].source.url);
  });

  it("Urdu independent ن traces to Zer o Zabar's bowl-first, dot-second animations", () => {
    const src = URDU_NUN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ن.*calligraphic and handwriting animations.*Nūn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /bowl first.*one uninterrupted right-to-left run.*lift once.*dot near the baseline.*final and independent nūn.*below the baseline.*initial and medial.*be-series tooth.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["ن"].source.url);
  });

  it("Urdu independent ہ traces to Zer o Zabar's unbroken teardrop animations", () => {
    const src = URDU_HE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ہ.*calligraphic and handwriting animations.*Chhoṭī he instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted counterclockwise loop.*upper right.*down and left.*around the base.*return up the right side.*cross at the top.*without lifting.*independent form.*oval or teardrop.*initial and medial.*small divot.*number-6-like mark.*final form.*up and then down.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ی traces to Zer o Zabar's dotless S-shaped animations", () => {
    const src = URDU_YE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ی.*calligraphic and handwriting animations.*Chhoṭī ye instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted dotless S-shaped body.*upper right.*descend through the upper curve.*sweep left around the below-baseline bowl.*rising tip.*without lifting.*independent and final chhoṭī ye.*ī sound.*initial and medial.*be-series tooth.*two dots below.*do not belong to the independent form.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ں traces to Zer o Zabar's dotless nūn animations", () => {
    const src = URDU_GHUNNA.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ں.*calligraphic and handwriting animations.*Nasalization with nūn-e ġhunna instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted right-to-left bowl.*below the baseline.*without lifting.*final and independent nūn-e ġhunna.*nūn without any dot.*initial and medial.*identical to regular nūn.*sukūn.*semicircular diacritic.*U\+06BA.*U\+0646.*body contour.*dot removed.*Nastaliq/i,
    );
  });

  it("Persian ب traces to the adjacent sourced bowl-and-dot demonstration", () => {
    const src = DUCTUS["ب"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ب.*00:11–00:15/i);
    expect(src.variation).toMatch(/right-to-left.*pen lift.*dot below.*Noto Naskh/i);
  });

  it("Persian ت traces to the later sourced bowl-and-two-dots demonstration", () => {
    const src = DUCTUS["ت"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ت.*00:22–00:27/i);
    expect(src.variation).toMatch(/right-to-left.*left dot.*another lift.*right dot.*Noto Naskh/i);
  });

  it("Persian س traces to the later continuous teeth-and-bowl demonstration", () => {
    const src = DUCTUS["س"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*س.*01:29–01:35/i);
    expect(src.variation).toMatch(/continuous right-to-left.*three teeth.*final bowl.*no pen lift.*Noto Naskh/i);
  });

  it("Persian ل traces to the later continuous upright-and-base demonstration", () => {
    const src = DUCTUS["ل"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ل.*02:29–02:32/i);
    expect(src.variation).toMatch(/isolated.*continuous Naskh.*upright descends.*base curve.*no pen lift.*Noto Naskh/i);
  });

  it("Persian م traces to the adjacent continuous head-and-tail demonstration", () => {
    const src = DUCTUS["م"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*م.*02:33–02:36/i);
    expect(src.variation).toMatch(/isolated.*continuous Naskh.*round head.*descending tail.*no pen lift.*Noto Naskh/i);
  });

  it("Persian ن traces to the adjacent bowl-and-dot demonstration", () => {
    const src = DUCTUS["ن"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ن.*02:37–02:43/i);
    expect(src.variation).toMatch(/isolated.*right-to-left Naskh bowl.*one lift.*dot above.*Noto Naskh/i);
  });

  it("Persian و traces to the intervening continuous loop-and-tail demonstration", () => {
    const src = DUCTUS["و"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*و.*02:43–02:45/i);
    expect(src.variation).toMatch(/isolated.*continuous Naskh.*small head.*leftward curving tail.*no pen lift.*Noto Naskh/i);
  });

  it("Persian ه traces to the later continuous looping-body demonstration", () => {
    const src = DUCTUS["ه"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ه.*02:47–02:50/i);
    expect(src.variation).toMatch(
      /simple closed handwritten loop.*no pen lift.*Noto Naskh.*two counters.*leftward baseline/i,
    );
  });
});

describe("pen-path geometry", () => {
  const stroke = DUCTUS["ம"].strokes[0];

  it("penPath joins segments head-to-tail without duplicating the join", () => {
    const segTotal = stroke.segments.reduce((n, s) => n + s.path.length, 0);
    // Two joins collapse two duplicated points, so the path is 2 shorter.
    expect(penPath(stroke).length).toBe(segTotal - (stroke.segments.length - 1));
  });

  it("penPathD grows monotonically with the fraction drawn", () => {
    const q = penPathD(stroke, 0.25).length;
    const h = penPathD(stroke, 0.5).length;
    const f = penPathD(stroke, 1).length;
    expect(q).toBeLessThan(h);
    expect(h).toBeLessThanOrEqual(f);
    expect(penPathD(stroke, 1)).toMatch(/^M/);
  });

  it("penTip advances along the stroke and ends where the pen ends", () => {
    const start = penTip(stroke, 0).at;
    const end = penTip(stroke, 1).at;
    const first = stroke.segments[0].path[0];
    expect(start.x).toBeCloseTo(first.x);
    expect(start.y).toBeCloseTo(first.y);
    // ம ends at the bottom of the middle upright, well right of and below its start.
    expect(end.x).toBeGreaterThan(start.x);
    expect(end.y).toBeLessThan(start.y);
  });

  // -------------------------------------------------------------------------
  // Controls: prove each honesty check above can actually FAIL.
  // -------------------------------------------------------------------------
  it("CONTROL: a stroke pushed off the glyph fails the on-ink check", () => {
    const inInk = makeInInk(tamil().glyphFor("ம")!.contours);
    const shifted = penPath(stroke).map((p) => ({ x: p.x + 400, y: p.y }));
    expect(fractionOnInk(shifted, inInk)).toBeLessThan(0.9);
  });

  it("CONTROL: a broken join is caught by the gap check", () => {
    const broken = {
      segments: [
        { label: "a", path: [{ x: 0, y: 0 }, { x: 100, y: 0 }] },
        { label: "b", path: [{ x: 100, y: 80 }, { x: 200, y: 80 }] }, // starts 80 away
      ],
    };
    expect(Math.max(...joinGaps(broken))).toBeGreaterThan(2);
  });

  it("CONTROL: dropping the arch leaves much of the letter untraced", () => {
    const inInk = tamil().glyphFor("ம")!;
    const pts = inkPoints(inInk.contours);
    const onlyFirstTwo = {
      segments: DUCTUS["ம"].strokes[0].segments.slice(0, 2),
    };
    const path = penPath(onlyFirstTwo);
    const strayed = pts.filter(([x, y]) => distanceToPath(x, y, path) > 130);
    expect(strayed.length / pts.length).toBeGreaterThan(0.1);
  });
});
