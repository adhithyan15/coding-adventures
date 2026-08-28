import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_U = DUCTUS["உ"];
const tamilUOutline = tamilOutline("உ");

it("owns U-B89 view evidence for உ", () => {
  expect(ductusFor("உ")).toBe(TAMIL_U);
});

describe("Tamil உ — one joined spiral, outer descent, and baseline", () => {
  const steps = ductusSteps(TAMIL_U);
  const strip = ductusFilmstrip(TAMIL_U, tamilUOutline);

  it("keeps every Frame 16 movement in stroke zero", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("finishes by carrying the baseline to the right", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TAMIL_U.strokes[0], 1));
  });
});
