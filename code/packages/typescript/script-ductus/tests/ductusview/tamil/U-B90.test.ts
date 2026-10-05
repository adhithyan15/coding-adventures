import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_AI = DUCTUS["ஐ"];
const outline = tamilOutline("ஐ");

it("owns U-B90 view evidence for ஐ", () => {
  expect(ductusFor("ஐ")).toBe(TAMIL_AI);
});

describe("Tamil ஐ — five animated parts in one unbroken stroke", () => {
  const steps = ductusSteps(TAMIL_AI);
  const strip = ductusFilmstrip(TAMIL_AI, outline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(5).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(5).fill(0));
  });

  it("reports 5 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
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
    expect(pen.attrs.d).toBe(penPathD(TAMIL_AI.strokes[0], 1));
  });
});
