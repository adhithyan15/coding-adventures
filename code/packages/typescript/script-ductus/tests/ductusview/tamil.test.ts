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

const A = DUCTUS["அ"];
const aOutline = tamilOutline("அ");
const AA = DUCTUS["ஆ"];
const aaOutline = tamilOutline("ஆ");
const I = DUCTUS["இ"];
const iOutline = tamilOutline("இ");
const TAMIL_U = DUCTUS["உ"];
const tamilUOutline = tamilOutline("உ");
const TAMIL_UU = DUCTUS["ஊ"];
const tamilUuOutline = tamilOutline("ஊ");
const TAMIL_O = DUCTUS["ஒ"];
const tamilOOutline = tamilOutline("ஒ");
const TAMIL_E = DUCTUS["எ"];
const tamilEOutline = tamilOutline("எ");
const TAMIL_ZHA = DUCTUS["ழ"];
const tamilZhaOutline = tamilOutline("ழ");

const KA = DUCTUS["க"];
const kaOutline = tamilOutline("க");
const NGA = DUCTUS["ங"];
const ngaOutline = tamilOutline("ங");
const NYA = DUCTUS["ஞ"];
const nyaOutline = tamilOutline("ஞ");

const CA = DUCTUS["ச"];
const caOutline = tamilOutline("ச");
const TTA = DUCTUS["ட"];
const ttaOutline = tamilOutline("ட");
const THA = DUCTUS["த"];
const thaOutline = tamilOutline("த");
const VA = DUCTUS["வ"];
const vaOutline = tamilOutline("வ");
const LA = DUCTUS["ல"];
const laOutline = tamilOutline("ல");
const RRA = DUCTUS["ற"];
const rraOutline = tamilOutline("ற");
const NNA = DUCTUS["ன"];
const nnaOutline = tamilOutline("ன");
const RETROFLEX_NNA = DUCTUS["ண"];
const retroflexNnaOutline = tamilOutline("ண");
const DENTAL_NA = DUCTUS["ந"];
const dentalNaOutline = tamilOutline("ந");

beforeAll(() => {
  expect(ductusFor("ம")?.glyph).toBe("ம");
  expect(ductusFor("அ")?.glyph).toBe("அ");
  expect(ductusFor("ஆ")?.glyph).toBe("ஆ");
  expect(ductusFor("இ")?.glyph).toBe("இ");
  expect(ductusFor("ஊ")?.glyph).toBe("ஊ");
  expect(ductusFor("ஒ")?.glyph).toBe("ஒ");
  expect(ductusFor("எ")?.glyph).toBe("எ");
  expect(ductusFor("க")?.glyph).toBe("க");
  expect(ductusFor("ங")?.glyph).toBe("ங");
  expect(ductusFor("ஞ")?.glyph).toBe("ஞ");
  expect(ductusFor("ச")?.glyph).toBe("ச");
  expect(ductusFor("ட")?.glyph).toBe("ட");
  expect(ductusFor("வ")?.glyph).toBe("வ");
  expect(ductusFor("ல")?.glyph).toBe("ல");
  expect(ductusFor("ள")?.glyph).toBe("ள");
  expect(ductusFor("ழ")?.glyph).toBe("ழ");
  expect(ductusFor("ற")?.glyph).toBe("ற");
  expect(ductusFor("ன")?.glyph).toBe("ன");
  expect(ductusFor("ண")?.glyph).toBe("ண");
  expect(ductusFor("ந")?.glyph).toBe("ந");
  expect(ductusFor("ப")?.glyph).toBe("ப");
  expect(ductusFor("த")?.glyph).toBe("த");
  expect(ductusFor("ர")?.glyph).toBe("ர");
  expect(ductusFor("ய")?.glyph).toBe("ய");
});

describe("அ — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(A);
  const strip = ductusFilmstrip(A, aOutline);

  it("places the only pen lift before the separate right upright", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
  });

  it("reports the source-backed movement, stroke, and lift counts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed body visible while drawing the upright", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(A.strokes[1], 1));
  });
});

describe("Tamil எ — six joined body movements, then the right upright", () => {
  const steps = ductusSteps(TAMIL_E);
  const strip = ductusFilmstrip(TAMIL_E, tamilEOutline);

  it("places the only lift before movement 7", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 0, 1,
    ]);
  });

  it("reports seven movements in two strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the joined body visible while the upright rises", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(TAMIL_E.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(TAMIL_E.strokes[1], 1));
  });
});

describe("Tamil உ — one joined spiral, outer descent, and baseline", () => {
  const steps = ductusSteps(TAMIL_U);
  const strip = ductusFilmstrip(TAMIL_U, tamilUOutline);

  it("keeps every Frame 16 movement in stroke zero", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("finishes by carrying the baseline to the right", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TAMIL_U.strokes[0], 1));
  });
});

describe("Tamil ஊ — familiar உ followed by the three-run ள overlay", () => {
  const steps = ductusSteps(TAMIL_UU);
  const strip = ductusFilmstrip(TAMIL_UU, tamilUuOutline);

  it("places three lifts only after உ and between ள's familiar runs", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 1, 1, 1, 2, 2, 3,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports nine movements in four strokes", () => {
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 9 movements");
    expect(TAMIL_UU.source.url).toContain("frame-17");
  });
});

describe("Tamil ஒ — joined upper loops followed by the lower bowl", () => {
  const steps = ductusSteps(TAMIL_O);
  const strip = ductusFilmstrip(TAMIL_O, tamilOOutline);

  it("places one lift before the lower bowl", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
  });

  it("reports three movements in two strokes", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });
});

