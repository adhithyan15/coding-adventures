import { describe, expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor, ductusFilmstrip, ductusSteps } from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";

const TAMIL_AI = DUCTUS["ஐ"];
const outline = tamilOutline("ஐ");

it("owns U-B90 view evidence for ஐ", () => {
  expect(ductusFor("ஐ")).toBe(TAMIL_AI);
});

describe("Tamil ஐ — five independently animated runs", () => {
  const steps = ductusSteps(TAMIL_AI);
  const strip = ductusFilmstrip(TAMIL_AI, outline);

  it("places a lift before each run after the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
  });

  it("reports five movements in five strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });
});
