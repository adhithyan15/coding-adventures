import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const RETROFLEX_NNA = DUCTUS["ண"];
const retroflexNnaOutline = tamilOutline("ண");

it("owns U-BA3 view evidence for ண", () => {
  expect(ductusFor("ண")).toBe(RETROFLEX_NNA);
});

describe("ண — a real cited one-stroke 7-movement filmstrip", () => {
  const steps = ductusSteps(RETROFLEX_NNA);
  const strip = ductusFilmstrip(RETROFLEX_NNA, retroflexNnaOutline);

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
    expect(pen.attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[0], 1));
  });
});
