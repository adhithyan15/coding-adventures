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

const GUJARATI_A = ductusFor("અ", "gujarati")!;
const gujaratiAOutline = gujaratiOutline("અ");
const GUJARATI_AA = ductusFor("આ", "gujarati")!;
const gujaratiAaOutline = gujaratiOutline("આ");
const GUJARATI_I = ductusFor("ઇ", "gujarati")!;
const gujaratiIOutline = gujaratiOutline("ઇ");
const GUJARATI_II = ductusFor("ઈ", "gujarati")!;
const gujaratiIiOutline = gujaratiOutline("ઈ");
const GUJARATI_U = ductusFor("ઉ", "gujarati")!;
const gujaratiUOutline = gujaratiOutline("ઉ");
const GUJARATI_UU = ductusFor("ઊ", "gujarati")!;
const gujaratiUuOutline = gujaratiOutline("ઊ");
const GUJARATI_VOCALIC_R = ductusFor("ઋ", "gujarati")!;
const gujaratiVocalicROutline = gujaratiOutline("ઋ");
const GUJARATI_E = ductusFor("એ", "gujarati")!;
const gujaratiEOutline = gujaratiOutline("એ");
const GUJARATI_AI = ductusFor("ઐ", "gujarati")!;
const gujaratiAiOutline = gujaratiOutline("ઐ");
const GUJARATI_O = ductusFor("ઓ", "gujarati")!;
const gujaratiOOutline = gujaratiOutline("ઓ");
const GUJARATI_AU = ductusFor("ઔ", "gujarati")!;
const gujaratiAuOutline = gujaratiOutline("ઔ");
const GUJARATI_KA = ductusFor("ક", "gujarati")!;
const gujaratiKaOutline = gujaratiOutline("ક");
const GUJARATI_KHA = ductusFor("ખ", "gujarati")!;
const gujaratiKhaOutline = gujaratiOutline("ખ");
const GUJARATI_GA = ductusFor("ગ", "gujarati")!;
const gujaratiGaOutline = gujaratiOutline("ગ");
const GUJARATI_GHA = ductusFor("ઘ", "gujarati")!;
const gujaratiGhaOutline = gujaratiOutline("ઘ");
const GUJARATI_NGA = ductusFor("ઙ", "gujarati")!;
const gujaratiNgaOutline = gujaratiOutline("ઙ");
const GUJARATI_CA = ductusFor("ચ", "gujarati")!;
const gujaratiCaOutline = gujaratiOutline("ચ");
const GUJARATI_CHA = ductusFor("છ", "gujarati")!;
const gujaratiChaOutline = gujaratiOutline("છ");
const GUJARATI_JA = ductusFor("જ", "gujarati")!;
const gujaratiJaOutline = gujaratiOutline("જ");
const GUJARATI_JHA = ductusFor("ઝ", "gujarati")!;
const gujaratiJhaOutline = gujaratiOutline("ઝ");
const GUJARATI_NYA = ductusFor("ઞ", "gujarati")!;
const gujaratiNyaOutline = gujaratiOutline("ઞ");
const GUJARATI_TTA = ductusFor("ટ", "gujarati")!;
const gujaratiTtaOutline = gujaratiOutline("ટ");
const GUJARATI_TTHA = ductusFor("ઠ", "gujarati")!;
const gujaratiTthaOutline = gujaratiOutline("ઠ");
const GUJARATI_DDA = ductusFor("ડ", "gujarati")!;
const gujaratiDdaOutline = gujaratiOutline("ડ");
const GUJARATI_DDHA = ductusFor("ઢ", "gujarati")!;
const gujaratiDdhaOutline = gujaratiOutline("ઢ");
const GUJARATI_NNA = ductusFor("ણ", "gujarati")!;
const gujaratiNnaOutline = gujaratiOutline("ણ");
const GUJARATI_TA = ductusFor("ત", "gujarati")!;
const gujaratiTaOutline = gujaratiOutline("ત");
const GUJARATI_THA = ductusFor("થ", "gujarati")!;
const gujaratiThaOutline = gujaratiOutline("થ");
const GUJARATI_DA = ductusFor("દ", "gujarati")!;
const gujaratiDaOutline = gujaratiOutline("દ");
const GUJARATI_DHA = ductusFor("ધ", "gujarati")!;
const gujaratiDhaOutline = gujaratiOutline("ધ");
const GUJARATI_NA = ductusFor("ન", "gujarati")!;
const gujaratiNaOutline = gujaratiOutline("ન");
const GUJARATI_PA = ductusFor("પ", "gujarati")!;
const gujaratiPaOutline = gujaratiOutline("પ");
const GUJARATI_PHA = ductusFor("ફ", "gujarati")!;
const gujaratiPhaOutline = gujaratiOutline("ફ");
const GUJARATI_BA = ductusFor("બ", "gujarati")!;
const gujaratiBaOutline = gujaratiOutline("બ");
const GUJARATI_BHA = ductusFor("ભ", "gujarati")!;
const gujaratiBhaOutline = gujaratiOutline("ભ");
const GUJARATI_MA = ductusFor("મ", "gujarati")!;
const gujaratiMaOutline = gujaratiOutline("મ");
const GUJARATI_YA = ductusFor("ય", "gujarati")!;
const gujaratiYaOutline = gujaratiOutline("ય");
const GUJARATI_RA = ductusFor("ર", "gujarati")!;
const gujaratiRaOutline = gujaratiOutline("ર");
const GUJARATI_LA = ductusFor("લ", "gujarati")!;
const gujaratiLaOutline = gujaratiOutline("લ");
const GUJARATI_LLA = ductusFor("ળ", "gujarati")!;
const gujaratiLlaOutline = gujaratiOutline("ળ");
const GUJARATI_VA = ductusFor("વ", "gujarati")!;
const gujaratiVaOutline = gujaratiOutline("વ");
const GUJARATI_SHA = ductusFor("શ", "gujarati")!;
const gujaratiShaOutline = gujaratiOutline("શ");
const GUJARATI_SA = ductusFor("સ", "gujarati")!;
const gujaratiSaOutline = gujaratiOutline("સ");
const GUJARATI_HA = ductusFor("હ", "gujarati")!;
const gujaratiHaOutline = gujaratiOutline("હ");

