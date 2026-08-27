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

const HEBREW_ALEF = DUCTUS[ductusKey("hebrew", "א")];
const HEBREW_BET = DUCTUS[ductusKey("hebrew", "ב")];
const HEBREW_GIMEL = DUCTUS[ductusKey("hebrew", "ג")];
const HEBREW_DALET = DUCTUS[ductusKey("hebrew", "ד")];
const HEBREW_HEI = DUCTUS[ductusKey("hebrew", "ה")];
const HEBREW_VAV = DUCTUS[ductusKey("hebrew", "ו")];
const HEBREW_ZAYIN = DUCTUS[ductusKey("hebrew", "ז")];
const HEBREW_HEIT = DUCTUS[ductusKey("hebrew", "ח")];
const HEBREW_TET = DUCTUS[ductusKey("hebrew", "ט")];
const HEBREW_YOD = DUCTUS[ductusKey("hebrew", "י")];
const HEBREW_KAF = DUCTUS[ductusKey("hebrew", "כ")];
const HEBREW_LAMED = DUCTUS[ductusKey("hebrew", "ל")];
const HEBREW_MEM = DUCTUS[ductusKey("hebrew", "מ")];
const HEBREW_NUN = DUCTUS[ductusKey("hebrew", "נ")];
const HEBREW_SAMEKH = DUCTUS[ductusKey("hebrew", "ס")];
const HEBREW_AYIN = DUCTUS[ductusKey("hebrew", "ע")];
const HEBREW_PE = DUCTUS[ductusKey("hebrew", "פ")];
const HEBREW_TSADI = DUCTUS[ductusKey("hebrew", "צ")];
const HEBREW_QOF = DUCTUS[ductusKey("hebrew", "ק")];
const HEBREW_RESH = DUCTUS[ductusKey("hebrew", "ר")];
const HEBREW_SHIN = DUCTUS[ductusKey("hebrew", "ש")];
const HEBREW_TAV = DUCTUS[ductusKey("hebrew", "ת")];

