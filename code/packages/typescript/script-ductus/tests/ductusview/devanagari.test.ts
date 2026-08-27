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

describe("Devanagari अ — joined left body before shoulder, stem, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_A);
  const strip = ductusFilmstrip(DEVANAGARI_A, devanagariAOutline);

  it("shows five movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariAOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_A.strokes[3], 1));
  });
});

describe("Devanagari आ — joined left body before shoulder, two stems, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AA);
  const strip = ductusFilmstrip(DEVANAGARI_AA, devanagariAaOutline);

  it("shows six movements across five sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 4]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the full headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_AA.strokes[4], 1));
  });
});

describe("Devanagari इ — continuous double-bowl body before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_I);
  const strip = ductusFilmstrip(DEVANAGARI_I, devanagariIOutline);

  it("shows five movements across two sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the upright from the headline",
      "turn left and curve around the upper bowl without lifting",
      "sweep right through the waist and around the lower bowl",
      "finish down-right through the tail without lifting",
      "lift, then draw the shirorekha left-to-right",
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
      "turn left and curve around the upper bowl without lifting",
      "sweep right through the waist and around the lower bowl",
      "finish down-right through the tail without lifting",
      "lift, then sweep the upper curl upward and around to the right",
      "lift, then draw the shirorekha left-to-right",
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
      "sweep back through the waist and around the lower loop without lifting",
      "lift, then draw the shirorekha left-to-right",
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
      "sweep back through the waist and around the lower loop without lifting",
      "lift, then sweep the right-hand loop up, around, and down-left",
      "lift, then draw the shirorekha left-to-right",
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
      "descend the long left stem from the headline",
      "curve right through the lower shoulder and sweep down the tail without lifting",
      "lift, then descend the shorter right stem into its inward hook",
      "lift, then draw the shirorekha left-to-right",
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
      "descend the long left stem from the headline",
      "curve right through the lower shoulder and sweep down the tail without lifting",
      "lift, then descend the shorter right stem into its inward hook",
      "lift, then sweep the upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
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

describe("Devanagari ओ — shared आ base before upper arc and headline", () => {
  const steps = ductusSteps(DEVANAGARI_O);
  const strip = ductusFilmstrip(DEVANAGARI_O, devanagariOOutline);

  it("shows seven movements across six sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 2, 3, 4, 5,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariOOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_O.strokes[5], 1));
  });
});

describe("Devanagari औ — shared आ base before two upper arcs and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AU);
  const strip = ductusFilmstrip(DEVANAGARI_AU, devanagariAuOutline);

  it("shows eight movements across seven sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the lower upper arc upward and left",
      "lift, then sweep the taller upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariAuOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_AU.strokes[6], 1));
  });
});

describe("Devanagari क — counterclockwise bowl before stem, arch, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_KA);
  const strip = ductusFilmstrip(DEVANAGARI_KA, devanagariKaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left over the top and around the bowl",
      "lift, then descend the central stem",
      "lift, then sweep the right-hand arch clockwise",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariKaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_KA.strokes[3], 1));
  });
});

describe("Devanagari ग — continuous loop and ascending stem before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_GA);
  const strip = ductusFilmstrip(DEVANAGARI_GA, devanagariGaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep counterclockwise around the loop and up the joined stem",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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

describe("Devanagari च — upper bar and rounded body before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_CA);
  const strip = ductusFilmstrip(DEVANAGARI_CA, devanagariCaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upper bar right and curve around the open body",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariCaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_CA.strokes[2], 1));
  });
});

describe("Devanagari त — right-to-left shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_TA);
  const strip = ductusFilmstrip(DEVANAGARI_TA, devanagariTaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder and curve down to the open tip",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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

describe("Devanagari द — short stem before the joined outer body, curl, and tail", () => {
  const steps = ductusSteps(DEVANAGARI_DA);
  const strip = ductusFilmstrip(DEVANAGARI_DA, devanagariDaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the short stem",
      "lift, then sweep around the body, inner curl, and tail",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariDaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_DA.strokes[2], 1));
  });
});

