import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_O = DUCTUS["ஒ"];
const tamilOOutline = tamilOutline("ஒ");

it("owns U-B92 view evidence for ஒ", () => {
  expect(ductusFor("ஒ")).toBe(TAMIL_O);
});

describe("Tamil ஒ — both loops and the lower bowl in one stroke", () => {
  const steps = ductusSteps(TAMIL_O);
  const strip = ductusFilmstrip(TAMIL_O, tamilOOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(3).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(3).fill(0));
  });

  it("reports 3 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
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
    expect(pen.attrs.d).toBe(penPathD(TAMIL_O.strokes[0], 1));
  });
});
