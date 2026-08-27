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

const CHINESE_REN = DUCTUS[ductusKey("chinese", "人")];
const CHINESE_PERSON_RADICAL = DUCTUS[ductusKey("chinese", "亻")];
const CHINESE_MOUTH = DUCTUS[ductusKey("chinese", "口")];
const CHINESE_WOMAN = DUCTUS[ductusKey("chinese", "女")];
const CHINESE_CHILD = DUCTUS[ductusKey("chinese", "子")];
const CHINESE_SUN = DUCTUS[ductusKey("chinese", "日")];
const CHINESE_SPEECH_RADICAL = DUCTUS[ductusKey("chinese", "讠")];
const CHINESE_WATER_RADICAL = DUCTUS[ductusKey("chinese", "氵")];
const CHINESE_ROOF_RADICAL = DUCTUS[ductusKey("chinese", "宀")];
const CHINESE_YOU = DUCTUS[ductusKey("chinese", "你")];
const CHINESE_GOOD = DUCTUS[ductusKey("chinese", "好")];
const CHINESE_I = DUCTUS[ductusKey("chinese", "我")];
const CHINESE_BE = DUCTUS[ductusKey("chinese", "是")];
const CHINESE_NOT = DUCTUS[ductusKey("chinese", "不")];
const CHINESE_NAME = DUCTUS[ductusKey("chinese", "名")];
const CHINESE_CHARACTER = DUCTUS[ductusKey("chinese", "字")];
const CHINESE_THANK = DUCTUS[ductusKey("chinese", "谢")];
const CHINESE_PLEASE = DUCTUS[ductusKey("chinese", "请")];
const CHINESE_AGAIN = DUCTUS[ductusKey("chinese", "再")];
const CHINESE_SEE = DUCTUS[ductusKey("chinese", "见")];
const CHINESE_WHAT = DUCTUS[ductusKey("chinese", "什")];
const CHINESE_PARTICLE_ME = DUCTUS[ductusKey("chinese", "么")];
const CHINESE_EARLY = DUCTUS[ductusKey("chinese", "早")];
const CHINESE_UP = DUCTUS[ductusKey("chinese", "上")];
const CHINESE_HAN = DUCTUS[ductusKey("chinese", "汉")];
const CHINESE_LANGUAGE = DUCTUS[ductusKey("chinese", "语")];
const CHINESE_WRITING = DUCTUS[ductusKey("chinese", "文")];
const CHINESE_COUNTRY = DUCTUS[ductusKey("chinese", "国")];
const CHINESE_LOOK = DUCTUS[ductusKey("chinese", "看")];
const CHINESE_BOOK = DUCTUS[ductusKey("chinese", "书")];

