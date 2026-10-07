// ---------------------------------------------------------------------------
// Latin pen paths: geometry against the font, order against the sources
// ---------------------------------------------------------------------------
//
// Two different questions are asked of every Latin letter here.
//
//   1. Does the path lie on the printed letter? `registerStrokeHonestyTests`
//      measures it against the Noto Sans Latin letters bundled in
//      NotoSansDevanagari-Static.ttf at the DEFAULT tolerances (no per-letter
//      override): every stroke at least 97% on ink, joins closed, at most 2%
//      of the ink untraced.
//   2. Does the path say what its source says? The letters follow the
//      Grundschrift-App's ordered paths (a school model; facts only, the
//      repository has no licence), and ñ's tilde, ¿ and ¡ follow the majority
//      of native Spanish writers in UJIpenchars2 (CC BY 4.0). The facts pinned
//      below are the ones a reader of those sources can check: how many times
//      the pen lifts, where it starts, which way it turns, and that a dot or a
//      tilde comes last.
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
] as const;
/** The letters cited to the Grundschrift-App; the rest cite UJIpenchars2. */
const SCHOOL = ["b", "c", "e", "g", "h", "i", "l", "n", "o", "r", "s", "u", "w", "ß", "G"] as const;
const NATIVE = ["ñ", "¿", "¡"] as const;
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
        "_fonts/NotoSansDevanagari-Static.ttf",
      );
    }
  });

  it("authors exactly the 18 glyphs the Spanish and German strips need, in owner order", () => {
    expect(letters.map((letter) => letter.glyph)).toEqual([...GLYPHS]);
    expect(Object.keys(DUCTUS).filter((key) => key.startsWith("latin:"))).toEqual(
      GLYPHS.map((glyph) => `latin:${glyph}`),
    );
  });

  it("draws no a: Noto prints a two-storey a, and every source draws the one-storey a", () => {
    for (const glyph of ["a", "á", "A"]) {
      expect(DUCTUS[ductusKey("latin", glyph)], glyph).toBeUndefined();
    }
    // Nor a mark the sources do not cover, nor one whose only lesson holds an a.
    for (const glyph of ["à", "â", "ç", "æ", "œ", "ë", "ï", "ä", "ö", "ü", "é", "í", "ó", "ú"]) {
      expect(DUCTUS[ductusKey("latin", glyph)], glyph).toBeUndefined();
    }
  });

  it("lifts the pen as often as its source", () => {
    expect(Object.fromEntries(GLYPHS.map((glyph) => [glyph, penLifts(LA(glyph))]))).toEqual({
      b: 0, c: 0, e: 0, g: 0, h: 0, i: 1, l: 0, n: 0, o: 0, r: 0, s: 0, u: 0, w: 0,
      "ß": 0, "ñ": 1, G: 0, "¿": 1, "¡": 1,
    });
  });

  it("puts the dot, or the tilde, last and above the body", () => {
    for (const glyph of ["i", "¿", "¡", "ñ"]) {
      const letter = LA(glyph);
      expect(letter.strokes, glyph).toHaveLength(2);
      const [body, mark] = letter.strokes.map((stroke) => penPath(stroke));
      expect(meanY(mark), glyph).toBeGreaterThan(meanY(body));
      expect(letter.strokes[1].segments[0].label, glyph).toMatch(/^lift, then the /);
    }
    // ¡'s bar runs down; ñ's tilde runs left to right.
    expect(last(LA("¡")).y).toBeLessThan(first(LA("¡")).y);
    expect(last(LA("ñ"), 1).x - first(LA("ñ"), 1).x).toBeGreaterThan(200);
  });

  it("turns each round letter the way its source does", () => {
    // Anticlockwise: c, o, e's curve, G's curve, the bowl of g, the hook of ¿.
    expect(turning(penPath(LA("c").strokes[0]))).toBeGreaterThan(150);
    expect(turning(penPath(LA("o").strokes[0]))).toBeGreaterThan(300);
    expect(turning(segmentPath(LA("e"), 0, 1))).toBeGreaterThan(250);
    expect(turning(segmentPath(LA("G"), 0, 0))).toBeGreaterThan(90);
    expect(turning(segmentPath(LA("G"), 0, 1))).toBeGreaterThan(150);
    expect(turning(segmentPath(LA("g"), 0, 0))).toBeGreaterThan(180);
    expect(turning(penPath(LA("¿").strokes[0]))).toBeGreaterThan(90);
    // s: anticlockwise over the top, then clockwise round the bottom.
    const s = penPath(LA("s").strokes[0]);
    const middle = Math.floor(s.length / 2);
    expect(turning(s.slice(0, middle + 1))).toBeGreaterThan(90);
    expect(turning(s.slice(middle))).toBeLessThan(-90);
    // Clockwise: b's bowl, the arch of h and n, ß over the top and round.
    expect(turning(segmentPath(LA("b"), 0, 1))).toBeLessThan(-250);
    expect(turning(segmentPath(LA("h"), 0, 1))).toBeLessThan(-150);
    expect(turning(segmentPath(LA("n"), 0, 1))).toBeLessThan(-150);
    expect(turning(segmentPath(LA("ß"), 0, 1))).toBeLessThan(-150);
  });

  it("starts each letter where its source starts", () => {
    // Top right: c, s, G, and g's bowl.
    for (const glyph of ["c", "s", "G", "g"]) {
      const start = first(LA(glyph));
      expect(start.x, glyph).toBeGreaterThan(350);
      expect(start.y, glyph).toBeGreaterThan(450);
    }
    // Top left, straight down: the stems of b h i l n r u, and w's first line.
    for (const glyph of ["b", "h", "i", "l", "n", "r", "u", "w"]) {
      const letter = LA(glyph);
      expect(first(letter).x, glyph).toBeLessThan(150);
      expect(first(letter).y, glyph).toBeGreaterThan(450);
      // The pen sets off downward.
      expect(segmentPath(letter, 0, 0)[1].y, glyph).toBeLessThan(first(letter).y);
    }
    // o from the top; e from the middle left, along the bar to the right.
    expect(first(LA("o")).y).toBeGreaterThan(480);
    expect(first(LA("e")).x).toBeLessThan(150);
    expect(Math.abs(first(LA("e")).y - 280)).toBeLessThan(40);
    expect(segmentPath(LA("e"), 0, 0).at(-1)!.x).toBeGreaterThan(400);
    // ß from the foot, upward; it ends at the lower left of its bowl.
    expect(first(LA("ß")).y).toBeLessThan(60);
    expect(segmentPath(LA("ß"), 0, 0).at(-1)!.y).toBeGreaterThan(500);
    expect(last(LA("ß")).y).toBeLessThan(60);
    // G ends in along its bar; g ends at the left of its tail.
    expect(last(LA("G")).x).toBeLessThan(first(LA("G")).x - 100);
    expect(Math.abs(last(LA("G")).y - 340)).toBeLessThan(30);
    expect(last(LA("g")).y).toBeLessThan(-100);
    expect(last(LA("g")).x).toBeLessThan(200);
    // ¿'s hook starts at its top and ends at the lower right.
    expect(first(LA("¿")).y).toBeGreaterThan(250);
    expect(last(LA("¿")).y).toBeLessThan(-100);
    expect(last(LA("¿")).x).toBeGreaterThan(first(LA("¿")).x);
  });

  it("runs back along its own ink where Noto joins what the source draws in one stroke", () => {
    for (const glyph of ["b", "h", "n", "r", "ñ"]) {
      expect(LA(glyph).strokes[0].segments[1].label, glyph).toMatch(/^back up/);
    }
    expect(LA("g").strokes[0].segments[1].label).toBe("back up, then down and hook left");
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
      expect(source.variation).toMatch(/no point is copied, and the path is fitted to the Noto Sans .* bundled in NotoSansDevanagari-Static\.ttf\. Handwriting varies by writer\.$/);
      if (glyph === "ß") {
        expect(source.variation).toMatch(/has no ß, so no native-writer count is cited/);
      } else {
        expect(source.variation).toMatch(/UJIpenchars2 \(Prat, Castro, Llorens, Marzal and Vilar; UCI Machine Learning Repository dataset 177, CC BY 4\.0\) holds 120 tablet pen traces/);
      }
    }
    expect(LA("ß").source.url).toMatch(/kleinbuchstaben\/sz\/metadata\.json$/);
    expect(LA("G").source.url).toMatch(/GROSSBUCHSTABEN\/G\/metadata\.json$/);
    for (const glyph of NATIVE) {
      const source = LA(glyph).source;
      expect(source.url).toBe(UJI);
      expect(source.citation).toMatch(
        new RegExp(`^UJIpenchars2 \\(F\\. Prat, M\\. J\\. Castro, D\\. Llorens, A\\. Marzal and J\\. M\\. Vilar\\), UCI Machine Learning Repository dataset 177, class ${glyph}: 120 tablet pen traces by 60 native Spanish writers \\(CC BY 4\\.0\\)`),
      );
      expect(source.variation).toMatch(/Only counts and shares are cited; no trace is copied/);
    }
    // ñ's n is the school model's n.
    expect(LA("ñ").source.citation).toContain("the n after the Grundschrift-App, level kleinbuchstaben/n");
    expect(LA("ñ").source.variation).toMatch(/108 of the 120, and in all 108 it is drawn last.*104 of those 108 draw it from left to right/);
  });
});
