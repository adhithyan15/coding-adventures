import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const TAMIL_LONG_E = DUCTUS["ஏ"];
const tamilLongEOutline = tamilOutline("ஏ");

it("owns U-B8F view evidence for ஏ", () => {
  expect(ductusFor("ஏ")).toBe(TAMIL_LONG_E);
});

describe("Tamil ஏ — six joined movements through the diagonal foot", () => {
  const steps = ductusSteps(TAMIL_LONG_E);
  const strip = ductusFilmstrip(TAMIL_LONG_E, tamilLongEOutline);

  it("keeps every movement in the first stroke", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports six movements in one stroke", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 6 movements");
  });
});
