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

// 尔 alone: five strokes, two of them hooked, so seven movements and four lifts.
describe("Chinese 尔 — five strokes, two hooks, four lifts", () => {
  const er = DUCTUS[ductusKey("chinese", "尔")];
  const steps = ductusSteps(er);
  const strip = ductusFilmstrip(er, chineseOutline("尔"));

  it("lifts before every stroke but the first, never inside a hook", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
      false,
      true,
      true,
    ]);
  });

  it("reports 7 movements across 5 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 7 movements");
  });
});

// 很 (chapter 30): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the long right-falling stroke.
describe("Chinese 很 — 彳 before 艮 in nine cited strokes", () => {
  const hen = ductusFor("很", "chinese")!;
  const henOutline = chineseOutline("很");
  const steps = ductusSteps(hen);
  const strip = ductusFilmstrip(hen, henOutline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the long right-falling stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(henOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(hen.strokes[8], 1));
  });
});

// 他 (chapter 31): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 他 — five cited strokes, four lifts", () => {
  const letter = ductusFor("他", "chinese")!;
  const outline = chineseOutline("他");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 们 (chapter 31): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 们 — five cited strokes, four lifts", () => {
  const letter = ductusFor("们", "chinese")!;
  const outline = chineseOutline("们");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 妈 (chapter 32): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 妈 — six cited strokes, five lifts", () => {
  const letter = ductusFor("妈", "chinese")!;
  const outline = chineseOutline("妈");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 男 (chapter 33): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 男 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("男", "chinese")!;
  const outline = chineseOutline("男");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 叫 (chapter 34): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 叫 — five cited strokes, four lifts", () => {
  const letter = ductusFor("叫", "chinese")!;
  const outline = chineseOutline("叫");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 来 (chapter 36): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 来 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("来", "chinese")!;
  const outline = chineseOutline("来");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 这 (chapter 37): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 这 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("这", "chinese")!;
  const outline = chineseOutline("这");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 那 (chapter 37): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 那 — six cited strokes, five lifts", () => {
  const letter = ductusFor("那", "chinese")!;
  const outline = chineseOutline("那");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 吃 (chapter 38): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 吃 — six cited strokes, five lifts", () => {
  const letter = ductusFor("吃", "chinese")!;
  const outline = chineseOutline("吃");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 饭 (chapter 38): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 饭 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("饭", "chinese")!;
  const outline = chineseOutline("饭");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 喝 (chapter 39): twelve strokes, every one lifted, and none of them turns in a
// second segment, so the strip is twelve movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 喝 — twelve cited strokes, eleven lifts", () => {
  const letter = ductusFor("喝", "chinese")!;
  const outline = chineseOutline("喝");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(12);
    expect(strip.penLifts).toBe(11);
    expect(strip.summary).toBe("12 strokes · 11 pen lifts · 12 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[11], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[11], 1));
  });
});

// 水 (chapter 39): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 水 — four cited strokes, three lifts", () => {
  const letter = ductusFor("水", "chinese")!;
  const outline = chineseOutline("水");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 买 (chapter 40): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 买 — six cited strokes, five lifts", () => {
  const letter = ductusFor("买", "chinese")!;
  const outline = chineseOutline("买");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 多 (chapter 41): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 多 — six cited strokes, five lifts", () => {
  const letter = ductusFor("多", "chinese")!;
  const outline = chineseOutline("多");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 少 (chapter 41): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 少 — four cited strokes, three lifts", () => {
  const letter = ductusFor("少", "chinese")!;
  const outline = chineseOutline("少");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 去 (chapter 43): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 去 — five cited strokes, four lifts", () => {
  const letter = ductusFor("去", "chinese")!;
  const outline = chineseOutline("去");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 车 (chapter 44): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 车 — four cited strokes, three lifts", () => {
  const letter = ductusFor("车", "chinese")!;
  const outline = chineseOutline("车");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 天 (chapter 45): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 天 — four cited strokes, three lifts", () => {
  const letter = ductusFor("天", "chinese")!;
  const outline = chineseOutline("天");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 今 (chapter 45): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 今 — four cited strokes, three lifts", () => {
  const letter = ductusFor("今", "chinese")!;
  const outline = chineseOutline("今");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 星 (chapter 46): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 星 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("星", "chinese")!;
  const outline = chineseOutline("星");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 期 (chapter 46): twelve strokes, every one lifted, and none of them turns in a
