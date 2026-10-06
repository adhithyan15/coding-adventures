// ---------------------------------------------------------------------------
// Bengali pen paths: geometry against the font, order against the evidence
// ---------------------------------------------------------------------------
//
// Two different questions are asked of every Bengali letter here.
//
//   1. Does the path lie on the printed letter? `registerStrokeHonestyTests`
//      measures it against Noto Sans Bengali at the DEFAULT tolerances (no
//      per-letter override): every stroke at least 97% on ink, joins closed,
//      at most 2% of the ink untraced.
//   2. Does the path say what the native writers' traces say? The order, the
//      start, the turning and the lift count are pinned below, in terms a
//      reader of the citation can check: "starts inside the curl", "turns
//      clockwise", "the headline comes first".
//
// Turning is measured with the shoelace formula on the pen path in FONT
// coordinates (y up), where a positive signed area means COUNTERclockwise as
// seen on the page — the reverse of the y-down convention of the traces.
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

const BN = (glyph: string): LetterDuctus => DUCTUS[ductusKey("bengali", glyph)];
const GLYPHS = ["এ", "ও", "খ", "থ", "ঞ", "ব", "র", "ঃ", "ঁ"] as const;

const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter(
  (letter) => letter.script === "bengali",
);

/** Signed area of a closed polyline; > 0 is counterclockwise on the page. */
const signedArea = (points: Point[]): number => {
  let sum = 0;
  for (let i = 0; i < points.length; i++) {
    const a = points[i];
    const b = points[(i + 1) % points.length];
    sum += a.x * b.y - b.x * a.y;
  }
  return sum / 2;
};
const labels = (letter: LetterDuctus): string[][] =>
  letter.strokes.map((stroke) => stroke.segments.map((segment) => segment.label));
const first = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke])[0];
const last = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke]).at(-1)!;

describe("Bengali handwriting ductus", () => {
  // Default tolerances for every letter: no overrides.
  registerStrokeHonestyTests(letters);

  beforeAll(() => {
    for (const glyph of GLYPHS) {
      expect(verifiedLetterFont(glyph, BN(glyph).source.url)).toBe(
        "_fonts/NotoSansBengali-Static.ttf",
      );
    }
  });

  it("authors exactly the nine glyphs whose traces agree, in owner order", () => {
    expect(letters.map((letter) => letter.glyph)).toEqual([...GLYPHS]);
    expect(Object.keys(DUCTUS).filter((key) => key.startsWith("bengali:"))).toEqual(
      GLYPHS.map((glyph) => `bengali:${glyph}`),
    );
  });

  it("lifts the pen exactly as often as the modal native count says", () => {
    expect(Object.fromEntries(GLYPHS.map((glyph) => [glyph, penLifts(BN(glyph))]))).toEqual({
      "এ": 0, "ও": 0, "খ": 0, "থ": 0, "ঞ": 1, "ব": 0, "র": 1, "ঃ": 1, "ঁ": 1,
    });
  });

  it("ব draws the headline first, left to right, and climbs the stem last", () => {
    const [bar, , , stem] = BN("ব").strokes[0].segments;
    expect(bar.label).toBe("draw the headline from left to right");
    expect(bar.path[0].x).toBeLessThan(bar.path.at(-1)!.x);
    expect(Math.min(...bar.path.map((p) => p.y))).toBeGreaterThan(550);
    expect(stem.path[0].y).toBeLessThan(50);
    expect(last(BN("ব")).y).toBeGreaterThan(550);
  });

  it("র is ব's body, then a lift and the dot below", () => {
    expect(BN("র").strokes[0]).toEqual(BN("ব").strokes[0]);
    expect(labels(BN("র"))[1]).toEqual(["lift, then place the dot below"]);
    expect(Math.max(...penPath(BN("র").strokes[1]).map((p) => p.y))).toBeLessThan(120);
  });

  it("খ and থ start inside the curl, turn opposite ways, and end in the flag", () => {
    const curl = (glyph: string) => BN(glyph).strokes[0].segments[0].path;
    expect(signedArea(curl("খ"))).toBeGreaterThan(0); // counterclockwise
    expect(signedArea(curl("থ"))).toBeLessThan(0); // clockwise
    for (const glyph of ["খ", "থ"]) {
      expect(first(BN(glyph)).x).toBeLessThan(200);
      const end = last(BN(glyph));
      expect(end.x).toBeGreaterThan(600);
      expect(end.y).toBeGreaterThan(550);
      expect(labels(BN(glyph))[0].at(-1)).toBe("climb the stem and turn right into the flag");
    }
  });

  it("এ, ও and ঞ's left part turn clockwise from an inner curl and end on the left", () => {
    for (const letter of [BN("এ"), BN("ও"), BN("ঞ")]) {
      expect(signedArea(penPath(letter.strokes[0]))).toBeLessThan(0);
      expect(last(letter).x).toBeLessThan(150);
    }
    expect(BN("ঞ").strokes[0]).toEqual(BN("এ").strokes[0]);
  });

  it("ঞ's second stroke starts at the stem and turns clockwise", () => {
    const right = penPath(BN("ঞ").strokes[1]);
    expect(right[0].x).toBeGreaterThan(first(BN("ঞ")).x);
    expect(signedArea(right)).toBeLessThan(0);
    expect(right.at(-1)!.y).toBeLessThan(316);
  });

  it("ঃ draws the upper loop first, each loop counterclockwise from its top", () => {
    const [upper, lower] = BN("ঃ").strokes.map((stroke) => penPath(stroke));
    expect(Math.min(...upper.map((p) => p.y))).toBeGreaterThan(Math.max(...lower.map((p) => p.y)));
    for (const loop of [upper, lower]) {
      expect(signedArea(loop)).toBeGreaterThan(0);
      expect(loop[0].y).toBe(Math.max(...loop.map((p) => p.y)));
    }
  });

  it("ঁ draws the bowl left to right through its bottom, then the dot", () => {
    const bowl = penPath(BN("ঁ").strokes[0]);
    expect(bowl[0].x).toBeLessThan(0);
    expect(bowl.at(-1)!.x).toBeGreaterThan(0);
    expect(signedArea(bowl)).toBeGreaterThan(0);
    expect(labels(BN("ঁ"))[1]).toEqual(["lift, then place the dot"]);
  });

  it("every Bengali order traces to LipiTk's Bangla recognizer, with counts", () => {
    for (const glyph of GLYPHS) {
      const source = BN(glyph).source;
      expect(source.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
      expect(source.citation).toMatch(
        /^HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4\.0, Bangla recognizer, class \d+ .*native Bengali writers \(MIT licence, 2012\)$/,
      );
      expect(source.variation).toMatch(/\d+ of the \d+ stored prototypes|All \d+ stored prototypes/);
      expect(source.variation).toMatch(/scaled to a square.*Noto Sans Bengali.*varies by writer/);
    }
  });
});
