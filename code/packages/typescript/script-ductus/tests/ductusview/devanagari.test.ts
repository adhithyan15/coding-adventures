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

const DEVANAGARI_A = ductusFor("अ", "devanagari")!;
const devanagariAOutline = devanagariOutline("अ");
const DEVANAGARI_AA = ductusFor("आ", "devanagari")!;
const devanagariAaOutline = devanagariOutline("आ");
const DEVANAGARI_I = ductusFor("इ", "devanagari")!;
const devanagariIOutline = devanagariOutline("इ");
const DEVANAGARI_II = ductusFor("ई", "devanagari")!;
const devanagariIiOutline = devanagariOutline("ई");
const DEVANAGARI_U = ductusFor("उ", "devanagari")!;
const devanagariUOutline = devanagariOutline("उ");
const DEVANAGARI_UU = ductusFor("ऊ", "devanagari")!;
const devanagariUuOutline = devanagariOutline("ऊ");
const DEVANAGARI_E = ductusFor("ए", "devanagari")!;
const devanagariEOutline = devanagariOutline("ए");
const DEVANAGARI_AI = ductusFor("ऐ", "devanagari")!;
const devanagariAiOutline = devanagariOutline("ऐ");
const DEVANAGARI_O = ductusFor("ओ", "devanagari")!;
const devanagariOOutline = devanagariOutline("ओ");
const DEVANAGARI_AU = ductusFor("औ", "devanagari")!;
const devanagariAuOutline = devanagariOutline("औ");
const DEVANAGARI_KA = ductusFor("क", "devanagari")!;
const devanagariKaOutline = devanagariOutline("क");
const DEVANAGARI_GA = ductusFor("ग", "devanagari")!;
const devanagariGaOutline = devanagariOutline("ग");
const DEVANAGARI_CA = ductusFor("च", "devanagari")!;
const devanagariCaOutline = devanagariOutline("च");
const DEVANAGARI_TA = ductusFor("त", "devanagari")!;
const devanagariTaOutline = devanagariOutline("त");
const DEVANAGARI_DA = ductusFor("द", "devanagari")!;
const devanagariDaOutline = devanagariOutline("द");
const DEVANAGARI_DHA = ductusFor("ध", "devanagari")!;
const devanagariDhaOutline = devanagariOutline("ध");
const DEVANAGARI_NA = ductusFor("न", "devanagari")!;
const devanagariNaOutline = devanagariOutline("न");
const DEVANAGARI_PA = ductusFor("प", "devanagari")!;
const devanagariPaOutline = devanagariOutline("प");
const DEVANAGARI_BA = ductusFor("ब", "devanagari")!;
const devanagariBaOutline = devanagariOutline("ब");
const DEVANAGARI_BHA = ductusFor("भ", "devanagari")!;
const devanagariBhaOutline = devanagariOutline("भ");
const DEVANAGARI_MA = ductusFor("म", "devanagari")!;
const devanagariMaOutline = devanagariOutline("म");
const DEVANAGARI_YA = ductusFor("य", "devanagari")!;
const devanagariYaOutline = devanagariOutline("य");
const DEVANAGARI_RA = ductusFor("र", "devanagari")!;
const devanagariRaOutline = devanagariOutline("र");
const DEVANAGARI_LA = ductusFor("ल", "devanagari")!;
const devanagariLaOutline = devanagariOutline("ल");
const DEVANAGARI_VA = ductusFor("व", "devanagari")!;
const devanagariVaOutline = devanagariOutline("व");
const DEVANAGARI_SHA = ductusFor("श", "devanagari")!;
const devanagariShaOutline = devanagariOutline("श");
const DEVANAGARI_SA = ductusFor("स", "devanagari")!;
const devanagariSaOutline = devanagariOutline("स");
const DEVANAGARI_HA = ductusFor("ह", "devanagari")!;
const devanagariHaOutline = devanagariOutline("ह");

describe("Devanagari अ — shoulder runs into the right stem before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_A);
  const strip = ductusFilmstrip(DEVANAGARI_A, devanagariAOutline);

  it("shows six movements across three strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue round the lower bowl",
      "lift, then sweep the middle shoulder right",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 1, 2]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_A.strokes[2], 1));
  });
});

describe("Devanagari आ — shoulder runs into the inner stem before trailing stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AA);
  const strip = ductusFilmstrip(DEVANAGARI_AA, devanagariAaOutline);

  it("shows seven movements across four strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue round the lower bowl",
      "lift, then sweep the middle shoulder right",
      "climb up the inner stem without lifting",
      "descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 1, 1, 2, 3,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the full headline", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_AA.strokes[3], 1));
  });
});

