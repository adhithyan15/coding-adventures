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

const CYRILLIC_A = DUCTUS[ductusKey("cyrillic", "а")];
const CYRILLIC_BE = DUCTUS[ductusKey("cyrillic", "б")];
const CYRILLIC_VE = DUCTUS[ductusKey("cyrillic", "в")];
const CYRILLIC_GE = DUCTUS[ductusKey("cyrillic", "г")];
const CYRILLIC_DE = DUCTUS[ductusKey("cyrillic", "д")];
const CYRILLIC_IE = DUCTUS[ductusKey("cyrillic", "е")];
const CYRILLIC_IO = DUCTUS[ductusKey("cyrillic", "ё")];
const CYRILLIC_ZHE = DUCTUS[ductusKey("cyrillic", "ж")];
const CYRILLIC_ZE = DUCTUS[ductusKey("cyrillic", "з")];
const CYRILLIC_I = DUCTUS[ductusKey("cyrillic", "и")];
const CYRILLIC_SHORT_I = DUCTUS[ductusKey("cyrillic", "й")];
const CYRILLIC_KA = DUCTUS[ductusKey("cyrillic", "к")];
const CYRILLIC_EL = DUCTUS[ductusKey("cyrillic", "л")];
const CYRILLIC_EM = DUCTUS[ductusKey("cyrillic", "м")];
const CYRILLIC_EN = DUCTUS[ductusKey("cyrillic", "н")];
const CYRILLIC_O = DUCTUS[ductusKey("cyrillic", "о")];
const CYRILLIC_PE = DUCTUS[ductusKey("cyrillic", "п")];
const CYRILLIC_ER = DUCTUS[ductusKey("cyrillic", "р")];
const CYRILLIC_ES = DUCTUS[ductusKey("cyrillic", "с")];
const CYRILLIC_TE = DUCTUS[ductusKey("cyrillic", "т")];
const CYRILLIC_U = DUCTUS[ductusKey("cyrillic", "у")];
const CYRILLIC_EF = DUCTUS[ductusKey("cyrillic", "ф")];
const CYRILLIC_HA = DUCTUS[ductusKey("cyrillic", "х")];
const CYRILLIC_TSE = DUCTUS[ductusKey("cyrillic", "ц")];
const CYRILLIC_CHE = DUCTUS[ductusKey("cyrillic", "ч")];
const CYRILLIC_SHA = DUCTUS[ductusKey("cyrillic", "ш")];
const CYRILLIC_SHCHA = DUCTUS[ductusKey("cyrillic", "щ")];
const CYRILLIC_HARD_SIGN = DUCTUS[ductusKey("cyrillic", "ъ")];
const CYRILLIC_YERY = DUCTUS[ductusKey("cyrillic", "ы")];
const CYRILLIC_SOFT_SIGN = DUCTUS[ductusKey("cyrillic", "ь")];
const CYRILLIC_E = DUCTUS[ductusKey("cyrillic", "э")];
const CYRILLIC_YU = DUCTUS[ductusKey("cyrillic", "ю")];
const CYRILLIC_YA = DUCTUS[ductusKey("cyrillic", "я")];

