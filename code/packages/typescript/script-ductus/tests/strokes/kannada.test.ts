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
const KANNADA_LONG_I = DUCTUS[ductusKey("kannada", "ಈ")];
const KANNADA_U = DUCTUS[ductusKey("kannada", "ಉ")];
const KANNADA_UU = DUCTUS[ductusKey("kannada", "ಊ")];
const KANNADA_E = DUCTUS[ductusKey("kannada", "ಎ")];
const KANNADA_EE = DUCTUS[ductusKey("kannada", "ಏ")];
const KANNADA_O = DUCTUS[ductusKey("kannada", "ಒ")];
const KANNADA_OO = DUCTUS[ductusKey("kannada", "ಓ")];
const KANNADA_AI = DUCTUS[ductusKey("kannada", "ಐ")];
const KANNADA_VOCALIC_R = DUCTUS[ductusKey("kannada", "ಋ")];
const KANNADA_VISARGA = DUCTUS[ductusKey("kannada", "ಃ")];
const KANNADA_NA = DUCTUS[ductusKey("kannada", "ನ")];
const KANNADA_TA = DUCTUS[ductusKey("kannada", "ತ")];
const KANNADA_DA = DUCTUS[ductusKey("kannada", "ದ")];
const KANNADA_RA = DUCTUS[ductusKey("kannada", "ರ")];
const KANNADA_KA = DUCTUS[ductusKey("kannada", "ಕ")];
const KANNADA_GA = DUCTUS[ductusKey("kannada", "ಗ")];
const KANNADA_BA = DUCTUS[ductusKey("kannada", "ಬ")];
const KANNADA_LLA = DUCTUS[ductusKey("kannada", "ಳ")];
const KANNADA_YA = DUCTUS[ductusKey("kannada", "ಯ")];
const KANNADA_DDA = DUCTUS[ductusKey("kannada", "ಡ")];
const KANNADA_HA = DUCTUS[ductusKey("kannada", "ಹ")];
const KANNADA_SA = DUCTUS[ductusKey("kannada", "ಸ")];
const KANNADA_CA = DUCTUS[ductusKey("kannada", "ಚ")];
const KANNADA_PA = DUCTUS[ductusKey("kannada", "ಪ")];
const KANNADA_JHA = DUCTUS[ductusKey("kannada", "ಝ")];
const KANNADA_THA = DUCTUS[ductusKey("kannada", "ಥ")];
const KANNADA_MA = DUCTUS[ductusKey("kannada", "ಮ")];
const KANNADA_LA = DUCTUS[ductusKey("kannada", "ಲ")];
const KANNADA_VA = DUCTUS[ductusKey("kannada", "ವ")];
const KANNADA_JA = DUCTUS[ductusKey("kannada", "ಜ")];

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

  it("Kannada ಈ separates its body from the crossbar with one lift", () => {
    expect(penLifts(KANNADA_LONG_I)).toBe(1);
    expect(KANNADA_LONG_I.strokes.map((stroke) =>
      stroke.segments.map((segment) => segment.label),
    )).toEqual([
      [
        "draw the broad rounded body and return to its upper-right join",
        "sweep the upper bar left, retrace it right, and curl upward",
      ],
      [
        "lift, then draw the horizontal crossbar from left to right",
        "turn around the small right loop and descend into the lower hook",
      ],
    ]);
    expect(KANNADA_LONG_I.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ee.gif",
    );
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

  it("Kannada ನ climbs from its tail through both bowls, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_NA)).toBe(1);
    expect(KANNADA_NA.strokes).toHaveLength(2);
    expect(
      KANNADA_NA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "rise from the tail around the left bowl",
        "slant down into the right bowl",
        "climb the right side to the top bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_NA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-na.gif",
    );
    expect(verifiedLetterFont("ನ", KANNADA_NA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ತ carries the bowl into its inner loop, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_TA)).toBe(1);
    expect(KANNADA_TA.strokes).toHaveLength(2);
    expect(
      KANNADA_TA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep down and around the broad bowl",
        "turn left over the top into the inner loop",
        "close the small loop and rise to the top bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_TA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-tha.gif",
    );
    expect(verifiedLetterFont("ತ", KANNADA_TA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ದ closes its pointed bowl in one run, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_DA)).toBe(1);
    expect(KANNADA_DA.strokes).toHaveLength(2);
    expect(
      KANNADA_DA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side into the left lobe",
        "drop from the point around the right lobe",
        "close the bowl leftward along the top",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_DA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-dha.gif",
    );
    expect(verifiedLetterFont("ದ", KANNADA_DA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ರ closes its round bowl in one run, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_RA)).toBe(1);
    expect(KANNADA_RA.strokes).toHaveLength(2);
    expect(
      KANNADA_RA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side and round the base",
        "climb the right side and close leftward",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_RA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ra.gif",
    );
    expect(verifiedLetterFont("ರ", KANNADA_RA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಕ draws bowl, lower bar, link, and hooked upper bar in four runs", () => {
    expect(penLifts(KANNADA_KA)).toBe(3);
    expect(KANNADA_KA.strokes).toHaveLength(4);
    expect(
      KANNADA_KA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side and round the base",
        "climb the right side and close leftward",
      ],
      [
        "lift, then draw the lower bar rightward",
      ],
      [
        "lift, then draw the short link upward",
      ],
      [
        "lift, then draw the upper bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_KA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ka.gif",
    );
    expect(verifiedLetterFont("ಕ", KANNADA_KA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಗ carries both legs over the arch, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_GA)).toBe(1);
    expect(KANNADA_GA.strokes).toHaveLength(2);
    expect(
      KANNADA_GA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the left leg into the arch",
        "arch over and go down the right leg",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_GA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ga.gif",
    );
    expect(verifiedLetterFont("ಗ", KANNADA_GA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಬ loops its head, both lobes, and the tall right side in one run", () => {
    expect(penLifts(KANNADA_BA)).toBe(0);
    expect(KANNADA_BA.strokes).toHaveLength(1);
    expect(
      KANNADA_BA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop over the head from its curled tip",
        "slant round the left lobe to the point",
        "drop round the right lobe's base",
        "climb the right side to its tip",
      ],
    ]);
    expect(KANNADA_BA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ba.gif",
    );
    expect(verifiedLetterFont("ಬ", KANNADA_BA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಳ carries both loops into the right bowl, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_LLA)).toBe(1);
    expect(KANNADA_LLA.strokes).toHaveLength(2);
    expect(
      KANNADA_LLA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "close the small loop counterclockwise",
        "sweep round the left and along the base",
        "round the lower loop and recross its top",
        "climb the right bowl to the top bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_LLA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-lla.gif",
    );
    expect(verifiedLetterFont("ಳ", KANNADA_LLA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಯ draws bowl, middle arm, hooked bar, and small right bowl in four runs", () => {
    expect(penLifts(KANNADA_YA)).toBe(3);
    expect(KANNADA_YA.strokes).toHaveLength(4);
    expect(
      KANNADA_YA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side and round the base",
        "close the bowl over the top",
      ],
      [
        "lift, then dip and climb the middle arm",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
      [
        "lift, then round the small right bowl",
        "climb its right side and curl in at the top",
      ],
    ]);
    expect(KANNADA_YA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ya.gif",
    );
    expect(verifiedLetterFont("ಯ", KANNADA_YA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಡ closes its bowl through the inner loop, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_DDA)).toBe(1);
    expect(KANNADA_DDA.strokes).toHaveLength(2);
    expect(
      KANNADA_DDA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down into the left lobe and the point",
        "round the right lobe and up the right side",
        "curl into the small loop and round it",
        "rise out and close the bowl along the top",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_DDA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-da.gif",
    );
    expect(verifiedLetterFont("ಡ", KANNADA_DDA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಹ carries both rings up the neck, then lifts once for the hooked bar", () => {
    expect(penLifts(KANNADA_HA)).toBe(1);
    expect(KANNADA_HA.strokes).toHaveLength(2);
    expect(
      KANNADA_HA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "close the left ring counterclockwise",
        "arch over into the right ring's outer side",
        "round the right ring and climb to the waist",
        "rise up the neck to the top bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_HA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ha.gif",
    );
    expect(verifiedLetterFont("ಹ", KANNADA_HA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಸ draws body, hooked bar, and dot in three runs", () => {
    expect(penLifts(KANNADA_SA)).toBe(2);
    expect(KANNADA_SA.strokes).toHaveLength(3);
    expect(
      KANNADA_SA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb from the tail round the left curve",
        "slant down to the right into the base",
        "climb the right side and curve in at the top",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
      [
        "lift, then set the dot in the middle",
      ],
    ]);
    expect(KANNADA_SA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-sa.gif",
    );
    expect(verifiedLetterFont("ಸ", KANNADA_SA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಚ draws ಬ's body, then the lower bar, link, and hooked bar after three lifts", () => {
    expect(penLifts(KANNADA_CA)).toBe(3);
    expect(KANNADA_CA.strokes).toHaveLength(4);
    expect(
      KANNADA_CA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop over the head from its curled tip",
        "slant round the left lobe to the point",
        "drop round the right lobe's base",
        "climb the right side to the lower bar",
      ],
      [
        "lift, then draw the lower bar rightward",
      ],
      [
        "lift, then draw the short link upward",
      ],
      [
        "lift, then draw the upper bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_CA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-cha.gif",
    );
    expect(verifiedLetterFont("ಚ", KANNADA_CA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಪ draws body, dot, and hooked bar in three runs", () => {
    expect(penLifts(KANNADA_PA)).toBe(2);
    expect(KANNADA_PA.strokes).toHaveLength(3);
    expect(
      KANNADA_PA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "wind round the curl from its inner tip",
        "run along the base into the middle point",
        "drop round the right lobe and climb",
      ],
      [
        "lift, then set the dot in the middle",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_PA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-pa.gif",
    );
    expect(verifiedLetterFont("ಪ", KANNADA_PA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಝ draws bowl, hooked bar, both arms, and tail in five runs", () => {
    expect(penLifts(KANNADA_JHA)).toBe(4);
    expect(KANNADA_JHA.strokes).toHaveLength(5);
    expect(
      KANNADA_JHA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side and round the base",
        "climb the right side and close leftward",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
      [
        "lift, then round the middle arm",
        "climb its right side and curl in at the top",
      ],
      [
        "lift, then round the right arm",
        "climb its right side and curl in at the top",
      ],
      [
        "lift, then draw the tail downward",
      ],
    ]);
    expect(KANNADA_JHA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-jha.gif",
    );
    expect(verifiedLetterFont("ಝ", KANNADA_JHA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಥ closes its bowl, then lifts for the hooked bar, the tail, and the dot", () => {
    expect(penLifts(KANNADA_THA)).toBe(3);
    expect(KANNADA_THA.strokes).toHaveLength(4);
    expect(
      KANNADA_THA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "go down the left side into the left lobe",
        "drop from the point around the right lobe",
        "close the bowl leftward along the top",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
      [
        "lift, then draw the tail downward",
      ],
      [
        "lift, then set the dot in the middle",
      ],
    ]);
    expect(KANNADA_THA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-thha.gif",
    );
    expect(verifiedLetterFont("ಥ", KANNADA_THA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಮ draws body, hooked bar, and right bowl in three runs", () => {
    expect(penLifts(KANNADA_MA)).toBe(2);
    expect(KANNADA_MA.strokes).toHaveLength(3);
    expect(
      KANNADA_MA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "wind round the curl from its inner tip",
        "run along the base into the middle point",
        "drop round the right lobe up to the bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
      [
        "lift, then round the right bowl",
        "climb its right side and curl in at the top",
      ],
    ]);
    expect(KANNADA_MA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ma.gif",
    );
    expect(verifiedLetterFont("ಮ", KANNADA_MA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಲ carries its loop round the base and up the right side in one run", () => {
    expect(penLifts(KANNADA_LA)).toBe(0);
    expect(KANNADA_LA.strokes).toHaveLength(1);
    expect(
      KANNADA_LA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "close the small loop counterclockwise",
        "sweep down the left side and round the base",
        "climb the right side and curl in to its tip",
      ],
    ]);
    expect(KANNADA_LA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-la.gif",
    );
    expect(verifiedLetterFont("ಲ", KANNADA_LA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ವ carries the curl, base, and right side up to the bar, then lifts once", () => {
    expect(penLifts(KANNADA_VA)).toBe(1);
    expect(KANNADA_VA.strokes).toHaveLength(2);
    expect(
      KANNADA_VA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "wind round the curl from its inner tip",
        "run along the base into the middle point",
        "drop round the right lobe up to the bar",
      ],
      [
        "lift, then draw the top bar rightward",
        "curl up into the hook",
      ],
    ]);
    expect(KANNADA_VA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-va.gif",
    );
    expect(verifiedLetterFont("ವ", KANNADA_VA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });

  it("Kannada ಜ draws ಬ's body in one run, then lifts once for the upper arc", () => {
    expect(penLifts(KANNADA_JA)).toBe(1);
    expect(KANNADA_JA.strokes).toHaveLength(2);
    expect(
      KANNADA_JA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "loop over the head from its curled tip",
        "slant round the left lobe to the point",
        "drop round the right lobe's base",
        "climb the right side and curl in",
      ],
      [
        "lift, then sweep the arc from the head",
      ],
    ]);
    expect(KANNADA_JA.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ja.gif",
    );
    expect(verifiedLetterFont("ಜ", KANNADA_JA.source.url)).toBe(
      "_fonts/NotoSansKannada-Static.ttf",
    );
  });
});
