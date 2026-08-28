import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const NGA = DUCTUS["ங"];
const ngaOutline = tamilOutline("ங");

it("owns U-B99 view evidence for ங", () => {
  expect(ductusFor("ங")).toBe(NGA);
});

describe("ங — a detached upright followed by one joined body", () => {
  const steps = ductusSteps(NGA);
  const strip = ductusFilmstrip(NGA, ngaOutline);

  it("keeps the five body movements joined after one lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports six movements in two strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed upright visible while the body finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(NGA.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(NGA.strokes[1], 1));
  });
});
