import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const NNA = DUCTUS["ன"];
const nnaOutline = tamilOutline("ன");

it("owns U-BA9 view evidence for ன", () => {
  expect(ductusFor("ன")).toBe(NNA);
});

describe("ன — a real cited two-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(NNA);
  const strip = ductusFilmstrip(NNA, nnaOutline);

  it("joins the loop, inner arch, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed loop-and-bar stroke visible while drawing the upright", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(NNA.strokes[1], 1));
  });
});
