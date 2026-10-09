import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const AA = DUCTUS["ஆ"];
const aaOutline = tamilOutline("ஆ");

it("owns U-B86 view evidence for ஆ", () => {
  expect(ductusFor("ஆ")).toBe(AA);
});

// As அ, then on from the upright into the long-vowel loop, without a lift
// (LipiTk, 28 of 29 prototypes are one stroke).
describe("ஆ — a cited one-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(AA);
  const strip = ductusFilmstrip(AA, aaOutline);

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
    expect(pen.attrs.d).toBe(penPathD(AA.strokes[0], 1));
  });
});
