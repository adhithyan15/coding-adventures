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

const CYRILLIC_A = ductusFor("а", "cyrillic")!;
const cyrillicAOutline = cyrillicOutline("а");
const CYRILLIC_BE = ductusFor("б", "cyrillic")!;
const cyrillicBeOutline = cyrillicOutline("б");
const CYRILLIC_VE = ductusFor("в", "cyrillic")!;
const cyrillicVeOutline = cyrillicOutline("в");
const CYRILLIC_GE = ductusFor("г", "cyrillic")!;
const cyrillicGeOutline = cyrillicOutline("г");
const CYRILLIC_DE = ductusFor("д", "cyrillic")!;
const cyrillicDeOutline = cyrillicOutline("д");
const CYRILLIC_IE = ductusFor("е", "cyrillic")!;
const cyrillicIeOutline = cyrillicOutline("е");
const CYRILLIC_IO = ductusFor("ё", "cyrillic")!;
const cyrillicIoOutline = cyrillicOutline("ё");
const CYRILLIC_ZHE = ductusFor("ж", "cyrillic")!;
const cyrillicZheOutline = cyrillicOutline("ж");
const CYRILLIC_ZE = ductusFor("з", "cyrillic")!;
const cyrillicZeOutline = cyrillicOutline("з");
const CYRILLIC_I = ductusFor("и", "cyrillic")!;
const cyrillicIOutline = cyrillicOutline("и");
const CYRILLIC_SHORT_I = ductusFor("й", "cyrillic")!;
const cyrillicShortIOutline = cyrillicOutline("й");
const CYRILLIC_KA = ductusFor("к", "cyrillic")!;
const cyrillicKaOutline = cyrillicOutline("к");
const CYRILLIC_EL = ductusFor("л", "cyrillic")!;
const cyrillicElOutline = cyrillicOutline("л");
const CYRILLIC_EM = ductusFor("м", "cyrillic")!;
const cyrillicEmOutline = cyrillicOutline("м");
const CYRILLIC_EN = ductusFor("н", "cyrillic")!;
const cyrillicEnOutline = cyrillicOutline("н");
const CYRILLIC_O = ductusFor("о", "cyrillic")!;
const cyrillicOOutline = cyrillicOutline("о");
const CYRILLIC_PE = ductusFor("п", "cyrillic")!;
const cyrillicPeOutline = cyrillicOutline("п");
const CYRILLIC_ER = ductusFor("р", "cyrillic")!;
const cyrillicErOutline = cyrillicOutline("р");
const CYRILLIC_ES = ductusFor("с", "cyrillic")!;
const cyrillicEsOutline = cyrillicOutline("с");
const CYRILLIC_TE = ductusFor("т", "cyrillic")!;
const cyrillicTeOutline = cyrillicOutline("т");
const CYRILLIC_U = ductusFor("у", "cyrillic")!;
const cyrillicUOutline = cyrillicOutline("у");
const CYRILLIC_EF = ductusFor("ф", "cyrillic")!;
const cyrillicEfOutline = cyrillicOutline("ф");
const CYRILLIC_HA = ductusFor("х", "cyrillic")!;
const cyrillicHaOutline = cyrillicOutline("х");
const CYRILLIC_TSE = ductusFor("ц", "cyrillic")!;
const cyrillicTseOutline = cyrillicOutline("ц");
const CYRILLIC_CHE = ductusFor("ч", "cyrillic")!;
const cyrillicCheOutline = cyrillicOutline("ч");
const CYRILLIC_SHA = ductusFor("ш", "cyrillic")!;
const cyrillicShaOutline = cyrillicOutline("ш");
const CYRILLIC_SHCHA = ductusFor("щ", "cyrillic")!;
const cyrillicShchaOutline = cyrillicOutline("щ");
const CYRILLIC_HARD_SIGN = ductusFor("ъ", "cyrillic")!;
const cyrillicHardSignOutline = cyrillicOutline("ъ");
const CYRILLIC_YERY = ductusFor("ы", "cyrillic")!;
const cyrillicYeryOutline = cyrillicOutline("ы");
const CYRILLIC_SOFT_SIGN = ductusFor("ь", "cyrillic")!;
const cyrillicSoftSignOutline = cyrillicOutline("ь");
const CYRILLIC_E = ductusFor("э", "cyrillic")!;
const cyrillicEOutline = cyrillicOutline("э");
const CYRILLIC_YU = ductusFor("ю", "cyrillic")!;
const cyrillicYuOutline = cyrillicOutline("ю");
const CYRILLIC_YA = ductusFor("я", "cyrillic")!;
const cyrillicYaOutline = cyrillicOutline("я");

