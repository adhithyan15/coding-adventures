import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const A = DUCTUS["அ"];
const aOutline = tamilOutline("அ");

it("owns U-B85 view evidence for அ", () => {
  expect(ductusFor("அ")).toBe(A);
});

describe("அ — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(A);
  const strip = ductusFilmstrip(A, aOutline);

  it("places the only pen lift before the separate right upright", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
  });

  it("reports the source-backed movement, stroke, and lift counts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed body visible while drawing the upright", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(A.strokes[1], 1));
  });
});