describe("Gujarati અ — joined body before the lifted right stem", () => {
  const steps = ductusSteps(GUJARATI_A);
  const strip = ductusFilmstrip(GUJARATI_A, gujaratiAOutline);

  it("shows the three-part body before the lifted stem and foot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep clockwise around the open left curve",
      "continue through the lower body and rise into the middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the right stem into its foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiAOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_A.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_A.strokes[1], 1));
  });
});

describe("Gujarati આ — complete અ before the lifted trailing ā stem", () => {
  const steps = ductusSteps(GUJARATI_AA);
  const strip = ductusFilmstrip(GUJARATI_AA, gujaratiAaOutline);

  it("shows the joined body before two separately descended stems", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep clockwise around the open left curve",
      "continue through the lower body and rise into the middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing ā stem into its foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiAaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(GUJARATI_AA.strokes[0], 1),
      penPathD(GUJARATI_AA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_AA.strokes[2], 1));
  });
});

describe("Gujarati ઇ — two loops flow into the rising hook without a lift", () => {
  const steps = ductusSteps(GUJARATI_I);
  const strip = ductusFilmstrip(GUJARATI_I, gujaratiIOutline);

  it("shows the upper loop, crossing, lower loop, and hook as one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper-left loop down to the middle crossing",
      "continue through the narrow crossing",
      "sweep clockwise around the broad lower loop",
      "rise along the right side into the upper hook",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiIOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_I.strokes[0], 1));
  });
});

describe("Gujarati ઈ — the ઇ run rises into a taller clockwise curl", () => {
  const steps = ductusSteps(GUJARATI_II);
  const strip = ductusFilmstrip(GUJARATI_II, gujaratiIiOutline);

  it("shows both loops before the extended top curl in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper-left loop down to the middle crossing",
      "continue through the narrow crossing",
      "sweep clockwise around the broad lower loop",
      "rise and curl clockwise around the extended top hook",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiIiOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_II.strokes[0], 1));
  });
});

describe("Gujarati ઉ — two bowls return around one tall outer curve", () => {
  const steps = ductusSteps(GUJARATI_U);
  const strip = ductusFilmstrip(GUJARATI_U, gujaratiUOutline);

  it("shows the upper bowl, lower bowl, and returning curve in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise through the small upper bowl to the middle cusp",
      "continue right and sweep clockwise around the broad lower bowl",
      "climb around the tall outer-left curve and finish at the upper right",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiUOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_U.strokes[0], 1));
  });
});

describe("Gujarati ઊ — the complete ઉ run descends a long right tail", () => {
  const steps = ductusSteps(GUJARATI_UU);
  const strip = ductusFilmstrip(GUJARATI_UU, gujaratiUuOutline);

  it("shows the complete ઉ body before its extended tail in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write ઉ through its upper bowl, middle cusp, and lower bowl",
      "continue around the tall outer-left curve",
      "cross the high shoulder and descend the long right tail into its foot",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiUuOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_UU.strokes[0], 1));
  });
});

