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

const JAPANESE_SHI = DUCTUS[ductusKey("japanese", "し")];
const japaneseShiOutline = japaneseOutline("し");
const JAPANESE_KU = DUCTUS[ductusKey("japanese", "く")];
const japaneseKuOutline = japaneseOutline("く");
const JAPANESE_TA = DUCTUS[ductusKey("japanese", "た")];
const japaneseTaOutline = japaneseOutline("た");
const JAPANESE_NE = DUCTUS[ductusKey("japanese", "ね")];
const japaneseNeOutline = japaneseOutline("ね");
const JAPANESE_MI = DUCTUS[ductusKey("japanese", "み")];
const japaneseMiOutline = japaneseOutline("み");
const JAPANESE_SE = DUCTUS[ductusKey("japanese", "せ")];
const japaneseSeOutline = japaneseOutline("せ");
const JAPANESE_TE = DUCTUS[ductusKey("japanese", "て")];
const japaneseTeOutline = japaneseOutline("て");
const JAPANESE_NA = DUCTUS[ductusKey("japanese", "な")];
const japaneseNaOutline = japaneseOutline("な");
const JAPANESE_MO = DUCTUS[ductusKey("japanese", "も")];
const japaneseMoOutline = japaneseOutline("も");
const JAPANESE_WA = DUCTUS[ductusKey("japanese", "わ")];
const japaneseWaOutline = japaneseOutline("わ");
const JAPANESE_YU = DUCTUS[ductusKey("japanese", "ゆ")];
const japaneseYuOutline = japaneseOutline("ゆ");
const JAPANESE_YO = DUCTUS[ductusKey("japanese", "よ")];
const japaneseYoOutline = japaneseOutline("よ");

describe("し — one continuous descending curve", () => {
  const steps = ductusSteps(JAPANESE_SHI);
  const strip = ductusFilmstrip(JAPANESE_SHI, japaneseShiOutline);

  it("keeps both movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
  });

  it("reports a two-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });
});

describe("く — one continuous angled turn", () => {
  const steps = ductusSteps(JAPANESE_KU);
  const strip = ductusFilmstrip(JAPANESE_KU, japaneseKuOutline);

  it("keeps both movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
  });

  it("reports a two-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });
});

describe("た — four separate source-verified runs", () => {
  const steps = ductusSteps(JAPANESE_TA);
  const strip = ductusFilmstrip(JAPANESE_TA, japaneseTaOutline);

  it("starts a new pen-down run after each lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
  });

  it("reports a four-frame three-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("keeps all earlier runs visible while drawing the lower bowl", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_TA.strokes[0], 1),
      penPathD(JAPANESE_TA.strokes[1], 1),
      penPathD(JAPANESE_TA.strokes[2], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_TA.strokes[3], 1));
  });
});

describe("ね — a vertical followed by one continuous hooked loop", () => {
  const steps = ductusSteps(JAPANESE_NE);
  const strip = ductusFilmstrip(JAPANESE_NE, japaneseNeOutline);

  it("lifts once before the three-movement hooked body", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
    ]);
  });

  it("reports a four-frame one-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the completed vertical visible while the loop finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_NE.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_NE.strokes[1], 1));
  });
});

describe("み — a loop followed by a lifted high-right sweep", () => {
  const steps = ductusSteps(JAPANESE_MI);
  const strip = ductusFilmstrip(JAPANESE_MI, japaneseMiOutline);

  it("keeps three movements joined before lifting for the final two", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports a five-frame one-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed loop visible while the high-right sweep finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_MI.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_MI.strokes[1], 1));
  });
});

describe("せ — a horizontal followed by two lifted crossing stems", () => {
  const steps = ductusSteps(JAPANESE_SE);
  const strip = ductusFilmstrip(JAPANESE_SE, japaneseSeOutline);

  it("lifts before each stem while keeping each stem's turn joined", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      true,
      false,
    ]);
  });

  it("reports a five-frame two-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps the horizontal and left stem visible while the right hook finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_SE.strokes[0], 1),
      penPathD(JAPANESE_SE.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_SE.strokes[2], 1));
  });
});

describe("て — one high bar returning through a broad lower curve", () => {
  const steps = ductusSteps(JAPANESE_TE);
  const strip = ductusFilmstrip(JAPANESE_TE, japaneseTeOutline);

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

  it("finishes with the whole continuous path as the active pen", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toEqual([]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_TE.strokes[0], 1));
  });
});

describe("な — three lifted marks followed by a looping body", () => {
  const steps = ductusSteps(JAPANESE_NA);
  const strip = ductusFilmstrip(JAPANESE_NA, japaneseNaOutline);

  it("lifts for the first three marks and keeps the final loop joined", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
      false,
    ]);
  });

  it("reports a five-frame three-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("keeps the three completed marks visible while the loop finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_NA.strokes[0], 1),
      penPathD(JAPANESE_NA.strokes[1], 1),
      penPathD(JAPANESE_NA.strokes[2], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_NA.strokes[3], 1));
  });
});

describe("も — a bowl followed by two lifted horizontals", () => {
  const steps = ductusSteps(JAPANESE_MO);
  const strip = ductusFilmstrip(JAPANESE_MO, japaneseMoOutline);

  it("starts a new pen-down run for each horizontal", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
  });

  it("reports a three-frame two-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("keeps both completed runs visible behind the final bar", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_MO.strokes[0], 1),
      penPathD(JAPANESE_MO.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_MO.strokes[2], 1));
  });
});

describe("わ — a vertical followed by one continuous hooked loop", () => {
  const steps = ductusSteps(JAPANESE_WA);
  const strip = ductusFilmstrip(JAPANESE_WA, japaneseWaOutline);

  it("lifts once before the three-movement hooked body", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
    ]);
  });

  it("reports a four-frame one-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the completed vertical visible while the broad loop finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_WA.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_WA.strokes[1], 1));
  });
});

describe("ゆ — a broad loop followed by a central descending curve", () => {
  const steps = ductusSteps(JAPANESE_YU);
  const strip = ductusFilmstrip(JAPANESE_YU, japaneseYuOutline);

  it("keeps the three loop movements joined before lifting for the central curve", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports a five-frame one-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed loop visible while the central curve finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_YU.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_YU.strokes[1], 1));
  });
});

describe("よ — a short bar followed by one looping stem", () => {
  const steps = ductusSteps(JAPANESE_YO);
  const strip = ductusFilmstrip(JAPANESE_YO, japaneseYoOutline);

  it("lifts once before the two-movement stem and loop", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
  });

  it("reports a three-frame one-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("keeps the upper bar visible while the looping stem finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter((node) => node.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((node) => node.attrs.class === "ductus__pen")!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(JAPANESE_YO.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(JAPANESE_YO.strokes[1], 1));
  });
});