const OWNER_SCRIPTS = new Set(["hebrew"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, {});

  beforeAll(() => {
    expect(verifiedLetterFont("א", HEBREW_ALEF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ב", HEBREW_BET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ג", HEBREW_GIMEL.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ד", HEBREW_DALET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ה", HEBREW_HEI.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ו", HEBREW_VAV.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ז", HEBREW_ZAYIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ח", HEBREW_HEIT.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ט", HEBREW_TET.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("י", HEBREW_YOD.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("כ", HEBREW_KAF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ל", HEBREW_LAMED.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("מ", HEBREW_MEM.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("נ", HEBREW_NUN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ס", HEBREW_SAMEKH.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ע", HEBREW_AYIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("פ", HEBREW_PE.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("צ", HEBREW_TSADI.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ק", HEBREW_QOF.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ר", HEBREW_RESH.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ש", HEBREW_SHIN.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
    expect(verifiedLetterFont("ת", HEBREW_TAV.source.url)).toBe(
      "_fonts/NotoSansHebrew-Static.ttf",
    );
  });

  it("marks Hebrew complete with 22 sourced letters and its corpus niqqud", () => {
    const hebrew = SCRIPTS.find((script) => script.script === "hebrew")!;
    expect(hebrew.complete).toBe(true);
    expect(hebrew.letters).toHaveLength(22);
    expect(new Set(hebrew.letters.map((letter) => letter.glyph)).size).toBe(22);
    expect(
      hebrew.letters.every((letter) => letter.strokeOrderSource !== undefined),
    ).toBe(true);
    expect(hebrew.marks).toHaveLength(9);
  });

  it("Hebrew א uses two crossed pen-down runs with one lift", () => {
    expect(HEBREW_ALEF.script).toBe("hebrew");
    expect(penLifts(HEBREW_ALEF)).toBe(1);
    expect(HEBREW_ALEF.strokes).toHaveLength(2);
    expect(HEBREW_ALEF.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 2],
    );
    const main = penPath(HEBREW_ALEF.strokes[0]);
    const opposing = penPath(HEBREW_ALEF.strokes[1]);
    expect(main[0].x).toBeLessThan(main.at(-1)!.x);
    expect(main[0].y).toBeGreaterThan(main.at(-1)!.y);
    expect(opposing[0].x).toBeGreaterThan(opposing.at(-1)!.x);
    expect(opposing[0].y).toBeGreaterThan(opposing.at(-1)!.y);
  });

  it("Hebrew ב joins its top and right side before the lifted baseline", () => {
    expect(HEBREW_BET.script).toBe("hebrew");
    expect(penLifts(HEBREW_BET)).toBe(1);
    expect(HEBREW_BET.strokes).toHaveLength(2);
    expect(HEBREW_BET.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const top = HEBREW_BET.strokes[0].segments[0].path;
    const down = HEBREW_BET.strokes[0].segments[1].path;
    const base = HEBREW_BET.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
  });

  it("Hebrew ה joins its top and right side before the detached left leg", () => {
    expect(HEBREW_HEI.script).toBe("hebrew");
    expect(penLifts(HEBREW_HEI)).toBe(1);
    expect(HEBREW_HEI.strokes).toHaveLength(2);
    expect(HEBREW_HEI.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const top = HEBREW_HEI.strokes[0].segments[0].path;
    const down = HEBREW_HEI.strokes[0].segments[1].path;
    const detached = HEBREW_HEI.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(detached[0].y).toBeGreaterThan(detached.at(-1)!.y);
  });

  it("Hebrew ו joins its small head directly to its descending stem", () => {
    expect(HEBREW_VAV.script).toBe("hebrew");
    expect(penLifts(HEBREW_VAV)).toBe(0);
    expect(HEBREW_VAV.strokes).toHaveLength(1);
    expect(HEBREW_VAV.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2,
    ]);
    const head = HEBREW_VAV.strokes[0].segments[0].path;
    const stem = HEBREW_VAV.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("Hebrew ז joins its short head directly to its curved stem", () => {
    expect(HEBREW_ZAYIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_ZAYIN)).toBe(0);
    expect(HEBREW_ZAYIN.strokes).toHaveLength(1);
    expect(
      HEBREW_ZAYIN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2]);
    const head = HEBREW_ZAYIN.strokes[0].segments[0].path;
    const stem = HEBREW_ZAYIN.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("Hebrew ח joins its top and right side before restarting the joined left leg", () => {
    expect(HEBREW_HEIT.script).toBe("hebrew");
    expect(penLifts(HEBREW_HEIT)).toBe(1);
    expect(HEBREW_HEIT.strokes).toHaveLength(2);
    expect(HEBREW_HEIT.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    const top = HEBREW_HEIT.strokes[0].segments[0].path;
    const down = HEBREW_HEIT.strokes[0].segments[1].path;
    const joined = HEBREW_HEIT.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(down[0]).toEqual(top.at(-1));
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(joined[0].y).toBeGreaterThan(joined.at(-1)!.y);
  });

  it("Hebrew ט draws its left-and-base body before the bottom-up hooked side", () => {
    expect(HEBREW_TET.script).toBe("hebrew");
    expect(penLifts(HEBREW_TET)).toBe(1);
    expect(HEBREW_TET.strokes).toHaveLength(2);
    expect(HEBREW_TET.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 2,
    ]);
    const left = HEBREW_TET.strokes[0].segments[0].path;
    const base = HEBREW_TET.strokes[0].segments[1].path;
    const right = HEBREW_TET.strokes[1].segments[0].path;
    const hook = HEBREW_TET.strokes[1].segments[1].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(base[0]).toEqual(left.at(-1));
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
    expect(right[0]).toEqual(base.at(-1));
    expect(right[0].y).toBeLessThan(right.at(-1)!.y);
    expect(hook[0]).toEqual(right.at(-1));
    expect(hook.at(-1)!.x).toBeLessThan(hook[0].x);
  });

  it("Hebrew י joins its tiny head directly to the short descending stem", () => {
    expect(HEBREW_YOD.script).toBe("hebrew");
    expect(penLifts(HEBREW_YOD)).toBe(0);
    expect(HEBREW_YOD.strokes).toHaveLength(1);
    expect(HEBREW_YOD.strokes[0].segments).toHaveLength(2);
    const head = HEBREW_YOD.strokes[0].segments[0].path;
    const stem = HEBREW_YOD.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(stem[0]).toEqual(head.at(-1));
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.min(...stem.map((point) => point.y))).toBeGreaterThan(250);
  });

  it("Hebrew כ turns its top bar into the right side and base without lifting", () => {
    expect(HEBREW_KAF.script).toBe("hebrew");
    expect(penLifts(HEBREW_KAF)).toBe(0);
    expect(HEBREW_KAF.strokes).toHaveLength(1);
    expect(HEBREW_KAF.strokes[0].segments).toHaveLength(3);
    const top = HEBREW_KAF.strokes[0].segments[0].path;
    const side = HEBREW_KAF.strokes[0].segments[1].path;
    const base = HEBREW_KAF.strokes[0].segments[2].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew ל descends, travels right, then turns diagonally down-left", () => {
    expect(HEBREW_LAMED.script).toBe("hebrew");
    expect(penLifts(HEBREW_LAMED)).toBe(0);
    expect(HEBREW_LAMED.strokes).toHaveLength(1);
    expect(HEBREW_LAMED.strokes[0].segments).toHaveLength(3);
    const tall = HEBREW_LAMED.strokes[0].segments[0].path;
    const bar = HEBREW_LAMED.strokes[0].segments[1].path;
    const lower = HEBREW_LAMED.strokes[0].segments[2].path;
    expect(tall[0].y).toBeGreaterThan(tall.at(-1)!.y);
    expect(bar[0]).toEqual(tall.at(-1));
    expect(bar[0].x).toBeLessThan(bar.at(-1)!.x);
    expect(lower[0]).toEqual(bar.at(-1));
    expect(lower[0].y).toBeGreaterThan(lower.at(-1)!.y);
    expect(lower[0].x).toBeGreaterThan(lower.at(-1)!.x);
  });

  it("Hebrew מ lifts once between its detached angled part and joined right body", () => {
    expect(HEBREW_MEM.script).toBe("hebrew");
    expect(penLifts(HEBREW_MEM)).toBe(1);
    expect(HEBREW_MEM.strokes).toHaveLength(2);
    expect(HEBREW_MEM.strokes[0].segments).toHaveLength(2);
    expect(HEBREW_MEM.strokes[1].segments).toHaveLength(3);
    const diagonal = HEBREW_MEM.strokes[0].segments[0].path;
    const inner = HEBREW_MEM.strokes[0].segments[1].path;
    const upper = HEBREW_MEM.strokes[1].segments[0].path;
    const side = HEBREW_MEM.strokes[1].segments[1].path;
    const base = HEBREW_MEM.strokes[1].segments[2].path;
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeLessThan(diagonal.at(-1)!.y);
    expect(inner[0]).toEqual(diagonal.at(-1));
    expect(inner[0].y).toBeGreaterThan(inner.at(-1)!.y);
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(side[0]).toEqual(upper.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew נ joins its top head, right descent, and leftward base", () => {
    expect(HEBREW_NUN.script).toBe("hebrew");
    expect(penLifts(HEBREW_NUN)).toBe(0);
    expect(HEBREW_NUN.strokes).toHaveLength(1);
    expect(HEBREW_NUN.strokes[0].segments).toHaveLength(3);
    const head = HEBREW_NUN.strokes[0].segments[0].path;
    const side = HEBREW_NUN.strokes[0].segments[1].path;
    const base = HEBREW_NUN.strokes[0].segments[2].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(side[0]).toEqual(head.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
  });

  it("Hebrew ס closes its printed loop clockwise without lifting", () => {
    expect(HEBREW_SAMEKH.script).toBe("hebrew");
    expect(penLifts(HEBREW_SAMEKH)).toBe(0);
    expect(HEBREW_SAMEKH.strokes).toHaveLength(1);
    expect(HEBREW_SAMEKH.strokes[0].segments).toHaveLength(4);
    const [top, right, base, left] = HEBREW_SAMEKH.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(right[0]).toEqual(top.at(-1));
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(base[0]).toEqual(right.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(left[0]).toEqual(base.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(left.at(-1)).toEqual(top[0]);
  });

  it("Hebrew ע descends, sweeps left, and climbs without lifting", () => {
    expect(HEBREW_AYIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_AYIN)).toBe(0);
    expect(HEBREW_AYIN.strokes).toHaveLength(1);
    expect(HEBREW_AYIN.strokes[0].segments).toHaveLength(3);
    const [right, base, left] = HEBREW_AYIN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(base[0]).toEqual(right.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(left[0]).toEqual(base.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
  });

  it("Hebrew פ draws its outer body before the lifted inner curl", () => {
    expect(HEBREW_PE.script).toBe("hebrew");
    expect(penLifts(HEBREW_PE)).toBe(1);
    expect(HEBREW_PE.strokes).toHaveLength(2);
    expect(HEBREW_PE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 1,
    ]);
    const [top, side, base] = HEBREW_PE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const inner = HEBREW_PE.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
    expect(base[0]).toEqual(side.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(inner[0].x).toBeLessThan(inner.at(-1)!.x);
  });

  it("Hebrew צ turns its long diagonal into the base before the lifted arm", () => {
    expect(HEBREW_TSADI.script).toBe("hebrew");
    expect(penLifts(HEBREW_TSADI)).toBe(1);
    expect(HEBREW_TSADI.strokes).toHaveLength(2);
    expect(
      HEBREW_TSADI.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1]);
    const diagonal = HEBREW_TSADI.strokes[0].segments[0].path;
    const base = HEBREW_TSADI.strokes[0].segments[1].path;
    const arm = HEBREW_TSADI.strokes[1].segments[0].path;
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
    expect(base[0]).toEqual(diagonal.at(-1));
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    expect(arm[0].x).toBeGreaterThan(arm.at(-1)!.x);
    expect(arm[0].y).toBeGreaterThan(arm.at(-1)!.y);
  });

  it("Hebrew ק joins its top and right body before the lifted descender", () => {
    expect(HEBREW_QOF.script).toBe("hebrew");
    expect(penLifts(HEBREW_QOF)).toBe(1);
    expect(HEBREW_QOF.strokes).toHaveLength(2);
    expect(HEBREW_QOF.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const top = HEBREW_QOF.strokes[0].segments[0].path;
    const body = HEBREW_QOF.strokes[0].segments[1].path;
    const stem = HEBREW_QOF.strokes[1].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(body[0]).toEqual(top.at(-1));
    expect(body[0].x).toBeGreaterThan(body.at(-1)!.x);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(stem[0].y).toBeGreaterThan(0);
    expect(stem.at(-1)!.y).toBeLessThan(0);
  });

  it("Hebrew ר rounds its top bar directly into the right downstroke", () => {
    expect(HEBREW_RESH.script).toBe("hebrew");
    expect(penLifts(HEBREW_RESH)).toBe(0);
    expect(HEBREW_RESH.strokes).toHaveLength(1);
    expect(HEBREW_RESH.strokes[0].segments).toHaveLength(2);
    const top = HEBREW_RESH.strokes[0].segments[0].path;
    const side = HEBREW_RESH.strokes[0].segments[1].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(side[0]).toEqual(top.at(-1));
    expect(side[0].y).toBeGreaterThan(side.at(-1)!.y);
  });

  it("Hebrew ש draws its outer bowl before the lifted middle branch", () => {
    expect(HEBREW_SHIN.script).toBe("hebrew");
    expect(penLifts(HEBREW_SHIN)).toBe(1);
    expect(HEBREW_SHIN.strokes).toHaveLength(2);
    expect(HEBREW_SHIN.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    const outer = HEBREW_SHIN.strokes[0].segments[0].path;
    const left = HEBREW_SHIN.strokes[0].segments[1].path;
    const middle = HEBREW_SHIN.strokes[1].segments[0].path;
    expect(outer[0].y).toBeGreaterThan(outer.at(-1)!.y);
    expect(outer[0].x).toBeGreaterThan(outer.at(-1)!.x);
    expect(left[0]).toEqual(outer.at(-1));
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(middle[0].y).toBeGreaterThan(middle.at(-1)!.y);
  });

  it("Hebrew ת joins its top and right side before the lifted left leg", () => {
    expect(HEBREW_TAV.script).toBe("hebrew");
    expect(penLifts(HEBREW_TAV)).toBe(1);
    expect(HEBREW_TAV.strokes).toHaveLength(2);
    expect(HEBREW_TAV.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 2,
    ]);
    const top = HEBREW_TAV.strokes[0].segments[0].path;
    const right = HEBREW_TAV.strokes[0].segments[1].path;
    const leg = HEBREW_TAV.strokes[1].segments[0].path;
    const foot = HEBREW_TAV.strokes[1].segments[1].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(right[0]).toEqual(top.at(-1));
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(leg[0].y).toBeGreaterThan(leg.at(-1)!.y);
    expect(foot[0]).toEqual(leg.at(-1));
    expect(foot[0].x).toBeGreaterThan(foot.at(-1)!.x);
  });

  it("Hebrew א traces its two-run order to the dedicated HebrewPod101 lesson", () => {
    const src = HEBREW_ALEF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=JBVpQzvrJ4w");
    expect(src.citation).toMatch(
      /Hebrew Writing #1.*Alef and Beit.*01:33.0–01:35.8.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /printed block Alef.*two handwritten variants.*descending main diagonal.*01:33.0–01:34.25.*lifts once.*opposing diagonal.*upper right.*crossing.*01:34.5–01:35.8.*styles vary.*X-like.*Noto Sans Hebrew.*two-stroke/i,
    );
  });

  it("Hebrew ב traces its block-style body separately from the optional dagesh", () => {
    const src = HEBREW_BET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=JBVpQzvrJ4w");
    expect(src.citation).toMatch(
      /Hebrew Writing #1.*Alef and Beit.*02:25.8–02:27.7.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /two handwritten Bet styles.*second.*block-style.*top bar left-to-right.*right side.*without lifting.*02:25.8–02:26.7.*lifts once.*baseline left-to-right.*02:26.9–02:27.7.*dagesh.*02:27.9–02:28.2.*not part of the base ב glyph.*one-lift body.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ג traces its printed angular form without erasing the cursive alternative", () => {
    const src = HEBREW_GIMEL.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tN6Mf7fxxS4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Gimel, Dalet and Kamats.*00:54.2–00:55.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /contrasts.*rounded cursive Gimel.*00:48.4–00:50.1.*printed form.*top bar left-to-right.*right stem.*short lower-right leg.*00:54.2–00:55.4.*lifts once.*lower junction.*longer diagonal leg down-left.*00:55.4–00:55.9.*Noto Sans Hebrew.*handwriting variation/i,
    );
  });

  it("Hebrew ד preserves its one-curve cursive order on the angular block outline", () => {
    const src = HEBREW_DALET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tN6Mf7fxxS4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Gimel, Dalet and Kamats.*03:43.8–03:45.0.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /cursive Dalet.*broad arch left-to-right.*03:43.8–03:44.1.*small lower loop.*03:44.1–03:44.7.*descending tail.*03:45.0.*without lifting.*just one curve.*03:46.6–03:47.3.*printed Dalet.*angular.*Noto Sans Hebrew.*block top-bar-and-right-downstroke.*single continuous run.*zero-lift/i,
    );
  });

  it("Hebrew ה traces its printed body separately from the detached left leg", () => {
    const src = HEBREW_HEI.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=FtCuWlS6V7g");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Hei.*00:59.6–01:01.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /curved handwritten Hei.*00:53.7–00:55.1.*printed form.*sharp angles.*top bar left-to-right.*right side.*without lifting.*00:59.6–01:00.8.*lifts once.*detached left leg top-to-bottom.*01:01.2–01:01.9.*curved in handwriting.*sharp angles in print.*01:03.5–01:09.4.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ו keeps its printed head and stem in one sourced run", () => {
    const src = HEBREW_VAV.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=kJUMyHR0zN4");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Vav, Hirik, and Shuruk.*01:08.6–01:09.8.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Vav.*top to bottom.*00:57.3–00:58.2.*printed form.*01:08.6–01:09.8.*head runs left-to-right.*vertical stem without lifting.*one stroke from top to bottom.*01:00.0–01:02.5.*print version.*small difference.*01:03.6–01:10.9.*Noto Sans Hebrew.*Hirik and Shuruk.*vowel marks.*not part of base U\+05D5.*zero-lift body count/i,
    );
  });

  it("Hebrew ז preserves its rounded handwritten run on the block outline", () => {
    const src = HEBREW_ZAYIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=XTqG_1dsFSU");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Zayin and Heit.*00:44.0–00:45.4.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Zayin.*one uninterrupted run.*00:44.0–00:45.4.*opening rises to the right.*curves down the right side.*around the base without lifting.*mirror image.*handwritten Gimel.*00:49.9–00:55.7.*facing directions.*01:00.1–01:05.0.*printed form.*angular.*not to write Zayin like Vav.*01:17.6–01:24.2.*left-to-right start.*continuous descent.*Noto Sans Hebrew/i,
    );
  });

  it("Hebrew ח traces its printed gate before restarting the joined left leg", () => {
    const src = HEBREW_HEIT.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=XTqG_1dsFSU");
    expect(src.citation).toMatch(
      /Hebrew Writing.*Zayin and Heit.*02:44.6–02:46.3.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Heit.*02:35.7–02:36.9.*arched top.*right side.*one lift.*left leg.*top junction.*printed demonstration.*top bar left-to-right.*right side.*02:44.6–02:45.3.*lifts once.*joined left leg top-to-bottom.*02:45.6–02:46.3.*handwriting corners round.*02:39.0–02:41.6.*print version sharper.*02:47.9–02:52.5.*Noto Sans Hebrew.*rounded handwriting variation/i,
    );
  });

  it("Hebrew ט traces its printed bowl before the unusual bottom-up hook", () => {
    const src = HEBREW_TET.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=NBUtBPVKchk");
    expect(src.citation).toMatch(
      /Hebrew Writing #6.*Tet and Yod.*00:54.2–00:56.3.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /rounded handwritten Tet.*00:44.9–00:46.0.*start at the bottom.*left curve.*right side.*curl inward.*unusual.*bottom up.*00:47.4–00:51.1.*printed demonstration.*left side top-to-bottom.*base.*00:54.2–00:55.4.*lifts once.*lower-right.*climbs the right side.*inward hook.*00:55.7–00:56.3.*rounding the printed corners.*handwritten form.*00:57.5–01:02.3.*Noto Sans Hebrew.*bottom-up handwritten variation/i,
    );
  });

  it("Hebrew י traces its tiny printed head and stem in one run", () => {
    const src = HEBREW_YOD.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=NBUtBPVKchk");
    expect(src.citation).toMatch(
      /Hebrew Writing #6.*Tet and Yod.*02:00.7–02:01.2.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Yod.*tiny comma-like run.*01:53.4–01:53.8.*high on the writing line.*simplest letter.*01:50.4–01:51.0.*little comma.*upper-right.*01:54.0–01:57.2.*printed demonstration.*head left-to-right.*short stem.*without lifting.*02:00.7–02:01.2.*print is almost the same.*little angle.*02:03.7–02:07.1.*Noto Sans Hebrew.*comma-like handwritten variation/i,
    );
  });

  it("Hebrew כ traces the sharp printed form in one continuous run", () => {
    const src = HEBREW_KAF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=EcQ0gL-NM-k");
    expect(src.citation).toMatch(
      /Hebrew Writing #7.*Kaf.*00:51.3–00:53.2.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Kaf.*rounded half-circle.*upper-left.*00:40.2–00:41.4.*half a circle to the right.*upper-left side.*going down.*00:41.8–00:47.0.*printed demonstration.*top bar left-to-right.*right side.*left along the base.*without lifting.*00:51.3–00:53.2.*same but with sharp corners.*00:47.3–00:49.5.*Noto Sans Hebrew.*rounded handwritten variation/i,
    );
  });

  it("Hebrew ל traces the tall printed form in one continuous run", () => {
    const src = HEBREW_LAMED.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=CBU6aSCcPrE");
    expect(src.citation).toMatch(
      /Hebrew Writing #8.*Lamed and Mem.*01:22.4–01:23.9.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Lamed.*rounded looping run.*00:52.8–00:53.7.*printed demonstration.*top.*tall left stroke.*descends.*middle junction.*right along the bar.*diagonally down-left.*without lifting.*01:22.4–01:23.9.*printed form is angular.*handwriting is looped and rounded.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew מ traces the open printed form in two pen-down runs", () => {
    const src = HEBREW_MEM.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=CBU6aSCcPrE");
    expect(src.citation).toMatch(
      /Hebrew Writing #8.*Lamed and Mem.*03:07.7–03:10.6.*HebrewPod101/i,
    );
    expect(src.variation).toMatch(
      /handwritten Mem.*N-like cursive zigzag.*03:02.8–03:05.3.*printed demonstration.*detached left part.*lower tip.*corner.*down-right.*short inner leg.*lifts once.*angular right body.*climb diagonally right.*upper shoulder.*down the right side.*left along the base.*03:07.7–03:10.6.*open at the bottom-left.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew נ traces the printed hook in one continuous run", () => {
    const src = HEBREW_NUN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:04.1–02:04.6.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Nun.*one continuous run.*02:04.1–02:04.6.*top head.*left-to-right.*right side.*left along the base.*without lifting.*cursive Nun.*purple.*rounder, wider hook.*02:05.2–02:05.8.*without lifting.*previously queued.*Hebrew Letters - NUN.*3gYCaDgB-Nk.*religious exposition.*not used.*Noto Sans Hebrew.*handwritten variation/i,
    );
  });

  it("Hebrew ס traces one printed clockwise loop and records the round cursive form", () => {
    const src = HEBREW_SAMEKH.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:19.5–02:20.8.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Samekh.*one continuous clockwise run.*02:19.5–02:20.8.*flat top.*left-to-right.*right side.*left along the base.*left side.*close.*without lifting.*cursive Samekh.*purple.*rounder oval.*02:23.8–02:24.7.*one uninterrupted loop.*Noto Sans Hebrew.*cursive variation/i,
    );
  });

  it("Hebrew ע traces one printed branch-and-base run and records cursive looping", () => {
    const src = HEBREW_AYIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:27.4–02:28.9.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Ayin.*one continuous run.*02:27.4–02:28.9.*right branch.*descends.*left into the base.*farther left.*turns back.*climbs the left branch.*without lifting.*cursive Ayin.*purple.*compact looped form.*02:31.6–02:32.7.*without lifting.*Noto Sans Hebrew.*cursive variation/i,
    );
  });

  it("Hebrew פ traces its two-run printed form and records the cursive spiral", () => {
    const src = HEBREW_PE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:36.3–02:38.9.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Pe.*two runs.*02:36.3–02:38.9.*outer body.*upper left.*right across the top.*right side.*left along the base.*one lift.*short inner curl.*left to right.*cursive Pe.*purple.*one inward spiral.*02:41.5–02:43.0.*without lifting.*Noto Sans Hebrew.*rounded cursive variation/i,
    );
  });

  it("Hebrew צ traces its two-run printed form and records the compact cursive form", () => {
    const src = HEBREW_TSADI.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*02:59.8–03:01.2.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Tsadi.*two runs.*02:59.8–03:01.2.*long diagonal.*upper left.*turns sharply left.*base.*without lifting.*one lift.*short upper-right arm.*down-left.*middle junction.*cursive Tsadi.*purple.*compact, rounded 3-like run.*03:03.2–03:04.0.*without lifting.*Noto Sans Hebrew.*compact cursive variation/i,
    );
  });

  it("Hebrew ק traces its two-run printed form and records the cursive hook", () => {
    const src = HEBREW_QOF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:18.3–03:20.0.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Qof.*two runs.*03:18.3–03:20.0.*top bar.*left-to-right.*down-left.*right body.*without lifting.*one lift.*separate inner-left stem.*below the writing line.*cursive Qof.*purple.*one continuous hooked descent.*03:22.0–03:23.3.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ר traces its one-run printed form and records the rounder cursive hook", () => {
    const src = HEBREW_RESH.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:26.2–03:27.1.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Resh.*one continuous run.*03:26.2–03:27.1.*top bar.*left-to-right.*rounds the top-right corner.*right side.*without lifting.*cursive Resh.*purple.*rounder hook.*03:29.3–03:30.0.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ש traces its two-run printed form and records the cursive loop", () => {
    const src = HEBREW_SHIN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:34.0–03:36.3.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Shin.*two runs.*03:34.0–03:36.3.*top of the right branch.*descends.*outer U-shaped base.*climbs the left branch.*without lifting.*one lift.*descends the middle branch.*cursive Shin.*purple.*one rounded inward loop.*short rightward exit.*03:39.2–03:40.2.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });

  it("Hebrew ת traces its two-run printed form and records the cursive retrace", () => {
    const src = HEBREW_TAV.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=8wi_uPY9uZA");
    expect(src.citation).toMatch(
      /How to write the Hebrew alphabet in print and cursive.*03:45.2–03:47.3.*Aural Writing.*8 June 2022/i,
    );
    expect(src.variation).toMatch(
      /printed Tav.*two runs.*03:45.2–03:47.3.*top bar.*left-to-right.*right side.*without lifting.*one lift.*separate left leg.*curves left.*small foot.*cursive Tav.*purple.*one continuous run.*03:49.0–03:50.9.*descend the left stem.*curl left.*retrace.*climb.*arch right.*short right side.*without lifting.*Noto Sans Hebrew.*one-run cursive variation/i,
    );
  });
});
