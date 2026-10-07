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
const MALAYALAM_UU = DUCTUS[ductusKey("malayalam", "ഊ")];
const MALAYALAM_E = DUCTUS[ductusKey("malayalam", "എ")];
const MALAYALAM_O = DUCTUS[ductusKey("malayalam", "ഒ")];
const MALAYALAM_OO = DUCTUS[ductusKey("malayalam", "ഓ")];
const MALAYALAM_CHILLU_L = DUCTUS[ductusKey("malayalam", "ൽ")];
const MALAYALAM_CHILLU_N = DUCTUS[ductusKey("malayalam", "ൻ")];
const MALAYALAM_CHILLU_NN = DUCTUS[ductusKey("malayalam", "ൺ")];
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
    expect(verifiedLetterFont("ഊ", MALAYALAM_UU.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ഒ", MALAYALAM_O.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ഓ", MALAYALAM_OO.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൽ", MALAYALAM_CHILLU_L.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൻ", MALAYALAM_CHILLU_N.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
    expect(verifiedLetterFont("ൺ", MALAYALAM_CHILLU_NN.source.url)).toBe(
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

  it("Malayalam ഊ lifts once before its joined right-side flourish", () => {
    expect(penLifts(MALAYALAM_UU)).toBe(1);
    expect(
      MALAYALAM_UU.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn outward around the compact left spiral and carry the upper arch right",
        "descend around the broad right lobe and curl left below the line",
        "carry the finishing baseline to the right",
      ],
      [
        "sweep over the compact upper arch and cross into the right lobe",
        "circle the broad right lobe and descend its finishing tail",
      ],
    ]);
  });

  it("Malayalam ഒ lifts once before its rounded right-side lobe", () => {
    expect(penLifts(MALAYALAM_O)).toBe(1);
    expect(
      MALAYALAM_O.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "curl clockwise from the compact inner tip and sweep through the broad left arch",
      ],
      [
        "sweep right from the upper junction and descend around the rounded lower lobe",
      ],
    ]);
  });

  it("Malayalam ഓ lifts twice before the body lobe and outer arc", () => {
    expect(penLifts(MALAYALAM_OO)).toBe(2);
    expect(
      MALAYALAM_OO.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "curl clockwise from the compact inner tip and sweep through the broad left arch",
      ],
      [
        "sweep right from the upper junction and descend around the rounded lower lobe",
      ],
      ["descend around the separate outer arc at the far right"],
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

  it("Malayalam chillu ൺ keeps all five animated movements in one run", () => {
    expect(penLifts(MALAYALAM_CHILLU_NN)).toBe(0);
    expect(MALAYALAM_CHILLU_NN.strokes).toHaveLength(1);
    expect(
      MALAYALAM_CHILLU_NN.strokes[0].segments.map(
        (segment) => segment.label,
      ),
    ).toEqual([
      "start at the inner-left tip and sweep clockwise around the compact inner loop",
      "without lifting, continue clockwise around the broad outer-left bowl and climb to the upper junction",
      "without lifting, descend the first central stem, retrace upward, carry the middle arch, and descend the second stem",
      "without lifting, carry the upper shoulder right and sweep clockwise around the right loop",
      "without lifting, rise into the chillu hook and curl left above the line",
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

// Base consonants cited to SPACE's Thooval formation arrows (facts only; the
// tool is GPL-3.0 and nothing of it is copied). Each is one continuous run:
// Thooval keeps the pen down by design, and two recorded sources that could
// show a lift (hand, grahyam) show none. The labels below are the turning
// points Thooval marks, in order.
const THOOVAL_CONSONANTS: ReadonlyArray<
  readonly [glyph: string, slug: string, labels: readonly string[]]
> = [
  [
    "ന",
    "NA",
    [
      "climb clockwise over the left arch",
      "descend the stem to the line",
      "retrace the stem upward",
      "sweep over the right arch to its foot",
    ],
  ],
  [
    "മ",
    "MA",
    [
      "climb the left side and arch over",
      "curve down the inner stroke",
      "run right along the base",
      "climb the right side to the top",
    ],
  ],
  [
    "സ",
    "SA",
    [
      "climb clockwise over the left arch",
      "descend the stem to the line",
      "retrace the stem upward",
      "sweep over the middle arch and down",
      "round the bowl and up to the top",
    ],
  ],
  [
    "ര",
    "RA",
    [
      "climb clockwise over the top",
      "descend the right side",
      "curl up around the inner loop",
    ],
  ],
  [
    "ത",
    "TA",
    [
      "climb clockwise over the left arch",
      "cross down the middle loop",
      "round its base and climb back up",
      "sweep over the right arch to its foot",
    ],
  ],
  [
    "ഷ",
    "SSA",
    [
      "climb over the small arch",
      "run right along the base",
      "climb the right upright",
      "slant down to the left",
      "circle the loop",
      "descend the stem to the line",
    ],
  ],
  [
    "പ",
    "PA",
    [
      "climb over the small arch",
      "run right along the base",
      "climb the upright to the top",
    ],
  ],
  [
    "വ",
    "VA",
    [
      "climb clockwise over the arch",
      "run right along the base",
      "climb the upright to the top",
    ],
  ],
  [
    "ണ",
    "NNA",
    [
      "circle the small inner loop",
      "climb over the big left arch",
      "descend the first stem",
      "climb back up over the middle arch",
      "descend the second stem",
      "climb back up and round the right arch",
    ],
  ],
  [
    "ട",
    "TTA",
    [
      "curve left over the top and down",
      "swing right through the middle",
      "round the lower bowl to the left",
    ],
  ],
  [
    "ദ",
    "DA",
    [
      "climb clockwise over the top",
      "curve in to the middle tip",
      "retrace and round the lower bowl",
    ],
  ],
  [
    "ഹ",
    "HA",
    [
      "climb over the small arch",
      "run right along the base",
      "climb the left side of the loop",
      "sweep over and down to the foot",
    ],
  ],
  [
    "ഗ",
    "GA",
    [
      "curve down around the left bowl",
      "climb the middle stem",
      "sweep over the right arch to its foot",
    ],
  ],
  [
    "റ",
    "RRA",
    [
      "climb the left side",
      "arch over and down to the foot",
    ],
  ],
  [
    "ല",
    "LA",
    [
      "draw the middle bar to the right",
      "climb and arch back over the top",
      "descend the left side",
      "run right along the base",
      "climb the upright to the top",
    ],
  ],
  [
    "ശ",
    "SHA",
    [
      "curve down around the left bowl",
      "climb the middle stem",
      "sweep over and down the right side",
      "round the base and curl up inside",
    ],
  ],
  [
    "ബ",
    "BA",
    [
      "circle the small inner loop",
      "climb over the big left arch",
      "descend the stem to the line",
      "climb back up over the right arch",
      "curve down and run along the base",
      "climb the upright to the top",
    ],
  ],
];

describe("Malayalam consonants cited to Thooval", () => {
  for (const [glyph, slug, labels] of THOOVAL_CONSONANTS) {
    const letter = DUCTUS[ductusKey("malayalam", glyph)];

    it(`${glyph} is one continuous run through Thooval's turning points`, () => {
      expect(letter).toBeDefined();
      expect(penLifts(letter)).toBe(0);
      expect(letter.strokes).toHaveLength(1);
      expect(
        letter.strokes[0].segments.map((segment) => segment.label),
      ).toEqual(labels);
      for (const gap of joinGaps(letter.strokes[0])) expect(gap).toBe(0);
    });

    it(`${glyph} cites the pinned Thooval formation image and the Malayalam font`, () => {
      expect(letter.source.url).toBe(
        `https://github.com/spacekerala/Thooval/blob/87143b560bf5aab43837d9da2cddab9bd59cd391/data/${slug}.png`,
      );
      expect(letter.source.citation).toMatch(/SPACE Kerala.*Thooval.*GPL-3\.0/);
      expect(letter.source.variation).toMatch(
        /only these facts are cited.*santhoshtr\/hand.*grahyam.*counts only.*Noto Sans Malayalam/,
      );
      expect(verifiedLetterFont(glyph, letter.source.url)).toBe(
        "_fonts/NotoSansMalayalam-Static.ttf",
      );
    });
  }
});

// Glyphs cited to Rodney F. Moag's Malayalam: A University Course and Reference
// Grammar (facts only; CC BY-NC-SA 4.0, nothing copied). Moag numbers MOVEMENTS,
// not pen lifts, so each glyph is one continuous run whose segments are Moag's
// numbered movements, in order; the scan page is pinned at the digital
// edition's commit.
const MOAG_GLYPHS: ReadonlyArray<
  readonly [glyph: string, scan: string, labels: readonly string[]]
> = [
  [
    "ക",
    "035",
    [
      "climb clockwise over the top loop",
      "round the bottom to the left",
      "climb the middle stem",
      "retrace down into the left bowl",
      "round the left bowl to the bar",
      "draw the crossbar to the right",
      "curl the right hook down",
    ],
  ],
  [
    "യ",
    "040",
    [
      "curve down round the left bowl",
      "climb to the top of the middle loop",
      "come back down its inner side",
      "round the right bowl to the top",
    ],
  ],
  [
    "ഏ",
    "028",
    [
      "climb clockwise over the small arch",
      "run right along the bar",
      "climb the stem",
      "retrace it down and round the loop",
      "climb clockwise over the big arch",
      "come down to the middle",
      "round the lower bowl",
    ],
  ],
  [
    "ഖ",
    "035",
    [
      "circle the small inner loop",
      "arch over and down to the base",
      "run right along the base",
      "climb the upright to the top",
    ],
  ],
  [
    "ങ",
    "035",
    [
      "circle the small inner loop",
      "arch over to the stem",
      "descend the stem to the line",
      "climb back and round the top bowl",
      "round the lower bowl",
    ],
  ],
  [
    "ച",
    "036",
    [
      "curl clockwise over the top",
      "run left along the base",
      "run back right along the base",
      "climb the upright to the top",
    ],
  ],
  [
    "ഛ",
    "036",
    [
      "curl clockwise over the top",
      "run left along the base",
      "run back right along the base",
      "climb clockwise over the big arch",
      "curl up round the inner loop",
    ],
  ],
  [
    "ഞ",
    "036",
    [
      "circle the small inner loop",
      "arch over to the stem",
      "descend the stem to the line",
      "climb the stem again",
      "loop over and round the oval",
      "arch over the right side",
      "come down to the right foot",
    ],
  ],
  [
    "ഥ",
    "038",
    [
      "descend the left stem",
      "run right along the base",
      "climb, arch over and come down",
    ],
  ],
  [
    "ധ",
    "038",
    [
      "curve down round the left bowl",
      "climb the middle stem",
      "retrace the stem down",
      "round the right bowl to the top",
    ],
  ],
  [
    "ഭ",
    "039",
    [
      "climb clockwise over the top",
      "come down into the middle",
      "curl left, then right",
      "round the lower bowl to the left",
    ],
  ],
  [
    "ഫ",
    "039",
    [
      "climb clockwise over the small arch",
      "run right along the base",
      "climb, arch over and come down",
    ],
  ],
  [
    "ജ",
    "036",
    [
      "circle the small loop",
      "arch over and down the stem",
      "climb the stem and arch over the right",
      "run back left and round the lower left",
      "run right to the top of the last loop",
      "circle down and round the last loop",
    ],
  ],
  [
    "ള",
    "041",
    [
      "circle the small inner loop",
      "arch over and down to the middle",
      "round the right side to the left",
      "loop down and run right below",
    ],
  ],
  [
    "ം",
    "034",
    [
      "circle clockwise",
    ],
  ],
  [
    "ാ",
    "030",
    [
      "curve clockwise round to the foot",
    ],
  ],
  [
    "ി",
    "030",
    [
      "arch over and draw the stem down",
    ],
  ],
  [
    "ീ",
    "030",
    [
      "circle the small loop",
      "arch over and draw the stem down",
    ],
  ],
  [
    "ു",
    "031",
    [
      "draw the stem down from the hook",
      "round the left side and bottom",
      "climb its right side",
    ],
  ],
  [
    "ൂ",
    "031",
    [
      "draw the stem down from the hook",
      "round the left side and bottom",
      "climb its right side",
      "circle the small inner loop",
    ],
  ],
  [
    "ൃ",
    "031",
    [
      "draw the stem down",
      "circle the loop clockwise",
    ],
  ],
  [
    "െ",
    "032",
    [
      "circle the small inner loop",
      "arch over and down to the foot",
    ],
  ],
  [
    "േ",
    "032",
    [
      "circle the small top loop",
      "sweep left and round the bottom",
      "curl up round the lower loop",
    ],
  ],
];

describe("Malayalam glyphs cited to Moag's numbered movements", () => {
  for (const [glyph, scan, labels] of MOAG_GLYPHS) {
    const letter = DUCTUS[ductusKey("malayalam", glyph)];

    it(`${glyph} is one continuous run through Moag's numbered movements`, () => {
      expect(letter).toBeDefined();
      expect(penLifts(letter)).toBe(0);
      expect(letter.strokes).toHaveLength(1);
      expect(
        letter.strokes[0].segments.map((segment) => segment.label),
      ).toEqual(labels);
      for (const gap of joinGaps(letter.strokes[0])) expect(gap).toBe(0);
    });

    it(`${glyph} cites the pinned Moag scan and the Malayalam font`, () => {
      expect(letter.source.url).toBe(
        `https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-${scan}.jpg`,
      );
      expect(letter.source.citation).toMatch(
        /^Rodney F\. Moag, Malayalam: A University Course and Reference Grammar .*CC BY-NC-SA 4\.0.*written by hand by Thomas Joseph/,
      );
      expect(letter.source.citation).toContain(` for ${glyph},`);
      expect(letter.source.variation).toMatch(
        /only these facts .* are cited; no drawing is copied.*not pen lifts.*Noto Sans Malayalam/,
      );
      expect(verifiedLetterFont(glyph, letter.source.url)).toBe(
        "_fonts/NotoSansMalayalam-Static.ttf",
      );
    });
  }

  it("states that ജ's stem descent is read between Moag's arrows, at medium confidence", () => {
    const ja = DUCTUS[ductusKey("malayalam", "ജ")];
    expect(ja.source.variation).toMatch(
      /arrows do not draw the short stem's descent.*arrow 2 ends above the stem and arrow 3 begins at its foot.*confidence in it is medium.*hand.*one stroke.*45 unique samples/,
    );
  });

  it("draws ൈ as two cited െ coils, 715 units apart, with the one lift between them", () => {
    // Noto Sans Malayalam composes the standalone ൈ from two copies of െ; the
    // gap between them is paper, so the pen must lift once, and each coil is
    // the run Moag numbers for െ (movements 1-2, then 3-4).
    const ai = DUCTUS[ductusKey("malayalam", "ൈ")];
    const e = DUCTUS[ductusKey("malayalam", "െ")];
    expect(penLifts(ai)).toBe(1);
    expect(
      ai.strokes.map((stroke) => stroke.segments.map((segment) => segment.label)),
    ).toEqual([
      ["circle the first small loop", "arch over and down to the foot"],
      ["lift, then circle the second small loop", "arch over and down to its foot"],
    ]);
    expect(penPath(ai.strokes[0])).toEqual(penPath(e.strokes[0]));
    expect(penPath(ai.strokes[1])).toEqual(
      penPath(e.strokes[0]).map((point) => ({ x: point.x + 715, y: point.y })),
    );
    for (const stroke of ai.strokes) {
      for (const gap of joinGaps(stroke)) expect(gap).toBe(0);
    }
    expect(ai.source.url).toBe(
      "https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-032.jpg",
    );
    expect(ai.source.citation).toMatch(
      /^Rodney F\. Moag, .*Table III 'How to Write Internal Vowel Symbols', p\. xxii: movements 1-4 for ൈ, written by hand by Thomas Joseph$/,
    );
    expect(ai.source.variation).toMatch(
      /No recording of ൈ itself.*reasoned rather than observed and confidence is medium.*claims no written order/,
    );
    expect(verifiedLetterFont("ൈ", ai.source.url)).toBe(
      "_fonts/NotoSansMalayalam-Static.ttf",
    );
  });

  it("states the anusvara's direction as medium-low confidence, with the conflicting sources", () => {
    const anusvara = DUCTUS[ductusKey("malayalam", "ം")];
    expect(anusvara.source.variation).toMatch(
      /clockwise.*medium-low confidence.*anticlockwise/,
    );
  });
});
