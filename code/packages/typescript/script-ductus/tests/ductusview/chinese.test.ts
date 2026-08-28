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

const CHINESE_REN = ductusFor("人", "chinese")!;
const chineseRenOutline = chineseOutline("人");
const CHINESE_PERSON_RADICAL = ductusFor("亻", "chinese")!;
const chinesePersonRadicalOutline = chineseOutline("亻");
const CHINESE_MOUTH = ductusFor("口", "chinese")!;
const chineseMouthOutline = chineseOutline("口");
const CHINESE_WOMAN = ductusFor("女", "chinese")!;
const chineseWomanOutline = chineseOutline("女");
const CHINESE_CHILD = ductusFor("子", "chinese")!;
const chineseChildOutline = chineseOutline("子");
const CHINESE_SUN = ductusFor("日", "chinese")!;
const chineseSunOutline = chineseOutline("日");
const CHINESE_SPEECH_RADICAL = ductusFor("讠", "chinese")!;
const chineseSpeechRadicalOutline = chineseOutline("讠");
const CHINESE_WATER_RADICAL = ductusFor("氵", "chinese")!;
const chineseWaterRadicalOutline = chineseOutline("氵");
const CHINESE_ROOF_RADICAL = ductusFor("宀", "chinese")!;
const chineseRoofRadicalOutline = chineseOutline("宀");
const CHINESE_YOU = ductusFor("你", "chinese")!;
const chineseYouOutline = chineseOutline("你");
const CHINESE_GOOD = ductusFor("好", "chinese")!;
const chineseGoodOutline = chineseOutline("好");
const CHINESE_I = ductusFor("我", "chinese")!;
const chineseIOutline = chineseOutline("我");
const CHINESE_BE = ductusFor("是", "chinese")!;
const chineseBeOutline = chineseOutline("是");
const CHINESE_NOT = ductusFor("不", "chinese")!;
const chineseNotOutline = chineseOutline("不");
const CHINESE_NAME = ductusFor("名", "chinese")!;
const chineseNameOutline = chineseOutline("名");
const CHINESE_CHARACTER = ductusFor("字", "chinese")!;
const chineseCharacterOutline = chineseOutline("字");
const CHINESE_THANK = ductusFor("谢", "chinese")!;
const chineseThankOutline = chineseOutline("谢");
const CHINESE_PLEASE = ductusFor("请", "chinese")!;
const chinesePleaseOutline = chineseOutline("请");
const CHINESE_AGAIN = ductusFor("再", "chinese")!;
const chineseAgainOutline = chineseOutline("再");
const CHINESE_SEE = ductusFor("见", "chinese")!;
const chineseSeeOutline = chineseOutline("见");
const CHINESE_WHAT = ductusFor("什", "chinese")!;
const chineseWhatOutline = chineseOutline("什");
const CHINESE_PARTICLE_ME = ductusFor("么", "chinese")!;
const chineseParticleMeOutline = chineseOutline("么");
const CHINESE_EARLY = ductusFor("早", "chinese")!;
const chineseEarlyOutline = chineseOutline("早");
const CHINESE_UP = ductusFor("上", "chinese")!;
const chineseUpOutline = chineseOutline("上");

describe("Chinese 人 — two cited falling strokes in PRC order", () => {
  const steps = ductusSteps(CHINESE_REN);
  const strip = ductusFilmstrip(CHINESE_REN, chineseRenOutline);

  it("shows the left-falling stroke before restarting for the right-falling stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left-falling piě stroke from the upper centre",
      "lift, then draw the right-falling nà stroke from the junction",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans SC glyph with the first stroke settled behind the second", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseRenOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(CHINESE_REN.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_REN.strokes[1], 1));
  });
});

describe("Chinese 亻 — a cited falling stroke followed by a vertical", () => {
  const steps = ductusSteps(CHINESE_PERSON_RADICAL);
  const strip = ductusFilmstrip(
    CHINESE_PERSON_RADICAL,
    chinesePersonRadicalOutline,
  );

  it("shows the left-falling stroke before restarting for the vertical", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left-falling piě stroke from upper right to lower left",
      "lift, then draw the vertical shù stroke from the junction to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans SC radical with the falling stroke settled behind the vertical", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chinesePersonRadicalOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(CHINESE_PERSON_RADICAL.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_PERSON_RADICAL.strokes[1], 1));
  });
});