describe("Cyrillic а — one joined body and finishing stem", () => {
  const steps = ductusSteps(CYRILLIC_A);
  const strip = ductusFilmstrip(CYRILLIC_A, cyrillicAOutline);

  it("shows two movements within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep over the shoulder and around the round body",
      "continue down the right-hand finishing stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the finishing stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicAOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_A.strokes[0], 1));
  });
});

describe("Cyrillic б — one joined lower body and top flag", () => {
  const steps = ductusSteps(CYRILLIC_BE);
  const strip = ductusFilmstrip(CYRILLIC_BE, cyrillicBeOutline);

  it("shows the body and top flag within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the rounded lower body",
      "continue through the rising shoulder and sweep the top flag right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the top flag", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicBeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_BE.strokes[0], 1));
  });
});

describe("Cyrillic в — one joined upper loop and lower bowl", () => {
  const steps = ductusSteps(CYRILLIC_VE);
  const strip = ductusFilmstrip(CYRILLIC_VE, cyrillicVeOutline);

  it("shows the upper loop and lower bowl within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb through the upper loop and descend to the baseline",
      "continue counterclockwise around the rounded lower bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the lower bowl", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicVeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_VE.strokes[0], 1));
  });
});

describe("Cyrillic г — one zero-lift printed fit for the cursive humps", () => {
  const steps = ductusSteps(CYRILLIC_GE);
  const strip = ductusFilmstrip(CYRILLIC_GE, cyrillicGeOutline);

  it("shows the outward and returning paths within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the upright and sweep the top bar right",
      "reverse along the top and descend to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the returning path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicGeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_GE.strokes[0], 1));
  });
});

describe("Cyrillic д — one zero-lift body and retraced printed base", () => {
  const steps = ductusSteps(CYRILLIC_DE);
  const strip = ductusFilmstrip(CYRILLIC_DE, cyrillicDeOutline);

  it("shows the closed body before the joined base-and-feet movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the closed body",
      "descend, retrace both feet, and finish along the base shelf",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicDeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_DE.strokes[0], 1));
  });
});

describe("Cyrillic е — one zero-lift upper loop and lower bowl", () => {
  const steps = ductusSteps(CYRILLIC_IE);
  const strip = ductusFilmstrip(CYRILLIC_IE, cyrillicIeOutline);

  it("shows the upper bowl and middle crossing before the lower bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve around the upper bowl and sweep through the middle",
      "reverse through the middle and circle the lower bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicIeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_IE.strokes[0], 1));
  });
});

describe("Cyrillic ё — looped body followed by two lifted dots", () => {
  const steps = ductusSteps(CYRILLIC_IO);
  const strip = ductusFilmstrip(CYRILLIC_IO, cyrillicIoOutline);

  it("shows the joined body before the left and right dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve around the upper bowl and sweep through the middle",
      "reverse through the middle and circle the lower bowl",
      "lift and place the left dot",
      "lift again and place the right dot",
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

  it("draws the exact dotted Noto Sans Cyrillic glyph behind all three runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicIoOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(2);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_IO.strokes[2], 1));
  });
});

describe("Cyrillic ж — one continuous left-centre-right run", () => {
  const steps = ductusSteps(CYRILLIC_ZHE);
  const strip = ductusFilmstrip(CYRILLIC_ZHE, cyrillicZheOutline);

  it("shows the left wings and centre before the joined right wings", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "trace the left wings and rise through the centre",
      "retrace the centre and trace the right wings",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicZheOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_ZHE.strokes[0], 1));
  });
});

