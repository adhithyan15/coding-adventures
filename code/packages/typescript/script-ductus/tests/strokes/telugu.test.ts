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

const TELUGU_A = DUCTUS[ductusKey("telugu", "అ")];
const TELUGU_KA = DUCTUS[ductusKey("telugu", "క")];
const TELUGU_KHA = DUCTUS[ductusKey("telugu", "ఖ")];
const TELUGU_GA = DUCTUS[ductusKey("telugu", "గ")];
const TELUGU_GHA = DUCTUS[ductusKey("telugu", "ఘ")];
const TELUGU_NGA = DUCTUS[ductusKey("telugu", "ఙ")];
const TELUGU_CA = DUCTUS[ductusKey("telugu", "చ")];
const TELUGU_CHHA = DUCTUS[ductusKey("telugu", "ఛ")];
const TELUGU_JA = DUCTUS[ductusKey("telugu", "జ")];
const TELUGU_JHA = DUCTUS[ductusKey("telugu", "ఝ")];
const TELUGU_NYA = DUCTUS[ductusKey("telugu", "ఞ")];
const TELUGU_TTA = DUCTUS[ductusKey("telugu", "ట")];
const TELUGU_TTHA = DUCTUS[ductusKey("telugu", "ఠ")];
const TELUGU_DDA = DUCTUS[ductusKey("telugu", "డ")];
const TELUGU_DDHA = DUCTUS[ductusKey("telugu", "ఢ")];
const TELUGU_NNA = DUCTUS[ductusKey("telugu", "ణ")];
const TELUGU_TA = DUCTUS[ductusKey("telugu", "త")];
const TELUGU_THA = DUCTUS[ductusKey("telugu", "థ")];
const TELUGU_DA = DUCTUS[ductusKey("telugu", "ద")];
const TELUGU_DHA = DUCTUS[ductusKey("telugu", "ధ")];
const TELUGU_NA = DUCTUS[ductusKey("telugu", "న")];
const TELUGU_PA = DUCTUS[ductusKey("telugu", "ప")];
const TELUGU_PHA = DUCTUS[ductusKey("telugu", "ఫ")];
const TELUGU_BA = DUCTUS[ductusKey("telugu", "బ")];
const TELUGU_BHA = DUCTUS[ductusKey("telugu", "భ")];
const TELUGU_MA = DUCTUS[ductusKey("telugu", "మ")];
const TELUGU_YA = DUCTUS[ductusKey("telugu", "య")];
const TELUGU_RA = DUCTUS[ductusKey("telugu", "ర")];
const TELUGU_LA = DUCTUS[ductusKey("telugu", "ల")];
const TELUGU_LLA = DUCTUS[ductusKey("telugu", "ళ")];
const TELUGU_VA = DUCTUS[ductusKey("telugu", "వ")];
const TELUGU_SHA = DUCTUS[ductusKey("telugu", "శ")];
const TELUGU_SSA = DUCTUS[ductusKey("telugu", "ష")];
const TELUGU_SA = DUCTUS[ductusKey("telugu", "స")];
const TELUGU_HA = DUCTUS[ductusKey("telugu", "హ")];
const TELUGU_AA = DUCTUS[ductusKey("telugu", "ఆ")];
const TELUGU_I = DUCTUS[ductusKey("telugu", "ఇ")];
const TELUGU_U = DUCTUS[ductusKey("telugu", "ఉ")];
const TELUGU_E = DUCTUS[ductusKey("telugu", "ఎ")];
const TELUGU_EE = DUCTUS[ductusKey("telugu", "ఏ")];
const TELUGU_O = DUCTUS[ductusKey("telugu", "ఒ")];
const TELUGU_AI = DUCTUS[ductusKey("telugu", "ఐ")];
const TELUGU_VOCALIC_R = DUCTUS[ductusKey("telugu", "ఋ")];