describe("Chinese 口 — a cited three-run box that closes last", () => {
  const steps = ductusSteps(CHINESE_MOUTH);
  const strip = ductusFilmstrip(CHINESE_MOUTH, chineseMouthOutline);

  it("shows the joined top-right corner before the separately closing bottom", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left vertical shù stroke from top to bottom",
      "lift, then draw the top bar from left to right",
      "turn the corner without lifting and descend the right side",
      "lift, then close the bottom from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC box with the first two runs behind the closing bottom", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseMouthOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_MOUTH.strokes[0], 1),
      penPathD(CHINESE_MOUTH.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_MOUTH.strokes[2], 1));
  });
});

describe("Chinese 女 — a cited bent first run followed by two lifted strokes", () => {
  const steps = ductusSteps(CHINESE_WOMAN);
  const strip = ductusFilmstrip(CHINESE_WOMAN, chineseWomanOutline);

  it("keeps the first bend joined before the falling and horizontal strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the first piědiǎn stroke down and left",
      "turn without lifting and sweep down to the lower right",
      "lift, then draw the left-falling piě stroke from upper right to lower left",
      "lift, then draw the middle horizontal héng from left to right",
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

  it("draws the exact Noto Sans SC glyph with both earlier runs behind the middle horizontal", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseWomanOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_WOMAN.strokes[0], 1),
      penPathD(CHINESE_WOMAN.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_WOMAN.strokes[2], 1));
  });
});

describe("Chinese 子 — two cited joined turns followed by a final horizontal", () => {
  const steps = ductusSteps(CHINESE_CHILD);
  const strip = ductusFilmstrip(CHINESE_CHILD, chineseChildOutline);

  it("keeps each turn joined inside its stroke before the final horizontal", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top horizontal héng from left to right",
      "turn without lifting and sweep down-left",
      "lift, then descend the central vertical",
      "hook left at the base without lifting",
      "lift, then draw the middle horizontal héng from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 2]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC glyph with both hooked runs behind the final horizontal", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseChildOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_CHILD.strokes[0], 1),
      penPathD(CHINESE_CHILD.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_CHILD.strokes[2], 1));
  });
});

describe("Chinese 日 — a cited joined corner with an inside-before-close order", () => {
  const steps = ductusSteps(CHINESE_SUN);
  const strip = ductusFilmstrip(CHINESE_SUN, chineseSunOutline);

  it("draws the left side, joined top-right corner, inside bar, then closing bottom", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left vertical shù from top to bottom",
      "lift, then draw the top horizontal héng from left to right",
      "turn without lifting and descend the right side",
      "lift, then draw the middle horizontal héng from left to right",
      "lift, then close the bottom horizontal héng from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC glyph with the inside bar behind the closing bottom", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseSunOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_SUN.strokes[0], 1),
      penPathD(CHINESE_SUN.strokes[1], 1),
      penPathD(CHINESE_SUN.strokes[2], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_SUN.strokes[3], 1));
  });
});

describe("Chinese 讠 — a cited dot followed by one double-turning stroke", () => {
  const steps = ductusSteps(CHINESE_SPEECH_RADICAL);
  const strip = ductusFilmstrip(
    CHINESE_SPEECH_RADICAL,
    chineseSpeechRadicalOutline,
  );

  it("keeps the horizontal, descent, and rising finish joined after the dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top dot down and right",
      "lift, then draw the short horizontal from left to right",
      "turn without lifting and descend the vertical",
      "turn without lifting and rise to the upper right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with the completed dot behind the joined body", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseSpeechRadicalOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(CHINESE_SPEECH_RADICAL.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_SPEECH_RADICAL.strokes[1], 1));
  });
});