// second segment, so the strip is twelve movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 期 — twelve cited strokes, eleven lifts", () => {
  const letter = ductusFor("期", "chinese")!;
  const outline = chineseOutline("期");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(12);
    expect(strip.penLifts).toBe(11);
    expect(strip.summary).toBe("12 strokes · 11 pen lifts · 12 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[11], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[11], 1));
  });
});

// 年 (chapter 47): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 年 — six cited strokes, five lifts", () => {
  const letter = ductusFor("年", "chinese")!;
  const outline = chineseOutline("年");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 明 (chapter 47): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 明 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("明", "chinese")!;
  const outline = chineseOutline("明");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 点 (chapter 48): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 点 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("点", "chinese")!;
  const outline = chineseOutline("点");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 午 (chapter 49): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 午 — four cited strokes, three lifts", () => {
  const letter = ductusFor("午", "chinese")!;
  const outline = chineseOutline("午");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 晚 (chapter 49): eleven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 晚 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("晚", "chinese")!;
  const outline = chineseOutline("晚");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 手 (chapter 52): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 手 — four cited strokes, three lifts", () => {
  const letter = ductusFor("手", "chinese")!;
  const outline = chineseOutline("手");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 说 (chapter 53): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 说 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("说", "chinese")!;
  const outline = chineseOutline("说");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 话 (chapter 53): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 话 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("话", "chinese")!;
  const outline = chineseOutline("话");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 想 (chapter 55): thirteen strokes, every one lifted, and none of them turns in
// a second segment, so the strip is thirteen movements. The last frame must
// still draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 想 — thirteen cited strokes, twelve lifts", () => {
  const letter = ductusFor("想", "chinese")!;
  const outline = chineseOutline("想");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(13);
    expect(strip.penLifts).toBe(12);
    expect(strip.summary).toBe("13 strokes · 12 pen lifts · 13 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[12], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[12], 1));
  });
});

