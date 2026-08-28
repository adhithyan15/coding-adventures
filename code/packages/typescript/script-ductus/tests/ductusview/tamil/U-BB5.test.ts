import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD } from "../../../src/strokes";
import {
  ductusFor,
  ductusFilmstrip,
  ductusSteps,
} from "../../../src/ductusview";
import { tamilOutline } from "../../support/font-fixtures";
import { byTag } from "../../support/svg-tree";

const VA = DUCTUS["வ"];
const vaOutline = tamilOutline("வ");

it("owns U-BB5 view evidence for வ", () => {
  expect(ductusFor("வ")).toBe(VA);
});

describe("வ — a real cited unbroken five-movement filmstrip", () => {
  const steps = ductusSteps(VA);
  const strip = ductusFilmstrip(VA, vaOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
  });

  it("reports five movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(VA.strokes[0], 1));
  });
});
