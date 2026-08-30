import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor, ductusFilmstrip, ductusSteps } from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const TAMIL_SHA = DUCTUS["ஷ"];
const outline = tamilOutline("ஷ");

it("owns U-BB7 view evidence for ஷ", () => {
  expect(ductusFor("ஷ")).toBe(TAMIL_SHA);
});

describe("Tamil ஷ — four independently animated runs", () => {
  const steps = ductusSteps(TAMIL_SHA);
  const strip = ductusFilmstrip(TAMIL_SHA, outline);

  it("places a lift before each run after the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
  });

  it("reports four movements in four strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });
});
