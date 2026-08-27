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

const HEBREW_ALEF = ductusFor("א", "hebrew")!;
const hebrewAlefOutline = hebrewOutline("א");
const HEBREW_BET = ductusFor("ב", "hebrew")!;
const hebrewBetOutline = hebrewOutline("ב");
const HEBREW_GIMEL = ductusFor("ג", "hebrew")!;
const hebrewGimelOutline = hebrewOutline("ג");
const HEBREW_DALET = ductusFor("ד", "hebrew")!;
const hebrewDaletOutline = hebrewOutline("ד");
const HEBREW_HEI = ductusFor("ה", "hebrew")!;
const hebrewHeiOutline = hebrewOutline("ה");
const HEBREW_VAV = ductusFor("ו", "hebrew")!;
const hebrewVavOutline = hebrewOutline("ו");
const HEBREW_ZAYIN = ductusFor("ז", "hebrew")!;
const hebrewZayinOutline = hebrewOutline("ז");
const HEBREW_HEIT = ductusFor("ח", "hebrew")!;
const hebrewHeitOutline = hebrewOutline("ח");
const HEBREW_TET = ductusFor("ט", "hebrew")!;
const hebrewTetOutline = hebrewOutline("ט");
const HEBREW_YOD = ductusFor("י", "hebrew")!;
const hebrewYodOutline = hebrewOutline("י");
const HEBREW_KAF = ductusFor("כ", "hebrew")!;
const hebrewKafOutline = hebrewOutline("כ");
const HEBREW_LAMED = ductusFor("ל", "hebrew")!;
const hebrewLamedOutline = hebrewOutline("ל");
const HEBREW_MEM = ductusFor("מ", "hebrew")!;
const hebrewMemOutline = hebrewOutline("מ");
const HEBREW_NUN = ductusFor("נ", "hebrew")!;
const hebrewNunOutline = hebrewOutline("נ");
const HEBREW_SAMEKH = ductusFor("ס", "hebrew")!;
const hebrewSamekhOutline = hebrewOutline("ס");
const HEBREW_AYIN = ductusFor("ע", "hebrew")!;
const hebrewAyinOutline = hebrewOutline("ע");
const HEBREW_PE = ductusFor("פ", "hebrew")!;
const hebrewPeOutline = hebrewOutline("פ");
const HEBREW_TSADI = ductusFor("צ", "hebrew")!;
const hebrewTsadiOutline = hebrewOutline("צ");
const HEBREW_QOF = ductusFor("ק", "hebrew")!;
const hebrewQofOutline = hebrewOutline("ק");
const HEBREW_RESH = ductusFor("ר", "hebrew")!;
const hebrewReshOutline = hebrewOutline("ר");
const HEBREW_SHIN = ductusFor("ש", "hebrew")!;
const hebrewShinOutline = hebrewOutline("ש");
const HEBREW_TAV = ductusFor("ת", "hebrew")!;
const hebrewTavOutline = hebrewOutline("ת");

describe("Hebrew א — two crossed handwritten runs fitted to the block outline", () => {
  const steps = ductusSteps(HEBREW_ALEF);
  const strip = ductusFilmstrip(HEBREW_ALEF, hebrewAlefOutline);

  it("shows the main diagonal before the lifted opposing run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the main diagonal down and right",
      "lift, then descend from the upper-right arm to the crossing",
      "continue through the crossing and down the lower-left leg",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("keeps the first run visible over the vendored Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewAlefOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_ALEF.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_ALEF.strokes[1], 1));
  });
});

describe("Hebrew ב — its top and right side precede the lifted baseline", () => {
  const steps = ductusSteps(HEBREW_BET);
  const strip = ductusFilmstrip(HEBREW_BET, hebrewBetOutline);

  it("shows the sourced three movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the baseline from left to right",
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

  it("keeps the joined top-and-right stroke over the Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewBetOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_BET.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_BET.strokes[1], 1));
  });
});

describe("Hebrew ג — its joined top and right leg precede the lifted left leg", () => {
  const steps = ductusSteps(HEBREW_GIMEL);
  const strip = ductusFilmstrip(HEBREW_GIMEL, hebrewGimelOutline);

  it("shows the sourced four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short top bar from left to right",
      "continue down the right stem without lifting",
      "continue into the short lower-right leg",
      "lift, restart at the lower junction, and draw the longer leg down-left",
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

  it("keeps the first angular run visible over the Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewGimelOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_GIMEL.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_GIMEL.strokes[1], 1));
  });
});

describe("Hebrew ד — one sourced curve fitted to the angular block outline", () => {
  const steps = ductusSteps(HEBREW_DALET);
  const strip = ductusFilmstrip(HEBREW_DALET, hebrewDaletOutline);

  it("keeps the top bar and right descent in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue around the sharp right corner and down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the continuous path over Noto Sans Hebrew without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewDaletOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_DALET.strokes[0], 2));
  });
});

