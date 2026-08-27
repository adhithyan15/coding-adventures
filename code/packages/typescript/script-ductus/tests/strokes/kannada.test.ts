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
const KANNADA_E = DUCTUS[ductusKey("kannada", "ಎ")];
const KANNADA_EE = DUCTUS[ductusKey("kannada", "ಏ")];
const KANNADA_O = DUCTUS[ductusKey("kannada", "ಒ")];

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
});
