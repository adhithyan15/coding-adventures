import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const RRA = DUCTUS["ற"];
const rraOutline = tamilOutline("ற");

it("owns U-BB1 view evidence for ற", () => {
  expect(ductusFor("ற")).toBe(RRA);
});

describe("ற — a real cited three-stroke five-movement filmstrip", () => {
  const steps = ductusSteps(RRA);
  const strip = ductusFilmstrip(RRA, rraOutline);

  it("marks exactly the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 2]);
  });

  it("reports five movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps both completed strokes visible while drawing the joined sweep", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(RRA.strokes[0], 1),
      penPathD(RRA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(RRA.strokes[2], 1));
  });
});
