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

// Native writers' order (LipiTk, 81 of 88 prototypes one stroke): the bowl
// from inside, the left side, the top bar, then the upright down.
describe("Tamil எ — a cited one-stroke four-movement filmstrip", () => {
  const steps = ductusSteps(TAMIL_E);
  const strip = ductusFilmstrip(TAMIL_E, tamilEOutline);

  it("never lifts the pen between movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual(
      Array(4).fill(false),
    );
    expect(steps.map((step) => step.strokeIndex)).toEqual(Array(4).fill(0));
  });

  it("reports 4 movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the whole letter as the one pen path in the last frame", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TAMIL_E.strokes[0], 1));
  });
});