const OWNER_SCRIPTS = new Set(["cyrillic"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, {});

  beforeAll(() => {
    expect(verifiedLetterFont("а", CYRILLIC_A.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("б", CYRILLIC_BE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("в", CYRILLIC_VE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("г", CYRILLIC_GE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("д", CYRILLIC_DE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("е", CYRILLIC_IE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("ё", CYRILLIC_IO.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("ж", CYRILLIC_ZHE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("з", CYRILLIC_ZE.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("и", CYRILLIC_I.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
    expect(verifiedLetterFont("й", CYRILLIC_SHORT_I.source.url)).toBe(
      "_fonts/NotoSansCyrillic-Static.ttf",
    );
  });

  it("marks Cyrillic complete with the 33 sourced lowercase Russian letters", () => {
    const cyrillic = SCRIPTS.find((script) => script.script === "cyrillic")!;
    expect(cyrillic.complete).toBe(true);
    expect(cyrillic.letters).toHaveLength(33);
    expect(new Set(cyrillic.letters.map((letter) => letter.glyph)).size).toBe(
      33,
    );
    expect(
      cyrillic.letters.every(
        (letter) => letter.strokeOrderSource !== undefined,
      ),
    ).toBe(true);
  });

  it("Cyrillic а keeps its shoulder, round body, and finishing stem in one run", () => {
    expect(CYRILLIC_A.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_A)).toBe(0);
    expect(CYRILLIC_A.strokes).toHaveLength(1);
    expect(CYRILLIC_A.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_A.strokes[0].segments[0].path;
    const stem = CYRILLIC_A.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(stem[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(
      body[0].y,
    );
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
  });

  it("Cyrillic б closes its lower body before rising into the top flag", () => {
    expect(CYRILLIC_BE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_BE)).toBe(0);
    expect(CYRILLIC_BE.strokes).toHaveLength(1);
    expect(CYRILLIC_BE.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_BE.strokes[0].segments[0].path;
    const flag = CYRILLIC_BE.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(flag[0]);
    expect(body.at(-1)).toEqual(body[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(flag.at(-1)!.y).toBeGreaterThan(flag[0].y);
    expect(flag.at(-1)!.x).toBeGreaterThan(flag[0].x);
  });

  it("Cyrillic в returns from its upper loop before circling the lower bowl", () => {
    expect(CYRILLIC_VE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_VE)).toBe(0);
    expect(CYRILLIC_VE.strokes).toHaveLength(1);
    expect(CYRILLIC_VE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_VE.strokes[0].segments[0].path;
    const lower = CYRILLIC_VE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(upper.at(-1)).toEqual(upper[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      upper[0].y,
    );
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(
      lower[0].x,
    );
    expect(lower.at(-1)!.y).toBeGreaterThan(lower[0].y);
  });

  it("Cyrillic г climbs, retraces its top bar, and descends without lifting", () => {
    expect(CYRILLIC_GE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_GE)).toBe(0);
    expect(CYRILLIC_GE.strokes).toHaveLength(1);
    expect(CYRILLIC_GE.strokes[0].segments).toHaveLength(2);
    const outward = CYRILLIC_GE.strokes[0].segments[0].path;
    const returnPath = CYRILLIC_GE.strokes[0].segments[1].path;
    expect(outward.at(-1)).toEqual(returnPath[0]);
    expect(outward[0].y).toBeLessThan(outward.at(-1)!.y);
    expect(outward[0].x).toBeLessThan(outward.at(-1)!.x);
    expect(returnPath.at(-1)).toEqual(outward[0]);
  });

  it("Cyrillic д closes its body before retracing both feet without lifting", () => {
    expect(CYRILLIC_DE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_DE)).toBe(0);
    expect(CYRILLIC_DE.strokes).toHaveLength(1);
    expect(CYRILLIC_DE.strokes[0].segments).toHaveLength(2);
    const body = CYRILLIC_DE.strokes[0].segments[0].path;
    const base = CYRILLIC_DE.strokes[0].segments[1].path;
    expect(body.at(-1)).toEqual(base[0]);
    expect(body.at(-1)).toEqual(body[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...base.map((point) => point.y))).toBeLessThan(
      Math.min(...body.map((point) => point.y)),
    );
    expect(Math.min(...base.map((point) => point.x))).toBeLessThan(body[0].x);
  });

  it("Cyrillic е crosses the middle before continuing around its lower bowl", () => {
    expect(CYRILLIC_IE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_IE)).toBe(0);
    expect(CYRILLIC_IE.strokes).toHaveLength(1);
    expect(CYRILLIC_IE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_IE.strokes[0].segments[0].path;
    const lower = CYRILLIC_IE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      upper[0].y,
    );
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(
      Math.min(...upper.map((point) => point.y)),
    );
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x - 100);
  });

  it("Cyrillic ё completes its body before two separately lifted dots", () => {
    expect(CYRILLIC_IO.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_IO)).toBe(2);
    expect(CYRILLIC_IO.strokes).toHaveLength(3);
    expect(CYRILLIC_IO.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1, 1],
    );
    expect(penPath(CYRILLIC_IO.strokes[0])).toEqual(
      penPath(CYRILLIC_IE.strokes[0]),
    );
    const leftDot = penPath(CYRILLIC_IO.strokes[1]);
    const rightDot = penPath(CYRILLIC_IO.strokes[2]);
    expect(leftDot[0].x).toBeLessThan(rightDot[0].x);
    expect(leftDot[0].y).toBe(rightDot[0].y);
    expect(leftDot[0].y).toBeGreaterThan(
      Math.max(...penPath(CYRILLIC_IO.strokes[0]).map((point) => point.y)),
    );
  });

  it("Cyrillic ж traces both wings through one continuous central run", () => {
    expect(CYRILLIC_ZHE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ZHE)).toBe(0);
    expect(CYRILLIC_ZHE.strokes).toHaveLength(1);
    expect(CYRILLIC_ZHE.strokes[0].segments).toHaveLength(2);
    const left = CYRILLIC_ZHE.strokes[0].segments[0].path;
    const right = CYRILLIC_ZHE.strokes[0].segments[1].path;
    expect(left.at(-1)).toEqual(right[0]);
    expect(Math.min(...left.map((point) => point.x))).toBeLessThan(right[0].x);
    expect(Math.max(...right.map((point) => point.x))).toBeGreaterThan(
      left.at(-1)!.x,
    );
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(right.at(-1)!.y).toBeLessThan(right[0].y);
  });

  it("Cyrillic з joins its smaller upper lobe to its larger lower lobe", () => {
    expect(CYRILLIC_ZE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ZE)).toBe(0);
    expect(CYRILLIC_ZE.strokes).toHaveLength(1);
    expect(CYRILLIC_ZE.strokes[0].segments).toHaveLength(2);
    const upper = CYRILLIC_ZE.strokes[0].segments[0].path;
    const lower = CYRILLIC_ZE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lower.map((point) => point.y)),
    );
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(
      Math.min(...upper.map((point) => point.y)),
    );
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(
      lower[0].x,
    );
  });

  it("Cyrillic и joins its two descending stems with a rising diagonal", () => {
    expect(CYRILLIC_I.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_I)).toBe(0);
    expect(CYRILLIC_I.strokes).toHaveLength(1);
    expect(CYRILLIC_I.strokes[0].segments).toHaveLength(3);
    const left = CYRILLIC_I.strokes[0].segments[0].path;
    const diagonal = CYRILLIC_I.strokes[0].segments[1].path;
    const right = CYRILLIC_I.strokes[0].segments[2].path;
    expect(left.at(-1)).toEqual(diagonal[0]);
    expect(diagonal.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(diagonal[0].y).toBeLessThan(diagonal.at(-1)!.y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
  });

  it("Cyrillic й completes its joined body before the lifted breve", () => {
    expect(CYRILLIC_SHORT_I.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHORT_I)).toBe(1);
    expect(CYRILLIC_SHORT_I.strokes).toHaveLength(2);
    expect(
      CYRILLIC_SHORT_I.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([3, 1]);
    const body = CYRILLIC_SHORT_I.strokes[0].segments;
    expect(body[0].path.at(-1)).toEqual(body[1].path[0]);
    expect(body[1].path.at(-1)).toEqual(body[2].path[0]);
    const breve = CYRILLIC_SHORT_I.strokes[1].segments[0].path;
    expect(breve[0].x).toBeLessThan(breve.at(-1)!.x);
    expect(Math.min(...breve.map((point) => point.y))).toBeLessThan(breve[0].y);
    expect(Math.min(...breve.map((point) => point.y))).toBeLessThan(
      breve.at(-1)!.y,
    );
  });

  it("Cyrillic к joins its descending stem to the upper and lower arms", () => {
    expect(CYRILLIC_KA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_KA)).toBe(0);
    expect(CYRILLIC_KA.strokes).toHaveLength(1);
    expect(CYRILLIC_KA.strokes[0].segments).toHaveLength(3);
    const [stem, upper, lower] = CYRILLIC_KA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      upper.at(-1)!.y,
    );
    expect(Math.max(...upper.map((point) => point.x))).toBeGreaterThan(
      upper.at(-1)!.x,
    );
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x);
    expect(lower.at(-1)!.y).toBeLessThan(lower[0].y);
  });

  it("Cyrillic л joins its hooked left leg to the top shoulder and right stem", () => {
    expect(CYRILLIC_EL.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EL)).toBe(0);
    expect(CYRILLIC_EL.strokes).toHaveLength(1);
    expect(CYRILLIC_EL.strokes[0].segments).toHaveLength(3);
    const [left, shoulder, right] = CYRILLIC_EL.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(left[0].x).toBeLessThan(left.at(-1)!.x);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic м joins its two apexes through the central valley", () => {
    expect(CYRILLIC_EM.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EM)).toBe(0);
    expect(CYRILLIC_EM.strokes).toHaveLength(1);
    expect(CYRILLIC_EM.strokes[0].segments).toHaveLength(4);
    const [left, down, up, right] = CYRILLIC_EM.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(down[0]);
    expect(down.at(-1)).toEqual(up[0]);
    expect(up.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeLessThan(left.at(-1)!.y);
    expect(down.at(-1)!.y).toBeLessThan(down[0].y);
    expect(up.at(-1)!.y).toBeGreaterThan(up[0].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic н joins its two stems through the middle bridge", () => {
    expect(CYRILLIC_EN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EN)).toBe(0);
    expect(CYRILLIC_EN.strokes).toHaveLength(1);
    expect(CYRILLIC_EN.strokes[0].segments).toHaveLength(3);
    const [left, bridge, right] = CYRILLIC_EN.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(bridge[0]);
    expect(bridge.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(Math.min(...bridge.map((point) => point.y))).toBe(left.at(-1)!.y);
    expect(bridge.some((point) => point.x > left[0].x && point.y === 274)).toBe(
      true,
    );
    expect(bridge.at(-1)!.y).toBeGreaterThan(bridge[0].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic о closes one counterclockwise oval without lifting", () => {
    expect(CYRILLIC_O.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_O)).toBe(0);
    expect(CYRILLIC_O.strokes).toHaveLength(1);
    expect(CYRILLIC_O.strokes[0].segments).toHaveLength(2);
    const [left, right] = CYRILLIC_O.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(left[0]);
    expect(Math.min(...left.map((point) => point.x))).toBeLessThan(left[0].x);
    expect(Math.max(...left.map((point) => point.y))).toBeGreaterThan(
      left[0].y,
    );
    expect(Math.min(...left.map((point) => point.y))).toBeLessThan(left[0].y);
    expect(Math.max(...right.map((point) => point.x))).toBeGreaterThan(
      left[0].x,
    );
  });

  it("Cyrillic п joins its two stems through the top shoulder", () => {
    expect(CYRILLIC_PE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_PE)).toBe(0);
    expect(CYRILLIC_PE.strokes).toHaveLength(1);
    expect(CYRILLIC_PE.strokes[0].segments).toHaveLength(3);
    const [left, shoulder, right] = CYRILLIC_PE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(shoulder.at(-1)!.y).toBe(shoulder[4].y);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic р joins its descender to one clockwise printed bowl", () => {
    expect(CYRILLIC_ER.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ER)).toBe(0);
    expect(CYRILLIC_ER.strokes).toHaveLength(1);
    expect(CYRILLIC_ER.strokes[0].segments).toHaveLength(3);
    const [stem, shoulder, bowl] = CYRILLIC_ER.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)).toEqual(bowl[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(Math.max(...shoulder.map((point) => point.y))).toBeGreaterThan(
      shoulder[0].y,
    );
    expect(Math.max(...bowl.map((point) => point.x))).toBeGreaterThan(
      bowl.at(-1)!.x,
    );
    expect(bowl.at(-1)!.x).toBe(stem[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(0);
  });

  it("Cyrillic с keeps one counterclockwise curve open on the right", () => {
    expect(CYRILLIC_ES.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_ES)).toBe(0);
    expect(CYRILLIC_ES.strokes).toHaveLength(1);
    expect(CYRILLIC_ES.strokes[0].segments).toHaveLength(2);
    const [upper, lower] = CYRILLIC_ES.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(upper[0].x).toBeGreaterThan(
      Math.min(...upper.map((point) => point.x)),
    );
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      upper[0].y,
    );
    expect(lower.at(-1)!.x).toBeGreaterThan(
      Math.min(...lower.map((point) => point.x)),
    );
    expect(lower.at(-1)!.y).toBeLessThan(upper[0].y);
    expect(lower.at(-1)).not.toEqual(upper[0]);
  });

  it("Cyrillic т joins its central stem to the full printed top bar", () => {
    expect(CYRILLIC_TE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_TE)).toBe(0);
    expect(CYRILLIC_TE.strokes).toHaveLength(1);
    expect(CYRILLIC_TE.strokes[0].segments).toHaveLength(3);
    const [stem, left, right] = CYRILLIC_TE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(left[0]);
    expect(left.at(-1)).toEqual(right[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(left.at(-1)!.x).toBeLessThan(stem[0].x);
    expect(right.at(-1)!.x).toBeGreaterThan(stem[0].x);
    expect(Math.max(...right.map((point) => point.y))).toBe(
      Math.min(...right.map((point) => point.y)),
    );
  });

  it("Cyrillic у joins both upper arms to its long curved descender", () => {
    expect(CYRILLIC_U.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_U)).toBe(0);
    expect(CYRILLIC_U.strokes).toHaveLength(1);
    expect(CYRILLIC_U.strokes[0].segments).toHaveLength(4);
    const [left, right, tail, terminal] = CYRILLIC_U.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(tail[0]);
    expect(tail.at(-1)).toEqual(terminal[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
    expect(terminal.at(-1)!.x).toBeLessThan(terminal[0].x);
  });

  it("Cyrillic ф draws the central stem before its joined printed bowls", () => {
    expect(CYRILLIC_EF.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_EF)).toBe(1);
    expect(CYRILLIC_EF.strokes).toHaveLength(2);
    expect(CYRILLIC_EF.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 4],
    );
    const stem = CYRILLIC_EF.strokes[0].segments[0].path;
    const [leftTop, leftBottom, rightBottom, rightTop] =
      CYRILLIC_EF.strokes[1].segments.map((segment) => segment.path);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(leftTop.at(-1)).toEqual(leftBottom[0]);
    expect(leftBottom.at(-1)).toEqual(rightBottom[0]);
    expect(rightBottom.at(-1)).toEqual(rightTop[0]);
    expect(Math.min(...leftTop.map((point) => point.x))).toBeLessThan(
      stem[0].x,
    );
    expect(Math.max(...rightTop.map((point) => point.x))).toBeGreaterThan(
      stem[0].x,
    );
  });

  it("Cyrillic х draws the left curved run before the crossing right run", () => {
    expect(CYRILLIC_HA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_HA)).toBe(1);
    expect(CYRILLIC_HA.strokes).toHaveLength(2);
    expect(CYRILLIC_HA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 2],
    );
    const [leftTop, leftBottom] = CYRILLIC_HA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const [rightTop, rightBottom] = CYRILLIC_HA.strokes[1].segments.map(
      (segment) => segment.path,
    );
    expect(leftTop.at(-1)).toEqual(leftBottom[0]);
    expect(rightTop.at(-1)).toEqual(rightBottom[0]);
    expect(leftTop.at(-1)).toEqual(rightTop.at(-1));
    expect(leftTop[0].x).toBeLessThan(rightTop[0].x);
    expect(leftBottom.at(-1)!.x).toBeLessThan(rightBottom.at(-1)!.x);
  });

  it("Cyrillic ц keeps both stems, the base, and the tail in one run", () => {
    expect(CYRILLIC_TSE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_TSE)).toBe(0);
    expect(CYRILLIC_TSE.strokes).toHaveLength(1);
    expect(CYRILLIC_TSE.strokes[0].segments).toHaveLength(4);
    const [left, rise, retrace, tail] = CYRILLIC_TSE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(rise[0]);
    expect(rise.at(-1)).toEqual(retrace[0]);
    expect(retrace.at(-1)).toEqual(tail[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(rise.at(-1)!.y).toBeGreaterThan(rise[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ч keeps the short stem, bowl, and full stem in one run", () => {
    expect(CYRILLIC_CHE.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_CHE)).toBe(0);
    expect(CYRILLIC_CHE.strokes).toHaveLength(1);
    expect(CYRILLIC_CHE.strokes[0].segments).toHaveLength(3);
    const [left, bowl, right] = CYRILLIC_CHE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(bowl[0]);
    expect(bowl.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
    expect(right.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ш keeps three stems and two base joins in one run", () => {
    expect(CYRILLIC_SHA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHA)).toBe(0);
    expect(CYRILLIC_SHA.strokes).toHaveLength(1);
    expect(CYRILLIC_SHA.strokes[0].segments).toHaveLength(5);
    const [left, firstRise, middle, secondRise, right] =
      CYRILLIC_SHA.strokes[0].segments.map((segment) => segment.path);
    expect(left.at(-1)).toEqual(firstRise[0]);
    expect(firstRise.at(-1)).toEqual(middle[0]);
    expect(middle.at(-1)).toEqual(secondRise[0]);
    expect(secondRise.at(-1)).toEqual(right[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(firstRise.at(-1)!.y).toBeGreaterThan(firstRise[0].y);
    expect(secondRise.at(-1)!.y).toBeGreaterThan(secondRise[0].y);
    expect(right.at(-1)!.y).toBeLessThan(right[0].y);
  });

  it("Cyrillic щ keeps three stems, two joins, and the tail in one run", () => {
    expect(CYRILLIC_SHCHA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SHCHA)).toBe(0);
    expect(CYRILLIC_SHCHA.strokes).toHaveLength(1);
    expect(CYRILLIC_SHCHA.strokes[0].segments).toHaveLength(6);
    const [left, firstRise, middle, secondRise, right, tail] =
      CYRILLIC_SHCHA.strokes[0].segments.map((segment) => segment.path);
    expect(left.at(-1)).toEqual(firstRise[0]);
    expect(firstRise.at(-1)).toEqual(middle[0]);
    expect(middle.at(-1)).toEqual(secondRise[0]);
    expect(secondRise.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(tail[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(firstRise.at(-1)!.y).toBeGreaterThan(firstRise[0].y);
    expect(secondRise.at(-1)!.y).toBeGreaterThan(secondRise[0].y);
    expect(tail.at(-1)!.y).toBeLessThan(left.at(-1)!.y);
  });

  it("Cyrillic ъ keeps its flag, stem, and lower bowl in one run", () => {
    expect(CYRILLIC_HARD_SIGN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_HARD_SIGN)).toBe(0);
    expect(CYRILLIC_HARD_SIGN.strokes).toHaveLength(1);
    expect(CYRILLIC_HARD_SIGN.strokes[0].segments).toHaveLength(5);
    const [flag, stem, lower, right, upper] =
      CYRILLIC_HARD_SIGN.strokes[0].segments.map((segment) => segment.path);
    expect(flag.at(-1)).toEqual(stem[0]);
    expect(stem.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(flag[0].x).toBeLessThan(flag.at(-1)!.x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
  });

  it("Cyrillic ы keeps the left stem and bowl joined before the lifted right stem", () => {
    expect(CYRILLIC_YERY.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YERY)).toBe(1);
    expect(CYRILLIC_YERY.strokes).toHaveLength(2);
    expect(
      CYRILLIC_YERY.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([4, 1]);
    const [left, lower, right, upper] = CYRILLIC_YERY.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const separate = CYRILLIC_YERY.strokes[1].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
    expect(separate[0].y).toBeGreaterThan(separate.at(-1)!.y);
    expect(separate[0].x).toBeGreaterThan(right.at(-1)!.x);
  });

  it("Cyrillic ь keeps the stem and lower bowl in one continuous run", () => {
    expect(CYRILLIC_SOFT_SIGN.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_SOFT_SIGN)).toBe(0);
    expect(CYRILLIC_SOFT_SIGN.strokes).toHaveLength(1);
    expect(CYRILLIC_SOFT_SIGN.strokes[0].segments).toHaveLength(4);
    const [stem, lower, right, upper] =
      CYRILLIC_SOFT_SIGN.strokes[0].segments.map((segment) => segment.path);
    expect(stem.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(upper[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(lower[0].x).toBeLessThan(lower.at(-1)!.x);
    expect(right.at(-1)!.y).toBeGreaterThan(right[0].y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
  });

  it("Cyrillic э draws the outer curve before the lifted right-to-left tongue", () => {
    expect(CYRILLIC_E.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_E)).toBe(1);
    expect(CYRILLIC_E.strokes).toHaveLength(2);
    expect(CYRILLIC_E.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 1,
    ]);
    const [upper, right, lower] = CYRILLIC_E.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const tongue = CYRILLIC_E.strokes[1].segments[0].path;
    expect(upper.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(lower[0]);
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(lower.at(-1)!.x).toBeLessThan(lower[0].x);
    expect(tongue[0].x).toBeGreaterThan(tongue.at(-1)!.x);
  });

  it("Cyrillic ю joins the left stem and middle bar to the clockwise oval", () => {
    expect(CYRILLIC_YU.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YU)).toBe(0);
    expect(CYRILLIC_YU.strokes).toHaveLength(1);
    expect(CYRILLIC_YU.strokes[0].segments).toHaveLength(5);
    const [stem, connector, upper, right, lower] =
      CYRILLIC_YU.strokes[0].segments.map((segment) => segment.path);
    expect(stem.at(-1)).toEqual(connector[0]);
    expect(connector.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(right[0]);
    expect(right.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(upper[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(connector.at(-1)!.x).toBeGreaterThan(connector[0].x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Cyrillic я joins the rising stem, counterclockwise bowl, and diagonal leg", () => {
    expect(CYRILLIC_YA.script).toBe("cyrillic");
    expect(penLifts(CYRILLIC_YA)).toBe(0);
    expect(CYRILLIC_YA.strokes).toHaveLength(1);
    expect(CYRILLIC_YA.strokes[0].segments).toHaveLength(4);
    const [stem, bowl, join, leg] = CYRILLIC_YA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(stem.at(-1)).toEqual(bowl[0]);
    expect(bowl.at(-1)).toEqual(join[0]);
    expect(join.at(-1)).toEqual(leg[0]);
    expect(stem.at(-1)!.y).toBeGreaterThan(stem[0].y);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(leg.at(-1)!.x).toBeLessThan(leg[0].x);
    expect(leg.at(-1)!.y).toBeLessThan(leg[0].y);
  });

  it("Cyrillic а traces its one-run school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_A.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase а.*00:50–00:55.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*00:50–00:55.*rounded body.*right-hand finishing stem.*one continuous pen-down run.*zero intervening lifts.*single-storey.*Noto Sans Cyrillic.*double-storey printed form.*extra upper shoulder.*one-run body-to-stem motion.*entry into the lower loop.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic б traces its joined body and flag to the all-letter native lesson", () => {
    const src = CYRILLIC_BE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase б.*01:13–01:18.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:13–01:18.*lower body counterclockwise.*upper-right junction.*rising shoulder.*rightward top flag.*one continuous pen-down run.*zero intervening lifts.*crosses diagonally.*bundled Noto Sans Cyrillic.*upper-left shoulder.*uninterrupted body-to-flag order.*printed upper shoulder.*font's ink.*connected cursive.*top flag.*exit join/i,
    );
  });

  it("Cyrillic в traces its upper loop and lower bowl to the all-letter native lesson", () => {
    const src = CYRILLIC_VE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase в.*01:33–01:38.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:33–01:38.*starts at the baseline.*tall upper loop.*descends to the baseline.*counterclockwise.*lower bowl.*one continuous pen-down run.*zero intervening lifts.*looped Latin cursive b.*bundled Noto Sans Cyrillic.*two stacked bowls.*straight left stem.*baseline-to-upper-loop-to-baseline-to-lower-bowl order.*printed upper bowl.*font's ink.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic г traces its zero-lift cursive humps to the all-letter native lesson", () => {
    const src = CYRILLIC_GE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase г.*01:54–01:57.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*01:54–01:57.*rises from the baseline.*main shoulder.*turns at the baseline.*smaller rising-and-falling exit arch.*one continuous pen-down run.*zero intervening lifts.*rounded cursive form.*two humps.*bundled Noto Sans Cyrillic.*block-like printed form.*straight upright.*top bar.*no exit arch.*baseline-to-shoulder-to-baseline lift count.*climb the upright.*reverse to the junction.*descend the upright.*connected cursive restores.*exit arch/i,
    );
  });

  it("Cyrillic д traces its zero-lift body and descender to the all-letter native lesson", () => {
    const src = CYRILLIC_DE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase д.*02:14–02:19.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:14–02:19.*circles the rounded body counterclockwise.*upper-right junction.*descends below the baseline.*loops left.*rightward exit.*one continuous pen-down run.*zero intervening lifts.*looped Latin cursive g.*bundled Noto Sans Cyrillic.*block-like printed form.*trapezoidal body.*joined base shelf.*two separated feet.*body-before-descender order.*circle the body.*right stem and foot.*sweep left.*left foot.*finish rightward.*connected cursive restores.*below-baseline loop.*exit join/i,
    );
  });

  it("Cyrillic е traces its zero-lift looped form to the all-letter native lesson", () => {
    const src = CYRILLIC_IE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase е.*02:26–02:30.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:26–02:30.*begins at the upper right.*curves left around the upper loop.*crosses through the middle.*counterclockwise.*rounded lower bowl.*one continuous pen-down run.*zero intervening lifts.*tall open epsilon-like form.*small crossing loop.*bundled Noto Sans Cyrillic.*compact printed e.*long middle bar.*open right side.*upper-loop-to-middle-to-lower-bowl order.*curve around the upper bowl.*sweep right.*reverse through the junction.*circle the lower bowl.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic ё traces its body and two dots to the all-letter native lesson", () => {
    const src = CYRILLIC_IO.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ё.*02:51–02:56.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*02:51–02:56.*upper-loop-to-middle-to-lower-bowl body.*one continuous pen-down run.*left dot.*right dot.*three strokes.*two intervening lifts.*tall open epsilon-like body.*small crossing loop.*two dots above.*bundled Noto Sans Cyrillic.*compact printed e.*long middle bar.*two circular dots.*body-before-left-dot-before-right-dot order.*curve around the upper bowl.*sweep and reverse.*circle the lower bowl.*left to right.*connected cursive.*dots remain separate.*substitutes е.*always stressed/i,
    );
  });

  it("Cyrillic ж traces its zero-lift wings to the all-letter native lesson", () => {
    const src = CYRILLIC_ZHE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ж.*03:16–03:21.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:16–03:21.*lower left.*left arch.*tall central loop.*descends through the middle.*right arch.*rightward exit.*without lifting.*one continuous pen-down run.*zero intervening lifts.*rounded, looped form.*joined cursive arches.*bundled Noto Sans Cyrillic.*symmetric printed form.*straight central upright.*four diagonal arms.*lower-left-to-centre-to-right order.*lower-left arm.*retrace the upright.*upper-right arm.*return to the centre.*lower-right arm.*connected cursive.*entry and exit joins/i,
    );
  });

  it("Cyrillic з traces its zero-lift lobes to the all-letter native lesson", () => {
    const src = CYRILLIC_ZE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase з.*03:34–03:39.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:34–03:39.*upper left.*smaller upper lobe.*descends through the middle.*larger lower lobe.*curls left along the baseline.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*looped digit 3.*open left side.*exit join.*bundled Noto Sans Cyrillic.*compact printed form.*two open-right lobes.*no exit join.*upper-lobe-to-lower-lobe order.*circle the upper lobe.*middle junction.*circle the lower lobe.*lower-right tip.*connected cursive restores.*small rising exit/i,
    );
  });

  it("Cyrillic и traces its zero-lift stems to the all-letter native lesson", () => {
    const src = CYRILLIC_I.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase и.*03:56–04:02.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*03:56–04:02.*upper left.*descends to the baseline.*rising diagonal.*descends the right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin u.*rising entry side.*exit join.*bundled Noto Sans Cyrillic.*printed backwards-N form.*two straight vertical stems.*no entry or exit joins.*left-stem-to-rising-diagonal-to-right-stem order.*descend the left stem.*baseline.*rise through the diagonal.*descend the right stem.*connected cursive restores.*entry and exit joins/i,
    );
  });

  it("Cyrillic й traces its body and breve to the all-letter native lesson", () => {
    const src = CYRILLIC_SHORT_I.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase й.*04:17–04:24.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*04:17–04:24.*same left-stem-to-rising-diagonal-to-right-stem body.*one continuous pen-down run.*small rising exit.*lifts.*breve above from left to right.*one dipped arc.*two strokes.*one intervening lift.*rounded Latin u.*entry and exit joins.*shallow curved breve.*bundled Noto Sans Cyrillic.*printed backwards-N body.*two straight stems.*separate thicker breve.*body-before-breve order.*left-to-right breve direction.*one-lift evidence.*descend the left stem.*rise through the diagonal.*descend the right stem.*sweep down and up through the breve.*connected cursive restores.*breve remains separate/i,
    );
  });

  it("Cyrillic к traces its zero-lift stem and arms to the all-letter native lesson", () => {
    const src = CYRILLIC_KA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase к.*04:45–04:51.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*04:45–04:51.*upper left.*descends the left stem to the baseline.*looped upper-right arm.*returns to the middle junction.*lower arm.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded, looped upper arm.*entry and exit joins.*bundled Noto Sans Cyrillic.*straight vertical stem.*two angular diagonal arms.*left-stem-to-upper-arm-to-lower-arm order.*zero-lift evidence.*retrace upward through the middle.*upper diagonal out and back.*lower diagonal.*connected cursive restores.*rounded upper loop/i,
    );
  });

  it("Cyrillic л traces its zero-lift legs to the all-letter native lesson", () => {
    const src = CYRILLIC_EL.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase л.*05:06–05:10.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:06–05:10.*near the baseline.*curves left around a small hook.*rises steeply.*high apex.*descends through the right leg.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*pointed apex.*slanted right leg.*entry and exit joins.*bundled Noto Sans Cyrillic.*block-like printed form.*curved left leg.*horizontal top shoulder.*straight right stem.*hooked-left-leg-to-apex-to-right-leg order.*zero-lift evidence.*curve through the left leg.*sweep along the top shoulder.*descend the right stem.*connected cursive restores.*rising exit/i,
    );
  });

  it("Cyrillic м traces its zero-lift arches to the all-letter native lesson", () => {
    const src = CYRILLIC_EM.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase м.*05:26–05:31.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:26–05:31.*near the baseline.*small entry hook.*first apex.*central valley.*second apex.*right leg.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*two rounded arches.*entry and exit joins.*bundled Noto Sans Cyrillic.*angular printed form.*two straight upright stems.*deep central V.*entry-to-first-apex-to-valley-to-second-apex-to-baseline order.*zero-lift evidence.*rise through the left stem.*central V.*descend the right stem.*connected cursive restores.*rounded arches/i,
    );
  });

  it("Cyrillic н traces its zero-lift bridge to the all-letter native lesson", () => {
    const src = CYRILLIC_EN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase н.*05:47–05:52.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:47–05:52.*upper left.*left stem.*baseline.*turns upward without lifting.*rounded middle bridge.*right shoulder.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded bridge.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed H-like form.*two straight vertical stems.*horizontal middle bar.*left-stem-to-middle-bridge-to-right-stem order.*zero-lift evidence.*retrace to the middle junction.*horizontal bar.*upper-right tip.*descend the right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic о traces its zero-lift oval to the all-letter native lesson", () => {
    const src = CYRILLIC_O.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase о.*05:59–06:03.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*05:59–06:03.*upper right.*left across the top.*left side.*bottom.*right side.*closes the oval.*one continuous counterclockwise pen-down run.*zero intervening lifts.*tall, slightly slanted oval.*bundled Noto Sans Cyrillic.*wider, upright printed oval.*upper-right-to-left-side-to-bottom-to-right-side closure order.*zero-lift evidence.*closed counterclockwise oval.*connected cursive may add.*entry and exit joins/i,
    );
  });

  it("Cyrillic п traces its zero-lift shoulder to the all-letter native lesson", () => {
    const src = CYRILLIC_PE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase п.*06:26–06:31.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*06:26–06:31.*upper left.*left stem.*baseline.*turns upward without lifting.*rounded top shoulder.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin n.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed squared arch.*two straight vertical stems.*horizontal top bar.*left-stem-to-top-shoulder-to-right-stem order.*zero-lift evidence.*retrace to the upper-left junction.*top bar.*descend the right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic р traces its zero-lift descender and bowl to the all-letter native lesson", () => {
    const src = CYRILLIC_ER.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase р.*06:46–06:52.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*06:46–06:52.*upper left.*descends below the baseline.*retraces upward without lifting.*rounded shoulder.*descends to the baseline.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*long-descender Latin p.*open rounded bowl.*entry and exit joins.*bundled Noto Sans Cyrillic.*printed p-like form.*straight descender stem.*closed rounded bowl.*stem-before-bowl order.*zero-lift evidence.*retrace to the upper-left shoulder.*circle the upper bowl clockwise.*return to the middle junction without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic с traces its zero-lift open curve to the all-letter native lesson", () => {
    const src = CYRILLIC_ES.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase с.*07:04–07:08.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:04–07:08.*upper right.*left across the top.*left side.*bottom.*lower-right exit.*one continuous counterclockwise pen-down run.*zero intervening lifts.*tall, slightly slanted open curve.*small rising exit.*bundled Noto Sans Cyrillic.*wider, upright printed C-like form.*blunt open tips.*upper-right-to-left-side-to-bottom-to-lower-right order.*zero-lift evidence.*one open counterclockwise curve.*connected cursive may add.*entry join.*exit join/i,
    );
  });

  it("Cyrillic т traces its zero-lift m-like school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_TE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase т.*07:29–07:36.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:29–07:36.*upper left.*left stem.*turns upward without lifting.*first arch.*middle stem.*second arch.*right stem.*small rising exit.*one continuous pen-down run.*zero intervening lifts.*rounded Latin m.*two arches.*rising exit.*bundled Noto Sans Cyrillic.*printed T-shaped form.*one central vertical stem.*horizontal top bar.*initial-descent-before-joined-top-movements order.*zero-lift evidence.*descend the central stem.*retrace to the top junction.*sweep left along the top bar.*retrace through the junction.*right tip without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic у traces its zero-lift loop-descender school hand to the all-letter native lesson", () => {
    const src = CYRILLIC_U.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase у.*07:50–07:55.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*07:50–07:55.*upper left.*descends to the baseline.*turns upward without lifting.*right arm.*retraces down.*long descender.*curls left.*lower loop.*crosses the descender.*short rightward exit.*one continuous pen-down run.*zero intervening lifts.*loop-descender Latin y.*narrow rounded upper body.*rising exit.*bundled Noto Sans Cyrillic.*printed y-like form.*two straight upper arms.*broad descender.*curves left without a loop or exit join.*left-arm-to-right-arm-to-descender order.*zero-lift evidence.*descend the left arm.*rise through the right arm.*retrace to the junction.*long descender.*curve left through its terminal without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ф traces its stem-first linked loops to the all-letter native lesson", () => {
    const src = CYRILLIC_EF.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ф.*08:16–08:26.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*08:16–08:26.*first descends the long central stem.*upper line.*below the baseline.*lifts once.*upper central junction.*narrow left loop.*crosses the stem.*narrow right loop.*small rising exit.*two pen-down runs.*one intervening lift.*stem first.*linked left-to-right looped body.*bundled Noto Sans Cyrillic.*printed phi-like form.*straight central ascender-descender.*two wider upright bowls.*without an exit join.*stem-before-left-loop-before-right-loop order.*one-lift evidence.*descend the central stem.*lift.*trace the left bowl.*central junction.*trace the right bowl.*connected cursive restores/i,
    );
  });

  it("Cyrillic х traces its two facing curves to the all-letter native lesson", () => {
    const src = CYRILLIC_HA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase х.*08:42–08:49.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*08:42–08:49.*upper left.*curves right.*middle crossing.*sweeps left.*lower terminal.*lifts once.*second run.*upper right.*curves left.*same crossing.*sweeps right.*small rising exit.*two facing curved runs.*one intervening lift.*right-bulging left stroke.*left-bulging right stroke.*bundled Noto Sans Cyrillic.*printed X-like form.*four straight diagonal arms.*centre.*no entry or exit joins.*upper-left-run-before-upper-right-run order.*top-to-bottom directions.*crossing.*one-lift evidence.*two straight left arms.*lift.*two straight right arms.*connected cursive restores/i,
    );
  });

  it("Cyrillic ц traces its joined body and tail to the all-letter native lesson", () => {
    const src = CYRILLIC_TSE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ц.*09:05–09:10.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:05–09:10.*upper left.*descends the left stem.*rounded baseline join.*rises through the right stem.*descends that stem.*small lower loop.*rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*joined diagonal up.*right stem down.*looped tail.*bundled Noto Sans Cyrillic.*printed squared U-like form.*two straight verticals.*horizontal bottom bar.*short separate-looking right descender.*without rounded joins or an exit loop.*left-stem-to-right-stem-to-tail order.*zero-lift evidence.*descend the left stem.*bottom bar.*rise and retrace the right stem.*tail shoulder.*descend the short tail without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ч traces its joined bowl and stems to the all-letter native lesson", () => {
    const src = CYRILLIC_CHE.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ч.*09:24–09:28.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:24–09:28.*upper left.*descends the short left stem.*middle join.*narrow rounded bridge.*rises to the top of the right stem.*descends that full stem.*baseline.*small rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*short left stem down.*joined rise.*long right stem down.*curled exit.*bundled Noto Sans Cyrillic.*printed ч-like form.*shorter straight left stem.*shallow rounded bowl.*full-height straight right stem.*without the narrow bridge.*curled baseline.*exit join.*short-stem-to-bowl-to-long-stem order.*zero-lift evidence.*descend the short left stem.*shallow bowl.*rise along the right stem.*descend the full right stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ш traces its joined three-stem order to the all-letter native lesson", () => {
    const src = CYRILLIC_SHA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ш.*09:49–09:57.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*09:49–09:57.*upper left.*descends the left stem.*baseline.*rounded diagonal join.*top of the middle stem.*descends the middle stem.*second rounded diagonal join.*top of the right stem.*descends the right stem.*small rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*first joined rise.*middle stem down.*second joined rise.*right stem down.*curled exit.*bundled Noto Sans Cyrillic.*printed ш-like form.*three straight full-height vertical stems.*two horizontal baseline bars.*without diagonal rounded joins.*curled baselines.*exit join.*left-to-middle-to-right order.*zero-lift evidence.*descend the left stem.*first bottom bar.*rise then retrace the middle stem.*second bottom bar.*rise then retrace the right stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic щ traces its joined three-stem-to-tail order to the native lesson", () => {
    const src = CYRILLIC_SHCHA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase щ.*10:17–10:25.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:17–10:25.*upper left.*descends the left stem.*baseline.*rounded diagonal join.*top of the middle stem.*descends the middle stem.*second rounded diagonal join.*top of the right stem.*descends the right stem.*small lower tail loop.*rising exit without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*first joined rise.*middle stem down.*second joined rise.*right stem down.*looped tail.*bundled Noto Sans Cyrillic.*printed щ-like form.*three straight full-height vertical stems.*two horizontal baseline bars.*short separate-looking right descender.*without diagonal rounded joins.*exit loop.*left-to-middle-to-right-to-tail order.*zero-lift evidence.*descend the left stem.*first bottom bar.*rise then retrace the middle stem.*second bottom bar.*rise then retrace the right stem.*tail shoulder.*descend the short tail without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ъ traces its joined flag-to-stem-to-bowl order to the native lesson", () => {
    const src = CYRILLIC_HARD_SIGN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ъ.*10:34–10:38.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:34–10:38.*upper left.*narrow entry loop.*rounded top shoulder.*descends the tall main stem.*baseline.*circles the lower bowl counterclockwise.*closes it against the stem without lifting.*one continuous pen-down run.*zero intervening lifts.*looped entry and top shoulder.*main stem down.*joined lower bowl.*bundled Noto Sans Cyrillic.*printed ъ-like form.*broad horizontal top flag.*straight main stem.*wide closed lower bowl.*without the narrow entry loop.*rounded top shoulder.*flag-to-stem-to-bowl order.*zero-lift evidence.*sweep right along the top flag.*descend the main stem.*circle the lower bowl counterclockwise.*close it against the stem without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic ы traces its body-before-right-stem order to the native lesson", () => {
    const src = CYRILLIC_YERY.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ы.*10:45–10:56.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*10:45–10:56.*upper left.*narrow entry loop.*descends the tall left stem.*baseline.*circles the joined lower bowl counterclockwise.*closes it against the stem.*lifts once.*upper right.*descends the separate tall right stem.*small rising exit.*two pen-down runs.*one intervening lift.*left stem and lower bowl first.*separate right stem.*bundled Noto Sans Cyrillic.*printed ы-like form.*straight full-height left upright.*wide closed lower bowl.*separate straight full-height right stem.*without the narrow entry loop.*curled exit.*body-before-right-stem order.*counterclockwise direction.*one-lift evidence.*descend the left stem.*circle and close the lower bowl.*lift once.*descend the separate right stem.*connected cursive restores/i,
    );
  });

  it("Cyrillic ь traces its zero-lift stem-before-bowl order to the native lesson", () => {
    const src = CYRILLIC_SOFT_SIGN.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ь.*11:16–11:20.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:16–11:20.*upper left.*narrow entry stroke.*baseline.*circles the joined lower bowl counterclockwise.*closes it against the stem.*without lifting.*one continuous pen-down run.*zero intervening lifts.*stem down.*joined lower bowl.*bundled Noto Sans Cyrillic.*printed ь-like form.*straight full-height upright.*wide closed lower bowl.*slightly slanted entry.*rounded handwritten join.*stem-to-bowl order.*counterclockwise direction.*zero-lift evidence.*descend the stem.*circle the lower bowl counterclockwise.*close it against the stem.*connected cursive may restore/i,
    );
  });

  it("Cyrillic э traces its outer-before-tongue order to the native lesson", () => {
    const src = CYRILLIC_E.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase э.*11:25–11:32.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:25–11:32.*upper-left opening.*curves right across the top.*descends around the outer right side.*sweeps left through the lower curve.*lower-left opening.*lifts once.*middle right.*middle tongue right-to-left.*gentle hook.*two pen-down runs.*one intervening lift.*outer backwards-C curve first.*bundled Noto Sans Cyrillic.*printed э-like form.*broad open-left outer curve.*straight horizontal middle bar.*narrower rounded curve.*hooked tongue.*outer-before-tongue order.*clockwise outer direction.*right-to-left tongue direction.*one-lift evidence.*upper left to lower left.*lift once.*middle bar from right to left.*connected cursive may narrow/i,
    );
  });

  it("Cyrillic ю traces its zero-lift stem-to-oval order to the native lesson", () => {
    const src = CYRILLIC_YU.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase ю.*11:44–11:58.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*11:44–11:58.*small looped entry.*descends the tall left stem.*baseline.*rising diagonal connector.*continues directly into the right oval.*upper-left side.*across the top.*descends the right side.*rounds the bottom.*rises along the left side.*without lifting.*one continuous pen-down run.*zero intervening lifts.*left stem down.*joined connector.*clockwise right oval.*bundled Noto Sans Cyrillic.*printed ю-like form.*straight full-height left upright.*horizontal middle bar.*wide closed oval.*looped entry.*diagonal connector.*narrow cursive oval.*stem-to-connector-to-oval order.*clockwise oval direction.*zero-lift evidence.*descend the left stem.*retrace to the middle.*circle the right oval clockwise.*close it without lifting.*connected cursive restores/i,
    );
  });

  it("Cyrillic я traces its zero-lift rise-to-loop-to-leg order to the native lesson", () => {
    const src = CYRILLIC_YA.source;
    expect(src.url).toBe("https://www.youtube.com/watch?v=tqDLDfYoO2o");
    expect(src.citation).toMatch(
      /RussianIrina.*Learning Russian - Alphabet letters, handwriting.*lowercase я.*12:13–12:21.*5 February 2013/i,
    );
    expect(src.variation).toMatch(
      /all 33 Russian letters.*classic handwritten form taught at school.*12:13–12:21.*baseline.*curved entry.*upper-right junction.*upper loop counterclockwise.*returns to the junction.*long diagonal leg.*baseline exit.*without lifting.*one continuous pen-down run.*zero intervening lifts.*rising entry.*upper loop.*descending diagonal leg.*bundled Noto Sans Cyrillic.*printed я-like form.*straight full-height right upright.*broad upper bowl.*angular lower-left leg.*curved rising entry.*narrow loop.*slanted leg.*exit join.*rise-to-loop-to-leg order.*counterclockwise loop direction.*zero-lift evidence.*climb the right stem.*circle the upper bowl counterclockwise.*descend the diagonal leg without lifting.*connected cursive restores/i,
    );
  });
});
