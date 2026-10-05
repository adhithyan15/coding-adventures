import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const NYA = DUCTUS["ஞ"];
const nyaOutline = tamilOutline("ஞ");

it("owns U-B9E view evidence for ஞ", () => {
  expect(ductusFor("ஞ")).toBe(NYA);
});

describe("ஞ — a real cited one-stroke 8-movement filmstrip", () => {
  const steps = ductusSteps(NYA);
  const strip = ductusFilmstrip(NYA, nyaOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(8).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(8).fill(0));
  });

  it("reports 8 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 8 movements");
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
    expect(pen.attrs.d).toBe(penPathD(NYA.strokes[0], 1));
  });
});