describe("Cyrillic з — one continuous double-lobe run", () => {
  const steps = ductusSteps(CYRILLIC_ZE);
  const strip = ductusFilmstrip(CYRILLIC_ZE, cyrillicZeOutline);

  it("shows the smaller upper lobe before the joined larger lower lobe", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the smaller upper lobe and descend through the middle",
      "circle the larger lower lobe and finish at the lower right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicZeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_ZE.strokes[0], 1));
  });
});

describe("Cyrillic и — one continuous stem-diagonal-stem run", () => {
  const steps = ductusSteps(CYRILLIC_I);
  const strip = ductusFilmstrip(CYRILLIC_I, cyrillicIOutline);

  it("shows the left stem, rising diagonal, and right stem without a lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise diagonally to the upper right",
      "descend the right stem and finish at the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicIOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_I.strokes[0], 1));
  });
});

describe("Cyrillic й — joined body followed by a lifted breve", () => {
  const steps = ductusSteps(CYRILLIC_SHORT_I);
  const strip = ductusFilmstrip(CYRILLIC_SHORT_I, cyrillicShortIOutline);

  it("shows the three-part body before the separately drawn breve", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise diagonally to the upper right",
      "descend the right stem and finish at the baseline",
      "lift, then draw the breve from left to right",
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

  it("keeps the joined body visible over the exact breve-bearing glyph", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicShortIOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_SHORT_I.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_SHORT_I.strokes[1], 1));
  });
});

describe("Cyrillic к — one joined stem-and-arms school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_KA);
  const strip = ductusFilmstrip(CYRILLIC_KA, cyrillicKaOutline);

  it("shows the descending stem before the upper and lower arms", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise through the upper arm and return to the middle junction",
      "continue down-right through the lower arm to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicKaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_KA.strokes[0], 1));
  });
});

describe("Cyrillic л — one joined hook-to-legs school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EL);
  const strip = ductusFilmstrip(CYRILLIC_EL, cyrillicElOutline);

  it("shows the hooked left leg before the top shoulder and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve from the baseline hook up the left leg",
      "sweep right along the top shoulder",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicElOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_EL.strokes[0], 1));
  });
});

describe("Cyrillic м — one joined two-arch school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EM);
  const strip = ductusFilmstrip(CYRILLIC_EM, cyrillicEmOutline);

  it("shows the left stem before the central valley, second apex, and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "rise from the baseline through the left stem",
      "descend diagonally to the central valley",
      "rise diagonally to the second apex",
      "descend the right stem to the baseline",
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

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicEmOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_EM.strokes[0], 1));
  });
});

describe("Cyrillic н — one joined middle-bridge school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EN);
  const strip = ductusFilmstrip(CYRILLIC_EN, cyrillicEnOutline);

  it("shows the left stem before the middle bridge and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace to the middle bridge and rise to the upper right",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicEnOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_EN.strokes[0], 1));
  });
});

describe("Cyrillic о — one closed counterclockwise school-hand oval", () => {
  const steps = ductusSteps(CYRILLIC_O);
  const strip = ductusFilmstrip(CYRILLIC_O, cyrillicOOutline);

  it("shows the top and left side before the bottom, right side, and closure", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve left over the top and descend the left side",
      "sweep through the bottom and rise to close the oval",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the closed path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicOOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_O.strokes[0], 1));
  });
});

describe("Cyrillic п — one joined top-shoulder school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_PE);
  const strip = ductusFilmstrip(CYRILLIC_PE, cyrillicPeOutline);

  it("shows the left stem before the top shoulder and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace to the top shoulder and sweep right",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicPeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_PE.strokes[0], 1));
  });
});

describe("Cyrillic р — one joined descender-and-bowl school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_ER);
  const strip = ductusFilmstrip(CYRILLIC_ER, cyrillicErOutline);

  it("shows the descender before the retraced shoulder and closed bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the stem below the baseline",
      "retrace to the upper shoulder and curve right",
      "sweep around the bowl and return to the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicErOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_ER.strokes[0], 1));
  });
});

