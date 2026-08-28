import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const CA = DUCTUS["ச"];
const caOutline = tamilOutline("ச");

it("owns U-B9A view evidence for ச", () => {
  expect(ductusFor("ச")).toBe(CA);
});

describe("ச — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(CA);
  const strip = ductusFilmstrip(CA, caOutline);

  it("has one frame per named movement", () => {
    expect(steps).toHaveLength(4);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("advances through the joined upper frame before the lifted bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the left upright",
      "carry the top bar to the right",
      "drop the inner upright and carry right",
      "turn around and close the lower-left bowl",
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
  });

  it("renders the completed upper frame behind the active bowl", () => {
    const final = strip.frames.at(-1)!;
    const done = byTag(final, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(final, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(CA.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(CA.strokes[1], 1));
  });
});
