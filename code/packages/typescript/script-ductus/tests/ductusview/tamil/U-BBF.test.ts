import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_I_SIGN = DUCTUS["ி"];
const outline = tamilOutline("ி");

it("owns U-BBF view evidence for ி", () => {
  expect(ductusFor("ி")).toBe(TAMIL_I_SIGN);
  expect(ductusFor("ி", "tamil")).toBe(TAMIL_I_SIGN);
});

describe("Tamil ி (i sign) — a vowel sign drawn alone in three joined movements", () => {
  const steps = ductusSteps(TAMIL_I_SIGN);
  const strip = ductusFilmstrip(TAMIL_I_SIGN, outline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the whole sign as the one pen path in the last frame", () => {
    const pen = byTag(strip.frames.at(-1)!, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(pen.attrs.d).toBe(penPathD(TAMIL_I_SIGN.strokes[0], 1));
  });
});
