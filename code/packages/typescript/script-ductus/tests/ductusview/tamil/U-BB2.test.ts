import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const LA = DUCTUS["ல"];
const laOutline = tamilOutline("ல");

it("owns U-BB2 view evidence for ல", () => {
  expect(ductusFor("ல")).toBe(LA);
});

describe("ல — a real cited unbroken four-movement filmstrip", () => {
  const steps = ductusSteps(LA);
  const strip = ductusFilmstrip(LA, laOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(LA.strokes[0], 1));
  });
});