describe("Gujarati ઋ — bent body, central stem, then right loop and tail", () => {
  const steps = ductusSteps(GUJARATI_VOCALIC_R);
  const strip = ductusFilmstrip(GUJARATI_VOCALIC_R, gujaratiVocalicROutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right along the upper body, then turn diagonally down-left",
      "lift, then descend the central stem into its foot",
      "lift again, circle the right loop, and descend through the tail",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiVocalicROutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(2);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_VOCALIC_R.strokes[2], 1));
  });
});

describe("Gujarati એ — joined body, right stem, then high arc", () => {
  const steps = ductusSteps(GUJARATI_E);
  const strip = ductusFilmstrip(GUJARATI_E, gujaratiEOutline);

  it("shows four movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise around the left bowl",
      "continue through the lower body and small right arch",
      "lift, then descend the full-height right stem into its foot",
      "lift again and sweep the high arcing mark from left to right",
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiEOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(2);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_E.strokes[2], 1));
  });
});

describe("Gujarati ઐ — the એ sequence gains a second high arc", () => {
  const steps = ductusSteps(GUJARATI_AI);
  const strip = ductusFilmstrip(GUJARATI_AI, gujaratiAiOutline);

  it("shows the body, stem, lower arc, then higher arc as four runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write એ through its joined bowl, lower body, and right arch",
      "lift, then descend the full-height right stem into its foot",
      "lift again and sweep the lower high arc from left to right",
      "lift once more and sweep the higher arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all four runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiAiOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(3);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_AI.strokes[3], 1));
  });
});

describe("Gujarati ઓ — the complete આ sequence gains a high arc", () => {
  const steps = ductusSteps(GUJARATI_O);
  const strip = ductusFilmstrip(GUJARATI_O, gujaratiOOutline);

  it("shows the body, two stems, then high arc as four runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write આ through its open left curve",
      "continue through the lower body and middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing stem into its foot",
      "lift once more and sweep the high arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all four runs", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiOOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(3);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_O.strokes[3], 1));
  });
});

describe("Gujarati ઔ — the ઓ sequence gains a second high arc", () => {
  const steps = ductusSteps(GUJARATI_AU);
  const strip = ductusFilmstrip(GUJARATI_AU, gujaratiAuOutline);

  it("shows the body, two stems, lower arc, then higher arc as five runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write ઓ through its open left curve, lower body, and arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing stem into its foot",
      "lift once more and sweep the lower high arc left to right",
      "lift again and sweep the higher arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all five runs", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiAuOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(4);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_AU.strokes[4], 1));
  });
});

describe("Gujarati ક — joined loop-body before the crossing diagonal", () => {
  const steps = ductusSteps(GUJARATI_KA);
  const strip = ductusFilmstrip(GUJARATI_KA, gujaratiKaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper loop and continue through the rounded lower body",
      "lift, then sweep the diagonal cross-stroke lower-left to upper-right",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiKaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_KA.strokes[1], 1));
  });
});

describe("Gujarati ખ — joined left body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_KHA);
  const strip = ductusFilmstrip(GUJARATI_KHA, gujaratiKhaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend through the left lobe and curl right through the middle",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiKhaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_KHA.strokes[1], 1));
  });
});

describe("Gujarati ગ — rounded body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_GA);
  const strip = ductusFilmstrip(GUJARATI_GA, gujaratiGaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded body from upper left to lower left",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiGaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_GA.strokes[1], 1));
  });
});

describe("Gujarati ઘ — joined double body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_GHA);
  const strip = ductusFilmstrip(GUJARATI_GHA, gujaratiGhaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper lobe, turn through the middle, and round the lower body",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiGhaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_GHA.strokes[1], 1));
  });
});

describe("Gujarati ઙ — S-like body before the separate upper-right dot", () => {
  const steps = ductusSteps(GUJARATI_NGA);
  const strip = ductusFilmstrip(GUJARATI_NGA, gujaratiNgaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep from the upper right through the S-like body to the lower left",
      "lift, then circle the separate upper-right dot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiNgaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_NGA.strokes[1], 1));
  });
});

describe("Gujarati ચ — joined bowls before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_CA);
  const strip = ductusFilmstrip(GUJARATI_CA, gujaratiCaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper bowl, turn through the middle loop, and round the lower body",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiCaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_CA.strokes[1], 1));
  });
});

