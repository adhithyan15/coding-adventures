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

const DEVANAGARI_A = DUCTUS[ductusKey("devanagari", "अ")];
const DEVANAGARI_AA = DUCTUS[ductusKey("devanagari", "आ")];
const DEVANAGARI_I = DUCTUS[ductusKey("devanagari", "इ")];
const DEVANAGARI_II = DUCTUS[ductusKey("devanagari", "ई")];
const DEVANAGARI_U = DUCTUS[ductusKey("devanagari", "उ")];
const DEVANAGARI_UU = DUCTUS[ductusKey("devanagari", "ऊ")];
const DEVANAGARI_E = DUCTUS[ductusKey("devanagari", "ए")];
const DEVANAGARI_AI = DUCTUS[ductusKey("devanagari", "ऐ")];
const DEVANAGARI_O = DUCTUS[ductusKey("devanagari", "ओ")];
const DEVANAGARI_AU = DUCTUS[ductusKey("devanagari", "औ")];
const DEVANAGARI_KA = DUCTUS[ductusKey("devanagari", "क")];
const DEVANAGARI_KHA = DUCTUS[ductusKey("devanagari", "ख")];
const DEVANAGARI_GA = DUCTUS[ductusKey("devanagari", "ग")];
const DEVANAGARI_GHA = DUCTUS[ductusKey("devanagari", "घ")];
const DEVANAGARI_CA = DUCTUS[ductusKey("devanagari", "च")];
const DEVANAGARI_CHA = DUCTUS[ductusKey("devanagari", "छ")];
const DEVANAGARI_JA = DUCTUS[ductusKey("devanagari", "ज")];
const DEVANAGARI_JHA = DUCTUS[ductusKey("devanagari", "झ")];
const DEVANAGARI_NYA = DUCTUS[ductusKey("devanagari", "ञ")];
const DEVANAGARI_TTA = DUCTUS[ductusKey("devanagari", "ट")];
const DEVANAGARI_TTHA = DUCTUS[ductusKey("devanagari", "ठ")];
const DEVANAGARI_DDA = DUCTUS[ductusKey("devanagari", "ड")];
const DEVANAGARI_DDHA = DUCTUS[ductusKey("devanagari", "ढ")];
const DEVANAGARI_NNA = DUCTUS[ductusKey("devanagari", "ण")];
const DEVANAGARI_TA = DUCTUS[ductusKey("devanagari", "त")];
const DEVANAGARI_THA = DUCTUS[ductusKey("devanagari", "थ")];
const DEVANAGARI_DA = DUCTUS[ductusKey("devanagari", "द")];
const DEVANAGARI_DHA = DUCTUS[ductusKey("devanagari", "ध")];
const DEVANAGARI_NA = DUCTUS[ductusKey("devanagari", "न")];
const DEVANAGARI_PA = DUCTUS[ductusKey("devanagari", "प")];
const DEVANAGARI_PHA = DUCTUS[ductusKey("devanagari", "फ")];
const DEVANAGARI_BA = DUCTUS[ductusKey("devanagari", "ब")];
const DEVANAGARI_BHA = DUCTUS[ductusKey("devanagari", "भ")];
const DEVANAGARI_MA = DUCTUS[ductusKey("devanagari", "म")];
const DEVANAGARI_YA = DUCTUS[ductusKey("devanagari", "य")];
const DEVANAGARI_RA = DUCTUS[ductusKey("devanagari", "र")];
const DEVANAGARI_LA = DUCTUS[ductusKey("devanagari", "ल")];
const DEVANAGARI_LLA = DUCTUS[ductusKey("devanagari", "ळ")];
const DEVANAGARI_VA = DUCTUS[ductusKey("devanagari", "व")];
const DEVANAGARI_SHA = DUCTUS[ductusKey("devanagari", "श")];
const DEVANAGARI_SSA = DUCTUS[ductusKey("devanagari", "ष")];
const DEVANAGARI_SA = DUCTUS[ductusKey("devanagari", "स")];
const DEVANAGARI_HA = DUCTUS[ductusKey("devanagari", "ह")];