describe("Chinese 氵 — two falling dots above one rising bottom stroke", () => {
  const steps = ductusSteps(CHINESE_WATER_RADICAL);
  const strip = ductusFilmstrip(
    CHINESE_WATER_RADICAL,
    chineseWaterRadicalOutline,
  );

  it("keeps all three sourced strokes separate while joining the final rise", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upper dot down and right",
      "lift, then draw the middle dot down and right",
      "lift, then begin the bottom stroke with a slight rise left",
      "continue without lifting in a long rise to the upper right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with both completed dots behind the rising stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseWaterRadicalOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_WATER_RADICAL.strokes[0], 1),
      penPathD(CHINESE_WATER_RADICAL.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_WATER_RADICAL.strokes[2], 1));
  });
});

describe("Chinese 宀 — two separate marks before a joined roof hook", () => {
  const steps = ductusSteps(CHINESE_ROOF_RADICAL);
  const strip = ductusFilmstrip(
    CHINESE_ROOF_RADICAL,
    chineseRoofRadicalOutline,
  );

  it("keeps the horizontal and down-left hook joined after two lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top dot down and right",
      "lift, then draw the left-side stroke down and left",
      "lift, then draw the horizontal roof from left to right",
      "hook down and left without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with both completed marks behind the roof hook", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseRoofRadicalOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_ROOF_RADICAL.strokes[0], 1),
      penPathD(CHINESE_ROOF_RADICAL.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_ROOF_RADICAL.strokes[2], 1));
  });
});

describe("Chinese 你 — seven cited strokes with two joined hooks", () => {
  const steps = ductusSteps(CHINESE_YOU);
  const strip = ductusFilmstrip(CHINESE_YOU, chineseYouOutline);

  it("writes 亻 first, keeps both hooks joined, and places both dots last", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 3, 4, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      false,
      true,
      false,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character with six completed strokes behind the final dot", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseYouOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual(
      CHINESE_YOU.strokes.slice(0, 6).map((stroke) => penPathD(stroke, 1)),
    );
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_YOU.strokes[6], 1));
  });
});

describe("Chinese 好 — six cited strokes with 女 before 子", () => {
  const steps = ductusSteps(CHINESE_GOOD);
  const strip = ductusFilmstrip(CHINESE_GOOD, chineseGoodOutline);

  it("keeps all three internal turns joined across six component-ordered strokes", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 2, 3, 3, 4, 4, 5,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
      false,
      true,
      false,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character with five completed strokes behind the final bar", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseGoodOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual(
      CHINESE_GOOD.strokes.slice(0, 5).map((stroke) => penPathD(stroke, 1)),
    );
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_GOOD.strokes[5], 1));
  });
});

describe("Chinese 我 — seven cited strokes with one joined hook", () => {
  const steps = ductusSteps(CHINESE_I);
  const strip = ductusFilmstrip(CHINESE_I, chineseIOutline);

  it("preserves seven strokes, one internal join, and six lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 2, 3, 4, 4, 5, 6,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseIOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_I.strokes[6], 1));
  });
});

describe("Chinese 是 — nine cited strokes with 日 first", () => {
  const steps = ductusSteps(CHINESE_BE);
  const strip = ductusFilmstrip(CHINESE_BE, chineseBeOutline);

  it("closes 日 before the lower body and preserves eight lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the final sweep", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseBeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_BE.strokes[8], 1));
  });
});

describe("Chinese 不 — four separately placed cited strokes", () => {
  const steps = ductusSteps(CHINESE_NOT);
  const strip = ductusFilmstrip(CHINESE_NOT, chineseNotOutline);

  it("keeps all four source strokes separate with three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseNotOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_NOT.strokes[3], 1));
  });
});

describe("Chinese 名 — 夕 before 口 in six cited strokes", () => {
  const steps = ductusSteps(CHINESE_NAME);
  const strip = ductusFilmstrip(CHINESE_NAME, chineseNameOutline);

  it("preserves both joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 2, 3, 4, 4, 5,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind 口's closing stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseNameOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_NAME.strokes[5], 1));
  });
});

