import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_UU = DUCTUS["ஊ"];
const tamilUuOutline = tamilOutline("ஊ");

it("owns U-B8A view evidence for ஊ", () => {
  expect(ductusFor("ஊ")).toBe(TAMIL_UU);
});

describe("Tamil ஊ — உ in one stroke, one lift, then ள in one stroke", () => {
  const steps = ductusSteps(TAMIL_UU);
  const strip = ductusFilmstrip(TAMIL_UU, tamilUuOutline);

  it("places the only lift between உ and ள", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 1, 1, 1, 1, 1, 1,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports nine movements in two strokes", () => {
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 9 movements");
    expect(TAMIL_UU.source.url).toContain("frame-17");
  });

  it("keeps the finished உ visible while ள is written", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(TAMIL_UU.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(TAMIL_UU.strokes[1], 1));
  });
});