// 机 (chapter 58): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 机 — six cited strokes, five lifts", () => {
  const letter = ductusFor("机", "chinese")!;
  const outline = chineseOutline("机");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 白 (chapter 60): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 白 — five cited strokes, four lifts", () => {
  const letter = ductusFor("白", "chinese")!;
  const outline = chineseOutline("白");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 长 (chapter 61): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 长 — four cited strokes, three lifts", () => {
  const letter = ductusFor("长", "chinese")!;
  const outline = chineseOutline("长");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 快 (chapter 62): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 快 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("快", "chinese")!;
  const outline = chineseOutline("快");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 电 (chapter 63): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 电 — five cited strokes, four lifts", () => {
  const letter = ductusFor("电", "chinese")!;
  const outline = chineseOutline("电");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 打 (chapter 63): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 打 — five cited strokes, four lifts", () => {
  const letter = ductusFor("打", "chinese")!;
  const outline = chineseOutline("打");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 起 (chapter 64): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 起 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("起", "chinese")!;
  const outline = chineseOutline("起");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 门 (chapter 65): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 门 — three cited strokes, two lifts", () => {
  const letter = ductusFor("门", "chinese")!;
  const outline = chineseOutline("门");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 问 (chapter 65): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 问 — six cited strokes, five lifts", () => {
  const letter = ductusFor("问", "chinese")!;
  const outline = chineseOutline("问");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 气 (chapter 66): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 气 — four cited strokes, three lifts", () => {
  const letter = ductusFor("气", "chinese")!;
  const outline = chineseOutline("气");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 雨 (chapter 66): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 雨 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("雨", "chinese")!;
  const outline = chineseOutline("雨");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 马 (chapter 67): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 马 — three cited strokes, two lifts", () => {
  const letter = ductusFor("马", "chinese")!;
  const outline = chineseOutline("马");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 山 (chapter 68): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 山 — three cited strokes, two lifts", () => {
  const letter = ductusFor("山", "chinese")!;
  const outline = chineseOutline("山");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 做 (chapter 69): eleven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 做 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("做", "chinese")!;
  const outline = chineseOutline("做");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 火 (chapter 72): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 火 — four cited strokes, three lifts", () => {
  const letter = ductusFor("火", "chinese")!;
  const outline = chineseOutline("火");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 开 (chapter 72): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 开 — four cited strokes, three lifts", () => {
  const letter = ductusFor("开", "chinese")!;
  const outline = chineseOutline("开");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 下 (chapter 73): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 下 — three cited strokes, two lifts", () => {
  const letter = ductusFor("下", "chinese")!;
  const outline = chineseOutline("下");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 进 (chapter 74): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 进 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("进", "chinese")!;
  const outline = chineseOutline("进");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 客 (chapter 74): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 客 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("客", "chinese")!;
  const outline = chineseOutline("客");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 乐 (chapter 75): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 乐 — five cited strokes, four lifts", () => {
  const letter = ductusFor("乐", "chinese")!;
  const outline = chineseOutline("乐");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 会 (chapter 76): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 会 — six cited strokes, five lifts", () => {
  const letter = ductusFor("会", "chinese")!;
  const outline = chineseOutline("会");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 课 (chapter 79): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 课 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("课", "chinese")!;
  const outline = chineseOutline("课");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 里 (chapter 81): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 里 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("里", "chinese")!;
  const outline = chineseOutline("里");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 前 (chapter 82): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 前 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("前", "chinese")!;
  const outline = chineseOutline("前");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 在 (chapter 85): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 在 — six cited strokes, five lifts", () => {
  const letter = ductusFor("在", "chinese")!;
  const outline = chineseOutline("在");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 哪 (chapter 86): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 哪 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("哪", "chinese")!;
  const outline = chineseOutline("哪");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 没 (chapter 87): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 没 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("没", "chinese")!;
  const outline = chineseOutline("没");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 两 (chapter 88): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 两 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("两", "chinese")!;
  const outline = chineseOutline("两");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 给 (chapter 89): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 给 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("给", "chinese")!;
  const outline = chineseOutline("给");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 能 (chapter 90): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 能 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("能", "chinese")!;
  const outline = chineseOutline("能");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 要 (chapter 91): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 要 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("要", "chinese")!;
  const outline = chineseOutline("要");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 也 (chapter 92): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 也 — three cited strokes, two lifts", () => {
  const letter = ductusFor("也", "chinese")!;
  const outline = chineseOutline("也");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 太 (chapter 93): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 太 — four cited strokes, three lifts", () => {
  const letter = ductusFor("太", "chinese")!;
  const outline = chineseOutline("太");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 都 (chapter 94): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 都 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("都", "chinese")!;
  const outline = chineseOutline("都");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 为 (chapter 95): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 为 — four cited strokes, three lifts", () => {
  const letter = ductusFor("为", "chinese")!;
  const outline = chineseOutline("为");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 因 (chapter 96): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 因 — six cited strokes, five lifts", () => {
  const letter = ductusFor("因", "chinese")!;
  const outline = chineseOutline("因");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 时 (chapter 97): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 时 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("时", "chinese")!;
  const outline = chineseOutline("时");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 洗 (chapter 99): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 洗 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("洗", "chinese")!;
  const outline = chineseOutline("洗");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 菜 (chapter 101): eleven strokes, every one lifted, and none of them turns in
