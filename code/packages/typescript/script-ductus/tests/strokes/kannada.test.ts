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

const KANNADA_A = DUCTUS[ductusKey("kannada", "ಅ")];
const KANNADA_AA = DUCTUS[ductusKey("kannada", "ಆ")];
const KANNADA_I = DUCTUS[ductusKey("kannada", "ಇ")];
const KANNADA_U = DUCTUS[ductusKey("kannada", "ಉ")];
const KANNADA_UU = DUCTUS[ductusKey("kannada", "ಊ")];
const KANNADA_E = DUCTUS[ductusKey("kannada", "ಎ")];
const KANNADA_EE = DUCTUS[ductusKey("kannada", "ಏ")];
const KANNADA_O = DUCTUS[ductusKey("kannada", "ಒ")];
const KANNADA_OO = DUCTUS[ductusKey("kannada", "ಓ")];
const KANNADA_AI = DUCTUS[ductusKey("kannada", "ಐ")];
const KANNADA_VOCALIC_R = DUCTUS[ductusKey("kannada", "ಋ")];
const KANNADA_VISARGA = DUCTUS[ductusKey("kannada", "ಃ")];

const OWNER_SCRIPTS = new Set(["kannada"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { ಆ: 0.92 });

  beforeAll(() => {
    expect(verifiedLetterFont("ಅ", KANNADA_A.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಅ keeps all four animated movements in one pen-down run", () => {
    expect(penLifts(KANNADA_A)).toBe(0);
    expect(KANNADA_A.strokes).toHaveLength(1);
    expect(
      KANNADA_A.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn clockwise around the compact left loop",
      "sweep around the broad lower bowl",
      "turn counterclockwise around the rounded right loop",
      "return left along the inward horizontal bar",
    ]);
  });

  it("Kannada ಆ lifts once between the broad bowl and rounded right loop", () => {
    expect(penLifts(KANNADA_AA)).toBe(1);
    expect(
      KANNADA_AA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn clockwise around the compact left loop",
        "sweep around the broad lower bowl and finish at the upper right",
      ],
      [
        "lift, then turn clockwise around the rounded right loop",
        "return left along the inward horizontal bar",
      ],
    ]);
    expect(KANNADA_AA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-aa.gif",
    );
  });

  it("Kannada ಇ retraces the middle stem and finishes without lifting", () => {
    expect(penLifts(KANNADA_I)).toBe(0);
    expect(KANNADA_I.strokes).toHaveLength(1);
    expect(
      KANNADA_I.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "climb the left upright, turn over the first arch, and descend the middle stem",
      "retrace the middle stem upward and turn over the second arch",
      "descend through the broad outer curve and turn left along the base",
      "close the lower loop and sweep out to the right",
    ]);
  });

  it("Kannada ಉ carries both bowls through the tall arch without lifting", () => {
    expect(penLifts(KANNADA_U)).toBe(0);
    expect(KANNADA_U.strokes).toHaveLength(1);
    expect(
      KANNADA_U.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "descend through the left shoulder and sweep around the broad lower-left bowl",
      "climb over the tall middle arch and descend into the lower-right bowl",
      "sweep around the outer-right curve and finish at the open upper terminal",
    ]);
    expect(KANNADA_U.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-u.gif",
    );
  });

  it("Kannada ಊ carries both arches into the lower-right spiral without lifting", () => {
    expect(penLifts(KANNADA_UU)).toBe(0);
    expect(KANNADA_UU.strokes).toHaveLength(1);
    expect(
      KANNADA_UU.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn counterclockwise around the compact upper-left spiral",
      "descend through the left shoulder and sweep around the broad lower-left bowl",
      "climb over the first tall arch, descend through the middle trough, and climb over the second arch",
      "descend the outer-right curve and curl around the small lower-right spiral",
    ]);
    expect(KANNADA_UU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-uu.gif",
    );
  });

  it("Kannada ಎ carries both lower curves into the tall arch without lifting", () => {
    expect(penLifts(KANNADA_E)).toBe(0);
    expect(KANNADA_E.strokes).toHaveLength(1);
    expect(
      KANNADA_E.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn clockwise around the compact left loop",
      "sweep through the joined lower-left curve",
      "turn around the rounded lower-right bowl and climb its right side",
      "carry the tall outer arch over and finish to the left",
    ]);
    expect(KANNADA_E.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ae.gif",
    );
  });

  it("Kannada ಏ adds its small upper loop after one lift", () => {
    expect(penLifts(KANNADA_EE)).toBe(1);
    expect(KANNADA_EE.strokes).toHaveLength(2);
    expect(
      KANNADA_EE.strokes.flatMap((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      "turn clockwise around the compact left loop",
      "sweep through the joined lower curves and climb the right side",
      "carry the tall outer arch over and finish at the upper left",
      "draw the small upper loop from left to right",
    ]);
    expect(KANNADA_EE.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-aee.gif",
    );
  });

  it("Kannada ಒ joins its upper loop, lower bowls, and open terminal", () => {
    expect(penLifts(KANNADA_O)).toBe(0);
    expect(KANNADA_O.strokes).toHaveLength(1);
    expect(
      KANNADA_O.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "descend through the curved middle into the lower-left bowl",
      "sweep through the join and around the lower-right bowl",
      "climb the right side and curl left at the open terminal",
    ]);
    expect(KANNADA_O.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-o.gif",
    );
  });

  it("Kannada ಓ adds its small upper flourish after one lift", () => {
    expect(penLifts(KANNADA_OO)).toBe(1);
    expect(KANNADA_OO.strokes).toHaveLength(2);
    expect(
      KANNADA_OO.strokes.flatMap((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "descend through the curved middle into the lower-left bowl",
      "sweep through the join and around the lower-right bowl",
      "climb the right side and curl left at the open terminal",
      "sweep left and curl upward through the small upper flourish",
    ]);
    expect(KANNADA_OO.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-oo.gif",
    );
  });

  it("Kannada ಐ carries its spiral, right loop, and high arch without lifting", () => {
    expect(penLifts(KANNADA_AI)).toBe(0);
    expect(KANNADA_AI.strokes).toHaveLength(1);
    expect(
      KANNADA_AI.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "turn clockwise through the compact left spiral and around its lower bowl",
      "sweep through the join and around the broad right loop",
      "carry the high arch leftward and finish at the open upper-left terminal",
    ]);
    expect(KANNADA_AI.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ai.gif",
    );
  });

  it("Kannada ಋ separates its high hook and right bowl with two lifts", () => {
    expect(penLifts(KANNADA_VOCALIC_R)).toBe(2);
    expect(KANNADA_VOCALIC_R.strokes).toHaveLength(3);
    expect(
      KANNADA_VOCALIC_R.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn clockwise around the compact upper-left spiral",
        "descend through the outer curve and curl around the lower-left spiral",
        "sweep through the join and around the rounded middle bowl",
      ],
      [
        "lift, then draw the inward bar from left to right",
        "curl upward into the high hook",
      ],
      [
        "lift, then sweep rightward around the lower bowl",
        "climb the outer side and finish at the open upper terminal",
      ],
    ]);
    expect(KANNADA_VOCALIC_R.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ru.gif",
    );
  });

  it("Kannada ಃ separates its upper and lower loops with one lift", () => {
    expect(penLifts(KANNADA_VISARGA)).toBe(1);
    expect(KANNADA_VISARGA.strokes).toHaveLength(2);
    expect(
      KANNADA_VISARGA.strokes.map((stroke) => stroke.segments[0].label),
    ).toEqual([
      "draw the upper dot as a closed loop",
      "lift, then draw the lower dot as a closed loop",
    ]);
    expect(KANNADA_VISARGA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-Alphabet-Aha.gif",
    );
    expect(verifiedLetterFont("ಃ", KANNADA_VISARGA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });
});
