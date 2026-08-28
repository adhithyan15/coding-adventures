import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const NYA = DUCTUS["ஞ"];
const nyaOutline = tamilOutline("ஞ");

it("owns U-B9E view evidence for ஞ", () => {
  expect(ductusFor("ஞ")).toBe(NYA);
});

describe("ஞ — four source-verified runs across eight movements", () => {
  const steps = ductusSteps(NYA);
  const strip = ductusFilmstrip(NYA, nyaOutline);

  it("places lifts before the top bar, central descent, and outer bowl", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 2, 2, 3, 3, 3,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      false,
      true,
      false,
      false,
    ]);
  });

  it("reports eight movements in four strokes", () => {
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 8 movements");
    expect(NYA.source.citation).toMatch(/Frame 8.*ஞ.*p\. 194/i);
  });
});
