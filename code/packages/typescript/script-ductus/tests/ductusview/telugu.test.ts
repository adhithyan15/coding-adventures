import { beforeAll, describe, expect, it } from "vitest";
import {
  DUCTUS,
  ductusKey,
  penPathD,
  type LetterDuctus,
} from "../../src/strokes";
import {
  ductusFilmstrip,
  ductusFor,
  ductusFrame,
  ductusSteps,
  escapeXml,
  isSafeName,
  segmentEndFractions,
  svgMarkup,
  viewBoxFor,
  wrapCaption,
  type GlyphOutline,
  type SvgNode,
} from "../../src/ductusview";
import {
  chineseOutline,
  cyrillicOutline,
  devanagariOutline,
  gujaratiOutline,
  hebrewOutline,
  japaneseOutline,
  kannadaOutline,
  malayalamOutline,
  naskhOutline,
  tamilOutline,
  teluguOutline,
} from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

const TELUGU_A = DUCTUS[ductusKey("telugu", "అ")];
const teluguAOutline = teluguOutline("అ");
const TELUGU_AA = DUCTUS[ductusKey("telugu", "ఆ")];
const teluguAaOutline = teluguOutline("ఆ");
const TELUGU_I = DUCTUS[ductusKey("telugu", "ఇ")];
const teluguIOutline = teluguOutline("ఇ");
const TELUGU_U = DUCTUS[ductusKey("telugu", "ఉ")];
const teluguUOutline = teluguOutline("ఉ");

describe("Telugu అ — four movements in one unbroken run", () => {
  // HP Labs India: one stroke for 97% of native writers, so the right lobe
  // continues up from the lower bowl instead of restarting at the bar.
  const steps = ductusSteps(TELUGU_A);
  const strip = ductusFilmstrip(TELUGU_A, teluguAOutline);

  it("places no lift between the four movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("inks the whole run by the time the inner bar returns left", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TELUGU_A.strokes[0], 1));
  });
});

describe("Telugu ఆ — three movements in one unbroken run", () => {
  // HP Labs India: one stroke for 99% of native writers (104 of 105), so the
  // pen crosses up from the bowl into the right lobe instead of lifting.
  const steps = ductusSteps(TELUGU_AA);
  const strip = ductusFilmstrip(TELUGU_AA, teluguAaOutline);

  it("places no lift between the three movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("inks the whole run by the time the bar returns left", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TELUGU_AA.strokes[0], 1));
  });
});

describe("Telugu ఇ — four movements in one unbroken run", () => {
  // HP Labs India: one stroke for 98% of native writers (104 of 106): the
  // two upper parts from left to right, then the bowl, then the tail.
  const steps = ductusSteps(TELUGU_I);
  const strip = ductusFilmstrip(TELUGU_I, teluguIOutline);

  it("places no lift between the four movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("inks the whole run by the time the tail comes down", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TELUGU_I.strokes[0], 1));
  });
});

describe("Telugu ఉ — joined body plus two separate printed components", () => {
  const steps = ductusSteps(TELUGU_U);
  const strip = ductusFilmstrip(TELUGU_U, teluguUOutline);

  it("places lifts before the inner bar and upper headstroke", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2]);
  });

  it("reports five movements in three strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps both earlier runs visible while drawing the headstroke", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(TELUGU_U.strokes[0], 1),
      penPathD(TELUGU_U.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(TELUGU_U.strokes[2], 1));
  });
});