// a second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 菜 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("菜", "chinese")!;
  const outline = chineseOutline("菜");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 用 (chapter 103): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 用 — five cited strokes, four lifts", () => {
  const letter = ductusFor("用", "chinese")!;
  const outline = chineseOutline("用");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 本 (chapter 104): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 本 — five cited strokes, four lifts", () => {
  const letter = ductusFor("本", "chinese")!;
  const outline = chineseOutline("本");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 站 (chapter 106): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 站 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("站", "chinese")!;
  const outline = chineseOutline("站");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 试 (chapter 108): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 试 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("试", "chinese")!;
  const outline = chineseOutline("试");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 网 (chapter 109): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 网 — six cited strokes, five lifts", () => {
  const letter = ductusFor("网", "chinese")!;
  const outline = chineseOutline("网");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 外 (chapter 110): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 外 — five cited strokes, four lifts", () => {
  const letter = ductusFor("外", "chinese")!;
  const outline = chineseOutline("外");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 边 (chapter 111): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 边 — five cited strokes, four lifts", () => {
  const letter = ductusFor("边", "chinese")!;
  const outline = chineseOutline("边");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 出 (chapter 112): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 出 — five cited strokes, four lifts", () => {
  const letter = ductusFor("出", "chinese")!;
  const outline = chineseOutline("出");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 心 (chapter 115): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 心 — four cited strokes, three lifts", () => {
  const letter = ductusFor("心", "chinese")!;
  const outline = chineseOutline("心");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 包 (chapter 119): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 包 — five cited strokes, four lifts", () => {
  const letter = ductusFor("包", "chinese")!;
  const outline = chineseOutline("包");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 以 (chapter 126): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 以 — four cited strokes, three lifts", () => {
  const letter = ductusFor("以", "chinese")!;
  const outline = chineseOutline("以");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 些 (chapter 127): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 些 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("些", "chinese")!;
  const outline = chineseOutline("些");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 音 (chapter 128): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 音 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("音", "chinese")!;
  const outline = chineseOutline("音");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 吵 (chapter 129): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 吵 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("吵", "chinese")!;
  const outline = chineseOutline("吵");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 病 (chapter 130): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 病 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("病", "chinese")!;
  const outline = chineseOutline("病");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 春 (chapter 131): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 春 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("春", "chinese")!;
  const outline = chineseOutline("春");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 千 (chapter 132): three strokes, every one lifted, and none of them turns in a
// second segment, so the strip is three movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 千 — three cited strokes, two lifts", () => {
  const letter = ductusFor("千", "chinese")!;
  const outline = chineseOutline("千");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[2], 1));
  });
});

// 扫 (chapter 133): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 扫 — six cited strokes, five lifts", () => {
  const letter = ductusFor("扫", "chinese")!;
  const outline = chineseOutline("扫");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 重 (chapter 134): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 重 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("重", "chinese")!;
  const outline = chineseOutline("重");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 节 (chapter 137): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 节 — five cited strokes, four lifts", () => {
  const letter = ductusFor("节", "chinese")!;
  const outline = chineseOutline("节");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 每 (chapter 138): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 每 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("每", "chinese")!;
  const outline = chineseOutline("每");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 半 (chapter 140): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 半 — five cited strokes, four lifts", () => {
  const letter = ductusFor("半", "chinese")!;
  const outline = chineseOutline("半");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 后 (chapter 147): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 后 — six cited strokes, five lifts", () => {
  const letter = ductusFor("后", "chinese")!;
  const outline = chineseOutline("后");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 月 (chapter 148): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 月 — four cited strokes, three lifts", () => {
  const letter = ductusFor("月", "chinese")!;
  const outline = chineseOutline("月");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 正 (chapter 149): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 正 — five cited strokes, four lifts", () => {
  const letter = ductusFor("正", "chinese")!;
  const outline = chineseOutline("正");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 过 (chapter 150): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 过 — six cited strokes, five lifts", () => {
  const letter = ductusFor("过", "chinese")!;
  const outline = chineseOutline("过");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 到 (chapter 151): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 到 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("到", "chinese")!;
  const outline = chineseOutline("到");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 得 (chapter 152): eleven strokes, every one lifted, and none of them turns in
