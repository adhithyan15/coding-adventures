import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const TAMIL_UU = DUCTUS["ஊ"];
const tamilUuOutline = tamilOutline("ஊ");

it("owns U-B8A view evidence for ஊ", () => {
  expect(ductusFor("ஊ")).toBe(TAMIL_UU);
});

describe("Tamil ஊ — familiar உ followed by the three-run ள overlay", () => {
  const steps = ductusSteps(TAMIL_UU);
  const strip = ductusFilmstrip(TAMIL_UU, tamilUuOutline);

  it("places three lifts only after உ and between ள's familiar runs", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 1, 1, 1, 2, 2, 3,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports nine movements in four strokes", () => {
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 9 movements");
    expect(TAMIL_UU.source.url).toContain("frame-17");
  });
});
