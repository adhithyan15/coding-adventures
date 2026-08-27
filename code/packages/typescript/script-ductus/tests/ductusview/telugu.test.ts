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

describe("Telugu అ — two joined movement pairs", () => {
  const steps = ductusSteps(TELUGU_A);
  const strip = ductusFilmstrip(TELUGU_A, teluguAOutline);

  it("places one lift between movements 2 and 3", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
  });

  it("reports four movements in two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the first run visible while the inner bar returns left", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(TELUGU_A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(TELUGU_A.strokes[1], 1));
  });
});

describe("Telugu ఆ — two source-verified component runs", () => {
  const steps = ductusSteps(TELUGU_AA);
  const strip = ductusFilmstrip(TELUGU_AA, teluguAaOutline);

  it("places one lift between the bowl and right lobe", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
  });

  it("reports two movements in two strokes", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("keeps the completed bowl visible while drawing the right lobe", () => {
    const last = strip.frames[1];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(TELUGU_AA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(TELUGU_AA.strokes[1], 1));
  });
});

describe("Telugu ఇ — three source-verified component runs", () => {
  const steps = ductusSteps(TELUGU_I);
  const strip = ductusFilmstrip(TELUGU_I, teluguIOutline);

  it("places lifts before the two upper components", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
  });

  it("reports three movements in three strokes", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("keeps both earlier components visible while drawing the shoulder", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(TELUGU_I.strokes[0], 1),
      penPathD(TELUGU_I.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(TELUGU_I.strokes[2], 1));
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
