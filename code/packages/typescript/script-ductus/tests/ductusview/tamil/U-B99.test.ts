import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const NGA = DUCTUS["ங"];
const ngaOutline = tamilOutline("ங");

it("owns U-B99 view evidence for ங", () => {
  expect(ductusFor("ங")).toBe(NGA);
});

// Native writers' order (LipiTk, 92 of 108 prototypes one stroke): the left
// upright down and back up first, the right upright upward last.
describe("ங — a cited one-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(NGA);
  const strip = ductusFilmstrip(NGA, ngaOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(6).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(6).fill(0));
  });

  it("reports 6 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 6 movements");
  });

  it("draws the whole letter as the one pen path in the last frame", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(NGA.strokes[0], 1));
  });
});
