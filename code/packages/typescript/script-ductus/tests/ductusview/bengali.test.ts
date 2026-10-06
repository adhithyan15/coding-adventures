import { describe, expect, it } from "vitest";
import { DUCTUS, ductusKey, penPathD } from "../../src/strokes";
import { ductusFilmstrip, ductusFor, ductusSteps } from "../../src/ductusview";
import { bengaliOutline } from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

// The filmstrip view of the Bengali owner: how many frames each letter gets,
// where the lifts fall, and that the last frame draws the whole cited path.
const EXPECTED: Record<string, { frames: number; lifts: number; summary: string }> = {
  "এ": { frames: 3, lifts: 0, summary: "one unbroken stroke · 3 movements" },
  "ও": { frames: 3, lifts: 0, summary: "one unbroken stroke · 3 movements" },
  "খ": { frames: 4, lifts: 0, summary: "one unbroken stroke · 4 movements" },
  "থ": { frames: 4, lifts: 0, summary: "one unbroken stroke · 4 movements" },
  "ঞ": { frames: 6, lifts: 1, summary: "2 strokes · 1 pen lift · 6 movements" },
  "ব": { frames: 4, lifts: 0, summary: "one unbroken stroke · 4 movements" },
  "র": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ঃ": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
  "ঁ": { frames: 2, lifts: 1, summary: "2 strokes · 1 pen lift · 2 movements" },
};

describe("Bengali filmstrips", () => {
  for (const [glyph, expected] of Object.entries(EXPECTED)) {
    const letter = DUCTUS[ductusKey("bengali", glyph)];

    it(`${glyph}: resolves by script, and only by script`, () => {
      expect(ductusFor(glyph, "bengali")).toBe(letter);
      expect(ductusFor(glyph)).toBeUndefined();
    });

    it(`${glyph}: ${expected.summary}`, () => {
      const strip = ductusFilmstrip(letter, bengaliOutline(glyph));
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
});
