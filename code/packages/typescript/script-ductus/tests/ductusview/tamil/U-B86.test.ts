import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const AA = DUCTUS["ஆ"];
const aaOutline = tamilOutline("ஆ");

it("owns U-B86 view evidence for ஆ", () => {
  expect(ductusFor("ஆ")).toBe(AA);
});

describe("ஆ — the upright and long-vowel loop stay connected", () => {
  const steps = ductusSteps(AA);
  const strip = ductusFilmstrip(AA, aaOutline);

  it("places one lift before the upright and none before its loop", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("finishes the connected upright-and-loop stroke in the last frame", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(AA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(AA.strokes[1], 1));
  });
});
