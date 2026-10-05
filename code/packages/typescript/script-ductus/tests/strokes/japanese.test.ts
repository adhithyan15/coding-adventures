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