const OWNER_SCRIPTS = new Set(["chinese"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, {});

  beforeAll(() => {
    expect(verifiedLetterFont("人", CHINESE_REN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("亻", CHINESE_PERSON_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("口", CHINESE_MOUTH.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("女", CHINESE_WOMAN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("子", CHINESE_CHILD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("日", CHINESE_SUN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("讠", CHINESE_SPEECH_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("氵", CHINESE_WATER_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("宀", CHINESE_ROOF_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("你", CHINESE_YOU.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("好", CHINESE_GOOD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("我", CHINESE_I.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("是", CHINESE_BE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("不", CHINESE_NOT.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("名", CHINESE_NAME.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("字", CHINESE_CHARACTER.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("谢", CHINESE_THANK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("请", CHINESE_PLEASE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("再", CHINESE_AGAIN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("见", CHINESE_SEE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("什", CHINESE_WHAT.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("么", CHINESE_PARTICLE_ME.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("早", CHINESE_EARLY.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("上", CHINESE_UP.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("汉", CHINESE_HAN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("语", CHINESE_LANGUAGE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("文", CHINESE_WRITING.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("国", CHINESE_COUNTRY.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("看", CHINESE_LOOK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("书", CHINESE_BOOK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
  });

  it("marks Chinese complete with every current-corpus row source-verified", () => {
    const chinese = SCRIPTS.find((script) => script.script === "chinese")!;
    expect(chinese.complete).toBe(true);
    expect(chinese.letters).toHaveLength(43);
    expect(new Set(chinese.letters.map((letter) => letter.glyph)).size).toBe(
      43,
    );
    expect(
      chinese.letters.every((letter) => letter.strokeOrderSource !== undefined),
    ).toBe(true);
  });

  it("Chinese 人 draws the left-falling stroke before the lifted right-falling stroke", () => {
    expect(CHINESE_REN.script).toBe("chinese");
    expect(penLifts(CHINESE_REN)).toBe(1);
    expect(CHINESE_REN.strokes).toHaveLength(2);
    expect(CHINESE_REN.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const left = CHINESE_REN.strokes[0].segments[0].path;
    const right = CHINESE_REN.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(right[0].x).toBeLessThan(right.at(-1)!.x);
  });

  it("Chinese 亻 draws the left-falling stroke before the lifted vertical", () => {
    expect(CHINESE_PERSON_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_PERSON_RADICAL)).toBe(1);
    expect(CHINESE_PERSON_RADICAL.strokes).toHaveLength(2);
    expect(
      CHINESE_PERSON_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const left = CHINESE_PERSON_RADICAL.strokes[0].segments[0].path;
    const vertical = CHINESE_PERSON_RADICAL.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical[0].x).toBe(vertical.at(-1)!.x);
  });

  it("Chinese 口 descends the left, joins the top and right, then closes the bottom", () => {
    expect(CHINESE_MOUTH.script).toBe("chinese");
    expect(penLifts(CHINESE_MOUTH)).toBe(2);
    expect(CHINESE_MOUTH.strokes).toHaveLength(3);
    expect(
      CHINESE_MOUTH.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1]);
    const left = CHINESE_MOUTH.strokes[0].segments[0].path;
    const top = CHINESE_MOUTH.strokes[1].segments[0].path;
    const right = CHINESE_MOUTH.strokes[1].segments[1].path;
    const bottom = CHINESE_MOUTH.strokes[2].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 女 keeps its first bend joined before the falling and horizontal strokes", () => {
    expect(CHINESE_WOMAN.script).toBe("chinese");
    expect(penLifts(CHINESE_WOMAN)).toBe(2);
    expect(CHINESE_WOMAN.strokes).toHaveLength(3);
    expect(
      CHINESE_WOMAN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1]);
    const descent = CHINESE_WOMAN.strokes[0].segments[0].path;
    const sweep = CHINESE_WOMAN.strokes[0].segments[1].path;
    const falling = CHINESE_WOMAN.strokes[1].segments[0].path;
    const horizontal = CHINESE_WOMAN.strokes[2].segments[0].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent[0].x).toBeGreaterThan(descent.at(-1)!.x);
    expect(descent.at(-1)).toEqual(sweep[0]);
    expect(sweep[0].x).toBeLessThan(sweep.at(-1)!.x);
    expect(sweep[0].y).toBeGreaterThan(sweep.at(-1)!.y);
    expect(falling[0].x).toBeGreaterThan(falling.at(-1)!.x);
    expect(falling[0].y).toBeGreaterThan(falling.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 子 keeps both hooks joined before its final horizontal", () => {
    expect(CHINESE_CHILD.script).toBe("chinese");
    expect(penLifts(CHINESE_CHILD)).toBe(2);
    expect(CHINESE_CHILD.strokes).toHaveLength(3);
    expect(
      CHINESE_CHILD.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 2, 1]);
    const top = CHINESE_CHILD.strokes[0].segments[0].path;
    const topTurn = CHINESE_CHILD.strokes[0].segments[1].path;
    const vertical = CHINESE_CHILD.strokes[1].segments[0].path;
    const baseHook = CHINESE_CHILD.strokes[1].segments[1].path;
    const horizontal = CHINESE_CHILD.strokes[2].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(topTurn[0]);
    expect(topTurn[0].x).toBeGreaterThan(topTurn.at(-1)!.x);
    expect(topTurn[0].y).toBeGreaterThan(topTurn.at(-1)!.y);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 日 writes the inside bar before separately closing the box", () => {
    expect(CHINESE_SUN.script).toBe("chinese");
    expect(penLifts(CHINESE_SUN)).toBe(3);
    expect(CHINESE_SUN.strokes).toHaveLength(4);
    expect(CHINESE_SUN.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 2, 1, 1],
    );
    const left = CHINESE_SUN.strokes[0].segments[0].path;
    const top = CHINESE_SUN.strokes[1].segments[0].path;
    const right = CHINESE_SUN.strokes[1].segments[1].path;
    const middle = CHINESE_SUN.strokes[2].segments[0].path;
    const bottom = CHINESE_SUN.strokes[3].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 讠 lifts after the dot and keeps both later turns joined", () => {
    expect(CHINESE_SPEECH_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_SPEECH_RADICAL)).toBe(1);
    expect(CHINESE_SPEECH_RADICAL.strokes).toHaveLength(2);
    expect(
      CHINESE_SPEECH_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3]);
    const dot = CHINESE_SPEECH_RADICAL.strokes[0].segments[0].path;
    const horizontal = CHINESE_SPEECH_RADICAL.strokes[1].segments[0].path;
    const descent = CHINESE_SPEECH_RADICAL.strokes[1].segments[1].path;
    const rise = CHINESE_SPEECH_RADICAL.strokes[1].segments[2].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(descent[0]);
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(rise[0]);
    expect(rise[0].x).toBeLessThan(rise.at(-1)!.x);
    expect(rise[0].y).toBeLessThan(rise.at(-1)!.y);
  });

  it("Chinese 氵 draws two falling dots before its joined rising bottom stroke", () => {
    expect(CHINESE_WATER_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_WATER_RADICAL)).toBe(2);
    expect(CHINESE_WATER_RADICAL.strokes).toHaveLength(3);
    expect(
      CHINESE_WATER_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2]);
    const upper = CHINESE_WATER_RADICAL.strokes[0].segments[0].path;
    const middle = CHINESE_WATER_RADICAL.strokes[1].segments[0].path;
    const bottomTurn = CHINESE_WATER_RADICAL.strokes[2].segments[0].path;
    const bottomRise = CHINESE_WATER_RADICAL.strokes[2].segments[1].path;
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(middle[0].y).toBeGreaterThan(middle.at(-1)!.y);
    expect(bottomTurn[0].x).toBeGreaterThan(bottomTurn[1].x);
    expect(bottomTurn.at(-1)).toEqual(bottomRise[0]);
    expect(bottomRise[0].x).toBeLessThan(bottomRise.at(-1)!.x);
    expect(bottomRise[0].y).toBeLessThan(bottomRise.at(-1)!.y);
  });

  it("Chinese 宀 draws two separate marks before its joined roof hook", () => {
    expect(CHINESE_ROOF_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_ROOF_RADICAL)).toBe(2);
    expect(CHINESE_ROOF_RADICAL.strokes).toHaveLength(3);
    expect(
      CHINESE_ROOF_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2]);
    const dot = CHINESE_ROOF_RADICAL.strokes[0].segments[0].path;
    const left = CHINESE_ROOF_RADICAL.strokes[1].segments[0].path;
    const roof = CHINESE_ROOF_RADICAL.strokes[2].segments[0].path;
    const hook = CHINESE_ROOF_RADICAL.strokes[2].segments[1].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(roof[0].x).toBeLessThan(roof.at(-1)!.x);
    expect(roof.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    expect(hook[0].y).toBeGreaterThan(hook.at(-1)!.y);
  });

  it("Chinese 你 writes 亻 before two joined hooks and two separate dots", () => {
    expect(CHINESE_YOU.script).toBe("chinese");
    expect(penLifts(CHINESE_YOU)).toBe(6);
    expect(CHINESE_YOU.strokes).toHaveLength(7);
    expect(CHINESE_YOU.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 1, 2, 2, 1, 1],
    );
    const personFall = CHINESE_YOU.strokes[0].segments[0].path;
    const personVertical = CHINESE_YOU.strokes[1].segments[0].path;
    const upperFall = CHINESE_YOU.strokes[2].segments[0].path;
    const horizontal = CHINESE_YOU.strokes[3].segments[0].path;
    const upperHook = CHINESE_YOU.strokes[3].segments[1].path;
    const vertical = CHINESE_YOU.strokes[4].segments[0].path;
    const baseHook = CHINESE_YOU.strokes[4].segments[1].path;
    const leftDot = CHINESE_YOU.strokes[5].segments[0].path;
    const rightDot = CHINESE_YOU.strokes[6].segments[0].path;
    expect(personFall[0].x).toBeGreaterThan(personFall.at(-1)!.x);
    expect(personVertical[0].y).toBeGreaterThan(personVertical.at(-1)!.y);
    expect(upperFall[0].x).toBeGreaterThan(upperFall.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(upperHook[0]);
    expect(upperHook[0].x).toBeGreaterThan(upperHook.at(-1)!.x);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(leftDot[0].x).toBeGreaterThan(leftDot.at(-1)!.x);
    expect(rightDot[0].x).toBeLessThan(rightDot.at(-1)!.x);
  });

  it("Chinese 好 writes 女 before 子 with three joined turns", () => {
    expect(CHINESE_GOOD.script).toBe("chinese");
    expect(penLifts(CHINESE_GOOD)).toBe(5);
    expect(CHINESE_GOOD.strokes).toHaveLength(6);
    expect(
      CHINESE_GOOD.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 2, 2, 1]);
    const womanBend = CHINESE_GOOD.strokes[0].segments[0].path;
    const womanSweep = CHINESE_GOOD.strokes[0].segments[1].path;
    const womanFall = CHINESE_GOOD.strokes[1].segments[0].path;
    const womanBar = CHINESE_GOOD.strokes[2].segments[0].path;
    const childBar = CHINESE_GOOD.strokes[3].segments[0].path;
    const childTurn = CHINESE_GOOD.strokes[3].segments[1].path;
    const childVertical = CHINESE_GOOD.strokes[4].segments[0].path;
    const childHook = CHINESE_GOOD.strokes[4].segments[1].path;
    const childMiddle = CHINESE_GOOD.strokes[5].segments[0].path;
    expect(womanBend.at(-1)).toEqual(womanSweep[0]);
    expect(womanSweep[0].x).toBeLessThan(womanSweep.at(-1)!.x);
    expect(womanFall[0].x).toBeGreaterThan(womanFall.at(-1)!.x);
    expect(womanBar[0].x).toBeLessThan(womanBar.at(-1)!.x);
    expect(childBar.at(-1)).toEqual(childTurn[0]);
    expect(childTurn[0].x).toBeGreaterThan(childTurn.at(-1)!.x);
    expect(childVertical.at(-1)).toEqual(childHook[0]);
    expect(childHook[0].x).toBeGreaterThan(childHook.at(-1)!.x);
    expect(childMiddle[0].x).toBeLessThan(childMiddle.at(-1)!.x);
  });

  it("Chinese 我 preserves seven strokes and the joined vertical hook", () => {
    expect(CHINESE_I.script).toBe("chinese");
    expect(penLifts(CHINESE_I)).toBe(6);
    expect(CHINESE_I.strokes).toHaveLength(7);
    expect(CHINESE_I.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 2, 1, 2, 1, 1,
    ]);
    const vertical = CHINESE_I.strokes[2].segments[0].path;
    const hook = CHINESE_I.strokes[2].segments[1].path;
    expect(vertical.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    const slash = CHINESE_I.strokes[4].segments[0].path;
    const slashHook = CHINESE_I.strokes[4].segments[1].path;
    expect(slash.at(-1)).toEqual(slashHook[0]);
    expect(slashHook[0].y).toBeLessThan(slashHook.at(-1)!.y);
  });

  it("Chinese 是 closes 日 before its five-stroke lower body", () => {
    expect(CHINESE_BE.script).toBe("chinese");
    expect(penLifts(CHINESE_BE)).toBe(8);
    expect(CHINESE_BE.strokes).toHaveLength(9);
    expect(CHINESE_BE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 2, 1, 1, 1, 1, 1, 1, 1,
    ]);
    const top = CHINESE_BE.strokes[1].segments[0].path;
    const right = CHINESE_BE.strokes[1].segments[1].path;
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Chinese 不 keeps its four source strokes as four pen-down runs", () => {
    expect(CHINESE_NOT.script).toBe("chinese");
    expect(penLifts(CHINESE_NOT)).toBe(3);
    expect(CHINESE_NOT.strokes).toHaveLength(4);
    expect(CHINESE_NOT.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 1, 1],
    );
    expect(CHINESE_NOT.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(
      CHINESE_NOT.strokes[0].segments[0].path[0].x,
    );
    expect(CHINESE_NOT.strokes[1].segments[0].path.at(-1)!.x).toBeLessThan(
      CHINESE_NOT.strokes[1].segments[0].path[0].x,
    );
  });

  it("Chinese 名 completes 夕 before 口 and preserves both joined turns", () => {
    expect(CHINESE_NAME.script).toBe("chinese");
    expect(penLifts(CHINESE_NAME)).toBe(5);
    expect(CHINESE_NAME.strokes).toHaveLength(6);
    expect(
      CHINESE_NAME.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1, 1, 2, 1]);
    expect(CHINESE_NAME.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_NAME.strokes[1].segments[1].path[0],
    );
    expect(CHINESE_NAME.strokes[4].segments[0].path.at(-1)).toEqual(
      CHINESE_NAME.strokes[4].segments[1].path[0],
    );
  });

  it("Chinese 字 completes 宀 before 子 and preserves all three joined turns", () => {
    expect(CHINESE_CHARACTER.script).toBe("chinese");
    expect(penLifts(CHINESE_CHARACTER)).toBe(5);
    expect(CHINESE_CHARACTER.strokes).toHaveLength(6);
    expect(
      CHINESE_CHARACTER.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2, 2, 2, 1]);
    for (const strokeIndex of [2, 3, 4]) {
      expect(
        CHINESE_CHARACTER.strokes[strokeIndex].segments[0].path.at(-1),
      ).toEqual(CHINESE_CHARACTER.strokes[strokeIndex].segments[1].path[0]);
    }
  });

  it("Chinese 谢 completes 讠, 身, and 寸 in order and preserves all five joined turns", () => {
    expect(CHINESE_THANK.script).toBe("chinese");
    expect(penLifts(CHINESE_THANK)).toBe(11);
    expect(CHINESE_THANK.strokes).toHaveLength(12);
    expect(
      CHINESE_THANK.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3, 1, 1, 3, 1, 1, 1, 1, 1, 2, 1]);
    for (const strokeIndex of [1, 4]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(
          CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex].path.at(-1),
        ).toEqual(
          CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex + 1].path[0],
        );
      }
    }
    expect(CHINESE_THANK.strokes[10].segments[0].path.at(-1)).toEqual(
      CHINESE_THANK.strokes[10].segments[1].path[0],
    );
  });

  it("Chinese 请 completes 讠 before 青 and preserves all four joined turns", () => {
    expect(CHINESE_PLEASE.script).toBe("chinese");
    expect(penLifts(CHINESE_PLEASE)).toBe(9);
    expect(CHINESE_PLEASE.strokes).toHaveLength(10);
    expect(
      CHINESE_PLEASE.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3, 1, 1, 1, 1, 1, 3, 1, 1]);
    for (const strokeIndex of [1, 7]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(
          CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex].path.at(
            -1,
          ),
        ).toEqual(
          CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex + 1]
            .path[0],
        );
      }
    }
  });

  it("Chinese 再 closes last and preserves both turns inside the enclosing stroke", () => {
    expect(CHINESE_AGAIN.script).toBe("chinese");
    expect(penLifts(CHINESE_AGAIN)).toBe(5);
    expect(CHINESE_AGAIN.strokes).toHaveLength(6);
    expect(
      CHINESE_AGAIN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 3, 1, 1, 1]);
    for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
      expect(
        CHINESE_AGAIN.strokes[2].segments[segmentIndex].path.at(-1),
      ).toEqual(CHINESE_AGAIN.strokes[2].segments[segmentIndex + 1].path[0]);
    }
  });

  it("Chinese 见 completes its open frame before both lower runs", () => {
    expect(CHINESE_SEE.script).toBe("chinese");
    expect(penLifts(CHINESE_SEE)).toBe(3);
    expect(CHINESE_SEE.strokes).toHaveLength(4);
    expect(CHINESE_SEE.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 2, 1, 3],
    );
    for (const strokeIndex of [1, 3]) {
      const stroke = CHINESE_SEE.strokes[strokeIndex];
      for (
        let segmentIndex = 0;
        segmentIndex + 1 < stroke.segments.length;
        segmentIndex++
      ) {
        expect(stroke.segments[segmentIndex].path.at(-1)).toEqual(
          stroke.segments[segmentIndex + 1].path[0],
        );
      }
    }
  });

  it("Chinese 什 completes 亻 before writing 十 in four separate strokes", () => {
    expect(CHINESE_WHAT.script).toBe("chinese");
    expect(penLifts(CHINESE_WHAT)).toBe(3);
    expect(CHINESE_WHAT.strokes).toHaveLength(4);
    expect(
      CHINESE_WHAT.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const leftStrokes = CHINESE_WHAT.strokes.slice(0, 2).flatMap(penPath);
    const rightStrokes = CHINESE_WHAT.strokes.slice(2).flatMap(penPath);
    expect(Math.max(...leftStrokes.map((point) => point.x))).toBeLessThan(
      Math.min(...rightStrokes.map((point) => point.x)),
    );
  });

  it("Chinese 么 keeps the second fall joined to its rightward base sweep", () => {
    expect(CHINESE_PARTICLE_ME.script).toBe("chinese");
    expect(penLifts(CHINESE_PARTICLE_ME)).toBe(2);
    expect(CHINESE_PARTICLE_ME.strokes).toHaveLength(3);
    expect(
      CHINESE_PARTICLE_ME.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1]);
    expect(CHINESE_PARTICLE_ME.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_PARTICLE_ME.strokes[1].segments[1].path[0],
    );
  });

  it("Chinese 早 completes 日 before the two strokes of 十", () => {
    expect(CHINESE_EARLY.script).toBe("chinese");
    expect(penLifts(CHINESE_EARLY)).toBe(5);
    expect(CHINESE_EARLY.strokes).toHaveLength(6);
    expect(
      CHINESE_EARLY.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1, 1, 1, 1]);
    expect(CHINESE_EARLY.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_EARLY.strokes[1].segments[1].path[0],
    );
    expect(
      Math.max(...penPath(CHINESE_EARLY.strokes[4]).map((point) => point.y)),
    ).toBeLessThan(
      Math.min(...penPath(CHINESE_EARLY.strokes[3]).map((point) => point.y)),
    );
  });

  it("Chinese 上 writes the vertical before its short and long horizontals", () => {
    expect(CHINESE_UP.script).toBe("chinese");
    expect(penLifts(CHINESE_UP)).toBe(2);
    expect(CHINESE_UP.strokes).toHaveLength(3);
    expect(CHINESE_UP.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const vertical = penPath(CHINESE_UP.strokes[0]);
    const shortHorizontal = penPath(CHINESE_UP.strokes[1]);
    const base = penPath(CHINESE_UP.strokes[2]);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(shortHorizontal[0].x).toBeLessThan(shortHorizontal.at(-1)!.x);
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
    expect(base.at(-1)!.x - base[0].x).toBeGreaterThan(
      shortHorizontal.at(-1)!.x - shortHorizontal[0].x,
    );
  });

  it("Chinese 人 traces its two medians to the pinned PRC-order dataset", () => {
    const src = CHINESE_REN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BA.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 人\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper centre.*descends down-left.*median 2.*central junction.*descends down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 亻 traces its falling stroke and vertical to the pinned PRC-order dataset", () => {
    const src = CHINESE_PERSON_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BB.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 亻\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper right.*bends slightly right.*descends down-left.*median 2.*central junction.*moves slightly down-right.*descends vertically.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 口 traces its three-run closing order to the pinned PRC-order dataset", () => {
    const src = CHINESE_MOUTH.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%8F%A3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 口\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined corner.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 女 traces its bent first run and two lifted strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_WOMAN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%B3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 女\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper centre.*short down-right entry.*bends down-left.*lower-left junction.*turns without lifting.*sweeps down-right.*median 2.*upper right.*falls down-left.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*first stroke's internal turn.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 子 traces its two joined turns and final horizontal to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHILD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%90.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 子\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*crosses the top from left to right.*turns without lifting.*sweeps down-left.*median 2.*upper-left turn.*descends through the centre.*hooks left at the base without lifting.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both internal turns.*two intervening pen lifts.*base hook is flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 日 traces its joined corner and inside-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SUN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 日\.json.*ordered stroke paths and medians 1–4.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*four ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*left edge.*crosses the middle from left to right.*median 4.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*inside-before-close order.*three intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 讠 traces its dot and joined turning stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_SPEECH_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AE%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 讠\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper left.*dot down-right.*median 2.*left edge.*rises slightly.*short horizontal.*turns without lifting.*descend.*turns again without lifting.*finishes up-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both turns inside the second stroke.*one intervening pen lift.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 氵 traces its falling dots and rising bottom stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_WATER_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%B0%B5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 氵\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends down-right.*median 2.*middle left.*descends down-right.*median 3.*bottom.*moves slightly up-left.*rises.*upper right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*source order.*two intervening pen lifts.*shallower turn.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 宀 traces its dot, left drop, and joined roof hook to the pinned PRC-order dataset", () => {
    const src = CHINESE_ROOF_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AE%80.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 宀\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*top dot down-right.*median 2.*left side.*descends down-left.*median 3.*left roof edge.*horizontally to the right.*hooks down-left without lifting.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined horizontal hook.*two intervening pen lifts.*squarer.*more vertical.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 你 traces its component order and two joined hooks to the pinned PRC-order dataset", () => {
    const src = CHINESE_YOU.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BD%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 你\.json.*ordered stroke paths and medians 1–7.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*seven ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*upper right.*falls down-left.*median 4.*crosses rightward.*hooks down-left without lifting.*median 5.*centre.*descends.*hooks left without lifting.*medians 6–7.*lower-left dot down-left.*lower-right dot down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*component order.*six intervening pen lifts.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 好 traces 女-before-子 and three joined turns to the pinned PRC-order dataset", () => {
    const src = CHINESE_GOOD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%BD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 好\.json.*ordered stroke paths and medians 1–6.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*six ordered strokes.*Medians 1–3.*女 first.*bent stroke.*down-left.*sweeps right without lifting.*separately started stroke falls down-left.*horizontal crosses left-to-right.*Medians 4–6.*子.*horizontal turns down-left without lifting.*vertical descends and hooks left without lifting.*middle horizontal crosses left-to-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*女-before-子 component order.*all three joined turns.*five intervening pen lifts.*more angular.*flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 我 traces seven strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_I.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%88%91.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 我\.json.*medians 1–7.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /seven ordered strokes.*upper-left falling stroke.*upper horizontal.*hooked vertical.*lower rising stroke.*long curved slash.*hooks upward.*without lifting.*separate rising slash up-left.*final upper-right dot.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*six lifts/i,
    );
  });

  it("Chinese 是 traces 日-first order to the pinned PRC-order dataset", () => {
    const src = CHINESE_BE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%98%AF.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 是\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–4.*日 first.*left vertical.*joined top and right sides.*inner horizontal.*closing bottom horizontal.*Medians 5–9.*wide horizontal.*central vertical.*short lower-right horizontal.*lower-left falling stroke.*long finishing stroke down-right.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*eight intervening lifts/i,
    );
  });

  it("Chinese 不 traces all four separate strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_NOT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 不\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*top horizontal.*left to right.*long falling stroke.*down-left.*central vertical.*right-falling dot.*People's Republic of China stroke order.*Noto Sans SC.*three intervening lifts/i,
    );
  });

  it("Chinese 名 traces 夕-before-口 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_NAME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%90%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 名\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*夕 first.*left-falling stroke.*horizontal.*down-left without lifting.*inner down-right dot.*Medians 4–6.*口.*left vertical.*top horizontal.*right side without lifting.*closing bottom horizontal.*People's Republic of China stroke order.*Noto Sans SC.*both joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 字 traces 宀-before-子 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHARACTER.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%97.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 字\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*宀 first.*down-right top dot.*left-side down-left stroke.*horizontal roof.*hooks down-left without lifting.*Medians 4–6.*子.*top horizontal.*turns down-left without lifting.*vertical.*hooks left without lifting.*final middle horizontal.*People's Republic of China stroke order.*Noto Sans SC.*all three joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 谢 traces 讠-before-身-before-寸 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_THANK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%B0%A2.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 谢\.json.*medians 1–12.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /twelve ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–9.*身.*upper falling stroke.*left side.*top horizontal.*right side.*hooks left.*two inner horizontals.*wide lower horizontal.*lower falling stroke down-left.*Medians 10–12.*寸.*horizontal.*vertical.*hooks left.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*all five internal turns.*eleven intervening lifts/i,
    );
  });

  it("Chinese 请 traces 讠-before-青 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_PLEASE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AF%B7.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 请\.json.*medians 1–10.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /ten ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–10.*青.*two upper horizontals.*central vertical.*wide middle horizontal.*lower left side.*lower top horizontal.*right side.*hooks left.*two inner horizontals.*People's Republic of China stroke order.*Noto Sans SC.*all four internal turns.*nine intervening lifts/i,
    );
  });

  it("Chinese 再 traces its frame-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_AGAIN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%86%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 再\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*top horizontal left-to-right.*left side.*Median 3.*frame's top.*right side.*hooks left.*central vertical.*inner horizontal.*closes with the long bottom horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both turns.*close-last rule.*five intervening lifts/i,
    );
  });

  it("Chinese 见 traces its frame-before-legs order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SEE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%A7%81.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 见\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*left side.*Median 2.*top horizontal.*right side.*left-falling leg.*Median 4.*second leg.*bends right.*upward hook.*People's Republic of China stroke order.*Noto Sans SC.*frame-before-legs.*three joined turns.*three intervening lifts/i,
    );
  });

  it("Chinese 什 traces its 亻-before-十 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_WHAT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BB%80.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 什\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*十's horizontal left-to-right.*median 4.*descends 十's vertical.*People's Republic of China stroke order.*Noto Sans SC.*亻-before-十.*three intervening lifts/i,
    );
  });

  it("Chinese 么 traces its joined lower sweep to the pinned PRC-order dataset", () => {
    const src = CHINESE_PARTICLE_ME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B9%88.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 么\.json.*medians 1–3.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*upper left-falling stroke.*Median 2.*upper right.*falls down-left.*turns without lifting.*sweeps right along the base.*Median 3.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*second stroke's joined turn.*two intervening lifts/i,
    );
  });

  it("Chinese 早 traces its complete 日-before-十 order to the pinned PRC dataset", () => {
    const src = CHINESE_EARLY.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A9.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 早\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–4.*complete 日 first.*left side.*top horizontal.*turns down the right side.*middle horizontal.*closing bottom horizontal.*Median 5.*十's horizontal left-to-right.*median 6.*descends its vertical.*People's Republic of China stroke order.*Noto Sans SC.*日-before-十.*joined top-right turn.*five intervening lifts/i,
    );
  });

  it("Chinese 上 traces its vertical-and-horizontals order to the pinned PRC dataset", () => {
    const src = CHINESE_UP.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8A.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 上\.json.*medians 1–3.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*central vertical.*top toward the base.*Median 2.*starts at the vertical.*short middle horizontal left-to-right.*Median 3.*long base horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both intervening lifts.*short-before-long horizontal contrast/i,
    );
  });

  it("Chinese 汉 preserves all five pinned source medians and their lift boundaries", () => {
    const src = CHINESE_HAN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%B1%89.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 汉\.json.*medians 1–5.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /five ordered strokes.*Medians 1–3.*氵.*median 4.*turns down-left.*median 5.*falls down-right.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_HAN.strokes).toHaveLength(5);
    expect(CHINESE_HAN.strokes.map((stroke) => penPath(stroke).length)).toEqual(
      [3, 3, 5, 13, 7],
    );
    expect(penLifts(CHINESE_HAN)).toBe(4);
  });

  it("Chinese 语 preserves the 讠-before-五-before-口 source sequence", () => {
    const src = CHINESE_LANGUAGE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AF%AD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 语\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–2.*讠.*medians 3–6.*五.*medians 7–9.*口.*bottom-closing.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_LANGUAGE.strokes).toHaveLength(9);
    expect(
      CHINESE_LANGUAGE.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([3, 10, 5, 4, 7, 6, 3, 7, 5]);
    expect(penLifts(CHINESE_LANGUAGE)).toBe(8);
  });

  it("Chinese 文 preserves its four separately started source medians", () => {
    const src = CHINESE_WRITING.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%96%87.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 文\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four separately started strokes.*top dot.*horizontal.*left-falling.*right-falling.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_WRITING.strokes).toHaveLength(4);
    expect(
      CHINESE_WRITING.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([3, 6, 12, 7]);
    expect(penLifts(CHINESE_WRITING)).toBe(3);
  });

  it("Chinese 国 closes the frame only after all five inner 玉 medians", () => {
    const src = CHINESE_COUNTRY.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%9B%BD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 国\.json.*medians 1–8.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /eight ordered strokes.*Medians 1–2.*outer frame.*medians 3–7.*玉.*median 8.*closes.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_COUNTRY.strokes).toHaveLength(8);
    expect(
      CHINESE_COUNTRY.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([6, 12, 6, 4, 5, 5, 3, 5]);
    expect(penLifts(CHINESE_COUNTRY)).toBe(7);
  });

  it("Chinese 看 preserves the four upper medians before all five 目 medians", () => {
    const src = CHINESE_LOOK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E7%9C%8B.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 看\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–4.*upper hand-like.*medians 5–9.*目.*People's Republic of China.*ordered sequence.*fitting.*Noto Sans SC/i,
    );
    expect(CHINESE_LOOK.strokes).toHaveLength(9);
    expect(
      CHINESE_LOOK.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([7, 5, 5, 9, 6, 9, 5, 5, 5]);
    expect(penLifts(CHINESE_LOOK)).toBe(8);
  });

  it("Chinese 书 preserves both folds before the upright and final dot", () => {
    const src = CHINESE_BOOK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B9%A6.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 书\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*two upper folding strokes.*central upright.*final upper-right dot.*People's Republic of China.*ordered sequence.*fitting.*Noto Sans SC/i,
    );
    expect(CHINESE_BOOK.strokes).toHaveLength(4);
    expect(
      CHINESE_BOOK.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([8, 13, 6, 5]);
    expect(penLifts(CHINESE_BOOK)).toBe(3);
  });
});
