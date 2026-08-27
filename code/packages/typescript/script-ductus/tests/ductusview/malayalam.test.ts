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

const MALAYALAM_E = DUCTUS[ductusKey("malayalam", "എ")];
const malayalamEOutline = malayalamOutline("എ");
const MALAYALAM_A = DUCTUS[ductusKey("malayalam", "അ")];
const malayalamAOutline = malayalamOutline("അ");
const MALAYALAM_AA = DUCTUS[ductusKey("malayalam", "ആ")];
const malayalamAaOutline = malayalamOutline("ആ");
const MALAYALAM_I = DUCTUS[ductusKey("malayalam", "ഇ")];
const malayalamIOutline = malayalamOutline("ഇ");
const MALAYALAM_U = DUCTUS[ductusKey("malayalam", "ഉ")];
const malayalamUOutline = malayalamOutline("ഉ");
const MALAYALAM_CHILLU_L = DUCTUS[ductusKey("malayalam", "ൽ")];
const malayalamChilluLOutline = malayalamOutline("ൽ");
const MALAYALAM_CHILLU_N = DUCTUS[ductusKey("malayalam", "ൻ")];
const malayalamChilluNOutline = malayalamOutline("ൻ");
const MALAYALAM_CHILLU_LL = DUCTUS[ductusKey("malayalam", "ൾ")];
const malayalamChilluLLOutline = malayalamOutline("ൾ");
const MALAYALAM_CHILLU_RR = DUCTUS[ductusKey("malayalam", "ർ")];
const malayalamChilluRROutline = malayalamOutline("ർ");
const MALAYALAM_ZHA = DUCTUS[ductusKey("malayalam", "ഴ")];
const malayalamZhaOutline = malayalamOutline("ഴ");

describe("Malayalam എ — joined body, then a separate broad outer arch", () => {
  const steps = ductusSteps(MALAYALAM_E);
  const strip = ductusFilmstrip(MALAYALAM_E, malayalamEOutline);

  it("places one lift before the broad outer arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
  });

  it("reports three movements in two strokes", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("keeps the joined body visible while the outer arch completes", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(MALAYALAM_E.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_E.strokes[1], 1));
  });
});

describe("Malayalam അ — joined left body, then joined right arch and loop", () => {
  const steps = ductusSteps(MALAYALAM_A);
  const strip = ductusFilmstrip(MALAYALAM_A, malayalamAOutline);

  it("places one lift before the right outer arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
  });

  it("reports five movements in two strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the left body visible while the right inner loop completes", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(MALAYALAM_A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_A.strokes[1], 1));
  });
});

describe("Malayalam ആ — standalone left arch, then one long body run", () => {
  const steps = ductusSteps(MALAYALAM_AA);
  const strip = ductusFilmstrip(MALAYALAM_AA, malayalamAaOutline);

  it("places the sole lift before the inner curl", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 1]);
  });

  it("reports five movements across two strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Malayalam ഇ — one expanding spiral-to-baseline run", () => {
  const steps = ductusSteps(MALAYALAM_I);
  const strip = ductusFilmstrip(MALAYALAM_I, malayalamIOutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Malayalam ഉ — one broad spiral-to-baseline run", () => {
  const steps = ductusSteps(MALAYALAM_U);
  const strip = ductusFilmstrip(MALAYALAM_U, malayalamUOutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });
});

describe("Malayalam ൽ — one joined run through both loops and the chillu hook", () => {
  const steps = ductusSteps(MALAYALAM_CHILLU_L);
  const strip = ductusFilmstrip(MALAYALAM_CHILLU_L, malayalamChilluLOutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
  });

  it("reports five movements in one stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("finishes the one-run path at the above-line hook", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_CHILLU_L.strokes[0], 1));
  });
});

describe("Malayalam ൻ — left body, then lifted right loop and chillu hook", () => {
  const steps = ductusSteps(MALAYALAM_CHILLU_N);
  const strip = ductusFilmstrip(MALAYALAM_CHILLU_N, malayalamChilluNOutline);

  it("places one lift before the right-side run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
  });

  it("reports four movements in two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the left body visible while the right-side hook completes", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(MALAYALAM_CHILLU_N.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_CHILLU_N.strokes[1], 1));
  });
});

describe("Malayalam ൾ — one joined run through both bowls and the chillu hook", () => {
  const steps = ductusSteps(MALAYALAM_CHILLU_LL);
  const strip = ductusFilmstrip(MALAYALAM_CHILLU_LL, malayalamChilluLLOutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("finishes the one-run path at the above-line hook", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_CHILLU_LL.strokes[0], 1));
  });
});

describe("Malayalam ർ — one joined run through the arch, loop, and chillu hook", () => {
  const steps = ductusSteps(MALAYALAM_CHILLU_RR);
  const strip = ductusFilmstrip(MALAYALAM_CHILLU_RR, malayalamChilluRROutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("finishes the one-run path at the above-line hook", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_CHILLU_RR.strokes[0], 1));
  });
});

describe("Malayalam ഴ — one joined run through the left arch, right loop, and lower hook", () => {
  const steps = ductusSteps(MALAYALAM_ZHA);
  const strip = ductusFilmstrip(MALAYALAM_ZHA, malayalamZhaOutline);

  it("keeps every movement in stroke zero without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
  });

  it("reports three movements in one stroke", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("finishes the one-run path at the lower hook", () => {
    const last = strip.frames[2];
    const done = byTag(last, "path").filter(
      (path) => path.attrs.class === "ductus__done",
    );
    const pen = byTag(last, "path").find(
      (path) => path.attrs.class === "ductus__pen",
    )!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(MALAYALAM_ZHA.strokes[0], 1));
  });
});
