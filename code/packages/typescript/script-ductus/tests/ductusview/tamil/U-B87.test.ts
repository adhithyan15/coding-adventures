import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const I = DUCTUS["இ"];
const iOutline = tamilOutline("இ");

it("owns U-B87 view evidence for இ", () => {
  expect(ductusFor("இ")).toBe(I);
});

describe("இ — a real cited one-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(I);
  const strip = ductusFilmstrip(I, iOutline);

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
    expect(pen.attrs.d).toBe(penPathD(I.strokes[0], 1));
  });
});
