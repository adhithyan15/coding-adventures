import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const TAMIL_E = DUCTUS["எ"];
const tamilEOutline = tamilOutline("எ");

it("owns U-B8E view evidence for எ", () => {
  expect(ductusFor("எ")).toBe(TAMIL_E);
});

describe("Tamil எ — six joined body movements, then the right upright", () => {
  const steps = ductusSteps(TAMIL_E);
  const strip = ductusFilmstrip(TAMIL_E, tamilEOutline);

  it("places the only lift before movement 7", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 0, 1,
    ]);
  });

  it("reports seven movements in two strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the joined body visible while the upright rises", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(TAMIL_E.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(TAMIL_E.strokes[1], 1));
  });
});