// a second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 得 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("得", "chinese")!;
  const outline = chineseOutline("得");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 比 (chapter 153): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 比 — four cited strokes, three lifts", () => {
  const letter = ductusFor("比", "chinese")!;
  const outline = chineseOutline("比");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 还 (chapter 154): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 还 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("还", "chinese")!;
  const outline = chineseOutline("还");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 从 (chapter 155): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 从 — four cited strokes, three lifts", () => {
  const letter = ductusFor("从", "chinese")!;
  const outline = chineseOutline("从");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 元 (chapter 156): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 元 — four cited strokes, three lifts", () => {
  const letter = ductusFor("元", "chinese")!;
  const outline = chineseOutline("元");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 房 (chapter 159): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 房 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("房", "chinese")!;
  const outline = chineseOutline("房");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 米 (chapter 161): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 米 — six cited strokes, five lifts", () => {
  const letter = ductusFor("米", "chinese")!;
  const outline = chineseOutline("米");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 衣 (chapter 165): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 衣 — six cited strokes, five lifts", () => {
  const letter = ductusFor("衣", "chinese")!;
  const outline = chineseOutline("衣");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 茶 (chapter 166): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 茶 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("茶", "chinese")!;
  const outline = chineseOutline("茶");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 事 (chapter 167): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 事 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("事", "chinese")!;
  const outline = chineseOutline("事");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 觉 (chapter 169): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 觉 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("觉", "chinese")!;
  const outline = chineseOutline("觉");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 杯 (chapter 170): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 杯 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("杯", "chinese")!;
  const outline = chineseOutline("杯");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 冷 (chapter 171): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 冷 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("冷", "chinese")!;
  const outline = chineseOutline("冷");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 候 (chapter 173): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 候 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("候", "chinese")!;
  const outline = chineseOutline("候");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 面 (chapter 175): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 面 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("面", "chinese")!;
  const outline = chineseOutline("面");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 常 (chapter 178): eleven strokes, every one lifted, and none of them turns in
// a second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 常 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("常", "chinese")!;
  const outline = chineseOutline("常");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 间 (chapter 181): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 间 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("间", "chinese")!;
  const outline = chineseOutline("间");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 钱 (chapter 188): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 钱 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("钱", "chinese")!;
  const outline = chineseOutline("钱");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 号 (chapter 189): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 号 — five cited strokes, four lifts", () => {
  const letter = ductusFor("号", "chinese")!;
  const outline = chineseOutline("号");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 店 (chapter 190): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 店 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("店", "chinese")!;
  const outline = chineseOutline("店");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});

// 回 (chapter 191): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 回 — six cited strokes, five lifts", () => {
  const letter = ductusFor("回", "chinese")!;
  const outline = chineseOutline("回");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 路 (chapter 194): thirteen strokes, every one lifted, and none of them turns
// in a second segment, so the strip is thirteen movements. The last frame must
// still draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 路 — thirteen cited strokes, twelve lifts", () => {
  const letter = ductusFor("路", "chinese")!;
  const outline = chineseOutline("路");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(13);
    expect(strip.penLifts).toBe(12);
    expect(strip.summary).toBe("13 strokes · 12 pen lifts · 13 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[12], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[12], 1));
  });
});