describe("Chinese 字 — 宀 before 子 in six cited strokes", () => {
  const steps = ductusSteps(CHINESE_CHARACTER);
  const strip = ductusFilmstrip(CHINESE_CHARACTER, chineseCharacterOutline);

  it("preserves all three joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 2, 3, 3, 4, 4, 5,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind 子's final horizontal", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseCharacterOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_CHARACTER.strokes[5], 1));
  });
});

describe("Chinese 谢 — 讠 before 身 before 寸 in twelve cited strokes", () => {
  const steps = ductusSteps(CHINESE_THANK);
  const strip = ductusFilmstrip(CHINESE_THANK, chineseThankOutline);

  it("preserves all five joined turns and eleven lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 1, 2, 3, 4, 4, 4, 5, 6, 7, 8, 9, 10, 10, 11,
    ]);
    expect(strip.frames).toHaveLength(17);
    expect(strip.penLifts).toBe(11);
    expect(strip.summary).toBe("12 strokes · 11 pen lifts · 17 movements");
  });

  it("draws the exact Noto Sans SC character behind 寸's final dot", () => {
    const paths = byTag(strip.frames[16], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseThankOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_THANK.strokes[11], 1));
  });
});

describe("Chinese 请 — 讠 before 青 in ten cited strokes", () => {
  const steps = ductusSteps(CHINESE_PLEASE);
  const strip = ductusFilmstrip(CHINESE_PLEASE, chinesePleaseOutline);

  it("preserves all four joined turns and nine lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 1, 2, 3, 4, 5, 6, 7, 7, 7, 8, 9,
    ]);
    expect(strip.frames).toHaveLength(14);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 14 movements");
  });

  it("draws the exact Noto Sans SC character behind 青's final inner horizontal", () => {
    const paths = byTag(strip.frames[13], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chinesePleaseOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_PLEASE.strokes[9], 1));
  });
});

describe("Chinese 再 — central frame before the closing bottom bar", () => {
  const steps = ductusSteps(CHINESE_AGAIN);
  const strip = ductusFilmstrip(CHINESE_AGAIN, chineseAgainOutline);

  it("preserves both joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 2, 2, 3, 4, 5,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the closing horizontal", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseAgainOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_AGAIN.strokes[5], 1));
  });
});

describe("Chinese 见 — open upper frame before the two lower runs", () => {
  const steps = ductusSteps(CHINESE_SEE);
  const strip = ductusFilmstrip(CHINESE_SEE, chineseSeeOutline);

  it("preserves all three joined turns and three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 2, 3, 3, 3,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the hooked second leg", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseSeeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_SEE.strokes[3], 1));
  });
});

describe("Chinese 什 — complete 亻 before writing 十", () => {
  const steps = ductusSteps(CHINESE_WHAT);
  const strip = ductusFilmstrip(CHINESE_WHAT, chineseWhatOutline);

  it("shows four separate source strokes with three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind 十's final vertical", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseWhatOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_WHAT.strokes[3], 1));
  });
});

describe("Chinese 么 — joined second fall and rightward base sweep", () => {
  const steps = ductusSteps(CHINESE_PARTICLE_ME);
  const strip = ductusFilmstrip(CHINESE_PARTICLE_ME, chineseParticleMeOutline);

  it("preserves the joined turn and two lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseParticleMeOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_PARTICLE_ME.strokes[2], 1));
  });
});

describe("Chinese 早 — complete 日 before writing 十 below", () => {
  const steps = ductusSteps(CHINESE_EARLY);
  const strip = ductusFilmstrip(CHINESE_EARLY, chineseEarlyOutline);

  it("preserves the joined top-right corner and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 2, 3, 4, 5,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the final vertical", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseEarlyOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_EARLY.strokes[5], 1));
  });
});

describe("Chinese 上 — vertical before short and long horizontals", () => {
  const steps = ductusSteps(CHINESE_UP);
  const strip = ductusFilmstrip(CHINESE_UP, chineseUpOutline);

  it("preserves three separate sourced strokes and two lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans SC character behind the long base", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(chineseUpOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(CHINESE_UP.strokes[2], 1));
  });
});
