import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_ZHA = DUCTUS["ழ"];
const tamilZhaOutline = tamilOutline("ழ");

it("owns U-BB4 view evidence for ழ", () => {
  expect(ductusFor("ழ")).toBe(TAMIL_ZHA);
});

describe("Tamil ழ — joined left body, joined right bowl, then lower hook", () => {
  const steps = ductusSteps(TAMIL_ZHA);
  const strip = ductusFilmstrip(TAMIL_ZHA, tamilZhaOutline);

  it("places lifts before movements 4 and 6", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both body runs visible while the detached hook completes", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done[0].attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[0], 1));
    expect(done[1].attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[1], 1));
    expect(pen.attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[2], 1));
  });
});
