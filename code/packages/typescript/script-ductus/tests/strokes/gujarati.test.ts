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

const GUJARATI_A = DUCTUS[ductusKey("gujarati", "અ")];
const GUJARATI_AA = DUCTUS[ductusKey("gujarati", "આ")];
const GUJARATI_I = DUCTUS[ductusKey("gujarati", "ઇ")];
const GUJARATI_II = DUCTUS[ductusKey("gujarati", "ઈ")];
const GUJARATI_U = DUCTUS[ductusKey("gujarati", "ઉ")];
const GUJARATI_UU = DUCTUS[ductusKey("gujarati", "ઊ")];
const GUJARATI_VOCALIC_R = DUCTUS[ductusKey("gujarati", "ઋ")];
const GUJARATI_E = DUCTUS[ductusKey("gujarati", "એ")];
const GUJARATI_AI = DUCTUS[ductusKey("gujarati", "ઐ")];
const GUJARATI_O = DUCTUS[ductusKey("gujarati", "ઓ")];
const GUJARATI_AU = DUCTUS[ductusKey("gujarati", "ઔ")];
const GUJARATI_KA = DUCTUS[ductusKey("gujarati", "ક")];
const GUJARATI_KHA = DUCTUS[ductusKey("gujarati", "ખ")];
const GUJARATI_GA = DUCTUS[ductusKey("gujarati", "ગ")];
const GUJARATI_GHA = DUCTUS[ductusKey("gujarati", "ઘ")];
const GUJARATI_NGA = DUCTUS[ductusKey("gujarati", "ઙ")];
const GUJARATI_CA = DUCTUS[ductusKey("gujarati", "ચ")];
const GUJARATI_CHA = DUCTUS[ductusKey("gujarati", "છ")];
const GUJARATI_JA = DUCTUS[ductusKey("gujarati", "જ")];
const GUJARATI_JHA = DUCTUS[ductusKey("gujarati", "ઝ")];
const GUJARATI_NYA = DUCTUS[ductusKey("gujarati", "ઞ")];
const GUJARATI_TTA = DUCTUS[ductusKey("gujarati", "ટ")];
const GUJARATI_TTHA = DUCTUS[ductusKey("gujarati", "ઠ")];
const GUJARATI_DDA = DUCTUS[ductusKey("gujarati", "ડ")];
const GUJARATI_DDHA = DUCTUS[ductusKey("gujarati", "ઢ")];
const GUJARATI_NNA = DUCTUS[ductusKey("gujarati", "ણ")];
const GUJARATI_TA = DUCTUS[ductusKey("gujarati", "ત")];
const GUJARATI_THA = DUCTUS[ductusKey("gujarati", "થ")];
const GUJARATI_DA = DUCTUS[ductusKey("gujarati", "દ")];
const GUJARATI_DHA = DUCTUS[ductusKey("gujarati", "ધ")];
const GUJARATI_NA = DUCTUS[ductusKey("gujarati", "ન")];
const GUJARATI_PA = DUCTUS[ductusKey("gujarati", "પ")];
const GUJARATI_PHA = DUCTUS[ductusKey("gujarati", "ફ")];
const GUJARATI_BA = DUCTUS[ductusKey("gujarati", "બ")];
const GUJARATI_BHA = DUCTUS[ductusKey("gujarati", "ભ")];
const GUJARATI_MA = DUCTUS[ductusKey("gujarati", "મ")];
const GUJARATI_YA = DUCTUS[ductusKey("gujarati", "ય")];
const GUJARATI_RA = DUCTUS[ductusKey("gujarati", "ર")];
const GUJARATI_LA = DUCTUS[ductusKey("gujarati", "લ")];
const GUJARATI_LLA = DUCTUS[ductusKey("gujarati", "ળ")];
const GUJARATI_VA = DUCTUS[ductusKey("gujarati", "વ")];
const GUJARATI_SHA = DUCTUS[ductusKey("gujarati", "શ")];
const GUJARATI_SA = DUCTUS[ductusKey("gujarati", "સ")];
const GUJARATI_HA = DUCTUS[ductusKey("gujarati", "હ")];

