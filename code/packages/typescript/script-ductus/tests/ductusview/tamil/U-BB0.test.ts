import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const RA = DUCTUS["ர"];
const raOutline = tamilOutline("ர");

it("owns U-BB0 view evidence for ர", () => {
  expect(ductusFor("ர")).toBe(RA);
});

describe("ர — a real cited one-stroke 4-movement filmstrip", () => {
  const steps = ductusSteps(RA);
  const strip = ductusFilmstrip(RA, raOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(4).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(4).fill(0));
  });

  it("reports 4 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
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
    expect(pen.attrs.d).toBe(penPathD(RA.strokes[0], 1));
  });
});
