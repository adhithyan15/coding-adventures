// ---------------------------------------------------------------------------
// Latin pen paths: geometry against the font, order against the sources
// ---------------------------------------------------------------------------
//
// Two different questions are asked of every Latin letter here.
//
//   1. Does the path lie on the printed letter? `registerStrokeHonestyTests`
//      measures it against LatinPrint-Subset.ttf (a renamed subset of SIL's
//      literacy typeface Andika, whose a is the one-storey a the sources
//      teach) at the DEFAULT tolerances (no per-letter override): every stroke
//      at least 97% on ink, joins closed, at most 2% of the ink untraced.
//   2. Does the path say what its source says? The letters follow the
//      Grundschrift-App's ordered paths (a school model; facts only, the
//      repository has no licence), and the marks of ñ, á é í ó ú and ü, and ¿
//      and ¡, follow the majority of native Spanish writers in UJIpenchars2
//      (CC BY 4.0). The facts pinned below are the ones a reader of those
//      sources can check: how many times the pen lifts, where it starts, which
//      way it turns, and that a dot, a tilde or an accent comes last.
//
// The coordinates below are the font's (1000 units to the em, y up): the
// x-height is about 500, the baseline 0, ascenders reach about 780 and
// descenders about -230.
//
// Turning is the sum of the pen's heading changes along a stroke, in FONT
// coordinates (y up), where a left turn is positive. So a POSITIVE sum is
// ANTICLOCKWISE as seen on the page. A turn of half a circle on the spot (the
// pen going back the way it came) is skipped: it has no sense of rotation.
// ---------------------------------------------------------------------------

import { beforeAll, describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../src/scriptdata";
import {
  DUCTUS,
  ductusKey,
  penLifts,
  penPath,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";
import { registerStrokeHonestyTests } from "../support/stroke-honesty";

const LA = (glyph: string): LetterDuctus => DUCTUS[ductusKey("latin", glyph)];
const GLYPHS = [
  "b", "c", "e", "g", "h", "i", "l", "n", "o", "r", "s", "u", "w", "ß", "ñ", "G", "¿", "¡",
  "a", "d", "p", "q", "t", "y", "H", "á", "é", "í", "ó", "ú", "ü",
  "v", "m", "R", "è", "ê", "ë", "ï", "ä", "ö", "ē", "ç",
] as const;
/** The letters cited to the Grundschrift-App; the rest cite UJIpenchars2. */
const SCHOOL = [
  "b", "c", "e", "g", "h", "i", "l", "n", "o", "r", "s", "u", "w", "ß", "G",
  "a", "d", "p", "q", "t", "y", "H", "v", "m", "R",
] as const;
/** Letters added when UJIpenchars2 was out of reach: no native-writer count. */
const SCHOOL_ONLY = ["v", "m", "R"] as const;
const NATIVE = ["ñ", "¿", "¡", "á", "é", "í", "ó", "ú", "ü"] as const;
/** Each precomposed letter and the cited letter whose path it begins with. */
const BASE: Record<string, string> = { "ñ": "n", "á": "a", "é": "e", "í": "i", "ó": "o", "ú": "u", "ü": "u" };
/**
 * The letters drawn BY ANALOGY, not separately sourced (the coordinator's
 * decision for the French, German and Latin lessons): each one's cited base
 * letter, then its mark last, as the cited ü, acute and tilde are drawn.
 */
const ANALOGY: Record<string, string> = {
  "è": "e", "ê": "e", "ë": "e", "ï": "i", "ä": "a", "ö": "o", "ē": "e", "ç": "c",
};
const GRUNDSCHRIFT =
  /^https:\/\/github\.com\/Medien-Treibhaus\/grundschrift-app-source\/blob\/f6dbd807adbb3fc2f94207fe578def439f6e9c49\/assets\/levels\/(kleinbuchstaben|GROSSBUCHSTABEN)\/(\w+)\/metadata\.json$/;
const UJI = "https://archive.ics.uci.edu/dataset/177/uji+pen+characters+version+2";

const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter(
  (letter) => letter.script === "latin",
);

/** Total turning of a polyline in degrees; > 0 is anticlockwise on the page. */
const turning = (points: Point[]): number => {
  let total = 0;
  for (let i = 0; i + 2 < points.length; i++) {
    const [a, b, c] = [points[i], points[i + 1], points[i + 2]];
    const h1 = Math.atan2(b.y - a.y, b.x - a.x);
    const h2 = Math.atan2(c.y - b.y, c.x - b.x);
    const d = ((h2 - h1 + 3 * Math.PI) % (2 * Math.PI)) - Math.PI;
    if (Math.abs(Math.abs(d) - Math.PI) < 0.05) continue;
    total += d;
  }
  return (total * 180) / Math.PI;
};
const segmentPath = (letter: LetterDuctus, stroke: number, segment: number): Point[] =>
  letter.strokes[stroke].segments[segment].path;
const first = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke])[0];
const last = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke]).at(-1)!;
const meanY = (points: Point[]): number => points.reduce((sum, p) => sum + p.y, 0) / points.length;