describe("Gujarati છ — both upper lobes join through one continuous body", () => {
  const steps = ductusSteps(GUJARATI_CHA);
  const strip = ductusFilmstrip(GUJARATI_CHA, gujaratiChaOutline);

  it("shows three connected movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper-left lobe and turn back through the middle",
      "continue around the broad lower body and climb the outer right curve",
      "circle the upper-right lobe and finish beside the outer curve",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiChaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_CHA.strokes[0], 1));
  });
});

describe("Gujarati જ — both loops join through the crossing and exit", () => {
  const steps = ductusSteps(GUJARATI_JA);
  const strip = ductusFilmstrip(GUJARATI_JA, gujaratiJaOutline);

  it("shows three connected movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper-left loop",
      "continue diagonally through the crossing body",
      "circle the lower-right loop and sweep into the upper-right exit",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiJaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_JA.strokes[0], 1));
  });
});

describe("Gujarati ઝ — left body before right loop and upper stem", () => {
  const steps = ductusSteps(GUJARATI_JHA);
  const strip = ductusFilmstrip(GUJARATI_JHA, gujaratiJhaOutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded left body from upper left to lower left",
      "lift, then circle the right loop and finish through its lower tail",
      "lift again, then descend the short upper stem",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiJhaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(GUJARATI_JHA.strokes[0], 1),
      penPathD(GUJARATI_JHA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_JHA.strokes[2], 1));
  });
});

describe("Gujarati ઞ — left body before shoulder and tall spine", () => {
  const steps = ductusSteps(GUJARATI_NYA);
  const strip = ductusFilmstrip(GUJARATI_NYA, gujaratiNyaOutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded left body from upper left to lower left",
      "lift, then sweep the short rightward shoulder",
      "lift again, then descend the tall spine and curl through its terminal",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiNyaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(GUJARATI_NYA.strokes[0], 1),
      penPathD(GUJARATI_NYA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_NYA.strokes[2], 1));
  });
});

describe("Gujarati ટ — upper turn and lower bowl stay joined", () => {
  const steps = ductusSteps(GUJARATI_TTA);
  const strip = ductusFilmstrip(GUJARATI_TTA, gujaratiTtaOutline);

  it("shows the complete joined form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the upper turn, bend down-left, and circle the lower bowl",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiTtaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_TTA.strokes[0], 1));
  });
});

describe("Gujarati ઠ — high shoulder, outer bowl, and inward curl stay joined", () => {
  const steps = ductusSteps(GUJARATI_TTHA);
  const strip = ductusFilmstrip(GUJARATI_TTHA, gujaratiTthaOutline);

  it("shows the complete joined form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder, circle the lower bowl, and curl inward",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiTthaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_TTHA.strokes[0], 1));
  });
});

describe("Gujarati ડ — high shoulder and lower bowl stay joined", () => {
  const steps = ductusSteps(GUJARATI_DDA);
  const strip = ductusFilmstrip(GUJARATI_DDA, gujaratiDdaOutline);

  it("shows the complete descending form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder, descend through the middle, and round the lower bowl",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiDdaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_DDA.strokes[0], 1));
  });
});

describe("Gujarati ઢ — outer bowl flows into the inner loop", () => {
  const steps = ductusSteps(GUJARATI_DDHA);
  const strip = ductusFilmstrip(GUJARATI_DDHA, gujaratiDdhaOutline);

  it("shows the complete looped form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the upper shoulder, round the outer bowl, and circle the inner loop",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiDdhaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_DDHA.strokes[0], 1));
  });
});

describe("Gujarati ણ — hooked body before bowl and right spine", () => {
  const steps = ductusSteps(GUJARATI_NNA);
  const strip = ductusFilmstrip(GUJARATI_NNA, gujaratiNnaOutline);

  it("shows the three source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left spine and sweep through the hooked lower tail",
      "lift, then circle the separate middle bowl",
      "lift again, descend the tall right spine, and turn through its foot",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiNnaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(GUJARATI_NNA.strokes[0], 1),
      penPathD(GUJARATI_NNA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_NNA.strokes[2], 1));
  });
});

describe("Gujarati ત — open body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_TA);
  const strip = ductusFilmstrip(GUJARATI_TA, gujaratiTaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep from the lower terminal around the open body and across the upper shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiTaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_TA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_TA.strokes[1], 1));
  });
});

describe("Gujarati થ — looped body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_THA);
  const strip = ductusFilmstrip(GUJARATI_THA, gujaratiThaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper loop, descend, and sweep around the broad body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiThaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_THA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_THA.strokes[1], 1));
  });
});

