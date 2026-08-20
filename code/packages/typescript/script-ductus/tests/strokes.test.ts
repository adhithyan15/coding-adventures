import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseFont, boundsOf, type Contour } from "../src/truetype";
import { SCRIPTS, verifiedLetterFont } from "../src/scriptdata";
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
const CHINESE_REN = DUCTUS[ductusKey("chinese", "人")];
const CHINESE_PERSON_RADICAL = DUCTUS[ductusKey("chinese", "亻")];
const CHINESE_MOUTH = DUCTUS[ductusKey("chinese", "口")];
const CHINESE_WOMAN = DUCTUS[ductusKey("chinese", "女")];
const CHINESE_CHILD = DUCTUS[ductusKey("chinese", "子")];
const CHINESE_SUN = DUCTUS[ductusKey("chinese", "日")];
const CHINESE_SPEECH_RADICAL = DUCTUS[ductusKey("chinese", "讠")];
const CHINESE_WATER_RADICAL = DUCTUS[ductusKey("chinese", "氵")];
const CHINESE_ROOF_RADICAL = DUCTUS[ductusKey("chinese", "宀")];
const CHINESE_YOU = DUCTUS[ductusKey("chinese", "你")];
const CHINESE_GOOD = DUCTUS[ductusKey("chinese", "好")];
const CHINESE_I = DUCTUS[ductusKey("chinese", "我")];
const CHINESE_BE = DUCTUS[ductusKey("chinese", "是")];
const CHINESE_NOT = DUCTUS[ductusKey("chinese", "不")];
const CHINESE_NAME = DUCTUS[ductusKey("chinese", "名")];
const CHINESE_CHARACTER = DUCTUS[ductusKey("chinese", "字")];
const CHINESE_THANK = DUCTUS[ductusKey("chinese", "谢")];
const CHINESE_PLEASE = DUCTUS[ductusKey("chinese", "请")];
const CHINESE_AGAIN = DUCTUS[ductusKey("chinese", "再")];
const CHINESE_SEE = DUCTUS[ductusKey("chinese", "见")];
const CHINESE_WHAT = DUCTUS[ductusKey("chinese", "什")];
const CHINESE_PARTICLE_ME = DUCTUS[ductusKey("chinese", "么")];
const CHINESE_EARLY = DUCTUS[ductusKey("chinese", "早")];
const CHINESE_UP = DUCTUS[ductusKey("chinese", "上")];
const DEVANAGARI_A = DUCTUS[ductusKey("devanagari", "अ")];
const DEVANAGARI_AA = DUCTUS[ductusKey("devanagari", "आ")];
const DEVANAGARI_I = DUCTUS[ductusKey("devanagari", "इ")];
const DEVANAGARI_II = DUCTUS[ductusKey("devanagari", "ई")];
const DEVANAGARI_U = DUCTUS[ductusKey("devanagari", "उ")];
const DEVANAGARI_UU = DUCTUS[ductusKey("devanagari", "ऊ")];
const DEVANAGARI_E = DUCTUS[ductusKey("devanagari", "ए")];
const DEVANAGARI_AI = DUCTUS[ductusKey("devanagari", "ऐ")];
const DEVANAGARI_O = DUCTUS[ductusKey("devanagari", "ओ")];
const DEVANAGARI_AU = DUCTUS[ductusKey("devanagari", "औ")];
const DEVANAGARI_KA = DUCTUS[ductusKey("devanagari", "क")];
const DEVANAGARI_GA = DUCTUS[ductusKey("devanagari", "ग")];
const DEVANAGARI_CA = DUCTUS[ductusKey("devanagari", "च")];
const DEVANAGARI_TA = DUCTUS[ductusKey("devanagari", "त")];
const DEVANAGARI_DA = DUCTUS[ductusKey("devanagari", "द")];
const DEVANAGARI_DHA = DUCTUS[ductusKey("devanagari", "ध")];
const DEVANAGARI_NA = DUCTUS[ductusKey("devanagari", "न")];
const DEVANAGARI_PA = DUCTUS[ductusKey("devanagari", "प")];
const DEVANAGARI_BA = DUCTUS[ductusKey("devanagari", "ब")];
const DEVANAGARI_BHA = DUCTUS[ductusKey("devanagari", "भ")];
const DEVANAGARI_MA = DUCTUS[ductusKey("devanagari", "म")];
const DEVANAGARI_YA = DUCTUS[ductusKey("devanagari", "य")];
const DEVANAGARI_RA = DUCTUS[ductusKey("devanagari", "र")];
const DEVANAGARI_LA = DUCTUS[ductusKey("devanagari", "ल")];
const DEVANAGARI_VA = DUCTUS[ductusKey("devanagari", "व")];
const DEVANAGARI_SHA = DUCTUS[ductusKey("devanagari", "श")];
const DEVANAGARI_SA = DUCTUS[ductusKey("devanagari", "स")];
const DEVANAGARI_HA = DUCTUS[ductusKey("devanagari", "ह")];
const CYRILLIC_A = DUCTUS[ductusKey("cyrillic", "а")];
const CYRILLIC_BE = DUCTUS[ductusKey("cyrillic", "б")];
const CYRILLIC_VE = DUCTUS[ductusKey("cyrillic", "в")];
const CYRILLIC_GE = DUCTUS[ductusKey("cyrillic", "г")];
const CYRILLIC_DE = DUCTUS[ductusKey("cyrillic", "д")];
const CYRILLIC_IE = DUCTUS[ductusKey("cyrillic", "е")];
const CYRILLIC_IO = DUCTUS[ductusKey("cyrillic", "ё")];
const CYRILLIC_ZHE = DUCTUS[ductusKey("cyrillic", "ж")];
const CYRILLIC_ZE = DUCTUS[ductusKey("cyrillic", "з")];
const CYRILLIC_I = DUCTUS[ductusKey("cyrillic", "и")];
const CYRILLIC_SHORT_I = DUCTUS[ductusKey("cyrillic", "й")];
const CYRILLIC_KA = DUCTUS[ductusKey("cyrillic", "к")];
const CYRILLIC_EL = DUCTUS[ductusKey("cyrillic", "л")];
const CYRILLIC_EM = DUCTUS[ductusKey("cyrillic", "м")];
const CYRILLIC_EN = DUCTUS[ductusKey("cyrillic", "н")];
const CYRILLIC_O = DUCTUS[ductusKey("cyrillic", "о")];
const CYRILLIC_PE = DUCTUS[ductusKey("cyrillic", "п")];
const CYRILLIC_ER = DUCTUS[ductusKey("cyrillic", "р")];
const CYRILLIC_ES = DUCTUS[ductusKey("cyrillic", "с")];
const CYRILLIC_TE = DUCTUS[ductusKey("cyrillic", "т")];
const CYRILLIC_U = DUCTUS[ductusKey("cyrillic", "у")];
const CYRILLIC_EF = DUCTUS[ductusKey("cyrillic", "ф")];
const CYRILLIC_HA = DUCTUS[ductusKey("cyrillic", "х")];
const CYRILLIC_TSE = DUCTUS[ductusKey("cyrillic", "ц")];
const CYRILLIC_CHE = DUCTUS[ductusKey("cyrillic", "ч")];
const CYRILLIC_SHA = DUCTUS[ductusKey("cyrillic", "ш")];
const CYRILLIC_SHCHA = DUCTUS[ductusKey("cyrillic", "щ")];
const CYRILLIC_HARD_SIGN = DUCTUS[ductusKey("cyrillic", "ъ")];
const CYRILLIC_YERY = DUCTUS[ductusKey("cyrillic", "ы")];
const CYRILLIC_SOFT_SIGN = DUCTUS[ductusKey("cyrillic", "ь")];
const CYRILLIC_E = DUCTUS[ductusKey("cyrillic", "э")];
const CYRILLIC_YU = DUCTUS[ductusKey("cyrillic", "ю")];
const CYRILLIC_YA = DUCTUS[ductusKey("cyrillic", "я")];
const GUJARATI_A = DUCTUS[ductusKey("gujarati", "અ")];
const GUJARATI_AA = DUCTUS[ductusKey("gujarati", "આ")];
const GUJARATI_I = DUCTUS[ductusKey("gujarati", "ઇ")];
const GUJARATI_II = DUCTUS[ductusKey("gujarati", "ઈ")];
const GUJARATI_U = DUCTUS[ductusKey("gujarati", "ઉ")];
const GUJARATI_UU = DUCTUS[ductusKey("gujarati", "ઊ")];
const GUJARATI_VOCALIC_R = DUCTUS[ductusKey("gujarati", "ઋ")];
const GUJARATI_E = DUCTUS[ductusKey("gujarati", "એ")];
const GUJARATI_AI = DUCTUS[ductusKey("gujarati", "ઐ")];
const GUJARATI_O = DUCTUS[ductusKey("gujarati", "ઓ")];
const GUJARATI_AU = DUCTUS[ductusKey("gujarati", "ઔ")];
const GUJARATI_KA = DUCTUS[ductusKey("gujarati", "ક")];
const GUJARATI_KHA = DUCTUS[ductusKey("gujarati", "ખ")];
const GUJARATI_GA = DUCTUS[ductusKey("gujarati", "ગ")];
const GUJARATI_GHA = DUCTUS[ductusKey("gujarati", "ઘ")];
const GUJARATI_NGA = DUCTUS[ductusKey("gujarati", "ઙ")];
const GUJARATI_CA = DUCTUS[ductusKey("gujarati", "ચ")];
const GUJARATI_CHA = DUCTUS[ductusKey("gujarati", "છ")];
const GUJARATI_JA = DUCTUS[ductusKey("gujarati", "જ")];
const GUJARATI_JHA = DUCTUS[ductusKey("gujarati", "ઝ")];
const GUJARATI_NYA = DUCTUS[ductusKey("gujarati", "ઞ")];
const GUJARATI_TTA = DUCTUS[ductusKey("gujarati", "ટ")];
const GUJARATI_TTHA = DUCTUS[ductusKey("gujarati", "ઠ")];
const GUJARATI_DDA = DUCTUS[ductusKey("gujarati", "ડ")];
const HEBREW_ALEF = DUCTUS[ductusKey("hebrew", "א")];
const HEBREW_BET = DUCTUS[ductusKey("hebrew", "ב")];
const HEBREW_GIMEL = DUCTUS[ductusKey("hebrew", "ג")];
const HEBREW_DALET = DUCTUS[ductusKey("hebrew", "ד")];
const HEBREW_HEI = DUCTUS[ductusKey("hebrew", "ה")];
const HEBREW_VAV = DUCTUS[ductusKey("hebrew", "ו")];
const HEBREW_ZAYIN = DUCTUS[ductusKey("hebrew", "ז")];
const HEBREW_HEIT = DUCTUS[ductusKey("hebrew", "ח")];
const HEBREW_TET = DUCTUS[ductusKey("hebrew", "ט")];
const HEBREW_YOD = DUCTUS[ductusKey("hebrew", "י")];
const HEBREW_KAF = DUCTUS[ductusKey("hebrew", "כ")];
const HEBREW_LAMED = DUCTUS[ductusKey("hebrew", "ל")];
const HEBREW_MEM = DUCTUS[ductusKey("hebrew", "מ")];
const HEBREW_NUN = DUCTUS[ductusKey("hebrew", "נ")];
const HEBREW_SAMEKH = DUCTUS[ductusKey("hebrew", "ס")];
const HEBREW_AYIN = DUCTUS[ductusKey("hebrew", "ע")];
const HEBREW_PE = DUCTUS[ductusKey("hebrew", "פ")];
const HEBREW_TSADI = DUCTUS[ductusKey("hebrew", "צ")];
const HEBREW_QOF = DUCTUS[ductusKey("hebrew", "ק")];
const HEBREW_RESH = DUCTUS[ductusKey("hebrew", "ר")];
const HEBREW_SHIN = DUCTUS[ductusKey("hebrew", "ש")];
const HEBREW_TAV = DUCTUS[ductusKey("hebrew", "ת")];
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
      // 100 units. The original note here said every real ink point measured within
      // 67 of its path; that went stale as the table grew and is corrected rather
      // than repeated -- sixteen Chinese entries already exceed 67, the worst being
      // 讠 at 92.8. The threshold still has margin and still catches a dropped
      // feature: leaving out any ONE stroke of any numeral pushes the stray ratio to
      // 0.093-0.576, an order of magnitude past the 0.02 limit.
        const strayed = pts.filter(([x, y]) => nearest(x, y) > 100);
        expect(strayed.length / pts.length, "large parts of the letter are never traced").toBeLessThan(0.02);
      });
    });
  }

  it("ம is written without lifting the pen (one stroke)", () => {
    expect(penLifts(DUCTUS["ம"])).toBe(0);
    expect(DUCTUS["ம"].strokes).toHaveLength(1);
  });

  it("Chinese 人 draws the left-falling stroke before the lifted right-falling stroke", () => {
    expect(CHINESE_REN.script).toBe("chinese");
    expect(penLifts(CHINESE_REN)).toBe(1);
    expect(CHINESE_REN.strokes).toHaveLength(2);
    expect(CHINESE_REN.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const left = CHINESE_REN.strokes[0].segments[0].path;
    const right = CHINESE_REN.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(right[0].x).toBeLessThan(right.at(-1)!.x);
  });

  it("Chinese 亻 draws the left-falling stroke before the lifted vertical", () => {
    expect(CHINESE_PERSON_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_PERSON_RADICAL)).toBe(1);
    expect(CHINESE_PERSON_RADICAL.strokes).toHaveLength(2);
    expect(CHINESE_PERSON_RADICAL.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const left = CHINESE_PERSON_RADICAL.strokes[0].segments[0].path;
    const vertical = CHINESE_PERSON_RADICAL.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical[0].x).toBe(vertical.at(-1)!.x);
  });

  it("Chinese 口 descends the left, joins the top and right, then closes the bottom", () => {
    expect(CHINESE_MOUTH.script).toBe("chinese");
    expect(penLifts(CHINESE_MOUTH)).toBe(2);
    expect(CHINESE_MOUTH.strokes).toHaveLength(3);
    expect(CHINESE_MOUTH.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1]);
    const left = CHINESE_MOUTH.strokes[0].segments[0].path;
    const top = CHINESE_MOUTH.strokes[1].segments[0].path;
    const right = CHINESE_MOUTH.strokes[1].segments[1].path;
    const bottom = CHINESE_MOUTH.strokes[2].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 女 keeps its first bend joined before the falling and horizontal strokes", () => {
    expect(CHINESE_WOMAN.script).toBe("chinese");
    expect(penLifts(CHINESE_WOMAN)).toBe(2);
    expect(CHINESE_WOMAN.strokes).toHaveLength(3);
    expect(CHINESE_WOMAN.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    const descent = CHINESE_WOMAN.strokes[0].segments[0].path;
    const sweep = CHINESE_WOMAN.strokes[0].segments[1].path;
    const falling = CHINESE_WOMAN.strokes[1].segments[0].path;
    const horizontal = CHINESE_WOMAN.strokes[2].segments[0].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent[0].x).toBeGreaterThan(descent.at(-1)!.x);
    expect(descent.at(-1)).toEqual(sweep[0]);
    expect(sweep[0].x).toBeLessThan(sweep.at(-1)!.x);
    expect(sweep[0].y).toBeGreaterThan(sweep.at(-1)!.y);
    expect(falling[0].x).toBeGreaterThan(falling.at(-1)!.x);
    expect(falling[0].y).toBeGreaterThan(falling.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 子 keeps both hooks joined before its final horizontal", () => {
    expect(CHINESE_CHILD.script).toBe("chinese");
    expect(penLifts(CHINESE_CHILD)).toBe(2);
    expect(CHINESE_CHILD.strokes).toHaveLength(3);
    expect(CHINESE_CHILD.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 2, 1]);
    const top = CHINESE_CHILD.strokes[0].segments[0].path;
    const topTurn = CHINESE_CHILD.strokes[0].segments[1].path;
    const vertical = CHINESE_CHILD.strokes[1].segments[0].path;
    const baseHook = CHINESE_CHILD.strokes[1].segments[1].path;
    const horizontal = CHINESE_CHILD.strokes[2].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(topTurn[0]);
    expect(topTurn[0].x).toBeGreaterThan(topTurn.at(-1)!.x);
    expect(topTurn[0].y).toBeGreaterThan(topTurn.at(-1)!.y);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 日 writes the inside bar before separately closing the box", () => {
    expect(CHINESE_SUN.script).toBe("chinese");
    expect(penLifts(CHINESE_SUN)).toBe(3);
    expect(CHINESE_SUN.strokes).toHaveLength(4);
    expect(CHINESE_SUN.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1, 1]);
    const left = CHINESE_SUN.strokes[0].segments[0].path;
    const top = CHINESE_SUN.strokes[1].segments[0].path;
    const right = CHINESE_SUN.strokes[1].segments[1].path;
    const middle = CHINESE_SUN.strokes[2].segments[0].path;
    const bottom = CHINESE_SUN.strokes[3].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 讠 lifts after the dot and keeps both later turns joined", () => {
    expect(CHINESE_SPEECH_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_SPEECH_RADICAL)).toBe(1);
    expect(CHINESE_SPEECH_RADICAL.strokes).toHaveLength(2);
    expect(CHINESE_SPEECH_RADICAL.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 3]);
    const dot = CHINESE_SPEECH_RADICAL.strokes[0].segments[0].path;
    const horizontal = CHINESE_SPEECH_RADICAL.strokes[1].segments[0].path;
    const descent = CHINESE_SPEECH_RADICAL.strokes[1].segments[1].path;
    const rise = CHINESE_SPEECH_RADICAL.strokes[1].segments[2].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(descent[0]);
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(rise[0]);
    expect(rise[0].x).toBeLessThan(rise.at(-1)!.x);
    expect(rise[0].y).toBeLessThan(rise.at(-1)!.y);
  });

  it("Chinese 氵 draws two falling dots before its joined rising bottom stroke", () => {
    expect(CHINESE_WATER_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_WATER_RADICAL)).toBe(2);
    expect(CHINESE_WATER_RADICAL.strokes).toHaveLength(3);
    expect(CHINESE_WATER_RADICAL.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 2]);
    const upper = CHINESE_WATER_RADICAL.strokes[0].segments[0].path;
    const middle = CHINESE_WATER_RADICAL.strokes[1].segments[0].path;
    const bottomTurn = CHINESE_WATER_RADICAL.strokes[2].segments[0].path;
    const bottomRise = CHINESE_WATER_RADICAL.strokes[2].segments[1].path;
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(middle[0].y).toBeGreaterThan(middle.at(-1)!.y);
    expect(bottomTurn[0].x).toBeGreaterThan(bottomTurn[1].x);
    expect(bottomTurn.at(-1)).toEqual(bottomRise[0]);
    expect(bottomRise[0].x).toBeLessThan(bottomRise.at(-1)!.x);
    expect(bottomRise[0].y).toBeLessThan(bottomRise.at(-1)!.y);
  });

  it("Chinese 宀 draws two separate marks before its joined roof hook", () => {
    expect(CHINESE_ROOF_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_ROOF_RADICAL)).toBe(2);
    expect(CHINESE_ROOF_RADICAL.strokes).toHaveLength(3);
    expect(CHINESE_ROOF_RADICAL.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 2]);
    const dot = CHINESE_ROOF_RADICAL.strokes[0].segments[0].path;
    const left = CHINESE_ROOF_RADICAL.strokes[1].segments[0].path;
    const roof = CHINESE_ROOF_RADICAL.strokes[2].segments[0].path;
    const hook = CHINESE_ROOF_RADICAL.strokes[2].segments[1].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(roof[0].x).toBeLessThan(roof.at(-1)!.x);
    expect(roof.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    expect(hook[0].y).toBeGreaterThan(hook.at(-1)!.y);
  });

  it("Chinese 你 writes 亻 before two joined hooks and two separate dots", () => {
    expect(CHINESE_YOU.script).toBe("chinese");
    expect(penLifts(CHINESE_YOU)).toBe(6);
    expect(CHINESE_YOU.strokes).toHaveLength(7);
    expect(CHINESE_YOU.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1, 2, 2, 1, 1]);
    const personFall = CHINESE_YOU.strokes[0].segments[0].path;
    const personVertical = CHINESE_YOU.strokes[1].segments[0].path;
    const upperFall = CHINESE_YOU.strokes[2].segments[0].path;
    const horizontal = CHINESE_YOU.strokes[3].segments[0].path;
    const upperHook = CHINESE_YOU.strokes[3].segments[1].path;
    const vertical = CHINESE_YOU.strokes[4].segments[0].path;
    const baseHook = CHINESE_YOU.strokes[4].segments[1].path;
    const leftDot = CHINESE_YOU.strokes[5].segments[0].path;
    const rightDot = CHINESE_YOU.strokes[6].segments[0].path;
    expect(personFall[0].x).toBeGreaterThan(personFall.at(-1)!.x);
    expect(personVertical[0].y).toBeGreaterThan(personVertical.at(-1)!.y);
    expect(upperFall[0].x).toBeGreaterThan(upperFall.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(upperHook[0]);
    expect(upperHook[0].x).toBeGreaterThan(upperHook.at(-1)!.x);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(leftDot[0].x).toBeGreaterThan(leftDot.at(-1)!.x);
    expect(rightDot[0].x).toBeLessThan(rightDot.at(-1)!.x);
  });

  it("Chinese 好 writes 女 before 子 with three joined turns", () => {
    expect(CHINESE_GOOD.script).toBe("chinese");
    expect(penLifts(CHINESE_GOOD)).toBe(5);
    expect(CHINESE_GOOD.strokes).toHaveLength(6);
    expect(CHINESE_GOOD.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 2, 2, 1]);
    const womanBend = CHINESE_GOOD.strokes[0].segments[0].path;
    const womanSweep = CHINESE_GOOD.strokes[0].segments[1].path;
    const womanFall = CHINESE_GOOD.strokes[1].segments[0].path;
    const womanBar = CHINESE_GOOD.strokes[2].segments[0].path;
    const childBar = CHINESE_GOOD.strokes[3].segments[0].path;
    const childTurn = CHINESE_GOOD.strokes[3].segments[1].path;
    const childVertical = CHINESE_GOOD.strokes[4].segments[0].path;
    const childHook = CHINESE_GOOD.strokes[4].segments[1].path;
    const childMiddle = CHINESE_GOOD.strokes[5].segments[0].path;
    expect(womanBend.at(-1)).toEqual(womanSweep[0]);
    expect(womanSweep[0].x).toBeLessThan(womanSweep.at(-1)!.x);
    expect(womanFall[0].x).toBeGreaterThan(womanFall.at(-1)!.x);
    expect(womanBar[0].x).toBeLessThan(womanBar.at(-1)!.x);
    expect(childBar.at(-1)).toEqual(childTurn[0]);
    expect(childTurn[0].x).toBeGreaterThan(childTurn.at(-1)!.x);
    expect(childVertical.at(-1)).toEqual(childHook[0]);
    expect(childHook[0].x).toBeGreaterThan(childHook.at(-1)!.x);
    expect(childMiddle[0].x).toBeLessThan(childMiddle.at(-1)!.x);
  });

  it("Chinese 我 preserves seven strokes and the joined vertical hook", () => {
    expect(CHINESE_I.script).toBe("chinese");
    expect(penLifts(CHINESE_I)).toBe(6);
    expect(CHINESE_I.strokes).toHaveLength(7);
    expect(CHINESE_I.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 2, 1, 2, 1, 1]);
    const vertical = CHINESE_I.strokes[2].segments[0].path;
    const hook = CHINESE_I.strokes[2].segments[1].path;
    expect(vertical.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    const slash = CHINESE_I.strokes[4].segments[0].path;
    const slashHook = CHINESE_I.strokes[4].segments[1].path;
    expect(slash.at(-1)).toEqual(slashHook[0]);
    expect(slashHook[0].y).toBeLessThan(slashHook.at(-1)!.y);
  });

  it("Chinese 是 closes 日 before its five-stroke lower body", () => {
    expect(CHINESE_BE.script).toBe("chinese");
    expect(penLifts(CHINESE_BE)).toBe(8);
    expect(CHINESE_BE.strokes).toHaveLength(9);
    expect(CHINESE_BE.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1, 1, 1, 1, 1, 1, 1]);
    const top = CHINESE_BE.strokes[1].segments[0].path;
    const right = CHINESE_BE.strokes[1].segments[1].path;
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Chinese 不 keeps its four source strokes as four pen-down runs", () => {
    expect(CHINESE_NOT.script).toBe("chinese");
    expect(penLifts(CHINESE_NOT)).toBe(3);
    expect(CHINESE_NOT.strokes).toHaveLength(4);
    expect(CHINESE_NOT.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1, 1]);
    expect(CHINESE_NOT.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(
      CHINESE_NOT.strokes[0].segments[0].path[0].x,
    );
    expect(CHINESE_NOT.strokes[1].segments[0].path.at(-1)!.x).toBeLessThan(
      CHINESE_NOT.strokes[1].segments[0].path[0].x,
    );
  });

  it("Chinese 名 completes 夕 before 口 and preserves both joined turns", () => {
    expect(CHINESE_NAME.script).toBe("chinese");
    expect(penLifts(CHINESE_NAME)).toBe(5);
    expect(CHINESE_NAME.strokes).toHaveLength(6);
    expect(CHINESE_NAME.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1, 1, 2, 1]);
    expect(CHINESE_NAME.strokes[1].segments[0].path.at(-1)).toEqual(CHINESE_NAME.strokes[1].segments[1].path[0]);
    expect(CHINESE_NAME.strokes[4].segments[0].path.at(-1)).toEqual(CHINESE_NAME.strokes[4].segments[1].path[0]);
  });

  it("Chinese 字 completes 宀 before 子 and preserves all three joined turns", () => {
    expect(CHINESE_CHARACTER.script).toBe("chinese");
    expect(penLifts(CHINESE_CHARACTER)).toBe(5);
    expect(CHINESE_CHARACTER.strokes).toHaveLength(6);
    expect(CHINESE_CHARACTER.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 2, 2, 2, 1]);
    for (const strokeIndex of [2, 3, 4]) {
      expect(CHINESE_CHARACTER.strokes[strokeIndex].segments[0].path.at(-1)).toEqual(
        CHINESE_CHARACTER.strokes[strokeIndex].segments[1].path[0],
      );
    }
  });

  it("Chinese 谢 completes 讠, 身, and 寸 in order and preserves all five joined turns", () => {
    expect(CHINESE_THANK.script).toBe("chinese");
    expect(penLifts(CHINESE_THANK)).toBe(11);
    expect(CHINESE_THANK.strokes).toHaveLength(12);
    expect(CHINESE_THANK.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 3, 1, 1, 3, 1, 1, 1, 1, 1, 2, 1,
    ]);
    for (const strokeIndex of [1, 4]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex].path.at(-1)).toEqual(
          CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex + 1].path[0],
        );
      }
    }
    expect(CHINESE_THANK.strokes[10].segments[0].path.at(-1)).toEqual(
      CHINESE_THANK.strokes[10].segments[1].path[0],
    );
  });

  it("Chinese 请 completes 讠 before 青 and preserves all four joined turns", () => {
    expect(CHINESE_PLEASE.script).toBe("chinese");
    expect(penLifts(CHINESE_PLEASE)).toBe(9);
    expect(CHINESE_PLEASE.strokes).toHaveLength(10);
    expect(CHINESE_PLEASE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 3, 1, 1, 1, 1, 1, 3, 1, 1,
    ]);
    for (const strokeIndex of [1, 7]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex].path.at(-1)).toEqual(
          CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex + 1].path[0],
        );
      }
    }
  });

  it("Chinese 再 closes last and preserves both turns inside the enclosing stroke", () => {
    expect(CHINESE_AGAIN.script).toBe("chinese");
    expect(penLifts(CHINESE_AGAIN)).toBe(5);
    expect(CHINESE_AGAIN.strokes).toHaveLength(6);
    expect(CHINESE_AGAIN.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 3, 1, 1, 1]);
    for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
      expect(CHINESE_AGAIN.strokes[2].segments[segmentIndex].path.at(-1)).toEqual(
        CHINESE_AGAIN.strokes[2].segments[segmentIndex + 1].path[0],
      );
    }
  });

  it("Chinese 见 completes its open frame before both lower runs", () => {
    expect(CHINESE_SEE.script).toBe("chinese");
    expect(penLifts(CHINESE_SEE)).toBe(3);
    expect(CHINESE_SEE.strokes).toHaveLength(4);
    expect(CHINESE_SEE.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1, 3]);
    for (const strokeIndex of [1, 3]) {
      const stroke = CHINESE_SEE.strokes[strokeIndex];
      for (let segmentIndex = 0; segmentIndex + 1 < stroke.segments.length; segmentIndex++) {
        expect(stroke.segments[segmentIndex].path.at(-1)).toEqual(
          stroke.segments[segmentIndex + 1].path[0],
        );
      }
    }
  });

  it("Chinese 什 completes 亻 before writing 十 in four separate strokes", () => {
    expect(CHINESE_WHAT.script).toBe("chinese");
    expect(penLifts(CHINESE_WHAT)).toBe(3);
    expect(CHINESE_WHAT.strokes).toHaveLength(4);
    expect(CHINESE_WHAT.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1, 1]);
    const leftStrokes = CHINESE_WHAT.strokes.slice(0, 2).flatMap(penPath);
    const rightStrokes = CHINESE_WHAT.strokes.slice(2).flatMap(penPath);
    expect(Math.max(...leftStrokes.map((point) => point.x))).toBeLessThan(
      Math.min(...rightStrokes.map((point) => point.x)),
    );
  });

  it("Chinese 么 keeps the second fall joined to its rightward base sweep", () => {
    expect(CHINESE_PARTICLE_ME.script).toBe("chinese");
    expect(penLifts(CHINESE_PARTICLE_ME)).toBe(2);
    expect(CHINESE_PARTICLE_ME.strokes).toHaveLength(3);
    expect(CHINESE_PARTICLE_ME.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 2, 1]);
    expect(CHINESE_PARTICLE_ME.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_PARTICLE_ME.strokes[1].segments[1].path[0],
    );
  });

  it("Chinese 早 completes 日 before the two strokes of 十", () => {
    expect(CHINESE_EARLY.script).toBe("chinese");
    expect(penLifts(CHINESE_EARLY)).toBe(5);
    expect(CHINESE_EARLY.strokes).toHaveLength(6);
    expect(CHINESE_EARLY.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 2, 1, 1, 1, 1,
    ]);
    expect(CHINESE_EARLY.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_EARLY.strokes[1].segments[1].path[0],
    );
    expect(Math.max(...penPath(CHINESE_EARLY.strokes[4]).map((point) => point.y))).toBeLessThan(
      Math.min(...penPath(CHINESE_EARLY.strokes[3]).map((point) => point.y)),
    );
  });

  it("Chinese 上 writes the vertical before its short and long horizontals", () => {
    expect(CHINESE_UP.script).toBe("chinese");
    expect(penLifts(CHINESE_UP)).toBe(2);
    expect(CHINESE_UP.strokes).toHaveLength(3);
    expect(CHINESE_UP.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1, 1]);
    const vertical = penPath(CHINESE_UP.strokes[0]);
    const shortHorizontal = penPath(CHINESE_UP.strokes[1]);
    const base = penPath(CHINESE_UP.strokes[2]);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(shortHorizontal[0].x).toBeLessThan(shortHorizontal.at(-1)!.x);
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
    expect(base.at(-1)!.x - base[0].x).toBeGreaterThan(
      shortHorizontal.at(-1)!.x - shortHorizontal[0].x,
    );
  });

  it("Devanagari अ joins its left body before the shoulder, stem, and headline", () => {
    expect(DEVANAGARI_A.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_A)).toBe(3);
    expect(DEVANAGARI_A.strokes).toHaveLength(4);
    expect(DEVANAGARI_A.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1]);
    const upper = DEVANAGARI_A.strokes[0].segments[0].path;
    const lower = DEVANAGARI_A.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    const shoulder = penPath(DEVANAGARI_A.strokes[1]);
    const stem = penPath(DEVANAGARI_A.strokes[2]);
    const headline = penPath(DEVANAGARI_A.strokes[3]);
    expect(shoulder[0].x).toBeLessThan(shoulder.at(-1)!.x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari आ adds its trailing stem before spanning both stems with the headline", () => {
    expect(DEVANAGARI_AA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AA)).toBe(4);
    expect(DEVANAGARI_AA.strokes).toHaveLength(5);
    expect(DEVANAGARI_AA.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1, 1]);
    expect(DEVANAGARI_AA.strokes.slice(0, 3).map(penPath)).toEqual(
      DEVANAGARI_A.strokes.slice(0, 3).map(penPath),
    );
    const innerStem = penPath(DEVANAGARI_AA.strokes[2]);
    const trailingStem = penPath(DEVANAGARI_AA.strokes[3]);
    const headline = penPath(DEVANAGARI_AA.strokes[4]);
    expect(innerStem[0].y).toBeGreaterThan(innerStem.at(-1)!.y);
    expect(trailingStem[0].y).toBeGreaterThan(trailingStem.at(-1)!.y);
    expect(trailingStem[0].x).toBeGreaterThan(innerStem[0].x);
    expect(headline[0].x).toBeLessThan(innerStem[0].x);
    expect(headline.at(-1)!.x).toBeGreaterThan(trailingStem[0].x);
  });

  it("Devanagari इ keeps both bowls and the tail in one run before the headline", () => {
    expect(DEVANAGARI_I.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_I)).toBe(1);
    expect(DEVANAGARI_I.strokes).toHaveLength(2);
    expect(DEVANAGARI_I.strokes.map((stroke) => stroke.segments.length)).toEqual([4, 1]);
    const [upright, upper, lower, tail] = DEVANAGARI_I.strokes[0].segments.map((segment) => segment.path);
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(tail[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upright[0].x);
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(upper.at(-1)!.x);
    expect(tail.at(-1)!.x).toBeGreaterThan(tail[0].x);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
    const headline = penPath(DEVANAGARI_I.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ई reuses the continuous इ body before its upper curl and headline", () => {
    expect(DEVANAGARI_II.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_II)).toBe(2);
    expect(DEVANAGARI_II.strokes).toHaveLength(3);
    expect(DEVANAGARI_II.strokes.map((stroke) => stroke.segments.length)).toEqual([4, 1, 1]);
    expect(penPath(DEVANAGARI_II.strokes[0])).toEqual(penPath(DEVANAGARI_I.strokes[0]));
    const curl = penPath(DEVANAGARI_II.strokes[1]);
    expect(Math.max(...curl.map((point) => point.y))).toBeGreaterThan(curl[0].y);
    expect(Math.min(...curl.map((point) => point.x))).toBeLessThan(curl[0].x);
    expect(curl.at(-1)!.x).toBeGreaterThan(curl[0].x);
    const headline = penPath(DEVANAGARI_II.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari उ keeps its upper bowl and lower loop joined before the headline", () => {
    expect(DEVANAGARI_U.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_U)).toBe(1);
    expect(DEVANAGARI_U.strokes).toHaveLength(2);
    expect(DEVANAGARI_U.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const [upper, lower] = DEVANAGARI_U.strokes[0].segments.map((segment) => segment.path);
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(upper.at(-1)!.y);
    expect(Math.max(...upper.map((point) => point.x))).toBeGreaterThan(upper.at(-1)!.x);
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(lower[0].x);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(lower.at(-1)!.x).toBeLessThan(lower[0].x);
    const headline = penPath(DEVANAGARI_U.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ऊ reuses the continuous उ body before its right loop and headline", () => {
    expect(DEVANAGARI_UU.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_UU)).toBe(2);
    expect(DEVANAGARI_UU.strokes).toHaveLength(3);
    expect(DEVANAGARI_UU.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    expect(penPath(DEVANAGARI_UU.strokes[0])).toEqual(penPath(DEVANAGARI_U.strokes[0]));
    const loop = penPath(DEVANAGARI_UU.strokes[1]);
    expect(Math.max(...loop.map((point) => point.y))).toBeGreaterThan(loop[0].y);
    expect(Math.max(...loop.map((point) => point.x))).toBeGreaterThan(loop[0].x);
    expect(loop.at(-1)!.y).toBeLessThan(loop[0].y);
    expect(loop.at(-1)!.x).toBeGreaterThan(loop[0].x);
    const headline = penPath(DEVANAGARI_UU.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ए joins its long stem to the tail before the short stem and headline", () => {
    expect(DEVANAGARI_E.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_E)).toBe(2);
    expect(DEVANAGARI_E.strokes).toHaveLength(3);
    expect(DEVANAGARI_E.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    const [longStem, tail] = DEVANAGARI_E.strokes[0].segments.map((segment) => segment.path);
    expect(longStem[0].y).toBeGreaterThan(longStem.at(-1)!.y);
    expect(longStem.at(-1)).toEqual(tail[0]);
    expect(tail.at(-1)!.x).toBeGreaterThan(tail[0].x);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
    const shortStem = penPath(DEVANAGARI_E.strokes[1]);
    expect(shortStem[0].y).toBeGreaterThan(shortStem.at(-1)!.y);
    expect(shortStem.at(-1)!.x).toBeLessThan(shortStem[0].x);
    const headline = penPath(DEVANAGARI_E.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ऐ reuses the ए base before its upper arc and headline", () => {
    expect(DEVANAGARI_AI.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AI)).toBe(3);
    expect(DEVANAGARI_AI.strokes).toHaveLength(4);
    expect(DEVANAGARI_AI.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1]);
    expect(penPath(DEVANAGARI_AI.strokes[0])).toEqual(penPath(DEVANAGARI_E.strokes[0]));
    expect(penPath(DEVANAGARI_AI.strokes[1])).toEqual(penPath(DEVANAGARI_E.strokes[1]));
    const arc = penPath(DEVANAGARI_AI.strokes[2]);
    expect(arc.at(-1)!.x).toBeLessThan(arc[0].x);
    expect(Math.max(...arc.map((point) => point.y))).toBeGreaterThan(arc[0].y);
    const headline = penPath(DEVANAGARI_AI.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ओ reuses the आ base before its upper arc and headline", () => {
    expect(DEVANAGARI_O.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_O)).toBe(5);
    expect(DEVANAGARI_O.strokes).toHaveLength(6);
    expect(DEVANAGARI_O.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1, 1, 1, 1]);
    expect(DEVANAGARI_O.strokes.slice(0, 4).map(penPath)).toEqual(
      DEVANAGARI_AA.strokes.slice(0, 4).map(penPath),
    );
    const arc = penPath(DEVANAGARI_O.strokes[4]);
    expect(arc.at(-1)!.x).toBeLessThan(arc[0].x);
    expect(Math.max(...arc.map((point) => point.y))).toBeGreaterThan(arc[0].y);
    const headline = penPath(DEVANAGARI_O.strokes[5]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari औ reuses the आ base before its two upper arcs and headline", () => {
    expect(DEVANAGARI_AU.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AU)).toBe(6);
    expect(DEVANAGARI_AU.strokes).toHaveLength(7);
    expect(DEVANAGARI_AU.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1, 1, 1, 1, 1,
    ]);
    expect(DEVANAGARI_AU.strokes.slice(0, 4).map(penPath)).toEqual(
      DEVANAGARI_AA.strokes.slice(0, 4).map(penPath),
    );
    const lowerArc = penPath(DEVANAGARI_AU.strokes[4]);
    const tallerArc = penPath(DEVANAGARI_AU.strokes[5]);
    expect(lowerArc.at(-1)!.x).toBeLessThan(lowerArc[0].x);
    expect(tallerArc.at(-1)!.x).toBeLessThan(tallerArc[0].x);
    expect(Math.max(...tallerArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    const headline = penPath(DEVANAGARI_AU.strokes[6]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari क draws its bowl before the central stem, right arch, and headline", () => {
    expect(DEVANAGARI_KA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_KA)).toBe(3);
    expect(DEVANAGARI_KA.strokes).toHaveLength(4);
    expect(DEVANAGARI_KA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const bowl = penPath(DEVANAGARI_KA.strokes[0]);
    expect(Math.min(...bowl.map((point) => point.x))).toBeLessThan(bowl[0].x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    const stem = penPath(DEVANAGARI_KA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const arch = penPath(DEVANAGARI_KA.strokes[2]);
    expect(Math.max(...arch.map((point) => point.x))).toBeGreaterThan(arch[0].x);
    expect(arch[0].y).toBeGreaterThan(arch.at(-1)!.y);
    const headline = penPath(DEVANAGARI_KA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ग joins its loop to the ascending stem before the right stem and headline", () => {
    expect(DEVANAGARI_GA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_GA)).toBe(2);
    expect(DEVANAGARI_GA.strokes).toHaveLength(3);
    expect(DEVANAGARI_GA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_GA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.y).toBeGreaterThan(body[0].y);
    const stem = penPath(DEVANAGARI_GA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_GA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari च joins its short bar to the rounded body before the right stem and headline", () => {
    expect(DEVANAGARI_CA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_CA)).toBe(2);
    expect(DEVANAGARI_CA.strokes).toHaveLength(3);
    expect(DEVANAGARI_CA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_CA.strokes[0]);
    expect(body[0].x).toBeLessThan(body[5].x);
    expect(Math.min(...body.slice(6).map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[6].x);
    const stem = penPath(DEVANAGARI_CA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_CA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari त sweeps its shoulder right-to-left before the right stem and headline", () => {
    expect(DEVANAGARI_TA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_TA)).toBe(2);
    expect(DEVANAGARI_TA.strokes).toHaveLength(3);
    expect(DEVANAGARI_TA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_TA.strokes[0]);
    expect(body[0].x).toBeGreaterThan(body[3].x);
    expect(Math.min(...body.slice(4).map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(Math.min(...body.map((point) => point.x)));
    const stem = penPath(DEVANAGARI_TA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_TA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari द joins its outer body to the inner curl and tail after the short stem", () => {
    expect(DEVANAGARI_DA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DA)).toBe(2);
    expect(DEVANAGARI_DA.strokes).toHaveLength(3);
    expect(DEVANAGARI_DA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const stem = penPath(DEVANAGARI_DA.strokes[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const body = penPath(DEVANAGARI_DA.strokes[1]);
    expect(body[0]).toEqual(stem.at(-1));
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[14].x);
    expect(body.at(-1)!.y).toBeLessThan(body[14].y);
    const headline = penPath(DEVANAGARI_DA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ध separates its upper spiral, lower bowl, right stem, and headline", () => {
    expect(DEVANAGARI_DHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DHA)).toBe(3);
    expect(DEVANAGARI_DHA.strokes).toHaveLength(4);
    expect(DEVANAGARI_DHA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const spiral = penPath(DEVANAGARI_DHA.strokes[0]);
    expect(Math.max(...spiral.map((point) => point.y))).toBeGreaterThan(spiral[0].y);
    expect(spiral.at(-1)!.x).toBeGreaterThan(spiral[0].x);
    const bowl = penPath(DEVANAGARI_DHA.strokes[1]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
    const stem = penPath(DEVANAGARI_DHA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_DHA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari न keeps its clockwise loop joined to the rightward shoulder", () => {
    expect(DEVANAGARI_NA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_NA)).toBe(2);
    expect(DEVANAGARI_NA.strokes).toHaveLength(3);
    expect(DEVANAGARI_NA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_NA.strokes[0]);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    const stem = penPath(DEVANAGARI_NA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_NA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari प joins its descending left stem to the lower bowl", () => {
    expect(DEVANAGARI_PA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_PA)).toBe(2);
    expect(DEVANAGARI_PA.strokes).toHaveLength(3);
    expect(DEVANAGARI_PA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_PA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[3].y);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeGreaterThan(Math.min(...body.map((point) => point.y)));
    const stem = penPath(DEVANAGARI_PA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_PA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ब separates its counterclockwise oval, right stem, inner diagonal, and headline", () => {
    expect(DEVANAGARI_BA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_BA)).toBe(3);
    expect(DEVANAGARI_BA.strokes).toHaveLength(4);
    expect(DEVANAGARI_BA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_BA.strokes[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(Math.min(...body.map((point) => point.x)));
    const stem = penPath(DEVANAGARI_BA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const diagonal = penPath(DEVANAGARI_BA.strokes[2]);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
    const headline = penPath(DEVANAGARI_BA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari भ joins both clockwise loops and the crossbar before the lifted right stem", () => {
    expect(DEVANAGARI_BHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_BHA)).toBe(2);
    expect(DEVANAGARI_BHA.strokes).toHaveLength(3);
    expect(DEVANAGARI_BHA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_BHA.strokes[0]);
    expect(Math.min(...body.slice(0, 14).map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.slice(0, 14).map((point) => point.y))).toBeGreaterThan(body[0].y);
    expect(Math.max(...body.slice(0, 14).map((point) => point.x))).toBeGreaterThan(body[0].x);
    expect(body[17].y).toBeLessThan(body[14].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[17].x);
    const stem = penPath(DEVANAGARI_BHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_BHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari म joins its descending left stem to the clockwise lower loop and crossbar", () => {
    expect(DEVANAGARI_MA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_MA)).toBe(2);
    expect(DEVANAGARI_MA.strokes).toHaveLength(3);
    expect(DEVANAGARI_MA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_MA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[5].y);
    expect(Math.min(...body.slice(5, 15).map((point) => point.x))).toBeLessThan(body[5].x);
    expect(Math.min(...body.slice(5, 15).map((point) => point.y))).toBeLessThan(body[5].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[5].x);
    const stem = penPath(DEVANAGARI_MA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_MA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari य separates its clockwise inner curl from the lower bowl", () => {
    expect(DEVANAGARI_YA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_YA)).toBe(3);
    expect(DEVANAGARI_YA.strokes).toHaveLength(4);
    expect(DEVANAGARI_YA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const curl = penPath(DEVANAGARI_YA.strokes[0]);
    expect(Math.max(...curl.map((point) => point.x))).toBeGreaterThan(curl[0].x);
    expect(curl.at(-1)!.x).toBeLessThan(curl[0].x);
    expect(curl.at(-1)!.y).toBeLessThan(curl[0].y);
    const bowl = penPath(DEVANAGARI_YA.strokes[1]);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    const stem = penPath(DEVANAGARI_YA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_YA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari र separates its looped stem from the diagonal tail", () => {
    expect(DEVANAGARI_RA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_RA)).toBe(2);
    expect(DEVANAGARI_RA.strokes).toHaveLength(3);
    expect(DEVANAGARI_RA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const loop = penPath(DEVANAGARI_RA.strokes[0]);
    expect(loop[0].y).toBeGreaterThan(loop[7].y);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[7].x);
    expect(Math.max(...loop.slice(8).map((point) => point.y))).toBeGreaterThan(loop[7].y);
    const tail = penPath(DEVANAGARI_RA.strokes[1]);
    expect(tail[0].x).toBeLessThan(tail.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
    const headline = penPath(DEVANAGARI_RA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ल draws its clockwise open loop before the diagonal arm", () => {
    expect(DEVANAGARI_LA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_LA)).toBe(3);
    expect(DEVANAGARI_LA.strokes).toHaveLength(4);
    expect(DEVANAGARI_LA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const loop = penPath(DEVANAGARI_LA.strokes[0]);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[0].x);
    expect(Math.max(...loop.map((point) => point.y))).toBeGreaterThan(loop[0].y);
    expect(loop.at(-1)!.x).toBeGreaterThan(Math.min(...loop.map((point) => point.x)));
    const arm = penPath(DEVANAGARI_LA.strokes[1]);
    expect(arm[0].x).toBeLessThan(arm.at(-1)!.x);
    expect(arm[0].y).toBeLessThan(arm.at(-1)!.y);
    const stem = penPath(DEVANAGARI_LA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_LA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari व circles counterclockwise before the right stem", () => {
    expect(DEVANAGARI_VA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_VA)).toBe(2);
    expect(DEVANAGARI_VA.strokes).toHaveLength(3);
    expect(DEVANAGARI_VA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const loop = penPath(DEVANAGARI_VA.strokes[0]);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[0].x);
    expect(Math.min(...loop.map((point) => point.y))).toBeLessThan(loop[0].y);
    expect(loop.at(-1)!.x).toBeGreaterThan(Math.min(...loop.map((point) => point.x)));
    const stem = penPath(DEVANAGARI_VA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_VA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari श joins both loops and its tail before the right stem", () => {
    expect(DEVANAGARI_SHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_SHA)).toBe(2);
    expect(DEVANAGARI_SHA.strokes).toHaveLength(3);
    expect(DEVANAGARI_SHA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_SHA.strokes[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    const stem = penPath(DEVANAGARI_SHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_SHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari स joins its hook and tail before the middle crossbar", () => {
    expect(DEVANAGARI_SA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_SA)).toBe(3);
    expect(DEVANAGARI_SA.strokes).toHaveLength(4);
    expect(DEVANAGARI_SA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_SA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(Math.min(...body.map((point) => point.x)));
    const crossbar = penPath(DEVANAGARI_SA.strokes[1]);
    expect(crossbar[0].x).toBeLessThan(crossbar.at(-1)!.x);
    const stem = penPath(DEVANAGARI_SA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_SA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ह joins the stem, shoulder, and hooked body before the outer tail", () => {
    expect(DEVANAGARI_HA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_HA)).toBe(2);
    expect(DEVANAGARI_HA.strokes).toHaveLength(3);
    expect(DEVANAGARI_HA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const body = penPath(DEVANAGARI_HA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(Math.min(...body.map((point) => point.x)));
    const tail = penPath(DEVANAGARI_HA.strokes[1]);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
    expect(Math.min(...tail.map((point) => point.x))).toBeLessThan(tail[0].x);
    expect(tail.at(-1)!.x).toBeGreaterThan(Math.min(...tail.map((point) => point.x)));
    const headline = penPath(DEVANAGARI_HA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Cyrillic а keeps its shoulder, round body, and finishing stem in one run", () => {
    expect(CYRILLIC_A.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_A)).toBe(0);
    expect(CYRILLIC_A.strokes).toHaveLength(1);
    expect(CYRILLIC_A.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_A.strokes[0].segments[0].path;
    const stem = CYRILLIC_A.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(stem[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("Cyrillic б closes its lower body before rising into the top flag", () => {
    expect(CYRILLIC_BE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_BE)).toBe(0);
    expect(CYRILLIC_BE.strokes).toHaveLength(1);
    expect(CYRILLIC_BE.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_BE.strokes[0].segments[0].path;
    const flag = CYRILLIC_BE.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(flag[0]);
    expect(body.at(-1)).toEqual(body[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(flag.at(-1)!.y).toBeGreaterThan(flag[0].y);
    expect(flag.at(-1)!.x).toBeGreaterThan(flag[0].x);
  });

  it("Cyrillic в returns from its upper loop before circling the lower bowl", () => {
    expect(CYRILLIC_VE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_VE)).toBe(0);
    expect(CYRILLIC_VE.strokes).toHaveLength(1);
    expect(CYRILLIC_VE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_VE.strokes[0].segments[0].path;
    const lower = CYRILLIC_VE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(upper.at(-1)).toEqual(upper[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(upper[0].y);
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(lower[0].x);
    expect(lower.at(-1)!.y).toBeGreaterThan(lower[0].y);
  });

  it("Cyrillic г climbs, retraces its top bar, and descends without lifting", () => {
    expect(CYRILLIC_GE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_GE)).toBe(0);
    expect(CYRILLIC_GE.strokes).toHaveLength(1);
    expect(CYRILLIC_GE.strokes[0].segments).toHaveLength(2);
    const outward = CYRILLIC_GE.strokes[0].segments[0].path;
    const returnPath = CYRILLIC_GE.strokes[0].segments[1].path;
    expect(outward.at(-1)).toEqual(returnPath[0]);
    expect(outward[0].y).toBeLessThan(outward.at(-1)!.y);
    expect(outward[0].x).toBeLessThan(outward.at(-1)!.x);
    expect(returnPath.at(-1)).toEqual(outward[0]);
  });

  it("Cyrillic д closes its body before retracing both feet without lifting", () => {
    expect(CYRILLIC_DE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_DE)).toBe(0);
    expect(CYRILLIC_DE.strokes).toHaveLength(1);
    expect(CYRILLIC_DE.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_DE.strokes[0].segments[0].path;
    const base = CYRILLIC_DE.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(base[0]);
    expect(body.at(-1)).toEqual(body[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...base.map((point) => point.y))).toBeLessThan(
      Math.min(...body.map((point) => point.y)),
    );
    expect(Math.min(...base.map((point) => point.x))).toBeLessThan(body[0].x);
  });

  it("Cyrillic е crosses the middle before continuing around its lower bowl", () => {
    expect(CYRILLIC_IE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_IE)).toBe(0);
    expect(CYRILLIC_IE.strokes).toHaveLength(1);
    expect(CYRILLIC_IE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_IE.strokes[0].segments[0].path;
    const lower = CYRILLIC_IE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(upper[0].y);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(
      Math.min(...upper.map((point) => point.y)),
    );
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x - 100);
  });

  it("Cyrillic ё completes its body before two separately lifted dots", () => {
    expect(CYRILLIC_IO.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_IO)).toBe(2);
    expect(CYRILLIC_IO.strokes).toHaveLength(3);
    expect(CYRILLIC_IO.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    expect(penPath(CYRILLIC_IO.strokes[0])).toEqual(penPath(CYRILLIC_IE.strokes[0]));
    const leftDot = penPath(CYRILLIC_IO.strokes[1]);
    const rightDot = penPath(CYRILLIC_IO.strokes[2]);
    expect(leftDot[0].x).toBeLessThan(rightDot[0].x);
    expect(leftDot[0].y).toBe(rightDot[0].y);
    expect(leftDot[0].y).toBeGreaterThan(
      Math.max(...penPath(CYRILLIC_IO.strokes[0]).map((point) => point.y)),
    );
  });

  it("Cyrillic ж traces both wings through one continuous central run", () => {
    expect(CYRILLIC_ZHE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ZHE)).toBe(0);
    expect(CYRILLIC_ZHE.strokes).toHaveLength(1);
    expect(CYRILLIC_ZHE.strokes[0].segments).toHaveLength(2);
    const left = CYRILLIC_ZHE.strokes[0].segments[0].path;
    const right = CYRILLIC_ZHE.strokes[0].segments[1].path;
    expect(left.at(-1)).toEqual(right[0]);
    expect(Math.min(...left.map((point) => point.x))).toBeLessThan(right[0].x);
    expect(Math.max(...right.map((point) => point.x))).toBeGreaterThan(left.at(-1)!.x);
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(right.at(-1)!.y).toBeLessThan(right[0].y);
  });

  it("Cyrillic з joins its smaller upper lobe to its larger lower lobe", () => {
    expect(CYRILLIC_ZE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ZE)).toBe(0);
    expect(CYRILLIC_ZE.strokes).toHaveLength(1);
    expect(CYRILLIC_ZE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_ZE.strokes[0].segments[0].path;
    const lower = CYRILLIC_ZE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lower.map((point) => point.y)),
    );
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(
      Math.min(...upper.map((point) => point.y)),
    );
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(lower[0].x);
  });

  it("Cyrillic и joins its two descending stems with a rising diagonal", () => {
    expect(CYRILLIC_I.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_I)).toBe(0);
    expect(CYRILLIC_I.strokes).toHaveLength(1);
    expect(CYRILLIC_I.strokes[0].segments).toHaveLength(3);
    const left = CYRILLIC_I.strokes[0].segments[0].path;
    const diagonal = CYRILLIC_I.strokes[0].segments[1].path;
    const right = CYRILLIC_I.strokes[0].segments[2].path;
    expect(left.at(-1)).toEqual(diagonal[0]);
    expect(diagonal.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(diagonal[0].y).toBeLessThan(diagonal.at(-1)!.y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
  });

  it("Cyrillic й completes its joined body before the lifted breve", () => {
    expect(CYRILLIC_SHORT_I.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHORT_I)).toBe(1);
    expect(CYRILLIC_SHORT_I.strokes).toHaveLength(2);
    expect(CYRILLIC_SHORT_I.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1]);
    const body = CYRILLIC_SHORT_I.strokes[0].segments;
    expect(body[0].path.at(-1)).toEqual(body[1].path[0]);
    expect(body[1].path.at(-1)).toEqual(body[2].path[0]);
    const breve = CYRILLIC_SHORT_I.strokes[1].segments[0].path;
    expect(breve[0].x).toBeLessThan(breve.at(-1)!.x);
    expect(Math.min(...breve.map((point) => point.y))).toBeLessThan(breve[0].y);
    expect(Math.min(...breve.map((point) => point.y))).toBeLessThan(breve.at(-1)!.y);
  });

  it("Cyrillic к joins its descending stem to the upper and lower arms", () => {
    expect(CYRILLIC_KA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_KA)).toBe(0);
    expect(CYRILLIC_KA.strokes).toHaveLength(1);
    expect(CYRILLIC_KA.strokes[0].segments).toHaveLength(3);
    const [stem, upper, lower] = CYRILLIC_KA.strokes[0].segments.map((segment) => segment.path);
    expect(stem.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(upper.at(-1)!.y);
    expect(Math.max(...upper.map((point) => point.x))).toBeGreaterThan(upper.at(-1)!.x);
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x);
    expect(lower.at(-1)!.y).toBeLessThan(lower[0].y);
  });

  it("Cyrillic л joins its hooked left leg to the top shoulder and right stem", () => {
    expect(CYRILLIC_EL.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EL)).toBe(0);
    expect(CYRILLIC_EL.strokes).toHaveLength(1);
    expect(CYRILLIC_EL.strokes[0].segments).toHaveLength(3);
    const [left, shoulder, right] = CYRILLIC_EL.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(left[0].x).toBeLessThan(left.at(-1)!.x);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic м joins its two apexes through the central valley", () => {
    expect(CYRILLIC_EM.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EM)).toBe(0);
    expect(CYRILLIC_EM.strokes).toHaveLength(1);
    expect(CYRILLIC_EM.strokes[0].segments).toHaveLength(4);
    const [left, down, up, right] = CYRILLIC_EM.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(down[0]);
    expect(down.at(-1)).toEqual(up[0]);
    expect(up.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(down.at(-1)!.y).toBeLessThan(down[0].y);
    expect(up.at(-1)!.y).toBeGreaterThan(up[0].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic н joins its two stems through the middle bridge", () => {
    expect(CYRILLIC_EN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EN)).toBe(0);
    expect(CYRILLIC_EN.strokes).toHaveLength(1);
    expect(CYRILLIC_EN.strokes[0].segments).toHaveLength(3);
    const [left, bridge, right] = CYRILLIC_EN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(bridge[0]);
    expect(bridge.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(Math.min(...bridge.map((point) => point.y))).toBe(left.at(-1)!.y);
    expect(bridge.some((point) => point.x > left[0].x && point.y === 274)).toBe(true);
    expect(bridge.at(-1)!.y).toBeGreaterThan(bridge[0].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic о closes one counterclockwise oval without lifting", () => {
    expect(CYRILLIC_O.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_O)).toBe(0);
    expect(CYRILLIC_O.strokes).toHaveLength(1);
    expect(CYRILLIC_O.strokes[0].segments).toHaveLength(2);
    const [left, right] = CYRILLIC_O.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(left[0]);
    expect(Math.min(...left.map((point) => point.x))).toBeLessThan(left[0].x);
    expect(Math.max(...left.map((point) => point.y))).toBeGreaterThan(left[0].y);
    expect(Math.min(...left.map((point) => point.y))).toBeLessThan(left[0].y);
    expect(Math.max(...right.map((point) => point.x))).toBeGreaterThan(left[0].x);
  });

  it("Cyrillic п joins its two stems through the top shoulder", () => {
    expect(CYRILLIC_PE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_PE)).toBe(0);
    expect(CYRILLIC_PE.strokes).toHaveLength(1);
    expect(CYRILLIC_PE.strokes[0].segments).toHaveLength(3);
    const [left, shoulder, right] = CYRILLIC_PE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(shoulder.at(-1)!.y).toBe(shoulder[4].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic р joins its descender to one clockwise printed bowl", () => {
    expect(CYRILLIC_ER.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ER)).toBe(0);
    expect(CYRILLIC_ER.strokes).toHaveLength(1);
    expect(CYRILLIC_ER.strokes[0].segments).toHaveLength(3);
    const [stem, shoulder, bowl] = CYRILLIC_ER.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(bowl[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.max(...shoulder.map((point) => point.y))).toBeGreaterThan(
      shoulder[0].y,
    );
    expect(Math.max(...bowl.map((point) => point.x))).toBeGreaterThan(
      bowl.at(-1)!.x,
    );
    expect(bowl.at(-1)!.x).toBe(stem[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(0);
  });

  it("Cyrillic с keeps one counterclockwise curve open on the right", () => {
    expect(CYRILLIC_ES.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ES)).toBe(0);
    expect(CYRILLIC_ES.strokes).toHaveLength(1);
    expect(CYRILLIC_ES.strokes[0].segments).toHaveLength(2);
    const [upper, lower] = CYRILLIC_ES.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(upper[0].x).toBeGreaterThan(Math.min(...upper.map((point) => point.x)));
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(upper[0].y);
    expect(lower.at(-1)!.x).toBeGreaterThan(Math.min(...lower.map((point) => point.x)));
    expect(lower.at(-1)!.y).toBeLessThan(upper[0].y);
    expect(lower.at(-1)).not.toEqual(upper[0]);
  });

  it("Cyrillic т joins its central stem to the full printed top bar", () => {
    expect(CYRILLIC_TE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_TE)).toBe(0);
    expect(CYRILLIC_TE.strokes).toHaveLength(1);
    expect(CYRILLIC_TE.strokes[0].segments).toHaveLength(3);
    const [stem, left, right] = CYRILLIC_TE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(left[0]);
    expect(left.at(-1)).toEqual(right[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(left.at(-1)!.x).toBeLessThan(stem[0].x);
    expect(right.at(-1)!.x).toBeGreaterThan(stem[0].x);
    expect(Math.max(...right.map((point) => point.y))).toBe(
      Math.min(...right.map((point) => point.y)),
    );
  });

  it("Cyrillic у joins both upper arms to its long curved descender", () => {
    expect(CYRILLIC_U.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_U)).toBe(0);
    expect(CYRILLIC_U.strokes).toHaveLength(1);
    expect(CYRILLIC_U.strokes[0].segments).toHaveLength(4);
    const [left, right, tail, terminal] = CYRILLIC_U.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(tail[0]);
    expect(tail.at(-1)).toEqual(terminal[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
    expect(terminal.at(-1)!.x).toBeLessThan(terminal[0].x);
  });

  it("Cyrillic ф draws the central stem before its joined printed bowls", () => {
    expect(CYRILLIC_EF.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EF)).toBe(1);
    expect(CYRILLIC_EF.strokes).toHaveLength(2);
    expect(CYRILLIC_EF.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 4]);
    const stem = CYRILLIC_EF.strokes[0].segments[0].path;
    const [leftTop, leftBottom, rightBottom, rightTop] = CYRILLIC_EF.strokes[1].segments.map(
      (segment) => segment.path,
    );
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(leftTop.at(-1)).toEqual(leftBottom[0]);
    expect(leftBottom.at(-1)).toEqual(rightBottom[0]);
    expect(rightBottom.at(-1)).toEqual(rightTop[0]);
    expect(Math.min(...leftTop.map((point) => point.x))).toBeLessThan(stem[0].x);
    expect(Math.max(...rightTop.map((point) => point.x))).toBeGreaterThan(stem[0].x);
  });

  it("Cyrillic х draws the left curved run before the crossing right run", () => {
    expect(CYRILLIC_HA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_HA)).toBe(1);
    expect(CYRILLIC_HA.strokes).toHaveLength(2);
    expect(CYRILLIC_HA.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 2]);
    const [leftTop, leftBottom] = CYRILLIC_HA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const [rightTop, rightBottom] = CYRILLIC_HA.strokes[1].segments.map(
      (segment) => segment.path,
    );
    expect(leftTop.at(-1)).toEqual(leftBottom[0]);
    expect(rightTop.at(-1)).toEqual(rightBottom[0]);
    expect(leftTop.at(-1)).toEqual(rightTop.at(-1));
    expect(leftTop[0].x).toBeLessThan(rightTop[0].x);
    expect(leftBottom.at(-1)!.x).toBeLessThan(rightBottom.at(-1)!.x);
  });

  it("Cyrillic ц keeps both stems, the base, and the tail in one run", () => {
    expect(CYRILLIC_TSE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_TSE)).toBe(0);
    expect(CYRILLIC_TSE.strokes).toHaveLength(1);
    expect(CYRILLIC_TSE.strokes[0].segments).toHaveLength(4);
    const [left, rise, retrace, tail] = CYRILLIC_TSE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(rise[0]);
    expect(rise.at(-1)).toEqual(retrace[0]);
    expect(retrace.at(-1)).toEqual(tail[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(rise.at(-1)!.y).toBeGreaterThan(rise[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ч keeps the short stem, bowl, and full stem in one run", () => {
    expect(CYRILLIC_CHE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_CHE)).toBe(0);
    expect(CYRILLIC_CHE.strokes).toHaveLength(1);
    expect(CYRILLIC_CHE.strokes[0].segments).toHaveLength(3);
    const [left, bowl, right] = CYRILLIC_CHE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(bowl[0]);
    expect(bowl.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
    expect(right.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ш keeps three stems and two base joins in one run", () => {
    expect(CYRILLIC_SHA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHA)).toBe(0);
    expect(CYRILLIC_SHA.strokes).toHaveLength(1);
    expect(CYRILLIC_SHA.strokes[0].segments).toHaveLength(5);
    const [left, firstRise, middle, secondRise, right] = CYRILLIC_SHA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(firstRise[0]);
    expect(firstRise.at(-1)).toEqual(middle[0]);
    expect(middle.at(-1)).toEqual(secondRise[0]);
    expect(secondRise.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(firstRise.at(-1)!.y).toBeGreaterThan(firstRise[0].y);
    expect(secondRise.at(-1)!.y).toBeGreaterThan(secondRise[0].y);
    expect(right.at(-1)!.y).toBeLessThan(right[0].y);
  });

  it("Cyrillic щ keeps three stems, two joins, and the tail in one run", () => {
    expect(CYRILLIC_SHCHA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHCHA)).toBe(0);
    expect(CYRILLIC_SHCHA.strokes).toHaveLength(1);
    expect(CYRILLIC_SHCHA.strokes[0].segments).toHaveLength(6);
    const [left, firstRise, middle, secondRise, right, tail] =
      CYRILLIC_SHCHA.strokes[0].segments.map((segment) => segment.path);
    expect(left.at(-1)).toEqual(firstRise[0]);
    expect(firstRise.at(-1)).toEqual(middle[0]);
    expect(middle.at(-1)).toEqual(secondRise[0]);
    expect(secondRise.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(tail[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(firstRise.at(-1)!.y).toBeGreaterThan(firstRise[0].y);
    expect(secondRise.at(-1)!.y).toBeGreaterThan(secondRise[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ъ keeps its flag, stem, and lower bowl in one run", () => {
    expect(CYRILLIC_HARD_SIGN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_HARD_SIGN)).toBe(0);
    expect(CYRILLIC_HARD_SIGN.strokes).toHaveLength(1);
    expect(CYRILLIC_HARD_SIGN.strokes[0].segments).toHaveLength(5);
    const [flag, stem, lower, right, upper] = CYRILLIC_HARD_SIGN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(flag.at(-1)).toEqual(stem[0]);
    expect(stem.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(flag[0].x).toBeLessThan(flag.at(-1)!.x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
  });

  it("Cyrillic ы keeps the left stem and bowl joined before the lifted right stem", () => {
    expect(CYRILLIC_YERY.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YERY)).toBe(1);
    expect(CYRILLIC_YERY.strokes).toHaveLength(2);
    expect(CYRILLIC_YERY.strokes.map((stroke) => stroke.segments.length)).toEqual([4, 1]);
    const [left, lower, right, upper] = CYRILLIC_YERY.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const separate = CYRILLIC_YERY.strokes[1].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
    expect(separate[0].y).toBeGreaterThan(separate.at(-1)!.y);
    expect(separate[0].x).toBeGreaterThan(right.at(-1)!.x);
  });

  it("Cyrillic ь keeps the stem and lower bowl in one continuous run", () => {
    expect(CYRILLIC_SOFT_SIGN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SOFT_SIGN)).toBe(0);
    expect(CYRILLIC_SOFT_SIGN.strokes).toHaveLength(1);
    expect(CYRILLIC_SOFT_SIGN.strokes[0].segments).toHaveLength(4);
    const [stem, lower, right, upper] = CYRILLIC_SOFT_SIGN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
  });

  it("Cyrillic э draws the outer curve before the lifted right-to-left tongue", () => {
    expect(CYRILLIC_E.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_E)).toBe(1);
    expect(CYRILLIC_E.strokes).toHaveLength(2);
    expect(CYRILLIC_E.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1]);
    const [upper, right, lower] = CYRILLIC_E.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const tongue = CYRILLIC_E.strokes[1].segments[0].path;
    expect(upper.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(lower[0]);
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(lower.at(-1)!.x).toBeLessThan(lower[0].x);
    expect(tongue[0].x).toBeGreaterThan(tongue.at(-1)!.x);
  });

  it("Cyrillic ю joins the left stem and middle bar to the clockwise oval", () => {
    expect(CYRILLIC_YU.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YU)).toBe(0);
    expect(CYRILLIC_YU.strokes).toHaveLength(1);
    expect(CYRILLIC_YU.strokes[0].segments).toHaveLength(5);
    const [stem, connector, upper, right, lower] = CYRILLIC_YU.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(connector[0]);
    expect(connector.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(upper[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(connector.at(-1)!.x).toBeGreaterThan(connector[0].x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic я joins the rising stem, counterclockwise bowl, and diagonal leg", () => {
    expect(CYRILLIC_YA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YA)).toBe(0);
    expect(CYRILLIC_YA.strokes).toHaveLength(1);
    expect(CYRILLIC_YA.strokes[0].segments).toHaveLength(4);
    const [stem, bowl, join, leg] = CYRILLIC_YA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(bowl[0]);
    expect(bowl.at(-1)).toEqual(join[0]);
    expect(join.at(-1)).toEqual(leg[0]);
    expect(stem.at(-1)!.y).toBeGreaterThan(stem[0].y);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(leg.at(-1)!.x).toBeLessThan(leg[0].x);
    expect(leg.at(-1)!.y).toBeLessThan(leg[0].y);
  });

  it("Gujarati અ draws the joined body before the lifted right stem", () => {
    expect(GUJARATI_A.script).toBe("gujarati");
    expect(penLifts(GUJARATI_A)).toBe(1);
    expect(GUJARATI_A.strokes).toHaveLength(2);
    expect(GUJARATI_A.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1]);
    const [left, lower, arch] = GUJARATI_A.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const stem = GUJARATI_A.strokes[1].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(arch[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(stem.at(-1)!.x).toBeGreaterThan(stem[0].x);
  });

  it("Gujarati આ adds a second lifted stem after the complete અ body", () => {
    expect(GUJARATI_AA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AA)).toBe(2);
    expect(GUJARATI_AA.strokes).toHaveLength(3);
    expect(GUJARATI_AA.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1, 1]);
    const [left, lower, arch] = GUJARATI_AA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const firstStem = GUJARATI_AA.strokes[1].segments[0].path;
    const trailingStem = GUJARATI_AA.strokes[2].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(arch[0]);
    expect(firstStem[0].y).toBeGreaterThan(firstStem.at(-1)!.y);
    expect(trailingStem[0].y).toBeGreaterThan(trailingStem.at(-1)!.y);
    expect(trailingStem[0].x).toBeGreaterThan(firstStem[0].x);
  });

  it("Gujarati ઇ keeps both loops and the rising hook in one pen-down run", () => {
    expect(GUJARATI_I.script).toBe("gujarati");
    expect(penLifts(GUJARATI_I)).toBe(0);
    expect(GUJARATI_I.strokes).toHaveLength(1);
    expect(GUJARATI_I.strokes[0].segments).toHaveLength(4);
    const [upper, crossing, lower, hook] = GUJARATI_I.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(crossing[0]);
    expect(crossing.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(hook[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(hook.at(-1)!.y).toBeGreaterThan(hook[0].y);
  });

  it("Gujarati ઈ extends the zero-lift ઇ run into a high clockwise curl", () => {
    expect(GUJARATI_II.script).toBe("gujarati");
    expect(penLifts(GUJARATI_II)).toBe(0);
    expect(GUJARATI_II.strokes).toHaveLength(1);
    expect(GUJARATI_II.strokes[0].segments).toHaveLength(4);
    const [upper, crossing, lower, curl] = GUJARATI_II.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(crossing[0]);
    expect(crossing.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(curl[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.max(...curl.map((point) => point.y))).toBeGreaterThan(upper[0].y);
    expect(curl.at(-1)!.x).toBeGreaterThan(curl[0].x);
  });

  it("Gujarati ઉ joins both bowls to its tall returning outer curve", () => {
    expect(GUJARATI_U.script).toBe("gujarati");
    expect(penLifts(GUJARATI_U)).toBe(0);
    expect(GUJARATI_U.strokes).toHaveLength(1);
    expect(GUJARATI_U.strokes[0].segments).toHaveLength(3);
    const [upper, lower, outer] = GUJARATI_U.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(outer[0]);
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(upper.at(-1)!.x);
    expect(Math.min(...outer.map((point) => point.x))).toBeLessThan(lower.at(-1)!.x);
    expect(outer.at(-1)!.y).toBeGreaterThan(upper[0].y);
  });

  it("Gujarati ઊ extends the zero-lift ઉ run down a long right tail", () => {
    expect(GUJARATI_UU.script).toBe("gujarati");
    expect(penLifts(GUJARATI_UU)).toBe(0);
    expect(GUJARATI_UU.strokes).toHaveLength(1);
    expect(GUJARATI_UU.strokes[0].segments).toHaveLength(3);
    const [body, outer, tail] = GUJARATI_UU.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(body.at(-1)).toEqual(outer[0]);
    expect(outer.at(-1)).toEqual(tail[0]);
    expect(Math.min(...outer.map((point) => point.x))).toBeLessThan(body.at(-1)!.x);
    expect(Math.max(...tail.map((point) => point.x))).toBeGreaterThan(outer.at(-1)!.x);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
  });

  it("Gujarati ઋ writes the bent body, central stem, then right loop and tail", () => {
    expect(GUJARATI_VOCALIC_R.script).toBe("gujarati");
    expect(penLifts(GUJARATI_VOCALIC_R)).toBe(2);
    expect(GUJARATI_VOCALIC_R.strokes).toHaveLength(3);
    expect(GUJARATI_VOCALIC_R.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_VOCALIC_R.strokes[0].segments[0].path;
    const stem = GUJARATI_VOCALIC_R.strokes[1].segments[0].path;
    const loopAndTail = GUJARATI_VOCALIC_R.strokes[2].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(Math.max(...body.map((point) => point.x)));
    expect(stem.at(-1)!.y).toBeLessThan(stem[0].y);
    expect(Math.max(...loopAndTail.map((point) => point.y))).toBeGreaterThan(loopAndTail[0].y);
    expect(loopAndTail.at(-1)!.y).toBeLessThan(loopAndTail[0].y);
  });

  it("Gujarati એ writes the joined body, right stem, then high arc", () => {
    expect(GUJARATI_E.script).toBe("gujarati");
    expect(penLifts(GUJARATI_E)).toBe(2);
    expect(GUJARATI_E.strokes).toHaveLength(3);
    expect(GUJARATI_E.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1, 1]);
    const [bowl, body] = GUJARATI_E.strokes[0].segments.map((segment) => segment.path);
    expect(bowl.at(-1)).toEqual(body[0]);
    expect(Math.min(...bowl.map((point) => point.x))).toBeLessThan(bowl[0].x);
    expect(GUJARATI_E.strokes[1].segments[0].path.at(-1)!.y).toBeLessThan(
      GUJARATI_E.strokes[1].segments[0].path[0].y,
    );
    expect(Math.max(...GUJARATI_E.strokes[2].segments[0].path.map((point) => point.y))).toBeGreaterThan(
      Math.max(...bowl.map((point) => point.y)),
    );
  });

  it("Gujarati ઐ extends એ with a second, higher arc", () => {
    expect(GUJARATI_AI.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AI)).toBe(3);
    expect(GUJARATI_AI.strokes).toHaveLength(4);
    expect(GUJARATI_AI.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const lowerArc = GUJARATI_AI.strokes[2].segments[0].path;
    const higherArc = GUJARATI_AI.strokes[3].segments[0].path;
    expect(Math.max(...higherArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    expect(lowerArc.at(-1)!.x).toBeGreaterThan(lowerArc[0].x);
    expect(higherArc.at(-1)!.x).toBeGreaterThan(higherArc[0].x);
  });

  it("Gujarati ઓ writes the complete આ sequence before its high arc", () => {
    expect(GUJARATI_O.script).toBe("gujarati");
    expect(penLifts(GUJARATI_O)).toBe(3);
    expect(GUJARATI_O.strokes).toHaveLength(4);
    expect(GUJARATI_O.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1, 1, 1]);
    const [left, body, arch] = GUJARATI_O.strokes[0].segments.map((segment) => segment.path);
    expect(left.at(-1)).toEqual(body[0]);
    expect(body.at(-1)).toEqual(arch[0]);
    expect(GUJARATI_O.strokes[2].segments[0].path[0].x).toBeGreaterThan(
      GUJARATI_O.strokes[1].segments[0].path[0].x,
    );
    const highArc = GUJARATI_O.strokes[3].segments[0].path;
    expect(highArc.at(-1)!.x).toBeGreaterThan(highArc[0].x);
    expect(Math.max(...highArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...left.map((point) => point.y)),
    );
  });

  it("Gujarati ઔ extends ઓ with a second, higher arc", () => {
    expect(GUJARATI_AU.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AU)).toBe(4);
    expect(GUJARATI_AU.strokes).toHaveLength(5);
    expect(GUJARATI_AU.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const lowerArc = GUJARATI_AU.strokes[3].segments[0].path;
    const higherArc = GUJARATI_AU.strokes[4].segments[0].path;
    expect(Math.max(...higherArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    expect(lowerArc.at(-1)!.x).toBeGreaterThan(lowerArc[0].x);
    expect(higherArc.at(-1)!.x).toBeGreaterThan(higherArc[0].x);
  });

  it("Gujarati ક writes its joined loop-body before the diagonal cross-stroke", () => {
    expect(GUJARATI_KA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_KA)).toBe(1);
    expect(GUJARATI_KA.strokes).toHaveLength(2);
    expect(GUJARATI_KA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_KA.strokes[0].segments[0].path;
    const cross = GUJARATI_KA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(cross.at(-1)!.x).toBeGreaterThan(cross[0].x);
    expect(cross.at(-1)!.y).toBeGreaterThan(cross[0].y);
  });

  it("Gujarati ખ writes its joined left body before the separate right spine", () => {
    expect(GUJARATI_KHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_KHA)).toBe(1);
    expect(GUJARATI_KHA.strokes).toHaveLength(2);
    expect(GUJARATI_KHA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_KHA.strokes[0].segments[0].path;
    const spine = GUJARATI_KHA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ગ writes its rounded body before the separate right spine", () => {
    expect(GUJARATI_GA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_GA)).toBe(1);
    expect(GUJARATI_GA.strokes).toHaveLength(2);
    expect(GUJARATI_GA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_GA.strokes[0].segments[0].path;
    const spine = GUJARATI_GA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ઘ joins its upper and lower bodies before the separate right spine", () => {
    expect(GUJARATI_GHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_GHA)).toBe(1);
    expect(GUJARATI_GHA.strokes).toHaveLength(2);
    expect(GUJARATI_GHA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_GHA.strokes[0].segments[0].path;
    const spine = GUJARATI_GHA.strokes[1].segments[0].path;
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(200);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(500);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ઙ writes its S-like body before the separate upper-right dot", () => {
    expect(GUJARATI_NGA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NGA)).toBe(1);
    expect(GUJARATI_NGA.strokes).toHaveLength(2);
    expect(GUJARATI_NGA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_NGA.strokes[0].segments[0].path;
    const dot = GUJARATI_NGA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(dot.at(-1)).toEqual(dot[0]);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(350);
  });

  it("Gujarati ચ joins its bowls and middle loop before the separate right spine", () => {
    expect(GUJARATI_CA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_CA)).toBe(1);
    expect(GUJARATI_CA.strokes).toHaveLength(2);
    expect(GUJARATI_CA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_CA.strokes[0].segments[0].path;
    const spine = GUJARATI_CA.strokes[1].segments[0].path;
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(200);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(500);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati છ joins both upper lobes through one continuous lower body", () => {
    expect(GUJARATI_CHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_CHA)).toBe(0);
    expect(GUJARATI_CHA.strokes).toHaveLength(1);
    expect(GUJARATI_CHA.strokes[0].segments).toHaveLength(3);
    expect(joinGaps(GUJARATI_CHA.strokes[0]).every((gap) => gap === 0)).toBe(true);
    const path = penPath(GUJARATI_CHA.strokes[0]);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(500);
    expect(Math.max(...path.map((point) => point.x))).toBeGreaterThan(600);
  });

  it("Gujarati જ joins its left loop, crossing, right loop, and exit", () => {
    expect(GUJARATI_JA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_JA)).toBe(0);
    expect(GUJARATI_JA.strokes).toHaveLength(1);
    expect(GUJARATI_JA.strokes[0].segments).toHaveLength(3);
    expect(joinGaps(GUJARATI_JA.strokes[0]).every((gap) => gap === 0)).toBe(true);
    const path = penPath(GUJARATI_JA.strokes[0]);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(500);
    expect(path.at(-1)!.x).toBeGreaterThan(path[0].x);
  });

  it("Gujarati ઝ writes its left body, right loop-and-tail, then upper stem", () => {
    expect(GUJARATI_JHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_JHA)).toBe(2);
    expect(GUJARATI_JHA.strokes).toHaveLength(3);
    expect(GUJARATI_JHA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const left = GUJARATI_JHA.strokes[0].segments[0].path;
    const stem = GUJARATI_JHA.strokes[2].segments[0].path;
    expect(left.at(-1)!.x).toBeLessThan(left[0].x);
    expect(left.at(-1)!.y).toBeLessThan(left[0].y);
    expect(stem.at(-1)!.y).toBeLessThan(stem[0].y);
  });

  it("Gujarati ઞ writes its left body, short shoulder, then tall spine", () => {
    expect(GUJARATI_NYA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NYA)).toBe(2);
    expect(GUJARATI_NYA.strokes).toHaveLength(3);
    expect(GUJARATI_NYA.strokes.every((stroke) => stroke.segments.length === 1)).toBe(true);
    const body = GUJARATI_NYA.strokes[0].segments[0].path;
    const shoulder = GUJARATI_NYA.strokes[1].segments[0].path;
    const spine = GUJARATI_NYA.strokes[2].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ટ joins its upper turn, middle transition, and lower bowl", () => {
    expect(GUJARATI_TTA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_TTA)).toBe(0);
    expect(GUJARATI_TTA.strokes).toHaveLength(1);
    expect(GUJARATI_TTA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_TTA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(20);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(50);
    expect(path.at(-1)!.x).toBeGreaterThan(path[0].x);
  });

  it("Gujarati ઠ joins its high shoulder, outer bowl, and inward curl", () => {
    expect(GUJARATI_TTHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_TTHA)).toBe(0);
    expect(GUJARATI_TTHA.strokes).toHaveLength(1);
    expect(GUJARATI_TTHA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_TTHA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(30);
    expect(path[0].x).toBeGreaterThan(path[6].x);
    expect(path.at(-1)!.y).toBeGreaterThan(Math.min(...path.map((point) => point.y)));
  });

  it("Gujarati ડ joins its high shoulder, middle descent, and lower bowl", () => {
    expect(GUJARATI_DDA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_DDA)).toBe(0);
    expect(GUJARATI_DDA.strokes).toHaveLength(1);
    expect(GUJARATI_DDA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_DDA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(25);
    expect(path[0].x).toBeGreaterThan(path[6].x);
    expect(path.at(-1)!.x).toBeLessThan(path[0].x);
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

  it("Hebrew ז joins its short head directly to its curved stem", () => {
    expect(HEBREW_ZAYIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_ZAYIN)).toBe(0);
    expect(HEBREW_ZAYIN.strokes).toHaveLength(1);
    expect(HEBREW_ZAYIN.strokes.map((stroke) => stroke.segments.length)).toEqual([2]);
    const head = HEBREW_ZAYIN.strokes[0].segments[0].path;
    const stem = HEBREW_ZAYIN.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("Hebrew ח joins its top and right side before restarting the joined left leg", () => {
    expect(HEBREW_HEIT.script).toBe("hebrew");
    expect(penLifts(HEBREW_HEIT)).toBe(1);
    expect(HEBREW_HEIT.strokes).toHaveLength(2);
    expect(HEBREW_HEIT.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const top = HEBREW_HEIT.strokes[0].segments[0].path;
    const down = HEBREW_HEIT.strokes[0].segments[1].path;
    const joined = HEBREW_HEIT.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(joined[0].y).toBeGreaterThan(joined.at(-1)!.y);
  });

  it("Hebrew ט draws its left-and-base body before the bottom-up hooked side", () => {
    expect(HEBREW_TET.script).toBe("hebrew");
    expect(penLifts(HEBREW_TET)).toBe(1);
    expect(HEBREW_TET.strokes).toHaveLength(2);
    expect(HEBREW_TET.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 2]);
    const left = HEBREW_TET.strokes[0].segments[0].path;
    const base = HEBREW_TET.strokes[0].segments[1].path;
    const right = HEBREW_TET.strokes[1].segments[0].path;
    const hook = HEBREW_TET.strokes[1].segments[1].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(base[0]).toEqual(left.at(-1));
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
    expect(right[0]).toEqual(base.at(-1));
    expect(right[0].y).toBeLessThan(right.at(-1)!.y);
    expect(hook[0]).toEqual(right.at(-1));
    expect(hook.at(-1)!.x).toBeLessThan(hook[0].x);
  });

  it("Hebrew י joins its tiny head directly to the short descending stem", () => {
    expect(HEBREW_YOD.script).toBe("hebrew");
    expect(penLifts(HEBREW_YOD)).toBe(0);
    expect(HEBREW_YOD.strokes).toHaveLength(1);
    expect(HEBREW_YOD.strokes[0].segments).toHaveLength(2);
    const head = HEBREW_YOD.strokes[0].segments[0].path;
    const stem = HEBREW_YOD.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.min(...stem.map((point) => point.y))).toBeGreaterThan(250);
  });

  it("Hebrew כ turns its top bar into the right side and base without lifting", () => {
    expect(HEBREW_KAF.script).toBe("hebrew");
    expect(penLifts(HEBREW_KAF)).toBe(0);
    expect(HEBREW_KAF.strokes).toHaveLength(1);
    expect(HEBREW_KAF.strokes[0].segments).toHaveLength(3);
    const top = HEBREW_KAF.strokes[0].segments[0].path;
    const side = HEBREW_KAF.strokes[0].segments[1].path;
    const base = HEBREW_KAF.strokes[0].segments[2].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew ל descends, travels right, then turns diagonally down-left", () => {
    expect(HEBREW_LAMED.script).toBe("hebrew");
    expect(penLifts(HEBREW_LAMED)).toBe(0);
    expect(HEBREW_LAMED.strokes).toHaveLength(1);
    expect(HEBREW_LAMED.strokes[0].segments).toHaveLength(3);
    const tall = HEBREW_LAMED.strokes[0].segments[0].path;
    const bar = HEBREW_LAMED.strokes[0].segments[1].path;
    const lower = HEBREW_LAMED.strokes[0].segments[2].path;
    expect(tall[0].y).toBeGreaterThan(tall.at(-1)!.y);
    expect(bar[0]).toEqual(tall.at(-1));
    expect(bar[0].x).toBeLessThan(bar.at(-1)!.x);
    expect(lower[0]).toEqual(bar.at(-1));
    expect(lower[0].y).toBeGreaterThan(lower.at(-1)!.y);
    expect(lower[0].x).toBeGreaterThan(lower.at(-1)!.x);
  });

  it("Hebrew מ lifts once between its detached angled part and joined right body", () => {
    expect(HEBREW_MEM.script).toBe("hebrew");
    expect(penLifts(HEBREW_MEM)).toBe(1);
    expect(HEBREW_MEM.strokes).toHaveLength(2);
    expect(HEBREW_MEM.strokes[0].segments).toHaveLength(2);
    expect(HEBREW_MEM.strokes[1].segments).toHaveLength(3);
    const diagonal = HEBREW_MEM.strokes[0].segments[0].path;
    const inner = HEBREW_MEM.strokes[0].segments[1].path;
    const upper = HEBREW_MEM.strokes[1].segments[0].path;
    const side = HEBREW_MEM.strokes[1].segments[1].path;
    const base = HEBREW_MEM.strokes[1].segments[2].path;
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeLessThan(diagonal.at(-1)!.y);
    expect(inner[0]).toEqual(diagonal.at(-1));
    expect(inner[0].y).toBeGreaterThan(inner.at(-1)!.y);
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(side[0]).toEqual(upper.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew נ joins its top head, right descent, and leftward base", () => {
    expect(HEBREW_NUN.script).toBe("hebrew");
    expect(penLifts(HEBREW_NUN)).toBe(0);
    expect(HEBREW_NUN.strokes).toHaveLength(1);
    expect(HEBREW_NUN.strokes[0].segments).toHaveLength(3);
    const head = HEBREW_NUN.strokes[0].segments[0].path;
    const side = HEBREW_NUN.strokes[0].segments[1].path;
    const base = HEBREW_NUN.strokes[0].segments[2].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(side[0]).toEqual(head.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew ס closes its printed loop clockwise without lifting", () => {
    expect(HEBREW_SAMEKH.script).toBe("hebrew");
    expect(penLifts(HEBREW_SAMEKH)).toBe(0);
    expect(HEBREW_SAMEKH.strokes).toHaveLength(1);
    expect(HEBREW_SAMEKH.strokes[0].segments).toHaveLength(4);
    const [top, right, base, left] = HEBREW_SAMEKH.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(right[0]).toEqual(top.at(-1));
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(base[0]).toEqual(right.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(left[0]).toEqual(base.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(left.at(-1)).toEqual(top[0]);
  });

  it("Hebrew ע descends, sweeps left, and climbs without lifting", () => {
    expect(HEBREW_AYIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_AYIN)).toBe(0);
    expect(HEBREW_AYIN.strokes).toHaveLength(1);
    expect(HEBREW_AYIN.strokes[0].segments).toHaveLength(3);
    const [right, base, left] = HEBREW_AYIN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(base[0]).toEqual(right.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(left[0]).toEqual(base.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
  });

  it("Hebrew פ draws its outer body before the lifted inner curl", () => {
    expect(HEBREW_PE.script).toBe("hebrew");
    expect(penLifts(HEBREW_PE)).toBe(1);
    expect(HEBREW_PE.strokes).toHaveLength(2);
    expect(HEBREW_PE.strokes.map((stroke) => stroke.segments.length)).toEqual([3, 1]);
    const [top, side, base] = HEBREW_PE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const inner = HEBREW_PE.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(inner[0].x).toBeLessThan(inner.at(-1)!.x);
  });

  it("Hebrew צ turns its long diagonal into the base before the lifted arm", () => {
    expect(HEBREW_TSADI.script).toBe("hebrew");
    expect(penLifts(HEBREW_TSADI)).toBe(1);
    expect(HEBREW_TSADI.strokes).toHaveLength(2);
    expect(HEBREW_TSADI.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const diagonal = HEBREW_TSADI.strokes[0].segments[0].path;
    const base = HEBREW_TSADI.strokes[0].segments[1].path;
    const arm = HEBREW_TSADI.strokes[1].segments[0].path;
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
    expect(base[0]).toEqual(diagonal.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(arm[0].x).toBeGreaterThan(arm.at(-1)!.x);
    expect(arm[0].y).toBeGreaterThan(arm.at(-1)!.y);
  });

  it("Hebrew ק joins its top and right body before the lifted descender", () => {
    expect(HEBREW_QOF.script).toBe("hebrew");
    expect(penLifts(HEBREW_QOF)).toBe(1);
    expect(HEBREW_QOF.strokes).toHaveLength(2);
    expect(HEBREW_QOF.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const top = HEBREW_QOF.strokes[0].segments[0].path;
    const body = HEBREW_QOF.strokes[0].segments[1].path;
    const stem = HEBREW_QOF.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(body[0]).toEqual(top.at(-1));
    expect(body[0].x).toBeGreaterThan(body.at(-1)!.x);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(stem[0].y).toBeGreaterThan(0);
    expect(stem.at(-1)!.y).toBeLessThan(0);
  });

  it("Hebrew ר rounds its top bar directly into the right downstroke", () => {
    expect(HEBREW_RESH.script).toBe("hebrew");
    expect(penLifts(HEBREW_RESH)).toBe(0);
    expect(HEBREW_RESH.strokes).toHaveLength(1);
    expect(HEBREW_RESH.strokes[0].segments).toHaveLength(2);
    const top = HEBREW_RESH.strokes[0].segments[0].path;
    const side = HEBREW_RESH.strokes[0].segments[1].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
  });

  it("Hebrew ש draws its outer bowl before the lifted middle branch", () => {
    expect(HEBREW_SHIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_SHIN)).toBe(1);
    expect(HEBREW_SHIN.strokes).toHaveLength(2);
    expect(HEBREW_SHIN.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 1]);
    const outer = HEBREW_SHIN.strokes[0].segments[0].path;
    const left = HEBREW_SHIN.strokes[0].segments[1].path;
    const middle = HEBREW_SHIN.strokes[1].segments[0].path;
    expect(outer[0].y).toBeGreaterThan(outer.at(-1)!.y);
    expect(outer[0].x).toBeGreaterThan(outer.at(-1)!.x);
    expect(left[0]).toEqual(outer.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(middle[0].y).toBeGreaterThan(middle.at(-1)!.y);
  });

  it("Hebrew ת joins its top and right side before the lifted left leg", () => {
    expect(HEBREW_TAV.script).toBe("hebrew");
    expect(penLifts(HEBREW_TAV)).toBe(1);
    expect(HEBREW_TAV.strokes).toHaveLength(2);
    expect(HEBREW_TAV.strokes.map((stroke) => stroke.segments.length)).toEqual([2, 2]);
    const top = HEBREW_TAV.strokes[0].segments[0].path;
    const right = HEBREW_TAV.strokes[0].segments[1].path;
    const leg = HEBREW_TAV.strokes[1].segments[0].path;
    const foot = HEBREW_TAV.strokes[1].segments[1].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(right[0]).toEqual(top.at(-1));
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(leg[0].y).toBeGreaterThan(leg.at(-1)!.y);
    expect(foot[0]).toEqual(leg.at(-1));
    expect(foot[0].x).toBeGreaterThan(foot.at(-1)!.x);
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
    expect(verifiedLetterFont("人", CHINESE_REN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("亻", CHINESE_PERSON_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("口", CHINESE_MOUTH.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("女", CHINESE_WOMAN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("子", CHINESE_CHILD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("日", CHINESE_SUN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("讠", CHINESE_SPEECH_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("氵", CHINESE_WATER_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("宀", CHINESE_ROOF_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("你", CHINESE_YOU.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("好", CHINESE_GOOD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("我", CHINESE_I.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("是", CHINESE_BE.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("不", CHINESE_NOT.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("名", CHINESE_NAME.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("字", CHINESE_CHARACTER.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("谢", CHINESE_THANK.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("请", CHINESE_PLEASE.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("再", CHINESE_AGAIN.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("见", CHINESE_SEE.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("什", CHINESE_WHAT.source.url)).toBe("_fonts/NotoSansSC-Subset.ttf");
    expect(verifiedLetterFont("么", CHINESE_PARTICLE_ME.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("早", CHINESE_EARLY.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("上", CHINESE_UP.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("अ", DEVANAGARI_A.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("आ", DEVANAGARI_AA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("इ", DEVANAGARI_I.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ई", DEVANAGARI_II.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("उ", DEVANAGARI_U.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ऊ", DEVANAGARI_UU.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ए", DEVANAGARI_E.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ऐ", DEVANAGARI_AI.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ओ", DEVANAGARI_O.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("औ", DEVANAGARI_AU.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("क", DEVANAGARI_KA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ग", DEVANAGARI_GA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("च", DEVANAGARI_CA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("त", DEVANAGARI_TA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("द", DEVANAGARI_DA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ध", DEVANAGARI_DHA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("न", DEVANAGARI_NA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("प", DEVANAGARI_PA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ब", DEVANAGARI_BA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("भ", DEVANAGARI_BHA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("म", DEVANAGARI_MA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("य", DEVANAGARI_YA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("а", CYRILLIC_A.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("б", CYRILLIC_BE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("в", CYRILLIC_VE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("г", CYRILLIC_GE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("д", CYRILLIC_DE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("е", CYRILLIC_IE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("ё", CYRILLIC_IO.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("ж", CYRILLIC_ZHE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("з", CYRILLIC_ZE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("и", CYRILLIC_I.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("й", CYRILLIC_SHORT_I.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
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
    expect(verifiedLetterFont("ז", HEBREW_ZAYIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ח", HEBREW_HEIT.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ט", HEBREW_TET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("י", HEBREW_YOD.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("כ", HEBREW_KAF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ל", HEBREW_LAMED.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("מ", HEBREW_MEM.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("נ", HEBREW_NUN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ס", HEBREW_SAMEKH.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ע", HEBREW_AYIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("פ", HEBREW_PE.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("צ", HEBREW_TSADI.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ק", HEBREW_QOF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ר", HEBREW_RESH.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ש", HEBREW_SHIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ת", HEBREW_TAV.source.url)).toBe(
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

  it("Chinese 人 traces its two medians to the pinned PRC-order dataset", () => {
    const src = CHINESE_REN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BA.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 人\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper centre.*descends down-left.*median 2.*central junction.*descends down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 亻 traces its falling stroke and vertical to the pinned PRC-order dataset", () => {
    const src = CHINESE_PERSON_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BB.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 亻\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper right.*bends slightly right.*descends down-left.*median 2.*central junction.*moves slightly down-right.*descends vertically.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 口 traces its three-run closing order to the pinned PRC-order dataset", () => {
    const src = CHINESE_MOUTH.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%8F%A3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 口\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined corner.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 女 traces its bent first run and two lifted strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_WOMAN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%B3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 女\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper centre.*short down-right entry.*bends down-left.*lower-left junction.*turns without lifting.*sweeps down-right.*median 2.*upper right.*falls down-left.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*first stroke's internal turn.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 子 traces its two joined turns and final horizontal to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHILD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%90.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 子\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*crosses the top from left to right.*turns without lifting.*sweeps down-left.*median 2.*upper-left turn.*descends through the centre.*hooks left at the base without lifting.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both internal turns.*two intervening pen lifts.*base hook is flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 日 traces its joined corner and inside-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SUN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 日\.json.*ordered stroke paths and medians 1–4.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*four ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*left edge.*crosses the middle from left to right.*median 4.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*inside-before-close order.*three intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 讠 traces its dot and joined turning stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_SPEECH_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AE%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 讠\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper left.*dot down-right.*median 2.*left edge.*rises slightly.*short horizontal.*turns without lifting.*descend.*turns again without lifting.*finishes up-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both turns inside the second stroke.*one intervening pen lift.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 氵 traces its falling dots and rising bottom stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_WATER_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%B0%B5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 氵\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends down-right.*median 2.*middle left.*descends down-right.*median 3.*bottom.*moves slightly up-left.*rises.*upper right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*source order.*two intervening pen lifts.*shallower turn.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 宀 traces its dot, left drop, and joined roof hook to the pinned PRC-order dataset", () => {
    const src = CHINESE_ROOF_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AE%80.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 宀\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*top dot down-right.*median 2.*left side.*descends down-left.*median 3.*left roof edge.*horizontally to the right.*hooks down-left without lifting.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined horizontal hook.*two intervening pen lifts.*squarer.*more vertical.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 你 traces its component order and two joined hooks to the pinned PRC-order dataset", () => {
    const src = CHINESE_YOU.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BD%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 你\.json.*ordered stroke paths and medians 1–7.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*seven ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*upper right.*falls down-left.*median 4.*crosses rightward.*hooks down-left without lifting.*median 5.*centre.*descends.*hooks left without lifting.*medians 6–7.*lower-left dot down-left.*lower-right dot down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*component order.*six intervening pen lifts.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 好 traces 女-before-子 and three joined turns to the pinned PRC-order dataset", () => {
    const src = CHINESE_GOOD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%BD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 好\.json.*ordered stroke paths and medians 1–6.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*six ordered strokes.*Medians 1–3.*女 first.*bent stroke.*down-left.*sweeps right without lifting.*separately started stroke falls down-left.*horizontal crosses left-to-right.*Medians 4–6.*子.*horizontal turns down-left without lifting.*vertical descends and hooks left without lifting.*middle horizontal crosses left-to-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*女-before-子 component order.*all three joined turns.*five intervening pen lifts.*more angular.*flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 我 traces seven strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_I.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%88%91.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 我\.json.*medians 1–7.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /seven ordered strokes.*upper-left falling stroke.*upper horizontal.*hooked vertical.*lower rising stroke.*long curved slash.*hooks upward.*without lifting.*separate rising slash up-left.*final upper-right dot.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*six lifts/i,
    );
  });

  it("Chinese 是 traces 日-first order to the pinned PRC-order dataset", () => {
    const src = CHINESE_BE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%98%AF.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 是\.json.*medians 1–9.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–4.*日 first.*left vertical.*joined top and right sides.*inner horizontal.*closing bottom horizontal.*Medians 5–9.*wide horizontal.*central vertical.*short lower-right horizontal.*lower-left falling stroke.*long finishing stroke down-right.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*eight intervening lifts/i,
    );
  });

  it("Chinese 不 traces all four separate strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_NOT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8D.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 不\.json.*medians 1–4.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /four ordered strokes.*top horizontal.*left to right.*long falling stroke.*down-left.*central vertical.*right-falling dot.*People's Republic of China stroke order.*Noto Sans SC.*three intervening lifts/i,
    );
  });

  it("Chinese 名 traces 夕-before-口 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_NAME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%90%8D.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 名\.json.*medians 1–6.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*夕 first.*left-falling stroke.*horizontal.*down-left without lifting.*inner down-right dot.*Medians 4–6.*口.*left vertical.*top horizontal.*right side without lifting.*closing bottom horizontal.*People's Republic of China stroke order.*Noto Sans SC.*both joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 字 traces 宀-before-子 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHARACTER.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%97.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 字\.json.*medians 1–6.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*宀 first.*down-right top dot.*left-side down-left stroke.*horizontal roof.*hooks down-left without lifting.*Medians 4–6.*子.*top horizontal.*turns down-left without lifting.*vertical.*hooks left without lifting.*final middle horizontal.*People's Republic of China stroke order.*Noto Sans SC.*all three joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 谢 traces 讠-before-身-before-寸 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_THANK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%B0%A2.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 谢\.json.*medians 1–12.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /twelve ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–9.*身.*upper falling stroke.*left side.*top horizontal.*right side.*hooks left.*two inner horizontals.*wide lower horizontal.*lower falling stroke down-left.*Medians 10–12.*寸.*horizontal.*vertical.*hooks left.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*all five internal turns.*eleven intervening lifts/i,
    );
  });

  it("Chinese 请 traces 讠-before-青 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_PLEASE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AF%B7.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 请\.json.*medians 1–10.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /ten ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–10.*青.*two upper horizontals.*central vertical.*wide middle horizontal.*lower left side.*lower top horizontal.*right side.*hooks left.*two inner horizontals.*People's Republic of China stroke order.*Noto Sans SC.*all four internal turns.*nine intervening lifts/i,
    );
  });

  it("Chinese 再 traces its frame-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_AGAIN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%86%8D.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 再\.json.*medians 1–6.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /six ordered strokes.*top horizontal left-to-right.*left side.*Median 3.*frame's top.*right side.*hooks left.*central vertical.*inner horizontal.*closes with the long bottom horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both turns.*close-last rule.*five intervening lifts/i,
    );
  });

  it("Chinese 见 traces its frame-before-legs order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SEE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%A7%81.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 见\.json.*medians 1–4.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /four ordered strokes.*left side.*Median 2.*top horizontal.*right side.*left-falling leg.*Median 4.*second leg.*bends right.*upward hook.*People's Republic of China stroke order.*Noto Sans SC.*frame-before-legs.*three joined turns.*three intervening lifts/i,
    );
  });

  it("Chinese 什 traces its 亻-before-十 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_WHAT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BB%80.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 什\.json.*medians 1–4.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /four ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*十's horizontal left-to-right.*median 4.*descends 十's vertical.*People's Republic of China stroke order.*Noto Sans SC.*亻-before-十.*three intervening lifts/i,
    );
  });

  it("Chinese 么 traces its joined lower sweep to the pinned PRC-order dataset", () => {
    const src = CHINESE_PARTICLE_ME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B9%88.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 么\.json.*medians 1–3.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*upper left-falling stroke.*Median 2.*upper right.*falls down-left.*turns without lifting.*sweeps right along the base.*Median 3.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*second stroke's joined turn.*two intervening lifts/i,
    );
  });

  it("Chinese 早 traces its complete 日-before-十 order to the pinned PRC dataset", () => {
    const src = CHINESE_EARLY.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A9.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 早\.json.*medians 1–6.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–4.*complete 日 first.*left side.*top horizontal.*turns down the right side.*middle horizontal.*closing bottom horizontal.*Median 5.*十's horizontal left-to-right.*median 6.*descends its vertical.*People's Republic of China stroke order.*Noto Sans SC.*日-before-十.*joined top-right turn.*five intervening lifts/i,
    );
  });

  it("Chinese 上 traces its vertical-and-horizontals order to the pinned PRC dataset", () => {
    const src = CHINESE_UP.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8A.json",
    );
    expect(src.citation).toMatch(/Hanzi Writer Data 上\.json.*medians 1–3.*snapshot 68d10a4/i);
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*central vertical.*top toward the base.*Median 2.*starts at the vertical.*short middle horizontal left-to-right.*Median 3.*long base horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both intervening lifts.*short-before-long horizontal contrast/i,
    );
  });

  it("Devanagari अ traces its four-run modern form and records the six-stroke variant", () => {
    const src = DEVANAGARI_A.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%85_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari अ stroke order\.svg.*frames 1–4.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /four buildup frames.*four ordered pen-down runs.*frame 1.*upper-left.*upper curve.*lower bowl.*without lifting.*frame 2.*middle junction.*sweeps right.*shoulder.*frame 3.*right stem top-to-bottom.*frame 4.*shirorekhā left-to-right.*three intervening lifts.*Thomas Egenes.*Learning the Sanskrit Alphabet.*p\. 12.*six-stroke traditional Sanskrit form.*Noto Sans Devanagari/i,
    );
  });

  it("Devanagari आ traces its five-run modern form without erasing base-letter variation", () => {
    const src = DEVANAGARI_AA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%86_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari आ stroke order\.svg.*frames 1–5.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /five buildup frames.*five ordered pen-down runs.*frame 1.*upper-left.*upper curve.*lower bowl.*without lifting.*frame 2.*middle junction.*sweeps right.*shoulder.*frame 3.*inner stem top-to-bottom.*frame 4.*trailing stem top-to-bottom.*frame 5.*shirorekhā left-to-right.*four intervening lifts.*Thomas Egenes.*Learning the Sanskrit Alphabet.*p\. 12.*shared base अ.*six-stroke traditional Sanskrit form.*joined modern body is not universal.*Noto Sans Devanagari/i,
    );
  });

  it("Devanagari इ traces its continuous body before the lifted headline", () => {
    const src = DEVANAGARI_I.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%87_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari इ stroke order\.svg.*panels 1–2.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /two-panel diagram.*body.*one continuous pen-down run.*panel 1.*green start.*top of the upright.*descend the upright.*turn left.*upper bowl.*sweep right.*waist.*lower bowl.*finish down-right.*tail.*without lifting.*panel 2.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ई traces the shared body, upper curl, and final headline", () => {
    const src = DEVANAGARI_II.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%88_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ई stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*same continuous upright.*upper bowl.*lower bowl.*down-right tail.*इ.*panel 2.*headline junction.*upper curl upward.*left.*around.*open right tip.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari उ traces its joined upper bowl and lower loop before the headline", () => {
    const src = DEVANAGARI_U.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%89_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari उ stroke order\.svg.*panels 1–2.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /two-panel diagram.*two ordered pen-down runs.*panel 1.*one green start.*headline junction.*first arrow.*down and left.*upper bowl.*second arrow.*same run.*back through the waist.*down around the lower loop.*without lifting.*panel 2.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ऊ traces the shared body, right loop, and final headline", () => {
    const src = DEVANAGARI_UU.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%8A_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ऊ stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*same continuous upper bowl.*lower loop.*उ.*panel 2.*waist.*right-hand loop upward and right.*outer turn.*down-left.*open tip.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ए traces its long stem and tail before the short stem and headline", () => {
    const src = DEVANAGARI_E.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%8F_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ए stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*headline junction.*long left stem.*lower shoulder.*down through the tail.*without lifting.*panel 2.*shorter right stem's headline junction.*descends.*inward.*open hook.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ऐ traces the shared ए base, upper arc, and final headline", () => {
    const src = DEVANAGARI_AI.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%90_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ऐ stroke order\.svg.*panels 1–4.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /four-panel diagram.*four ordered pen-down runs.*panels 1 and 2.*same continuous long left stem and tail.*shorter inward-hooked right stem.*ए.*panel 3.*right headline junction.*upper arc upward and left.*open tip.*panel 4.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ओ traces the shared आ base, upper arc, and final headline", () => {
    const src = DEVANAGARI_O.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%93_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ओ stroke order\.svg.*panels 1–6.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /six-panel diagram.*six ordered pen-down runs.*panel 1.*joined upper-and-lower left body.*अ.*panel 2.*middle.*shoulder right.*panels 3 and 4.*inner and trailing stems.*आ.*panel 5.*trailing stem's headline junction.*upper arc upward and left.*open tip.*panel 6.*headline's left edge.*shirorekhā left-to-right.*five intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari औ traces the shared आ base, two upper arcs, and final headline", () => {
    const src = DEVANAGARI_AU.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%94_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari औ stroke order\.svg.*panels 1–7.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /seven-panel diagram.*seven ordered pen-down runs.*panel 1.*joined upper-and-lower left body.*अ.*panel 2.*middle.*shoulder right.*panels 3 and 4.*inner and trailing stems.*आ.*panels 5 and 6.*trailing stem's headline junction.*lower upper arc.*upward and left.*taller upper arc.*open tip.*panel 7.*headline's left edge.*shirorekhā left-to-right.*six intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari क traces the animated four-run bowl, stem, arch, and headline order", () => {
    const src = DEVANAGARI_KA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%95-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-क-order\.gif.*strokes 1–4.*Wikimedia Commons.*8 May 2009/i,
    );
    expect(src.variation).toMatch(
      /27-frame animation.*four ordered pen-down runs.*frames 2–11.*upper-right junction.*left over the top.*down the left side.*around the bottom.*lower-right junction.*frames 12–15.*central stem top-to-bottom.*frames 16–19.*upper junction.*right-hand arch clockwise.*open tip.*frames 20–27.*shirorekhā left-to-right.*three intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit III.*p\. 12.*same four-part buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari ग traces the animated three-run loop, right stem, and headline order", () => {
    const src = DEVANAGARI_GA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%97-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ग-order\.gif.*strokes 1–3.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /18-frame animation.*three ordered pen-down runs.*frames 2–9.*upper-right junction.*left over the top.*counterclockwise loop.*continue up the joined stem.*headline.*frames 10–13.*right stem's headline junction.*top-to-bottom.*frames 14–18.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit III.*p\. 14.*same three-part buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari च traces the animated three-run bar-body, right stem, and headline order", () => {
    const src = DEVANAGARI_CA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9A-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-च-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*three ordered pen-down runs.*frames 4–14.*upper bar left-to-right.*turn down and left.*shoulder.*rounded body.*open right junction.*without lifting.*frames 15–18.*right stem's headline junction.*top-to-bottom.*frames 19–22.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IV.*p\. 17.*upper bar.*rounded body.*right stem.*headline order.*staging.*separately.*rather than proving their join.*three-run lift count.*animation.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari त traces the animated three-run body, right stem, and headline order", () => {
    const src = DEVANAGARI_TA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A4-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-त-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /17-frame animation.*three ordered pen-down runs.*frames 1–7.*upper-right junction.*left across the shoulder.*curve down around the left side.*finish down-right.*open lower tip.*without lifting.*frames 8–12.*right stem's headline junction.*top-to-bottom.*frames 13–16.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 30.*same body.*right stem.*headline buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari द traces the animated three-run stem, joined body-tail, and headline order", () => {
    const src = DEVANAGARI_DA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A6-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-द-order\.gif.*strokes 1–3.*Wikimedia Commons.*8 May 2009/i,
    );
    expect(src.variation).toMatch(
      /18-frame animation.*three ordered pen-down runs.*frames 2–3.*short stem top-to-bottom.*frames 4–13.*lower junction.*left through the shoulder.*outer body.*inward and clockwise.*loop.*continue down-right.*tail without lifting.*frames 14–17.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 3 and 13.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 32.*short stem.*outer body.*inward curl-tail.*headline order.*staging.*separately.*rather than proving their join.*three-run lift count.*animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari ध traces the animated four-run spiral, bowl, stem, and headline order", () => {
    const src = DEVANAGARI_DHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A7-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ध-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /27-frame animation.*four ordered pen-down runs.*frames 2–11.*upper spiral's inner crossing.*small opening.*widen left and down.*outer loop.*right through the shoulder.*without lifting.*frames 12–19.*left waist.*down and around the lower bowl.*right junction.*frames 20–22.*right stem's headline junction.*top-to-bottom.*frames 23–26.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 11, 19, and 22.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 33.*same upper spiral.*lower bowl.*right stem.*headline buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari न traces the animated three-run clockwise loop, stem, and headline order", () => {
    const src = DEVANAGARI_NA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A8-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-न-order\.gif.*strokes 1–3.*Wikimedia Commons.*11 May 2009/i,
    );
    expect(src.variation).toMatch(
      /20-frame animation.*three ordered pen-down runs.*frames 2–9.*inner-right curve.*down and clockwise.*small opening.*continue right along the shoulder.*without lifting.*frames 10–14.*right stem's headline junction.*top-to-bottom.*frames 15–18.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 9 and 14.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 34.*same clockwise loop-and-shoulder.*right stem.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari प traces the animated three-run left stem, bowl, right stem, and headline order", () => {
    const src = DEVANAGARI_PA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%AA-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-प-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /19-frame animation.*three ordered pen-down runs.*frames 2–10.*descend the left stem.*curve right around the lower bowl.*rise to its upper-right junction.*without lifting.*frames 11–13.*right stem's headline junction.*top-to-bottom.*frames 14–17.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 10 and 13.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 35.*same left stem-and-bowl.*right stem.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari ब traces the animated four-run oval, right stem, inner diagonal, and headline order", () => {
    const src = DEVANAGARI_BA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_b_%E0%A4%AC.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari b ब\.gif.*strokes 1–4.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /13-frame animation.*four ordered pen-down runs.*frames 0–6.*upper-right junction.*counterclockwise around the oval.*lower-right junction.*frames 7–8.*right stem's headline junction.*top-to-bottom.*frame 9.*upper-left interior.*inner diagonal down-right.*frames 10–12.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*spatial restarts.*frames 7, 9, and 10.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 37.*same oval body.*right stem.*inner diagonal.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari भ traces the animated joined-body, right-stem, and headline order", () => {
    const src = DEVANAGARI_BHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_b%CA%B0_%E0%A4%AD.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari bʰ भ\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /15-frame animation.*three ordered pen-down runs.*frames 0–9.*upper loop's lower inner tip.*sweep left and clockwise around the upper loop.*descend the joined trunk.*clockwise around the lower bowl.*continue right through the crossbar.*without lifting.*frames 10–12.*right stem's headline junction.*top-to-bottom.*frames 13–14.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 10 and 13.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 38.*upper loop and trunk.*lower bowl and crossbar.*right stem.*headline order.*staging the upper and lower body parts separately.*rather than proving their join.*three-run lift count from the animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari म traces the animated joined left-body, right-stem, and headline order", () => {
    const src = DEVANAGARI_MA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_m_%E0%A4%AE.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari m म\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /12-frame animation.*three ordered pen-down runs.*frames 0–5.*descend the left stem.*curl left and clockwise around the lower loop.*continue right through the crossbar.*without lifting.*frames 6–8.*right stem's headline junction.*top-to-bottom.*frames 9–11.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 6 and 9.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 39.*left stem.*lower loop and crossbar.*right stem.*headline order.*staging the left stem and loop-crossbar separately.*rather than proving their join.*three-run lift count from the animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari य traces the corroborated four-run form and records the joined-body variation", () => {
    const src = DEVANAGARI_YA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%AF-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-य-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–5.*beneath the headline.*clockwise around the inner curl.*left waist.*frames 6–13.*restart at that waist.*down and right around the lower bowl.*right-stem junction.*frames 14–16.*right stem's headline junction.*top-to-bottom.*frames 17–20.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*190 ms hold.*frame 5.*250 ms holds.*frames 13 and 16.*one-second completed frame 21.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 41.*inner curl.*lower bowl.*right stem.*headline buildup.*JackPotte.*11-frame.*Devanagari j य\.gif.*29 March 2009.*joins the inner curl and lower bowl.*separately descended right stem.*left-to-right headline.*four-run form.*Noto Sans Devanagari.*three-run join.*simplify the bowls/i,
    );
  });

  it("Devanagari र traces the corroborated three-run form and records the joined-tail variation", () => {
    const src = DEVANAGARI_RA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B0-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-र-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /17-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–9.*right stem's headline junction.*descend top-to-bottom.*curl left and clockwise around the lower loop.*tail junction.*frames 10–12.*restart at that junction.*diagonal tail down-right.*frames 13–16.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*240 ms hold.*frame 9.*250 ms hold.*frame 12.*one-second completed frame 16.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 42.*looped stem.*diagonal tail.*headline buildup.*JackPotte.*seven-frame.*Devanagari r र\.gif.*29 March 2009.*joins the descending stem.*clockwise lower loop.*diagonal tail.*one continuous body.*separate left-to-right headline.*three-run form.*Noto Sans Devanagari.*two-run join.*simplify the lower loop/i,
    );
  });

  it("Devanagari ल traces the corroborated loop-first form and records the stem-first variation", () => {
    const src = DEVANAGARI_LA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B2-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ल-order\.gif.*strokes 1–4.*Wikimedia Commons.*11 May 2009/i,
    );
    expect(src.variation).toMatch(
      /23-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–9.*open lower-left tip.*curve up and clockwise around the left loop.*inner junction.*frames 10–12.*restart at that junction.*diagonal arm up-right.*right stem.*frames 13–17.*right stem's headline junction.*descend top-to-bottom.*frames 18–21.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 9, 12, and 17.*one-second completed frame 22.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 43.*left loop.*diagonal arm.*right stem.*headline buildup.*JackPotte.*12-frame.*Devanagari l ल\.gif.*29 March 2009.*right stem first.*frames 0–2.*diagonal arm.*frames 3–5.*left loop.*frames 6–8.*headline.*frames 9–10.*all frames last 100 ms.*loop-first four-run form.*Noto Sans Devanagari.*stem-first order.*simplify the loop/i,
    );
  });

  it("Devanagari व traces its three-run animation and deskbook buildup", () => {
    const src = DEVANAGARI_VA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_v_%E0%A4%B5.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari v व\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /11-frame animation.*three ordered pen-down runs.*frames 0–5.*upper-right of the body.*travel left around the top.*counterclockwise around the loop.*right-side junction.*frames 6–7.*right stem's headline junction.*descend top-to-bottom.*frames 8–10.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 6 and 8.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 44.*loop.*right stem.*headline buildup.*animation supplies.*directions and lift evidence.*corroborated three-run form.*Noto Sans Devanagari.*simplify or narrow the loop/i,
    );
  });

  it("Devanagari श traces its corroborated joined-body animation", () => {
    const src = DEVANAGARI_SHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B6-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-श-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /25-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–16.*upper loop's lower inner tip.*sweep left and clockwise around the upper loop.*descend the joined outer curve.*curl around the lower loop.*continue down-right through the diagonal tail.*without lifting.*frames 17–21.*right stem's headline junction.*descend top-to-bottom.*frames 22–24.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 16 and 21.*one-second completed frame 24.*JackPotte.*26-frame.*Devanagari ç श\.gif.*same joined body.*restarted right stem.*restarted headline.*spatial restarts.*frames 14 and 21.*all its frames last 100 ms.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 45.*body.*right-stem.*headline buildup.*animations supply.*directions and lift evidence.*corroborated three-run form.*Noto Sans Devanagari.*simplify or narrow the loops/i,
    );
  });

  it("Devanagari स traces its joined-body animation and staged corroboration", () => {
    const src = DEVANAGARI_SA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_s_%E0%A4%B8.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari s स\.gif.*strokes 1–4.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /13-frame animation.*four ordered pen-down runs.*frames 0–4.*just below the headline.*descend the left stem.*curve left around the central hook.*continue diagonally down-right through the tail.*without lifting.*frames 5–6.*central junction.*middle crossbar left-to-right.*frames 7–9.*right stem's headline junction.*descend top-to-bottom.*frames 10–12.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*spatial restarts.*frames 5, 7, and 10.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 47.*left curve.*diagonal tail.*middle crossbar.*right stem.*headline order.*stages the left curve and tail separately.*rather than proving their join.*four-run lift count.*animation.*Noto Sans Devanagari.*divide or simplify the joined body/i,
    );
  });

  it("Devanagari ह traces its three-run animation and staged corroboration", () => {
    const src = DEVANAGARI_HA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B9-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ह-order\.gif.*strokes 1–3.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–12.*right stem's headline junction.*descend top-to-bottom.*sweep left through the shoulder.*curve clockwise around the hooked body.*lower-right tip.*without lifting.*frames 13–16.*body's left junction.*sweep down-left around the outer curve.*continue diagonally down-right through the tail.*frames 17–21.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*230 ms hold.*frame 12.*250 ms hold.*frame 16.*one-second completed frame 21.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 48.*right stem.*leftward shoulder.*hooked body.*outer curve and tail.*headline buildup.*stages the joined first body.*more component steps.*three-run lift count.*animation.*Noto Sans Devanagari.*divide or simplify the body/i,
    );
  });

  it("Cyrillic а traces its one-run school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_A.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase а.*00:50–00:55.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*00:50–00:55.*rounded body.*right-hand finishing stem.*one continuous pen-down run.*zero intervening lifts.*single-storey.*Noto Sans Cyrillic.*double-storey printed form.*extra upper shoulder.*one-run body-to-stem motion.*entry into the lower loop.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic б traces its joined body and flag to the all-letter native lesson", () => {
    const src = CYRILLIC_BE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase б.*01:13–01:18.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:13–01:18.*lower body counterclockwise.*upper-right junction.*rising shoulder.*rightward top flag.*one continuous pen-down run.*zero intervening lifts.*crosses diagonally.*bundled Noto Sans Cyrillic.*upper-left shoulder.*uninterrupted body-to-flag order.*printed upper shoulder.*font's ink.*connected cursive.*top flag.*exit join/i,
    );
  });

  it("Cyrillic в traces its upper loop and lower bowl to the all-letter native lesson", () => {
    const src = CYRILLIC_VE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase в.*01:33–01:38.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:33–01:38.*starts at the baseline.*tall upper loop.*descends to the baseline.*counterclockwise.*lower bowl.*one continuous pen-down run.*zero intervening lifts.*looped Latin cursive b.*bundled Noto Sans Cyrillic.*two stacked bowls.*straight left stem.*baseline-to-upper-loop-to-baseline-to-lower-bowl order.*printed upper bowl.*font's ink.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic г traces its zero-lift cursive humps to the all-letter native lesson", () => {
    const src = CYRILLIC_GE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase г.*01:54–01:57.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:54–01:57.*rises from the baseline.*main shoulder.*turns at the baseline.*smaller rising-and-falling exit arch.*one continuous pen-down run.*zero intervening lifts.*rounded cursive form.*two humps.*bundled Noto Sans Cyrillic.*block-like printed form.*straight upright.*top bar.*no exit arch.*baseline-to-shoulder-to-baseline lift count.*climb the upright.*reverse to the junction.*descend the upright.*connected cursive restores.*exit arch/i,
    );
  });

  it("Cyrillic д traces its zero-lift body and descender to the all-letter native lesson", () => {
    const src = CYRILLIC_DE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase д.*02:14–02:19.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:14–02:19.*circles the rounded body counterclockwise.*upper-right junction.*descends below the baseline.*loops left.*rightward exit.*one continuous pen-down run.*zero intervening lifts.*looped Latin cursive g.*bundled Noto Sans Cyrillic.*block-like printed form.*trapezoidal body.*joined base shelf.*two separated feet.*body-before-descender order.*circle the body.*right stem and foot.*sweep left.*left foot.*finish rightward.*connected cursive restores.*below-baseline loop.*exit join/i,
    );
  });

  it("Cyrillic е traces its zero-lift looped form to the all-letter native lesson", () => {
    const src = CYRILLIC_IE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase е.*02:26–02:30.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:26–02:30.*begins at the upper right.*curves left around the upper loop.*crosses through the middle.*counterclockwise.*rounded lower bowl.*one continuous pen-down run.*zero intervening lifts.*tall open epsilon-like form.*small crossing loop.*bundled Noto Sans Cyrillic.*compact printed e.*long middle bar.*open right side.*upper-loop-to-middle-to-lower-bowl order.*curve around the upper bowl.*sweep right.*reverse through the junction.*circle the lower bowl.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic ё traces its body and two dots to the all-letter native lesson", () => {
    const src = CYRILLIC_IO.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ё.*02:51–02:56.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:51–02:56.*upper-loop-to-middle-to-lower-bowl body.*one continuous pen-down run.*left dot.*right dot.*three strokes.*two intervening lifts.*tall open epsilon-like body.*small crossing loop.*two dots above.*bundled Noto Sans Cyrillic.*compact printed e.*long middle bar.*two circular dots.*body-before-left-dot-before-right-dot order.*curve around the upper bowl.*sweep and reverse.*circle the lower bowl.*left to right.*connected cursive.*dots remain separate.*substitutes е.*always stressed/i,
    );
  });

  it("Cyrillic ж traces its zero-lift wings to the all-letter native lesson", () => {
    const src = CYRILLIC_ZHE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ж.*03:16–03:21.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:16–03:21.*lower left.*left arch.*tall central loop.*descends through the middle.*right arch.*rightward exit.*without lifting.*one continuous pen-down run.*zero intervening lifts.*rounded, looped form.*joined cursive arches.*bundled Noto Sans Cyrillic.*symmetric printed form.*straight central upright.*four diagonal arms.*lower-left-to-centre-to-right order.*lower-left arm.*retrace the upright.*upper-right arm.*return to the centre.*lower-right arm.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic з traces its zero-lift lobes to the all-letter native lesson", () => {
    const src = CYRILLIC_ZE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase з.*03:34–03:39.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:34–03:39.*upper left.*smaller upper lobe.*descends through the middle.*larger lower lobe.*curls left along the baseline.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*looped digit 3.*open left side.*exit join.*bundled Noto Sans Cyrillic.*compact printed form.*two open-right lobes.*no exit join.*upper-lobe-to-lower-lobe order.*circle the upper lobe.*middle junction.*circle the lower lobe.*lower-right tip.*connected cursive restores.*small rising exit/i,
    );
  });

  it("Cyrillic и traces its zero-lift stems to the all-letter native lesson", () => {
    const src = CYRILLIC_I.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase и.*03:56–04:02.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:56–04:02.*upper left.*descends to the baseline.*rising diagonal.*descends the right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin u.*rising entry side.*exit join.*bundled Noto Sans Cyrillic.*printed backwards-N form.*two straight vertical stems.*no entry or exit joins.*left-stem-to-rising-diagonal-to-right-stem order.*descend the left stem.*baseline.*rise through the diagonal.*descend the right stem.*connected cursive restores.*entry and exit joins/i,
    );
  });

  it("Cyrillic й traces its body and breve to the all-letter native lesson", () => {
    const src = CYRILLIC_SHORT_I.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase й.*04:17–04:24.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*04:17–04:24.*same left-stem-to-rising-diagonal-to-right-stem body.*one continuous pen-down run.*small rising exit.*lifts.*breve above from left to right.*one dipped arc.*two strokes.*one intervening lift.*rounded Latin u.*entry and exit joins.*shallow curved breve.*bundled Noto Sans Cyrillic.*printed backwards-N body.*two straight stems.*separate thicker breve.*body-before-breve order.*left-to-right breve direction.*one-lift evidence.*descend the left stem.*rise through the diagonal.*descend the right stem.*sweep down and up through the breve.*connected cursive restores.*breve remains separate/i,
    );
  });

  it("Cyrillic к traces its zero-lift stem and arms to the all-letter native lesson", () => {
    const src = CYRILLIC_KA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase к.*04:45–04:51.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*04:45–04:51.*upper left.*descends the left stem to the baseline.*looped upper-right arm.*returns to the middle junction.*lower arm.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded, looped upper arm.*entry and exit joins.*bundled Noto Sans Cyrillic.*straight vertical stem.*two angular diagonal arms.*left-stem-to-upper-arm-to-lower-arm order.*zero-lift evidence.*retrace upward through the middle.*upper diagonal out and back.*lower diagonal.*connected cursive restores.*rounded upper loop/i,
    );
  });

  it("Cyrillic л traces its zero-lift legs to the all-letter native lesson", () => {
    const src = CYRILLIC_EL.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase л.*05:06–05:10.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:06–05:10.*near the baseline.*curves left around a small hook.*rises steeply.*high apex.*descends through the right leg.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*pointed apex.*slanted right leg.*entry and exit joins.*bundled Noto Sans Cyrillic.*block-like printed form.*curved left leg.*horizontal top shoulder.*straight right stem.*hooked-left-leg-to-apex-to-right-leg order.*zero-lift evidence.*curve through the left leg.*sweep along the top shoulder.*descend the right stem.*connected cursive restores.*rising exit/i,
    );
  });

  it("Cyrillic м traces its zero-lift arches to the all-letter native lesson", () => {
    const src = CYRILLIC_EM.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase м.*05:26–05:31.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:26–05:31.*near the baseline.*small entry hook.*first apex.*central valley.*second apex.*right leg.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*two rounded arches.*entry and exit joins.*bundled Noto Sans Cyrillic.*angular printed form.*two straight upright stems.*deep central V.*entry-to-first-apex-to-valley-to-second-apex-to-baseline order.*zero-lift evidence.*rise through the left stem.*central V.*descend the right stem.*connected cursive restores.*rounded arches/i,
    );
  });

  it("Cyrillic н traces its zero-lift bridge to the all-letter native lesson", () => {
    const src = CYRILLIC_EN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase н.*05:47–05:52.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:47–05:52.*upper left.*left stem.*baseline.*turns upward without lifting.*rounded middle bridge.*right shoulder.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded bridge.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed H-like form.*two straight vertical stems.*horizontal middle bar.*left-stem-to-middle-bridge-to-right-stem order.*zero-lift evidence.*retrace to the middle junction.*horizontal bar.*upper-right tip.*descend the right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic о traces its zero-lift oval to the all-letter native lesson", () => {
    const src = CYRILLIC_O.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase о.*05:59–06:03.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:59–06:03.*upper right.*left across the top.*left side.*bottom.*right side.*closes the oval.*one continuous counterclockwise pen-down run.*zero intervening lifts.*tall, slightly slanted oval.*bundled Noto Sans Cyrillic.*wider, upright printed oval.*upper-right-to-left-side-to-bottom-to-right-side closure order.*zero-lift evidence.*closed counterclockwise oval.*connected cursive may add.*entry and exit joins/i,
    );
  });

  it("Cyrillic п traces its zero-lift shoulder to the all-letter native lesson", () => {
    const src = CYRILLIC_PE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase п.*06:26–06:31.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*06:26–06:31.*upper left.*left stem.*baseline.*turns upward without lifting.*rounded top shoulder.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin n.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed squared arch.*two straight vertical stems.*horizontal top bar.*left-stem-to-top-shoulder-to-right-stem order.*zero-lift evidence.*retrace to the upper-left junction.*top bar.*descend the right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic р traces its zero-lift descender and bowl to the all-letter native lesson", () => {
    const src = CYRILLIC_ER.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase р.*06:46–06:52.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*06:46–06:52.*upper left.*descends below the baseline.*retraces upward without lifting.*rounded shoulder.*descends to the baseline.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*long-descender Latin p.*open rounded bowl.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed p-like form.*straight descender stem.*closed rounded bowl.*stem-before-bowl order.*zero-lift evidence.*retrace to the upper-left shoulder.*circle the upper bowl clockwise.*return to the middle junction without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic с traces its zero-lift open curve to the all-letter native lesson", () => {
    const src = CYRILLIC_ES.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase с.*07:04–07:08.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:04–07:08.*upper right.*left across the top.*left side.*bottom.*lower-right exit.*one continuous counterclockwise pen-down run.*zero intervening lifts.*tall, slightly slanted open curve.*small rising exit.*bundled Noto Sans Cyrillic.*wider, upright printed C-like form.*blunt open tips.*upper-right-to-left-side-to-bottom-to-lower-right order.*zero-lift evidence.*one open counterclockwise curve.*connected cursive may add.*entry join.*exit join/i,
    );
  });

  it("Cyrillic т traces its zero-lift m-like school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_TE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase т.*07:29–07:36.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:29–07:36.*upper left.*left stem.*turns upward without lifting.*first arch.*middle stem.*second arch.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin m.*two arches.*rising exit.*bundled Noto Sans Cyrillic.*printed T-shaped form.*one central vertical stem.*horizontal top bar.*initial-descent-before-joined-top-movements order.*zero-lift evidence.*descend the central stem.*retrace to the top junction.*sweep left along the top bar.*retrace through the junction.*right tip without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic у traces its zero-lift loop-descender school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_U.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase у.*07:50–07:55.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:50–07:55.*upper left.*descends to the baseline.*turns upward without lifting.*right arm.*retraces down.*long descender.*curls left.*lower loop.*crosses the descender.*short rightward exit.*one continuous pen-down run.*zero intervening lifts.*loop-descender Latin y.*narrow rounded upper body.*rising exit.*bundled Noto Sans Cyrillic.*printed y-like form.*two straight upper arms.*broad descender.*curves left without a loop or exit join.*left-arm-to-right-arm-to-descender order.*zero-lift evidence.*descend the left arm.*rise through the right arm.*retrace to the junction.*long descender.*curve left through its terminal without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ф traces its stem-first linked loops to the all-letter native lesson", () => {
    const src = CYRILLIC_EF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ф.*08:16–08:26.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*08:16–08:26.*first descends the long central stem.*upper line.*below the baseline.*lifts once.*upper central junction.*narrow left loop.*crosses the stem.*narrow right loop.*small rising exit.*two pen-down runs.*one intervening lift.*stem first.*linked left-to-right looped body.*bundled Noto Sans Cyrillic.*printed phi-like form.*straight central ascender-descender.*two wider upright bowls.*without an exit join.*stem-before-left-loop-before-right-loop order.*one-lift evidence.*descend the central stem.*lift.*trace the left bowl.*central junction.*trace the right bowl.*connected cursive restores/i,
    );
  });

  it("Cyrillic х traces its two facing curves to the all-letter native lesson", () => {
    const src = CYRILLIC_HA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase х.*08:42–08:49.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*08:42–08:49.*upper left.*curves right.*middle crossing.*sweeps left.*lower terminal.*lifts once.*second run.*upper right.*curves left.*same crossing.*sweeps right.*small rising exit.*two facing curved runs.*one intervening lift.*right-bulging left stroke.*left-bulging right stroke.*bundled Noto Sans Cyrillic.*printed X-like form.*four straight diagonal arms.*centre.*no entry or exit joins.*upper-left-run-before-upper-right-run order.*top-to-bottom directions.*crossing.*one-lift evidence.*two straight left arms.*lift.*two straight right arms.*connected cursive restores/i,
    );
  });

  it("Cyrillic ц traces its joined body and tail to the all-letter native lesson", () => {
    const src = CYRILLIC_TSE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ц.*09:05–09:10.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:05–09:10.*upper left.*descends the left stem.*rounded baseline join.*rises through the right stem.*descends that stem.*small lower loop.*rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*joined diagonal up.*right stem down.*looped tail.*bundled Noto Sans Cyrillic.*printed squared U-like form.*two straight verticals.*horizontal bottom bar.*short separate-looking right descender.*without rounded joins or an exit loop.*left-stem-to-right-stem-to-tail order.*zero-lift evidence.*descend the left stem.*bottom bar.*rise and retrace the right stem.*tail shoulder.*descend the short tail without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ч traces its joined bowl and stems to the all-letter native lesson", () => {
    const src = CYRILLIC_CHE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ч.*09:24–09:28.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:24–09:28.*upper left.*descends the short left stem.*middle join.*narrow rounded bridge.*rises to the top of the right stem.*descends that full stem.*baseline.*small rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*short left stem down.*joined rise.*long right stem down.*curled exit.*bundled Noto Sans Cyrillic.*printed ч-like form.*shorter straight left stem.*shallow rounded bowl.*full-height straight right stem.*without the narrow bridge.*curled baseline.*exit join.*short-stem-to-bowl-to-long-stem order.*zero-lift evidence.*descend the short left stem.*shallow bowl.*rise along the right stem.*descend the full right stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ш traces its joined three-stem order to the all-letter native lesson", () => {
    const src = CYRILLIC_SHA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ш.*09:49–09:57.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:49–09:57.*upper left.*descends the left stem.*baseline.*rounded diagonal join.*top of the middle stem.*descends the middle stem.*second rounded diagonal join.*top of the right stem.*descends the right stem.*small rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*first joined rise.*middle stem down.*second joined rise.*right stem down.*curled exit.*bundled Noto Sans Cyrillic.*printed ш-like form.*three straight full-height vertical stems.*two horizontal baseline bars.*without diagonal rounded joins.*curled baselines.*exit join.*left-to-middle-to-right order.*zero-lift evidence.*descend the left stem.*first bottom bar.*rise then retrace the middle stem.*second bottom bar.*rise then retrace the right stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic щ traces its joined three-stem-to-tail order to the native lesson", () => {
    const src = CYRILLIC_SHCHA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase щ.*10:17–10:25.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:17–10:25.*upper left.*descends the left stem.*baseline.*rounded diagonal join.*top of the middle stem.*descends the middle stem.*second rounded diagonal join.*top of the right stem.*descends the right stem.*small lower tail loop.*rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*first joined rise.*middle stem down.*second joined rise.*right stem down.*looped tail.*bundled Noto Sans Cyrillic.*printed щ-like form.*three straight full-height vertical stems.*two horizontal baseline bars.*short separate-looking right descender.*without diagonal rounded joins.*exit loop.*left-to-middle-to-right-to-tail order.*zero-lift evidence.*descend the left stem.*first bottom bar.*rise then retrace the middle stem.*second bottom bar.*rise then retrace the right stem.*tail shoulder.*descend the short tail without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ъ traces its joined flag-to-stem-to-bowl order to the native lesson", () => {
    const src = CYRILLIC_HARD_SIGN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ъ.*10:34–10:38.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:34–10:38.*upper left.*narrow entry loop.*rounded top shoulder.*descends the tall main stem.*baseline.*circles the lower bowl counterclockwise.*closes it against the stem without lifting.*one continuous pen-down run.*zero intervening lifts.*looped entry and top shoulder.*main stem down.*joined lower bowl.*bundled Noto Sans Cyrillic.*printed ъ-like form.*broad horizontal top flag.*straight main stem.*wide closed lower bowl.*without the narrow entry loop.*rounded top shoulder.*flag-to-stem-to-bowl order.*zero-lift evidence.*sweep right along the top flag.*descend the main stem.*circle the lower bowl counterclockwise.*close it against the stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ы traces its body-before-right-stem order to the native lesson", () => {
    const src = CYRILLIC_YERY.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ы.*10:45–10:56.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:45–10:56.*upper left.*narrow entry loop.*descends the tall left stem.*baseline.*circles the joined lower bowl counterclockwise.*closes it against the stem.*lifts once.*upper right.*descends the separate tall right stem.*small rising exit.*two pen-down runs.*one intervening lift.*left stem and lower bowl first.*separate right stem.*bundled Noto Sans Cyrillic.*printed ы-like form.*straight full-height left upright.*wide closed lower bowl.*separate straight full-height right stem.*without the narrow entry loop.*curled exit.*body-before-right-stem order.*counterclockwise direction.*one-lift evidence.*descend the left stem.*circle and close the lower bowl.*lift once.*descend the separate right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic ь traces its zero-lift stem-before-bowl order to the native lesson", () => {
    const src = CYRILLIC_SOFT_SIGN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ь.*11:16–11:20.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:16–11:20.*upper left.*narrow entry stroke.*baseline.*circles the joined lower bowl counterclockwise.*closes it against the stem.*without lifting.*one continuous pen-down run.*zero intervening lifts.*stem down.*joined lower bowl.*bundled Noto Sans Cyrillic.*printed ь-like form.*straight full-height upright.*wide closed lower bowl.*slightly slanted entry.*rounded handwritten join.*stem-to-bowl order.*counterclockwise direction.*zero-lift evidence.*descend the stem.*circle the lower bowl counterclockwise.*close it against the stem.*connected cursive may restore/i,
    );
  });

  it("Cyrillic э traces its outer-before-tongue order to the native lesson", () => {
    const src = CYRILLIC_E.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase э.*11:25–11:32.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:25–11:32.*upper-left opening.*curves right across the top.*descends around the outer right side.*sweeps left through the lower curve.*lower-left opening.*lifts once.*middle right.*middle tongue right-to-left.*gentle hook.*two pen-down runs.*one intervening lift.*outer backwards-C curve first.*bundled Noto Sans Cyrillic.*printed э-like form.*broad open-left outer curve.*straight horizontal middle bar.*narrower rounded curve.*hooked tongue.*outer-before-tongue order.*clockwise outer direction.*right-to-left tongue direction.*one-lift evidence.*upper left to lower left.*lift once.*middle bar from right to left.*connected cursive may narrow/i,
    );
  });

  it("Cyrillic ю traces its zero-lift stem-to-oval order to the native lesson", () => {
    const src = CYRILLIC_YU.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ю.*11:44–11:58.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:44–11:58.*small looped entry.*descends the tall left stem.*baseline.*rising diagonal connector.*continues directly into the right oval.*upper-left side.*across the top.*descends the right side.*rounds the bottom.*rises along the left side.*without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*joined connector.*clockwise right oval.*bundled Noto Sans Cyrillic.*printed ю-like form.*straight full-height left upright.*horizontal middle bar.*wide closed oval.*looped entry.*diagonal connector.*narrow cursive oval.*stem-to-connector-to-oval order.*clockwise oval direction.*zero-lift evidence.*descend the left stem.*retrace to the middle.*circle the right oval clockwise.*close it without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic я traces its zero-lift rise-to-loop-to-leg order to the native lesson", () => {
    const src = CYRILLIC_YA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase я.*12:13–12:21.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*12:13–12:21.*baseline.*curved entry.*upper-right junction.*upper loop counterclockwise.*returns to the junction.*long diagonal leg.*baseline exit.*without lifting.*one continuous pen-down run.*zero intervening lifts.*rising entry.*upper loop.*descending diagonal leg.*bundled Noto Sans Cyrillic.*printed я-like form.*straight full-height right upright.*broad upper bowl.*angular lower-left leg.*curved rising entry.*narrow loop.*slanted leg.*exit join.*rise-to-loop-to-leg order.*counterclockwise loop direction.*zero-lift evidence.*climb the right stem.*circle the upper bowl counterclockwise.*descend the diagonal leg without lifting.*connected cursive restores/i,
    );
  });

  it("Gujarati અ traces its body-before-stem order to the teaching animation", () => {
    const src = GUJARATI_A.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*અ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper-left tip.*clockwise.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*separate right stem.*lower-right foot.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem order.*one-lift evidence/i,
    );
  });

  it("Gujarati આ traces its two lifted stems to the next teaching animation", () => {
    const src = GUJARATI_AA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*આ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*joined અ body.*open curve.*clockwise.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*separate right stem.*lifts again.*third SVG path.*trailing ā stem.*matching foot.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઇ traces its unbroken loop-and-hook order to the next animation", () => {
    const src = GUJARATI_I.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઇ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper loop.*narrow middle crossing.*broad lower body.*right side.*upper-right hook.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-loop-to-lower-loop-to-hook order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઈ traces its unbroken extended curl to the adjacent animation", () => {
    const src = GUJARATI_II.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઈ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper loop.*narrow middle crossing.*broad lower body.*right side.*clockwise.*extended top hook.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-loop-to-lower-loop-to-extended-curl order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઉ traces its unbroken bowls and returning curve to the next animation", () => {
    const src = GUJARATI_U.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઉ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper bowl.*middle cusp.*broad lower bowl.*tall outer-left curve.*upper right.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-bowl-to-lower-bowl-to-outer-curve order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઊ traces its unbroken extended tail to the adjacent animation", () => {
    const src = GUJARATI_UU.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઊ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*repeats ઉ.*small upper bowl.*middle cusp.*broad lower bowl.*tall outer-left return.*upper shoulder.*long right-side tail.*lower foot.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*complete-u-before-extended-tail order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઋ traces its bent body, stem, and right loop to three paths", () => {
    const src = GUJARATI_VOCALIC_R.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઋ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*begins at the left.*sweeps right.*shallow upper body.*middle turn.*reverses diagonally down-left.*lower terminal.*lifts once.*second SVG path.*separate central stem.*lower foot.*lifts again.*third SVG path.*stem junction.*compact upper-right loop.*longer right-side tail.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-central-stem-before-right-loop-and-tail order.*two-lift evidence/i,
    );
  });

  it("Gujarati એ traces its body, stem, and high arc to three ordered paths", () => {
    const src = GUJARATI_E.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*એ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*left bowl.*broad lower body.*small right arch.*lifts once.*second SVG path.*full-height right stem.*lower foot.*lifts again.*third SVG path.*high arcing mark.*left to right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem-before-high-arc order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઐ traces its stacked arcs to four ordered paths", () => {
    const src = GUJARATI_AI.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઐ animation.*first through fourth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /four ordered pen-down runs.*first SVG path.*joined left bowl.*broad lower body.*small right arch.*lifts once.*second SVG path.*full-height right stem.*lower foot.*lifts again.*third SVG path.*lower high arc.*left to right.*lifts once more.*fourth SVG path.*second higher arc.*same direction.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem-before-lower-arc-before-higher-arc order.*three-lift evidence/i,
    );
  });

  it("Gujarati ઓ traces its આ sequence and high arc to four ordered paths", () => {
    const src = GUJARATI_O.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઓ animation.*first through fourth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /four ordered pen-down runs.*first SVG path.*આ's joined body.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*first separate right stem.*lower foot.*lifts again.*third SVG path.*trailing stem.*matching foot.*lifts once more.*fourth SVG path.*high arc.*left to right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem-before-high-arc order.*three-lift evidence/i,
    );
  });

  it("Gujarati ઔ traces its stacked arcs to five ordered paths", () => {
    const src = GUJARATI_AU.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઔ animation.*first through fifth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /five ordered pen-down runs.*first SVG path.*ઓ's joined body.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*first separate right stem.*lower foot.*lifts again.*third SVG path.*trailing stem.*matching foot.*lifts once more.*fourth SVG path.*lower high arc.*left to right.*lifts a fourth time.*fifth SVG path.*second, higher arc.*same direction.*remaining path slot is empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem-before-lower-arc-before-higher-arc order.*four-lift evidence/i,
    );
  });

  it("Gujarati ક traces its joined body and crossing diagonal to two paths", () => {
    const src = GUJARATI_KA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ક animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*circles left.*upper loop.*diagonally down-right.*middle crossing.*rounded lower body.*lower left.*lifts once.*second SVG path.*diagonal cross-stroke.*lower left to upper right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-loop-body-before-diagonal-cross-stroke order.*one-lift evidence/i,
    );
  });

  it("Gujarati ખ traces its joined body and right spine to two paths", () => {
    const src = GUJARATI_KHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ખ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*left lobe.*curls through the middle.*beside the right spine.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ગ traces its rounded body and right spine to two paths", () => {
    const src = GUJARATI_GA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ગ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*clockwise.*rounded body.*lower left.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*rounded-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ઘ traces its joined double body and right spine to two paths", () => {
    const src = GUJARATI_GHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઘ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*upper lobe.*back left through the middle.*clockwise.*rounded lower body.*upper right.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-upper-and-lower-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ઙ traces its S-like body and upper-right dot to two paths", () => {
    const src = GUJARATI_NGA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઙ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*upper turn.*diagonally through the middle.*rounded lower body.*lower left.*lifts once.*second SVG path.*upper-right dot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-dot order.*one-lift evidence/i,
    );
  });

  it("Gujarati ચ traces its joined bowls and right spine to two paths", () => {
    const src = GUJARATI_CA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ચ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*upper bowl.*middle loop.*clockwise.*broad lower body.*upper right.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati છ traces both lobes and the lower body to one continuous path", () => {
    const src = GUJARATI_CHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*છ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper left.*upper-left lobe.*middle.*broad lower body.*outer-right curve.*upper-right lobe.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-left-lobe-to-lower-body-to-upper-right-lobe order.*zero-lift evidence/i,
    );
  });

  it("Gujarati જ traces both loops, crossing, and exit to one continuous path", () => {
    const src = GUJARATI_JA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*જ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper left.*upper-left loop.*diagonally down-right.*crossing body.*lower-right loop.*upper-right exit.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-loop-to-crossing-to-right-loop-to-exit order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઝ traces its left body, right loop, and stem to three paths", () => {
    const src = GUJARATI_JHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(/t30apps\.com.*version 1\.0.*ઝ animation.*first through third SVG paths/i);
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*upper left.*rounded left body.*lower left.*lifts once.*second SVG path.*right loop.*lower tail.*lifts again.*third SVG path.*upper stem.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-right-loop-and-tail-before-upper-stem order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઞ traces its left body, shoulder, and spine to three paths", () => {
    const src = GUJARATI_NYA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(/t30apps\.com.*version 1\.0.*ઞ animation.*first through third SVG paths/i);
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*upper left.*rounded left body.*lower left.*lifts once.*second SVG path.*rightward shoulder.*lifts again.*third SVG path.*tall right spine.*lower-right terminal.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-rightward-shoulder-before-tall-spine order.*two-lift evidence/i,
    );
  });

  it("Gujarati ટ traces its joined upper turn and lower bowl to one path", () => {
    const src = GUJARATI_TTA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(/t30apps\.com.*version 1\.0.*ટ animation.*first SVG path/i);
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper left.*rounded upper turn.*diagonally down-left.*broad lower bowl.*right side.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-turn-to-middle-to-lower-bowl order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઠ traces its shoulder, outer bowl, and inward curl to one path", () => {
    const src = GUJARATI_TTHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(/t30apps\.com.*version 1\.0.*ઠ animation.*first SVG path/i);
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper right.*left across the high shoulder.*descends through the middle.*outer lower bowl.*curls back inward.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*shoulder-to-outer-bowl-to-inner-terminal order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ડ traces its shoulder, middle descent, and lower bowl to one path", () => {
    const src = GUJARATI_DDA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(/t30apps\.com.*version 1\.0.*ડ animation.*first SVG path/i);
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper right.*left across the high shoulder.*descends through the middle.*broad lower bowl.*lower-left terminal.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*shoulder-to-middle-to-lower-bowl order.*zero-lift evidence/i,
    );
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

  it("Hebrew ז preserves its rounded handwritten run on the block outline", () => {
    const src = HEBREW_ZAYIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=XTqG_1dsFSU");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Zayin and Heit.*00:44.0–00:45.4.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Zayin.*one uninterrupted run.*00:44.0–00:45.4.*opening rises to the right.*curves down the right side.*around the base without lifting.*mirror image.*handwritten Gimel.*00:49.9–00:55.7.*facing directions.*01:00.1–01:05.0.*printed form.*angular.*not to write Zayin like Vav.*01:17.6–01:24.2.*left-to-right start.*continuous descent.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ח traces its printed gate before restarting the joined left leg", () => {
    const src = HEBREW_HEIT.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=XTqG_1dsFSU");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Zayin and Heit.*02:44.6–02:46.3.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Heit.*02:35.7–02:36.9.*arched top.*right side.*one lift.*left leg.*top junction.*printed demonstration.*top bar left-to-right.*right side.*02:44.6–02:45.3.*lifts once.*joined left leg top-to-bottom.*02:45.6–02:46.3.*handwriting corners round.*02:39.0–02:41.6.*print version sharper.*02:47.9–02:52.5.*Noto Sans Hebrew.*rounded handwriting variation/i,
    );
  });

  it("Hebrew ט traces its printed bowl before the unusual bottom-up hook", () => {
    const src = HEBREW_TET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=NBUtBPVKchk");
    expect(src.citation).toMatch(
      /Hebrew Writing #6.*Tet and Yod.*00:54.2–00:56.3.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Tet.*00:44.9–00:46.0.*start at the bottom.*left curve.*right side.*curl inward.*unusual.*bottom up.*00:47.4–00:51.1.*printed demonstration.*left side top-to-bottom.*base.*00:54.2–00:55.4.*lifts once.*lower-right.*climbs the right side.*inward hook.*00:55.7–00:56.3.*rounding the printed corners.*handwritten form.*00:57.5–01:02.3.*Noto Sans Hebrew.*bottom-up handwritten variation/i,
    );
  });

  it("Hebrew י traces its tiny printed head and stem in one run", () => {
    const src = HEBREW_YOD.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=NBUtBPVKchk");
    expect(src.citation).toMatch(
      /Hebrew Writing #6.*Tet and Yod.*02:00.7–02:01.2.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Yod.*tiny comma-like run.*01:53.4–01:53.8.*high on the writing line.*simplest letter.*01:50.4–01:51.0.*little comma.*upper-right.*01:54.0–01:57.2.*printed demonstration.*head left-to-right.*short stem.*without lifting.*02:00.7–02:01.2.*print is almost the same.*little angle.*02:03.7–02:07.1.*Noto Sans Hebrew.*comma-like handwritten variation/i,
    );
  });

  it("Hebrew כ traces the sharp printed form in one continuous run", () => {
    const src = HEBREW_KAF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=EcQ0gL-NM-k");
    expect(src.citation).toMatch(
      /Hebrew Writing #7.*Kaf.*00:51.3–00:53.2.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Kaf.*rounded half-circle.*upper-left.*00:40.2–00:41.4.*half a circle to the right.*upper-left side.*going down.*00:41.8–00:47.0.*printed demonstration.*top bar left-to-right.*right side.*left along the base.*without lifting.*00:51.3–00:53.2.*same but with sharp corners.*00:47.3–00:49.5.*Noto Sans Hebrew.*rounded handwritten variation/i,
    );
  });

  it("Hebrew ל traces the tall printed form in one continuous run", () => {
    const src = HEBREW_LAMED.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=CBU6aSCcPrE");
    expect(src.citation).toMatch(
      /Hebrew Writing #8.*Lamed and Mem.*01:22.4–01:23.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Lamed.*rounded looping run.*00:52.8–00:53.7.*printed demonstration.*top.*tall left stroke.*descends.*middle junction.*right along the bar.*diagonally down-left.*without lifting.*01:22.4–01:23.9.*printed form is angular.*handwriting is looped and rounded.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew מ traces the open printed form in two pen-down runs", () => {
    const src = HEBREW_MEM.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=CBU6aSCcPrE");
    expect(src.citation).toMatch(
      /Hebrew Writing #8.*Lamed and Mem.*03:07.7–03:10.6.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Mem.*N-like cursive zigzag.*03:02.8–03:05.3.*printed demonstration.*detached left part.*lower tip.*corner.*down-right.*short inner leg.*lifts once.*angular right body.*climb diagonally right.*upper shoulder.*down the right side.*left along the base.*03:07.7–03:10.6.*open at the bottom-left.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew נ traces the printed hook in one continuous run", () => {
    const src = HEBREW_NUN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:04.1–02:04.6.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Nun.*one continuous run.*02:04.1–02:04.6.*top head.*left-to-right.*right side.*left along the base.*without lifting.*cursive Nun.*purple.*rounder, wider hook.*02:05.2–02:05.8.*without lifting.*previously queued.*Hebrew Letters - NUN.*3gYCaDgB-Nk.*religious exposition.*not used.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew ס traces one printed clockwise loop and records the round cursive form", () => {
    const src = HEBREW_SAMEKH.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:19.5–02:20.8.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Samekh.*one continuous clockwise run.*02:19.5–02:20.8.*flat top.*left-to-right.*right side.*left along the base.*left side.*close.*without lifting.*cursive Samekh.*purple.*rounder oval.*02:23.8–02:24.7.*one uninterrupted loop.*Noto Sans Hebrew.*cursive variation/i,
    );
  });

  it("Hebrew ע traces one printed branch-and-base run and records cursive looping", () => {
    const src = HEBREW_AYIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:27.4–02:28.9.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Ayin.*one continuous run.*02:27.4–02:28.9.*right branch.*descends.*left into the base.*farther left.*turns back.*climbs the left branch.*without lifting.*cursive Ayin.*purple.*compact looped form.*02:31.6–02:32.7.*without lifting.*Noto Sans Hebrew.*cursive variation/i,
    );
  });

  it("Hebrew פ traces its two-run printed form and records the cursive spiral", () => {
    const src = HEBREW_PE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:36.3–02:38.9.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Pe.*two runs.*02:36.3–02:38.9.*outer body.*upper left.*right across the top.*right side.*left along the base.*one lift.*short inner curl.*left to right.*cursive Pe.*purple.*one inward spiral.*02:41.5–02:43.0.*without lifting.*Noto Sans Hebrew.*rounded cursive variation/i,
    );
  });

  it("Hebrew צ traces its two-run printed form and records the compact cursive form", () => {
    const src = HEBREW_TSADI.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:59.8–03:01.2.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Tsadi.*two runs.*02:59.8–03:01.2.*long diagonal.*upper left.*turns sharply left.*base.*without lifting.*one lift.*short upper-right arm.*down-left.*middle junction.*cursive Tsadi.*purple.*compact, rounded 3-like run.*03:03.2–03:04.0.*without lifting.*Noto Sans Hebrew.*compact cursive variation/i,
    );
  });

  it("Hebrew ק traces its two-run printed form and records the cursive hook", () => {
    const src = HEBREW_QOF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:18.3–03:20.0.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Qof.*two runs.*03:18.3–03:20.0.*top bar.*left-to-right.*down-left.*right body.*without lifting.*one lift.*separate inner-left stem.*below the writing line.*cursive Qof.*purple.*one continuous hooked descent.*03:22.0–03:23.3.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ר traces its one-run printed form and records the rounder cursive hook", () => {
    const src = HEBREW_RESH.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:26.2–03:27.1.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Resh.*one continuous run.*03:26.2–03:27.1.*top bar.*left-to-right.*rounds the top-right corner.*right side.*without lifting.*cursive Resh.*purple.*rounder hook.*03:29.3–03:30.0.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ש traces its two-run printed form and records the cursive loop", () => {
    const src = HEBREW_SHIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:34.0–03:36.3.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Shin.*two runs.*03:34.0–03:36.3.*top of the right branch.*descends.*outer U-shaped base.*climbs the left branch.*without lifting.*one lift.*descends the middle branch.*cursive Shin.*purple.*one rounded inward loop.*short rightward exit.*03:39.2–03:40.2.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ת traces its two-run printed form and records the cursive retrace", () => {
    const src = HEBREW_TAV.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:45.2–03:47.3.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Tav.*two runs.*03:45.2–03:47.3.*top bar.*left-to-right.*right side.*without lifting.*one lift.*separate left leg.*curves left.*small foot.*cursive Tav.*purple.*one continuous run.*03:49.0–03:50.9.*descend the left stem.*curl left.*retrace.*climb.*arch right.*short right side.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
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