describe("Latin handwriting ductus", () => {
  // Default tolerances for every letter: no overrides.
  registerStrokeHonestyTests(letters);

  beforeAll(() => {
    for (const glyph of GLYPHS) {
      expect(verifiedLetterFont(glyph, LA(glyph).source.url)).toBe(
        "_fonts/LatinPrint-Subset.ttf",
      );
    }
  });

  it("authors exactly the 42 glyphs (34 cited, 8 by analogy), in owner order", () => {
    expect(letters.map((letter) => letter.glyph)).toEqual([...GLYPHS]);
    expect(Object.keys(DUCTUS).filter((key) => key.startsWith("latin:"))).toEqual(
      GLYPHS.map((glyph) => `latin:${glyph}`),
    );
  });

  it("draws no marked letter beyond the cited ones and the eight by analogy", () => {
    for (const glyph of ["à", "â", "î", "ô", "û", "ù", "ã", "õ", "ā", "æ", "œ", "ÿ", "Ä", "Ö", "Ü", "É", "Ç"]) {
      expect(DUCTUS[ductusKey("latin", glyph)], glyph).toBeUndefined();
    }
  });

  it("lifts the pen as often as its source", () => {
    expect(Object.fromEntries(GLYPHS.map((glyph) => [glyph, penLifts(LA(glyph))]))).toEqual({
      b: 0, c: 0, e: 0, g: 0, h: 0, i: 1, l: 0, n: 0, o: 0, r: 0, s: 0, u: 0, w: 0,
      "ß": 0, "ñ": 1, G: 0, "¿": 1, "¡": 1,
      a: 0, d: 0, p: 0, q: 0, t: 1, y: 1, H: 2,
      "á": 1, "é": 1, "í": 1, "ó": 1, "ú": 1, "ü": 2,
      v: 0, m: 0, R: 1,
      "è": 1, "ê": 1, "ë": 2, "ï": 2, "ä": 2, "ö": 2, "ē": 1, "ç": 1,
    });
  });

  it("puts the dot, the tilde or the accent last and above the body", () => {
    for (const glyph of ["i", "¿", "¡", "ñ", "á", "é", "í", "ó", "ú"]) {
      const letter = LA(glyph);
      expect(letter.strokes, glyph).toHaveLength(2);
      const [body, mark] = letter.strokes.map((stroke) => penPath(stroke));
      expect(meanY(mark), glyph).toBeGreaterThan(meanY(body));
      expect(letter.strokes[1].segments[0].label, glyph).toMatch(/^lift, then the /);
    }
    // ¡'s bar runs down; ñ's tilde runs left to right.
    expect(last(LA("¡")).y).toBeLessThan(first(LA("¡")).y);
    expect(last(LA("ñ"), 1).x - first(LA("ñ"), 1).x).toBeGreaterThan(200);
    // Every acute rises to the right, the majority's way.
    for (const glyph of ["á", "é", "í", "ó", "ú"]) {
      const [start, end] = [first(LA(glyph), 1), last(LA(glyph), 1)];
      expect(end.x - start.x, glyph).toBeGreaterThan(100);
      expect(end.y - start.y, glyph).toBeGreaterThan(100);
      expect(LA(glyph).strokes[1].segments[0].label, glyph).toBe("lift, then the acute, up to the right");
    }
  });

  it("writes a precomposed letter as its base letter's own path, then the mark", () => {
    for (const [glyph, base] of Object.entries(BASE)) {
      expect(LA(glyph).strokes[0], glyph).toEqual(LA(base).strokes[0]);
    }
    // ü: the u, then the left dot, then the right dot, both above the u.
    const u = LA("ü");
    expect(u.strokes).toHaveLength(3);
    expect(u.strokes[1].segments[0].label).toBe("lift, then the left dot");
    expect(u.strokes[2].segments[0].label).toBe("lift, then the right dot");
    expect(first(u, 1).x).toBeLessThan(first(u, 2).x - 100);
    for (const dot of [1, 2]) {
      expect(meanY(penPath(u.strokes[dot]))).toBeGreaterThan(meanY(penPath(u.strokes[0])));
    }
  });

  it("turns each round letter the way its source does", () => {
    // Anticlockwise: c, o, e's curve, G's curve, the bowls of a d g q, the hook of ¿.
    expect(turning(penPath(LA("c").strokes[0]))).toBeGreaterThan(150);
    expect(turning(penPath(LA("o").strokes[0]))).toBeGreaterThan(300);
    expect(turning(segmentPath(LA("e"), 0, 1))).toBeGreaterThan(250);
    expect(turning(segmentPath(LA("G"), 0, 0))).toBeGreaterThan(90);
    expect(turning(segmentPath(LA("G"), 0, 1))).toBeGreaterThan(150);
    for (const glyph of ["a", "d", "g", "q"]) {
      expect(turning(segmentPath(LA(glyph), 0, 0)), glyph).toBeGreaterThan(180);
    }
    expect(turning(penPath(LA("¿").strokes[0]))).toBeGreaterThan(90);
    // s: anticlockwise over the top, then clockwise round the bottom.
    const s = penPath(LA("s").strokes[0]);
    const middle = Math.floor(s.length / 2);
    expect(turning(s.slice(0, middle + 1))).toBeGreaterThan(90);
    expect(turning(s.slice(middle))).toBeLessThan(-90);
    // Clockwise: the bowls of b and p, the arch of h and n, ß over the top and round.
    expect(turning(segmentPath(LA("b"), 0, 1))).toBeLessThan(-250);
    expect(turning(segmentPath(LA("p"), 0, 1))).toBeLessThan(-250);
    expect(turning(segmentPath(LA("h"), 0, 1))).toBeLessThan(-150);
    expect(turning(segmentPath(LA("n"), 0, 1))).toBeLessThan(-150);
    expect(turning(segmentPath(LA("ß"), 0, 1))).toBeLessThan(-150);
  });

  it("starts each letter where its source starts", () => {
    // Top right: c, s, G, and the bowls of a, d, g and q.
    for (const glyph of ["c", "s", "G", "g", "a", "d", "q"]) {
      const start = first(LA(glyph));
      expect(start.x, glyph).toBeGreaterThan(350);
      expect(start.y, glyph).toBeGreaterThan(400);
    }
    // Top left, straight down: the stems of b h i l n p r u, w's first line,
    // and the left stem of H.
    for (const glyph of ["b", "h", "i", "l", "n", "p", "r", "u", "w", "H"]) {
      const letter = LA(glyph);
      expect(first(letter).x, glyph).toBeLessThan(150);
      expect(first(letter).y, glyph).toBeGreaterThan(450);
      // The pen sets off downward.
      expect(segmentPath(letter, 0, 0)[1].y, glyph).toBeLessThan(first(letter).y);
    }
    // o from the top; e from the middle left, along the bar to the right.
    expect(first(LA("o")).y).toBeGreaterThan(440);
    expect(first(LA("e")).x).toBeLessThan(150);
    expect(Math.abs(first(LA("e")).y - 280)).toBeLessThan(40);
    expect(segmentPath(LA("e"), 0, 0).at(-1)!.x).toBeGreaterThan(400);
    // ß from the foot, upward; it ends at the lower left of its bowl.
    expect(first(LA("ß")).y).toBeLessThan(60);
    expect(segmentPath(LA("ß"), 0, 0).at(-1)!.y).toBeGreaterThan(500);
    expect(last(LA("ß")).y).toBeLessThan(60);
    // G ends in along its bar; g ends at the left of its tail.
    expect(last(LA("G")).x).toBeLessThan(first(LA("G")).x - 100);
    expect(Math.abs(last(LA("G")).y - 325)).toBeLessThan(30);
    expect(last(LA("g")).y).toBeLessThan(-100);
    expect(last(LA("g")).x).toBeLessThan(200);
    // ¿'s hook starts at its top and ends at the lower right.
    expect(first(LA("¿")).y).toBeGreaterThan(250);
    expect(last(LA("¿")).y).toBeLessThan(-100);
    expect(last(LA("¿")).x).toBeGreaterThan(first(LA("¿")).x);
  });

  it("ends a, d, p and q where their stems end, after going back up", () => {
    // a and d come down to the foot of the stem on the right; q below the line.
    for (const glyph of ["a", "d"]) {
      expect(last(LA(glyph)).y, glyph).toBeLessThan(100);
      expect(last(LA(glyph)).x, glyph).toBeGreaterThan(400);
    }
    expect(last(LA("q")).y).toBeLessThan(-150);
    // d goes up to the top of its ascender before it comes down.
    expect(Math.max(...segmentPath(LA("d"), 0, 1).map((point) => point.y))).toBeGreaterThan(650);
    // p goes down below the line first, then back up and round the bowl.
    expect(segmentPath(LA("p"), 0, 0).at(-1)!.y).toBeLessThan(-150);
  });

  it("draws t, y and H stroke by stroke in the school model's order", () => {
    // t: the stem from the top, then the crossbar, level, left to right.
    const t = LA("t");
    expect(first(t).y).toBeGreaterThan(550);
    expect(last(t).x).toBeGreaterThan(first(t).x + 100);
    expect(Math.abs(last(t, 1).y - first(t, 1).y)).toBeLessThan(20);
    expect(last(t, 1).x - first(t, 1).x).toBeGreaterThan(200);
    // y: the short line from the top left, then the long line from the top
    // right down to the tail at the lower left.
    const y = LA("y");
    expect(first(y).x).toBeLessThan(150);
    expect(last(y).x).toBeGreaterThan(first(y).x);
    expect(last(y).y).toBeLessThan(first(y).y - 300);
    expect(first(y, 1).x).toBeGreaterThan(350);
    expect(last(y, 1).y).toBeLessThan(-100);
    expect(last(y, 1).x).toBeLessThan(150);
    // H: the left stem down, the crossbar left to right, the right stem down.
    const H = LA("H");
    expect(H.strokes).toHaveLength(3);
    expect(last(H).y).toBeLessThan(first(H).y);
    expect(Math.abs(last(H, 1).y - first(H, 1).y)).toBeLessThan(20);
    expect(last(H, 1).x - first(H, 1).x).toBeGreaterThan(300);
    expect(first(H, 2).x).toBeGreaterThan(450);
    expect(last(H, 2).y).toBeLessThan(first(H, 2).y);
  });

  it("runs back along its own ink where the outline joins what the source draws in one stroke", () => {
    for (const glyph of ["b", "h", "n", "p", "r", "ñ"]) {
      expect(LA(glyph).strokes[0].segments[1].label, glyph).toMatch(/^back up/);
    }
    expect(LA("g").strokes[0].segments[1].label).toBe("back up, then down and hook left");
    expect(LA("a").strokes[0].segments[1].label).toBe("back up, then down to the foot");
    expect(LA("q").strokes[0].segments[1].label).toBe("back up, then down the stem");
    expect(LA("d").strokes[0].segments[1].label).toBe("up the stem to the top, then down to the foot");
  });

  it("cites the Grundschrift-App per level, or UJIpenchars2 per class, with counts in every variation", () => {
    for (const glyph of SCHOOL) {
      const source = LA(glyph).source;
      const match = source.url.match(GRUNDSCHRIFT);
      expect(match, glyph).not.toBeNull();
      const level = `${match![1]}/${match![2]}`;
      expect(source.citation).toBe(
        `Grundschrift-App (Laborschule Bielefeld, Bielefeld University, with the Grundschulverband), level ${level}, ordered stroke paths for ${glyph} (assets/levels/${level}/metadata.json, commit f6dbd80; no licence, facts only)`,
      );
      expect(source.variation).toMatch(/^The Grundschrift-App was made in a research project of the Laborschule/);
      expect(source.variation).toMatch(/no point is copied, and the path is fitted to the .* of the bundled LatinPrint-Subset\.ttf, a renamed subset of SIL Global's literacy typeface Andika 7\.000, which prints the one-storey a the school model teaches; the book's own text face is a different font\. Handwriting varies by writer\.$/);
      if (glyph === "ß") {
        expect(source.variation).toMatch(/has no ß, so no native-writer count is cited/);
      } else if ((SCHOOL_ONLY as readonly string[]).includes(glyph)) {
        expect(source.variation).toMatch(
          new RegExp(`could not be reached when ${glyph} was added, so no native-writer count is cited for it`),
        );
      } else {
        expect(source.variation).toMatch(/UJIpenchars2 \(Prat, Castro, Llorens, Marzal and Vilar; UCI Machine Learning Repository dataset 177, CC BY 4\.0\) holds 120 tablet pen traces/);
      }
    }
    expect(LA("ß").source.url).toMatch(/kleinbuchstaben\/sz\/metadata\.json$/);
    expect(LA("G").source.url).toMatch(/GROSSBUCHSTABEN\/G\/metadata\.json$/);
    expect(LA("H").source.url).toMatch(/GROSSBUCHSTABEN\/H\/metadata\.json$/);
    for (const glyph of NATIVE) {
      const source = LA(glyph).source;
      expect(source.url).toBe(UJI);
      expect(source.citation).toMatch(
        new RegExp(`^UJIpenchars2 \\(F\\. Prat, M\\. J\\. Castro, D\\. Llorens, A\\. Marzal and J\\. M\\. Vilar\\), UCI Machine Learning Repository dataset 177, class ${glyph}: 120 tablet pen traces by 60 native Spanish writers \\(CC BY 4\\.0\\)`),
      );
      expect(source.variation).toMatch(/Only counts and shares are cited; no trace is copied/);
      expect(source.variation).toMatch(/LatinPrint-Subset\.ttf/);
    }
    // A precomposed letter's base is the school model's letter.
    for (const [glyph, base] of Object.entries(BASE)) {
      expect(LA(glyph).source.citation, glyph).toContain(
        `the ${base} after the Grundschrift-App, level kleinbuchstaben/${base}`,
      );
    }
    expect(LA("ñ").source.variation).toMatch(/108 of the 120, and in all 108 it is drawn last.*104 of those 108 draw it from left to right/);
    // The acute's direction is split, and the split is said.
    expect(LA("á").source.variation).toMatch(/74 draw it up to the right, 40 down to the left/);
    expect(LA("é").source.variation).toMatch(/75 draw it up to the right, 35 down to the left/);
    expect(LA("ü").source.variation).toMatch(/left dot comes first in 116 of the 118/);
    // Where the adult writers part from the school model, the record says so.
    expect(LA("y").source.variation).toMatch(/ONE stroke \(91 of the 120\)/);
    expect(LA("q").source.variation).toMatch(/89 are two strokes/);
    expect(LA("H").source.variation).toMatch(/second in 37 and third in 25/);
  });

  it("draws v, m and R as the Grundschrift-App does", () => {
    // v: one stroke from the top left, down to the point on the line, up to the top right.
    const v = penPath(LA("v").strokes[0]);
    expect(LA("v").strokes).toHaveLength(1);
    expect(v[0].x).toBeLessThan(150);
    expect(v[0].y).toBeGreaterThan(400);
    const bottom = v.reduce((low, point) => (point.y < low.y ? point : low));
    expect(bottom.y).toBeLessThan(100);
    expect(bottom.x).toBeGreaterThan(200);
    expect(bottom.x).toBeLessThan(320);
    expect(v.at(-1)!.x).toBeGreaterThan(400);
    expect(v.at(-1)!.y).toBeGreaterThan(400);
    // m: one stroke of three movements, the stem down, then two arches, each
    // after going back up the line just drawn; both arches turn clockwise.
    const m = LA("m");
    expect(m.strokes).toHaveLength(1);
    expect(m.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "draw the stem down",
      "back up, over and down",
      "back up, over and down",
    ]);
    expect(first(m).x).toBeLessThan(150);
    expect(first(m).y).toBeGreaterThan(400);
    expect(segmentPath(m, 0, 0).at(-1)!.y).toBeLessThan(100);
    for (const arch of [1, 2]) {
      expect(turning(segmentPath(m, 0, arch)), `arch ${arch}`).toBeLessThan(-150);
      expect(segmentPath(m, 0, arch).at(-1)!.y, `arch ${arch}`).toBeLessThan(100);
    }
    expect(segmentPath(m, 0, 1).at(-1)!.x).toBeGreaterThan(350);
    expect(segmentPath(m, 0, 1).at(-1)!.x).toBeLessThan(450);
    expect(last(m).x).toBeGreaterThan(650);
    // R: the stem down; then from the top of the stem, over the top and
    // clockwise round the bowl, back along its foot and down the leg.
    const R = LA("R");
    expect(R.strokes).toHaveLength(2);
    expect(first(R).x).toBeLessThan(150);
    expect(first(R).y).toBeGreaterThan(600);
    expect(last(R).y).toBeLessThan(100);
    expect(first(R, 1).y).toBeGreaterThan(600);
    expect(first(R, 1).x).toBeLessThan(150);
    expect(turning(segmentPath(R, 1, 0))).toBeLessThan(-150);
    expect(segmentPath(R, 1, 1)[0].x).toBeLessThan(150);
    expect(last(R, 1).x).toBeGreaterThan(500);
    expect(last(R, 1).y).toBeLessThan(100);
    expect(R.strokes[1].segments[1].label).toBe("back along the foot of the bowl and down the leg");
  });

  it("draws each letter by analogy as its cited base letter, then the mark last", () => {
    for (const [glyph, base] of Object.entries(ANALOGY)) {
      const letter = LA(glyph);
      // The base letter's own first stroke, unchanged.
      expect(letter.strokes[0], glyph).toEqual(LA(base).strokes[0]);
      for (const mark of letter.strokes.slice(1)) {
        expect(mark.segments, glyph).toHaveLength(1);
        expect(mark.segments[0].label, glyph).toMatch(/^lift, then the /);
        const markY = meanY(penPath(mark));
        const bodyY = meanY(penPath(letter.strokes[0]));
        // Every mark sits above its letter but the cedilla, which hangs below.
        if (glyph === "ç") expect(markY, glyph).toBeLessThan(0);
        else expect(markY, glyph).toBeGreaterThan(bodyY + 300);
      }
    }
    // The diaeresis: two dots, the left one first, as the cited ü.
    for (const glyph of ["ë", "ï", "ä", "ö"]) {
      const letter = LA(glyph);
      expect(letter.strokes, glyph).toHaveLength(3);
      expect(letter.strokes[1].segments[0].label, glyph).toBe("lift, then the left dot");
      expect(letter.strokes[2].segments[0].label, glyph).toBe("lift, then the right dot");
      expect(first(letter, 1).x, glyph).toBeLessThan(first(letter, 2).x - 100);
    }
    // One mark each, in the direction these records give a mark no cited
    // record covers: left to right and top to bottom.
    const grave = LA("è");
    expect(grave.strokes[1].segments[0].label).toBe("lift, then the grave, down to the right");
    expect(last(grave, 1).x - first(grave, 1).x).toBeGreaterThan(100);
    expect(first(grave, 1).y - last(grave, 1).y).toBeGreaterThan(100);
    const roof = penPath(LA("ê").strokes[1]);
    expect(roof.at(-1)!.x - roof[0].x).toBeGreaterThan(300);
    const peak = roof.reduce((high, point) => (point.y > high.y ? point : high));
    expect(peak.y - roof[0].y).toBeGreaterThan(100);
    expect(peak.y - roof.at(-1)!.y).toBeGreaterThan(100);
    const macron = LA("ē");
    expect(last(macron, 1).x - first(macron, 1).x).toBeGreaterThan(250);
    expect(Math.abs(last(macron, 1).y - first(macron, 1).y)).toBeLessThan(20);
    const cedilla = LA("ç");
    expect(first(cedilla, 1).y).toBeGreaterThan(-20);
    expect(last(cedilla, 1).y).toBeLessThan(-150);
    expect(last(cedilla, 1).x).toBeLessThan(first(cedilla, 1).x);
  });

  it("says plainly that a letter drawn by analogy is not separately sourced, and no other letter says so", () => {
    for (const [glyph, base] of Object.entries(ANALOGY)) {
      const source = LA(glyph).source;
      // Its url is the base letter's Grundschrift level: the only part of the
      // letter that is sourced.
      expect(source.url, glyph).toBe(LA(base).source.url);
      expect(source.citation, glyph).toMatch(/^By analogy with the cited (ü|é and ñ), not separately sourced: /);
      expect(source.variation, glyph).toMatch(/^ORDER BY ANALOGY, NOT SEPARATELY SOURCED\. /);
      expect(source.variation, glyph).toMatch(/only by analogy; no point is copied/);
      expect(source.variation, glyph).toMatch(/LatinPrint-Subset\.ttf/);
    }
    for (const letter of letters.filter((entry) => !(entry.glyph in ANALOGY))) {
      expect(letter.source.citation, letter.glyph).not.toMatch(/analogy/i);
      expect(letter.source.variation, letter.glyph).not.toMatch(/analogy/i);
    }
  });
});