describe("Hebrew ה — joined top and right body plus a detached left leg", () => {
  const steps = ductusSteps(HEBREW_HEI);
  const strip = ductusFilmstrip(HEBREW_HEI, hebrewHeiOutline);

  it("keeps the top and right side joined before restarting the left leg", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the detached left leg from top to bottom",
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

  it("draws Noto Sans Hebrew and preserves the completed body behind the detached leg", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewHeiOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_HEI.strokes[0], 2));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_HEI.strokes[1], 1));
  });
});

describe("Hebrew ו — one joined head-and-stem stroke", () => {
  const steps = ductusSteps(HEBREW_VAV);
  const strip = ductusFilmstrip(HEBREW_VAV, hebrewVavOutline);

  it("keeps the small head joined to the top-to-bottom stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the small head from left to right",
      "continue straight down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws Noto Sans Hebrew with no completed-stroke overlay before the stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewVavOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_VAV.strokes[0], 2));
  });
});

describe("Hebrew ז — one joined head-and-curved-stem stroke", () => {
  const steps = ductusSteps(HEBREW_ZAYIN);
  const strip = ductusFilmstrip(HEBREW_ZAYIN, hebrewZayinOutline);

  it("keeps the short head joined to the curved descent", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short head from left to right",
      "continue down through the curved stem without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws Noto Sans Hebrew with no completed-stroke overlay before the stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewZayinOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_ZAYIN.strokes[0], 2));
  });
});

describe("Hebrew ח — joined top and right body plus a joined left leg", () => {
  const steps = ductusSteps(HEBREW_HEIT);
  const strip = ductusFilmstrip(HEBREW_HEIT, hebrewHeitOutline);

  it("keeps the top and right side joined before restarting the left leg", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the joined left leg from top to bottom",
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

  it("draws Noto Sans Hebrew and preserves the completed body behind the left leg", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewHeitOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_HEIT.strokes[0], 2));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_HEIT.strokes[1], 1));
  });
});

describe("Hebrew ט — left-and-base body plus a bottom-up hooked side", () => {
  const steps = ductusSteps(HEBREW_TET);
  const strip = ductusFilmstrip(HEBREW_TET, hebrewTetOutline);

  it("keeps each body pair joined with one restart at the lower right", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left side from top to bottom",
      "continue around the bottom from left to right without lifting",
      "lift, restart at the lower-right, and climb the right side",
      "turn down-left into the inward hook without lifting",
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

  it("draws Noto Sans Hebrew and preserves the first body behind the hooked side", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewTetOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_TET.strokes[0], 2));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_TET.strokes[1], 2));
  });
});

describe("Hebrew י — one tiny joined head-and-stem stroke", () => {
  const steps = ductusSteps(HEBREW_YOD);
  const strip = ductusFilmstrip(HEBREW_YOD, hebrewYodOutline);

  it("keeps the tiny head joined to its short stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the small head from left to right",
      "continue down through the short angled stem without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact compact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewYodOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_YOD.strokes[0], 2));
  });
});

describe("Hebrew כ — one continuous sharp-cornered half-circle", () => {
  const steps = ductusSteps(HEBREW_KAF);
  const strip = ductusFilmstrip(HEBREW_KAF, hebrewKafOutline);

  it("keeps the top, rounded side, and base in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the rounded right side without lifting",
      "turn left along the base without lifting",
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

  it("draws the exact Noto Sans Hebrew Kaf in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewKafOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_KAF.strokes[0], 2));
  });
});

describe("Hebrew ל — one tall angular run", () => {
  const steps = ductusSteps(HEBREW_LAMED);
  const strip = ductusFilmstrip(HEBREW_LAMED, hebrewLamedOutline);

  it("keeps the tall stroke, middle bar, and diagonal lower stroke joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the tall left stroke from top to bottom",
      "continue right along the middle bar without lifting",
      "turn diagonally down-left through the lower stroke without lifting",
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

  it("draws the exact tall Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewLamedOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_LAMED.strokes[0], 2));
  });
});

describe("Hebrew מ — detached angled part, then one joined angular body", () => {
  const steps = ductusSteps(HEBREW_MEM);
  const strip = ductusFilmstrip(HEBREW_MEM, hebrewMemOutline);

  it("shows the source's five movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the detached left part from its lower tip up to the corner",
      "turn down-right through its short inner leg without lifting",
      "lift, then climb diagonally right through the upper shoulder",
      "turn down the right side without lifting",
      "turn left along the base without lifting, stopping before the left part",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact open Noto Sans Hebrew glyph and preserves the diagonal", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewMemOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_MEM.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_MEM.strokes[1], 2));
  });
});

