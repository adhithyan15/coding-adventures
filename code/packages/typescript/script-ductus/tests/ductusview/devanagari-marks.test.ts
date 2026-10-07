// How the nine Devanagari signs print as filmstrips: one frame per movement,
// a lift exactly where HP Labs India's native writers lift the pen, and each
// sign drawn over the bundled Noto Sans Devanagari outline of the sign by
// itself — no consonant, and no headline the printed sign does not carry (ā
// carries a piece of one, so its strip draws it, last).
import { describe, expect, it } from "vitest";
import { DUCTUS, ductusKey, penPathD } from "../../src/strokes";
import { ductusFilmstrip, ductusFor, ductusSteps } from "../../src/ductusview";
import { devanagariOutline } from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

//   sign   lifts before each movement            summary
const VIEWS: Record<string, { lifts: boolean[]; summary: string }> = {
  "ा": { lifts: [false, true], summary: "2 strokes · 1 pen lift · 2 movements" },
  "ु": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ू": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "े": { lifts: [false, false], summary: "one unbroken stroke · 2 movements" },
  "ं": { lifts: [false, false], summary: "one unbroken stroke · 2 movements" },
  "़": { lifts: [false], summary: "one unbroken stroke · 1 movement" },
  "्": { lifts: [false], summary: "one unbroken stroke · 1 movement" },
  "ृ": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ँ": { lifts: [false, false, true], summary: "2 strokes · 1 pen lift · 3 movements" },
};

for (const [glyph, view] of Object.entries(VIEWS)) {
  describe(`Devanagari ${glyph} as a filmstrip`, () => {
    const sign = DUCTUS[ductusKey("devanagari", glyph)];
    const strip = ductusFilmstrip(sign, devanagariOutline(glyph));

    it("is found by glyph and script", () => {
      expect(ductusFor(glyph, "devanagari")).toBe(sign);
    });

    it("lifts the pen only between the native writers' separate strokes", () => {
      expect(ductusSteps(sign).map((step) => step.startsAfterLift)).toEqual(view.lifts);
      expect(strip.frames).toHaveLength(view.lifts.length);
      expect(strip.penLifts).toBe(view.lifts.filter(Boolean).length);
      expect(strip.summary).toBe(view.summary);
    });

    it("ends with the sign's own outline behind every finished stroke", () => {
      const outline = devanagariOutline(glyph);
      const paths = byTag(strip.frames.at(-1)!, "path");
      expect(paths.find((node) => node.attrs.class === "ductus__glyph")!.attrs.d).toBe(outline.path);
      expect(
        paths.filter((node) => node.attrs.class === "ductus__done").map((node) => node.attrs.d),
      ).toEqual(sign.strokes.slice(0, -1).map((stroke) => penPathD(stroke, 1)));
      expect(paths.find((node) => node.attrs.class === "ductus__pen")!.attrs.d).toBe(
        penPathD(sign.strokes.at(-1)!, 1),
      );
    });
  });
}
