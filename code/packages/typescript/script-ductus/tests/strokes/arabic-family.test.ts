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

const ARABIC_ALEF = DUCTUS[ductusKey("arabic", "ا")];
const ARABIC_BAA = DUCTUS[ductusKey("arabic", "ب")];
const ARABIC_TAA = DUCTUS[ductusKey("arabic", "ت")];
const ARABIC_THAA = DUCTUS[ductusKey("arabic", "ث")];
const ARABIC_JEEM = DUCTUS[ductusKey("arabic", "ج")];
const ARABIC_HAA = DUCTUS[ductusKey("arabic", "ح")];
const PERSIAN_HAH = DUCTUS[ductusKey("perso-arabic", "ح")];
const URDU_BARI_HE = DUCTUS[ductusKey("urdu-nastaliq", "ح")];
const ARABIC_KHAA = DUCTUS[ductusKey("arabic", "خ")];
const PERSIAN_KHEH = DUCTUS[ductusKey("perso-arabic", "خ")];
const URDU_KHE = DUCTUS[ductusKey("urdu-nastaliq", "خ")];
const ARABIC_DAAL = DUCTUS[ductusKey("arabic", "د")];
const ARABIC_DHAAL = DUCTUS[ductusKey("arabic", "ذ")];
const ARABIC_RAA = DUCTUS[ductusKey("arabic", "ر")];
const PERSIAN_RA = DUCTUS[ductusKey("perso-arabic", "ر")];
const ARABIC_ZAY = DUCTUS[ductusKey("arabic", "ز")];
const PERSIAN_ZAY = DUCTUS[ductusKey("perso-arabic", "ز")];
const URDU_ZE = DUCTUS[ductusKey("urdu-nastaliq", "ز")];
const ARABIC_SEEN = DUCTUS[ductusKey("arabic", "س")];
const ARABIC_SHIIN = DUCTUS[ductusKey("arabic", "ش")];
const ARABIC_SAAD = DUCTUS[ductusKey("arabic", "ص")];
const ARABIC_DAAD = DUCTUS[ductusKey("arabic", "ض")];
const ARABIC_TAH = DUCTUS[ductusKey("arabic", "ط")];
const PERSIAN_TAH = DUCTUS[ductusKey("perso-arabic", "ط")];
const URDU_TOE = DUCTUS[ductusKey("urdu-nastaliq", "ط")];
const ARABIC_ZAH = DUCTUS[ductusKey("arabic", "ظ")];
const PERSIAN_ZAH = DUCTUS[ductusKey("perso-arabic", "ظ")];
const URDU_ZOE = DUCTUS[ductusKey("urdu-nastaliq", "ظ")];
const ARABIC_AYN = DUCTUS[ductusKey("arabic", "ع")];
const ARABIC_GHAYN = DUCTUS[ductusKey("arabic", "غ")];
const ARABIC_FAA = DUCTUS[ductusKey("arabic", "ف")];
const PERSIAN_FEH = DUCTUS[ductusKey("perso-arabic", "ف")];
const URDU_FE = DUCTUS[ductusKey("urdu-nastaliq", "ف")];
const ARABIC_QAF = DUCTUS[ductusKey("arabic", "ق")];
const PERSIAN_QAF = DUCTUS[ductusKey("perso-arabic", "ق")];
const URDU_QAF = DUCTUS[ductusKey("urdu-nastaliq", "ق")];
const ARABIC_KAF = DUCTUS[ductusKey("arabic", "ك")];
const ARABIC_LAM = DUCTUS[ductusKey("arabic", "ل")];
const ARABIC_MEEM = DUCTUS[ductusKey("arabic", "م")];
const ARABIC_NOON = DUCTUS[ductusKey("arabic", "ن")];
const ARABIC_HEH = DUCTUS[ductusKey("arabic", "ه")];
const ARABIC_WAW = DUCTUS[ductusKey("arabic", "و")];
const ARABIC_YAA = DUCTUS[ductusKey("arabic", "ي")];
const ARABIC_HAMZA = DUCTUS[ductusKey("arabic", "ء")];
const ARABIC_LAM_ALIF = DUCTUS[ductusKey("arabic", "لا")];
const URDU_ALEF = DUCTUS[ductusKey("urdu-nastaliq", "ا")];
const URDU_BEH = DUCTUS[ductusKey("urdu-nastaliq", "ب")];
const URDU_PEH = DUCTUS[ductusKey("urdu-nastaliq", "پ")];
const URDU_TE = DUCTUS[ductusKey("urdu-nastaliq", "ت")];
const URDU_TTE = DUCTUS[ductusKey("urdu-nastaliq", "ٹ")];
const PERSIAN_CHE = DUCTUS[ductusKey("perso-arabic", "چ")];
const PERSIAN_SHIN = DUCTUS[ductusKey("perso-arabic", "ش")];
const URDU_JIM = DUCTUS[ductusKey("urdu-nastaliq", "ج")];
const URDU_CHE = DUCTUS[ductusKey("urdu-nastaliq", "چ")];
const URDU_DAL = DUCTUS[ductusKey("urdu-nastaliq", "د")];
const URDU_RE = DUCTUS[ductusKey("urdu-nastaliq", "ر")];
const URDU_RRE = DUCTUS[ductusKey("urdu-nastaliq", "ڑ")];
const URDU_WAW = DUCTUS[ductusKey("urdu-nastaliq", "و")];
const URDU_SIN = DUCTUS[ductusKey("urdu-nastaliq", "س")];
const URDU_SHIN = DUCTUS[ductusKey("urdu-nastaliq", "ش")];
const URDU_KAF = DUCTUS[ductusKey("urdu-nastaliq", "ک")];
const URDU_GAF = DUCTUS[ductusKey("urdu-nastaliq", "گ")];
const PERSIAN_KAF = DUCTUS[ductusKey("perso-arabic", "ک")];
const PERSIAN_GAF = DUCTUS[ductusKey("perso-arabic", "گ")];
const URDU_LAM = DUCTUS[ductusKey("urdu-nastaliq", "ل")];
const URDU_MIM = DUCTUS[ductusKey("urdu-nastaliq", "م")];
const URDU_NUN = DUCTUS[ductusKey("urdu-nastaliq", "ن")];
const URDU_GHUNNA = DUCTUS[ductusKey("urdu-nastaliq", "ں")];
const URDU_HE = DUCTUS[ductusKey("urdu-nastaliq", "ہ")];
const URDU_DO_CHASHMI_HE = DUCTUS[ductusKey("urdu-nastaliq", "ھ")];
const URDU_YE = DUCTUS[ductusKey("urdu-nastaliq", "ی")];
const PERSIAN_YEH = DUCTUS[ductusKey("perso-arabic", "ی")];
const URDU_BARI_YE = DUCTUS[ductusKey("urdu-nastaliq", "ے")];

