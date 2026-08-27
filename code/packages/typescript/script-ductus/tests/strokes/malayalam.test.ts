import { beforeAll, describe, expect, it } from "vitest";
import { SCRIPTS, verifiedLetterFont } from "../../src/scriptdata";
import {
  DUCTUS,
  ductusFor,
  ductusKey,
  joinGaps,
  penLifts,
  penPath,
  penPathD,
  penTip,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";
import { registerStrokeHonestyTests } from "../support/stroke-honesty";

const MALAYALAM_A = DUCTUS[ductusKey("malayalam", "അ")];
const MALAYALAM_AA = DUCTUS[ductusKey("malayalam", "ആ")];
const MALAYALAM_I = DUCTUS[ductusKey("malayalam", "ഇ")];
const MALAYALAM_U = DUCTUS[ductusKey("malayalam", "ഉ")];
const MALAYALAM_E = DUCTUS[ductusKey("malayalam", "എ")];
const MALAYALAM_CHILLU_L = DUCTUS[ductusKey("malayalam", "ൽ")];
const MALAYALAM_CHILLU_N = DUCTUS[ductusKey("malayalam", "ൻ")];
const MALAYALAM_CHILLU_LL = DUCTUS[ductusKey("malayalam", "ൾ")];
const MALAYALAM_CHILLU_RR = DUCTUS[ductusKey("malayalam", "ർ")];
const MALAYALAM_ZHA = DUCTUS[ductusKey("malayalam", "ഴ")];

const OWNER_SCRIPTS = new Set(["malayalam"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { അ: 0.96, ആ: 0.89 });

  beforeAll(() => {
    expect(verifiedLetterFont("എ", MALAYALAM_E.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("അ", MALAYALAM_A.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ഇ", MALAYALAM_I.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ഉ", MALAYALAM_U.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൽ", MALAYALAM_CHILLU_L.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൻ", MALAYALAM_CHILLU_N.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൾ", MALAYALAM_CHILLU_LL.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ർ", MALAYALAM_CHILLU_RR.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ഴ", MALAYALAM_ZHA.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
  });

  it("Malayalam എ keeps its joined body separate from the broad outer arch", () => {
    expect(penLifts(MALAYALAM_E)).toBe(1);
    expect(MALAYALAM_E.strokes).toHaveLength(2);
    expect(
      MALAYALAM_E.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn around the compact left hook and carry the middle bar right",
        "climb the upright, retrace it downward, and loop below the line",
      ],
      ["sweep up and over through the broad outer arch, ending below the line"],
    ]);
  });

  it("Malayalam അ keeps both animated runs internally joined", () => {
    expect(penLifts(MALAYALAM_A)).toBe(1);
    expect(MALAYALAM_A.strokes).toHaveLength(2);
    expect(
      MALAYALAM_A.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the left outer arch and curve through the upper turn",
        "circle the broad lower loop and return to the junction",
        "sweep up through the central crown and descend the upright",
      ],
      [
        "sweep up and over through the right outer arch and descend its far side",
        "curl left around the lower inner loop",
      ],
    ]);
  });

  it("Malayalam ആ lifts once after the standalone left outer arch", () => {
    expect(penLifts(MALAYALAM_AA)).toBe(1);
    expect(
      MALAYALAM_AA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["climb the left outer arch and curve inward at the top"],
      [
        "turn inward around the compact inner curl and circle the broad lower loop",
        "sweep up through the central crown and descend the upright",
        "retrace the upright and sweep around the rounded right loop",
        "descend the far side and curl left below the line",
      ],
    ]);
    expect(MALAYALAM_AA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B4%86_order.gif",
    );
  });

  it("Malayalam ഇ keeps all four animated movements in one run", () => {
    expect(penLifts(MALAYALAM_I)).toBe(0);
    expect(MALAYALAM_I.strokes).toHaveLength(1);
    expect(
      MALAYALAM_I.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn outward around the compact left spiral and descend the central stem",
      "retrace the central stem and sweep around the broad right lobe",
      "curl left below the line",
      "carry the finishing baseline to the right",
    ]);
  });

  it("Malayalam ഉ keeps all three animated movements in one run", () => {
    expect(penLifts(MALAYALAM_U)).toBe(0);
    expect(MALAYALAM_U.strokes).toHaveLength(1);
    expect(
      MALAYALAM_U.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn outward around the compact left spiral and carry the upper arch right",
      "descend around the broad right lobe and curl left below the line",
      "carry the finishing baseline to the right",
    ]);
  });

  it("Malayalam chillu ൽ keeps all five animated movements in one run", () => {
    expect(penLifts(MALAYALAM_CHILLU_L)).toBe(0);
    expect(MALAYALAM_CHILLU_L.strokes).toHaveLength(1);
    expect(
      MALAYALAM_CHILLU_L.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "climb the left entry arch and turn inward at the top",
      "descend clockwise around the central loop and return to its upper junction",
      "carry the upper shoulder right",
      "sweep clockwise around the right loop and return to the upper crossing",
      "rise into the chillu hook and curl left above the line",
    ]);
  });

  it("Malayalam chillu ൻ keeps each animated run internally joined", () => {
    expect(penLifts(MALAYALAM_CHILLU_N)).toBe(1);
    expect(MALAYALAM_CHILLU_N.strokes).toHaveLength(2);
    expect(
      MALAYALAM_CHILLU_N.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb clockwise around the left arch and turn inward at the upper junction",
        "descend the central stem to the line",
      ],
      [
        "carry the upper shoulder right, sweep clockwise around the outer loop, and return through its inner curve",
        "rise into the chillu hook and curl left above the line",
      ],
    ]);
  });

  it("Malayalam chillu ൾ keeps all four animated movements in one run", () => {
    expect(penLifts(MALAYALAM_CHILLU_LL)).toBe(0);
    expect(MALAYALAM_CHILLU_LL.strokes).toHaveLength(1);
    expect(
      MALAYALAM_CHILLU_LL.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "descend clockwise around the left bowl and climb the central rise",
      "carry the upper shoulder right",
      "sweep clockwise around the right loop and return to the upper crossing",
      "rise into the chillu hook and curl left above the line",
    ]);
  });

  it("Malayalam chillu ർ keeps all three animated movements in one run", () => {
    expect(penLifts(MALAYALAM_CHILLU_RR)).toBe(0);
    expect(MALAYALAM_CHILLU_RR.strokes).toHaveLength(1);
    expect(
      MALAYALAM_CHILLU_RR.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "climb around the left arch and carry the upper shoulder right",
      "sweep clockwise around the right loop and return to the upper crossing",
      "rise into the chillu hook and curl left above the line",
    ]);
  });

  it("Malayalam ഴ keeps all three animated movements in one run", () => {
    expect(penLifts(MALAYALAM_ZHA)).toBe(0);
    expect(MALAYALAM_ZHA.strokes).toHaveLength(1);
    expect(
      MALAYALAM_ZHA.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "descend around the left entry arch and sweep right into the lower junction",
      "turn clockwise around the right loop and return through its inner side",
      "descend through the inner return and curl left around the lower hook",
    ]);
  });
});