const OWNER_SCRIPTS = new Set(["gujarati"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { હ: 0.92 });

  it("marks Gujarati complete only with all 44 unique letters source-verified", () => {
    const gujarati = SCRIPTS.find((script) => script.script === "gujarati")!;
    expect(gujarati.complete).toBe(true);
    expect(gujarati.letters).toHaveLength(44);
    expect(
      gujarati.letters.every(
        (letter) => letter.strokeOrderSource !== undefined,
      ),
    ).toBe(true);
  });

  it("Gujarati અ draws the joined body before the lifted right stem", () => {
    expect(GUJARATI_A.script).toBe("gujarati");
    expect(penLifts(GUJARATI_A)).toBe(1);
    expect(GUJARATI_A.strokes).toHaveLength(2);
    expect(GUJARATI_A.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 1,
    ]);
    const [left, lower, arch] = GUJARATI_A.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const stem = GUJARATI_A.strokes[1].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(arch[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(stem.at(-1)!.x).toBeGreaterThan(stem[0].x);
  });

  it("Gujarati આ adds a second lifted stem after the complete અ body", () => {
    expect(GUJARATI_AA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AA)).toBe(2);
    expect(GUJARATI_AA.strokes).toHaveLength(3);
    expect(GUJARATI_AA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [3, 1, 1],
    );
    const [left, lower, arch] = GUJARATI_AA.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const firstStem = GUJARATI_AA.strokes[1].segments[0].path;
    const trailingStem = GUJARATI_AA.strokes[2].segments[0].path;
    expect(left.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(arch[0]);
    expect(firstStem[0].y).toBeGreaterThan(firstStem.at(-1)!.y);
    expect(trailingStem[0].y).toBeGreaterThan(trailingStem.at(-1)!.y);
    expect(trailingStem[0].x).toBeGreaterThan(firstStem[0].x);
  });

  it("Gujarati ઇ keeps both loops and the rising hook in one pen-down run", () => {
    expect(GUJARATI_I.script).toBe("gujarati");
    expect(penLifts(GUJARATI_I)).toBe(0);
    expect(GUJARATI_I.strokes).toHaveLength(1);
    expect(GUJARATI_I.strokes[0].segments).toHaveLength(4);
    const [upper, crossing, lower, hook] = GUJARATI_I.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(crossing[0]);
    expect(crossing.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(hook[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(hook.at(-1)!.y).toBeGreaterThan(hook[0].y);
  });

  it("Gujarati ઈ extends the zero-lift ઇ run into a high clockwise curl", () => {
    expect(GUJARATI_II.script).toBe("gujarati");
    expect(penLifts(GUJARATI_II)).toBe(0);
    expect(GUJARATI_II.strokes).toHaveLength(1);
    expect(GUJARATI_II.strokes[0].segments).toHaveLength(4);
    const [upper, crossing, lower, curl] = GUJARATI_II.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(crossing[0]);
    expect(crossing.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(curl[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(Math.max(...curl.map((point) => point.y))).toBeGreaterThan(
      upper[0].y,
    );
    expect(curl.at(-1)!.x).toBeGreaterThan(curl[0].x);
  });

  it("Gujarati ઉ joins both bowls to its tall returning outer curve", () => {
    expect(GUJARATI_U.script).toBe("gujarati");
    expect(penLifts(GUJARATI_U)).toBe(0);
    expect(GUJARATI_U.strokes).toHaveLength(1);
    expect(GUJARATI_U.strokes[0].segments).toHaveLength(3);
    const [upper, lower, outer] = GUJARATI_U.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(outer[0]);
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(
      upper.at(-1)!.x,
    );
    expect(Math.min(...outer.map((point) => point.x))).toBeLessThan(
      lower.at(-1)!.x,
    );
    expect(outer.at(-1)!.y).toBeGreaterThan(upper[0].y);
  });

  it("Gujarati ઊ extends the zero-lift ઉ run down a long right tail", () => {
    expect(GUJARATI_UU.script).toBe("gujarati");
    expect(penLifts(GUJARATI_UU)).toBe(0);
    expect(GUJARATI_UU.strokes).toHaveLength(1);
    expect(GUJARATI_UU.strokes[0].segments).toHaveLength(3);
    const [body, outer, tail] = GUJARATI_UU.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(body.at(-1)).toEqual(outer[0]);
    expect(outer.at(-1)).toEqual(tail[0]);
    expect(Math.min(...outer.map((point) => point.x))).toBeLessThan(
      body.at(-1)!.x,
    );
    expect(Math.max(...tail.map((point) => point.x))).toBeGreaterThan(
      outer.at(-1)!.x,
    );
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
  });

  it("Gujarati ઋ writes the bent body, central stem, then right loop and tail", () => {
    expect(GUJARATI_VOCALIC_R.script).toBe("gujarati");
    expect(penLifts(GUJARATI_VOCALIC_R)).toBe(2);
    expect(GUJARATI_VOCALIC_R.strokes).toHaveLength(3);
    expect(
      GUJARATI_VOCALIC_R.strokes.every(
        (stroke) => stroke.segments.length === 1,
      ),
    ).toBe(true);
    const body = GUJARATI_VOCALIC_R.strokes[0].segments[0].path;
    const stem = GUJARATI_VOCALIC_R.strokes[1].segments[0].path;
    const loopAndTail = GUJARATI_VOCALIC_R.strokes[2].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(
      Math.max(...body.map((point) => point.x)),
    );
    expect(stem.at(-1)!.y).toBeLessThan(stem[0].y);
    expect(Math.max(...loopAndTail.map((point) => point.y))).toBeGreaterThan(
      loopAndTail[0].y,
    );
    expect(loopAndTail.at(-1)!.y).toBeLessThan(loopAndTail[0].y);
  });

  it("Gujarati એ writes the joined body, right stem, then high arc", () => {
    expect(GUJARATI_E.script).toBe("gujarati");
    expect(penLifts(GUJARATI_E)).toBe(2);
    expect(GUJARATI_E.strokes).toHaveLength(3);
    expect(GUJARATI_E.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1,
    ]);
    const [bowl, body] = GUJARATI_E.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(bowl.at(-1)).toEqual(body[0]);
    expect(Math.min(...bowl.map((point) => point.x))).toBeLessThan(bowl[0].x);
    expect(GUJARATI_E.strokes[1].segments[0].path.at(-1)!.y).toBeLessThan(
      GUJARATI_E.strokes[1].segments[0].path[0].y,
    );
    expect(
      Math.max(
        ...GUJARATI_E.strokes[2].segments[0].path.map((point) => point.y),
      ),
    ).toBeGreaterThan(Math.max(...bowl.map((point) => point.y)));
  });

  it("Gujarati ઐ extends એ with a second, higher arc", () => {
    expect(GUJARATI_AI.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AI)).toBe(3);
    expect(GUJARATI_AI.strokes).toHaveLength(4);
    expect(
      GUJARATI_AI.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const lowerArc = GUJARATI_AI.strokes[2].segments[0].path;
    const higherArc = GUJARATI_AI.strokes[3].segments[0].path;
    expect(Math.max(...higherArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    expect(lowerArc.at(-1)!.x).toBeGreaterThan(lowerArc[0].x);
    expect(higherArc.at(-1)!.x).toBeGreaterThan(higherArc[0].x);
  });

  it("Gujarati ઓ writes the complete આ sequence before its high arc", () => {
    expect(GUJARATI_O.script).toBe("gujarati");
    expect(penLifts(GUJARATI_O)).toBe(3);
    expect(GUJARATI_O.strokes).toHaveLength(4);
    expect(GUJARATI_O.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 1, 1, 1,
    ]);
    const [left, body, arch] = GUJARATI_O.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(left.at(-1)).toEqual(body[0]);
    expect(body.at(-1)).toEqual(arch[0]);
    expect(GUJARATI_O.strokes[2].segments[0].path[0].x).toBeGreaterThan(
      GUJARATI_O.strokes[1].segments[0].path[0].x,
    );
    const highArc = GUJARATI_O.strokes[3].segments[0].path;
    expect(highArc.at(-1)!.x).toBeGreaterThan(highArc[0].x);
    expect(Math.max(...highArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...left.map((point) => point.y)),
    );
  });

  it("Gujarati ઔ extends ઓ with a second, higher arc", () => {
    expect(GUJARATI_AU.script).toBe("gujarati");
    expect(penLifts(GUJARATI_AU)).toBe(4);
    expect(GUJARATI_AU.strokes).toHaveLength(5);
    expect(
      GUJARATI_AU.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const lowerArc = GUJARATI_AU.strokes[3].segments[0].path;
    const higherArc = GUJARATI_AU.strokes[4].segments[0].path;
    expect(Math.max(...higherArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    expect(lowerArc.at(-1)!.x).toBeGreaterThan(lowerArc[0].x);
    expect(higherArc.at(-1)!.x).toBeGreaterThan(higherArc[0].x);
  });

  it("Gujarati ક writes its joined loop-body before the diagonal cross-stroke", () => {
    expect(GUJARATI_KA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_KA)).toBe(1);
    expect(GUJARATI_KA.strokes).toHaveLength(2);
    expect(
      GUJARATI_KA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_KA.strokes[0].segments[0].path;
    const cross = GUJARATI_KA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(cross.at(-1)!.x).toBeGreaterThan(cross[0].x);
    expect(cross.at(-1)!.y).toBeGreaterThan(cross[0].y);
  });

  it("Gujarati ખ writes its joined left body before the separate right spine", () => {
    expect(GUJARATI_KHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_KHA)).toBe(1);
    expect(GUJARATI_KHA.strokes).toHaveLength(2);
    expect(
      GUJARATI_KHA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_KHA.strokes[0].segments[0].path;
    const spine = GUJARATI_KHA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ગ writes its rounded body before the separate right spine", () => {
    expect(GUJARATI_GA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_GA)).toBe(1);
    expect(GUJARATI_GA.strokes).toHaveLength(2);
    expect(
      GUJARATI_GA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_GA.strokes[0].segments[0].path;
    const spine = GUJARATI_GA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ઘ joins its upper and lower bodies before the separate right spine", () => {
    expect(GUJARATI_GHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_GHA)).toBe(1);
    expect(GUJARATI_GHA.strokes).toHaveLength(2);
    expect(
      GUJARATI_GHA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_GHA.strokes[0].segments[0].path;
    const spine = GUJARATI_GHA.strokes[1].segments[0].path;
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(200);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(500);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ઙ writes its S-like body before the separate upper-right dot", () => {
    expect(GUJARATI_NGA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NGA)).toBe(1);
    expect(GUJARATI_NGA.strokes).toHaveLength(2);
    expect(
      GUJARATI_NGA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_NGA.strokes[0].segments[0].path;
    const dot = GUJARATI_NGA.strokes[1].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(dot.at(-1)).toEqual(dot[0]);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(350);
  });

  it("Gujarati ચ joins its bowls and middle loop before the separate right spine", () => {
    expect(GUJARATI_CA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_CA)).toBe(1);
    expect(GUJARATI_CA.strokes).toHaveLength(2);
    expect(
      GUJARATI_CA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_CA.strokes[0].segments[0].path;
    const spine = GUJARATI_CA.strokes[1].segments[0].path;
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(200);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(500);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(spine.at(-1)!.x).toBeGreaterThan(spine[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati છ joins both upper lobes through one continuous lower body", () => {
    expect(GUJARATI_CHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_CHA)).toBe(0);
    expect(GUJARATI_CHA.strokes).toHaveLength(1);
    expect(GUJARATI_CHA.strokes[0].segments).toHaveLength(3);
    expect(joinGaps(GUJARATI_CHA.strokes[0]).every((gap) => gap === 0)).toBe(
      true,
    );
    const path = penPath(GUJARATI_CHA.strokes[0]);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(500);
    expect(Math.max(...path.map((point) => point.x))).toBeGreaterThan(600);
  });

  it("Gujarati જ joins its left loop, crossing, right loop, and exit", () => {
    expect(GUJARATI_JA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_JA)).toBe(0);
    expect(GUJARATI_JA.strokes).toHaveLength(1);
    expect(GUJARATI_JA.strokes[0].segments).toHaveLength(3);
    expect(joinGaps(GUJARATI_JA.strokes[0]).every((gap) => gap === 0)).toBe(
      true,
    );
    const path = penPath(GUJARATI_JA.strokes[0]);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(500);
    expect(path.at(-1)!.x).toBeGreaterThan(path[0].x);
  });

  it("Gujarati ઝ writes its left body, right loop-and-tail, then upper stem", () => {
    expect(GUJARATI_JHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_JHA)).toBe(2);
    expect(GUJARATI_JHA.strokes).toHaveLength(3);
    expect(
      GUJARATI_JHA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const left = GUJARATI_JHA.strokes[0].segments[0].path;
    const stem = GUJARATI_JHA.strokes[2].segments[0].path;
    expect(left.at(-1)!.x).toBeLessThan(left[0].x);
    expect(left.at(-1)!.y).toBeLessThan(left[0].y);
    expect(stem.at(-1)!.y).toBeLessThan(stem[0].y);
  });

  it("Gujarati ઞ writes its left body, short shoulder, then tall spine", () => {
    expect(GUJARATI_NYA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NYA)).toBe(2);
    expect(GUJARATI_NYA.strokes).toHaveLength(3);
    expect(
      GUJARATI_NYA.strokes.every((stroke) => stroke.segments.length === 1),
    ).toBe(true);
    const body = GUJARATI_NYA.strokes[0].segments[0].path;
    const shoulder = GUJARATI_NYA.strokes[1].segments[0].path;
    const spine = GUJARATI_NYA.strokes[2].segments[0].path;
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(shoulder.at(-1)!.x).toBeGreaterThan(shoulder[0].x);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ટ joins its upper turn, middle transition, and lower bowl", () => {
    expect(GUJARATI_TTA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_TTA)).toBe(0);
    expect(GUJARATI_TTA.strokes).toHaveLength(1);
    expect(GUJARATI_TTA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_TTA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(20);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(50);
    expect(path.at(-1)!.x).toBeGreaterThan(path[0].x);
  });

  it("Gujarati ઠ joins its high shoulder, outer bowl, and inward curl", () => {
    expect(GUJARATI_TTHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_TTHA)).toBe(0);
    expect(GUJARATI_TTHA.strokes).toHaveLength(1);
    expect(GUJARATI_TTHA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_TTHA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(30);
    expect(path[0].x).toBeGreaterThan(path[6].x);
    expect(path.at(-1)!.y).toBeGreaterThan(
      Math.min(...path.map((point) => point.y)),
    );
  });

  it("Gujarati ડ joins its high shoulder, middle descent, and lower bowl", () => {
    expect(GUJARATI_DDA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_DDA)).toBe(0);
    expect(GUJARATI_DDA.strokes).toHaveLength(1);
    expect(GUJARATI_DDA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_DDA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(25);
    expect(path[0].x).toBeGreaterThan(path[6].x);
    expect(path.at(-1)!.x).toBeLessThan(path[0].x);
  });

  it("Gujarati ઢ joins its upper shoulder, outer bowl, and inner loop", () => {
    expect(GUJARATI_DDHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_DDHA)).toBe(0);
    expect(GUJARATI_DDHA.strokes).toHaveLength(1);
    expect(GUJARATI_DDHA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_DDHA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(34);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(570);
    expect(path.at(-1)!.y).toBeGreaterThan(
      Math.min(...path.map((point) => point.y)),
    );
  });

  it("Gujarati ણ separates its hooked body, middle bowl, and right spine", () => {
    expect(GUJARATI_NNA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NNA)).toBe(2);
    expect(GUJARATI_NNA.strokes).toHaveLength(3);
    expect(
      GUJARATI_NNA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = GUJARATI_NNA.strokes[0].segments[0].path;
    const bowl = GUJARATI_NNA.strokes[1].segments[0].path;
    const spine = GUJARATI_NNA.strokes[2].segments[0].path;
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    expect(bowl.length).toBeGreaterThanOrEqual(15);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ત separates its open body and tall right spine", () => {
    expect(GUJARATI_TA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_TA)).toBe(1);
    expect(GUJARATI_TA.strokes).toHaveLength(2);
    expect(GUJARATI_TA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const body = GUJARATI_TA.strokes[0].segments[0].path;
    const spine = GUJARATI_TA.strokes[1].segments[0].path;
    expect(body[0].y).toBeLessThan(body.at(-1)!.y);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati થ separates its looped body and tall right spine", () => {
    expect(GUJARATI_THA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_THA)).toBe(1);
    expect(GUJARATI_THA.strokes).toHaveLength(2);
    expect(
      GUJARATI_THA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = GUJARATI_THA.strokes[0].segments[0].path;
    const spine = GUJARATI_THA.strokes[1].segments[0].path;
    expect(body.length).toBeGreaterThanOrEqual(28);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati દ keeps its upper and lower bodies in one stroke", () => {
    expect(GUJARATI_DA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_DA)).toBe(0);
    expect(GUJARATI_DA.strokes).toHaveLength(1);
    expect(GUJARATI_DA.strokes[0].segments).toHaveLength(1);
    const path = GUJARATI_DA.strokes[0].segments[0].path;
    expect(path.length).toBeGreaterThanOrEqual(28);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(550);
    expect(Math.min(...path.map((point) => point.y))).toBeLessThan(50);
  });

  it("Gujarati ધ separates its joined body and tall right spine", () => {
    expect(GUJARATI_DHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_DHA)).toBe(1);
    expect(GUJARATI_DHA.strokes).toHaveLength(2);
    expect(
      GUJARATI_DHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = GUJARATI_DHA.strokes[0].segments[0].path;
    const spine = GUJARATI_DHA.strokes[1].segments[0].path;
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(600);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ન separates its loop-and-shoulder body and tall right spine", () => {
    expect(GUJARATI_NA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_NA)).toBe(1);
    expect(GUJARATI_NA.strokes).toHaveLength(2);
    expect(GUJARATI_NA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const body = GUJARATI_NA.strokes[0].segments[0].path;
    const spine = GUJARATI_NA.strokes[1].segments[0].path;
    expect(body.length).toBeGreaterThanOrEqual(18);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati પ separates its hooked lower body and tall right spine", () => {
    expect(GUJARATI_PA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_PA)).toBe(1);
    expect(GUJARATI_PA.strokes).toHaveLength(2);
    expect(GUJARATI_PA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const body = GUJARATI_PA.strokes[0].segments[0].path;
    const spine = GUJARATI_PA.strokes[1].segments[0].path;
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(550);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ફ separates its winding body and diagonal cross-stroke", () => {
    expect(GUJARATI_PHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_PHA)).toBe(1);
    expect(GUJARATI_PHA.strokes).toHaveLength(2);
    expect(
      GUJARATI_PHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = GUJARATI_PHA.strokes[0].segments[0].path;
    const crossStroke = GUJARATI_PHA.strokes[1].segments[0].path;
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(-100);
    expect(crossStroke.at(-1)!.y).toBeGreaterThan(crossStroke[0].y);
  });

  it("Gujarati બ completes its rounded body before the right spine", () => {
    expect(GUJARATI_BA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_BA)).toBe(1);
    expect(GUJARATI_BA.strokes).toHaveLength(2);
    expect(GUJARATI_BA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const body = GUJARATI_BA.strokes[0].segments[0].path;
    const spine = GUJARATI_BA.strokes[1].segments[0].path;
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(550);
    expect(spine.at(-1)!.y).toBeLessThan(spine[0].y);
  });

  it("Gujarati ભ completes its loop before the right spine", () => {
    expect(GUJARATI_BHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_BHA)).toBe(1);
    expect(GUJARATI_BHA.strokes).toHaveLength(2);
  });

  it("Gujarati મ completes its left body before the right spine", () => {
    expect(GUJARATI_MA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_MA)).toBe(1);
    expect(GUJARATI_MA.strokes).toHaveLength(2);
  });

  it("Gujarati ય completes its rounded body before the right spine", () => {
    expect(GUJARATI_YA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_YA)).toBe(1);
    expect(GUJARATI_YA.strokes).toHaveLength(2);
  });

  it("Gujarati ર keeps its upper body, middle loop, and tail continuous", () => {
    expect(GUJARATI_RA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_RA)).toBe(0);
    expect(GUJARATI_RA.strokes).toHaveLength(1);
  });

  it("Gujarati લ completes its body and shoulder before the right spine", () => {
    expect(GUJARATI_LA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_LA)).toBe(2);
    expect(GUJARATI_LA.strokes).toHaveLength(3);
  });

  it("Gujarati ળ keeps its bowl, turn, arch, and spine continuous", () => {
    expect(GUJARATI_LLA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_LLA)).toBe(0);
    expect(GUJARATI_LLA.strokes).toHaveLength(1);
  });

  it("Gujarati વ completes its rounded body before the right spine", () => {
    expect(GUJARATI_VA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_VA)).toBe(1);
    expect(GUJARATI_VA.strokes).toHaveLength(2);
  });

  it("Gujarati શ completes its looped body before the right spine", () => {
    expect(GUJARATI_SHA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_SHA)).toBe(1);
    expect(GUJARATI_SHA.strokes).toHaveLength(2);
  });

  it("Gujarati સ completes its looped body and shoulder before the right spine", () => {
    expect(GUJARATI_SA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_SA)).toBe(1);
    expect(GUJARATI_SA.strokes).toHaveLength(2);
  });

  it("Gujarati હ joins its upper loop and broad lower bowl without lifting", () => {
    expect(GUJARATI_HA.script).toBe("gujarati");
    expect(penLifts(GUJARATI_HA)).toBe(0);
    expect(GUJARATI_HA.strokes).toHaveLength(1);
  });

  it("Gujarati અ traces its body-before-stem order to the teaching animation", () => {
    const src = GUJARATI_A.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*અ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper-left tip.*clockwise.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*separate right stem.*lower-right foot.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem order.*one-lift evidence/i,
    );
  });

  it("Gujarati આ traces its two lifted stems to the next teaching animation", () => {
    const src = GUJARATI_AA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*આ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*joined અ body.*open curve.*clockwise.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*separate right stem.*lifts again.*third SVG path.*trailing ā stem.*matching foot.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઇ traces its unbroken loop-and-hook order to the next animation", () => {
    const src = GUJARATI_I.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઇ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper loop.*narrow middle crossing.*broad lower body.*right side.*upper-right hook.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-loop-to-lower-loop-to-hook order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઈ traces its unbroken extended curl to the adjacent animation", () => {
    const src = GUJARATI_II.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઈ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper loop.*narrow middle crossing.*broad lower body.*right side.*clockwise.*extended top hook.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-loop-to-lower-loop-to-extended-curl order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઉ traces its unbroken bowls and returning curve to the next animation", () => {
    const src = GUJARATI_U.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઉ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper-left.*small upper bowl.*middle cusp.*broad lower bowl.*tall outer-left curve.*upper right.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-bowl-to-lower-bowl-to-outer-curve order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઊ traces its unbroken extended tail to the adjacent animation", () => {
    const src = GUJARATI_UU.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઊ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*repeats ઉ.*small upper bowl.*middle cusp.*broad lower bowl.*tall outer-left return.*upper shoulder.*long right-side tail.*lower foot.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*complete-u-before-extended-tail order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઋ traces its bent body, stem, and right loop to three paths", () => {
    const src = GUJARATI_VOCALIC_R.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઋ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*begins at the left.*sweeps right.*shallow upper body.*middle turn.*reverses diagonally down-left.*lower terminal.*lifts once.*second SVG path.*separate central stem.*lower foot.*lifts again.*third SVG path.*stem junction.*compact upper-right loop.*longer right-side tail.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-central-stem-before-right-loop-and-tail order.*two-lift evidence/i,
    );
  });

  it("Gujarati એ traces its body, stem, and high arc to three ordered paths", () => {
    const src = GUJARATI_E.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*એ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*left bowl.*broad lower body.*small right arch.*lifts once.*second SVG path.*full-height right stem.*lower foot.*lifts again.*third SVG path.*high arcing mark.*left to right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem-before-high-arc order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઐ traces its stacked arcs to four ordered paths", () => {
    const src = GUJARATI_AI.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઐ animation.*first through fourth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /four ordered pen-down runs.*first SVG path.*joined left bowl.*broad lower body.*small right arch.*lifts once.*second SVG path.*full-height right stem.*lower foot.*lifts again.*third SVG path.*lower high arc.*left to right.*lifts once more.*fourth SVG path.*second higher arc.*same direction.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-right-stem-before-lower-arc-before-higher-arc order.*three-lift evidence/i,
    );
  });

  it("Gujarati ઓ traces its આ sequence and high arc to four ordered paths", () => {
    const src = GUJARATI_O.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઓ animation.*first through fourth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /four ordered pen-down runs.*first SVG path.*આ's joined body.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*first separate right stem.*lower foot.*lifts again.*third SVG path.*trailing stem.*matching foot.*lifts once more.*fourth SVG path.*high arc.*left to right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem-before-high-arc order.*three-lift evidence/i,
    );
  });

  it("Gujarati ઔ traces its stacked arcs to five ordered paths", () => {
    const src = GUJARATI_AU.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઔ animation.*first through fifth SVG paths/i,
    );
    expect(src.variation).toMatch(
      /five ordered pen-down runs.*first SVG path.*ઓ's joined body.*open left curve.*broad lower body.*middle shoulder.*small right arch.*lifts once.*second SVG path.*first separate right stem.*lower foot.*lifts again.*third SVG path.*trailing stem.*matching foot.*lifts once more.*fourth SVG path.*lower high arc.*left to right.*lifts a fourth time.*fifth SVG path.*second, higher arc.*same direction.*remaining path slot is empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-first-stem-before-trailing-stem-before-lower-arc-before-higher-arc order.*four-lift evidence/i,
    );
  });

  it("Gujarati ક traces its joined body and crossing diagonal to two paths", () => {
    const src = GUJARATI_KA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ક animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*circles left.*upper loop.*diagonally down-right.*middle crossing.*rounded lower body.*lower left.*lifts once.*second SVG path.*diagonal cross-stroke.*lower left to upper right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-loop-body-before-diagonal-cross-stroke order.*one-lift evidence/i,
    );
  });

  it("Gujarati ખ traces its joined body and right spine to two paths", () => {
    const src = GUJARATI_KHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ખ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*left lobe.*curls through the middle.*beside the right spine.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ગ traces its rounded body and right spine to two paths", () => {
    const src = GUJARATI_GA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ગ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*clockwise.*rounded body.*lower left.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*rounded-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ઘ traces its joined double body and right spine to two paths", () => {
    const src = GUJARATI_GHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઘ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*upper lobe.*back left through the middle.*clockwise.*rounded lower body.*upper right.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-upper-and-lower-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ઙ traces its S-like body and upper-right dot to two paths", () => {
    const src = GUJARATI_NGA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ઙ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*upper turn.*diagonally through the middle.*rounded lower body.*lower left.*lifts once.*second SVG path.*upper-right dot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-dot order.*one-lift evidence/i,
    );
  });

  it("Gujarati ચ traces its joined bowls and right spine to two paths", () => {
    const src = GUJARATI_CA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*ચ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*upper bowl.*middle loop.*clockwise.*broad lower body.*upper right.*lifts once.*second SVG path.*full right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati છ traces both lobes and the lower body to one continuous path", () => {
    const src = GUJARATI_CHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*છ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper left.*upper-left lobe.*middle.*broad lower body.*outer-right curve.*upper-right lobe.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-left-lobe-to-lower-body-to-upper-right-lobe order.*zero-lift evidence/i,
    );
  });

  it("Gujarati જ traces both loops, crossing, and exit to one continuous path", () => {
    const src = GUJARATI_JA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*Gujarati Alphabet Writing Practice.*version 1\.0.*જ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper left.*upper-left loop.*diagonally down-right.*crossing body.*lower-right loop.*upper-right exit.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-loop-to-crossing-to-right-loop-to-exit order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઝ traces its left body, right loop, and stem to three paths", () => {
    const src = GUJARATI_JHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ઝ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*upper left.*rounded left body.*lower left.*lifts once.*second SVG path.*right loop.*lower tail.*lifts again.*third SVG path.*upper stem.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-right-loop-and-tail-before-upper-stem order.*two-lift evidence/i,
    );
  });

  it("Gujarati ઞ traces its left body, shoulder, and spine to three paths", () => {
    const src = GUJARATI_NYA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ઞ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*upper left.*rounded left body.*lower left.*lifts once.*second SVG path.*rightward shoulder.*lifts again.*third SVG path.*tall right spine.*lower-right terminal.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*left-body-before-rightward-shoulder-before-tall-spine order.*two-lift evidence/i,
    );
  });

  it("Gujarati ટ traces its joined upper turn and lower bowl to one path", () => {
    const src = GUJARATI_TTA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ટ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper left.*rounded upper turn.*diagonally down-left.*broad lower bowl.*right side.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-turn-to-middle-to-lower-bowl order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઠ traces its shoulder, outer bowl, and inward curl to one path", () => {
    const src = GUJARATI_TTHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ઠ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper right.*left across the high shoulder.*descends through the middle.*outer lower bowl.*curls back inward.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*shoulder-to-outer-bowl-to-inner-terminal order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ડ traces its shoulder, middle descent, and lower bowl to one path", () => {
    const src = GUJARATI_DDA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ડ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper right.*left across the high shoulder.*descends through the middle.*broad lower bowl.*lower-left terminal.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*shoulder-to-middle-to-lower-bowl order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ઢ traces its shoulder, outer bowl, and inner loop to one path", () => {
    const src = GUJARATI_DDHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ઢ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper left.*right across the high shoulder.*descends through the middle.*broad outer lower bowl.*small inner loop.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*shoulder-to-outer-bowl-to-inner-loop order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ણ traces its hooked body, bowl, and right spine to three paths", () => {
    const src = GUJARATI_NNA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ણ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*left spine.*hooked lower tail.*lifts once.*second SVG path.*middle bowl.*lifts again.*third SVG path.*tall right spine.*lower foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*body-before-middle-bowl-before-right-spine order.*two-lift evidence/i,
    );
  });

  it("Gujarati ત traces its open body and right spine to two paths", () => {
    const src = GUJARATI_TA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ત animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*lower terminal.*open left body.*upper shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*open-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati થ traces its looped body and right spine to two paths", () => {
    const src = GUJARATI_THA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*થ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*small upper loop.*downward through the middle.*broad lower body.*right shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*loop-and-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati દ traces its joined upper and lower bodies to one path", () => {
    const src = GUJARATI_DA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*દ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper right.*rounded upper body.*middle turn.*broad lower body.*lower-right terminal.*without lifting.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*upper-body-to-middle-turn-to-lower-body order.*zero-lift evidence/i,
    );
  });

  it("Gujarati ધ traces its joined body and right spine to two paths", () => {
    const src = GUJARATI_DHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ધ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*high left entry.*upper turn.*middle.*broad lower body.*right shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*joined-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ન traces its loop, shoulder, and right spine to two paths", () => {
    const src = GUJARATI_NA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ન animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*small loop.*long shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*loop-and-shoulder-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati પ traces its hooked body and right spine to two paths", () => {
    const src = GUJARATI_PA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*પ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*high left hook.*curls upward and right.*left stem.*broad lower body.*right shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*hooked-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ફ traces its winding body and cross-stroke to two paths", () => {
    const src = GUJARATI_PHA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ફ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*high cap.*winding main body.*lower body.*small lower-left loop.*descending right tail.*lifts once.*second SVG path.*diagonal cross-stroke.*lower left to upper right.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*complete-body-before-cross-stroke order.*one-lift evidence/i,
    );
  });

  it("Gujarati બ traces its rounded body and right spine to two paths", () => {
    const src = GUJARATI_BA.source;
    expect(src.url).toBe("https://www.t30apps.com/gujarati-alphabet-writing/");
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*બ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper left.*rounded left body.*compact middle turn.*right.*shoulder.*lifts once.*second SVG path.*tall right spine.*lower-right foot.*remaining path slots are empty.*one variant.*not a universal standard.*bundled Noto Sans Gujarati.*rounded-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ભ traces its loop and right spine to two paths", () => {
    const src = GUJARATI_BHA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ભ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*broad left loop.*compact middle turn.*long shoulder.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*loop-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati મ traces its left body and right spine to two paths", () => {
    const src = GUJARATI_MA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*મ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*upper left.*compact inner turn.*long shoulder.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*left-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ય traces its rounded body and right spine to two paths", () => {
    const src = GUJARATI_YA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ય animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*upper left.*rounded upper turn.*broad lower body.*long shoulder.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*rounded-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati ર traces its upper body, middle loop, and tail to one path", () => {
    const src = GUJARATI_RA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ર animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper left.*rounded upper body.*small middle loop.*lower-right tail.*without lifting.*one variant.*upper-body-to-middle-loop-to-lower-tail order.*zero-lift evidence/i,
    );
  });

  it("Gujarati લ traces its body, shoulder, and spine to three paths", () => {
    const src = GUJARATI_LA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*લ animation.*first through third SVG paths/i,
    );
    expect(src.variation).toMatch(
      /three ordered pen-down runs.*first SVG path.*upper right.*broad rounded left body.*lower-right terminal.*lifts once.*second SVG path.*middle shoulder.*left to right.*lifts again.*third SVG path.*tall right spine.*remaining path slots are empty.*one variant.*rounded-body-before-middle-shoulder-before-right-spine order.*two-lift evidence/i,
    );
  });

  it("Gujarati ળ traces its bowl, turn, arch, and spine to one path", () => {
    const src = GUJARATI_LLA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*ળ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*first SVG path.*remaining path slots are empty.*upper left.*broad left bowl.*narrow middle turn.*high right arch.*tall right spine.*without lifting.*one variant.*left-bowl-to-middle-turn-to-right-spine order.*zero-lift evidence/i,
    );
  });

  it("Gujarati વ traces its rounded body and spine to two paths", () => {
    const src = GUJARATI_VA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*વ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*broad rounded left body.*right shoulder.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*rounded-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati શ traces its looped body and spine to two paths", () => {
    const src = GUJARATI_SHA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*શ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*small upper loop.*broad lower body.*lower-right tail.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*loop-and-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati સ traces its looped body, shoulder, and spine to two paths", () => {
    const src = GUJARATI_SA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*સ animation.*first and second SVG paths/i,
    );
    expect(src.variation).toMatch(
      /two ordered pen-down runs.*first SVG path.*upper right.*rounded upper loop.*left body.*long right shoulder.*lifts once.*second SVG path.*tall right spine.*remaining path slots are empty.*one variant.*loop-and-body-before-right-spine order.*one-lift evidence/i,
    );
  });

  it("Gujarati હ traces its loop and lower bowl to one continuous path", () => {
    const src = GUJARATI_HA.source;
    expect(src.citation).toMatch(
      /t30apps\.com.*version 1\.0.*હ animation.*first SVG path/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*remaining path slots are empty.*upper right.*compact upper loop.*middle turn.*broad lower bowl.*rightward finish.*without lifting.*one variant.*upper-loop-to-middle-turn-to-lower-bowl order.*zero-lift evidence/i,
    );
  });
});
