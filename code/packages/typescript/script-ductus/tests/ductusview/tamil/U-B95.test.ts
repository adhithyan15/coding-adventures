import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const KA = DUCTUS["க"];
const kaOutline = tamilOutline("க");

it("owns U-B95 view evidence for க", () => {
  expect(ductusFor("க")).toBe(KA);
});

describe("க — a real cited three-stroke filmstrip", () => {
  const steps = ductusSteps(KA);
  const strip = ductusFilmstrip(KA, kaOutline);

  it("places lifts before each lower bowl", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible while drawing the right bowl", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(KA.strokes[0], 1),
      penPathD(KA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(KA.strokes[2], 1));
  });
});
