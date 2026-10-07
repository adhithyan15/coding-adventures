import { describe, expect, it } from "vitest";
import { DUCTUS, ductusKey, penPathD } from "../../src/strokes";
import { ductusFilmstrip, ductusFor, ductusSteps } from "../../src/ductusview";
import { latinOutline } from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

// The filmstrip view of the Latin owner: how many frames each letter gets,
// where the lifts fall, and that the last frame draws the whole cited path.
const EXPECTED: Record<string, { frames: number; lifts: number; summary: string }> = {
  b: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  c: { frames: 1, lifts: 0, summary: "one unbroken stroke · 1 movement" },
  e: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  g: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  h: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  i: { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  l: { frames: 1, lifts: 0, summary: "one unbroken stroke · 1 movement" },
  n: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  o: { frames: 1, lifts: 0, summary: "one unbroken stroke · 1 movement" },
  r: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  s: { frames: 1, lifts: 0, summary: "one unbroken stroke · 1 movement" },
  u: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  w: { frames: 1, lifts: 0, summary: "one unbroken stroke · 1 movement" },
  "ß": { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  "ñ": { frames: 3, lifts: 1, summary: "2 strokes · 1 pen lift · 3 movements" },
  G: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  "¿": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  "¡": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  a: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  d: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  p: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  q: { frames: 2, lifts: 0, summary: "one unbroken stroke · 2 movements" },
  t: { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  y: { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  H: { frames: 3, lifts: 2, summary: "3 strokes · 2 pen lifts · 3 movements" },
  "á": { frames: 3, lifts: 1, summary: "2 strokes · 1 pen lift · 3 movements" },
  "é": { frames: 3, lifts: 1, summary: "2 strokes · 1 pen lift · 3 movements" },
  "í": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  "ó": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  "ú": { frames: 3, lifts: 1, summary: "2 strokes · 1 pen lift · 3 movements" },
  "ü": { frames: 4, lifts: 2, summary: "3 strokes · 2 pen lifts · 4 movements" },
};

describe("Latin filmstrips", () => {
  for (const [glyph, expected] of Object.entries(EXPECTED)) {
    const letter = DUCTUS[ductusKey("latin", glyph)];

    it(`${glyph}: resolves by script, and only by script`, () => {
      expect(ductusFor(glyph, "latin")).toBe(letter);
      expect(ductusFor(glyph)).toBeUndefined();
    });

    it(`${glyph}: ${expected.summary}`, () => {
      const strip = ductusFilmstrip(letter, latinOutline(glyph));
      expect(strip.frames).toHaveLength(expected.frames);
      expect(strip.penLifts).toBe(expected.lifts);
      expect(strip.summary).toBe(expected.summary);
      const steps = ductusSteps(letter);
      expect(steps.filter((step) => step.startsAfterLift)).toHaveLength(expected.lifts);
      // The last frame: earlier strokes are settled ("done") and the last
      // stroke is the live pen, drawn whole.
      const paths = byTag(strip.frames.at(-1)!, "path");
      const pen = paths.filter((node) => node.attrs.class === "ductus__pen");
      expect(pen.map((node) => node.attrs.d)).toEqual([
        penPathD(letter.strokes.at(-1)!, 1),
      ]);
      expect(paths.filter((node) => node.attrs.class === "ductus__done")).toHaveLength(
        letter.strokes.length - 1,
      );
    });
  }

  it("draws ñ from the precomposed glyph, never from n plus a combining tilde", () => {
    expect(ductusFor("ñ", "latin")).toBeDefined();
    expect(ductusFor("ñ", "latin")).toBeUndefined();
  });

  it("draws the accented vowels and ü from their precomposed glyphs only", () => {
    for (const glyph of ["á", "é", "í", "ó", "ú", "ü"]) {
      expect(ductusFor(glyph, "latin"), glyph).toBeDefined();
      expect(ductusFor(glyph.normalize("NFD"), "latin"), glyph).toBeUndefined();
    }
  });
});