const OWNER_SCRIPTS = new Set(["arabic", "perso-arabic", "urdu-nastaliq"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, {});

  beforeAll(() => {
    expect(verifiedLetterFont("ی", PERSIAN_YEH.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("و", DUCTUS["و"].source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ه", DUCTUS["ه"].source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ا", URDU_ALEF.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ا", ARABIC_ALEF.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ب", ARABIC_BAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ت", ARABIC_TAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ج", ARABIC_JEEM.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ح", ARABIC_HAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("خ", ARABIC_KHAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("د", ARABIC_DAAL.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ر", ARABIC_RAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ز", ARABIC_ZAY.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ط", ARABIC_TAH.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ظ", ARABIC_ZAH.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("غ", ARABIC_GHAYN.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ف", ARABIC_FAA.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(verifiedLetterFont("ق", ARABIC_QAF.source.url)).toBe(
      "_fonts/NotoNaskhArabic-Static.ttf",
    );
    expect(
      verifiedLetterFont("و", "https://example.invalid/wrong-source"),
    ).toBeUndefined();
  });

  it("marks Arabic complete after shape and composition closure", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    expect(arabic.complete).toBe(true);
    expect(arabic.letters).toHaveLength(31);
    expect(
      arabic.letters.every((letter) => letter.strokeOrderSource !== undefined),
    ).toBe(true);
    expect(arabic.ligatures?.map((ligature) => ligature.sequence)).toEqual([
      "لا",
    ]);
  });

  it("keeps taa marbuta word-final and body-first", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    const ending = arabic.letters.find((letter) => letter.glyph === "ة")!;
    expect(ending.forms).toEqual({ isolated: "ة", final: "ـة" });
    expect(ending.penLifts).toBe(2);
    expect(ending.strokeOrderSource!.url).toBe(
      "https://alarabiyah.sakura.ne.jp/arabic/alphabets/naskh/taabarbuutah/",
    );
    const ductus = DUCTUS[ductusKey("arabic", "ة")];
    expect(ductus.strokes).toHaveLength(3);
    expect(penPath(ductus.strokes[0]).at(-1)).toEqual(
      penPath(ductus.strokes[0])[0],
    );
    expect(penPath(ductus.strokes[1])[0].x).toBeLessThan(
      penPath(ductus.strokes[2])[0].x,
    );
  });

  it("keeps alif maqsura word-final, dotless, and distinct from Yaa", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    const ending = arabic.letters.find((letter) => letter.glyph === "ى")!;
    expect(ending.forms).toEqual({ isolated: "ى", final: "ـى" });
    expect(ending.penLifts).toBe(0);
    expect(ending.strokeOrderSource!.url).toBe(
      "https://alarabiyah.sakura.ne.jp/arabic/alphabets/naskh/alifmaqsuurah/",
    );
    const maqsura = DUCTUS[ductusKey("arabic", "ى")];
    const yaa = DUCTUS[ductusKey("arabic", "ي")];
    expect(maqsura.strokes).toHaveLength(1);
    expect(penPath(maqsura.strokes[0])).toEqual(penPath(yaa.strokes[0]));
    expect(yaa.strokes).toHaveLength(3);
  });

  it("keeps lam-alif as an obligatory two-letter ligature", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    expect(arabic.letters).toHaveLength(31);
    expect(arabic.letters.some((letter) => letter.glyph === "لا")).toBe(false);
    expect(arabic.ligatures).toHaveLength(1);
    const ligature = arabic.ligatures![0];
    expect(ligature.sequence).toBe("لا");
    expect([...ligature.sequence]).toEqual(["ل", "ا"]);
    expect(ligature.displayGlyph).toBe("ﻻ");
    expect(ligature.forms).toEqual({ isolated: "لا", final: "ـلا" });
    expect(ligature.penLifts).toBe(1);
    expect(ligature.strokeOrderSource!.url).toBe(
      "https://alarabiyah.sakura.ne.jp/arabic/alphabets/naskh/laamalif/",
    );
    expect(ARABIC_LAM_ALIF.glyph).toBe("ﻻ");
    expect(ARABIC_LAM_ALIF.strokes).toHaveLength(2);
  });

  it("models seated Hamza as sourced carrier composition, not duplicate letters", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    const hamzaMarks = arabic.marks!.filter((mark) =>
      ["ٔ", "ٕ"].includes(mark.mark),
    );
    expect(hamzaMarks.map((mark) => mark.mark)).toEqual(["ٔ", "ٕ"]);
    expect(
      hamzaMarks.flatMap((mark) =>
        mark.examples!.map((example) => example.combined),
      ),
    ).toEqual(["أ", "ؤ", "ئ", "إ"]);
    for (const mark of hamzaMarks) {
      expect(mark.compositionOrder).toHaveLength(2);
      expect(mark.compositionSource!.url).toBe(
        "https://alarabiyah.sakura.ne.jp/arabic/alphabets/naskh/hamzah/",
      );
      expect(mark.compositionSource!.citation).toMatch(
        /Arabic Language Learning Notes.*Basic Naskh.*Hamza.*2022-04-09.*2026-08-23/i,
      );
      expect(mark.compositionSource!.variation).toMatch(
        /carrier.*first.*Hamza.*after|Hamza below.*alif.*carrier.*first.*Hamza.*after/i,
      );
      for (const example of mark.examples!) {
        expect(example.combined.normalize("NFD")).toBe(
          `${example.base}${mark.mark}`,
        );
        expect(
          arabic.letters.some((letter) => letter.glyph === example.combined),
        ).toBe(false);
        expect(DUCTUS[ductusKey("arabic", example.base)]).toBeDefined();
      }
    }
    expect(DUCTUS[ductusKey("arabic", "ء")]).toBeDefined();
  });

  it("models alif maddah as sourced alef-plus-mark composition", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    const maddah = arabic.marks!.find((mark) => mark.mark === "ٓ")!;
    expect(maddah.role).toBe("diacritic");
    expect(maddah.attachesAs).toMatch(/maddah above.*alif carrier/i);
    expect(maddah.example).toEqual({ base: "ا", combined: "آ", sound: "ʾā" });
    expect(maddah.example!.combined.normalize("NFD")).toBe(
      `${maddah.example!.base}${maddah.mark}`,
    );
    expect(maddah.compositionOrder).toEqual([
      "write the source-verified alif carrier downward according to its normal positional rule",
      "add maddah above as a short horizontal wave",
    ]);
    expect(maddah.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-9/",
    );
    expect(maddah.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*§9\.2.*U\+0622.*U\+0627.*U\+0653/i,
    );
    expect(maddah.compositionSource?.variation).toMatch(
      /special harakat.*above alef.*\/ʾaa\/.*canonically decomposes.*carrier.*wave-shaped mark.*not a universal handwriting sequence.*learner convention/i,
    );
    expect(arabic.letters.some((letter) => letter.glyph === "آ")).toBe(false);
    expect(DUCTUS[ductusKey("arabic", "ا")]).toBeDefined();
  });

  it("keeps Persian and Urdu maddah on their independently sourced alif carriers", () => {
    const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!;
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!;
    const persianMaddah = persian.marks!.find((mark) => mark.mark === "ٓ")!;
    const urduMaddah = urdu.marks!.find((mark) => mark.mark === "ٓ")!;
    for (const mark of [persianMaddah, urduMaddah]) {
      expect(mark.example!.combined.normalize("NFD")).toBe(
        `${mark.example!.base}${mark.mark}`,
      );
      expect(mark.compositionOrder).toHaveLength(2);
      expect(mark.compositionSource?.url).toBe(
        "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-9/",
      );
      expect(mark.compositionSource?.variation).toMatch(
        /separately source-verified.*alif.*wave-shaped mark.*not a universal handwriting sequence.*learner convention/i,
      );
    }
    expect(DUCTUS["ا"]).toBeDefined();
    expect(DUCTUS[ductusKey("urdu-nastaliq", "ا")]).toBeDefined();
    expect(persianMaddah.sound).toMatch(/â/);
    expect(urduMaddah.sound).toMatch(/long aa/i);
    expect(urduMaddah.compositionSource?.variation).toMatch(
      /Nastaliq.*Naskh fallback/i,
    );
  });

  it("Persian ا is one downward pen-down run", () => {
    const alef = DUCTUS["ا"];
    expect(penLifts(alef)).toBe(0);
    expect(alef.strokes).toHaveLength(1);
    expect(alef.strokes[0].segments).toHaveLength(1);
    const path = penPath(alef.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Urdu independent ا is its own one-stroke downward ductus", () => {
    expect(URDU_ALEF.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_ALEF)).toBe(0);
    expect(URDU_ALEF.strokes).toHaveLength(1);
    expect(URDU_ALEF.strokes[0].segments).toHaveLength(1);
    const path = penPath(URDU_ALEF.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Urdu independent ب draws its bowl before the single lower dot", () => {
    expect(URDU_BEH.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_BEH)).toBe(1);
    expect(URDU_BEH.strokes).toHaveLength(2);
    expect(URDU_BEH.strokes.map((stroke) => stroke.segments[0].label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the single dot below",
    ]);
    const bowl = URDU_BEH.strokes[0].segments[0].path;
    const dot = URDU_BEH.strokes[1].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.max(...dot.map((point) => point.y))).toBeLessThan(
      Math.min(...bowl.map((point) => point.y)),
    );
    expect(URDU_BEH.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/be-kaf-and-short-vowels/",
    );
    expect(URDU_BEH.source.citation).toMatch(
      /Zer o Zabar.*independent ب.*Be instructions/i,
    );
    expect(
      new Set([
        ARABIC_BAA.source.url,
        DUCTUS["ب"].source.url,
        URDU_BEH.source.url,
      ]).size,
    ).toBe(3);
  });

  it("Urdu independent پ draws its bowl before the three-dot triangle", () => {
    expect(URDU_PEH.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_PEH)).toBe(3);
    expect(URDU_PEH.strokes).toHaveLength(4);
    expect(URDU_PEH.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1, 1,
    ]);
    expect(URDU_PEH.strokes.map((stroke) => stroke.segments[0].label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the lower-left dot nearer the main line",
      "after another lift, place the lower-right dot nearer the main line",
      "after a third lift, place the lower-center dot",
    ]);
    const [left, right, center] = URDU_PEH.strokes
      .slice(1)
      .map((stroke) => stroke.segments[0].path);
    expect(left[0].x).toBeLessThan(right[0].x);
    expect(center[0].y).toBeLessThan(left[0].y);
    expect(center[0].y).toBeLessThan(right[0].y);
    expect(URDU_PEH.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(URDU_PEH.source.citation).toMatch(
      /Zer o Zabar.*independent پ.*Pe instructions/i,
    );
    expect(DUCTUS["پ"].source.url).not.toBe(URDU_PEH.source.url);
  });

  it("Urdu independent ت draws its bowl before two upper dots", () => {
    expect(URDU_TE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_TE)).toBe(2);
    expect(URDU_TE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    expect(URDU_TE.strokes.map((stroke) => stroke.segments[0].label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the left dot above the main line",
      "after another lift, place the right dot beside it",
    ]);
    const left = URDU_TE.strokes[1].segments[0].path;
    const right = URDU_TE.strokes[2].segments[0].path;
    expect(left[0].x).toBeLessThan(right[0].x);
    expect(URDU_TE.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(URDU_TE.source.url).not.toBe(ARABIC_TAA.source.url);
    expect(URDU_TE.source.url).not.toBe(DUCTUS["ت"].source.url);
  });

  it("Urdu independent ٹ draws its bowl before the lifted retroflex mark", () => {
    expect(URDU_TTE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_TTE)).toBe(1);
    expect(URDU_TTE.strokes).toHaveLength(2);
    expect(URDU_TTE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1,
    ]);
    expect(URDU_TTE.strokes.map((stroke) => stroke.segments[0].label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, draw the small retroflex mark downward, back upward, and down again to close its loop",
    ]);
    const bowl = URDU_TTE.strokes[0].segments[0].path;
    const mark = URDU_TTE.strokes[1].segments[0].path;
    expect(Math.min(...mark.map((point) => point.y))).toBeGreaterThan(
      Math.max(...bowl.map((point) => point.y)) - 50,
    );
    expect(URDU_TTE.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    expect(URDU_TTE.source.citation).toMatch(
      /Zer o Zabar.*independent ٹ.*Ṭe instructions/i,
    );
  });

  it("Urdu independent ج places its dot, then joins the pointed head to the bowl", () => {
    expect(URDU_JIM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_JIM)).toBe(1);
    expect(URDU_JIM.strokes).toHaveLength(2);
    expect(URDU_JIM.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 2,
    ]);
    expect(URDU_JIM.strokes[0].segments[0].label).toBe("place the dot below");
    const head = URDU_JIM.strokes[1].segments[0].path;
    const bowl = URDU_JIM.strokes[1].segments[1].path;
    expect(head[0].x).toBeGreaterThan(
      Math.min(...head.map((point) => point.x)),
    );
    expect(bowl[0]).toEqual(head.at(-1));
    expect(bowl[0].y).toBeGreaterThan(bowl.at(-1)!.y);
  });

  it("Urdu independent چ draws its body before the three-dot triangle", () => {
    expect(URDU_CHE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_CHE)).toBe(3);
    expect(URDU_CHE.strokes).toHaveLength(4);
    expect(URDU_CHE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1, 1,
    ]);
    expect(URDU_CHE.strokes.map((stroke) => stroke.segments[0].label)).toEqual([
      "sweep left through the pointed hooked head",
      "after one lift, place the lower-left dot",
      "after another lift, place the lower-right dot",
      "after a third lift, place the lower-center dot",
    ]);
    expect(URDU_CHE.strokes[0].segments[1].path[0]).toEqual(
      URDU_CHE.strokes[0].segments[0].path.at(-1),
    );
    const [left, right, center] = URDU_CHE.strokes
      .slice(1)
      .map((stroke) => stroke.segments[0].path);
    expect(left[0].x).toBeLessThan(right[0].x);
    expect(center[0].y).toBeLessThan(left[0].y);
    expect(center[0].y).toBeLessThan(right[0].y);
  });

  it("Persian and Urdu چ share geometry but retain script-owned sources", () => {
    expect(PERSIAN_CHE.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_CHE)).toBe(3);
    expect(PERSIAN_CHE.strokes).toHaveLength(4);
    expect(PERSIAN_CHE.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1, 1, 1],
    );
    expect(
      PERSIAN_CHE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    ).toEqual(
      URDU_CHE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    );
    expect(PERSIAN_CHE.source.url).toContain(
      "laits.utexas.edu/persian_grammar/video",
    );
    expect(PERSIAN_CHE.source.url).not.toBe(URDU_CHE.source.url);
  });

  it("Urdu independent د folds into its baseline without lifting", () => {
    expect(URDU_DAL.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_DAL)).toBe(0);
    expect(URDU_DAL.strokes).toHaveLength(1);
    expect(URDU_DAL.strokes[0].segments).toHaveLength(2);
    const shoulder = URDU_DAL.strokes[0].segments[0].path;
    const baseline = URDU_DAL.strokes[0].segments[1].path;
    expect(baseline[0]).toEqual(shoulder.at(-1));
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
    expect(
      Math.min(...baseline.map((point) => point.y)),
    ).toBeGreaterThanOrEqual(0);
  });

  it("Urdu independent ر joins its downward line directly to the leftward curve", () => {
    expect(URDU_RE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_RE)).toBe(0);
    expect(URDU_RE.strokes).toHaveLength(1);
    expect(URDU_RE.strokes[0].segments).toHaveLength(2);
    const down = URDU_RE.strokes[0].segments[0].path;
    const curve = URDU_RE.strokes[0].segments[1].path;
    expect(down[0].y).toBeGreaterThan(down.at(-1)!.y);
    expect(curve[0]).toEqual(down.at(-1));
    expect(curve[0].x).toBeGreaterThan(curve.at(-1)!.x);
  });

  it("Urdu independent ڑ adds its retroflex mark after the re-series body", () => {
    expect(URDU_RRE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_RRE)).toBe(1);
    expect(URDU_RRE.strokes.map((stroke) =>
      stroke.segments.map((segment) => segment.label),
    )).toEqual([
      [
        "draw the independent re-series body downward",
        "continue curving to the left",
      ],
      [
        "after one lift, draw the small retroflex mark downward, back upward, and down again to close its loop",
      ],
    ]);
    expect(URDU_RRE.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
  });

  it("Urdu independent و joins its looped head directly to the leftward tail", () => {
    expect(URDU_WAW.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_WAW)).toBe(0);
    expect(URDU_WAW.strokes).toHaveLength(1);
    expect(URDU_WAW.strokes[0].segments).toHaveLength(2);
    const head = URDU_WAW.strokes[0].segments[0].path;
    const tail = URDU_WAW.strokes[0].segments[1].path;
    expect(tail[0]).toEqual(head.at(-1));
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("Urdu independent س joins its three close teeth directly to the final bowl", () => {
    expect(URDU_SIN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_SIN)).toBe(0);
    expect(URDU_SIN.strokes).toHaveLength(1);
    expect(URDU_SIN.strokes[0].segments).toHaveLength(2);
    const teeth = URDU_SIN.strokes[0].segments[0].path;
    const bowl = URDU_SIN.strokes[0].segments[1].path;
    expect(teeth[0].x).toBeGreaterThan(teeth.at(-1)!.x);
    expect(bowl[0]).toEqual(teeth.at(-1));
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Urdu independent ش writes its س body before three separately lifted dots", () => {
    expect(URDU_SHIN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_SHIN)).toBe(3);
    expect(URDU_SHIN.strokes).toHaveLength(4);
    expect(URDU_SHIN.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1, 1,
    ]);
    const teeth = URDU_SHIN.strokes[0].segments[0].path;
    const bowl = URDU_SHIN.strokes[0].segments[1].path;
    expect(bowl[0]).toEqual(teeth.at(-1));
    const [lowerLeft, lowerRight, upper] = URDU_SHIN.strokes
      .slice(1)
      .map((stroke) => stroke.segments[0].path);
    expect(lowerLeft[0].x).toBeLessThan(lowerRight[0].x);
    expect(upper[0].y).toBeGreaterThan(lowerLeft[0].y);
    expect(upper[0].y).toBeGreaterThan(lowerRight[0].y);
  });

  it("Urdu independent ک writes its main-line body before the separately lifted slash", () => {
    expect(URDU_KAF.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_KAF)).toBe(1);
    expect(URDU_KAF.strokes).toHaveLength(2);
    expect(URDU_KAF.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const stem = URDU_KAF.strokes[0].segments[0].path;
    const bowl = URDU_KAF.strokes[0].segments[1].path;
    const slash = URDU_KAF.strokes[1].segments[0].path;
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(bowl[0]).toEqual(stem.at(-1));
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(slash[0].x).toBeGreaterThan(slash.at(-1)!.x);
    expect(slash[0].y).toBeGreaterThan(slash.at(-1)!.y);
  });

  it("Urdu independent گ adds a shorter floating slash above the kāf construction", () => {
    expect(URDU_GAF.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_GAF)).toBe(2);
    expect(URDU_GAF.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1,
    ]);
    expect(URDU_GAF.strokes[2].segments[0].label).toMatch(
      /shorter floating slash above/i,
    );
    const longSlash = URDU_GAF.strokes[1].segments[0].path;
    const shortSlash = URDU_GAF.strokes[2].segments[0].path;
    expect(shortSlash[0].y).toBeGreaterThan(longSlash[0].y);
    expect(URDU_GAF.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
  });

  it("Persian independent ک keeps its scoped body-first two-run source", () => {
    expect(PERSIAN_KAF.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_KAF)).toBe(1);
    expect(PERSIAN_KAF.strokes).toHaveLength(2);
    expect(PERSIAN_KAF.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    expect(
      PERSIAN_KAF.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    ).toEqual(
      URDU_KAF.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    );
    expect(PERSIAN_KAF.source.citation).toMatch(
      /Persian Online.*ک.*02:19–02:23/i,
    );
    expect(PERSIAN_KAF.source.url).not.toBe(URDU_KAF.source.url);
  });

  it("Persian independent گ keeps its scoped body-first three-run source", () => {
    expect(PERSIAN_GAF.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_GAF)).toBe(2);
    expect(PERSIAN_GAF.strokes).toHaveLength(3);
    expect(PERSIAN_GAF.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1, 1],
    );
    expect(
      PERSIAN_GAF.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    ).toEqual(
      URDU_GAF.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    );
    expect(PERSIAN_GAF.source.citation).toMatch(
      /Persian Online.*گ.*02:24–02:28/i,
    );
    expect(PERSIAN_GAF.source.url).not.toBe(URDU_GAF.source.url);
  });

  it("Persian and Urdu independent ز keep scoped body-first sources", () => {
    for (const zay of [PERSIAN_ZAY, URDU_ZE]) {
      expect(penLifts(zay)).toBe(1);
      expect(zay.strokes.map((stroke) => stroke.segments.length)).toEqual([
        2, 1,
      ]);
      expect(
        zay.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.path),
        ),
      ).toEqual(
        ARABIC_ZAY.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.path),
        ),
      );
    }
    expect(PERSIAN_ZAY.source.citation).toMatch(
      /Persian Online.*ز.*01:13–01:16/i,
    );
    expect(URDU_ZE.source.citation).toMatch(
      /Zer o Zabar.*independent ز.*Ze instructions/i,
    );
    expect(PERSIAN_ZAY.source.url).not.toBe(ARABIC_ZAY.source.url);
    expect(URDU_ZE.source.url).not.toBe(ARABIC_ZAY.source.url);
    expect(PERSIAN_ZAY.source.url).not.toBe(URDU_ZE.source.url);
  });

  it("Urdu independent ل descends through its below-baseline bowl without lifting", () => {
    expect(URDU_LAM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_LAM)).toBe(0);
    expect(URDU_LAM.strokes).toHaveLength(1);
    expect(URDU_LAM.strokes[0].segments).toHaveLength(2);
    const upright = URDU_LAM.strokes[0].segments[0].path;
    const bowl = URDU_LAM.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(bowl[0]).toEqual(upright.at(-1));
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(
      Math.min(...bowl.map((point) => point.y)),
    );
  });

  it("Urdu independent م joins its round head to a below-baseline tail", () => {
    expect(URDU_MIM.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_MIM)).toBe(0);
    expect(URDU_MIM.strokes).toHaveLength(1);
    expect(URDU_MIM.strokes[0].segments).toHaveLength(2);
    const head = URDU_MIM.strokes[0].segments[0].path;
    const tail = URDU_MIM.strokes[0].segments[1].path;
    expect(tail[0]).toEqual(head.at(-1));
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(
      head[0].y,
    );
    expect(Math.min(...tail.map((point) => point.y))).toBeLessThan(0);
    expect(tail.at(-1)!.y).toBeLessThan(tail[0].y);
  });

  it("Urdu independent ن draws its below-baseline bowl before the lifted dot", () => {
    expect(URDU_NUN.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_NUN)).toBe(1);
    expect(URDU_NUN.strokes).toHaveLength(2);
    expect(URDU_NUN.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1,
    ]);
    const bowl = URDU_NUN.strokes[0].segments[0].path;
    const dot = URDU_NUN.strokes[1].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(0);
  });

  it("Urdu independent ں reuses ن's below-baseline bowl without a dot or lift", () => {
    expect(URDU_GHUNNA.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_GHUNNA)).toBe(0);
    expect(URDU_GHUNNA.strokes).toHaveLength(1);
    expect(URDU_GHUNNA.strokes[0].segments).toHaveLength(1);
    const bowl = URDU_GHUNNA.strokes[0].segments[0].path;
    expect(bowl).toEqual(URDU_NUN.strokes[0].segments[0].path);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
  });

  it("Urdu independent ہ closes its counterclockwise teardrop without lifting", () => {
    expect(URDU_HE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_HE)).toBe(0);
    expect(URDU_HE.strokes).toHaveLength(1);
    expect(URDU_HE.strokes[0].segments).toHaveLength(1);
    const loop = URDU_HE.strokes[0].segments[0].path;
    expect(loop[1].x).toBeLessThan(loop[0].x);
    expect(loop[1].y).toBeLessThan(loop[0].y);
    expect(Math.min(...loop.map((point) => point.y))).toBeLessThan(100);
    expect(Math.max(...loop.slice(9).map((point) => point.x))).toBeGreaterThan(
      loop[0].x,
    );
    expect(loop.at(-1)!.y).toBeGreaterThan(loop[0].y);
  });

  it("Urdu independent ھ joins both eyes and the low finish without lifting", () => {
    expect(URDU_DO_CHASHMI_HE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_DO_CHASHMI_HE)).toBe(0);
    expect(URDU_DO_CHASHMI_HE.strokes).toHaveLength(1);
    expect(URDU_DO_CHASHMI_HE.strokes[0].segments).toHaveLength(4);
    const [rightEye, baseline, leftEye, finish] =
      URDU_DO_CHASHMI_HE.strokes[0].segments;
    expect(rightEye.path.at(-1)).toEqual(baseline.path[0]);
    expect(baseline.path.at(-1)).toEqual(leftEye.path[0]);
    expect(leftEye.path.at(-1)).toEqual(finish.path[0]);
    expect(Math.max(...rightEye.path.map((point) => point.x))).toBeGreaterThan(
      Math.max(...leftEye.path.map((point) => point.x)),
    );
    expect(baseline.path.at(-1)!.x).toBeLessThan(baseline.path[0].x);
    expect(leftEye.path[1].y).toBeGreaterThan(leftEye.path[0].y);
    expect(finish.path.at(-1)!.x).toBeLessThan(finish.path[0].x);
  });

  it("Urdu independent ی keeps its dotless S and bowl in one unbroken stroke", () => {
    expect(URDU_YE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_YE)).toBe(0);
    expect(URDU_YE.strokes).toHaveLength(1);
    expect(URDU_YE.strokes[0].segments).toHaveLength(2);
    const upper = URDU_YE.strokes[0].segments[0].path;
    const bowl = URDU_YE.strokes[0].segments[1].path;
    expect(upper.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...upper.map((point) => point.x))).toBeLessThan(upper[0].x);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(-200);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
  });

  it("Persian independent ی keeps the same Noto path with separate provenance", () => {
    expect(PERSIAN_YEH.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_YEH)).toBe(0);
    expect(PERSIAN_YEH.strokes).toHaveLength(1);
    expect(PERSIAN_YEH.strokes[0].segments).toHaveLength(2);
    expect(
      PERSIAN_YEH.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    ).toEqual(
      URDU_YE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.path),
      ),
    );
    expect(PERSIAN_YEH.source.url).toContain(
      "laits.utexas.edu/persian_grammar/video",
    );
    expect(PERSIAN_YEH.source.url).not.toBe(URDU_YE.source.url);
  });

  it("Urdu independent ے folds its broad bowl back underneath without lifting", () => {
    expect(URDU_BARI_YE.script).toBe("urdu-nastaliq");
    expect(penLifts(URDU_BARI_YE)).toBe(0);
    expect(URDU_BARI_YE.strokes).toHaveLength(1);
    expect(URDU_BARI_YE.strokes[0].segments).toHaveLength(3);
    const upper = URDU_BARI_YE.strokes[0].segments[0].path;
    const curl = URDU_BARI_YE.strokes[0].segments[1].path;
    const lower = URDU_BARI_YE.strokes[0].segments[2].path;
    expect(upper.at(-1)).toEqual(curl[0]);
    expect(curl.at(-1)).toEqual(lower[0]);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(upper.at(-1)!.x).toBeLessThan(upper[0].x);
    expect(Math.min(...curl.map((point) => point.x))).toBeLessThan(
      upper.at(-1)!.x,
    );
    expect(lower.at(-1)!.x).toBeGreaterThan(lower[0].x);
  });

  it("Arabic independent ا descends in one unbroken stroke", () => {
    expect(penLifts(ARABIC_ALEF)).toBe(0);
    expect(ARABIC_ALEF.strokes).toHaveLength(1);
    expect(ARABIC_ALEF.strokes[0].segments).toHaveLength(1);
    const path = penPath(ARABIC_ALEF.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("Arabic independent ب sweeps right-to-left, then lifts once for the dot", () => {
    expect(penLifts(ARABIC_BAA)).toBe(1);
    expect(ARABIC_BAA.strokes).toHaveLength(2);
    expect(ARABIC_BAA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1,
    ]);
    const bowl = penPath(ARABIC_BAA.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Arabic independent ت uses the shared bowl, then two separately lifted dots", () => {
    expect(penLifts(ARABIC_TAA)).toBe(2);
    expect(ARABIC_TAA.strokes).toHaveLength(3);
    expect(ARABIC_TAA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const bowl = penPath(ARABIC_TAA.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(ARABIC_TAA.strokes[1].segments[0].path[0].x).toBeLessThan(
      ARABIC_TAA.strokes[2].segments[0].path[0].x,
    );
  });

  it("Arabic independent ث uses the shared bowl, then three separately lifted dots", () => {
    expect(penLifts(ARABIC_THAA)).toBe(3);
    expect(ARABIC_THAA.strokes).toHaveLength(4);
    expect(ARABIC_THAA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 1, 1],
    );
    const bowl = penPath(ARABIC_THAA.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(ARABIC_THAA.strokes[3].segments[0].path[0].y).toBeGreaterThan(
      ARABIC_THAA.strokes[1].segments[0].path[0].y,
    );
  });

  it("Arabic independent ج draws its body first, then lifts once for the dot", () => {
    expect(penLifts(ARABIC_JEEM)).toBe(1);
    expect(ARABIC_JEEM.strokes).toHaveLength(2);
    expect(ARABIC_JEEM.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    const head = ARABIC_JEEM.strokes[0].segments[0].path;
    const bowl = ARABIC_JEEM.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(
      Math.min(...head.map((point) => point.y)),
    );
  });

  it("Arabic independent ح draws a short stem, then lifts once for its dotless bowl", () => {
    expect(penLifts(ARABIC_HAA)).toBe(1);
    expect(ARABIC_HAA.strokes).toHaveLength(2);
    expect(ARABIC_HAA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 2,
    ]);
    const stem = ARABIC_HAA.strokes[0].segments[0].path;
    const head = ARABIC_HAA.strokes[1].segments[0].path;
    const bowl = ARABIC_HAA.strokes[1].segments[1].path;
    expect(stem[0].y).toBeGreaterThan(stem.at(-1)!.y);
    expect(head[0]).toEqual(stem[0]);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(
      Math.min(...head.map((point) => point.y)),
    );
  });

  it("Persian and Urdu independent ح keep separate sources for one body-first run", () => {
    for (const letter of [PERSIAN_HAH, URDU_BARI_HE]) {
      expect(penLifts(letter)).toBe(0);
      expect(letter.strokes).toHaveLength(1);
      expect(letter.strokes[0].segments).toHaveLength(2);
      expect(letter.strokes[0].segments[0].path.at(-1)).toEqual(
        letter.strokes[0].segments[1].path[0],
      );
    }
    expect(PERSIAN_HAH.source.url).not.toBe(URDU_BARI_HE.source.url);
    expect(PERSIAN_HAH.source.citation).toMatch(
      /Persian Online.*ح.*00:42.?00:46/i,
    );
    expect(URDU_BARI_HE.source.citation).toMatch(/Zer o Zabar.*baṛī he.*ح/i);
  });

  it("Arabic independent خ draws its body first, then lifts once for the upper dot", () => {
    expect(penLifts(ARABIC_KHAA)).toBe(1);
    expect(ARABIC_KHAA.strokes).toHaveLength(2);
    expect(ARABIC_KHAA.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    const head = ARABIC_KHAA.strokes[0].segments[0].path;
    const bowl = ARABIC_KHAA.strokes[0].segments[1].path;
    const dot = ARABIC_KHAA.strokes[1].segments[0].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...head.map((point) => point.y)),
    );
  });

  it("Arabic independent د descends and turns left without lifting", () => {
    expect(penLifts(ARABIC_DAAL)).toBe(0);
    expect(ARABIC_DAAL.strokes).toHaveLength(1);
    expect(ARABIC_DAAL.strokes[0].segments).toHaveLength(2);
    const shoulder = ARABIC_DAAL.strokes[0].segments[0].path;
    const baseline = ARABIC_DAAL.strokes[0].segments[1].path;
    expect(shoulder[0].y).toBeGreaterThan(shoulder.at(-1)!.y);
    expect(shoulder[0].x).toBeLessThan(shoulder.at(-1)!.x);
    expect(shoulder.at(-1)).toEqual(baseline[0]);
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
  });

  it("Arabic independent ذ preserves the Daal body before placing its upper dot", () => {
    expect(penLifts(ARABIC_DHAAL)).toBe(1);
    expect(ARABIC_DHAAL.strokes).toHaveLength(2);
    expect(ARABIC_DHAAL.strokes[0]).toEqual(ARABIC_DAAL.strokes[0]);
    const body = ARABIC_DHAAL.strokes[0].segments.flatMap(
      (segment) => segment.path,
    );
    const dot = ARABIC_DHAAL.strokes[1].segments[0].path;
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...body.map((point) => point.y)),
    );
  });

  it("Arabic independent ر descends and sweeps left without lifting", () => {
    expect(penLifts(ARABIC_RAA)).toBe(0);
    expect(ARABIC_RAA.strokes).toHaveLength(1);
    expect(ARABIC_RAA.strokes[0].segments).toHaveLength(2);
    const descent = ARABIC_RAA.strokes[0].segments[0].path;
    const curve = ARABIC_RAA.strokes[0].segments[1].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(curve[0]);
    expect(curve[0].x).toBeGreaterThan(curve.at(-1)!.x);
  });

  it("Arabic independent ز preserves the Raa body before placing its upper dot", () => {
    expect(penLifts(ARABIC_ZAY)).toBe(1);
    expect(ARABIC_ZAY.strokes).toHaveLength(2);
    expect(ARABIC_ZAY.strokes[0]).toEqual(ARABIC_RAA.strokes[0]);
    const body = ARABIC_ZAY.strokes[0].segments.flatMap(
      (segment) => segment.path,
    );
    const dot = ARABIC_ZAY.strokes[1].segments[0].path;
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...body.map((point) => point.y)),
    );
  });

  it("Arabic independent س joins its three close teeth directly to the final bowl", () => {
    expect(ARABIC_SEEN.script).toBe("arabic");
    expect(penLifts(ARABIC_SEEN)).toBe(0);
    expect(ARABIC_SEEN.strokes).toHaveLength(1);
    expect(ARABIC_SEEN.strokes[0].segments).toHaveLength(2);
    const teeth = ARABIC_SEEN.strokes[0].segments[0].path;
    const bowl = ARABIC_SEEN.strokes[0].segments[1].path;
    expect(teeth[0].x).toBeGreaterThan(teeth.at(-1)!.x);
    expect(teeth.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Arabic independent ش writes its body before three separately lifted dots", () => {
    expect(ARABIC_SHIIN.script).toBe("arabic");
    expect(penLifts(ARABIC_SHIIN)).toBe(3);
    expect(ARABIC_SHIIN.strokes).toHaveLength(4);
    expect(
      ARABIC_SHIIN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1]);
    const teeth = ARABIC_SHIIN.strokes[0].segments[0].path;
    const bowl = ARABIC_SHIIN.strokes[0].segments[1].path;
    expect(teeth.at(-1)).toEqual(bowl[0]);
    const [lowerLeft, lowerRight, upper] = ARABIC_SHIIN.strokes
      .slice(1)
      .map((stroke) => stroke.segments[0].path);
    expect(lowerLeft[0].x).toBeLessThan(lowerRight[0].x);
    expect(upper[0].y).toBeGreaterThan(lowerLeft[0].y);
    expect(upper[0].y).toBeGreaterThan(lowerRight[0].y);
  });

  it("Arabic independent ص lifts once between its closed oval and trailing bowl", () => {
    expect(ARABIC_SAAD.script).toBe("arabic");
    expect(penLifts(ARABIC_SAAD)).toBe(1);
    expect(ARABIC_SAAD.strokes).toHaveLength(2);
    expect(ARABIC_SAAD.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1],
    );
    const oval = ARABIC_SAAD.strokes[0].segments[0].path;
    const shoulder = ARABIC_SAAD.strokes[0].segments[1].path;
    expect(oval[0]).toEqual(oval.at(-1));
    expect(oval.at(-1)).toEqual(shoulder[0]);
    expect(shoulder.at(-1)!.y).toBeGreaterThan(shoulder[0].y);
    const bowl = ARABIC_SAAD.strokes[1].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(bowl[0].y);
  });

  it("Arabic independent ض repeats the ص body before a second lift places its dot", () => {
    expect(ARABIC_DAAD.script).toBe("arabic");
    expect(penLifts(ARABIC_DAAD)).toBe(2);
    expect(ARABIC_DAAD.strokes).toHaveLength(3);
    expect(ARABIC_DAAD.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1, 1],
    );
    expect(ARABIC_DAAD.strokes.slice(0, 2)).toEqual(ARABIC_SAAD.strokes);
    const dot = ARABIC_DAAD.strokes[2].segments[0].path;
    const bodyTop = Math.max(
      ...ARABIC_DAAD.strokes
        .slice(0, 2)
        .flatMap((stroke) =>
          stroke.segments.flatMap((segment) =>
            segment.path.map((point) => point.y),
          ),
        ),
    );
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(bodyTop);
    expect(dot[0]).toEqual(dot.at(-1));
  });

  it("Arabic independent ط closes its oval before drawing the upright downward", () => {
    expect(ARABIC_TAH.script).toBe("arabic");
    expect(penLifts(ARABIC_TAH)).toBe(1);
    expect(ARABIC_TAH.strokes).toHaveLength(2);
    expect(ARABIC_TAH.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const loop = ARABIC_TAH.strokes[0].segments[0].path;
    const exit = ARABIC_TAH.strokes[0].segments[1].path;
    const upright = ARABIC_TAH.strokes[1].segments[0].path;
    expect(loop[0]).toEqual(loop.at(-1));
    expect(loop.at(-1)).toEqual(exit[0]);
    expect(exit.at(-1)!.x).toBeLessThan(exit[0].x);
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
  });

  it("Persian and Urdu independent ط retain scoped body-before-upright sources", () => {
    for (const letter of [PERSIAN_TAH, URDU_TOE]) {
      expect(penLifts(letter)).toBe(1);
      expect(letter.strokes).toHaveLength(2);
      expect(letter.strokes[0].segments.map((segment) => segment.path)).toEqual(
        ARABIC_TAH.strokes[0].segments.map((segment) => segment.path),
      );
      expect(letter.strokes[1].segments[0].path).toEqual(
        ARABIC_TAH.strokes[1].segments[0].path,
      );
    }
    expect(PERSIAN_TAH.script).toBe("perso-arabic");
    expect(PERSIAN_TAH.source.citation).toMatch(
      /Persian Online.*ط.*01:54–01:56/i,
    );
    expect(PERSIAN_TAH.source.variation).toMatch(
      /body-first.*counterclockwise.*closed.*baseline.*lift once.*upright.*Persian-scoped/i,
    );
    expect(URDU_TOE.script).toBe("urdu-nastaliq");
    expect(URDU_TOE.source.citation).toMatch(
      /Zer o Zabar.*independent ط.*To’e instructions/i,
    );
    expect(URDU_TOE.source.variation).toMatch(
      /body.*leftward finish.*upright.*one lift.*Noto Naskh.*Nastaliq.*Urdu-specific/i,
    );
    expect(
      new Set([
        ARABIC_TAH.source.url,
        PERSIAN_TAH.source.url,
        URDU_TOE.source.url,
      ]).size,
    ).toBe(3);
  });

  it("Arabic independent ظ repeats ط, placing its dot before the upright", () => {
    expect(ARABIC_ZAH.script).toBe("arabic");
    expect(penLifts(ARABIC_ZAH)).toBe(2);
    expect(ARABIC_ZAH.strokes).toHaveLength(3);
    expect(ARABIC_ZAH.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1,
    ]);
    expect(ARABIC_ZAH.strokes[0]).toEqual(ARABIC_TAH.strokes[0]);
    expect(ARABIC_ZAH.strokes[2].segments[0].path).toEqual(
      ARABIC_TAH.strokes[1].segments[0].path,
    );
    const dot = ARABIC_ZAH.strokes[1].segments[0].path;
    expect(dot[0]).toEqual(dot.at(-1));
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      ARABIC_ZAH.strokes[0].segments[0].path[0].y,
    );
  });

  it("Persian and Urdu independent ظ retain scoped body-upright-dot sources", () => {
    for (const letter of [PERSIAN_ZAH, URDU_ZOE]) {
      expect(penLifts(letter)).toBe(2);
      expect(letter.strokes).toHaveLength(3);
      expect(letter.strokes[0]).toEqual(ARABIC_ZAH.strokes[0]);
      expect(letter.strokes[1].segments[0].path).toEqual(
        ARABIC_ZAH.strokes[2].segments[0].path,
      );
      expect(letter.strokes[2].segments[0].path).toEqual(
        ARABIC_ZAH.strokes[1].segments[0].path,
      );
    }
    expect(PERSIAN_ZAH.script).toBe("perso-arabic");
    expect(PERSIAN_ZAH.source.citation).toMatch(
      /Persian Online.*ظ.*01:57–01:59/i,
    );
    expect(URDU_ZOE.script).toBe("urdu-nastaliq");
    expect(URDU_ZOE.source.citation).toMatch(/Zer o Zabar.*Zo’e/i);
  });

  it("Arabic independent ع joins its open head directly to the lower bowl", () => {
    expect(ARABIC_AYN.script).toBe("arabic");
    expect(penLifts(ARABIC_AYN)).toBe(0);
    expect(ARABIC_AYN.strokes).toHaveLength(1);
    expect(ARABIC_AYN.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_AYN.strokes[0].segments[0].path;
    const bowl = ARABIC_AYN.strokes[0].segments[1].path;
    expect(head[0].x).toBeGreaterThan(
      Math.min(...head.map((point) => point.x)),
    );
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.x).toBeGreaterThan(bowl[0].x);
  });

  it("Arabic independent غ repeats the complete ع body before placing its dot", () => {
    expect(ARABIC_GHAYN.script).toBe("arabic");
    expect(penLifts(ARABIC_GHAYN)).toBe(1);
    expect(ARABIC_GHAYN.strokes).toHaveLength(2);
    expect(
      ARABIC_GHAYN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1]);
    expect(ARABIC_GHAYN.strokes[0]).toEqual(ARABIC_AYN.strokes[0]);
    const dot = ARABIC_GHAYN.strokes[1].segments[0].path;
    expect(dot[0]).toEqual(dot.at(-1));
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(
        ...ARABIC_GHAYN.strokes[0].segments[0].path.map((point) => point.y),
      ),
    );
  });

  it("Arabic independent ف joins its closed head to the bowl before its dot", () => {
    expect(ARABIC_FAA.script).toBe("arabic");
    expect(penLifts(ARABIC_FAA)).toBe(1);
    expect(ARABIC_FAA.strokes).toHaveLength(2);
    expect(ARABIC_FAA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const head = ARABIC_FAA.strokes[0].segments[0].path;
    const bowl = ARABIC_FAA.strokes[0].segments[1].path;
    const dot = ARABIC_FAA.strokes[1].segments[0].path;
    expect(head[0]).toEqual(head.at(-1));
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(bowl.at(-1)!.x).toBeLessThan(bowl[0].x);
    expect(dot[0]).toEqual(dot.at(-1));
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...head.map((point) => point.y)),
    );
  });

  it("Persian and Urdu ف keep the joined head-and-bowl run with script-owned sources", () => {
    for (const ductus of [PERSIAN_FEH, URDU_FE]) {
      expect(penLifts(ductus)).toBe(1);
      expect(ductus.strokes.map((stroke) => stroke.segments.length)).toEqual([
        2, 1,
      ]);
      expect(ductus.strokes[0].segments[0].path[0]).toEqual(
        ductus.strokes[0].segments[0].path.at(-1),
      );
      expect(ductus.strokes[0].segments[0].path.at(-1)).toEqual(
        ductus.strokes[0].segments[1].path[0],
      );
    }
    expect(PERSIAN_FEH.source.citation).toMatch(
      /Persian Online.*ف.*02:09–02:13/i,
    );
    expect(PERSIAN_FEH.source.variation).toMatch(
      /body-first.*clockwise.*closed head.*broad bowl.*lift once.*dot.*Persian-scoped/i,
    );
    expect(URDU_FE.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    expect(URDU_FE.source.citation).toMatch(
      /Zer o Zabar.*independent ف.*Fe instructions.*Northwestern/i,
    );
    expect(URDU_FE.source.variation).toMatch(
      /clockwise.*above the main line.*shallow curved tail.*lift.*dot.*looped head.*Noto Naskh.*Nastaliq.*Urdu-specific/i,
    );
    expect(
      new Set([
        ARABIC_FAA.source.url,
        PERSIAN_FEH.source.url,
        URDU_FE.source.url,
      ]).size,
    ).toBe(3);
  });

  it("Arabic independent ق joins its closed head to the deep bowl before its two dots", () => {
    expect(ARABIC_QAF.script).toBe("arabic");
    expect(penLifts(ARABIC_QAF)).toBe(2);
    expect(ARABIC_QAF.strokes).toHaveLength(3);
    expect(ARABIC_QAF.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1,
    ]);
    const head = ARABIC_QAF.strokes[0].segments[0].path;
    const bowl = ARABIC_QAF.strokes[0].segments[1].path;
    const rightDot = ARABIC_QAF.strokes[1].segments[0].path;
    const leftDot = ARABIC_QAF.strokes[2].segments[0].path;
    expect(head[0]).toEqual(head.at(-1));
    expect(head.at(-1)).toEqual(bowl[0]);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(head[0].y);
    expect(rightDot[0]).toEqual(rightDot.at(-1));
    expect(leftDot[0]).toEqual(leftDot.at(-1));
    expect(Math.min(...rightDot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...head.map((point) => point.y)),
    );
    expect(Math.min(...leftDot.map((point) => point.y))).toBeGreaterThan(
      Math.max(...head.map((point) => point.y)),
    );
    expect(Math.min(...rightDot.map((point) => point.x))).toBeGreaterThan(
      Math.max(...leftDot.map((point) => point.x)),
    );
  });

  it("Persian and Urdu ق keep body-first two-dot order under script-owned provenance", () => {
    for (const ductus of [PERSIAN_QAF, URDU_QAF]) {
      expect(penLifts(ductus)).toBe(2);
      expect(ductus.strokes.map((stroke) => stroke.segments.length)).toEqual([
        2, 1, 1,
      ]);
      expect(
        ductus.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.path),
        ),
      ).toEqual(
        ARABIC_QAF.strokes.map((stroke) =>
          stroke.segments.map((segment) => segment.path),
        ),
      );
      expect(ductus.strokes[0].segments[0].path[0]).toEqual(
        ductus.strokes[0].segments[0].path.at(-1),
      );
      expect(ductus.strokes[0].segments[0].path.at(-1)).toEqual(
        ductus.strokes[0].segments[1].path[0],
      );
    }
    expect(PERSIAN_QAF.script).toBe("perso-arabic");
    expect(PERSIAN_QAF.source.citation).toMatch(
      /Persian Online.*ق.*02:14–02:18/i,
    );
    expect(PERSIAN_QAF.source.variation).toMatch(
      /body-first.*counterclockwise.*deep bowl.*upper-right dot.*upper-left dot.*Persian-scoped/i,
    );
    expect(URDU_QAF.script).toBe("urdu-nastaliq");
    expect(URDU_QAF.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    expect(URDU_QAF.source.citation).toMatch(
      /Zer o Zabar.*independent ق.*Qāf instructions.*Northwestern/i,
    );
    expect(URDU_QAF.source.variation).toMatch(
      /looped head.*deep leftward bowl.*upper-right dot.*upper-left dot.*right-to-left dot order.*Nastaliq.*Urdu-specific/i,
    );
    expect(
      new Set([
        ARABIC_QAF.source.url,
        PERSIAN_QAF.source.url,
        URDU_QAF.source.url,
      ]).size,
    ).toBe(3);
  });

  it("Arabic independent ك turns along its base before lifting for the inner arm", () => {
    expect(ARABIC_KAF.script).toBe("arabic");
    expect(penLifts(ARABIC_KAF)).toBe(1);
    expect(ARABIC_KAF.strokes).toHaveLength(2);
    expect(ARABIC_KAF.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    const upright = ARABIC_KAF.strokes[0].segments[0].path;
    const base = ARABIC_KAF.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(base[0]);
    expect(base[0].x).toBeGreaterThan(base.at(-1)!.x);
    const inner = ARABIC_KAF.strokes[1].segments[0].path;
    expect(inner[0].x).toBeGreaterThan(inner.at(-1)!.x);
    expect(inner[0].y).toBeGreaterThan(inner.at(-1)!.y);
  });

  it("Arabic independent ل descends through its leftward bowl without lifting", () => {
    expect(ARABIC_LAM.script).toBe("arabic");
    expect(penLifts(ARABIC_LAM)).toBe(0);
    expect(ARABIC_LAM.strokes).toHaveLength(1);
    expect(ARABIC_LAM.strokes[0].segments).toHaveLength(2);
    const upright = ARABIC_LAM.strokes[0].segments[0].path;
    const bowl = ARABIC_LAM.strokes[0].segments[1].path;
    expect(upright[0].y).toBeGreaterThan(upright.at(-1)!.y);
    expect(upright.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(bowl[0].y);
    expect(bowl.at(-1)!.y).toBeGreaterThan(
      Math.min(...bowl.map((point) => point.y)),
    );
  });

  it("Arabic independent م joins its closed head to the below-baseline tail", () => {
    expect(ARABIC_MEEM.script).toBe("arabic");
    expect(penLifts(ARABIC_MEEM)).toBe(0);
    expect(ARABIC_MEEM.strokes).toHaveLength(1);
    expect(ARABIC_MEEM.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_MEEM.strokes[0].segments[0].path;
    const tail = ARABIC_MEEM.strokes[0].segments[1].path;
    expect(head.at(-1)).toEqual(tail[0]);
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
    expect(Math.min(...tail.map((point) => point.y))).toBeLessThan(0);
  });

  it("Arabic independent ن sweeps its deep bowl before lifting for the dot", () => {
    expect(ARABIC_NOON.script).toBe("arabic");
    expect(penLifts(ARABIC_NOON)).toBe(1);
    expect(ARABIC_NOON.strokes).toHaveLength(2);
    expect(ARABIC_NOON.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const bowl = ARABIC_NOON.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
    const dot = ARABIC_NOON.strokes[1].segments[0].path;
    expect(Math.min(...dot.map((point) => point.y))).toBeGreaterThan(0);
  });

  it("Arabic independent ه closes both counters before its leftward baseline sweep", () => {
    expect(ARABIC_HEH.script).toBe("arabic");
    expect(penLifts(ARABIC_HEH)).toBe(0);
    expect(ARABIC_HEH.strokes).toHaveLength(1);
    expect(ARABIC_HEH.strokes[0].segments).toHaveLength(3);
    const lower = ARABIC_HEH.strokes[0].segments[0].path;
    const upperRight = ARABIC_HEH.strokes[0].segments[1].path;
    const baseline = ARABIC_HEH.strokes[0].segments[2].path;
    expect(Math.min(...lower.map((point) => point.y))).toBeLessThan(lower[0].y);
    expect(lower.at(-1)).toEqual(upperRight[0]);
    expect(Math.max(...upperRight.map((point) => point.x))).toBeGreaterThan(
      upperRight[0].x,
    );
    expect(upperRight.at(-1)).toEqual(baseline[0]);
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
  });

  it("Arabic independent و closes its head before continuing through the leftward tail", () => {
    expect(ARABIC_WAW.script).toBe("arabic");
    expect(penLifts(ARABIC_WAW)).toBe(0);
    expect(ARABIC_WAW.strokes).toHaveLength(1);
    expect(ARABIC_WAW.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_WAW.strokes[0].segments[0].path;
    const tail = ARABIC_WAW.strokes[0].segments[1].path;
    expect(head[0]).toEqual(head.at(-1));
    expect(Math.min(...head.map((point) => point.x))).toBeLessThan(head[0].x);
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(
      head[0].y,
    );
    expect(head.at(-1)).toEqual(tail[0]);
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("Arabic independent ي completes its bowl before the lower-left and lower-right dots", () => {
    expect(ARABIC_YAA.script).toBe("arabic");
    expect(penLifts(ARABIC_YAA)).toBe(2);
    expect(ARABIC_YAA.strokes).toHaveLength(3);
    expect(ARABIC_YAA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 1,
    ]);
    const descent = ARABIC_YAA.strokes[0].segments[0].path;
    const bowl = ARABIC_YAA.strokes[0].segments[1].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(bowl[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    const bodyFloor = Math.min(
      ...ARABIC_YAA.strokes[0].segments.flatMap((segment) =>
        segment.path.map((point) => point.y),
      ),
    );
    const leftDot = ARABIC_YAA.strokes[1].segments[0].path;
    const rightDot = ARABIC_YAA.strokes[2].segments[0].path;
    expect(Math.max(...leftDot.map((point) => point.y))).toBeLessThan(
      bodyFloor,
    );
    expect(Math.max(...rightDot.map((point) => point.y))).toBeLessThan(
      bodyFloor,
    );
    expect(Math.max(...leftDot.map((point) => point.x))).toBeLessThan(
      Math.min(...rightDot.map((point) => point.x)),
    );
  });

  it("Arabic independent ء continues from its c-shaped head through the lower diagonal", () => {
    expect(ARABIC_HAMZA.script).toBe("arabic");
    expect(penLifts(ARABIC_HAMZA)).toBe(0);
    expect(ARABIC_HAMZA.strokes).toHaveLength(1);
    expect(ARABIC_HAMZA.strokes[0].segments).toHaveLength(2);
    const head = ARABIC_HAMZA.strokes[0].segments[0].path;
    const diagonal = ARABIC_HAMZA.strokes[0].segments[1].path;
    expect(head[0].x).toBeGreaterThan(head.at(-1)!.x);
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(
      head[0].y,
    );
    expect(head.at(-1)).toEqual(diagonal[0]);
    expect(diagonal[0].x).toBeLessThan(diagonal.at(-1)!.x);
    expect(diagonal[0].y).toBeGreaterThan(diagonal.at(-1)!.y);
  });

  it("Persian ب sweeps right-to-left, then lifts once for the dot", () => {
    const beh = DUCTUS["ب"];
    expect(penLifts(beh)).toBe(1);
    expect(beh.strokes).toHaveLength(2);
    expect(beh.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = penPath(beh.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Persian ت sweeps right-to-left, then lifts for each dot", () => {
    const teh = DUCTUS["ت"];
    expect(penLifts(teh)).toBe(2);
    expect(teh.strokes).toHaveLength(3);
    expect(teh.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const bowl = penPath(teh.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(teh.strokes[1].segments[0].path[0].x).toBeLessThan(
      teh.strokes[2].segments[0].path[0].x,
    );
  });

  it("Persian د folds into its baseline without lifting", () => {
    const dal = DUCTUS["د"];
    expect(dal.script).toBe("perso-arabic");
    expect(penLifts(dal)).toBe(0);
    expect(dal.strokes).toHaveLength(1);
    expect(dal.strokes[0].segments).toHaveLength(2);
    const shoulder = dal.strokes[0].segments[0].path;
    const baseline = dal.strokes[0].segments[1].path;
    expect(baseline[0]).toEqual(shoulder.at(-1));
    expect(baseline[0].x).toBeGreaterThan(baseline.at(-1)!.x);
  });

  it("Persian ر descends and sweeps left without lifting", () => {
    expect(PERSIAN_RA.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_RA)).toBe(0);
    expect(PERSIAN_RA.strokes).toHaveLength(1);
    expect(PERSIAN_RA.strokes[0].segments).toHaveLength(2);
    const descent = PERSIAN_RA.strokes[0].segments[0].path;
    const curve = PERSIAN_RA.strokes[0].segments[1].path;
    expect(curve[0]).toEqual(descent.at(-1));
    expect(curve[0].x).toBeGreaterThan(curve.at(-1)!.x);
  });

  it("Persian س joins its three teeth directly to the final bowl", () => {
    const sin = DUCTUS["س"];
    expect(penLifts(sin)).toBe(0);
    expect(sin.strokes).toHaveLength(1);
    expect(sin.strokes[0].segments).toHaveLength(2);
    const path = penPath(sin.strokes[0]);
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  it("Persian ش draws its body before three separately lifted dots", () => {
    expect(PERSIAN_SHIN.script).toBe("perso-arabic");
    expect(penLifts(PERSIAN_SHIN)).toBe(3);
    expect(PERSIAN_SHIN.strokes).toHaveLength(4);
    expect(
      PERSIAN_SHIN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 1]);
    const teeth = PERSIAN_SHIN.strokes[0].segments[0].path;
    const bowl = PERSIAN_SHIN.strokes[0].segments[1].path;
    expect(bowl[0]).toEqual(teeth.at(-1));
    const [lowerLeft, lowerRight, upper] = PERSIAN_SHIN.strokes
      .slice(1)
      .map((stroke) => stroke.segments[0].path[1]);
    expect(lowerLeft.x).toBeLessThan(lowerRight.x);
    expect(upper.x).toBeGreaterThan(lowerLeft.x);
    expect(upper.x).toBeLessThan(lowerRight.x);
    expect(upper.y).toBeGreaterThan(lowerLeft.y);
  });

  it("Persian ل joins its descending upright directly to the base curve", () => {
    const lam = DUCTUS["ل"];
    expect(penLifts(lam)).toBe(0);
    expect(lam.strokes).toHaveLength(1);
    expect(lam.strokes[0].segments).toHaveLength(2);
    const path = penPath(lam.strokes[0]);
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  it("Persian م joins its round head directly to the descending tail", () => {
    const mim = DUCTUS["م"];
    expect(penLifts(mim)).toBe(0);
    expect(mim.strokes).toHaveLength(1);
    expect(mim.strokes[0].segments).toHaveLength(2);
    const head = mim.strokes[0].segments[0].path;
    const tail = mim.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("Persian ن sweeps its bowl right-to-left, then lifts once for the dot", () => {
    const nun = DUCTUS["ن"];
    expect(penLifts(nun)).toBe(1);
    expect(nun.strokes).toHaveLength(2);
    expect(nun.strokes.map((stroke) => stroke.segments.length)).toEqual([1, 1]);
    const bowl = penPath(nun.strokes[0]);
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("Persian و joins its small head loop directly to the leftward tail", () => {
    const waw = DUCTUS["و"];
    expect(penLifts(waw)).toBe(0);
    expect(waw.strokes).toHaveLength(1);
    expect(waw.strokes[0].segments).toHaveLength(2);
    const head = waw.strokes[0].segments[0].path;
    const tail = waw.strokes[0].segments[1].path;
    expect(Math.max(...head.map((point) => point.y))).toBeGreaterThan(
      Math.max(...tail.map((point) => point.y)),
    );
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
  });

  it("Persian ه keeps its isolated looping body in one pen-down run", () => {
    const heh = DUCTUS["ه"];
    expect(penLifts(heh)).toBe(0);
    expect(heh.strokes).toHaveLength(1);
    expect(heh.strokes[0].segments).toHaveLength(1);
    const path = penPath(heh.strokes[0]);
    expect(Math.max(...path.map((point) => point.y))).toBeGreaterThan(
      Math.min(...path.map((point) => point.y)),
    );
    expect(path[0].x).toBeGreaterThan(path.at(-1)!.x);
  });

  it("Persian ا traces to UT Austin's opening right-to-left freehand demonstration", () => {
    const src = DUCTUS["ا"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ا.*00:08–00:11/i);
    expect(src.variation).toMatch(/top-to-bottom.*right-to-left.*Noto Naskh/i);
  });

  it("Urdu independent ا traces to Zer o Zabar's top-to-bottom animation", () => {
    const src = URDU_ALEF.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(src.citation).toMatch(/Zer o Zabar.*independent ا.*Northwestern/i);
    expect(src.variation).toMatch(
      /independent.*top-to-bottom.*one continuous stroke.*final.*bottom-to-top.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["ا"].source.url);
  });

  it("Persian and Urdu د keep their independently verified sources", () => {
    const persian = DUCTUS["د"].source;
    const urdu = URDU_DAL.source;
    expect(persian.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(persian.citation).toMatch(/Persian Online.*د.*01:04–01:06/i);
    expect(persian.variation).toMatch(
      /continuous Naskh.*upper tip.*shoulder.*baseline.*without lifting.*non-connector.*Persian-scoped/i,
    );
    expect(urdu.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/dal-re-and-waw/",
    );
    expect(urdu.citation).toMatch(
      /Zer o Zabar.*independent د.*Dāl instructions.*Northwestern/i,
    );
    expect(urdu.variation).toMatch(
      /one uninterrupted stroke.*folded shoulder.*leftward baseline.*90-degree angle.*does not drop below.*Naskh.*Nastaliq.*Urdu-specific/i,
    );
    expect(persian.url).not.toBe(urdu.url);
    expect(persian.url).not.toBe(ARABIC_DAAL.source.url);
    expect(urdu.url).not.toBe(ARABIC_DAAL.source.url);
  });

  it("Persian and Urdu خ keep body-first, dot-last script-owned sources", () => {
    expect(
      PERSIAN_KHEH.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1]);
    expect(URDU_KHE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1,
    ]);
    expect(penLifts(PERSIAN_KHEH)).toBe(1);
    expect(penLifts(URDU_KHE)).toBe(1);
    expect(PERSIAN_KHEH.source.citation).toMatch(
      /Persian Online.*خ.*00:49–00:54/i,
    );
    expect(PERSIAN_KHEH.source.variation).toMatch(
      /body-first.*head.*left to right.*deep bowl.*lifts once.*dot above.*Persian-scoped/i,
    );
    expect(URDU_KHE.source.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/khe-ze-zal-swad-and-zwad/",
    );
    expect(URDU_KHE.source.citation).toMatch(
      /Zer o Zabar.*independent خ.*Ḳhe instructions.*Northwestern/i,
    );
    expect(URDU_KHE.source.variation).toMatch(
      /deep bowl.*body-first.*lifts once.*dot above.*jīm shape.*Noto Naskh.*Nastaliq.*Urdu-specific/i,
    );
    expect(
      new Set([
        ARABIC_KHAA.source.url,
        PERSIAN_KHEH.source.url,
        URDU_KHE.source.url,
      ]).size,
    ).toBe(3);
  });

  it("Arabic independent ا traces to the University of Oregon's top-to-bottom video", () => {
    const src = ARABIC_ALEF.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%A8/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ا ب.*00:05–00:07.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /one continuous top-to-bottom stroke.*no pen lift.*one-way connector.*isolated and final forms.*Noto Naskh.*Arabic provenance.*Persian and Urdu/i,
    );
    expect(src.url).not.toBe(DUCTUS["ا"].source.url);
    expect(src.url).not.toBe(URDU_ALEF.source.url);
  });

  it("Arabic independent ب traces to the University of Oregon's bowl-first video", () => {
    const src = ARABIC_BAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%A8/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ا ب.*Baa.*00:02–00:04.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /upper-right tip.*right-to-left.*shallow bowl.*left tip.*lifting once.*dot below.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic provenance.*Persian/i,
    );
    expect(src.url).not.toBe(DUCTUS["ب"].source.url);
  });

  it("Arabic independent ت traces its bowl and separate dots to the University of Oregon", () => {
    const src = ARABIC_TAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/two-way-connectors-%D8%A8-%D8%AA-%D8%AB-%D9%86-%D9%8A/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ب ت ث.*Baa.*00:02–00:04.*Taa.*00:00–00:01.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Baa demonstration.*upper-right tip.*right-to-left.*turned-up left tip.*Taa demonstration opens.*complete bowl.*left dot.*00:00.45–00:00.70.*right dot.*00:00.75–00:01.00.*does not redraw.*rather than inferring.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic provenance.*Persian/i,
    );
    expect(src.url).not.toBe(DUCTUS["ت"].source.url);
  });

  it("Arabic independent ث traces its bowl-first form to the University of Oregon", () => {
    const src = ARABIC_THAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/two-way-connectors-%D8%A8-%D8%AA-%D8%AB-%D9%86-%D9%8A/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ب ت ث.*Thaa demonstration.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /dedicated Thaa video.*body-first.*upper-right tip.*right-to-left.*turned-up left tip.*three upper dots.*two-lower-and-one-centred-upper.*lower-left.*lower-right.*centred upper.*four pen-down runs.*three lifts.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic provenance/i,
    );
  });

  it("Arabic independent ج traces its body-first order to the University of Oregon", () => {
    const src = ARABIC_JEEM.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Jeem.*00:05–00:06.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /body first.*00:05.1–00:05.8.*upper head.*left-to-right.*turns downward.*curls back left.*rounded bowl.*without lifting.*lifts once.*dot below.*00:06.3–00:06.5.*two-way connector.*contextual shapes.*Noto Naskh.*Arabic body-first provenance.*Urdu dot-first/i,
    );
    expect(src.url).not.toBe(URDU_JIM.source.url);
  });

  it("Arabic independent ح traces its stem-first order to the page's Haa attachment", () => {
    const src = ARABIC_HAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Haa.*00:00–00:01.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Haa attachment.*two pen-down runs.*opens.*first mark already underway.*short left stem downward.*00:00.00–00:00.15.*lifts once.*restarts near the stem's upper portion.*00:00.32.*down-right and around the bowl.*without another lift.*00:00.82.*two-way connector.*contextual shapes.*no dot stroke.*stem-first order.*rather than inherited from ج.*Noto Naskh.*Arabic provenance/i,
    );
    expect(src.url).toBe(ARABIC_JEEM.source.url);
  });

  it("Arabic independent خ traces its body-first order to its own Khaa clip", () => {
    const src = ARABIC_KHAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%AC-%D8%AD-%D8%AE/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: ج ح خ.*Khaa.*00:02–00:04.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /Khaa QuickTime clip.*body-first.*00:02.8–00:03.9.*upper head.*left-to-right.*same pen-down run.*turns downward.*curls around the bowl.*lifts once.*dot above.*00:04.2–00:04.4.*two-way connector.*contextual shapes.*own clip.*matches adjacent Jeem.*rather than Haa.*stem-first restart.*Noto Naskh.*Arabic provenance/i,
    );
    expect(src.url).toBe(ARABIC_JEEM.source.url);
    expect(src.url).toBe(ARABIC_HAA.source.url);
  });

  it("Arabic independent د traces its unbroken turn to the University of Oregon", () => {
    const src = ARABIC_DAAL.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/chapter-1/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: د ذ ر.*Daal.*00:07.0–00:07.6.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*00:07.0–00:07.6.*upper tip.*diagonally down and right.*curved shoulder.*turns left.*baseline.*without lifting.*one-way connector.*independent and final forms.*Noto Naskh.*scoped to Arabic.*contextual form/i,
    );
  });

  it("Arabic independent ذ traces its body-first dot-last order to the University of Oregon", () => {
    const src = ARABIC_DHAAL.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/chapter-1/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: د ذ ر.*Dhaal.*Oregon.*2023.*2026-08-21/i,
    );
    expect(src.variation).toMatch(
      /directly linked dhaal\.mp4.*body-first.*upper tip.*diagonally down and right.*curved shoulder.*turns left.*baseline.*without lifting.*pen lifts once.*single dot above.*one-way connector.*this.*throw.*dh.*independent and final forms.*two-stroke.*one-lift.*Noto Naskh.*shares its body with د.*own video.*rather than inferred/i,
    );
  });

  it("Arabic independent ر traces its unbroken curve to the University of Oregon", () => {
    const src = ARABIC_RAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/chapter-1/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: د ذ ر.*Raa.*00:08.8–00:09.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /one continuous pen-down run.*00:08.8–00:09.3.*upper tip.*descends through the short stroke.*sweeps left.*lower curve.*without lifting.*one-way connector.*independent and final forms.*Noto Naskh.*scoped to Arabic.*Urdu ر source.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(URDU_RE.source.url);
  });

  it("Arabic independent ز traces its body-first dot-last order to the University of Oregon", () => {
    const src = ARABIC_ZAY.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%B1-%D8%B2-%D9%88/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ر ز و.*Zay.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked zaay\.mp4.*body-first.*upper tip.*descends through the short stroke.*sweeps left.*lower curve.*without lifting.*complete Raa-shaped body.*lifts once.*single dot above.*one-way connector.*English z sound.*\/z\/ transliteration.*independent and final forms.*two-stroke.*one-lift.*Noto Naskh.*shares its body with ر.*own video.*rather than inferred/i,
    );
  });

  it("Arabic independent س traces its continuous teeth and bowl to the University of Oregon", () => {
    const src = ARABIC_SEEN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Seen.*00:01.6–00:02.8.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-8.mov.*one continuous pen-down run.*00:01.6–00:02.8.*upper right.*three close teeth.*right to left.*final bowl.*without lifting.*two-way connector.*contextual shapes.*Noto Naskh.*scoped to Arabic.*Persian or Urdu س sources.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["س"].source.url);
    expect(src.url).not.toBe(URDU_SIN.source.url);
  });

  it("Arabic independent ش traces its body-first dots to the University of Oregon", () => {
    const src = ARABIC_SHIIN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Shiin.*00:00.7–00:03.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-7.mov.*body-first.*one continuous pen-down run.*00:00.7–00:02.2.*three close teeth.*right to left.*final bowl.*lower-left dot.*00:02.4–00:02.5.*lower-right dot.*00:02.7–00:02.8.*centered upper dot.*00:02.9–00:03.0.*two-way connector.*contextual shapes.*four-stroke.*three-lift.*Noto Naskh.*scoped to Arabic.*Urdu ش source.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(URDU_SHIN.source.url);
  });

  it("Arabic independent ص traces its lifted trailing bowl to the University of Oregon", () => {
    const src = ARABIC_SAAD.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Saad.*00:01.1–00:03.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /FullSizeRender-6.mov.*two pen-down runs.*00:01.1–00:02.4.*lower-left junction.*oval clockwise.*turns left.*short shoulder.*without lifting.*one lift.*00:02.6–00:03.3.*baseline junction.*descends.*trailing bowl.*sweeps left.*finishes above the baseline.*two-way connector.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*distinct from.*Seen and Shiin/i,
    );
  });

  it("Arabic independent ض traces its Saad skeleton and final dot to the embedded Oregon lesson", () => {
    const src = ARABIC_DAAD.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B3-%D8%B4-%D8%B5-%D8%B6/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet: س ش ص ض.*Daad.*00:43.1–00:46.3.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /embedded Panopto Daad lesson.*three pen-down runs.*00:43.1–00:46.3.*00:43.1–00:45.0.*lower-left junction.*oval clockwise.*short shoulder.*without lifting.*one lift.*00:45.2–00:45.4.*baseline junction.*trailing bowl.*second lift.*upper dot last.*00:46.0–00:46.3.*FullSizeRender-5.mov.*HTTP 403.*accessible embedded primary lesson.*embedded Saad lesson.*direct Saad clip.*same two body runs.*two-way connector.*contextual shapes.*three-stroke.*two-lift.*Noto Naskh.*independently evidenced.*Saad/i,
    );
  });

  it("Arabic independent ط traces its loop-before-upright order to the Oregon lesson", () => {
    const src = ARABIC_TAH.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%B7-%D8%B8/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ط ظ.*emphatic Taa.*00:01.2–00:03.0.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked taaemphatic\.mov.*embedded Panopto mirror.*two pen-down runs.*00:01.2–00:03.0.*upper-right edge.*counterclockwise.*closed body.*leftward.*baseline.*one lift.*upright's top.*descends.*right junction.*two-way connector.*emphatic t sound.*T transliteration.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*retraces.*upper-left arc.*without crossing the counter.*directions stay unchanged.*independent form.*connected examples/i,
    );
  });

  it("Arabic independent ظ traces its body-dot-upright order to the Oregon lesson", () => {
    const src = ARABIC_ZAH.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%D8%B7-%D8%B8/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ط ظ.*emphatic DHaa.*00:01.3–00:02.8.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked zaa-emphatic\.mov.*embedded Panopto mirror.*three pen-down runs.*00:01.3–00:02.8.*00:01.3–00:02.1.*upper-right edge.*counterclockwise.*ط-shaped body.*leftward.*baseline.*one lift.*upper dot.*00:02.4–00:02.5.*second lift.*00:02.6–00:02.8.*upright's top.*descends.*right junction.*two-way connector.*emphatic dh sound.*DH or Z transliteration.*three-stroke.*two-lift.*Noto Naskh.*body median.*independently evidenced ط body.*dot-before-upright.*separately sourced/i,
    );
  });

  it("Arabic independent ع traces its unbroken head and bowl to the Oregon MOV", () => {
    const src = ARABIC_AYN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B9-%D8%BA/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ع غ.*Ayn.*00:03.1–00:04.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked ayn.mov.*one continuous pen-down run.*00:03.1–00:04.0.*00:03.1–00:03.5.*upper-right tip.*sweeps left.*hooks downward.*open head.*without lifting.*00:03.5–00:04.0.*left side.*lower bowl.*floor.*finishes toward the right.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*distinct from.*Ghayn.*upper dot/i,
    );
  });

  it("Arabic independent غ traces its complete body and final dot to the Oregon MOV", () => {
    const src = ARABIC_GHAYN.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%D8%B9-%D8%BA/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ع غ.*Ghayn.*00:02.4–00:04.0.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked ghayn\.mov.*two pen-down runs.*00:02.4–00:04.0.*00:02.4–00:03.2.*upper-right tip.*sweeps left.*open ع head.*without lifting.*broad lower bowl.*finish toward the right.*one lift.*upper dot.*00:03.9–00:04.0.*two-way connector.*no English equivalent.*gh transliteration.*two-stroke.*one-lift.*Noto Naskh.*independently evidenced dot-last order.*distinct from.*Ayn/i,
    );
  });

  it("Arabic independent ف traces its joined head and bowl before the final dot", () => {
    const src = ARABIC_FAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%81-%d9%82/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ف ق.*Faa.*00:01.7–00:03.3.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked faa\.mov.*two pen-down runs.*00:01.7–00:03.3.*00:01.7–00:02.5.*upper-right edge.*counterclockwise.*closed counter.*without lifting.*down from the head.*left through the broad independent bowl.*rising left tip.*one lift.*upper dot.*00:03.2–00:03.3.*two-way connector.*f sound.*transliteration.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*loop-to-bowl continuity.*dot-last order/i,
    );
  });

  it("Arabic independent ق traces its joined head and deep bowl before two ordered dots", () => {
    const src = ARABIC_QAF.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%81-%d9%82/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ف ق.*Qaf.*00:01.5–00:03.5.*Oregon.*2023.*2026-08-23/i,
    );
    expect(src.variation).toMatch(
      /directly linked qaf\.mov.*three pen-down runs.*00:01.5–00:03.5.*00:01.5–00:03.1.*upper-right edge.*counterclockwise.*closed counter.*without lifting.*deep independent bowl.*rising left tip.*one lift.*upper-right dot.*00:03.4.*second lift.*upper-left dot.*00:03.5.*two-way connector.*no English equivalent.*deep echo.*q transliteration.*three-stroke.*two-lift.*Noto Naskh.*independently demonstrated deep bowl.*right-to-left dot order/i,
    );
  });

  it("Arabic independent ك traces its joined outer body and restarted inner arm to the Oregon MOV", () => {
    const src = ARABIC_KAF.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Kaf.*00:11.8–00:13.4.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked kaf.mov.*two pen-down runs.*00:11.8–00:12.9.*main upright.*turns left.*baseline.*without lifting.*one lift.*00:13.2–00:13.4.*upper right.*inner arm.*down-left.*two-way connector.*contextual shapes.*two-stroke.*one-lift.*Noto Naskh.*Arabic-scoped ك.*distinct from Urdu ک.*different Unicode glyph.*separate source-backed fallback order/i,
    );
    expect(src.url).not.toBe(URDU_KAF.source.url);
    expect(ARABIC_KAF.glyph).not.toBe(URDU_KAF.glyph);
  });

  it("Arabic independent ل traces its unbroken upright and bowl to the Oregon MOV", () => {
    const src = ARABIC_LAM.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Lam.*00:01.9–00:02.4.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked lam.mov.*one continuous pen-down run.*00:01.9–00:02.4.*descends the tall upright.*turns left.*base bowl.*without lifting.*rises.*outer edge.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*Arabic-scoped ل.*distinct from.*Persian and Urdu.*same Unicode glyph.*own source-backed orders/i,
    );
    expect(src.url).not.toBe(DUCTUS["ل"].source.url);
    expect(src.url).not.toBe(URDU_LAM.source.url);
    expect(ARABIC_LAM.glyph).toBe(DUCTUS["ل"].glyph);
    expect(ARABIC_LAM.glyph).toBe(URDU_LAM.glyph);
  });

  it("Arabic independent م traces its continuous head-and-tail order to Waraqa", () => {
    const src = ARABIC_MEEM.source;
    expect(src.url).toBe(
      "https://www.waraqaweb.com/lessons/letter-meem-in-arabic",
    );
    expect(src.citation).toMatch(
      /Waraqa Institute.*Letter Meem in Arabic.*Writing Meem.*Round Head.*Tail.*Step 4.*2026-08-21/i,
    );
    expect(src.variation).toMatch(
      /beginner lesson.*isolated م.*head-first.*small tightly closed.*circular or oval loop.*tail downward and leftward.*one continuous curve.*below the baseline.*two-way connector.*isolated\/final tail.*tailless initial\/medial.*one-stroke.*zero-lift.*Noto Naskh.*Arabic-scoped provenance.*Persian and Urdu.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["م"].source.url);
    expect(src.url).not.toBe(URDU_MIM.source.url);
    expect(ARABIC_MEEM.glyph).toBe(DUCTUS["م"].glyph);
    expect(ARABIC_MEEM.glyph).toBe(URDU_MIM.glyph);
  });

  it("Arabic independent ن traces its body-first bowl-and-dot order to Waraqa", () => {
    const src = ARABIC_NOON.source;
    expect(src.url).toBe(
      "https://www.waraqaweb.com/lessons/letter-noon-in-arabic",
    );
    expect(src.citation).toMatch(
      /Waraqa Institute.*Letter Noon in Arabic.*Writing Noon.*Bowl Shape.*Single Dot.*Step 4.*2026-08-21/i,
    );
    expect(src.variation).toMatch(
      /beginner lesson.*isolated ن.*body-first.*top right.*sweep down and around.*deep below-baseline bowl.*single centred upper dot last.*isolated\/final bowl.*ب, ت, and ث.*initial\/medial form.*small tooth.*two-way connector.*two-stroke.*one-lift.*Noto Naskh.*Arabic-scoped provenance.*Persian and Urdu.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["ن"].source.url);
    expect(src.url).not.toBe(URDU_NUN.source.url);
    expect(ARABIC_NOON.glyph).toBe(DUCTUS["ن"].glyph);
    expect(ARABIC_NOON.glyph).toBe(URDU_NUN.glyph);
  });

  it("Arabic independent ي traces its bowl and left-then-right lower dots to the Oregon MOV", () => {
    const src = ARABIC_YAA.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/alphabet-%d9%82-%d9%84-%d9%85/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ي ك ل.*Yaa.*00:33.2–00:35.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked yaa.mov.*three pen-down runs.*upper right.*00:33.2.*descends.*sweeps left.*independent bowl.*without lifting.*00:34.4.*one lift.*lower-left dot.*00:34.5–00:34.7.*second lift.*lower-right dot.*00:34.8–00:35.0.*two-way connector.*contextual shapes.*three-stroke.*two-lift.*Noto Naskh.*U\+064A.*separate from Urdu ی.*U\+06CC.*no lower dots.*own source-backed order/i,
    );
    expect(src.url).not.toBe(URDU_YE.source.url);
    expect(ARABIC_YAA.glyph).not.toBe(URDU_YE.glyph);
  });

  it("Arabic independent ء traces its one-stroke variant to Arabic Language Learning Notes", () => {
    const src = ARABIC_HAMZA.source;
    expect(src.url).toBe(
      "https://alarabiyah.sakura.ne.jp/arabic/alphabets/naskh/hamzah/",
    );
    expect(src.citation).toMatch(
      /Arabic Language Learning Notes.*Basic Naskh.*Hamza ء.*00:33–00:38.*2022-04-09.*2026-08-21/i,
    );
    expect(src.variation).toMatch(
      /c-shaped upper head.*lower slash.*books vary.*lift after the c.*one-stroke variant.*without lifting.*embedded original video.*00:33.*lower-left end.*lower diagonal.*right.*00:38.*upper part of ع.*alone or on a carrier.*one-stroke.*zero-lift.*Noto Naskh.*alternative two-stroke convention/i,
    );
  });

  it("Arabic independent ه traces its two counters and baseline sweep to the Oregon MOV", () => {
    const src = ARABIC_HEH.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%d9%87-%d9%88-%d9%8a/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ه و ي.*Heh.*00:04.9–00:06.0.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked letter-haa.mov.*one continuous pen-down run.*00:04.9–00:06.0.*upper right.*00:04.9–00:05.4.*down-left.*lower counter.*without lifting.*centre.*upper-right counter.*00:05.4–00:05.7.*baseline.*00:06.0.*two-way connector.*contextual shapes.*one-stroke.*zero-lift.*Noto Naskh.*Arabic ه.*script-scoped provenance.*Persian.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["ه"].source.url);
    expect(ARABIC_HEH.glyph).toBe(DUCTUS["ه"].glyph);
  });

  it("Arabic independent و traces its closed head and leftward tail to the Oregon MOV", () => {
    const src = ARABIC_WAW.source;
    expect(src.url).toBe(
      "https://opentext.uoregon.edu/introarabic/chapter/%d9%87-%d9%88-%d9%8a/",
    );
    expect(src.citation).toMatch(
      /Introduction to Arabic.*Alphabet ه و ي.*Waw.*00:45.7–00:46.9.*Oregon/i,
    );
    expect(src.variation).toMatch(
      /directly linked waw.mov.*one continuous pen-down run.*00:45.7–00:46.9.*lower-right junction.*00:45.7–00:46.5.*sweeps left.*curves up and around.*small head loop.*without lifting.*00:46.5–00:46.9.*descends.*curls left.*tail.*one-way connector.*consonant w.*long-vowel ū.*one-stroke.*zero-lift.*Noto Naskh.*Arabic و.*script-scoped provenance.*Persian.*same Unicode glyph/i,
    );
    expect(src.url).not.toBe(DUCTUS["و"].source.url);
    expect(ARABIC_WAW.glyph).toBe(DUCTUS["و"].glyph);
  });

  it("Urdu independent ج traces to Zer o Zabar's dot-first pointed-head animation", () => {
    const src = URDU_JIM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ج.*flat-head.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /dot below first.*lifts once.*pointed hooked head.*one continuous stroke.*pointed rather than rounded.*flat-head.*purely aesthetic.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent چ traces to Zer o Zabar's body-first three-dot animations", () => {
    const src = URDU_CHE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent چ.*calligraphic and handwriting animations.*Che instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /pointed hooked head.*deep bowl.*body-first stroke.*lower-left dot.*lower-right dot.*lower-center dot.*three pen lifts.*jīm-series shape.*three dots below.*ch sound.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Persian independent چ traces to its adjacent body-first freehand demonstration", () => {
    const src = PERSIAN_CHE.source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*چ.*00:35–00:41/i);
    expect(src.variation).toMatch(
      /body-first.*head.*left to right.*deep bowl.*three separate dots below.*left, right, then lower-center.*Noto Naskh.*Persian-scoped.*Urdu/i,
    );
  });

  it("Urdu independent ر traces to Zer o Zabar's downward-then-leftward animation", () => {
    const src = URDU_RE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/dal-re-and-waw/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ر.*Re instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*downward line.*curve to the left.*final form.*lower left.*final re rises in Naskh.*not in Nastaliq.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent س traces to Zer o Zabar's continuous teeth-and-bowl animations", () => {
    const src = URDU_SIN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent س.*calligraphic and handwriting animations.*Sīn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*three close teeth.*right to left.*final bowl without lifting.*optional long gentle curve.*especially common in handwriting.*adjacent sīns.*standard toothed.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["س"].source.url);
  });

  it("Urdu independent ش traces to Zer o Zabar's body-first three-dot animations", () => {
    const src = URDU_SHIN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ش.*calligraphic and handwriting animations.*Shīn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /standard toothed sīn body first.*lower-left dot.*lower-right dot.*centered upper dot.*four strokes.*three pen lifts.*two below.*nestled.*optional long gentle curve.*dots stay centered.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ک traces to Zer o Zabar's body-first two-stroke animations", () => {
    const src = URDU_KAF.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/be-kaf-and-short-vowels/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ک.*calligraphic and handwriting animations.*Kāf instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /two separate pen strokes.*stem.*main line.*right to left.*flatter bowl.*pronounced final hook.*lift once.*upper right.*long downward slash.*not to write kāf in one penstroke.*flatter than be.*Noto Naskh.*rather than Arabic ك.*Nastaliq/i,
    );
  });

  it("Urdu independent ل traces to Zer o Zabar's unbroken downward-and-around animations", () => {
    const src = URDU_LAM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ل.*calligraphic and handwriting animations.*Lām instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*begin at the top.*descend the tall upright.*below the baseline.*leftward bowl.*back up.*without lifting.*connector.*final form.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent م traces to Zer o Zabar's unbroken head-and-tail animations", () => {
    const src = URDU_MIM.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent م.*calligraphic and handwriting animations.*Mīm instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*calligraphic and handwritten.*ordinary constant-width pen.*counterclockwise loop.*independent or final mīm drops below the baseline.*head-to-tail.*zero-lift.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["م"].source.url);
  });

  it("Urdu independent ن traces to Zer o Zabar's bowl-first, dot-second animations", () => {
    const src = URDU_NUN.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ن.*calligraphic and handwriting animations.*Nūn instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /bowl first.*one uninterrupted right-to-left run.*lift once.*dot near the baseline.*final and independent nūn.*below the baseline.*initial and medial.*be-series tooth.*Noto Naskh.*Nastaliq/i,
    );
    expect(src.url).not.toBe(DUCTUS["ن"].source.url);
  });

  it("Urdu independent ہ traces to Zer o Zabar's unbroken teardrop animations", () => {
    const src = URDU_HE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ہ.*calligraphic and handwriting animations.*Chhoṭī he instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted counterclockwise loop.*upper right.*down and left.*around the base.*return up the right side.*cross at the top.*without lifting.*independent form.*oval or teardrop.*initial and medial.*small divot.*number-6-like mark.*final form.*up and then down.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ھ traces to Zer o Zabar's unbroken two-eyed animations", () => {
    const src = URDU_DO_CHASHMI_HE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ھ.*calligraphic and handwriting animations.*Aspiration with do-chashmī he instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted stroke.*upper center.*right eye clockwise.*down and left along the baseline.*reverse at the left edge.*left eye.*low leftward sweep.*without lifting.*two eyes.*aspiration.*pen stops and reverses.*medial and final forms.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Urdu independent ی traces to Zer o Zabar's dotless S-shaped animations", () => {
    const src = URDU_YE.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ی.*calligraphic and handwriting animations.*Chhoṭī ye instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted dotless S-shaped body.*upper right.*descend through the upper curve.*sweep left around the below-baseline bowl.*rising tip.*without lifting.*independent and final chhoṭī ye.*ī sound.*initial and medial.*be-series tooth.*two dots below.*do not belong to the independent form.*Noto Naskh.*Nastaliq/i,
    );
  });

  it("Persian independent ی traces to Persian Online's closing freehand demonstration", () => {
    const src = PERSIAN_YEH.source;
    expect(src.url).toBe(
      "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
    );
    expect(src.citation).toMatch(
      /Persian Online.*closing ی.*02:55–02:58.*Texas/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted dotless S-shaped run.*upper right.*sweep left.*central turn.*below-baseline bowl.*rising tip.*without lifting.*initial and medial.*be-series tooth.*Noto Naskh.*Persian-scoped.*Urdu/i,
    );
  });

  it("Urdu independent ں traces to Zer o Zabar's dotless nūn animations", () => {
    const src = URDU_GHUNNA.source;
    expect(src.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(src.citation).toMatch(
      /Zer o Zabar.*independent ں.*calligraphic and handwriting animations.*Nasalization with nūn-e ġhunna instructions.*Northwestern/i,
    );
    expect(src.variation).toMatch(
      /one uninterrupted right-to-left bowl.*below the baseline.*without lifting.*final and independent nūn-e ġhunna.*nūn without any dot.*initial and medial.*identical to regular nūn.*sukūn.*semicircular diacritic.*U\+06BA.*U\+0646.*body contour.*dot removed.*Nastaliq/i,
    );
  });

  it("Persian ب traces to the adjacent sourced bowl-and-dot demonstration", () => {
    const src = DUCTUS["ب"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ب.*00:11–00:15/i);
    expect(src.variation).toMatch(
      /right-to-left.*pen lift.*dot below.*Noto Naskh/i,
    );
  });

  it("Persian پ traces to the intervening sourced bowl-and-three-dots demonstration", () => {
    const src = DUCTUS["پ"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*پ.*00:16–00:21/i);
    expect(src.variation).toMatch(
      /right-to-left.*three separate dots below.*left, right, then lower-center.*Noto Naskh/i,
    );
  });

  it("Persian ت traces to the later sourced bowl-and-two-dots demonstration", () => {
    const src = DUCTUS["ت"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ت.*00:22–00:27/i);
    expect(src.variation).toMatch(
      /right-to-left.*left dot.*another lift.*right dot.*Noto Naskh/i,
    );
  });

  it("Persian س traces to the later continuous teeth-and-bowl demonstration", () => {
    const src = DUCTUS["س"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*س.*01:23–01:28/i);
    expect(src.variation).toMatch(
      /continuous right-to-left.*three teeth.*final bowl.*no pen lift.*Noto Naskh/i,
    );
  });

  it("Persian ش traces its body-first three-dot order to the freehand demonstration", () => {
    const src = PERSIAN_SHIN.source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ش.*01:29–01:35/i);
    expect(src.variation).toMatch(
      /body-first.*continuous right-to-left.*three teeth.*final bowl.*lower-left.*lower-right.*centered-upper.*four-stroke.*Noto Naskh.*Persian-scoped provenance.*Arabic or Urdu ش sources/i,
    );
  });

  it("Persian ل traces to the later continuous upright-and-base demonstration", () => {
    const src = DUCTUS["ل"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ل.*02:29–02:32/i);
    expect(src.variation).toMatch(
      /isolated.*continuous Naskh.*upright descends.*base curve.*no pen lift.*Noto Naskh/i,
    );
  });

  it("Persian م traces to the adjacent continuous head-and-tail demonstration", () => {
    const src = DUCTUS["م"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*م.*02:33–02:36/i);
    expect(src.variation).toMatch(
      /isolated.*continuous Naskh.*round head.*descending tail.*no pen lift.*Noto Naskh/i,
    );
  });

  it("Persian ن traces to the adjacent bowl-and-dot demonstration", () => {
    const src = DUCTUS["ن"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ن.*02:37–02:43/i);
    expect(src.variation).toMatch(
      /isolated.*right-to-left Naskh bowl.*one lift.*dot above.*Noto Naskh/i,
    );
  });

  it("Persian و traces to the intervening continuous loop-and-tail demonstration", () => {
    const src = DUCTUS["و"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*و.*02:43–02:45/i);
    expect(src.variation).toMatch(
      /isolated.*continuous Naskh.*small head.*leftward curving tail.*no pen lift.*Noto Naskh/i,
    );
  });

  it("Persian ه traces to the later continuous looping-body demonstration", () => {
    const src = DUCTUS["ه"].source;
    expect(src.url).toContain("laits.utexas.edu/persian_grammar/video");
    expect(src.citation).toMatch(/Persian Online.*ه.*02:47–02:50/i);
    expect(src.variation).toMatch(
      /simple closed handwritten loop.*no pen lift.*Noto Naskh.*two counters.*leftward baseline/i,
    );
  });
});
