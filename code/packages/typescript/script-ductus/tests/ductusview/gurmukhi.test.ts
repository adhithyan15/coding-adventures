import { describe, expect, it } from "vitest";
import { DUCTUS, ductusKey, penPathD } from "../../src/strokes";
import { ductusFilmstrip, ductusFor, ductusSteps } from "../../src/ductusview";
import { gurmukhiOutline } from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

// The filmstrip view of the Gurmukhi owner: how many frames each letter gets,
// where the lifts fall, and that the last frame draws the whole cited path.
const EXPECTED: Record<string, { frames: number; lifts: number; summary: string }> = {
  "ਅ": { frames: 8, lifts: 1, summary: "2 strokes · 1 pen lift · 8 movements" },
  "ਸ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ਹ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਕ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਖ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ਗ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ਘ": { frames: 6, lifts: 1, summary: "2 strokes · 1 pen lift · 6 movements" },
  "ਚ": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ਛ": { frames: 6, lifts: 1, summary: "2 strokes · 1 pen lift · 6 movements" },
  "ਜ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ਟ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਠ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਡ": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ਣ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ਤ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਥ": { frames: 5, lifts: 3, summary: "4 strokes · 3 pen lifts · 5 movements" },
  "ਦ": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ਨ": { frames: 4, lifts: 2, summary: "3 strokes · 2 pen lifts · 4 movements" },
  "ਪ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਫ": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ਬ": { frames: 6, lifts: 2, summary: "3 strokes · 2 pen lifts · 6 movements" },
  "ਭ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਮ": { frames: 5, lifts: 1, summary: "2 strokes · 1 pen lift · 5 movements" },
  "ਰ": { frames: 4, lifts: 1, summary: "2 strokes · 1 pen lift · 4 movements" },
  "ਲ": { frames: 6, lifts: 3, summary: "4 strokes · 3 pen lifts · 6 movements" },
  "ਵ": { frames: 5, lifts: 2, summary: "3 strokes · 2 pen lifts · 5 movements" },
  "ੜ": { frames: 7, lifts: 2, summary: "3 strokes · 2 pen lifts · 7 movements" },
};

describe("Gurmukhi filmstrips", () => {
  for (const [glyph, expected] of Object.entries(EXPECTED)) {
    const letter = DUCTUS[ductusKey("gurmukhi", glyph)];

    it(`${glyph}: resolves by script, and only by script`, () => {
      expect(ductusFor(glyph, "gurmukhi")).toBe(letter);
      expect(ductusFor(glyph)).toBeUndefined();
    });

    it(`${glyph}: ${expected.summary}`, () => {
      const strip = ductusFilmstrip(letter, gurmukhiOutline(glyph));
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