describe("Hebrew נ — one joined printed hook", () => {
  const steps = ductusSteps(HEBREW_NUN);
  const strip = ductusFilmstrip(HEBREW_NUN, hebrewNunOutline);

  it("keeps the head, right descent, and leftward base joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short top head from left to right",
      "continue down the right side without lifting",
      "turn left along the base without lifting",
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

  it("draws the exact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewNunOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_NUN.strokes[0], 2));
  });
});

describe("Hebrew ס — one closed clockwise printed loop", () => {
  const steps = ductusSteps(HEBREW_SAMEKH);
  const strip = ductusFilmstrip(HEBREW_SAMEKH, hebrewSamekhOutline);

  it("keeps the top, right side, base, and closing left side joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the flat top from left to right",
      "round down the right side without lifting",
      "sweep left along the base without lifting",
      "climb the left side and close the loop without lifting",
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

  it("draws the exact closed Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewSamekhOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_SAMEKH.strokes[0], 3));
  });
});

describe("Hebrew ע — one joined branch-and-base run", () => {
  const steps = ductusSteps(HEBREW_AYIN);
  const strip = ductusFilmstrip(HEBREW_AYIN, hebrewAyinOutline);

  it("keeps the right descent, base, and left climb joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the right branch and curve left into the base",
      "sweep left along the base without lifting",
      "turn back and climb the left branch without lifting",
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

  it("draws the exact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewAyinOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_AYIN.strokes[0], 2));
  });
});

describe("Hebrew פ — an outer body followed by a lifted inner curl", () => {
  const steps = ductusSteps(HEBREW_PE);
  const strip = ductusFilmstrip(HEBREW_PE, hebrewPeOutline);

  it("keeps the top, side, and base joined before the inner curl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the outer top from left to right",
      "turn down the right side without lifting",
      "return left along the base without lifting",
      "lift, then draw the short inner curl from left to right",
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

  it("draws the exact Noto Sans Hebrew glyph and preserves the outer body", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewPeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_PE.strokes[0], 2));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_PE.strokes[1], 1));
  });
});

describe("Hebrew צ — a joined diagonal and base followed by a lifted arm", () => {
  const steps = ductusSteps(HEBREW_TSADI);
  const strip = ductusFilmstrip(HEBREW_TSADI, hebrewTsadiOutline);

  it("keeps the long diagonal joined to the base before the upper-right arm", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long diagonal from the upper left",
      "turn left along the base without lifting",
      "lift, then curve the upper-right arm down-left into the junction",
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

  it("draws the exact Noto Sans Hebrew glyph and preserves the first run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewTsadiOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_TSADI.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_TSADI.strokes[1], 1));
  });
});

describe("Hebrew ק — a joined top and right body followed by a lifted stem", () => {
  const steps = ductusSteps(HEBREW_QOF);
  const strip = ductusFilmstrip(HEBREW_QOF, hebrewQofOutline);

  it("keeps the top joined to the right body before the separate descender", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "turn down-left through the right body without lifting",
      "lift, then descend the separate inner-left stem below the line",
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

  it("draws the exact Noto Sans Hebrew glyph and preserves the first run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewQofOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_QOF.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_QOF.strokes[1], 1));
  });
});

describe("Hebrew ר — one rounded top-and-right run", () => {
  const steps = ductusSteps(HEBREW_RESH);
  const strip = ductusFilmstrip(HEBREW_RESH, hebrewReshOutline);

  it("keeps the top bar and rounded right descent joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "round the top-right corner and continue down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph with no completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewReshOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_RESH.strokes[0], 1));
  });
});

describe("Hebrew ש — an outer bowl followed by a lifted middle branch", () => {
  const steps = ductusSteps(HEBREW_SHIN);
  const strip = ductusFilmstrip(HEBREW_SHIN, hebrewShinOutline);

  it("keeps the outer bowl joined before the separate middle branch", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the right branch and round left along the base",
      "continue up the left branch without lifting",
      "lift, then descend the middle branch into the base",
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

  it("draws the exact Noto Sans Hebrew glyph and preserves the outer run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewShinOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_SHIN.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_SHIN.strokes[1], 1));
  });
});

describe("Hebrew ת — a joined top and right side, then a lifted left leg", () => {
  const steps = ductusSteps(HEBREW_TAV);
  const strip = ductusFilmstrip(HEBREW_TAV, hebrewTavOutline);

  it("keeps the top and right side joined before the separate left leg and foot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then descend the separate left leg",
      "curve left into the small foot without lifting",
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

  it("draws the exact Noto Sans Hebrew glyph and preserves both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(hebrewTavOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(HEBREW_TAV.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(HEBREW_TAV.strokes[1], 1));
  });
});