describe("Tamil ழ — joined left body, joined right bowl, then lower hook", () => {
  const steps = ductusSteps(TAMIL_ZHA);
  const strip = ductusFilmstrip(TAMIL_ZHA, tamilZhaOutline);

  it("places lifts before movements 4 and 6", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both body runs visible while the detached hook completes", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done[0].attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[0], 1));
    expect(done[1].attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[1], 1));
    expect(pen.attrs.d).toBe(penPathD(TAMIL_ZHA.strokes[2], 1));
  });
});

describe("ஆ — the upright and long-vowel loop stay connected", () => {
  const steps = ductusSteps(AA);
  const strip = ductusFilmstrip(AA, aaOutline);

  it("places one lift before the upright and none before its loop", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("finishes the connected upright-and-loop stroke in the last frame", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(AA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(AA.strokes[1], 1));
  });
});

describe("இ — a real cited seven-movement filmstrip", () => {
  const steps = ductusSteps(I);
  const strip = ductusFilmstrip(I, iOutline);

  it("places one lift before the outer climb and joins that climb to the arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 1, 1,
    ]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("finishes the joined outer climb-and-arch stroke in the last frame", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(I.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(I.strokes[1], 1));
  });
});

describe("க — a real cited three-stroke filmstrip", () => {
  const steps = ductusSteps(KA);
  const strip = ductusFilmstrip(KA, kaOutline);

  it("places lifts before each lower bowl", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible while drawing the right bowl", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(KA.strokes[0], 1),
      penPathD(KA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(KA.strokes[2], 1));
  });
});

describe("ங — a detached upright followed by one joined body", () => {
  const steps = ductusSteps(NGA);
  const strip = ductusFilmstrip(NGA, ngaOutline);

  it("keeps the five body movements joined after one lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports six movements in two strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed upright visible while the body finishes", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(NGA.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(NGA.strokes[1], 1));
  });
});

describe("ஞ — four source-verified runs across eight movements", () => {
  const steps = ductusSteps(NYA);
  const strip = ductusFilmstrip(NYA, nyaOutline);

  it("places lifts before the top bar, central descent, and outer bowl", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 2, 2, 3, 3, 3,
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      false,
      true,
      false,
      false,
    ]);
  });

  it("reports eight movements in four strokes", () => {
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 8 movements");
    expect(NYA.source.citation).toMatch(/Frame 8.*ஞ.*p\. 194/i);
  });
});

describe("ச — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(CA);
  const strip = ductusFilmstrip(CA, caOutline);

  it("has one frame per named movement", () => {
    expect(steps).toHaveLength(4);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("advances through the joined upper frame before the lifted bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the left upright",
      "carry the top bar to the right",
      "drop the inner upright and carry right",
      "turn around and close the lower-left bowl",
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
  });

  it("renders the completed upper frame behind the active bowl", () => {
    const final = strip.frames.at(-1)!;
    const done = byTag(final, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(final, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(CA.strokes[0], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(CA.strokes[1], 1));
  });
});

describe("ட — a real cited unbroken two-movement filmstrip", () => {
  const steps = ductusSteps(TTA);
  const strip = ductusFilmstrip(TTA, ttaOutline);

  it("keeps the cornering movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "down the left upright",
      "along the long rightward foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the sole stroke without a completed-stroke overlay", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(TTA.strokes[0], 1));
  });
});

describe("த — a real cited four-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(THA);
  const strip = ductusFilmstrip(THA, thaOutline);

  it("shows the three source-marked lifts between four runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 1, 1, 2, 2, 3,
    ]);
  });

  it("reports seven movements across four strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });

  it("finishes with three completed runs behind the leftward tail", () => {
    const last = strip.frames.at(-1)!;
    const done = byTag(last, "path").filter(
      (node) => node.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (node) => node.attrs.class === "ductus__pen",
    )!;
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(THA.strokes[0], 1),
      penPathD(THA.strokes[1], 1),
      penPathD(THA.strokes[2], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(THA.strokes[3], 1));
  });
});

describe("வ — a real cited unbroken five-movement filmstrip", () => {
  const steps = ductusSteps(VA);
  const strip = ductusFilmstrip(VA, vaOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
  });

  it("reports five movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(VA.strokes[0], 1));
  });
});

describe("ல — a real cited unbroken four-movement filmstrip", () => {
  const steps = ductusSteps(LA);
  const strip = ductusFilmstrip(LA, laOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(LA.strokes[0], 1));
  });
});

describe("ற — a real cited three-stroke five-movement filmstrip", () => {
  const steps = ductusSteps(RRA);
  const strip = ductusFilmstrip(RRA, rraOutline);

  it("marks exactly the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 2]);
  });

  it("reports five movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps both completed strokes visible while drawing the joined sweep", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(RRA.strokes[0], 1),
      penPathD(RRA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(RRA.strokes[2], 1));
  });
});

describe("ன — a real cited two-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(NNA);
  const strip = ductusFilmstrip(NNA, nnaOutline);

  it("joins the loop, inner arch, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed loop-and-bar stroke visible while drawing the upright", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(NNA.strokes[1], 1));
  });
});

describe("ண — a real cited two-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(RETROFLEX_NNA);
  const strip = ductusFilmstrip(RETROFLEX_NNA, retroflexNnaOutline);

  it("joins the loop, both inner arches, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 0, 0, 0, 0, 0, 1,
    ]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the completed double-arch stroke visible while drawing the upright", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[1], 1));
  });
});

describe("ந — a real cited three-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(DENTAL_NA);
  const strip = ductusFilmstrip(DENTAL_NA, dentalNaOutline);

  it("marks the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 2, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible during the right-bowl tail", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(DENTAL_NA.strokes[0], 1),
      penPathD(DENTAL_NA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(DENTAL_NA.strokes[2], 1));
  });
});
