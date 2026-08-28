import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TTA = DUCTUS["ட"];
const ttaOutline = tamilOutline("ட");

it("owns U-B9F view evidence for ட", () => {
  expect(ductusFor("ட")).toBe(TTA);
});

describe("ட — a real cited unbroken two-movement filmstrip", () => {
  const steps = ductusSteps(TTA);
  const strip = ductusFilmstrip(TTA, ttaOutline);

  it("keeps the cornering movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "down the left upright",
      "along the long rightward foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the sole stroke without a completed-stroke overlay", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TTA.strokes[0], 1));
  });
});
