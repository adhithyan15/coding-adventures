import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const DENTAL_NA = DUCTUS["ந"];
const dentalNaOutline = tamilOutline("ந");

it("owns U-BA8 view evidence for ந", () => {
  expect(ductusFor("ந")).toBe(DENTAL_NA);
});

describe("ந — a real cited three-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(DENTAL_NA);
  const strip = ductusFilmstrip(DENTAL_NA, dentalNaOutline);

  it("marks the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 2, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible during the right-bowl tail", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(DENTAL_NA.strokes[0], 1),
      penPathD(DENTAL_NA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(DENTAL_NA.strokes[2], 1));
  });
});
