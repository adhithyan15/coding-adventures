import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_PULLI = DUCTUS["்"];
const outline = tamilOutline("்");

it("owns U-BCD view evidence for ்", () => {
  expect(ductusFor("்")).toBe(TAMIL_PULLI);
  expect(ductusFor("்", "tamil")).toBe(TAMIL_PULLI);
});

describe("Tamil ் (puḷḷi) — the dot drawn alone, in one touch", () => {
  const steps = ductusSteps(TAMIL_PULLI);
  const strip = ductusFilmstrip(TAMIL_PULLI, outline);

  it("is one movement with no lift of its own", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
  });

  it("reports one movement in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the whole sign as the one pen path in its only frame", () => {
    const pen = byTag(strip.frames.at(-1)!, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(pen.attrs.d).toBe(penPathD(TAMIL_PULLI.strokes[0], 1));
  });
});
