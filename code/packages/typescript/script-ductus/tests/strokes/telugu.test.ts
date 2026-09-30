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
const TELUGU_JA = DUCTUS[ductusKey("telugu", "జ")];
const TELUGU_NYA = DUCTUS[ductusKey("telugu", "ఞ")];
const TELUGU_TTA = DUCTUS[ductusKey("telugu", "ట")];
const TELUGU_TTHA = DUCTUS[ductusKey("telugu", "ఠ")];
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
  // The sourced క, ఙ, ఞ and ట routes and five-run pedagogical ఐ route cross
  // narrow printed counter transitions while still covering their complete
  // outlines. ట's six separately numbered source movements cut across the
  // broad printed bowl most strongly.
  registerStrokeHonestyTests(letters, {
    అ: 0.96,
    క: 0.93,
    ఖ: 0.93,
    ఙ: 0.9,
    ఞ: 0.86,
    ట: 0.55,
    ఐ: 0.59,
    ఒ: 0.84,
    ఋ: 0.84,
  });

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
    expect(verifiedLetterFont("జ", TELUGU_JA.source.url)).toBe(
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
        "continue right through the middle shoulder",
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
        "sweep right and up around the broad outer bowl",
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
        "continue down and right around the lower-left bowl",
        "turn upward around the broad middle arch",
      ],
      ["sweep right and up around the outer arch"],
      ["restart and cup through the upper flourish"],
      ["restart and draw the separate downward stem"],
    ]);
  });

  it("Telugu చ groups four source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_CA)).toBe(1);
    expect(TELUGU_CA.strokes).toHaveLength(2);
    expect(
      TELUGU_CA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "draw the upper bar from left to right",
        "continue down and around the left bowl",
        "sweep right and up around the outer bowl",
      ],
      ["restart and cup through the upper flourish"],
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
        "continue down and around the broad lower bowl",
        "curl upward around the rounded right lobe",
      ],
      ["lift and draw the inner horizontal bar from left to right"],
      ["lift again and draw the short upper headstroke downward"],
    ]);
  });

  it("Telugu జ preserves all four source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_JA)).toBe(3);
    expect(TELUGU_JA.strokes).toHaveLength(4);
    expect(
      TELUGU_JA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "sweep right across the rounded upper-left arch",
      "curve down and right around the lower-left bowl",
      "sweep right and up around the lower-right bowl",
      "restart and curl through the upper-right flourish",
    ]);
  });

  it("Telugu ఞ preserves all eight source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_NYA)).toBe(7);
    expect(TELUGU_NYA.strokes).toHaveLength(8);
    expect(
      TELUGU_NYA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "sweep up around the upper-left loop",
      "sweep right around the upper-right loop",
      "curve down and left around the broad lower bowl",
      "curl upward around the inner-left loop",
      "curl down and right around the inner bowl",
      "draw the short downward tail",
      "draw the right horizontal bar",
      "draw the separate upper vertical stem downward",
    ]);
  });

  it("Telugu ట preserves all six source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_TTA)).toBe(5);
    expect(TELUGU_TTA.strokes).toHaveLength(6);
    expect(
      TELUGU_TTA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "curl upward along the inner shoulder",
      "sweep down around the upper-left curve",
      "turn right around the lower-left bowl",
      "sweep right and upward around the lower-right bowl",
      "curve upward and left around the outer shoulder",
      "draw the separate upper stem downward",
    ]);
  });

  it("Telugu ఠ preserves all three source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_TTHA)).toBe(2);
    expect(TELUGU_TTHA.strokes).toHaveLength(3);
    expect(
      TELUGU_TTHA.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "sweep left and around the broad circular body",
      "curl upward through the separate top flourish",
      "place the separate inner dot",
    ]);
  });

  it("Telugu అ groups four source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_A)).toBe(1);
    expect(TELUGU_A.strokes).toHaveLength(2);
    expect(
      TELUGU_A.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["turn around the left lobe", "sweep around the broad lower bowl"],
      ["turn around the right lobe", "return left along the inner bar"],
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
        "turn around the hooked left lobe and sweep through the broad lower bowl",
      ],
      [
        "turn around the rounded right lobe and return left along the inner bar",
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
        "continue down and around the broad lower bowl",
        "curl upward around the rounded right lobe without lifting",
      ],
      ["lift and draw the inner horizontal bar from left to right"],
      ["lift again and draw the short upper headstroke downward"],
    ]);
  });

  it("Telugu ఎ groups three source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_E)).toBe(1);
    expect(TELUGU_E.strokes).toHaveLength(2);
    expect(
      TELUGU_E.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left around the compact lower loop",
        "continue around its base and return to the central junction",
      ],
      ["restart at the junction and sweep up through the broad outer arch"],
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
        "turn down and left around the compact lower loop",
        "continue around its base and return to the central junction",
      ],
      [
        "restart at the lower-right tail and sweep up through the broad outer arch",
      ],
      ["restart below the upper-left hook and sweep upward to its tip"],
    ]);
  });

  it("Telugu ఋ preserves all six source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_VOCALIC_R)).toBe(5);
    expect(TELUGU_VOCALIC_R.strokes).toHaveLength(6);
    expect(
      TELUGU_VOCALIC_R.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "sweep right across the upper shoulder",
      "curve down around the left bowl",
      "sweep right around the lower bowl",
      "curl up around the first right lobe",
      "curl up around the middle lobe",
      "curl up around the final lobe",
    ]);
  });

  it("Telugu ఐ preserves all five source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_AI)).toBe(4);
    expect(TELUGU_AI.strokes).toHaveLength(5);
    expect(
      TELUGU_AI.strokes.map((stroke) => stroke.segments[0]!.label),
    ).toEqual([
      "sweep left across the compact upper arch",
      "curve down around the left bowl",
      "sweep right around the broad lower bowl",
      "sweep left across the upper-right arch",
      "sweep left across the upper-left arch",
    ]);
  });

  it("Telugu ఒ preserves all three source-verified pen-down runs", () => {
    expect(penLifts(TELUGU_O)).toBe(2);
    expect(TELUGU_O.strokes).toHaveLength(3);
    expect(TELUGU_O.strokes.map((stroke) => stroke.segments[0]!.label)).toEqual(
      [
        "sweep right across the upper arch",
        "curve down around the left bowl",
        "sweep right around the broad lower bowl",
      ],
    );
  });
});