describe("Devanagari इ — continuous double-bowl body before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_I);
  const strip = ductusFilmstrip(DEVANAGARI_I, devanagariIOutline);

  it("shows five movements across two sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the upright from the headline",
      "turn left round the upper bowl",
      "sweep through the waist and lower bowl",
      "finish down-right through the tail",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariIOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_I.strokes[1], 1));
  });
});

describe("Devanagari ई — shared double-bowl body before curl and headline", () => {
  const steps = ductusSteps(DEVANAGARI_II);
  const strip = ductusFilmstrip(DEVANAGARI_II, devanagariIiOutline);

  it("shows six movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the upright from the headline",
      "turn left round the upper bowl",
      "sweep through the waist and lower bowl",
      "finish down-right through the tail",
      "lift, then curl up and round to the right",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariIiOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_II.strokes[2], 1));
  });
});

describe("Devanagari उ — joined upper bowl and lower loop before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_U);
  const strip = ductusFilmstrip(DEVANAGARI_U, devanagariUOutline);

  it("shows three movements across two sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve down and left around the upper bowl",
      "sweep back round the lower loop",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariUOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_U.strokes[1], 1));
  });
});

describe("Devanagari ऊ — shared body before the right loop and headline", () => {
  const steps = ductusSteps(DEVANAGARI_UU);
  const strip = ductusFilmstrip(DEVANAGARI_UU, devanagariUuOutline);

  it("shows four movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve down and left around the upper bowl",
      "sweep back round the lower loop",
      "lift, then loop up, round and down-left",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariUuOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_UU.strokes[2], 1));
  });
});

describe("Devanagari ए — long stem and tail before short stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_E);
  const strip = ductusFilmstrip(DEVANAGARI_E, devanagariEOutline);

  it("shows four movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long left stem",
      "curve right and sweep down the tail",
      "lift, then descend the short stem and hook",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariEOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_E.strokes[2], 1));
  });
});

describe("Devanagari ऐ — shared ए base before upper arc and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AI);
  const strip = ductusFilmstrip(DEVANAGARI_AI, devanagariAiOutline);

  it("shows five movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long left stem",
      "curve right and sweep down the tail",
      "lift, then descend the short stem and hook",
      "lift, then sweep the upper arc up and left",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAiOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_AI.strokes[3], 1));
  });
});

describe("Devanagari ओ — shoulder runs into the inner stem before trailing stem, arc, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_O);
  const strip = ductusFilmstrip(DEVANAGARI_O, devanagariOOutline);

  it("shows eight movements across five strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue round the lower bowl",
      "lift, then sweep the middle shoulder right",
      "climb up the inner stem without lifting",
      "descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the upper arc up and left",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 1, 1, 2, 3, 4,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariOOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_O.strokes[4], 1));
  });
});

describe("Devanagari औ — shoulder runs into the inner stem before trailing stem, two arcs, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AU);
  const strip = ductusFilmstrip(DEVANAGARI_AU, devanagariAuOutline);

  it("shows nine movements across six strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue round the lower bowl",
      "lift, then sweep the middle shoulder right",
      "climb up the inner stem without lifting",
      "descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the lower arc up and left",
      "lift, then sweep the taller arc up and left",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 1, 1, 2, 3, 4, 5,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAuOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_AU.strokes[5], 1));
  });
});

describe("Devanagari क — bowl, stem, and arch in one run before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_KA);
  const strip = ductusFilmstrip(DEVANAGARI_KA, devanagariKaOutline);

  it("shows six movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left over the top and around the bowl",
      "climb up the central stem without lifting",
      "descend the central stem",
      "climb back up to the upper junction",
      "sweep the right-hand arch clockwise",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariKaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_KA.strokes[1], 1));
  });
});

describe("Devanagari ग — continuous loop and ascending stem before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_GA);
  const strip = ductusFilmstrip(DEVANAGARI_GA, devanagariGaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "loop anticlockwise and up the stem",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariGaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_GA.strokes[2], 1));
  });
});

describe("Devanagari च — upper bar and rounded body run into the right stem", () => {
  const steps = ductusSteps(DEVANAGARI_CA);
  const strip = ductusFilmstrip(DEVANAGARI_CA, devanagariCaOutline);

  it("shows four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar, curve round the body",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariCaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_CA.strokes[1], 1));
  });
});

describe("Devanagari त — right-to-left shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_TA);
  const strip = ductusFilmstrip(DEVANAGARI_TA, devanagariTaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left and curve down to the tip",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariTaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_TA.strokes[2], 1));
  });
});

describe("Devanagari द — short stem runs into the outer body, curl, and tail", () => {
  const steps = ductusSteps(DEVANAGARI_DA);
  const strip = ductusFilmstrip(DEVANAGARI_DA, devanagariDaOutline);

  it("shows three movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the short stem",
      "continue round the body, curl and tail",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariDaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_DA.strokes[1], 1));
  });
});

