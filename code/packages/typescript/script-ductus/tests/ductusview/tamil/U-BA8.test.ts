import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const DENTAL_NA = DUCTUS["ந"];
const dentalNaOutline = tamilOutline("ந");

it("owns U-BA8 view evidence for ந", () => {
  expect(ductusFor("ந")).toBe(DENTAL_NA);
});

describe("ந — a real cited one-stroke 6-movement filmstrip", () => {
  const steps = ductusSteps(DENTAL_NA);
  const strip = ductusFilmstrip(DENTAL_NA, dentalNaOutline);

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
    expect(pen.attrs.d).toBe(penPathD(DENTAL_NA.strokes[0], 1));
  });
});
