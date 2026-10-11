import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import { ductusFor, ductusFilmstrip, ductusSteps } from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_SA = DUCTUS["ஸ"];
const outline = tamilOutline("ஸ");

it("owns U-BB8 view evidence for ஸ", () => {
  expect(ductusFor("ஸ")).toBe(TAMIL_SA);
});

describe("Tamil ஸ — one unbroken stroke in seven movements", () => {
  const steps = ductusSteps(TAMIL_SA);
  const strip = ductusFilmstrip(TAMIL_SA, outline);

  it("never lifts the pen", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 0]);
    expect(steps.some((step) => step.startsAfterLift)).toBe(false);
  });

  it("reports seven movements in one stroke, the whole path live in the last frame", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 7 movements");
    const paths = byTag(strip.frames.at(-1)!, "path");
    expect(paths.find((node) => node.attrs.class === "ductus__glyph")!.attrs.d).toBe(outline.path);
    expect(paths.find((node) => node.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(TAMIL_SA.strokes[0], 1),
    );
  });
});
