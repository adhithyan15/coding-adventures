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
const GLYPHS = [
  "এ", "ও", "খ", "থ", "ঞ", "ব", "র", "ঃ", "ঁ", "ং",
  "ই", "চ", "ছ", "জ", "ড", "ত", "দ", "ন", "ফ", "ভ", "ম", "য", "ল", "হ",
] as const;

// The letters whose traces give the headline NO majority placement, so the
// body follows the majority and the headline is drawn last, left to right, by
// the documented convention (see src/strokes/bengali.ts). This list is the
// whole of the convention's reach: a letter joins it only with its counts.
const HEADLINE_LAST_BY_CONVENTION = [
  "ই", "চ", "ছ", "জ", "ড", "ত", "দ", "ন", "ফ", "ভ", "ম", "য", "ল", "হ",
] as const;
const HEADLINE_LAST = "lift, then draw the headline last, from left to right";

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

  it("authors exactly the twenty-four glyphs whose traces agree, in owner order", () => {
    expect(letters.map((letter) => letter.glyph)).toEqual([...GLYPHS]);
    expect(Object.keys(DUCTUS).filter((key) => key.startsWith("bengali:"))).toEqual(
      GLYPHS.map((glyph) => `bengali:${glyph}`),
    );
  });

  it("lifts the pen exactly as often as the modal native count says", () => {
    expect(Object.fromEntries(GLYPHS.map((glyph) => [glyph, penLifts(BN(glyph))]))).toEqual({
      "এ": 0, "ও": 0, "খ": 0, "থ": 0, "ঞ": 1, "ব": 0, "র": 1, "ঃ": 1, "ঁ": 1, "ং": 1,
      // The modal BODY count, plus one lift for the headline drawn last.
      "ই": 2, "চ": 1, "ছ": 1, "জ": 2, "ড": 1, "ত": 1, "দ": 1, "ন": 1, "ফ": 1,
      "ভ": 1, "ম": 1, "য": 1, "ল": 1, "হ": 1,
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

  it("ং draws the ring counterclockwise from its top, then lifts for the tail, down to the right", () => {
    // The commonest native form (71 of 183): ring first, counterclockwise,
    // then the tail from its upper-left end. The order is the weakest claim
    // (103 of 183 ring first), and the record says so.
    expect(labels(BN("ং"))).toEqual([
      ["draw the ring counterclockwise from its top"],
      ["lift, then draw the tail from its upper-left end down to the right"],
    ]);
    const [ring, tail] = BN("ং").strokes.map((stroke) => penPath(stroke));
    expect(signedArea(ring)).toBeGreaterThan(0);
    expect(ring[0].y).toBe(Math.max(...ring.map((p) => p.y)));
    expect(ring.at(-1)).toEqual(ring[0]);
    expect(Math.min(...ring.map((p) => p.y))).toBeGreaterThan(Math.max(...tail.map((p) => p.y)));
    expect(tail.at(-1)!.x).toBeGreaterThan(tail[0].x + 200);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y - 200);
  });

  it("draws the headline last, left to right, exactly where the convention applies", () => {
    for (const glyph of GLYPHS) {
      const letter = BN(glyph);
      const finalStroke = letter.strokes.at(-1)!;
      const byConvention = (HEADLINE_LAST_BY_CONVENTION as readonly string[]).includes(glyph);
      const variation = letter.source.variation ?? "";
      expect(variation.includes("By the convention"), glyph).toBe(byConvention);
      expect(
        finalStroke.segments.length === 1 && finalStroke.segments[0].label === HEADLINE_LAST,
        glyph,
      ).toBe(byConvention);
      if (!byConvention) continue;
      // The record names the case it is in, with its counts.
      expect(variation, glyph).toMatch(/so no placement wins a majority/);
      // One straight line along the printed bar, left to right, across it.
      const bar = penPath(finalStroke);
      expect(new Set(bar.map((p) => p.y)), glyph).toEqual(new Set([586]));
      expect(bar[0].x, glyph).toBeLessThan(30);
      expect(bar.at(-1)!.x - bar[0].x, glyph).toBeGreaterThan(480);
      // Nothing before it is a headline: the body never runs along the bar.
      for (const stroke of letter.strokes.slice(0, -1)) {
        for (const segment of stroke.segments) expect(segment.label, glyph).not.toMatch(/draw the headline/);
      }
    }
    // ব and র, whose majority opens with the headline, keep it first.
    for (const glyph of ["ব", "র"]) {
      expect(BN(glyph).strokes[0].segments[0].label).toBe("draw the headline from left to right");
    }
  });

  it("ত, ভ and ড turn clockwise and end on the left arm; চ turns counterclockwise", () => {
    for (const glyph of ["ত", "ভ", "ড"]) {
      expect(signedArea(penPath(BN(glyph).strokes[0])), glyph).toBeLessThan(0);
      expect(last(BN(glyph)).x, glyph).toBeLessThan(100);
      expect(last(BN(glyph)).y, glyph).toBeGreaterThan(400);
    }
    expect(signedArea(penPath(BN("চ").strokes[0]))).toBeGreaterThan(0);
    // ত and ভ start inside the curl; ড and চ at the stem under the headline.
    expect(first(BN("ত")).x).toBeGreaterThan(250);
    expect(first(BN("ভ")).x).toBeGreaterThan(250);
    expect(first(BN("ড")).y).toBeGreaterThan(550);
    expect(first(BN("চ")).y).toBeGreaterThan(550);
  });

  it("ন, ম, য and ল reach the foot of the stem, then climb it to the headline", () => {
    for (const glyph of ["ন", "ম", "য", "ল"]) {
      const body = penPath(BN(glyph).strokes[0]);
      const footIndex = body.findIndex((p) => p.y < 60);
      expect(footIndex, glyph).toBeGreaterThan(0);
      expect(body.at(-1)!.y, glyph).toBeGreaterThan(550);
      expect(labels(BN(glyph))[0].at(-1), glyph).toBe("climb the stem to the headline");
    }
    // ন and ল start inside the curl at the lower left; ম and য under the headline.
    expect(first(BN("ন")).y).toBeLessThan(300);
    expect(first(BN("ল")).y).toBeLessThan(300);
    expect(first(BN("ম")).y).toBeGreaterThan(550);
    expect(first(BN("য")).y).toBeGreaterThan(550);
  });

  it("ম turns its loop clockwise", () => {
    const loop = BN("ম").strokes[0].segments.find((segment) => segment.label === "round the loop clockwise")!;
    expect(signedArea(loop.path)).toBeLessThan(0);
  });

  it("জ draws its left part first, clockwise, then its right part down to the foot", () => {
    const [left, right] = BN("জ").strokes.map((stroke) => penPath(stroke));
    expect(signedArea(left)).toBeLessThan(0);
    expect(left.at(-1)!.x).toBeLessThan(150);
    expect(right[0].y).toBeGreaterThan(550);
    expect(right.at(-1)!.x).toBeGreaterThan(650);
    expect(right.at(-1)!.y).toBeLessThan(50);
  });

  it("ছ, দ and হ end at the bottom right; ফ ends inside its loop", () => {
    for (const glyph of ["ছ", "দ", "হ"]) {
      expect(last(BN(glyph)).y, glyph).toBeLessThan(60);
      expect(last(BN(glyph)).x, glyph).toBeGreaterThan(380);
    }
    expect(last(BN("ফ")).x).toBeGreaterThan(550);
    expect(last(BN("ফ")).y).toBeGreaterThan(200);
  });

  it("ই is হ's body, then the hook climbed from the headline to its tip", () => {
    expect(BN("ই").strokes[0]).toEqual(BN("হ").strokes[0]);
    const hook = penPath(BN("ই").strokes[1]);
    expect(hook[0].y).toBeLessThan(600);
    expect(hook.at(-1)!.y).toBe(Math.max(...hook.map((p) => p.y)));
    expect(hook.at(-1)!.x).toBeLessThan(100);
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