describe("Devanagari ध — upper spiral, lower bowl, and right stem in one run", () => {
  const steps = ductusSteps(DEVANAGARI_DHA);
  const strip = ductusFilmstrip(DEVANAGARI_DHA, devanagariDhaOutline);

  it("shows six movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curl the spiral and pull the shoulder right",
      "turn back left along the shoulder",
      "sweep down and around the lower bowl",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariDhaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_DHA.strokes[1], 1));
  });
});

describe("Devanagari न — clockwise loop and shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_NA);
  const strip = ductusFilmstrip(DEVANAGARI_NA, devanagariNaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the loop clockwise, sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariNaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_NA.strokes[2], 1));
  });
});

describe("Devanagari प — left stem and bowl run into the right stem", () => {
  const steps = ductusSteps(DEVANAGARI_PA);
  const strip = ductusFilmstrip(DEVANAGARI_PA, devanagariPaOutline);

  it("shows four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem and round the bowl",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariPaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_PA.strokes[1], 1));
  });
});

describe("Devanagari ब — oval runs into the right stem before the inner diagonal", () => {
  const steps = ductusSteps(DEVANAGARI_BA);
  const strip = ductusFilmstrip(DEVANAGARI_BA, devanagariBaOutline);

  it("shows five movements across three strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the oval body counterclockwise",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then cross the body down and right",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariBaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_BA.strokes[2], 1));
  });
});

describe("Devanagari भ — joined loops and crossbar run into the right stem", () => {
  const steps = ductusSteps(DEVANAGARI_BHA);
  const strip = ductusFilmstrip(DEVANAGARI_BHA, devanagariBhaOutline);

  it("shows four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle both loops clockwise, sweep right",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariBhaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_BHA.strokes[1], 1));
  });
});

describe("Devanagari म — left stem, lower loop, and crossbar run into the right stem", () => {
  const steps = ductusSteps(DEVANAGARI_MA);
  const strip = ductusFilmstrip(DEVANAGARI_MA, devanagariMaOutline);

  it("shows four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend, loop clockwise, sweep right",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariMaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_MA.strokes[1], 1));
  });
});

describe("Devanagari य — inner curl, lower bowl, and right stem in one run", () => {
  const steps = ductusSteps(DEVANAGARI_YA);
  const strip = ductusFilmstrip(DEVANAGARI_YA, devanagariYaOutline);

  it("shows five movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve clockwise around the inner curl",
      "continue around the lower bowl to the right",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariYaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_YA.strokes[1], 1));
  });
});

describe("Devanagari र — looped stem runs into the diagonal tail", () => {
  const steps = ductusSteps(DEVANAGARI_RA);
  const strip = ductusFilmstrip(DEVANAGARI_RA, devanagariRaOutline);

  it("shows three movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend and curl clockwise into the loop",
      "continue down-right along the diagonal tail",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariRaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_RA.strokes[1], 1));
  });
});

describe("Devanagari ल — open loop, diagonal arm, and right stem in one run", () => {
  const steps = ductusSteps(DEVANAGARI_LA);
  const strip = ductusFilmstrip(DEVANAGARI_LA, devanagariLaOutline);

  it("shows five movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve up clockwise round the open loop",
      "sweep the diagonal arm up-right",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariLaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_LA.strokes[1], 1));
  });
});

describe("Devanagari व — counterclockwise loop runs into the right stem", () => {
  const steps = ductusSteps(DEVANAGARI_VA);
  const strip = ductusFilmstrip(DEVANAGARI_VA, devanagariVaOutline);

  it("shows four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the left loop counterclockwise",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
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

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariVaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_VA.strokes[1], 1));
  });
});

describe("Devanagari श — joined double-loop body before stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_SHA);
  const strip = ductusFilmstrip(DEVANAGARI_SHA, devanagariShaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "trace both loops and the diagonal tail",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariShaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_SHA.strokes[2], 1));
  });
});

describe("Devanagari स — crossbar runs into the right stem after the joined hook and tail", () => {
  const steps = ductusSteps(DEVANAGARI_SA);
  const strip = ductusFilmstrip(DEVANAGARI_SA, devanagariSaOutline);

  it("shows five movements across three strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend through the hook and diagonal tail",
      "lift, then draw the middle bar rightward",
      "climb up the right stem without lifting",
      "descend the right stem",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 2]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariSaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_SA.strokes[2], 1));
  });
});

describe("Devanagari ह — joined stem and hooked body before the outer tail", () => {
  const steps = ductusSteps(DEVANAGARI_HA);
  const strip = ductusFilmstrip(DEVANAGARI_HA, devanagariHaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend, sweep left, round the hooked body",
      "lift, then sweep down-left into the tail",
      "lift, then draw the shirorekha rightward",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariHaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_HA.strokes[2], 1));
  });
});