describe("Devanagari ध — upper spiral before the lower bowl and lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_DHA);
  const strip = ductusFilmstrip(DEVANAGARI_DHA, devanagariDhaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curl around the upper spiral and sweep right through the shoulder",
      "lift, then sweep down and around the lower bowl",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariDhaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_DHA.strokes[3], 1));
  });
});

describe("Devanagari न — clockwise loop and shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_NA);
  const strip = ductusFilmstrip(DEVANAGARI_NA, devanagariNaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise around the left loop and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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

describe("Devanagari प — descending left stem curves through the bowl before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_PA);
  const strip = ductusFilmstrip(DEVANAGARI_PA, devanagariPaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem and curve right around the lower bowl",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariPaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_PA.strokes[2], 1));
  });
});

describe("Devanagari ब — counterclockwise oval before the lifted stem and inner diagonal", () => {
  const steps = ductusSteps(DEVANAGARI_BA);
  const strip = ductusFilmstrip(DEVANAGARI_BA, devanagariBaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the oval body",
      "lift, then descend the right stem",
      "lift, then cross the body down and right",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariBaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_BA.strokes[3], 1));
  });
});

describe("Devanagari भ — joined clockwise loops before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_BHA);
  const strip = ductusFilmstrip(DEVANAGARI_BHA, devanagariBhaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise through both loops and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariBhaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_BHA.strokes[2], 1));
  });
});

describe("Devanagari म — descending left stem joins the clockwise lower loop", () => {
  const steps = ductusSteps(DEVANAGARI_MA);
  const strip = ductusFilmstrip(DEVANAGARI_MA, devanagariMaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem, circle clockwise through the loop, and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariMaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_MA.strokes[2], 1));
  });
});

describe("Devanagari य — inner curl precedes the restarted lower bowl", () => {
  const steps = ductusSteps(DEVANAGARI_YA);
  const strip = ductusFilmstrip(DEVANAGARI_YA, devanagariYaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve clockwise around the inner curl",
      "lift, then curve around the lower bowl to the right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariYaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_YA.strokes[3], 1));
  });
});

describe("Devanagari र — looped stem precedes the restarted diagonal tail", () => {
  const steps = ductusSteps(DEVANAGARI_RA);
  const strip = ductusFilmstrip(DEVANAGARI_RA, devanagariRaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend and curl clockwise around the lower loop",
      "lift, then draw the diagonal tail down-right",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariRaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_RA.strokes[2], 1));
  });
});

describe("Devanagari ल — open loop precedes the restarted diagonal arm", () => {
  const steps = ductusSteps(DEVANAGARI_LA);
  const strip = ductusFilmstrip(DEVANAGARI_LA, devanagariLaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve up and clockwise around the open left loop",
      "lift, then sweep the diagonal arm up-right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariLaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_LA.strokes[3], 1));
  });
});

describe("Devanagari व — counterclockwise loop before stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_VA);
  const strip = ductusFilmstrip(DEVANAGARI_VA, devanagariVaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the left loop",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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
    ).toBe(devanagariVaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_VA.strokes[2], 1));
  });
});

describe("Devanagari श — joined double-loop body before stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_SHA);
  const strip = ductusFilmstrip(DEVANAGARI_SHA, devanagariShaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "trace the joined double-loop body and diagonal tail",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
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

describe("Devanagari स — joined hook and tail before crossbar and stems", () => {
  const steps = ductusSteps(DEVANAGARI_SA);
  const strip = ductusFilmstrip(DEVANAGARI_SA, devanagariSaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend through the hook and diagonal tail",
      "lift, then draw the middle crossbar left-to-right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(devanagariSaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(DEVANAGARI_SA.strokes[3], 1));
  });
});

describe("Devanagari ह — joined stem and hooked body before the outer tail", () => {
  const steps = ductusSteps(DEVANAGARI_HA);
  const strip = ductusFilmstrip(DEVANAGARI_HA, devanagariHaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend, sweep left, and curve around the hooked body",
      "lift, then sweep down-left and through the diagonal tail",
      "lift, then draw the shirorekha left-to-right",
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
