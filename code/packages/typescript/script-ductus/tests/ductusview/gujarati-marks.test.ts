// How the eleven Gujarati signs print as filmstrips: one frame per movement,
// a lift exactly where KanoAI's templates lift the pen, and each sign drawn
// over the bundled Noto Sans Gujarati outline of the sign by itself.
import { describe, expect, it } from "vitest";
import { DUCTUS, ductusKey, penPathD } from "../../src/strokes";
import { ductusFilmstrip, ductusFor, ductusSteps } from "../../src/ductusview";
import { gujaratiOutline } from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

//   sign   lifts before each movement            summary
const VIEWS: Record<string, { lifts: boolean[]; summary: string }> = {
  "ા": { lifts: [false, false], summary: "one unbroken stroke · 2 movements" },
  "િ": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ી": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ુ": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ૂ": { lifts: [false, false, false], summary: "one unbroken stroke · 3 movements" },
  "ે": { lifts: [false, false], summary: "one unbroken stroke · 2 movements" },
  "ૈ": { lifts: [false, false, true, false], summary: "2 strokes · 1 pen lift · 4 movements" },
  "ો": { lifts: [false, false, true, false], summary: "2 strokes · 1 pen lift · 4 movements" },
  "ૌ": {
    lifts: [false, false, true, false, true, false],
    summary: "3 strokes · 2 pen lifts · 6 movements",
  },
  "ં": { lifts: [false, false], summary: "one unbroken stroke · 2 movements" },
  "ઃ": { lifts: [false, false, true, false], summary: "2 strokes · 1 pen lift · 4 movements" },
};

for (const [glyph, view] of Object.entries(VIEWS)) {
  describe(`Gujarati ${glyph} as a filmstrip`, () => {
    const sign = DUCTUS[ductusKey("gujarati", glyph)];
    const strip = ductusFilmstrip(sign, gujaratiOutline(glyph));

    it("is found by glyph and script", () => {
      expect(ductusFor(glyph, "gujarati")).toBe(sign);
    });

    it("lifts the pen only between KanoAI's separate runs", () => {
      expect(ductusSteps(sign).map((step) => step.startsAfterLift)).toEqual(view.lifts);
      expect(strip.frames).toHaveLength(view.lifts.length);
      expect(strip.penLifts).toBe(view.lifts.filter(Boolean).length);
      expect(strip.summary).toBe(view.summary);
    });

    it("ends with the sign's own outline behind every finished run", () => {
      const outline = gujaratiOutline(glyph);
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
