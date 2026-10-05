import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const THA = DUCTUS["த"];
const thaOutline = tamilOutline("த");

it("owns U-BA4 view evidence for த", () => {
  expect(ductusFor("த")).toBe(THA);
});

describe("த — a real cited one-stroke 7-movement filmstrip", () => {
  const steps = ductusSteps(THA);
  const strip = ductusFilmstrip(THA, thaOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(7).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(7).fill(0));
  });

  it("reports 7 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 7 movements");
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
    expect(pen.attrs.d).toBe(penPathD(THA.strokes[0], 1));
  });
});