describe("Gujarati દ — one continuous upper and lower body", () => {
  const steps = ductusSteps(GUJARATI_DA);
  const strip = ductusFilmstrip(GUJARATI_DA, gujaratiDaOutline);

  it("shows the single source run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper body, narrow through the middle, and sweep around the lower body into its terminal",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
  });

  it("draws the exact Noto Sans Gujarati character behind the run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiDaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_DA.strokes[0], 1));
  });
});

describe("Gujarati ધ — joined body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_DHA);
  const strip = ductusFilmstrip(GUJARATI_DHA, gujaratiDhaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the high entry through the turns and sweep around the broad body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiDhaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_DHA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_DHA.strokes[1], 1));
  });
});

describe("Gujarati ન — loop and shoulder before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_NA);
  const strip = ductusFilmstrip(GUJARATI_NA, gujaratiNaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small left loop and continue across the long rightward shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiNaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_NA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_NA.strokes[1], 1));
  });
});

describe("Gujarati પ — hooked lower body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_PA);
  const strip = ductusFilmstrip(GUJARATI_PA, gujaratiPaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curl over the high left hook, descend, and sweep around the broad lower body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiPaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_PA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_PA.strokes[1], 1));
  });
});

describe("Gujarati ફ — winding body before the diagonal cross-stroke", () => {
  const steps = ductusSteps(GUJARATI_PHA);
  const strip = ductusFilmstrip(GUJARATI_PHA, gujaratiPhaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the high cap, wind around the body and lower-left loop, then exit through the tail",
      "lift and draw the diagonal cross-stroke from lower left to upper right",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiPhaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_PHA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_PHA.strokes[1], 1));
  });
});

describe("Gujarati બ — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_BA);
  const strip = ductusFilmstrip(GUJARATI_BA, gujaratiBaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded body, wind through the inner turn, and exit across the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiBaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(GUJARATI_BA.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_BA.strokes[1], 1));
  });
});

describe("Gujarati ભ — broad loop before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_BHA);
  const strip = ductusFilmstrip(GUJARATI_BHA, gujaratiBhaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiBhaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_BHA.strokes[1], 1));
  });
});

describe("Gujarati મ — left body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_MA);
  const strip = ductusFilmstrip(GUJARATI_MA, gujaratiMaOutline);
  it("shows two source runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiMaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_MA.strokes[1], 1));
  });
});

describe("Gujarati ય — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_YA);
  const strip = ductusFilmstrip(GUJARATI_YA, gujaratiYaOutline);
  it("shows two source runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiYaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_YA.strokes[1], 1));
  });
});

describe("Gujarati ર — upper body, middle loop, and tail stay joined", () => {
  const steps = ductusSteps(GUJARATI_RA);
  const strip = ductusFilmstrip(GUJARATI_RA, gujaratiRaOutline);
  it("shows one continuous source run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiRaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_RA.strokes[0], 1));
  });
});

describe("Gujarati લ — broad body before shoulder and tall spine", () => {
  const steps = ductusSteps(GUJARATI_LA);
  const strip = ductusFilmstrip(GUJARATI_LA, gujaratiLaOutline);
  it("shows the three source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(3);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiLaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_LA.strokes[2], 1));
  });
});

describe("Gujarati ળ — left bowl flows through the arch into the tall spine", () => {
  const steps = ductusSteps(GUJARATI_LLA);
  const strip = ductusFilmstrip(GUJARATI_LLA, gujaratiLlaOutline);
  it("shows one continuous source run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiLlaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_LLA.strokes[0], 1));
  });
});

describe("Gujarati વ — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_VA);
  const strip = ductusFilmstrip(GUJARATI_VA, gujaratiVaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiVaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_VA.strokes[1], 1));
  });
});

describe("Gujarati શ — upper loop and lower body before the tall spine", () => {
  const steps = ductusSteps(GUJARATI_SHA);
  const strip = ductusFilmstrip(GUJARATI_SHA, gujaratiShaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiShaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_SHA.strokes[1], 1));
  });
});

describe("Gujarati સ — rounded loop and shoulder before the tall spine", () => {
  const steps = ductusSteps(GUJARATI_SA);
  const strip = ductusFilmstrip(GUJARATI_SA, gujaratiSaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiSaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_SA.strokes[1], 1));
  });
});

describe("Gujarati હ — upper loop flowing into the broad lower bowl", () => {
  const steps = ductusSteps(GUJARATI_HA);
  const strip = ductusFilmstrip(GUJARATI_HA, gujaratiHaOutline);
  it("shows the single source run without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(gujaratiHaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(GUJARATI_HA.strokes[0], 1));
  });
});
