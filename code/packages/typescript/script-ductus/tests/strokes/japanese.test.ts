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
const JAPANESE_TSU = DUCTUS[ductusKey("japanese", "つ")];
const JAPANESE_RO = DUCTUS[ductusKey("japanese", "ろ")];
const JAPANESE_SMALL_YU = DUCTUS[ductusKey("japanese", "ゅ")];
const JAPANESE_NO = DUCTUS[ductusKey("japanese", "の")];
const JAPANESE_HI = DUCTUS[ductusKey("japanese", "ひ")];
const JAPANESE_FU = DUCTUS[ductusKey("japanese", "ふ")];
const JAPANESE_HO = DUCTUS[ductusKey("japanese", "ほ")];
const JAPANESE_MU = DUCTUS[ductusKey("japanese", "む")];
const JAPANESE_YA = DUCTUS[ductusKey("japanese", "や")];
const JAPANESE_SMALL_YA = DUCTUS[ductusKey("japanese", "ゃ")];
const JAPANESE_SMALL_YO = DUCTUS[ductusKey("japanese", "ょ")];
const JAPANESE_WO = DUCTUS[ductusKey("japanese", "を")];
const JAPANESE_SO = DUCTUS[ductusKey("japanese", "そ")];
const JAPANESE_RE = DUCTUS[ductusKey("japanese", "れ")];
const JAPANESE_RU = DUCTUS[ductusKey("japanese", "る")];
const JAPANESE_KI = DUCTUS[ductusKey("japanese", "き")];
const JAPANESE_KE = DUCTUS[ductusKey("japanese", "け")];
const JAPANESE_NU = DUCTUS[ductusKey("japanese", "ぬ")];
const JAPANESE_HE = DUCTUS[ductusKey("japanese", "へ")];
const JAPANESE_RA = DUCTUS[ductusKey("japanese", "ら")];
const JAPANESE_A = DUCTUS[ductusKey("japanese", "あ")];
const JAPANESE_I = DUCTUS[ductusKey("japanese", "い")];
const JAPANESE_U = DUCTUS[ductusKey("japanese", "う")];
const JAPANESE_E = DUCTUS[ductusKey("japanese", "え")];
const JAPANESE_O = DUCTUS[ductusKey("japanese", "お")];
const JAPANESE_KA = DUCTUS[ductusKey("japanese", "か")];
const JAPANESE_KO = DUCTUS[ductusKey("japanese", "こ")];
const JAPANESE_SA = DUCTUS[ductusKey("japanese", "さ")];
const JAPANESE_SU = DUCTUS[ductusKey("japanese", "す")];
const JAPANESE_CHI = DUCTUS[ductusKey("japanese", "ち")];
const JAPANESE_TO = DUCTUS[ductusKey("japanese", "と")];

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

  it("Japanese つ completes its arch and lower return without lifting", () => {
    expect(penLifts(JAPANESE_TSU)).toBe(0);
    expect(JAPANESE_TSU.strokes).toHaveLength(1);
    expect(
      JAPANESE_TSU.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "sweep right from the upper left across the high arch",
      "turn down around the right side and return left along the broad lower curve",
    ]);
    expect(JAPANESE_TSU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%A4_stroke_order_animation.gif",
    );
    expect(JAPANESE_TSU.source.citation).toMatch(
      /Sirgazil.*つ.*24 frames.*2\.4 seconds.*1 October 2009/i,
    );
  });

  // The two signs the cardinals one to ten cost. ろ carries its own frame-by-frame
  // observation; ゅ carries ゆ's, under the same named rule っ uses from つ.
  it("Japanese ろ draws shoulder, diagonal and belly in one unbroken run", () => {
    expect(penLifts(JAPANESE_RO)).toBe(0);
    expect(JAPANESE_RO.strokes).toHaveLength(1);
    expect(
      JAPANESE_RO.strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "begin at the upper left and draw the short high shoulder to the right",
      "turn down at the corner and descend the long diagonal to the lower left",
      "swing right into the broad clockwise belly and finish with a short tail at the bottom",
    ]);
    expect(JAPANESE_RO.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%8D_stroke_order_animation.gif",
    );
    expect(JAPANESE_RO.source.citation).toMatch(
      /Sirgazil.*ろ.*26 frames.*2\.6 seconds.*1 October 2009/i,
    );
    // The pen reaches the FOOT of the diagonal before the belly departs, and it
    // has to: a path that turns at the belly's departure instead leaves ~10% of
    // the letter's ink more than 100 units from any stroke, which the coverage
    // check in registerStrokeHonestyTests rejects.
    const diagonal = JAPANESE_RO.strokes[0].segments[1].path;
    const foot = diagonal[diagonal.length - 1];
    expect(foot.x).toBeLessThan(200);
    expect(foot.y).toBeLessThan(300);
  });

  it("Japanese small ゅ scales ゆ's two-run movement to its own glyph", () => {
    expect(penLifts(JAPANESE_SMALL_YU)).toBe(1);
    expect(JAPANESE_SMALL_YU.strokes).toHaveLength(2);
    // The labels are ゆ's, verbatim, because the movement is ゆ's. Only the
    // coordinates are the small glyph's.
    expect(
      JAPANESE_SMALL_YU.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual(
      JAPANESE_YU.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    );
    expect(JAPANESE_SMALL_YU.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%86_stroke_order_animation.gif",
    );
    expect(JAPANESE_SMALL_YU.source.citation).toMatch(
      /Sirgazil.*ゆ.*30 frames.*U\+3085 HIRAGANA LETTER SMALL YU/i,
    );
    // Phrase-by-phrase rather than one regex with six greedy `.*` gaps between
    // them: that shape backtracks polynomially on a subject that does NOT match
    // (measured at 23 seconds for a 5,000-character one), and each phrase is a
    // separate claim anyway, so a failure names which one went missing. The
    // older assertions in this file still carry the greedy shape and are worth
    // the same treatment when they are next touched.
    for (const phrase of [
      "two pen-down runs",
      "one lift",
      "small yu",
      "scaling",
      "explicit rather than presented as independent handwriting evidence",
    ]) {
      expect(JAPANESE_SMALL_YU.source.variation, phrase).toContain(phrase);
    }
    // Every point sits inside the SMALL glyph's own box, so the entry is not the
    // full-size path wearing a smaller caption.
    const yuPoints = JAPANESE_YU.strokes.flatMap((stroke) =>
      stroke.segments.flatMap((segment) => segment.path),
    );
    const smallPoints = JAPANESE_SMALL_YU.strokes.flatMap((stroke) =>
      stroke.segments.flatMap((segment) => segment.path),
    );
    const top = (points: Point[]) => Math.max(...points.map((p) => p.y));
    const right = (points: Point[]) => Math.max(...points.map((p) => p.x));
    expect(top(smallPoints)).toBeLessThan(top(yuPoints));
    expect(right(smallPoints)).toBeLessThan(right(yuPoints));
  });

  // Small ゃ and ょ follow the rule small ゅ set: the movement and captions are
  // the full-size sign's, the coordinates are the small glyph's own. Both are
  // pinned the same way, and both must sit inside a smaller box than their
  // full-size twin so neither can be the big path wearing a small caption.
  for (const [small, full, name, url, lifts] of [
    [
      JAPANESE_SMALL_YA,
      JAPANESE_YA,
      "small ya",
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%84_stroke_order_animation.gif",
      2,
    ],
    [
      JAPANESE_SMALL_YO,
      JAPANESE_YO,
      "small yo",
      "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%88_stroke_order_animation.gif",
      1,
    ],
  ] as const) {
    it(`Japanese ${small.glyph} scales ${full.glyph}'s movement to its own glyph`, () => {
      expect(penLifts(small)).toBe(lifts);
      expect(penLifts(full)).toBe(lifts);
      expect(
        small.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      ).toEqual(
        full.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      );
      expect(small.source.url).toBe(url);
      // Phrase by phrase, for the reason given in the small ゅ test above.
      for (const phrase of [
        "Sirgazil",
        "KanjiVG",
        `HIRAGANA LETTER ${name.toUpperCase()}`,
      ]) {
        expect(small.source.citation, phrase).toContain(phrase);
      }
      for (const phrase of [
        name,
        "scaling",
        "explicit rather than presented as independent handwriting evidence",
      ]) {
        expect(small.source.variation, phrase).toContain(phrase);
      }
      const points = (letter: LetterDuctus) =>
        letter.strokes.flatMap((stroke) =>
          stroke.segments.flatMap((segment) => segment.path),
        );
      const top = (list: Point[]) => Math.max(...list.map((p) => p.y));
      const right = (list: Point[]) => Math.max(...list.map((p) => p.x));
      expect(top(points(small))).toBeLessThan(top(points(full)));
      expect(right(points(small))).toBeLessThan(right(points(full)));
    });
  }

  it("Japanese を draws bar, diagonal-and-arch, then the separate lower curve", () => {
    // KanjiVG's 03092 file is three directed paths, so three runs and two
    // lifts. Stroke 2 does not lift at its sharp foot: the turn is a segment
    // boundary, and the join between the two segments is exact.
    expect(penLifts(JAPANESE_WO)).toBe(2);
    expect(
      JAPANESE_WO.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["draw the short upper bar from left to right"],
      [
        "start above the bar and cut down-left through it",
        "turn up and right into the arch, then down the stem",
      ],
      [
        "start at the right and sweep down-left across the middle",
        "round the lower left and finish along the base to the right",
      ],
    ]);
    for (const stroke of JAPANESE_WO.strokes) {
      for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
    }
    expect(JAPANESE_WO.source.url).toBe(
      "https://github.com/KanjiVG/kanjivg/blob/master/kanji/03092.svg",
    );
    for (const phrase of ["KanjiVG", "U+3092 HIRAGANA LETTER WO", "CC BY-SA 3.0"]) {
      expect(JAPANESE_WO.source.citation, phrase).toContain(phrase);
    }
    // The bar is drawn left to right, the diagonal starts ABOVE the bar, and
    // the lower curve starts on the right and ends on the right after rounding
    // the left: the three directional claims the caption makes.
    const [bar, hook, curve] = JAPANESE_WO.strokes.map((stroke) => penPath(stroke));
    expect(bar[0].x).toBeLessThan(bar[bar.length - 1].x);
    expect(hook[0].y).toBeGreaterThan(Math.max(...bar.map((p) => p.y)));
    const curveLeft = Math.min(...curve.map((p) => p.x));
    expect(curve[0].x).toBeGreaterThan(curveLeft + 400);
    expect(curve[curve.length - 1].x).toBeGreaterThan(curveLeft + 400);
  });

  // Chapter 132's three signs are fitted from KanjiVG's directed paths the
  // way を is: the file supplies the order and direction, the bundled outline
  // supplies the coordinates. Each test pins the run and lift counts, the
  // labels in order, exact joins, the citation, and the directional claims
  // the captions make.
  for (const [letter, file, name, labels] of [
    [
      JAPANESE_SO,
      "0305d",
      "U+305D HIRAGANA LETTER SO",
      [
        [
          "draw the short top bar from left to right",
          "turn and run down-left to the bar",
          "swing back right along the long bar",
          "double back and drop into the curve",
          "round the lower left and finish on the base",
        ],
      ],
    ],
    [
      JAPANESE_RE,
      "0308c",
      "U+308C HIRAGANA LETTER RE",
      [
        ["draw the vertical from top to bottom"],
        [
          "draw the short bar from left to right",
          "turn and cut down to the lower left",
          "climb back up and rise right over the arch",
          "come down and flick up to the right",
        ],
      ],
    ],
    [
      JAPANESE_RU,
      "0308b",
      "U+308B HIRAGANA LETTER RU",
      [
        [
          "draw the short top bar from left to right",
          "turn and run the long diagonal down-left",
          "swing back right and round the big bowl",
          "curl into the small loop at the base",
        ],
      ],
    ],
  ] as const) {
    it(`Japanese ${letter.glyph} follows KanjiVG's ${labels.length}-path order on its own outline`, () => {
      expect(letter.strokes).toHaveLength(labels.length);
      expect(penLifts(letter)).toBe(labels.length - 1);
      expect(
        letter.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      ).toEqual(labels);
      for (const stroke of letter.strokes) {
        for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
      }
      expect(letter.source.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of [
        "KanjiVG",
        name,
        "Ulrich Apel and contributors, CC BY-SA 3.0",
      ]) {
        expect(letter.source.citation, phrase).toContain(phrase);
      }
      for (const phrase of [
        "Only the order and direction",
        "Noto Sans JP outline's own medial line",
      ]) {
        expect(letter.source.variation, phrase).toContain(phrase);
      }
    });
  }

  it("Japanese そ, る and れ run in the directions their captions claim", () => {
    // そ and る both open with a top bar drawn left to right that turns
    // sharply into a stroke heading down and LEFT.
    for (const letter of [JAPANESE_SO, JAPANESE_RU]) {
      const [bar, diagonal] = letter.strokes[0].segments.map(
        (segment) => segment.path,
      );
      expect(bar[0].x).toBeLessThan(bar[bar.length - 1].x);
      expect(diagonal[diagonal.length - 1].x).toBeLessThan(diagonal[0].x - 300);
      expect(diagonal[diagonal.length - 1].y).toBeLessThan(diagonal[0].y - 250);
    }
    // そ swings back right along the long bar, then finishes on the right of
    // its curve's leftmost point, along the base.
    const so = penPath(JAPANESE_SO.strokes[0]);
    const across = JAPANESE_SO.strokes[0].segments[2].path;
    expect(across[across.length - 1].x).toBeGreaterThan(across[0].x + 500);
    const soLeftOfCurve = Math.min(
      ...JAPANESE_SO.strokes[0].segments[4].path.map((p) => p.x),
    );
    expect(so[so.length - 1].x).toBeGreaterThan(soLeftOfCurve + 250);
    expect(so[so.length - 1].y).toBeLessThan(50);
    // る's loop closes: the finish sits back inside the bowl, right of the
    // loop's leftmost point and below its top.
    const loop = JAPANESE_RU.strokes[0].segments[3].path;
    const loopLeft = Math.min(...loop.map((p) => p.x));
    const loopTop = Math.max(...loop.map((p) => p.y));
    const ruEnd = loop[loop.length - 1];
    expect(ruEnd.x).toBeGreaterThan(loopLeft + 150);
    expect(ruEnd.y).toBeLessThan(loopTop - 100);
    // れ: the vertical runs top to bottom, the bar starts LEFT of it, and the
    // second stroke finishes far to the right with an upward flick.
    const vertical = penPath(JAPANESE_RE.strokes[0]);
    expect(vertical[0].y).toBeGreaterThan(vertical[vertical.length - 1].y + 600);
    const second = penPath(JAPANESE_RE.strokes[1]);
    expect(second[0].x).toBeLessThan(Math.min(...vertical.map((p) => p.x)));
    const flick = JAPANESE_RE.strokes[1].segments[3].path;
    const footY = Math.min(...flick.map((p) => p.y));
    expect(flick[flick.length - 1].y).toBeGreaterThan(footY + 50);
    expect(flick[flick.length - 1].x).toBeGreaterThan(
      Math.max(...vertical.map((p) => p.x)) + 500,
    );
  });

  // Chapter 133's four signs, the last of the basic hiragana, are fitted the
  // same way: KanjiVG's file supplies the order and direction, the bundled
  // outline supplies the coordinates. Each test pins the run and lift counts,
  // the labels in order, exact joins and the citation.
  for (const [letter, file, name, labels] of [
    [
      JAPANESE_KI,
      "0304d",
      "U+304D HIRAGANA LETTER KI",
      [
        ["draw the upper bar from left to right"],
        ["draw the lower bar from left to right"],
        [
          "run the long diagonal down to the right",
          "turn sharply and hook back to the left",
        ],
        ["curve down and along the base"],
      ],
    ],
    [
      JAPANESE_KE,
      "03051",
      "U+3051 HIRAGANA LETTER KE",
      [
        ["draw the left vertical down", "turn and flick up to the right"],
        ["draw the bar from left to right"],
        ["run down and sweep to the lower left"],
      ],
    ],
    [
      JAPANESE_NU,
      "0306c",
      "U+306C HIRAGANA LETTER NU",
      [
        ["draw the short diagonal"],
        [
          "cut down to the lower left",
          "loop up and over the arch",
          "come down the right side",
          "tie a small loop and flick out",
        ],
      ],
    ],
    [
      JAPANESE_HE,
      "03078",
      "U+3078 HIRAGANA LETTER HE",
      [["rise up to the peak", "run down to the right"]],
    ],
    // ら has been written since chapter 8, but only now has a record and a
    // ductus; it is fitted the same way.
    [
      JAPANESE_RA,
      "03089",
      "U+3089 HIRAGANA LETTER RA",
      [
        ["draw the top stroke to the right"],
        ["run down the left side", "turn up and round the open bowl"],
      ],
    ],
  ] as const) {
    it(`Japanese ${letter.glyph} follows KanjiVG's ${labels.length}-path order on its own outline`, () => {
      expect(letter.strokes).toHaveLength(labels.length);
      expect(penLifts(letter)).toBe(labels.length - 1);
      expect(
        letter.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      ).toEqual(labels);
      for (const stroke of letter.strokes) {
        for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
      }
      expect(letter.source.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of [
        "KanjiVG",
        name,
        "Ulrich Apel and contributors, CC BY-SA 3.0",
      ]) {
        expect(letter.source.citation, phrase).toContain(phrase);
      }
      for (const phrase of [
        "Only the order and direction",
        "Noto Sans JP outline's own medial line",
      ]) {
        expect(letter.source.variation, phrase).toContain(phrase);
      }
    });
  }

  it("Japanese き, け, ぬ, へ and ら run in the directions their captions claim", () => {
    const first = (path: readonly Point[]) => path[0];
    const last = (path: readonly Point[]) => path[path.length - 1];
    // き: both bars run rightward; the diagonal runs down to the RIGHT and
    // hooks back LEFT; the separate curve drops from the left and finishes
    // along the base, far to the right of where it began.
    for (const stroke of JAPANESE_KI.strokes.slice(0, 2)) {
      const bar = penPath(stroke);
      expect(first(bar).x).toBeLessThan(last(bar).x - 400);
    }
    const [diagonal, hook] = JAPANESE_KI.strokes[2].segments.map(
      (segment) => segment.path,
    );
    expect(last(diagonal).y).toBeLessThan(first(diagonal).y - 400);
    expect(last(diagonal).x).toBeGreaterThan(first(diagonal).x + 150);
    expect(last(hook).x).toBeLessThan(first(hook).x - 100);
    const kiCurve = penPath(JAPANESE_KI.strokes[3]);
    expect(last(kiCurve).y).toBeLessThan(first(kiCurve).y - 150);
    expect(last(kiCurve).x).toBeGreaterThan(first(kiCurve).x + 300);
    expect(Math.min(...kiCurve.map((p) => p.x))).toBeLessThan(first(kiCurve).x);
    // け: the left stroke comes down, then flicks up and to the right; the
    // bar runs rightward; the long stroke comes down and ends to the LEFT.
    const [down, flick] = JAPANESE_KE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(last(down).y).toBeLessThan(first(down).y - 600);
    expect(last(flick).y).toBeGreaterThan(first(flick).y + 200);
    expect(last(flick).x).toBeGreaterThan(first(flick).x + 50);
    const keBar = penPath(JAPANESE_KE.strokes[1]);
    expect(first(keBar).x).toBeLessThan(last(keBar).x - 400);
    const sweep = penPath(JAPANESE_KE.strokes[2]);
    expect(last(sweep).y).toBeLessThan(first(sweep).y - 600);
    expect(last(sweep).x).toBeLessThan(first(sweep).x - 100);
    // ぬ: the short stroke runs down to the right; the long one starts above
    // it, cuts down to the LEFT, and its last part swings back left round
    // the small loop before it flicks out further right than it began.
    const nuShort = penPath(JAPANESE_NU.strokes[0]);
    expect(last(nuShort).x).toBeGreaterThan(first(nuShort).x + 150);
    expect(last(nuShort).y).toBeLessThan(first(nuShort).y - 300);
    const nuParts = JAPANESE_NU.strokes[1].segments.map(
      (segment) => segment.path,
    );
    expect(first(nuParts[0]).y).toBeGreaterThan(first(nuShort).y);
    expect(last(nuParts[0]).x).toBeLessThan(first(nuParts[0]).x - 200);
    const loop = nuParts[3];
    expect(Math.min(...loop.map((p) => p.x))).toBeLessThan(first(loop).x - 200);
    expect(last(loop).x).toBeGreaterThan(first(loop).x + 50);
    // へ: the peak is the highest point, it sits left of the middle, and the
    // stroke finishes lower than it started.
    const [rise, fall] = JAPANESE_HE.strokes[0].segments.map(
      (segment) => segment.path,
    );
    const peak = last(rise);
    expect(peak.y).toBeGreaterThan(first(rise).y + 250);
    expect(peak.y).toBeGreaterThan(last(fall).y + 400);
    expect(peak.x - first(rise).x).toBeLessThan(last(fall).x - peak.x);
    expect(last(fall).y).toBeLessThan(first(rise).y);
    // ら: the top stroke runs rightward; the left stroke comes down, then the
    // bowl rises to the right of it and finishes low, left of its widest point.
    const raTop = penPath(JAPANESE_RA.strokes[0]);
    expect(first(raTop).x).toBeLessThan(last(raTop).x - 200);
    const [raDown, bowl] = JAPANESE_RA.strokes[1].segments.map(
      (segment) => segment.path,
    );
    expect(last(raDown).y).toBeLessThan(first(raDown).y - 250);
    expect(Math.max(...bowl.map((p) => p.y))).toBeGreaterThan(first(bowl).y + 100);
    const bowlRight = Math.max(...bowl.map((p) => p.x));
    expect(bowlRight).toBeGreaterThan(first(bowl).x + 400);
    expect(last(bowl).x).toBeLessThan(bowlRight - 300);
    expect(last(bowl).y).toBeLessThan(first(bowl).y - 150);
  });

  // あ, い, う, え, お and か have been written since chapters 1, 3 and 10 but
  // had no cited stroke-order source until now, so their writing lessons
  // printed no filmstrip. They are fitted the same way as き, け, ぬ, へ and
  // ら: KanjiVG's file supplies the order and direction, the bundled outline
  // supplies the coordinates.
  for (const [letter, file, name, labels] of [
    [
      JAPANESE_A,
      "03042",
      "U+3042 HIRAGANA LETTER A",
      [
        ["draw the bar from left to right"],
        ["draw the vertical down"],
        [
          "cut down to the lower left",
          "loop up across the vertical",
          "round the right side and finish low",
        ],
      ],
    ],
    [
      JAPANESE_I,
      "03044",
      "U+3044 HIRAGANA LETTER I",
      [
        ["come down the left side", "curve round and flick up to the right"],
        ["draw the short stroke down"],
      ],
    ],
    [
      JAPANESE_U,
      "03046",
      "U+3046 HIRAGANA LETTER U",
      [
        ["draw the short top stroke to the right"],
        [
          "rise to the right along the top",
          "curve down and sweep to the lower left",
        ],
      ],
    ],
    [
      JAPANESE_E,
      "03048",
      "U+3048 HIRAGANA LETTER E",
      [
        ["draw the short top stroke to the right"],
        [
          "draw the bar to the right",
          "cut down to the lower left",
          "climb back up the diagonal",
          "turn over the hump and down",
          "run along the base to the right",
        ],
      ],
    ],
    [
      JAPANESE_O,
      "0304a",
      "U+304A HIRAGANA LETTER O",
      [
        ["draw the bar from left to right"],
        [
          "run the long stroke down",
          "turn and loop up across it",
          "round the right side and finish low",
        ],
        ["draw the dot down to the right"],
      ],
    ],
    [
      JAPANESE_KA,
      "0304b",
      "U+304B HIRAGANA LETTER KA",
      [
        [
          "draw the bar to the right",
          "turn down the right side",
          "hook back to the left",
        ],
        ["draw the long stroke down to the left"],
        ["draw the dot down to the right"],
      ],
    ],
  ] as const) {
    it(`Japanese ${letter.glyph} follows KanjiVG's ${labels.length}-path order on its own outline`, () => {
      expect(letter.strokes).toHaveLength(labels.length);
      expect(penLifts(letter)).toBe(labels.length - 1);
      expect(
        letter.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      ).toEqual(labels);
      for (const stroke of letter.strokes) {
        for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
      }
      expect(letter.source.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of [
        "KanjiVG",
        name,
        "Ulrich Apel and contributors, CC BY-SA 3.0",
      ]) {
        expect(letter.source.citation, phrase).toContain(phrase);
      }
      for (const phrase of [
        "Only the order and direction",
        "Noto Sans JP outline's own medial line",
      ]) {
        expect(letter.source.variation, phrase).toContain(phrase);
      }
    });
  }

  it("Japanese あ, い, う, え, お and か run in the directions their captions claim", () => {
    const first = (path: readonly Point[]) => path[0];
    const last = (path: readonly Point[]) => path[path.length - 1];
    const parts = (letter: LetterDuctus, stroke: number) =>
      letter.strokes[stroke].segments.map((segment) => segment.path);
    // Every bar and every short top stroke runs rightward.
    for (const bar of [
      penPath(JAPANESE_A.strokes[0]),
      penPath(JAPANESE_U.strokes[0]),
      penPath(JAPANESE_E.strokes[0]),
      penPath(JAPANESE_O.strokes[0]),
      parts(JAPANESE_E, 1)[0],
      parts(JAPANESE_KA, 0)[0],
    ]) {
      expect(first(bar).x).toBeLessThan(last(bar).x - 250);
    }
    // あ: the vertical comes down; the third stroke cuts down to the LOWER
    // LEFT, loops back up across the vertical, and rounds the bowl on the
    // right to finish low, heading left.
    const aVertical = penPath(JAPANESE_A.strokes[1]);
    expect(last(aVertical).y).toBeLessThan(first(aVertical).y - 600);
    const [aCut, aLoop, aRound] = parts(JAPANESE_A, 2);
    expect(last(aCut).x).toBeLessThan(first(aCut).x - 400);
    expect(last(aCut).y).toBeLessThan(first(aCut).y - 400);
    expect(Math.min(...aLoop.map((p) => p.x))).toBeLessThan(last(aCut).x);
    expect(last(aLoop).y).toBeGreaterThan(first(aLoop).y + 300);
    expect(first(aLoop).x).toBeLessThan(last(aVertical).x);
    expect(last(aLoop).x).toBeGreaterThan(last(aVertical).x);
    expect(last(aRound).y).toBeLessThan(first(aRound).y - 300);
    expect(Math.max(...aRound.map((p) => p.x))).toBeGreaterThan(last(aRound).x + 150);
    expect(last(aRound).x).toBeLessThan(aRound[aRound.length - 2].x);
    // い: the left stroke comes down, then flicks up and to the right; the
    // short right stroke starts right of all of it and runs down.
    const [iDown, iFlick] = parts(JAPANESE_I, 0);
    expect(last(iDown).y).toBeLessThan(first(iDown).y - 500);
    expect(last(iFlick).y).toBeGreaterThan(first(iFlick).y + 100);
    expect(last(iFlick).x).toBeGreaterThan(first(iFlick).x + 100);
    const iRight = penPath(JAPANESE_I.strokes[1]);
    expect(first(iRight).x).toBeGreaterThan(
      Math.max(...penPath(JAPANESE_I.strokes[0]).map((p) => p.x)),
    );
    expect(last(iRight).y).toBeLessThan(first(iRight).y - 300);
    // う: the curve rises to the right along the top, then swings out to the
    // right and comes down to finish at the LOWER LEFT.
    const [uRise, uSweep] = parts(JAPANESE_U, 1);
    expect(last(uRise).x).toBeGreaterThan(first(uRise).x + 300);
    expect(last(uRise).y).toBeGreaterThan(first(uRise).y);
    expect(Math.max(...uSweep.map((p) => p.x))).toBeGreaterThan(first(uSweep).x + 50);
    expect(last(uSweep).y).toBeLessThan(first(uSweep).y - 400);
    expect(last(uSweep).x).toBeLessThan(first(uSweep).x - 200);
    // え: after the bar the body cuts down to the LOWER LEFT, climbs back up
    // the same ink, turns over the hump and down, and runs out to the right.
    const [, eCut, eClimb, eHump, eBase] = parts(JAPANESE_E, 1);
    expect(last(eCut).x).toBeLessThan(first(eCut).x - 400);
    expect(last(eCut).y).toBeLessThan(first(eCut).y - 400);
    expect(last(eClimb).y).toBeGreaterThan(first(eClimb).y + 200);
    for (const p of eClimb) {
      expect(
        Math.min(...eCut.map((q) => Math.hypot(p.x - q.x, p.y - q.y))),
      ).toBeLessThan(40);
    }
    expect(last(eHump).y).toBeLessThan(first(eHump).y - 150);
    expect(last(eBase).x).toBeGreaterThan(first(eBase).x + 200);
    expect(Math.abs(last(eBase).y - first(eBase).y)).toBeLessThan(50);
    // お: the long stroke comes down, turns at its foot and loops up to the
    // LEFT and back across itself, then rounds the bowl to finish low; the
    // dot sits right of the bar and runs down to the right.
    const [oDown, oLoop, oRound] = parts(JAPANESE_O, 1);
    expect(last(oDown).y).toBeLessThan(first(oDown).y - 600);
    expect(Math.min(...oLoop.map((p) => p.x))).toBeLessThan(first(oLoop).x - 150);
    expect(last(oLoop).x).toBeGreaterThan(first(oDown).x);
    expect(last(oLoop).y).toBeGreaterThan(first(oLoop).y + 300);
    expect(last(oRound).y).toBeLessThan(first(oRound).y - 300);
    expect(Math.max(...oRound.map((p) => p.x))).toBeGreaterThan(last(oRound).x + 200);
    const oShort = penPath(JAPANESE_O.strokes[2]);
    expect(first(oShort).x).toBeGreaterThan(last(penPath(JAPANESE_O.strokes[0])).x);
    expect(last(oShort).x).toBeGreaterThan(first(oShort).x + 100);
    expect(last(oShort).y).toBeLessThan(first(oShort).y - 50);
    // か: the bar turns DOWN the right side and hooks back LEFT; the long
    // stroke falls to the LOWER LEFT; the dot runs down to the right, right
    // of where the bar turned.
    const [, kaDown, kaHook] = parts(JAPANESE_KA, 0);
    expect(last(kaDown).y).toBeLessThan(first(kaDown).y - 400);
    expect(last(kaHook).x).toBeLessThan(first(kaHook).x - 100);
    const kaFall = penPath(JAPANESE_KA.strokes[1]);
    expect(last(kaFall).y).toBeLessThan(first(kaFall).y - 600);
    expect(last(kaFall).x).toBeLessThan(first(kaFall).x - 200);
    const kaShort = penPath(JAPANESE_KA.strokes[2]);
    expect(first(kaShort).x).toBeGreaterThan(first(kaDown).x);
    expect(last(kaShort).x).toBeGreaterThan(first(kaShort).x + 100);
    expect(last(kaShort).y).toBeLessThan(first(kaShort).y - 200);
  });

  // こ, さ, す, ち and と have been written since chapters 2 to 4 but had no
  // cited stroke-order source until now, so their writing lessons printed no
  // filmstrip. They are fitted the same way as あ to か: KanjiVG's file
  // supplies the order and direction, the bundled outline supplies the
  // coordinates.
  for (const [letter, file, name, labels] of [
    [
      JAPANESE_KO,
      "03053",
      "U+3053 HIRAGANA LETTER KO",
      [
        ["draw the top stroke to the right"],
        ["curve down and run out to the right"],
      ],
    ],
    [
      JAPANESE_SA,
      "03055",
      "U+3055 HIRAGANA LETTER SA",
      [
        ["draw the bar to the right"],
        ["slant down to the right through the bar", "hook back to the left"],
        ["curve down and run out to the right"],
      ],
    ],
    [
      JAPANESE_SU,
      "03059",
      "U+3059 HIRAGANA LETTER SU",
      [
        ["draw the bar from left to right"],
        [
          "draw the vertical down through the bar",
          "loop round to the left and over the top",
          "come down and sweep to the lower left",
        ],
      ],
    ],
    [
      JAPANESE_CHI,
      "03061",
      "U+3061 HIRAGANA LETTER TI",
      [
        ["draw the bar to the right"],
        [
          "fall through the bar to the lower left",
          "turn sharply and rise to the right",
          "round the right side and finish low",
        ],
      ],
    ],
    [
      JAPANESE_TO,
      "03068",
      "U+3068 HIRAGANA LETTER TO",
      [
        ["draw the short stroke down to the right"],
        ["sweep down to the left", "curve round and run out to the right"],
      ],
    ],
  ] as const) {
    it(`Japanese ${letter.glyph} follows KanjiVG's ${labels.length}-path order on its own outline`, () => {
      expect(letter.strokes).toHaveLength(labels.length);
      expect(penLifts(letter)).toBe(labels.length - 1);
      expect(
        letter.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.label),
        ),
      ).toEqual(labels);
      for (const stroke of letter.strokes) {
        for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
      }
      expect(letter.source.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of [
        "KanjiVG",
        name,
        "Ulrich Apel and contributors, CC BY-SA 3.0",
      ]) {
        expect(letter.source.citation, phrase).toContain(phrase);
      }
      for (const phrase of [
        "Only the order and direction",
        "Noto Sans JP outline's own medial line",
      ]) {
        expect(letter.source.variation, phrase).toContain(phrase);
      }
    });
  }

  it("Japanese こ, さ, す, ち and と run in the directions their captions claim", () => {
    const first = (path: readonly Point[]) => path[0];
    const last = (path: readonly Point[]) => path[path.length - 1];
    const parts = (letter: LetterDuctus, stroke: number) =>
      letter.strokes[stroke].segments.map((segment) => segment.path);
    const nearest = (p: Point, path: readonly Point[]) =>
      Math.min(...path.map((q) => Math.hypot(p.x - q.x, p.y - q.y)));
    // Every top stroke and bar runs rightward.
    for (const letter of [
      JAPANESE_KO,
      JAPANESE_SA,
      JAPANESE_SU,
      JAPANESE_CHI,
    ]) {
      const bar = penPath(letter.strokes[0]);
      expect(first(bar).x, letter.glyph).toBeLessThan(last(bar).x - 250);
    }
    // こ and the foot of さ: begin at the upper left, curve DOWN, then run out
    // to the RIGHT along the base and rise slightly at the end.
    for (const lower of [
      penPath(JAPANESE_KO.strokes[1]),
      penPath(JAPANESE_SA.strokes[2]),
    ]) {
      const lowest = Math.min(...lower.map((p) => p.y));
      expect(first(lower).y).toBeGreaterThan(lowest + 200);
      expect(last(lower).x).toBeGreaterThan(first(lower).x + 400);
      expect(last(lower).y).toBeGreaterThan(lowest + 10);
    }
    expect(
      Math.max(...penPath(JAPANESE_KO.strokes[1]).map((p) => p.y)),
    ).toBeLessThan(
      Math.min(...penPath(JAPANESE_KO.strokes[0]).map((p) => p.y)) - 250,
    );
    // さ: the second stroke starts ABOVE the bar, slants down to the RIGHT
    // through it, and hooks back to the LEFT.
    const saBarTop = Math.max(...penPath(JAPANESE_SA.strokes[0]).map((p) => p.y));
    const [saSlant, saHook] = parts(JAPANESE_SA, 1);
    expect(first(saSlant).y).toBeGreaterThan(saBarTop + 50);
    expect(last(saSlant).x).toBeGreaterThan(first(saSlant).x + 150);
    expect(last(saSlant).y).toBeLessThan(first(saSlant).y - 400);
    expect(last(saHook).x).toBeLessThan(first(saHook).x - 100);
    // す: the vertical starts above the bar and runs down through it; the
    // loop swings LEFT and back over the top; the tail comes back down the
    // vertical's own ink and sweeps to the LOWER LEFT.
    const suBar = penPath(JAPANESE_SU.strokes[0]);
    const [suDown, suLoop, suTail] = parts(JAPANESE_SU, 1);
    expect(first(suDown).y).toBeGreaterThan(Math.max(...suBar.map((p) => p.y)) + 50);
    expect(last(suDown).y).toBeLessThan(Math.min(...suBar.map((p) => p.y)) - 250);
    expect(Math.min(...suLoop.map((p) => p.x))).toBeLessThan(first(suLoop).x - 200);
    expect(Math.max(...suLoop.map((p) => p.y))).toBeGreaterThan(first(suLoop).y + 120);
    for (const p of suTail.slice(0, 2)) {
      expect(nearest(p, suDown)).toBeLessThan(40);
    }
    expect(last(suTail).x).toBeLessThan(first(suTail).x - 200);
    expect(last(suTail).y).toBeLessThan(first(suTail).y - 300);
    // ち: the body starts above the bar and falls through it to the LOWER
    // LEFT, turns sharply back up its own ink to the RIGHT, rounds the belly
    // and finishes low, heading LEFT.
    const chiBarTop = Math.max(...penPath(JAPANESE_CHI.strokes[0]).map((p) => p.y));
    const [chiFall, chiRise, chiRound] = parts(JAPANESE_CHI, 1);
    expect(first(chiFall).y).toBeGreaterThan(chiBarTop + 50);
    expect(last(chiFall).x).toBeLessThan(first(chiFall).x - 100);
    expect(last(chiFall).y).toBeLessThan(first(chiFall).y - 400);
    expect(nearest(chiRise[1], chiFall)).toBeLessThan(40);
    expect(last(chiRise).x).toBeGreaterThan(first(chiRise).x + 300);
    expect(last(chiRise).y).toBeGreaterThan(first(chiRise).y + 100);
    expect(Math.max(...chiRound.map((p) => p.x))).toBeGreaterThan(first(chiRound).x + 150);
    expect(last(chiRound).y).toBeLessThan(first(chiRound).y - 300);
    expect(last(chiRound).x).toBeLessThan(chiRound[chiRound.length - 2].x);
    // と: the short stroke runs down to the RIGHT and ends ON the long one;
    // the long stroke starts right of it, sweeps down to the LOWER LEFT,
    // curves round and runs out to the RIGHT along the base.
    const toShort = penPath(JAPANESE_TO.strokes[0]);
    expect(last(toShort).x).toBeGreaterThan(first(toShort).x + 100);
    expect(last(toShort).y).toBeLessThan(first(toShort).y - 200);
    const [toSweep, toBase] = parts(JAPANESE_TO, 1);
    expect(first(toSweep).x).toBeGreaterThan(Math.max(...toShort.map((p) => p.x)) + 200);
    expect(last(toSweep).x).toBeLessThan(first(toSweep).x - 400);
    expect(last(toSweep).y).toBeLessThan(first(toSweep).y - 350);
    expect(nearest(last(toShort), toSweep)).toBeLessThan(40);
    expect(last(toBase).x).toBeGreaterThan(first(toBase).x + 400);
  });

  it("Japanese counter hiragana preserve the cited stroke and lift counts", () => {
    expect([
      [JAPANESE_NO.glyph, JAPANESE_NO.strokes.length, penLifts(JAPANESE_NO)],
      [JAPANESE_HI.glyph, JAPANESE_HI.strokes.length, penLifts(JAPANESE_HI)],
      [JAPANESE_FU.glyph, JAPANESE_FU.strokes.length, penLifts(JAPANESE_FU)],
      [JAPANESE_HO.glyph, JAPANESE_HO.strokes.length, penLifts(JAPANESE_HO)],
      [JAPANESE_MU.glyph, JAPANESE_MU.strokes.length, penLifts(JAPANESE_MU)],
      [JAPANESE_YA.glyph, JAPANESE_YA.strokes.length, penLifts(JAPANESE_YA)],
    ]).toEqual([
      ["の", 1, 0],
      ["ひ", 1, 0],
      ["ふ", 4, 3],
      ["ほ", 4, 3],
      ["む", 3, 2],
      ["や", 3, 2],
    ]);
    for (const letter of [
      JAPANESE_NO,
      JAPANESE_HI,
      JAPANESE_FU,
      JAPANESE_HO,
      JAPANESE_MU,
      JAPANESE_YA,
    ]) {
      expect(letter.source.url).toContain(
        "commons.wikimedia.org/wiki/File:Hiragana_",
      );
      expect(letter.source.citation).toMatch(/Sirgazil.*frames.*seconds/i);
    }
  });
});