describe("Cyrillic с — one open counterclockwise school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_ES);
  const strip = ductusFilmstrip(CYRILLIC_ES, cyrillicEsOutline);

  it("shows the upper-left sweep before the lower-right exit", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve left over the top and descend the left side",
      "sweep through the bottom and rise to the lower-right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the open curve", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicEsOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_ES.strokes[0], 1));
  });
});

describe("Cyrillic т — one joined central-stem-and-top-bar run", () => {
  const steps = ductusSteps(CYRILLIC_TE);
  const strip = ductusFilmstrip(CYRILLIC_TE, cyrillicTeOutline);

  it("shows the central descent before both halves of the top bar", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the central stem to the baseline",
      "retrace to the top junction and sweep left",
      "retrace through the junction and sweep to the right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicTeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_TE.strokes[0], 1));
  });
});

describe("Cyrillic у — one joined upper-body-and-descender run", () => {
  const steps = ductusSteps(CYRILLIC_U);
  const strip = ductusFilmstrip(CYRILLIC_U, cyrillicUOutline);

  it("shows both upper arms before the long left-curving terminal", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left arm to the middle junction",
      "turn and rise through the right arm",
      "retrace to the junction and descend below the baseline",
      "curve left through the descender terminal",
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

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicUOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_U.strokes[0], 1));
  });
});

describe("Cyrillic ф — stem first, then one joined two-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_EF);
  const strip = ductusFilmstrip(CYRILLIC_EF, cyrillicEfOutline);

  it("shows the long stem before the lifted left-to-right bowl sequence", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long central stem below the baseline",
      "lift and curve over and around the left bowl",
      "sweep through the lower-left curve to the centre",
      "continue through the lower-right curve",
      "rise over the right bowl to the upper junction",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind both runs", () => {
    const paths = byTag(strip.frames[4], "path");
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    const pen = paths.find((path) => path.attrs.class === "ductus__pen")!;
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicEfOutline.path);
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(CYRILLIC_EF.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(CYRILLIC_EF.strokes[1], 1));
  });
});

describe("Cyrillic х — two facing curves fitted through one printed crossing", () => {
  const steps = ductusSteps(CYRILLIC_HA);
  const strip = ductusFilmstrip(CYRILLIC_HA, cyrillicHaOutline);

  it("shows the complete left run before the lifted right run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper-left tip to the central crossing",
      "sweep down-left from the crossing to the lower-left tip",
      "lift and descend from the upper-right tip to the crossing",
      "sweep down-right from the crossing to the lower-right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    const pen = paths.find((path) => path.attrs.class === "ductus__pen")!;
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicHaOutline.path);
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(CYRILLIC_HA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(CYRILLIC_HA.strokes[1], 1));
  });
});

describe("Cyrillic ц — one joined stem-to-stem-to-tail run", () => {
  const steps = ductusSteps(CYRILLIC_TSE);
  const strip = ductusFilmstrip(CYRILLIC_TSE, cyrillicTseOutline);

  it("keeps the square printed body and descender in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "sweep along the base and rise through the right stem",
      "retrace the right stem and cross the tail shoulder",
      "descend the short tail below the baseline",
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

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicTseOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_TSE.strokes[0], 1));
  });
});

describe("Cyrillic ч — one joined short-stem-to-bowl-to-long-stem run", () => {
  const steps = ductusSteps(CYRILLIC_CHE);
  const strip = ductusFilmstrip(CYRILLIC_CHE, cyrillicCheOutline);

  it("keeps the shorter left stem, bowl, and full right stem in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the short left stem to the middle join",
      "sweep through the bowl and rise along the right stem",
      "descend the full right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicCheOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_CHE.strokes[0], 1));
  });
});