// 票 (chapter 196): eleven strokes, every one lifted, and none of them turns in
// a second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 票 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("票", "chinese")!;
  const outline = chineseOutline("票");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 找 (chapter 197): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 找 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("找", "chinese")!;
  const outline = chineseOutline("找");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 关 (chapter 198): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 关 — six cited strokes, five lifts", () => {
  const letter = ductusFor("关", "chinese")!;
  const outline = chineseOutline("关");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 东 (chapter 203): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 东 — five cited strokes, four lifts", () => {
  const letter = ductusFor("东", "chinese")!;
  const outline = chineseOutline("东");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 离 (chapter 205): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 离 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("离", "chinese")!;
  const outline = chineseOutline("离");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 公 (chapter 205): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 公 — four cited strokes, three lifts", () => {
  const letter = ductusFor("公", "chinese")!;
  const outline = chineseOutline("公");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 地 (chapter 207): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 地 — six cited strokes, five lifts", () => {
  const letter = ductusFor("地", "chinese")!;
  const outline = chineseOutline("地");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 风 (chapter 218): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 风 — four cited strokes, three lifts", () => {
  const letter = ductusFor("风", "chinese")!;
  const outline = chineseOutline("风");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 云 (chapter 219): four strokes, every one lifted, and none of them turns in a
// second segment, so the strip is four movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 云 — four cited strokes, three lifts", () => {
  const letter = ductusFor("云", "chinese")!;
  const outline = chineseOutline("云");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
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

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[3], 1));
  });
});

// 雪 (chapter 221): eleven strokes, every one lifted, and none of them turns in
// a second segment, so the strip is eleven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 雪 — eleven cited strokes, ten lifts", () => {
  const letter = ductusFor("雪", "chinese")!;
  const outline = chineseOutline("雪");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(11);
    expect(strip.penLifts).toBe(10);
    expect(strip.summary).toBe("11 strokes · 10 pen lifts · 11 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[10], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[10], 1));
  });
});

// 花 (chapter 224): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 花 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("花", "chinese")!;
  const outline = chineseOutline("花");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 草 (chapter 226): nine strokes, every one lifted, and none of them turns in a
// second segment, so the strip is nine movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 草 — nine cited strokes, eight lifts", () => {
  const letter = ductusFor("草", "chinese")!;
  const outline = chineseOutline("草");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[8], 1));
  });
});

// 头 (chapter 228): five strokes, every one lifted, and none of them turns in a
// second segment, so the strip is five movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 头 — five cited strokes, four lifts", () => {
  const letter = ductusFor("头", "chinese")!;
  const outline = chineseOutline("头");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[4], 1));
  });
});

// 疼 (chapter 228): ten strokes, every one lifted, and none of them turns in a
// second segment, so the strip is ten movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 疼 — ten cited strokes, nine lifts", () => {
  const letter = ductusFor("疼", "chinese")!;
  const outline = chineseOutline("疼");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[9], 1));
  });
});

// 身 (chapter 230): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 身 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("身", "chinese")!;
  const outline = chineseOutline("身");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 体 (chapter 230): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 体 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("体", "chinese")!;
  const outline = chineseOutline("体");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 医 (chapter 231): seven strokes, every one lifted, and none of them turns in a
// second segment, so the strip is seven movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 医 — seven cited strokes, six lifts", () => {
  const letter = ductusFor("医", "chinese")!;
  const outline = chineseOutline("医");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[6], 1));
  });
});

// 西 (chapter 231): six strokes, every one lifted, and none of them turns in a
// second segment, so the strip is six movements. The last frame must still draw
// the exact Noto Sans SC character behind the last stroke.
describe("Chinese 西 — six cited strokes, five lifts", () => {
  const letter = ductusFor("西", "chinese")!;
  const outline = chineseOutline("西");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[5], 1));
  });
});

// 现 (chapter 236): eight strokes, every one lifted, and none of them turns in a
// second segment, so the strip is eight movements. The last frame must still
// draw the exact Noto Sans SC character behind the last stroke.
describe("Chinese 现 — eight cited strokes, seven lifts", () => {
  const letter = ductusFor("现", "chinese")!;
  const outline = chineseOutline("现");
  const steps = ductusSteps(letter);
  const strip = ductusFilmstrip(letter, outline);

  it("lifts before every stroke but the first", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      true,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(7);
    expect(strip.summary).toBe("8 strokes · 7 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the last stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(outline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[7], 1));
  });
});
