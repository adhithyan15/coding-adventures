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

const JAPANESE_SHI = DUCTUS[ductusKey("japanese", "し")];
const JAPANESE_KU = DUCTUS[ductusKey("japanese", "く")];
const JAPANESE_TA = DUCTUS[ductusKey("japanese", "た")];
const JAPANESE_NE = DUCTUS[ductusKey("japanese", "ね")];
const JAPANESE_MI = DUCTUS[ductusKey("japanese", "み")];
const JAPANESE_SE = DUCTUS[ductusKey("japanese", "せ")];
const JAPANESE_TE = DUCTUS[ductusKey("japanese", "て")];
const JAPANESE_NA = DUCTUS[ductusKey("japanese", "な")];
const JAPANESE_SMALL_TSU = DUCTUS[ductusKey("japanese", "っ")];
const JAPANESE_MO = DUCTUS[ductusKey("japanese", "も")];
const JAPANESE_WA = DUCTUS[ductusKey("japanese", "わ")];
const JAPANESE_YU = DUCTUS[ductusKey("japanese", "ゆ")];
const JAPANESE_YO = DUCTUS[ductusKey("japanese", "よ")];
const JAPANESE_ME = DUCTUS[ductusKey("japanese", "め")];

const OWNER_SCRIPTS = new Set(["japanese"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  // よ's sourced handwritten loop briefly bridges the open counter in the
  // bundled print outline; keep that bounded variation explicit.
  registerStrokeHonestyTests(letters, { ね: 0.88, わ: 0.88, よ: 0.95 });

  it("Japanese し descends and sweeps upward right without lifting", () => {
    expect(penLifts(JAPANESE_SHI)).toBe(0);
    expect(JAPANESE_SHI.strokes).toHaveLength(1);
    expect(
      JAPANESE_SHI.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "descend nearly straight from the top",
      "turn around the broad lower curve and sweep upward right",
    ]);
    expect(JAPANESE_SHI.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%97_stroke_order_animation.gif",
    );
  });

  it("Japanese く turns from a down-left sweep into a down-right sweep without lifting", () => {
    expect(penLifts(JAPANESE_KU)).toBe(0);
    expect(JAPANESE_KU.strokes).toHaveLength(1);
    expect(
      JAPANESE_KU.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "sweep down-left from the upper right into the central turn",
      "continue down-right to the lower tip",
    ]);
    expect(JAPANESE_KU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%8F_stroke_order_animation.gif",
    );
  });

  it("Japanese た draws four source-verified runs in order", () => {
    expect(penLifts(JAPANESE_TA)).toBe(3);
    expect(JAPANESE_TA.strokes).toHaveLength(4);
    expect(
      JAPANESE_TA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["draw the upper horizontal from left to right"],
      ["descend through the crossing stem and curve left at the foot"],
      ["draw the short right horizontal from left to right"],
      ["descend into the lower-right bowl and sweep right along its base"],
    ]);
    expect(JAPANESE_TA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%9F_stroke_order_animation.gif",
    );
  });

  it("Japanese ね draws the vertical before the crossing hook and loop", () => {
    expect(penLifts(JAPANESE_NE)).toBe(1);
    expect(JAPANESE_NE.strokes).toHaveLength(2);
    expect(
      JAPANESE_NE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["descend through the short left vertical"],
      [
        "sweep left from the upper right across the vertical",
        "hook down along the diagonal and return to the crossing",
        "finish clockwise around the lower-right loop",
      ],
    ]);
    expect(JAPANESE_NE.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%AD_stroke_order_animation.gif",
    );
    expect(JAPANESE_NE.source.citation).toMatch(
      /Sirgazil.*ね.*35 frames.*3\.5 seconds/i,
    );
  });

  it("Japanese み draws its loop before the lifted high-right sweep", () => {
    expect(penLifts(JAPANESE_MI)).toBe(1);
    expect(JAPANESE_MI.strokes).toHaveLength(2);
    expect(
      JAPANESE_MI.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "draw the top bar from left to right",
        "descend diagonally into the lower-left loop",
        "continue around the loop and sweep out through the middle",
      ],
      [
        "begin high on the right and curve down to the left",
        "turn upward at the finish",
      ],
    ]);
    expect(JAPANESE_MI.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%BF_stroke_order_animation.gif",
    );
    expect(JAPANESE_MI.source.citation).toMatch(
      /Sirgazil.*み.*29 frames.*2\.9 seconds/i,
    );
  });

  it("Japanese せ draws the crossing bar before its two lifted stems", () => {
    expect(penLifts(JAPANESE_SE)).toBe(2);
    expect(JAPANESE_SE.strokes).toHaveLength(3);
    expect(
      JAPANESE_SE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["draw the long crossing horizontal from left to right"],
      ["descend through the left crossing", "curve right along the base"],
      ["descend through the right crossing", "hook left at the finish"],
    ]);
    expect(JAPANESE_SE.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%9B_stroke_order_animation.gif",
    );
    expect(JAPANESE_SE.source.citation).toMatch(
      /Sirgazil.*せ.*33 frames.*3\.3 seconds/i,
    );
  });

  it("Japanese て keeps its bar, return, and lower curve in one run", () => {
    expect(penLifts(JAPANESE_TE)).toBe(0);
    expect(JAPANESE_TE.strokes).toHaveLength(1);
    expect(
      JAPANESE_TE.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "draw the high horizontal from left to right",
      "turn back down and left through the diagonal",
      "round the broad lower curve and sweep right to the finish",
    ]);
    expect(JAPANESE_TE.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%A6_stroke_order_animation.gif",
    );
    expect(JAPANESE_TE.source.citation).toMatch(
      /Sirgazil.*て.*28 frames.*2\.8 seconds/i,
    );
  });

  it("Japanese な draws three lifted marks before its looping body", () => {
    expect(penLifts(JAPANESE_NA)).toBe(3);
    expect(JAPANESE_NA.strokes).toHaveLength(4);
    expect(
      JAPANESE_NA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["draw the upper-left horizontal from left to right"],
      ["descend through the crossing left-falling stem"],
      ["draw the short upper-right diagonal down and right"],
      [
        "descend through the lower-right stem",
        "turn around the loop and sweep right to the finish",
      ],
    ]);
    expect(JAPANESE_NA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%AA_stroke_order_animation.gif",
    );
    expect(JAPANESE_NA.source.citation).toMatch(
      /Sirgazil.*な.*32 frames.*3\.2 seconds/i,
    );
  });

  it("Japanese small っ scales つ's one-run movement to its own glyph", () => {
    expect(penLifts(JAPANESE_SMALL_TSU)).toBe(0);
    expect(JAPANESE_SMALL_TSU.strokes).toHaveLength(1);
    expect(
      JAPANESE_SMALL_TSU.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "begin at the upper left and sweep right across the high shoulder",
      "round down the right side and finish by sweeping left along the lower curve",
    ]);
    expect(JAPANESE_SMALL_TSU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%A4_stroke_order_animation.gif",
    );
    expect(JAPANESE_SMALL_TSU.source.citation).toMatch(
      /Sirgazil.*つ.*24 frames.*Unicode Standard 17\.0.*U\+3063/i,
    );
    expect(JAPANESE_SMALL_TSU.source.variation).toMatch(
      /one uninterrupted run.*small tsu.*scaling.*explicit/i,
    );
  });

  it("Japanese も draws its bowl before two lifted left-to-right bars", () => {
    expect(penLifts(JAPANESE_MO)).toBe(2);
    expect(JAPANESE_MO.strokes).toHaveLength(3);
    expect(
      JAPANESE_MO.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["descend and turn around the broad lower bowl to the rising right tip"],
      ["draw the upper horizontal from left to right across the stem"],
      ["draw the lower horizontal from left to right across the stem"],
    ]);
    expect(JAPANESE_MO.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%82_stroke_order_animation.gif",
    );
  });

  it("Japanese わ draws the vertical before the crossing hook and broad loop", () => {
    expect(penLifts(JAPANESE_WA)).toBe(1);
    expect(JAPANESE_WA.strokes).toHaveLength(2);
    expect(
      JAPANESE_WA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["descend through the long left vertical"],
      [
        "sweep right from the upper left across the vertical",
        "hook down and left, then return through the central crossing",
        "continue clockwise around the broad right loop",
      ],
    ]);
    expect(JAPANESE_WA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%8F_stroke_order_animation.gif",
    );
    expect(JAPANESE_WA.source.citation).toMatch(
      /Sirgazil.*わ.*30 frames.*3\.0 seconds/i,
    );
  });

  it("Japanese ゆ draws its broad loop before the central descending curve", () => {
    expect(penLifts(JAPANESE_YU)).toBe(1);
    expect(JAPANESE_YU.strokes).toHaveLength(2);
    expect(
      JAPANESE_YU.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "descend through the left stem and turn up across the high shoulder",
        "continue clockwise around the broad loop",
        "curve left to the inner finish",
      ],
      [
        "descend through the center of the loop",
        "curve down and left to the finish",
      ],
    ]);
    expect(JAPANESE_YU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%86_stroke_order_animation.gif",
    );
    expect(JAPANESE_YU.source.citation).toMatch(
      /Sirgazil.*ゆ.*30 frames.*3\.0 seconds/i,
    );
  });

  it("Japanese よ draws its corrected left-to-right bar before the looping stem", () => {
    expect(penLifts(JAPANESE_YO)).toBe(1);
    expect(JAPANESE_YO.strokes).toHaveLength(2);
    expect(
      JAPANESE_YO.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["draw the short upper horizontal from left to right"],
      [
        "descend through the upper bar and turn left",
        "continue clockwise around the broad lower loop to the rightward finish",
      ],
    ]);
    expect(JAPANESE_YO.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%88_stroke_order_animation.gif",
    );
    expect(JAPANESE_YO.source.citation).toMatch(
      /Sirgazil.*よ.*26 frames.*2\.6 seconds.*corrected.*4 January 2012/i,
    );
  });

  it("Japanese め draws the short left curve before its crossing paired loop", () => {
    expect(penLifts(JAPANESE_ME)).toBe(1);
    expect(JAPANESE_ME.strokes).toHaveLength(2);
    expect(
      JAPANESE_ME.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["descend from the upper left and curve down and right"],
      [
        "descend diagonally left through the first stroke",
        "loop around the lower left and sweep upward across the top",
        "continue clockwise around the broad right curve to the lower finish",
      ],
    ]);
    expect(JAPANESE_ME.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%81_stroke_order_animation.gif",
    );
    expect(JAPANESE_ME.source.citation).toMatch(
      /Sirgazil.*め.*32 frames.*3\.2 seconds.*1 October 2009/i,
    );
  });
});