const OWNER_SCRIPTS = new Set(["telugu"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  // The sourced క, ఙ, ఛ and ఝ routes cross narrow printed counter
  // transitions while still covering their complete outlines. ఞ, ట, ఢ, డ, ఐ,
  // ఋ, త, అ and ఒ no longer need a bound: their native-lift refits run every
  // stroke along Noto's own ink, so they meet the default 0.97 on-ink floor.
  registerStrokeHonestyTests(
    letters,
    {
      క: 0.93,
      ఖ: 0.93,
      ఙ: 0.9,
      ఛ: 0.92,
      ఝ: 0.75,
    },
    {
      // ణ, బ, ళ, త and హ no longer need a bound: their native-lift refits
      // run through Noto's printed joins, so they meet the default 2% ceiling.
    },
  );

  beforeAll(() => {
    expect(verifiedLetterFont("క", TELUGU_KA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఖ", TELUGU_KHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("గ", TELUGU_GA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఘ", TELUGU_GHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఙ", TELUGU_NGA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("చ", TELUGU_CA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఛ", TELUGU_CHHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("జ", TELUGU_JA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఝ", TELUGU_JHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఞ", TELUGU_NYA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ట", TELUGU_TTA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఠ", TELUGU_TTHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("డ", TELUGU_DDA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఢ", TELUGU_DDHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ణ", TELUGU_NNA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("త", TELUGU_TA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("థ", TELUGU_THA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ద", TELUGU_DA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ధ", TELUGU_DHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("న", TELUGU_NA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ప", TELUGU_PA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఫ", TELUGU_PHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("బ", TELUGU_BA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("భ", TELUGU_BHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("మ", TELUGU_MA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("య", TELUGU_YA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ర", TELUGU_RA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ల", TELUGU_LA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ళ", TELUGU_LLA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("వ", TELUGU_VA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("శ", TELUGU_SHA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ష", TELUGU_SSA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("స", TELUGU_SA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("హ", TELUGU_HA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("అ", TELUGU_A.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఆ", TELUGU_AA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఇ", TELUGU_I.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఉ", TELUGU_U.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఎ", TELUGU_E.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఏ", TELUGU_EE.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఒ", TELUGU_O.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఐ", TELUGU_AI.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఋ", TELUGU_VOCALIC_R.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
  });

  it("Telugu క groups five source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_KA)).toBe(1);
    expect(TELUGU_KA.strokes).toHaveLength(2);
    expect(
      TELUGU_KA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left around the upper bowl",
        "continue right over the middle shoulder",
        "curve down and left around the lower bowl",
        "finish upward along the left tail",
      ],
      ["sweep up through the separate headstroke"],
    ]);
  });

  it("Telugu ఖ groups six source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_KHA)).toBe(1);
    expect(TELUGU_KHA.strokes).toHaveLength(2);
    expect(
      TELUGU_KHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "circle up around the upper-left bowl",
        "descend through the central curve",
        "turn up around the left shoulder",
        "sweep right and up the broad outer bowl",
        "return left along the crown",
      ],
      ["draw the separate downward stem"],
    ]);
  });

  it("Telugu గ keeps its two source-verified movements in separate pen-down runs", () => {
    expect(penLifts(TELUGU_GA)).toBe(1);
    expect(TELUGU_GA.strokes).toHaveLength(2);
    expect(
      TELUGU_GA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["sweep up and over the broad lower arch"],
      ["cup through the separate upper flourish"],
    ]);
  });

  it("Telugu ఘ groups six source-verified movements into four pen-down runs", () => {
    expect(penLifts(TELUGU_GHA)).toBe(3);
    expect(TELUGU_GHA.strokes).toHaveLength(4);
    expect(
      TELUGU_GHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left around the upper-left shoulder",
        "continue down round the lower-left bowl",
        "turn upward around the broad middle arch",
      ],
      ["sweep right and up around the outer arch"],
      ["restart and cup the upper flourish"],
      ["restart and draw the separate downward stem"],
    ]);
  });

  it("Telugu చ groups four source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 93% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_CA)).toBe(0);
    expect(TELUGU_CA.strokes).toHaveLength(1);
    expect(
      TELUGU_CA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "draw the upper bar from left to right",
        "continue down and around the left bowl",
        "sweep right and up around the outer bowl",
        "climb up the flourish's left arm",
        "cup the upper flourish",
      ],
    ]);
  });

  it("Telugu ఛ groups five source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_CHHA)).toBe(1);
    expect(TELUGU_CHHA.strokes).toHaveLength(2);
    expect(
      TELUGU_CHHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "draw the upper bar from left to right",
        "continue down and around the left bowl",
        "sweep right and up around the outer bowl",
        "continue through the upper flourish",
      ],
      ["restart and draw the separate downward stem"],
    ]);
  });

  it("Telugu ఙ groups five source-verified movements into three pen-down runs", () => {
    expect(penLifts(TELUGU_NGA)).toBe(2);
    expect(TELUGU_NGA.strokes).toHaveLength(3);
    expect(
      TELUGU_NGA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn around the compact upper-left lobe",
        "continue down round the broad lower bowl",
        "curl upward around the rounded right lobe",
      ],
      ["lift and draw the inner bar left to right"],
      ["lift again, draw the upper headstroke down"],
    ]);
  });

  it("Telugu జ groups four source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 97% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_JA)).toBe(1);
    expect(TELUGU_JA.strokes).toHaveLength(2);
    expect(
      TELUGU_JA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep right across the upper-left arch",
        "curve down and right round the left bowl",
        "sweep right and up the lower-right bowl",
      ],
      [
        "curl the upper-right flourish",
      ],
    ]);
  });

  it("Telugu ఝ preserves all five source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_JHA)).toBe(4);
    expect(TELUGU_JHA.strokes).toHaveLength(5);
    expect(
      TELUGU_JHA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "circle around the broad left bowl",
      "restart and circle around the middle bowl",
      "restart and circle around the right bowl",
      "restart and sweep the upper flourish",
      "restart and draw the separate downward stem",
    ]);
  });

  it("Telugu ఞ groups eight source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 89% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_NYA)).toBe(2);
    expect(TELUGU_NYA.strokes).toHaveLength(3);
    expect(
      TELUGU_NYA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep up around the upper-left loop",
        "sweep right around the upper-right loop",
        "curve down and left around the lower bowl",
        "curl upward around the inner-left loop",
        "curl down and right around the inner bowl",
        "draw the short downward tail",
      ],
      ["draw the right horizontal bar"],
      ["draw the separate upper stem downward"],
    ]);
  });

  it("Telugu ట groups six source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 94% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_TTA)).toBe(1);
    expect(TELUGU_TTA.strokes).toHaveLength(2);
    expect(
      TELUGU_TTA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "curl upward along the inner shoulder",
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve up and left over the outer shoulder",
      ],
      ["draw the separate upper stem downward"],
    ]);
  });

  it("Telugu ఠ groups three source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 71% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_TTHA)).toBe(1);
    expect(TELUGU_TTHA.strokes).toHaveLength(2);
    expect(
      TELUGU_TTHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left round the broad circular body",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
      [
        "place the separate inner dot",
      ],
    ]);
  });

  it("Telugu డ groups five source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 75% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_DDA)).toBe(1);
    expect(TELUGU_DDA.strokes).toHaveLength(2);
    expect(
      TELUGU_DDA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
      ],
      [
        "curl up through the separate top flourish",
      ],
    ]);
  });

  it("Telugu ఢ groups six source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 79% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_DDHA)).toBe(2);
    expect(TELUGU_DDHA.strokes).toHaveLength(3);
    expect(
      TELUGU_DDHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
      ],
      ["curl up through the separate top flourish"],
      ["draw the separate lower stem downward"],
    ]);
  });

  it("Telugu ణ groups five source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 79% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_NNA)).toBe(0);
    expect(TELUGU_NNA.strokes).toHaveLength(1);
    expect(
      TELUGU_NNA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left and up the lower-left bowl",
        "curve right across the upper-left bowl",
        "arch right and down the upper-right bowl",
        "turn left around the lower-right bowl",
        "sweep upward along the inner curve",
      ],
    ]);
  });

  it("Telugu త groups seven source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 51% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_TA)).toBe(0);
    expect(TELUGU_TA.strokes).toHaveLength(1);
    expect(
      TELUGU_TA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "curl upward along the inner-left shoulder",
        "turn downward around the outer-left bowl",
        "sweep right around the broad lower bowl",
        "curve upward around the outer-right bowl",
        "turn down the inner-right shoulder",
        "curve up and left over the upper shoulder",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
    ]);
  });

  it("Telugu థ groups seven source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 78% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_THA)).toBe(2);
    expect(TELUGU_THA.strokes).toHaveLength(3);
    expect(
      TELUGU_THA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
      ["draw the separate lower stem downward"],
      ["place the separate inner dot"],
    ]);
  });

  it("Telugu ద groups five source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 81% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_DA)).toBe(0);
    expect(TELUGU_DA.strokes).toHaveLength(1);
    expect(
      TELUGU_DA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
    ]);
  });

  it("Telugu ధ groups six source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 84% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_DHA)).toBe(1);
    expect(TELUGU_DHA.strokes).toHaveLength(2);
    expect(
      TELUGU_DHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down around the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
      ["draw the separate lower stem downward"],
    ]);
  });

  it("Telugu న groups three source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 92% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_NA)).toBe(0);
    expect(TELUGU_NA.strokes).toHaveLength(1);
    expect(
      TELUGU_NA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep up the left bowl into the middle",
        "sweep right and up the broad lower bowl",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
    ]);
  });

  it("Telugu ప groups four source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 76% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_PA)).toBe(1);
    expect(TELUGU_PA.strokes).toHaveLength(2);
    expect(
      TELUGU_PA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left across the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep upward around the broad right bowl",
      ],
      [
        "curl up through the separate top flourish",
      ],
    ]);
  });

  it("Telugu ఫ groups five source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 78% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_PHA)).toBe(2);
    expect(TELUGU_PHA.strokes).toHaveLength(3);
    expect(
      TELUGU_PHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left across the upper-left curve",
        "turn right around the lower-left bowl",
        "sweep upward around the broad right bowl",
      ],
      [
        "curl up through the separate top flourish",
      ],
      [
        "draw the lower stem downward",
      ],
    ]);
  });

  it("Telugu బ groups four source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 97% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_BA)).toBe(0);
    expect(TELUGU_BA.strokes).toHaveLength(1);
    expect(
      TELUGU_BA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep right around the upper-left curve",
        "curve down and left through the shoulder",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "curve left round the upper-right shoulder",
      ],
    ]);
  });

  it("Telugu భ groups six source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 85% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_BHA)).toBe(1);
    expect(TELUGU_BHA.strokes).toHaveLength(2);
    expect(
      TELUGU_BHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep right around the upper-left curve",
        "curve down and left through the shoulder",
        "turn right around the lower-left bowl",
        "sweep right and up the lower-right bowl",
        "climb up the flourish's left arm",
        "curl upward through the top flourish",
      ],
      ["draw the lower stem downward"],
    ]);
  });

  it("Telugu మ groups seven source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 78% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_MA)).toBe(1);
    expect(TELUGU_MA.strokes).toHaveLength(2);
    expect(
      TELUGU_MA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left around the upper-left arch",
        "turn right around the lower-left bowl",
        "turn right around the lower-middle bowl",
        "arc up and left over the central shoulder",
        "climb up the flourish's left arm",
        "sweep right and up into the top flourish",
      ],
      [
        "sweep right and up the lower-right bowl",
        "curve up and left over the right shoulder",
      ],
    ]);
  });

  it("Telugu య groups four source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 48% of native writers, the
    // most common count;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_YA)).toBe(2);
    expect(TELUGU_YA.strokes).toHaveLength(3);
    expect(
      TELUGU_YA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop the left bowl counterclockwise",
        "loop the centre bowl counterclockwise",
      ],
      [
        "draw down and up the lower angled join",
      ],
      [
        "loop the right bowl counterclockwise",
      ],
    ]);
  });

  it("Telugu ర groups two source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 81% of native writers;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_RA)).toBe(0);
    expect(TELUGU_RA.strokes).toHaveLength(1);
    expect(
      TELUGU_RA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop the main bowl counterclockwise",
        "climb up the chevron's left arm",
        "draw the upper chevron down and up",
      ],
    ]);
  });

  it("Telugu ల groups two source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 99% of native writers;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_LA)).toBe(0);
    expect(TELUGU_LA.strokes).toHaveLength(1);
    expect(
      TELUGU_LA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop the small upper bowl counterclockwise",
        "sweep down round the broad lower bowl",
      ],
    ]);
  });

  it("Telugu ళ groups four source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 87% of native writers; the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_LLA)).toBe(0);
    expect(TELUGU_LLA.strokes).toHaveLength(1);
    expect(
      TELUGU_LLA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop the small inner bowl counterclockwise",
        "sweep down the body, loop the lower bowl",
        "sweep right and up the broad outer body",
        "climb up the chevron's left arm",
        "draw the upper chevron down and up",
      ],
    ]);
  });

  it("Telugu వ groups three source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 75% of native writers;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_VA)).toBe(0);
    expect(TELUGU_VA.strokes).toHaveLength(1);
    expect(
      TELUGU_VA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop the lower-left bowl counterclockwise",
        "sweep round the lower and right body",
        "climb up the chevron's left arm",
        "draw the upper chevron down and up",
      ],
    ]);
  });

  it("Telugu శ groups three source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 93% of native writers;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_SHA)).toBe(0);
    expect(TELUGU_SHA.strokes).toHaveLength(1);
    expect(
      TELUGU_SHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop around the broad lower-left bowl",
        "sweep round the tall lower and right body",
        "climb up the chevron's left arm",
        "draw the upper chevron down and up",
      ],
    ]);
  });

  it("Telugu ష groups four source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 27% of native writers (two
    // for 69%, but the tail cannot join the body in source order);
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_SSA)).toBe(2);
    expect(TELUGU_SSA.strokes).toHaveLength(3);
    expect(
      TELUGU_SSA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop around the lower-left bowl",
        "sweep round the lower and right body",
      ],
      [
        "add the short lower-right tail",
      ],
      [
        "draw the separate chevron down and up",
      ],
    ]);
  });

  it("Telugu స preserves both source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_SA)).toBe(1);
    expect(TELUGU_SA.strokes).toHaveLength(2);
    expect(
      TELUGU_SA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "loop the left bowl, sweep the right body",
      "draw the separate chevron down and up",
    ]);
  });

  it("Telugu హ groups four source-verified movements into two pen-down runs", () => {
    // HP Labs India: two strokes for 74% of native writers;
    // the source's ordered movements stay as segments.
    expect(penLifts(TELUGU_HA)).toBe(1);
    expect(TELUGU_HA.strokes).toHaveLength(2);
    expect(
      TELUGU_HA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop around the lower-left bowl",
        "sweep round the lower and right body",
        "draw the middle bar right and curl its end",
      ],
      [
        "draw the separate chevron down and up",
      ],
    ]);
  });

  it("Telugu అ groups four source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 97% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_A)).toBe(0);
    expect(TELUGU_A.strokes).toHaveLength(1);
    expect(
      TELUGU_A.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn around the left lobe",
        "sweep around the broad lower bowl",
        "turn around the right lobe",
        "return left along the inner bar",
      ],
    ]);
  });

  it("Telugu ఆ keeps its two source-verified components in separate pen-down runs", () => {
    expect(penLifts(TELUGU_AA)).toBe(1);
    expect(TELUGU_AA.strokes).toHaveLength(2);
    expect(
      TELUGU_AA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn round the left lobe and lower bowl",
      ],
      [
        "turn the right lobe, then left along the bar",
      ],
    ]);
  });

  it("Telugu ఇ keeps its three source-verified components in separate pen-down runs", () => {
    expect(penLifts(TELUGU_I)).toBe(2);
    expect(TELUGU_I.strokes).toHaveLength(3);
    expect(
      TELUGU_I.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["turn around the broad outer bowl"],
      ["form the compact upper-left lobe"],
      ["form the angled upper-right shoulder"],
    ]);
  });

  it("Telugu ఉ groups five source-verified movements into three pen-down runs", () => {
    expect(penLifts(TELUGU_U)).toBe(2);
    expect(TELUGU_U.strokes).toHaveLength(3);
    expect(
      TELUGU_U.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left across the rounded upper arch",
        "continue down round the broad lower bowl",
        "curl up round the right lobe, no lift",
      ],
      ["lift and draw the inner bar left to right"],
      ["lift again, draw the upper headstroke down"],
    ]);
  });

  it("Telugu ఎ groups three source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 86% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_E)).toBe(0);
    expect(TELUGU_E.strokes).toHaveLength(1);
    expect(
      TELUGU_E.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left round the lower loop",
        "round its base and back to the junction",
        "sweep up the broad outer arch",
      ],
    ]);
  });

  it("Telugu ఏ groups four source-verified movements into three pen-down runs", () => {
    expect(penLifts(TELUGU_EE)).toBe(2);
    expect(TELUGU_EE.strokes).toHaveLength(3);
    expect(
      TELUGU_EE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left round the lower loop",
        "round its base and back to the junction",
      ],
      [
        "restart at the tail, sweep up the outer arch",
      ],
      ["restart and sweep up the upper-left hook"],
    ]);
  });

  it("Telugu ఋ groups six source-verified movements into three pen-down runs", () => {
    // HP Labs India: three strokes for 48% of native writers, the
    // most common count; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_VOCALIC_R)).toBe(2);
    expect(TELUGU_VOCALIC_R.strokes).toHaveLength(3);
    expect(
      TELUGU_VOCALIC_R.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep right across the upper shoulder",
        "curve down around the left bowl",
        "sweep right around the lower bowl",
        "curl up around the first right lobe",
      ],
      [
        "curl up around the middle lobe",
      ],
      [
        "curl up around the final lobe",
      ],
    ]);
  });

  it("Telugu ఐ groups five source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 70% of native writers; the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_AI)).toBe(0);
    expect(TELUGU_AI.strokes).toHaveLength(1);
    expect(
      TELUGU_AI.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left across the compact upper arch",
        "curve down around the left bowl",
        "sweep right around the broad lower bowl",
        "sweep left across the upper-right arch",
        "sweep left across the upper-left arch",
      ],
    ]);
  });

  it("Telugu ఒ groups three source-verified movements into one pen-down run", () => {
    // HP Labs India: one stroke for 99% of native writers;
    // the source's numbered movements stay as segments.
    expect(penLifts(TELUGU_O)).toBe(0);
    expect(TELUGU_O.strokes).toHaveLength(1);
    expect(
      TELUGU_O.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep right across the upper arch",
        "curve down around the left bowl",
        "sweep right around the broad lower bowl",
      ],
    ]);
  });
});
