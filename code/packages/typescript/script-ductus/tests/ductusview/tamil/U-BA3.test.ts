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

describe("ண — a real cited two-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(RETROFLEX_NNA);
  const strip = ductusFilmstrip(RETROFLEX_NNA, retroflexNnaOutline);

  it("joins the loop, both inner arches, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 0, 1,
    ]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the completed double-arch stroke visible while drawing the upright", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[1], 1));
  });
});
