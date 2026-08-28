import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const TAMIL_O = DUCTUS["ஒ"];
const tamilOOutline = tamilOutline("ஒ");

it("owns U-B92 view evidence for ஒ", () => {
  expect(ductusFor("ஒ")).toBe(TAMIL_O);
});

describe("Tamil ஒ — joined upper loops followed by the lower bowl", () => {
  const steps = ductusSteps(TAMIL_O);
  const strip = ductusFilmstrip(TAMIL_O, tamilOOutline);

  it("places one lift before the lower bowl", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
  });

  it("reports three movements in two strokes", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });
});
