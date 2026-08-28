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

const KANNADA_A = DUCTUS[ductusKey("kannada", "ಅ")];
const kannadaAOutline = kannadaOutline("ಅ");
const KANNADA_AA = DUCTUS[ductusKey("kannada", "ಆ")];
const kannadaAaOutline = kannadaOutline("ಆ");
const KANNADA_I = DUCTUS[ductusKey("kannada", "ಇ")];
const kannadaIOutline = kannadaOutline("ಇ");
const KANNADA_U = DUCTUS[ductusKey("kannada", "ಉ")];
const kannadaUOutline = kannadaOutline("ಉ");
const KANNADA_E = DUCTUS[ductusKey("kannada", "ಎ")];
const kannadaEOutline = kannadaOutline("ಎ");
const KANNADA_EE = DUCTUS[ductusKey("kannada", "ಏ")];
const kannadaEeOutline = kannadaOutline("ಏ");
const KANNADA_O = DUCTUS[ductusKey("kannada", "ಒ")];
const kannadaOOutline = kannadaOutline("ಒ");
const KANNADA_AI = DUCTUS[ductusKey("kannada", "ಐ")];
const kannadaAiOutline = kannadaOutline("ಐ");
const KANNADA_VOCALIC_R = DUCTUS[ductusKey("kannada", "ಋ")];
const kannadaVocalicROutline = kannadaOutline("ಋ");

describe("Kannada ಅ — four movements in one unbroken run", () => {
  const steps = ductusSteps(KANNADA_A);
  const strip = ductusFilmstrip(KANNADA_A, kannadaAOutline);

  it("never inserts a pen lift between the four movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports one stroke, zero lifts, and four movements", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಆ — two joined pairs separated by one lift", () => {
  const steps = ductusSteps(KANNADA_AA);
  const strip = ductusFilmstrip(KANNADA_AA, kannadaAaOutline);

  it("starts the rounded right loop only after the lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports four movements across two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಇ — one retraced four-movement run", () => {
  const steps = ductusSteps(KANNADA_I);
  const strip = ductusFilmstrip(KANNADA_I, kannadaIOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports four movements without a lift", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಉ — one loop-to-terminal run", () => {
  const steps = ductusSteps(KANNADA_U);
  const strip = ductusFilmstrip(KANNADA_U, kannadaUOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಎ — one loop-to-arch run", () => {
  const steps = ductusSteps(KANNADA_E);
  const strip = ductusFilmstrip(KANNADA_E, kannadaEOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಏ — joined body, then the small upper loop", () => {
  const steps = ductusSteps(KANNADA_EE);
  const strip = ductusFilmstrip(KANNADA_EE, kannadaEeOutline);

  it("places one lift before the small upper loop", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
    ]);
  });

  it("reports four movements in two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಒ — one upper-loop-to-terminal run", () => {
  const steps = ductusSteps(KANNADA_O);
  const strip = ductusFilmstrip(KANNADA_O, kannadaOOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಐ — one spiral-to-returning-arch run", () => {
  const steps = ductusSteps(KANNADA_AI);
  const strip = ductusFilmstrip(KANNADA_AI, kannadaAiOutline);

  it("keeps all three movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
  });

  it("reports a three-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });
});

describe("Kannada ಋ — three source-attested pen-down runs", () => {
  const steps = ductusSteps(KANNADA_VOCALIC_R);
  const strip = ductusFilmstrip(KANNADA_VOCALIC_R, kannadaVocalicROutline);

  it("starts the high hook and right bowl after separate lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
      false,
    ]);
  });

  it("reports a seven-frame, two-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 7 movements");
  });
});
