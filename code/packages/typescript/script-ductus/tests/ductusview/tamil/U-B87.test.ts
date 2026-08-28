import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const I = DUCTUS["இ"];
const iOutline = tamilOutline("இ");

it("owns U-B87 view evidence for இ", () => {
  expect(ductusFor("இ")).toBe(I);
});

describe("இ — a real cited seven-movement filmstrip", () => {
  const steps = ductusSteps(I);
  const strip = ductusFilmstrip(I, iOutline);

  it("places one lift before the outer climb and joins that climb to the arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 1, 1,
    ]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("finishes the joined outer climb-and-arch stroke in the last frame", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(I.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(I.strokes[1], 1));
  });
});
