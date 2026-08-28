import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const THA = DUCTUS["த"];
const thaOutline = tamilOutline("த");

it("owns U-BA4 view evidence for த", () => {
  expect(ductusFor("த")).toBe(THA);
});

describe("த — a real cited four-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(THA);
  const strip = ductusFilmstrip(THA, thaOutline);

  it("shows the three source-marked lifts between four runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 1, 2, 2, 3,
    ]);
  });

  it("reports seven movements across four strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });

  it("finishes with three completed runs behind the leftward tail", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(THA.strokes[0], 1),
      penPathD(THA.strokes[1], 1),
      penPathD(THA.strokes[2], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(THA.strokes[3], 1));
  });
});