const OWNER_SCRIPTS = new Set(["devanagari"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { ख: 0.95 });

  beforeAll(() => {
    expect(verifiedLetterFont("अ", DEVANAGARI_A.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("आ", DEVANAGARI_AA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("इ", DEVANAGARI_I.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ई", DEVANAGARI_II.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("उ", DEVANAGARI_U.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ऊ", DEVANAGARI_UU.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ए", DEVANAGARI_E.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ऐ", DEVANAGARI_AI.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ओ", DEVANAGARI_O.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("औ", DEVANAGARI_AU.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("क", DEVANAGARI_KA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ग", DEVANAGARI_GA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("च", DEVANAGARI_CA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("त", DEVANAGARI_TA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("द", DEVANAGARI_DA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ध", DEVANAGARI_DHA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("न", DEVANAGARI_NA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("प", DEVANAGARI_PA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("ब", DEVANAGARI_BA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("भ", DEVANAGARI_BHA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("म", DEVANAGARI_MA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
    expect(verifiedLetterFont("य", DEVANAGARI_YA.source.url)).toBe(
      "_fonts/NotoSansDevanagari-Static.ttf",
    );
  });

  it("covers the shared Devanagari corpus's candrabindu and visarga marks", () => {
    const devanagari = SCRIPTS.find(
      (script) => script.script === "devanagari",
    )!;
    expect(devanagari.marks?.map((mark) => mark.mark)).toContain("ँ");
    expect(devanagari.marks?.map((mark) => mark.mark)).toContain("ः");
    expect(devanagari.marks).toHaveLength(15);
    expect(devanagari.complete).toBe(true);
  });

  it("models Devanagari nukta as one sourced below-base carrier composition", () => {
    const devanagari = SCRIPTS.find(
      (script) => script.script === "devanagari",
    )!;
    const nukta = devanagari.marks!.find((mark) => mark.mark === "़")!;
    expect(nukta.role).toBe("diacritic");
    expect(nukta.attachesAs).toMatch(/dot below a consonant.*modified sound/i);
    expect(nukta.examples?.map((example) => example.combined)).toEqual([
      "क़",
      "ख़",
      "ग़",
      "ज़",
      "ड़",
      "ढ़",
      "फ़",
    ]);
    for (const example of nukta.examples!) {
      expect(example.combined.normalize("NFD")).toBe(
        `${example.base}${nukta.mark}`,
      );
    }
    expect(nukta.compositionOrder).toEqual([
      "write the consonant carrier first",
      "add the nukta as a single subscript dot below the consonant",
    ]);
    expect(nukta.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/#G71127",
    );
    expect(nukta.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.1\.3.*U\+093C.*NUKTA/i,
    );
    expect(nukta.compositionSource?.variation).toMatch(
      /true diacritic.*subscript dot.*seven carrier combinations.*not a universal handwriting sequence.*learner convention/i,
    );
  });

  it("Devanagari अ joins its left body before the shoulder, stem, and headline", () => {
    expect(DEVANAGARI_A.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_A)).toBe(3);
    expect(DEVANAGARI_A.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_A.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1]);
    const upper = DEVANAGARI_A.strokes[0].segments[0].path;
    const lower = DEVANAGARI_A.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    const shoulder = penPath(DEVANAGARI_A.strokes[1]);
    const stem = penPath(DEVANAGARI_A.strokes[2]);
    const headline = penPath(DEVANAGARI_A.strokes[3]);
    expect(shoulder[0].x).toBeLessThan(shoulder.at(-1)!.x);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari आ adds its trailing stem before spanning both stems with the headline", () => {
    expect(DEVANAGARI_AA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AA)).toBe(4);
    expect(DEVANAGARI_AA.strokes).toHaveLength(5);
    expect(
      DEVANAGARI_AA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1, 1]);
    expect(DEVANAGARI_AA.strokes.slice(0, 3).map(penPath)).toEqual(
      DEVANAGARI_A.strokes.slice(0, 3).map(penPath),
    );
    const innerStem = penPath(DEVANAGARI_AA.strokes[2]);
    const trailingStem = penPath(DEVANAGARI_AA.strokes[3]);
    const headline = penPath(DEVANAGARI_AA.strokes[4]);
    expect(innerStem[0].y).toBeGreaterThan(innerStem.at(-1)!.y);
    expect(trailingStem[0].y).toBeGreaterThan(trailingStem.at(-1)!.y);
    expect(trailingStem[0].x).toBeGreaterThan(innerStem[0].x);
    expect(headline[0].x).toBeLessThan(innerStem[0].x);
    expect(headline.at(-1)!.x).toBeGreaterThan(trailingStem[0].x);
  });

  it("Devanagari इ keeps both bowls and the tail in one run before the headline", () => {
    expect(DEVANAGARI_I.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_I)).toBe(1);
    expect(DEVANAGARI_I.strokes).toHaveLength(2);
    expect(
      DEVANAGARI_I.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([4, 1]);
    const [upright, upper, lower, tail] = DEVANAGARI_I.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(upper[0]);
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(lower.at(-1)).toEqual(tail[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(
      upright[0].x,
    );
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(
      upper.at(-1)!.x,
    );
    expect(tail.at(-1)!.x).toBeGreaterThan(tail[0].x);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
    const headline = penPath(DEVANAGARI_I.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ई reuses the continuous इ body before its upper curl and headline", () => {
    expect(DEVANAGARI_II.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_II)).toBe(2);
    expect(DEVANAGARI_II.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_II.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([4, 1, 1]);
    expect(penPath(DEVANAGARI_II.strokes[0])).toEqual(
      penPath(DEVANAGARI_I.strokes[0]),
    );
    const curl = penPath(DEVANAGARI_II.strokes[1]);
    expect(Math.max(...curl.map((point) => point.y))).toBeGreaterThan(
      curl[0].y,
    );
    expect(Math.min(...curl.map((point) => point.x))).toBeLessThan(curl[0].x);
    expect(curl.at(-1)!.x).toBeGreaterThan(curl[0].x);
    const headline = penPath(DEVANAGARI_II.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari उ keeps its upper bowl and lower loop joined before the headline", () => {
    expect(DEVANAGARI_U.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_U)).toBe(1);
    expect(DEVANAGARI_U.strokes).toHaveLength(2);
    expect(
      DEVANAGARI_U.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1]);
    const [upper, lower] = DEVANAGARI_U.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(upper.at(-1)).toEqual(lower[0]);
    expect(Math.max(...upper.map((point) => point.y))).toBeGreaterThan(
      upper.at(-1)!.y,
    );
    expect(Math.max(...upper.map((point) => point.x))).toBeGreaterThan(
      upper.at(-1)!.x,
    );
    expect(Math.max(...lower.map((point) => point.x))).toBeGreaterThan(
      lower[0].x,
    );
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(lower.at(-1)!.x).toBeLessThan(lower[0].x);
    const headline = penPath(DEVANAGARI_U.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ऊ reuses the continuous उ body before its right loop and headline", () => {
    expect(DEVANAGARI_UU.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_UU)).toBe(2);
    expect(DEVANAGARI_UU.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_UU.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1]);
    expect(penPath(DEVANAGARI_UU.strokes[0])).toEqual(
      penPath(DEVANAGARI_U.strokes[0]),
    );
    const loop = penPath(DEVANAGARI_UU.strokes[1]);
    expect(Math.max(...loop.map((point) => point.y))).toBeGreaterThan(
      loop[0].y,
    );
    expect(Math.max(...loop.map((point) => point.x))).toBeGreaterThan(
      loop[0].x,
    );
    expect(loop.at(-1)!.y).toBeLessThan(loop[0].y);
    expect(loop.at(-1)!.x).toBeGreaterThan(loop[0].x);
    const headline = penPath(DEVANAGARI_UU.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ए joins its long stem to the tail before the short stem and headline", () => {
    expect(DEVANAGARI_E.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_E)).toBe(2);
    expect(DEVANAGARI_E.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_E.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1]);
    const [longStem, tail] = DEVANAGARI_E.strokes[0].segments.map(
      (segment) => segment.path,
    );
    expect(longStem[0].y).toBeGreaterThan(longStem.at(-1)!.y);
    expect(longStem.at(-1)).toEqual(tail[0]);
    expect(tail.at(-1)!.x).toBeGreaterThan(tail[0].x);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
    const shortStem = penPath(DEVANAGARI_E.strokes[1]);
    expect(shortStem[0].y).toBeGreaterThan(shortStem.at(-1)!.y);
    expect(shortStem.at(-1)!.x).toBeLessThan(shortStem[0].x);
    const headline = penPath(DEVANAGARI_E.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ऐ reuses the ए base before its upper arc and headline", () => {
    expect(DEVANAGARI_AI.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AI)).toBe(3);
    expect(DEVANAGARI_AI.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_AI.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1]);
    expect(penPath(DEVANAGARI_AI.strokes[0])).toEqual(
      penPath(DEVANAGARI_E.strokes[0]),
    );
    expect(penPath(DEVANAGARI_AI.strokes[1])).toEqual(
      penPath(DEVANAGARI_E.strokes[1]),
    );
    const arc = penPath(DEVANAGARI_AI.strokes[2]);
    expect(arc.at(-1)!.x).toBeLessThan(arc[0].x);
    expect(Math.max(...arc.map((point) => point.y))).toBeGreaterThan(arc[0].y);
    const headline = penPath(DEVANAGARI_AI.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ओ reuses the आ base before its upper arc and headline", () => {
    expect(DEVANAGARI_O.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_O)).toBe(5);
    expect(DEVANAGARI_O.strokes).toHaveLength(6);
    expect(
      DEVANAGARI_O.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1, 1, 1]);
    expect(DEVANAGARI_O.strokes.slice(0, 4).map(penPath)).toEqual(
      DEVANAGARI_AA.strokes.slice(0, 4).map(penPath),
    );
    const arc = penPath(DEVANAGARI_O.strokes[4]);
    expect(arc.at(-1)!.x).toBeLessThan(arc[0].x);
    expect(Math.max(...arc.map((point) => point.y))).toBeGreaterThan(arc[0].y);
    const headline = penPath(DEVANAGARI_O.strokes[5]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari औ reuses the आ base before its two upper arcs and headline", () => {
    expect(DEVANAGARI_AU.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_AU)).toBe(6);
    expect(DEVANAGARI_AU.strokes).toHaveLength(7);
    expect(
      DEVANAGARI_AU.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1, 1, 1, 1]);
    expect(DEVANAGARI_AU.strokes.slice(0, 4).map(penPath)).toEqual(
      DEVANAGARI_AA.strokes.slice(0, 4).map(penPath),
    );
    const lowerArc = penPath(DEVANAGARI_AU.strokes[4]);
    const tallerArc = penPath(DEVANAGARI_AU.strokes[5]);
    expect(lowerArc.at(-1)!.x).toBeLessThan(lowerArc[0].x);
    expect(tallerArc.at(-1)!.x).toBeLessThan(tallerArc[0].x);
    expect(Math.max(...tallerArc.map((point) => point.y))).toBeGreaterThan(
      Math.max(...lowerArc.map((point) => point.y)),
    );
    const headline = penPath(DEVANAGARI_AU.strokes[6]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari क draws its bowl before the central stem, right arch, and headline", () => {
    expect(DEVANAGARI_KA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_KA)).toBe(3);
    expect(DEVANAGARI_KA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_KA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const bowl = penPath(DEVANAGARI_KA.strokes[0]);
    expect(Math.min(...bowl.map((point) => point.x))).toBeLessThan(bowl[0].x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    const stem = penPath(DEVANAGARI_KA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const arch = penPath(DEVANAGARI_KA.strokes[2]);
    expect(Math.max(...arch.map((point) => point.x))).toBeGreaterThan(
      arch[0].x,
    );
    expect(arch[0].y).toBeGreaterThan(arch.at(-1)!.y);
    const headline = penPath(DEVANAGARI_KA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ख keeps its descending left body joined before the upper loop, right stem, and headline", () => {
    expect(DEVANAGARI_KHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_KHA)).toBe(3);
    expect(DEVANAGARI_KHA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_KHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const body = penPath(DEVANAGARI_KHA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    const loop = penPath(DEVANAGARI_KHA.strokes[1]);
    expect(Math.max(...loop.map((point) => point.x))).toBeGreaterThan(
      loop[0].x,
    );
    expect(Math.min(...loop.map((point) => point.y))).toBeLessThan(loop[0].y);
    const stem = penPath(DEVANAGARI_KHA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_KHA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ग joins its loop to the ascending stem before the right stem and headline", () => {
    expect(DEVANAGARI_GA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_GA)).toBe(2);
    expect(DEVANAGARI_GA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_GA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_GA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.y).toBeGreaterThan(body[0].y);
    const stem = penPath(DEVANAGARI_GA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_GA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari घ joins its curls and lower bowl before the short stem and headline", () => {
    expect(DEVANAGARI_GHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_GHA)).toBe(2);
    expect(DEVANAGARI_GHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_GHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_GHA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.y).toBeGreaterThan(body[0].y);
    const stem = penPath(DEVANAGARI_GHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_GHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari च joins its short bar to the rounded body before the right stem and headline", () => {
    expect(DEVANAGARI_CA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_CA)).toBe(2);
    expect(DEVANAGARI_CA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_CA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_CA.strokes[0]);
    expect(body[0].x).toBeLessThan(body[5].x);
    expect(Math.min(...body.slice(6).map((point) => point.y))).toBeLessThan(
      body[0].y,
    );
    expect(body.at(-1)!.x).toBeGreaterThan(body[6].x);
    const stem = penPath(DEVANAGARI_CA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_CA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ज keeps its open hook, lower bowl, and middle bar joined before the stem and headline", () => {
    expect(DEVANAGARI_JA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_JA)).toBe(2);
    expect(DEVANAGARI_JA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_JA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_JA.strokes[0]);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(body.at(-1)!.x).toBeGreaterThan(body.at(-2)!.x);
    const stem = penPath(DEVANAGARI_JA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_JA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari झ joins its bowls and tail before the crossbar, stem, and headline", () => {
    expect(DEVANAGARI_JHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_JHA)).toBe(3);
    expect(DEVANAGARI_JHA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_JHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const body = penPath(DEVANAGARI_JHA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    const crossbar = penPath(DEVANAGARI_JHA.strokes[1]);
    expect(crossbar[0].x).toBeLessThan(crossbar.at(-1)!.x);
    const stem = penPath(DEVANAGARI_JHA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_JHA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ञ separates its bowl, rising shoulder, lower stem, and headline", () => {
    expect(DEVANAGARI_NYA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_NYA)).toBe(3);
    expect(DEVANAGARI_NYA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_NYA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const bowl = penPath(DEVANAGARI_NYA.strokes[0]);
    expect(Math.max(...bowl.map((point) => point.x))).toBeGreaterThan(
      bowl[0].x,
    );
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    const shoulder = penPath(DEVANAGARI_NYA.strokes[1]);
    expect(shoulder.at(-1)!.y).toBeGreaterThan(shoulder[0].y);
    const stem = penPath(DEVANAGARI_NYA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_NYA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari छ keeps both left loops, the lower bowl, and inner loop joined before the stem and headline", () => {
    expect(DEVANAGARI_CHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_CHA)).toBe(2);
    expect(DEVANAGARI_CHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_CHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_CHA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(body.at(-1)!.y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    const stem = penPath(DEVANAGARI_CHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_CHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ट joins its descending stem to the open round body before the headline", () => {
    expect(DEVANAGARI_TTA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_TTA)).toBe(1);
    expect(DEVANAGARI_TTA.strokes).toHaveLength(2);
    expect(
      DEVANAGARI_TTA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = penPath(DEVANAGARI_TTA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[4].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    const headline = penPath(DEVANAGARI_TTA.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ठ separates its short stem, counterclockwise closed body, and headline", () => {
    expect(DEVANAGARI_TTHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_TTHA)).toBe(2);
    expect(DEVANAGARI_TTHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_TTHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const stem = penPath(DEVANAGARI_TTHA.strokes[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const body = penPath(DEVANAGARI_TTHA.strokes[1]);
    expect(body[0]).toEqual(body.at(-1));
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    const headline = penPath(DEVANAGARI_TTHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ण joins its left stem, lower bowl, and inner stem before the outer stem and headline", () => {
    expect(DEVANAGARI_NNA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_NNA)).toBe(2);
    expect(DEVANAGARI_NNA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_NNA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_NNA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    expect(body.at(-1)!.y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    const stem = penPath(DEVANAGARI_NNA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_NNA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ड keeps its stem, upper loop, and open lower bowl joined before the headline", () => {
    expect(DEVANAGARI_DDA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DDA)).toBe(1);
    expect(DEVANAGARI_DDA.strokes).toHaveLength(2);
    expect(
      DEVANAGARI_DDA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = penPath(DEVANAGARI_DDA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[4].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(
      body.at(-1)!.y,
    );
    expect(body.at(-1)!.x).toBeLessThan(body[0].x);
    const headline = penPath(DEVANAGARI_DDA.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ढ keeps its stem, outer bowl, and inner loop joined before the headline", () => {
    expect(DEVANAGARI_DDHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DDHA)).toBe(1);
    expect(DEVANAGARI_DDHA.strokes).toHaveLength(2);
    expect(
      DEVANAGARI_DDHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const body = penPath(DEVANAGARI_DDHA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[4].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.slice(20).map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(body.at(-1)!.y).toBeLessThan(
      Math.max(...body.map((point) => point.y)),
    );
    const headline = penPath(DEVANAGARI_DDHA.strokes[1]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari त sweeps its shoulder right-to-left before the right stem and headline", () => {
    expect(DEVANAGARI_TA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_TA)).toBe(2);
    expect(DEVANAGARI_TA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_TA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_TA.strokes[0]);
    expect(body[0].x).toBeGreaterThan(body[3].x);
    expect(Math.min(...body.slice(4).map((point) => point.y))).toBeLessThan(
      body[0].y,
    );
    expect(body.at(-1)!.x).toBeGreaterThan(
      Math.min(...body.map((point) => point.x)),
    );
    const stem = penPath(DEVANAGARI_TA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_TA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari थ keeps its upper spiral and broad lower bowl joined before the stem and headline", () => {
    expect(DEVANAGARI_THA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_THA)).toBe(2);
    expect(DEVANAGARI_THA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_THA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_THA.strokes[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(
      body[0].y,
    );
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    const stem = penPath(DEVANAGARI_THA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_THA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari द joins its outer body to the inner curl and tail after the short stem", () => {
    expect(DEVANAGARI_DA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DA)).toBe(2);
    expect(DEVANAGARI_DA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_DA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const stem = penPath(DEVANAGARI_DA.strokes[0]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const body = penPath(DEVANAGARI_DA.strokes[1]);
    expect(body[0]).toEqual(stem.at(-1));
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[14].x);
    expect(body.at(-1)!.y).toBeLessThan(body[14].y);
    const headline = penPath(DEVANAGARI_DA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ध separates its upper spiral, lower bowl, right stem, and headline", () => {
    expect(DEVANAGARI_DHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_DHA)).toBe(3);
    expect(DEVANAGARI_DHA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_DHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const spiral = penPath(DEVANAGARI_DHA.strokes[0]);
    expect(Math.max(...spiral.map((point) => point.y))).toBeGreaterThan(
      spiral[0].y,
    );
    expect(spiral.at(-1)!.x).toBeGreaterThan(spiral[0].x);
    const bowl = penPath(DEVANAGARI_DHA.strokes[1]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
    const stem = penPath(DEVANAGARI_DHA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_DHA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari न keeps its clockwise loop joined to the rightward shoulder", () => {
    expect(DEVANAGARI_NA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_NA)).toBe(2);
    expect(DEVANAGARI_NA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_NA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_NA.strokes[0]);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(
      body[0].y,
    );
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    const stem = penPath(DEVANAGARI_NA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_NA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari प joins its descending left stem to the lower bowl", () => {
    expect(DEVANAGARI_PA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_PA)).toBe(2);
    expect(DEVANAGARI_PA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_PA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_PA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[3].y);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeGreaterThan(
      Math.min(...body.map((point) => point.y)),
    );
    const stem = penPath(DEVANAGARI_PA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_PA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari फ joins its lower bowl to the retraced central stem before the arch and headline", () => {
    expect(DEVANAGARI_PHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_PHA)).toBe(2);
    expect(DEVANAGARI_PHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_PHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_PHA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(
      body[0].x + 20,
    );
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(body.at(-1)!.y).toBeLessThan(Math.min(body[0].y, body[17].y));
    const arch = penPath(DEVANAGARI_PHA.strokes[1]);
    expect(Math.max(...arch.map((point) => point.x))).toBeGreaterThan(
      arch[0].x,
    );
    expect(arch.at(-1)!.y).toBeLessThan(arch[0].y);
    const headline = penPath(DEVANAGARI_PHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ब separates its counterclockwise oval, right stem, inner diagonal, and headline", () => {
    expect(DEVANAGARI_BA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_BA)).toBe(3);
    expect(DEVANAGARI_BA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_BA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const body = penPath(DEVANAGARI_BA.strokes[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(
      body[0].y,
    );
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.x).toBeGreaterThan(
      Math.min(...body.map((point) => point.x)),
    );
    const stem = penPath(DEVANAGARI_BA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const diagonal = penPath(DEVANAGARI_BA.strokes[2]);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
    const headline = penPath(DEVANAGARI_BA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari भ joins both clockwise loops and the crossbar before the lifted right stem", () => {
    expect(DEVANAGARI_BHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_BHA)).toBe(2);
    expect(DEVANAGARI_BHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_BHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_BHA.strokes[0]);
    expect(Math.min(...body.slice(0, 14).map((point) => point.x))).toBeLessThan(
      body[0].x,
    );
    expect(
      Math.max(...body.slice(0, 14).map((point) => point.y)),
    ).toBeGreaterThan(body[0].y);
    expect(
      Math.max(...body.slice(0, 14).map((point) => point.x)),
    ).toBeGreaterThan(body[0].x);
    expect(body[17].y).toBeLessThan(body[14].y);
    expect(body.at(-1)!.x).toBeGreaterThan(body[17].x);
    const stem = penPath(DEVANAGARI_BHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_BHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari म joins its descending left stem to the clockwise lower loop and crossbar", () => {
    expect(DEVANAGARI_MA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_MA)).toBe(2);
    expect(DEVANAGARI_MA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_MA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_MA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body[5].y);
    expect(Math.min(...body.slice(5, 15).map((point) => point.x))).toBeLessThan(
      body[5].x,
    );
    expect(Math.min(...body.slice(5, 15).map((point) => point.y))).toBeLessThan(
      body[5].y,
    );
    expect(body.at(-1)!.x).toBeGreaterThan(body[5].x);
    const stem = penPath(DEVANAGARI_MA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_MA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari य separates its clockwise inner curl from the lower bowl", () => {
    expect(DEVANAGARI_YA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_YA)).toBe(3);
    expect(DEVANAGARI_YA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_YA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const curl = penPath(DEVANAGARI_YA.strokes[0]);
    expect(Math.max(...curl.map((point) => point.x))).toBeGreaterThan(
      curl[0].x,
    );
    expect(curl.at(-1)!.x).toBeLessThan(curl[0].x);
    expect(curl.at(-1)!.y).toBeLessThan(curl[0].y);
    const bowl = penPath(DEVANAGARI_YA.strokes[1]);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    const stem = penPath(DEVANAGARI_YA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_YA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari र separates its looped stem from the diagonal tail", () => {
    expect(DEVANAGARI_RA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_RA)).toBe(2);
    expect(DEVANAGARI_RA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_RA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const loop = penPath(DEVANAGARI_RA.strokes[0]);
    expect(loop[0].y).toBeGreaterThan(loop[7].y);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[7].x);
    expect(Math.max(...loop.slice(8).map((point) => point.y))).toBeGreaterThan(
      loop[7].y,
    );
    const tail = penPath(DEVANAGARI_RA.strokes[1]);
    expect(tail[0].x).toBeLessThan(tail.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
    const headline = penPath(DEVANAGARI_RA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ल draws its clockwise open loop before the diagonal arm", () => {
    expect(DEVANAGARI_LA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_LA)).toBe(3);
    expect(DEVANAGARI_LA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_LA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const loop = penPath(DEVANAGARI_LA.strokes[0]);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[0].x);
    expect(Math.max(...loop.map((point) => point.y))).toBeGreaterThan(
      loop[0].y,
    );
    expect(loop.at(-1)!.x).toBeGreaterThan(
      Math.min(...loop.map((point) => point.x)),
    );
    const arm = penPath(DEVANAGARI_LA.strokes[1]);
    expect(arm[0].x).toBeLessThan(arm.at(-1)!.x);
    expect(arm[0].y).toBeLessThan(arm.at(-1)!.y);
    const stem = penPath(DEVANAGARI_LA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_LA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ळ joins both loops before the short stem and headline", () => {
    expect(DEVANAGARI_LLA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_LLA)).toBe(2);
    expect(DEVANAGARI_LLA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_LLA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_LLA.strokes[0]);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(
      body[0].x,
    );
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(100);
    expect(Math.max(...body.map((point) => point.x))).toBeGreaterThan(650);
    const stem = penPath(DEVANAGARI_LLA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_LLA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari व circles counterclockwise before the right stem", () => {
    expect(DEVANAGARI_VA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_VA)).toBe(2);
    expect(DEVANAGARI_VA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_VA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const loop = penPath(DEVANAGARI_VA.strokes[0]);
    expect(Math.min(...loop.map((point) => point.x))).toBeLessThan(loop[0].x);
    expect(Math.min(...loop.map((point) => point.y))).toBeLessThan(loop[0].y);
    expect(loop.at(-1)!.x).toBeGreaterThan(
      Math.min(...loop.map((point) => point.x)),
    );
    const stem = penPath(DEVANAGARI_VA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_VA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari श joins both loops and its tail before the right stem", () => {
    expect(DEVANAGARI_SHA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_SHA)).toBe(2);
    expect(DEVANAGARI_SHA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_SHA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_SHA.strokes[0]);
    expect(Math.max(...body.map((point) => point.y))).toBeGreaterThan(
      body[0].y,
    );
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(body[0].x);
    expect(body.at(-1)!.y).toBeLessThan(body[0].y);
    const stem = penPath(DEVANAGARI_SHA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_SHA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ष separates its U-shaped body, descending right stem, diagonal, and headline", () => {
    expect(DEVANAGARI_SSA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_SSA)).toBe(3);
    expect(DEVANAGARI_SSA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_SSA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const body = penPath(DEVANAGARI_SSA.strokes[0]);
    expect(Math.min(...body.map((point) => point.y))).toBeLessThan(body[0].y);
    expect(body.at(-1)!.y).toBe(body[0].y);
    const stem = penPath(DEVANAGARI_SSA.strokes[1]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const diagonal = penPath(DEVANAGARI_SSA.strokes[2]);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
    const headline = penPath(DEVANAGARI_SSA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari स joins its hook and tail before the middle crossbar", () => {
    expect(DEVANAGARI_SA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_SA)).toBe(3);
    expect(DEVANAGARI_SA.strokes).toHaveLength(4);
    expect(
      DEVANAGARI_SA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const body = penPath(DEVANAGARI_SA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(
      Math.min(...body.map((point) => point.x)),
    );
    const crossbar = penPath(DEVANAGARI_SA.strokes[1]);
    expect(crossbar[0].x).toBeLessThan(crossbar.at(-1)!.x);
    const stem = penPath(DEVANAGARI_SA.strokes[2]);
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    const headline = penPath(DEVANAGARI_SA.strokes[3]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari ह joins the stem, shoulder, and hooked body before the outer tail", () => {
    expect(DEVANAGARI_HA.script).toBe("devanagari");
    expect(penLifts(DEVANAGARI_HA)).toBe(2);
    expect(DEVANAGARI_HA.strokes).toHaveLength(3);
    expect(
      DEVANAGARI_HA.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1]);
    const body = penPath(DEVANAGARI_HA.strokes[0]);
    expect(body[0].y).toBeGreaterThan(body.at(-1)!.y);
    expect(Math.min(...body.map((point) => point.x))).toBeLessThan(body[0].x);
    expect(body.at(-1)!.x).toBeGreaterThan(
      Math.min(...body.map((point) => point.x)),
    );
    const tail = penPath(DEVANAGARI_HA.strokes[1]);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
    expect(Math.min(...tail.map((point) => point.x))).toBeLessThan(tail[0].x);
    expect(tail.at(-1)!.x).toBeGreaterThan(
      Math.min(...tail.map((point) => point.x)),
    );
    const headline = penPath(DEVANAGARI_HA.strokes[2]);
    expect(headline[0].x).toBeLessThan(headline.at(-1)!.x);
  });

  it("Devanagari अ traces its four-run modern form and records the six-stroke variant", () => {
    const src = DEVANAGARI_A.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%85_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari अ stroke order\.svg.*frames 1–4.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /four buildup frames.*four ordered pen-down runs.*frame 1.*upper-left.*upper curve.*lower bowl.*without lifting.*frame 2.*middle junction.*sweeps right.*shoulder.*frame 3.*right stem top-to-bottom.*frame 4.*shirorekhā left-to-right.*three intervening lifts.*Thomas Egenes.*Learning the Sanskrit Alphabet.*p\. 12.*six-stroke traditional Sanskrit form.*Noto Sans Devanagari/i,
    );
  });

  it("Devanagari आ traces its five-run modern form without erasing base-letter variation", () => {
    const src = DEVANAGARI_AA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%86_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari आ stroke order\.svg.*frames 1–5.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /five buildup frames.*five ordered pen-down runs.*frame 1.*upper-left.*upper curve.*lower bowl.*without lifting.*frame 2.*middle junction.*sweeps right.*shoulder.*frame 3.*inner stem top-to-bottom.*frame 4.*trailing stem top-to-bottom.*frame 5.*shirorekhā left-to-right.*four intervening lifts.*Thomas Egenes.*Learning the Sanskrit Alphabet.*p\. 12.*shared base अ.*six-stroke traditional Sanskrit form.*joined modern body is not universal.*Noto Sans Devanagari/i,
    );
  });

  it("Devanagari इ traces its continuous body before the lifted headline", () => {
    const src = DEVANAGARI_I.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%87_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari इ stroke order\.svg.*panels 1–2.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /two-panel diagram.*body.*one continuous pen-down run.*panel 1.*green start.*top of the upright.*descend the upright.*turn left.*upper bowl.*sweep right.*waist.*lower bowl.*finish down-right.*tail.*without lifting.*panel 2.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ई traces the shared body, upper curl, and final headline", () => {
    const src = DEVANAGARI_II.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%88_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ई stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*same continuous upright.*upper bowl.*lower bowl.*down-right tail.*इ.*panel 2.*headline junction.*upper curl upward.*left.*around.*open right tip.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari उ traces its joined upper bowl and lower loop before the headline", () => {
    const src = DEVANAGARI_U.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%89_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari उ stroke order\.svg.*panels 1–2.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /two-panel diagram.*two ordered pen-down runs.*panel 1.*one green start.*headline junction.*first arrow.*down and left.*upper bowl.*second arrow.*same run.*back through the waist.*down around the lower loop.*without lifting.*panel 2.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ऊ traces the shared body, right loop, and final headline", () => {
    const src = DEVANAGARI_UU.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%8A_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ऊ stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*same continuous upper bowl.*lower loop.*उ.*panel 2.*waist.*right-hand loop upward and right.*outer turn.*down-left.*open tip.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ए traces its long stem and tail before the short stem and headline", () => {
    const src = DEVANAGARI_E.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%8F_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ए stroke order\.svg.*panels 1–3.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /three-panel diagram.*three ordered pen-down runs.*panel 1.*headline junction.*long left stem.*lower shoulder.*down through the tail.*without lifting.*panel 2.*shorter right stem's headline junction.*descends.*inward.*open hook.*panel 3.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ऐ traces the shared ए base, upper arc, and final headline", () => {
    const src = DEVANAGARI_AI.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%90_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ऐ stroke order\.svg.*panels 1–4.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /four-panel diagram.*four ordered pen-down runs.*panels 1 and 2.*same continuous long left stem and tail.*shorter inward-hooked right stem.*ए.*panel 3.*right headline junction.*upper arc upward and left.*open tip.*panel 4.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari ओ traces the shared आ base, upper arc, and final headline", () => {
    const src = DEVANAGARI_O.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%93_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari ओ stroke order\.svg.*panels 1–6.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /six-panel diagram.*six ordered pen-down runs.*panel 1.*joined upper-and-lower left body.*अ.*panel 2.*middle.*shoulder right.*panels 3 and 4.*inner and trailing stems.*आ.*panel 5.*trailing stem's headline junction.*upper arc upward and left.*open tip.*panel 6.*headline's left edge.*shirorekhā left-to-right.*five intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari औ traces the shared आ base, two upper arcs, and final headline", () => {
    const src = DEVANAGARI_AU.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%94_stroke_order.svg",
    );
    expect(src.citation).toMatch(
      /Saurmandal.*Devanagari औ stroke order\.svg.*panels 1–7.*Wikimedia Commons.*5 August 2023/i,
    );
    expect(src.variation).toMatch(
      /seven-panel diagram.*seven ordered pen-down runs.*panel 1.*joined upper-and-lower left body.*अ.*panel 2.*middle.*shoulder right.*panels 3 and 4.*inner and trailing stems.*आ.*panels 5 and 6.*trailing stem's headline junction.*lower upper arc.*upward and left.*taller upper arc.*open tip.*panel 7.*headline's left edge.*shirorekhā left-to-right.*six intervening lifts.*modern printed teaching form.*Noto Sans Devanagari.*rather than.*universal standard/i,
    );
  });

  it("Devanagari क traces the animated four-run bowl, stem, arch, and headline order", () => {
    const src = DEVANAGARI_KA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%95-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-क-order\.gif.*strokes 1–4.*Wikimedia Commons.*8 May 2009/i,
    );
    expect(src.variation).toMatch(
      /27-frame animation.*four ordered pen-down runs.*frames 2–11.*upper-right junction.*left over the top.*down the left side.*around the bottom.*lower-right junction.*frames 12–15.*central stem top-to-bottom.*frames 16–19.*upper junction.*right-hand arch clockwise.*open tip.*frames 20–27.*shirorekhā left-to-right.*three intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit III.*p\. 12.*same four-part buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari ख traces the animated four-run joined body, upper loop, right stem, and headline order", () => {
    const src = DEVANAGARI_KHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%96-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ख-order\.gif.*strokes 1–4.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /28-frame animation.*four ordered pen-down runs.*frames 2–12.*descend the left stem.*clockwise around the small left opening.*broad lower bowl.*without lifting.*frames 13–20.*upper-left of the right opening.*clockwise around its oval.*open lower-left tip.*frames 21–23.*right stem's headline junction.*top-to-bottom.*frames 24–27.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*Noto Sans Devanagari.*everyday handwriting.*narrow or join the bowls/i,
    );
  });

  it("Devanagari ग traces the animated three-run loop, right stem, and headline order", () => {
    const src = DEVANAGARI_GA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%97-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ग-order\.gif.*strokes 1–3.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /18-frame animation.*three ordered pen-down runs.*frames 2–9.*upper-right junction.*left over the top.*counterclockwise loop.*continue up the joined stem.*headline.*frames 10–13.*right stem's headline junction.*top-to-bottom.*frames 14–18.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit III.*p\. 14.*same three-part buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari घ traces the animated joined-body, lower-stem, and headline order", () => {
    const src = DEVANAGARI_GHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%98-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-घ-order\.gif.*strokes 1–3.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*three ordered pen-down runs.*gray guide.*frames 3–14.*upper-left.*clockwise around the upper curl.*middle hook.*clockwise around the lower bowl.*right side.*headline.*without lifting.*frames 15–16.*lower right-side junction.*short stem.*below the bowl.*frames 17–21.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*210 ms hold.*frame 14.*260 ms hold.*frame 16.*one-second completed frame 21.*frame 6's 200 ms pause.*within the continuous body.*does not add a lift.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari च traces the animated three-run bar-body, right stem, and headline order", () => {
    const src = DEVANAGARI_CA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9A-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-च-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*three ordered pen-down runs.*frames 4–14.*upper bar left-to-right.*turn down and left.*shoulder.*rounded body.*open right junction.*without lifting.*frames 15–18.*right stem's headline junction.*top-to-bottom.*frames 19–22.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IV.*p\. 17.*upper bar.*rounded body.*right stem.*headline order.*staging.*separately.*rather than proving their join.*three-run lift count.*animation.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari ज traces the animated three-run hook-bowl, right stem, and headline order", () => {
    const src = DEVANAGARI_JA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9C-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ज-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /20-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–10.*open upper-left tip.*clockwise around the lower bowl.*inner shoulder.*middle bar.*right-stem junction.*without lifting.*frames 11–15.*right stem's headline junction.*top-to-bottom.*frames 16–18.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 10 and 15.*one-second completed frame 19.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari झ traces the animated joined body, crossbar, stem, and headline order", () => {
    const src = DEVANAGARI_JHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9D-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-झ-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /32-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–18.*short upper stem.*clockwise around the upper bowl.*waist.*clockwise around the lower loop.*diagonal tail.*without lifting.*frames 19–21.*central junction.*middle crossbar left-to-right.*frames 22–26.*right stem's headline junction.*top-to-bottom.*frames 27–31.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 18, 21, and 26.*one-second completed frame 31.*120–130 ms pauses.*joined body.*do not add lifts.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari ञ traces the animated bowl, rising shoulder, lower stem, and headline order", () => {
    const src = DEVANAGARI_NYA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9E-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ञ-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /21-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–8.*open upper-left tip.*clockwise around the bowl.*open lower-left tip.*frames 9–13.*bowl's right junction.*shoulder.*right side.*headline.*frames 14–16.*lower right-side junction.*short stem.*below the bowl.*frames 17–20.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 8, 13, and 16.*one-second completed frame 20.*frame 10's 120 ms pause.*within the shoulder.*does not add a lift.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari छ traces the animated three-run joined loops, upper stem, and headline order", () => {
    const src = DEVANAGARI_CHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9B-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-छ-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /28-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–20.*upper loop's headline-side junction.*counterclockwise around the upper-left loop.*clockwise around the broad lower bowl.*outer right side.*counterclockwise around the inner loop.*down-right tip.*without lifting.*frames 21–22.*short upper stem's headline junction.*top-to-bottom.*frames 23–26.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 20 and 22.*one-second completed frame 27.*Noto Sans Devanagari.*everyday handwriting.*join, narrow, or simplify/i,
    );
  });

  it("Devanagari ट traces the animated joined stem-body and headline order", () => {
    const src = DEVANAGARI_TTA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%9F-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ट-order\.gif.*strokes 1–2.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /17-frame animation.*two ordered pen-down runs.*gray guide.*frames 2–11.*central stem's headline junction.*top-to-bottom.*turn left.*upper shoulder.*counterclockwise around the open round body.*up-right tip.*without lifting.*frames 12–15.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*250 ms hold.*frame 11.*one-second completed frame 16.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari ठ traces the animated stem, counterclockwise closed body, and headline order", () => {
    const src = DEVANAGARI_TTHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A0-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ठ-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /18-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–3.*central stem.*top-to-bottom.*frames 4–13.*stem's lower junction.*counterclockwise around the closed.*body.*without lifting.*frames 14–16.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*230 ms hold at frame 3.*250 ms hold at frame 13.*one-second completed frame 17.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari ण traces the animated three-run joined bowl, outer stem, and headline order", () => {
    const src = DEVANAGARI_NNA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A3-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ण-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /19-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–9.*left stem's headline junction.*top-to-bottom.*clockwise around the lower bowl.*inner right stem.*back to the headline.*without lifting.*frames 10–13.*outer right stem's headline junction.*top-to-bottom.*frames 14–17.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 9 and 13.*one-second completed frame 18.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari ड traces the animated joined S-body and headline order", () => {
    const src = DEVANAGARI_DDA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A1-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ड-order\.gif.*strokes 1–2.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /20-frame animation.*two ordered pen-down runs.*gray guide.*frames 2–15.*right stem's headline junction.*top-to-bottom.*turn left.*upper shoulder.*counterclockwise around the upper-left loop.*clockwise around the broad lower bowl.*open up-left tip.*without lifting.*frames 16–18.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*250 ms hold.*frame 15.*one-second completed frame 19.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari ढ traces the animated joined bowl, inner loop, and headline order", () => {
    const src = DEVANAGARI_DDHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A2-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ढ-order\.gif.*strokes 1–2.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*two ordered pen-down runs.*gray guide.*frames 2–16.*right stem's headline junction.*top-to-bottom.*turn left.*upper shoulder.*counterclockwise around the broad outer bowl.*counterclockwise around the closed inner loop.*without lifting.*frames 17–20.*headline's left edge.*shirorekhā left-to-right.*one intervening lift.*250 ms hold.*frame 16.*one-second completed frame 21.*120–220 ms pauses.*frames 3, 7–8, and 12–15.*continuous body.*do not add lifts.*Noto Sans Devanagari.*everyday handwriting.*narrow or simplify/i,
    );
  });

  it("Devanagari त traces the animated three-run body, right stem, and headline order", () => {
    const src = DEVANAGARI_TA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A4-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-त-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /17-frame animation.*three ordered pen-down runs.*frames 1–7.*upper-right junction.*left across the shoulder.*curve down around the left side.*finish down-right.*open lower tip.*without lifting.*frames 8–12.*right stem's headline junction.*top-to-bottom.*frames 13–16.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 30.*same body.*right stem.*headline buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari थ traces the animated joined spiral-bowl, right stem, and headline order", () => {
    const src = DEVANAGARI_THA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A5-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-थ-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /27-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–19.*upper spiral's inner-right tip.*clockwise around the small opening.*outward to the left waist.*clockwise around the broad lower bowl.*right-stem junction.*without lifting.*frames 20–22.*right stem's headline junction.*top-to-bottom.*frames 23–25.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 19 and 22.*one-second completed frame 26.*Noto Sans Devanagari.*everyday handwriting.*narrow, divide, or simplify/i,
    );
  });

  it("Devanagari द traces the animated three-run stem, joined body-tail, and headline order", () => {
    const src = DEVANAGARI_DA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A6-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-द-order\.gif.*strokes 1–3.*Wikimedia Commons.*8 May 2009/i,
    );
    expect(src.variation).toMatch(
      /18-frame animation.*three ordered pen-down runs.*frames 2–3.*short stem top-to-bottom.*frames 4–13.*lower junction.*left through the shoulder.*outer body.*inward and clockwise.*loop.*continue down-right.*tail without lifting.*frames 14–17.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 3 and 13.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 32.*short stem.*outer body.*inward curl-tail.*headline order.*staging.*separately.*rather than proving their join.*three-run lift count.*animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari ध traces the animated four-run spiral, bowl, stem, and headline order", () => {
    const src = DEVANAGARI_DHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A7-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ध-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /27-frame animation.*four ordered pen-down runs.*frames 2–11.*upper spiral's inner crossing.*small opening.*widen left and down.*outer loop.*right through the shoulder.*without lifting.*frames 12–19.*left waist.*down and around the lower bowl.*right junction.*frames 20–22.*right stem's headline junction.*top-to-bottom.*frames 23–26.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 11, 19, and 22.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 33.*same upper spiral.*lower bowl.*right stem.*headline buildup.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari न traces the animated three-run clockwise loop, stem, and headline order", () => {
    const src = DEVANAGARI_NA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%A8-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-न-order\.gif.*strokes 1–3.*Wikimedia Commons.*11 May 2009/i,
    );
    expect(src.variation).toMatch(
      /20-frame animation.*three ordered pen-down runs.*frames 2–9.*inner-right curve.*down and clockwise.*small opening.*continue right along the shoulder.*without lifting.*frames 10–14.*right stem's headline junction.*top-to-bottom.*frames 15–18.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 9 and 14.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VI.*p\. 34.*same clockwise loop-and-shoulder.*right stem.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari प traces the animated three-run left stem, bowl, right stem, and headline order", () => {
    const src = DEVANAGARI_PA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%AA-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-प-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /19-frame animation.*three ordered pen-down runs.*frames 2–10.*descend the left stem.*curve right around the lower bowl.*rise to its upper-right junction.*without lifting.*frames 11–13.*right stem's headline junction.*top-to-bottom.*frames 14–17.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 10 and 13.*one-second completed frame.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 35.*same left stem-and-bowl.*right stem.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari फ traces the animated joined bowl-stem, right arch, and headline order", () => {
    const src = DEVANAGARI_PHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_p%CA%B0_%E0%A4%AB.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari pʰ फ\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /15-frame animation.*three ordered pen-down runs.*frames 0–6.*left stem.*lower bowl.*central side.*retrace.*central stem.*without lifting.*frames 7–10.*clockwise.*right arch.*frames 11–14.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts at frames 7 and 11.*all frames last 100 ms.*no long inter-stroke holds.*Deskbook.*Unit VII.*p\. 36.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari ब traces the animated four-run oval, right stem, inner diagonal, and headline order", () => {
    const src = DEVANAGARI_BA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_b_%E0%A4%AC.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari b ब\.gif.*strokes 1–4.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /13-frame animation.*four ordered pen-down runs.*frames 0–6.*upper-right junction.*counterclockwise around the oval.*lower-right junction.*frames 7–8.*right stem's headline junction.*top-to-bottom.*frame 9.*upper-left interior.*inner diagonal down-right.*frames 10–12.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*spatial restarts.*frames 7, 9, and 10.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 37.*same oval body.*right stem.*inner diagonal.*headline buildup and directions.*Noto Sans Devanagari.*everyday handwriting.*join or simplify/i,
    );
  });

  it("Devanagari भ traces the animated joined-body, right-stem, and headline order", () => {
    const src = DEVANAGARI_BHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_b%CA%B0_%E0%A4%AD.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari bʰ भ\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /15-frame animation.*three ordered pen-down runs.*frames 0–9.*upper loop's lower inner tip.*sweep left and clockwise around the upper loop.*descend the joined trunk.*clockwise around the lower bowl.*continue right through the crossbar.*without lifting.*frames 10–12.*right stem's headline junction.*top-to-bottom.*frames 13–14.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 10 and 13.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 38.*upper loop and trunk.*lower bowl and crossbar.*right stem.*headline order.*staging the upper and lower body parts separately.*rather than proving their join.*three-run lift count from the animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari म traces the animated joined left-body, right-stem, and headline order", () => {
    const src = DEVANAGARI_MA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_m_%E0%A4%AE.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari m म\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /12-frame animation.*three ordered pen-down runs.*frames 0–5.*descend the left stem.*curl left and clockwise around the lower loop.*continue right through the crossbar.*without lifting.*frames 6–8.*right stem's headline junction.*top-to-bottom.*frames 9–11.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 6 and 9.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VII.*p\. 39.*left stem.*lower loop and crossbar.*right stem.*headline order.*staging the left stem and loop-crossbar separately.*rather than proving their join.*three-run lift count from the animation.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari य traces the corroborated four-run form and records the joined-body variation", () => {
    const src = DEVANAGARI_YA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%AF-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-य-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–5.*beneath the headline.*clockwise around the inner curl.*left waist.*frames 6–13.*restart at that waist.*down and right around the lower bowl.*right-stem junction.*frames 14–16.*right stem's headline junction.*top-to-bottom.*frames 17–20.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*190 ms hold.*frame 5.*250 ms holds.*frames 13 and 16.*one-second completed frame 21.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 41.*inner curl.*lower bowl.*right stem.*headline buildup.*JackPotte.*11-frame.*Devanagari j य\.gif.*29 March 2009.*joins the inner curl and lower bowl.*separately descended right stem.*left-to-right headline.*four-run form.*Noto Sans Devanagari.*three-run join.*simplify the bowls/i,
    );
  });

  it("Devanagari र traces the corroborated three-run form and records the joined-tail variation", () => {
    const src = DEVANAGARI_RA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B0-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-र-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /17-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–9.*right stem's headline junction.*descend top-to-bottom.*curl left and clockwise around the lower loop.*tail junction.*frames 10–12.*restart at that junction.*diagonal tail down-right.*frames 13–16.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*240 ms hold.*frame 9.*250 ms hold.*frame 12.*one-second completed frame 16.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 42.*looped stem.*diagonal tail.*headline buildup.*JackPotte.*seven-frame.*Devanagari r र\.gif.*29 March 2009.*joins the descending stem.*clockwise lower loop.*diagonal tail.*one continuous body.*separate left-to-right headline.*three-run form.*Noto Sans Devanagari.*two-run join.*simplify the lower loop/i,
    );
  });

  it("Devanagari ल traces the corroborated loop-first form and records the stem-first variation", () => {
    const src = DEVANAGARI_LA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B2-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ल-order\.gif.*strokes 1–4.*Wikimedia Commons.*11 May 2009/i,
    );
    expect(src.variation).toMatch(
      /23-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–9.*open lower-left tip.*curve up and clockwise around the left loop.*inner junction.*frames 10–12.*restart at that junction.*diagonal arm up-right.*right stem.*frames 13–17.*right stem's headline junction.*descend top-to-bottom.*frames 18–21.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 9, 12, and 17.*one-second completed frame 22.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 43.*left loop.*diagonal arm.*right stem.*headline buildup.*JackPotte.*12-frame.*Devanagari l ल\.gif.*29 March 2009.*right stem first.*frames 0–2.*diagonal arm.*frames 3–5.*left loop.*frames 6–8.*headline.*frames 9–10.*all frames last 100 ms.*loop-first four-run form.*Noto Sans Devanagari.*stem-first order.*simplify the loop/i,
    );
  });

  it("Devanagari ळ traces the published three-array figure-eight order", () => {
    const src = DEVANAGARI_LLA.source;
    expect(src.url).toBe(
      "https://helanomad.com/indic-script-explorer/assets/strokes/deva/consonants/U0933.json",
    );
    expect(src.citation).toMatch(
      /Hela Nomad.*U0933\.json.*Devanagari ळ stroke data.*strokes 1–3.*Indic Script Explorer.*accessed 24 August 2026/i,
    );
    expect(src.variation).toMatch(
      /published data.*letter ळ.*three ordered path arrays.*Stroke 1.*128 points.*center junction.*left.*complete left loop.*back through the center.*right.*complete right loop.*return.*without lifting.*Stroke 2.*10 points.*one x-position.*descends.*headline area.*top of the right loop.*Stroke 3.*36 points.*headline's left edge.*right edge.*two intervening lifts.*companion Devanagari Alphabet page.*stroke-by-stroke visualization.*Martel Sans.*three-array order.*Noto Sans Devanagari.*handwriting.*narrow, tilt, or join/i,
    );
  });

  it("Devanagari व traces its three-run animation and deskbook buildup", () => {
    const src = DEVANAGARI_VA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_v_%E0%A4%B5.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari v व\.gif.*strokes 1–3.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /11-frame animation.*three ordered pen-down runs.*frames 0–5.*upper-right of the body.*travel left around the top.*counterclockwise around the loop.*right-side junction.*frames 6–7.*right stem's headline junction.*descend top-to-bottom.*frames 8–10.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*spatial restarts.*frames 6 and 8.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit VIII.*p\. 44.*loop.*right stem.*headline buildup.*animation supplies.*directions and lift evidence.*corroborated three-run form.*Noto Sans Devanagari.*simplify or narrow the loop/i,
    );
  });

  it("Devanagari श traces its corroborated joined-body animation", () => {
    const src = DEVANAGARI_SHA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B6-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-श-order\.gif.*strokes 1–3.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /25-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–16.*upper loop's lower inner tip.*sweep left and clockwise around the upper loop.*descend the joined outer curve.*curl around the lower loop.*continue down-right through the diagonal tail.*without lifting.*frames 17–21.*right stem's headline junction.*descend top-to-bottom.*frames 22–24.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*250 ms holds.*frames 16 and 21.*one-second completed frame 24.*JackPotte.*26-frame.*Devanagari ç श\.gif.*same joined body.*restarted right stem.*restarted headline.*spatial restarts.*frames 14 and 21.*all its frames last 100 ms.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 45.*body.*right-stem.*headline buildup.*animations supply.*directions and lift evidence.*corroborated three-run form.*Noto Sans Devanagari.*simplify or narrow the loops/i,
    );
  });

  it("Devanagari ष traces the animated U-body, right stem, diagonal, and headline order", () => {
    const src = DEVANAGARI_SSA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B7-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ष-order\.gif.*strokes 1–4.*Wikimedia Commons.*10 May 2009/i,
    );
    expect(src.variation).toMatch(
      /24-frame animation.*four ordered pen-down runs.*gray guide.*frames 2–11.*left side.*counterclockwise around the lower bowl.*right side.*headline.*without lifting.*frames 12–14.*right stem's headline junction.*top-to-bottom.*frames 15–18.*upper-left interior.*diagonal down-right.*frames 19–22.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*250 ms holds.*frames 11, 14, and 18.*one-second completed frame 23.*Noto Sans Devanagari.*everyday handwriting.*divide or simplify/i,
    );
  });

  it("Devanagari स traces its joined-body animation and staged corroboration", () => {
    const src = DEVANAGARI_SA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_s_%E0%A4%B8.gif",
    );
    expect(src.citation).toMatch(
      /JackPotte.*Devanagari s स\.gif.*strokes 1–4.*Wikimedia Commons.*29 March 2009/i,
    );
    expect(src.variation).toMatch(
      /13-frame animation.*four ordered pen-down runs.*frames 0–4.*just below the headline.*descend the left stem.*curve left around the central hook.*continue diagonally down-right through the tail.*without lifting.*frames 5–6.*central junction.*middle crossbar left-to-right.*frames 7–9.*right stem's headline junction.*descend top-to-bottom.*frames 10–12.*headline's left edge.*shirorekhā left-to-right.*three intervening lifts.*spatial restarts.*frames 5, 7, and 10.*all frames last 100 ms.*no long inter-stroke holds.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 47.*left curve.*diagonal tail.*middle crossbar.*right stem.*headline order.*stages the left curve and tail separately.*rather than proving their join.*four-run lift count.*animation.*Noto Sans Devanagari.*divide or simplify the joined body/i,
    );
  });

  it("Devanagari ह traces its three-run animation and staged corroboration", () => {
    const src = DEVANAGARI_HA.source;
    expect(src.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Deva-%E0%A4%B9-order.gif",
    );
    expect(src.citation).toMatch(
      /Opiaterein.*Deva-ह-order\.gif.*strokes 1–3.*Wikimedia Commons.*9 May 2009/i,
    );
    expect(src.variation).toMatch(
      /22-frame animation.*three ordered pen-down runs.*gray guide.*frames 2–12.*right stem's headline junction.*descend top-to-bottom.*sweep left through the shoulder.*curve clockwise around the hooked body.*lower-right tip.*without lifting.*frames 13–16.*body's left junction.*sweep down-left around the outer curve.*continue diagonally down-right through the tail.*frames 17–21.*headline's left edge.*shirorekhā left-to-right.*two intervening lifts.*230 ms hold.*frame 12.*250 ms hold.*frame 16.*one-second completed frame 21.*Central Hindi Directorate.*2019 Deskbook on Orthography of Devanagari Script.*Lesson 2.*Unit IX.*p\. 48.*right stem.*leftward shoulder.*hooked body.*outer curve and tail.*headline buildup.*stages the joined first body.*more component steps.*three-run lift count.*animation.*Noto Sans Devanagari.*divide or simplify the body/i,
    );
  });
});