describe("Cyrillic ш — one joined three-stem run", () => {
  const steps = ductusSteps(CYRILLIC_SHA);
  const strip = ductusFilmstrip(CYRILLIC_SHA, cyrillicShaOutline);

  it("keeps all three stems and both base joins in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "cross the first base join and rise through the middle stem",
      "retrace the middle stem to the baseline",
      "cross the second base join and rise through the right stem",
      "retrace the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicShaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_SHA.strokes[0], 1));
  });
});

describe("Cyrillic щ — one joined three-stem-to-tail run", () => {
  const steps = ductusSteps(CYRILLIC_SHCHA);
  const strip = ductusFilmstrip(CYRILLIC_SHCHA, cyrillicShchaOutline);

  it("keeps all three stems, both joins, and the tail in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "cross the first base join and rise through the middle stem",
      "retrace the middle stem to the baseline",
      "cross the second base join and rise through the right stem",
      "retrace the right stem and cross the tail shoulder",
      "descend the short tail below the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 6 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicShchaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_SHCHA.strokes[0], 1));
  });
});

describe("Cyrillic ъ — one joined flag-to-stem-to-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_HARD_SIGN);
  const strip = ductusFilmstrip(CYRILLIC_HARD_SIGN, cyrillicHardSignOutline);

  it("keeps the top flag, descending stem, and lower bowl in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right along the broad top flag",
      "descend the main stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicHardSignOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_HARD_SIGN.strokes[0], 1));
  });
});

describe("Cyrillic ы — joined left body before a lifted right stem", () => {
  const steps = ductusSteps(CYRILLIC_YERY);
  const strip = ductusFilmstrip(CYRILLIC_YERY, cyrillicYeryOutline);

  it("keeps the left stem and bowl together before the separate right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
      "lift, then descend the separate right stem",
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

  it("draws the exact Noto Sans Cyrillic character behind the final stem", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicYeryOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(CYRILLIC_YERY.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_YERY.strokes[1], 1));
  });
});

describe("Cyrillic ь — one joined stem-and-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_SOFT_SIGN);
  const strip = ductusFilmstrip(CYRILLIC_SOFT_SIGN, cyrillicSoftSignOutline);

  it("keeps the descending stem joined to the counterclockwise lower bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
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

  it("draws the exact Noto Sans Cyrillic character behind the closed bowl", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicSoftSignOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_SOFT_SIGN.strokes[0], 1));
  });
});

describe("Cyrillic э — outer curve before a lifted middle tongue", () => {
  const steps = ductusSteps(CYRILLIC_E);
  const strip = ductusFilmstrip(CYRILLIC_E, cyrillicEOutline);

  it("keeps the backwards-C run before the right-to-left tongue", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right across the upper curve",
      "continue down around the outer right side",
      "sweep left through the lower curve",
      "lift, then draw the middle tongue right-to-left",
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

  it("draws the exact Noto Sans Cyrillic character behind the final tongue", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicEOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([penPathD(CYRILLIC_E.strokes[0], 1)]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_E.strokes[1], 1));
  });
});

describe("Cyrillic ю — one joined stem-to-oval run", () => {
  const steps = ductusSteps(CYRILLIC_YU);
  const strip = ductusFilmstrip(CYRILLIC_YU, cyrillicYuOutline);

  it("keeps the left stem and connector joined to the clockwise oval", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace upward and sweep right along the middle bar",
      "curve upward around the oval and across its top",
      "continue down around the oval's right side",
      "sweep left through the bottom and rise to close",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the closed oval", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicYuOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_YU.strokes[0], 1));
  });
});

describe("Cyrillic я — one joined rise-to-loop-to-leg run", () => {
  const steps = ductusSteps(CYRILLIC_YA);
  const strip = ductusFilmstrip(CYRILLIC_YA, cyrillicYaOutline);

  it("keeps the rising stem, counterclockwise bowl, and diagonal leg joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the right stem from the baseline to the top",
      "curve counterclockwise around the upper bowl",
      "sweep left through the bowl's lower join",
      "descend the diagonal leg to the lower-left tip",
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

  it("draws the exact Noto Sans Cyrillic character behind the joined run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(cyrillicYaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CYRILLIC_YA.strokes[0], 1));
  });
});
