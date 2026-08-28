import { beforeAll, describe, expect, it } from "vitest";
import {
  DUCTUS,
  ductusKey,
  penPathD,
  type LetterDuctus,
} from "../../src/strokes";
import {
  ductusFilmstrip,
  ductusFor,
  ductusFrame,
  ductusSteps,
  escapeXml,
  isSafeName,
  segmentEndFractions,
  svgMarkup,
  viewBoxFor,
  wrapCaption,
  type GlyphOutline,
  type SvgNode,
} from "../../src/ductusview";
import {
  chineseOutline,
  cyrillicOutline,
  devanagariOutline,
  gujaratiOutline,
  hebrewOutline,
  japaneseOutline,
  kannadaOutline,
  malayalamOutline,
  naskhOutline,
  tamilOutline,
  teluguOutline,
} from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

const PERSIAN_ALEF = DUCTUS["ا"];
const persianAlefOutline = naskhOutline("ا");
const ARABIC_ALEF = ductusFor("ا", "arabic")!;
const arabicAlefOutline = naskhOutline("ا");
const ARABIC_BAA = ductusFor("ب", "arabic")!;
const arabicBaaOutline = naskhOutline("ب");
const ARABIC_TAA = ductusFor("ت", "arabic")!;
const arabicTaaOutline = naskhOutline("ت");
const ARABIC_THAA = ductusFor("ث", "arabic")!;
const arabicThaaOutline = naskhOutline("ث");
const ARABIC_JEEM = ductusFor("ج", "arabic")!;
const arabicJeemOutline = naskhOutline("ج");
const ARABIC_HAA = ductusFor("ح", "arabic")!;
const PERSIAN_HAH = ductusFor("ح", "perso-arabic")!;
const URDU_BARI_HE = ductusFor("ح", "urdu-nastaliq")!;
const arabicHaaOutline = naskhOutline("ح");
const ARABIC_KHAA = ductusFor("خ", "arabic")!;
const arabicKhaaOutline = naskhOutline("خ");
const ARABIC_DAAL = ductusFor("د", "arabic")!;
const arabicDaalOutline = naskhOutline("د");
const ARABIC_RAA = ductusFor("ر", "arabic")!;
const arabicRaaOutline = naskhOutline("ر");
const ARABIC_SEEN = ductusFor("س", "arabic")!;
const arabicSeenOutline = naskhOutline("س");
const ARABIC_SHIIN = ductusFor("ش", "arabic")!;
const arabicShiinOutline = naskhOutline("ش");
const ARABIC_SAAD = ductusFor("ص", "arabic")!;
const arabicSaadOutline = naskhOutline("ص");
const ARABIC_DAAD = ductusFor("ض", "arabic")!;
const arabicDaadOutline = naskhOutline("ض");
const ARABIC_AYN = ductusFor("ع", "arabic")!;
const arabicAynOutline = naskhOutline("ع");
const ARABIC_KAF = ductusFor("ك", "arabic")!;
const arabicKafOutline = naskhOutline("ك");
const ARABIC_LAM = ductusFor("ل", "arabic")!;
const arabicLamOutline = naskhOutline("ل");
const ARABIC_MEEM = ductusFor("م", "arabic")!;
const arabicMeemOutline = naskhOutline("م");
const ARABIC_NOON = ductusFor("ن", "arabic")!;
const arabicNoonOutline = naskhOutline("ن");
const ARABIC_HEH = ductusFor("ه", "arabic")!;
const arabicHehOutline = naskhOutline("ه");
const ARABIC_WAW = ductusFor("و", "arabic")!;
const arabicWawOutline = naskhOutline("و");
const ARABIC_YAA = ductusFor("ي", "arabic")!;
const arabicYaaOutline = naskhOutline("ي");
const ARABIC_HAMZA = ductusFor("ء", "arabic")!;
const arabicHamzaOutline = naskhOutline("ء");
const URDU_ALEF = ductusFor("ا", "urdu-nastaliq")!;
const urduAlefOutline = naskhOutline("ا");
const URDU_JIM = ductusFor("ج", "urdu-nastaliq")!;
const urduJimOutline = naskhOutline("ج");
const URDU_DAL = ductusFor("د", "urdu-nastaliq")!;
const urduDalOutline = naskhOutline("د");
const URDU_RE = ductusFor("ر", "urdu-nastaliq")!;
const urduReOutline = naskhOutline("ر");
const URDU_WAW = ductusFor("و", "urdu-nastaliq")!;
const urduWawOutline = naskhOutline("و");
const URDU_SIN = ductusFor("س", "urdu-nastaliq")!;
const urduSinOutline = naskhOutline("س");
const URDU_SHIN = ductusFor("ش", "urdu-nastaliq")!;
const urduShinOutline = naskhOutline("ش");
const URDU_FE = ductusFor("ف", "urdu-nastaliq")!;
const urduFeOutline = naskhOutline("ف");
const URDU_QAF = ductusFor("ق", "urdu-nastaliq")!;
const urduQafOutline = naskhOutline("ق");
const URDU_TOE = ductusFor("ط", "urdu-nastaliq")!;
const urduToeOutline = naskhOutline("ط");
const URDU_BEH = ductusFor("ب", "urdu-nastaliq")!;
const urduBehOutline = naskhOutline("ب");
const URDU_PEH = ductusFor("پ", "urdu-nastaliq")!;
const urduPehOutline = naskhOutline("پ");
const URDU_TE = ductusFor("ت", "urdu-nastaliq")!;
const urduTeOutline = naskhOutline("ت");
const URDU_KAF = ductusFor("ک", "urdu-nastaliq")!;
const urduKafOutline = naskhOutline("ک");
const URDU_GAF = ductusFor("گ", "urdu-nastaliq")!;
const urduGafOutline = naskhOutline("گ");
const URDU_LAM = ductusFor("ل", "urdu-nastaliq")!;
const urduLamOutline = naskhOutline("ل");
const URDU_MIM = ductusFor("م", "urdu-nastaliq")!;
const urduMimOutline = naskhOutline("م");
const URDU_NUN = ductusFor("ن", "urdu-nastaliq")!;
const urduNunOutline = naskhOutline("ن");
const URDU_GHUNNA = ductusFor("ں", "urdu-nastaliq")!;
const urduGhunnaOutline = naskhOutline("ں");
const URDU_HE = ductusFor("ہ", "urdu-nastaliq")!;
const urduHeOutline = naskhOutline("ہ");
const URDU_YE = ductusFor("ی", "urdu-nastaliq")!;
const urduYeOutline = naskhOutline("ی");
const URDU_BARI_YE = ductusFor("ے", "urdu-nastaliq")!;
const urduBariYeOutline = naskhOutline("ے");
const PERSIAN_BEH = DUCTUS["ب"];
const persianBehOutline = naskhOutline("ب");
const PERSIAN_PEH = DUCTUS["پ"];
const persianPehOutline = naskhOutline("پ");
const PERSIAN_TEH = DUCTUS["ت"];
const persianTehOutline = naskhOutline("ت");
const PERSIAN_DAL = DUCTUS["د"];
const persianDalOutline = naskhOutline("د");
const PERSIAN_RA = ductusFor("ر", "perso-arabic")!;
const persianRaOutline = naskhOutline("ر");
const PERSIAN_SIN = DUCTUS["س"];
const persianSinOutline = naskhOutline("س");
const PERSIAN_SHIN = ductusFor("ش", "perso-arabic")!;
const persianShinOutline = naskhOutline("ش");
const PERSIAN_FEH = ductusFor("ف", "perso-arabic")!;
const persianFehOutline = naskhOutline("ف");
const PERSIAN_QAF = ductusFor("ق", "perso-arabic")!;
const persianQafOutline = naskhOutline("ق");
const PERSIAN_TAH = ductusFor("ط", "perso-arabic")!;
const persianTahOutline = naskhOutline("ط");
const PERSIAN_GAF = ductusFor("گ", "perso-arabic")!;
const persianGafOutline = naskhOutline("گ");
const PERSIAN_ZAY = ductusFor("ز", "perso-arabic")!;
const persianZayOutline = naskhOutline("ز");
const URDU_ZE = ductusFor("ز", "urdu-nastaliq")!;
const urduZeOutline = naskhOutline("ز");
const PERSIAN_LAM = DUCTUS["ل"];
const persianLamOutline = naskhOutline("ل");
const PERSIAN_MIM = DUCTUS["م"];
const persianMimOutline = naskhOutline("م");
const PERSIAN_NUN = DUCTUS["ن"];
const persianNunOutline = naskhOutline("ن");
const PERSIAN_WAW = DUCTUS["و"];
const persianWawOutline = naskhOutline("و");
const PERSIAN_HEH = DUCTUS["ه"];
const persianHehOutline = naskhOutline("ه");
const PERSIAN_YEH = ductusFor("ی", "perso-arabic")!;
const persianYehOutline = naskhOutline("ی");

beforeAll(() => {
  expect(ductusFor("ا")?.glyph).toBe("ا");
  expect(ductusFor("ب")?.glyph).toBe("ب");
  expect(ductusFor("ت")?.glyph).toBe("ت");
  expect(ductusFor("د")?.glyph).toBe("د");
  expect(ductusFor("س")?.glyph).toBe("س");
  expect(ductusFor("ل")?.glyph).toBe("ل");
  expect(ductusFor("م")?.glyph).toBe("م");
  expect(ductusFor("ن")?.glyph).toBe("ن");
  expect(ductusFor("و")?.glyph).toBe("و");
  expect(ductusFor("ه")?.glyph).toBe("ه");
  expect(ductusFor("ا", "arabic")?.glyph).toBe("ا");
  expect(ductusFor("ب", "arabic")?.glyph).toBe("ب");
  expect(ductusFor("ث", "arabic")?.glyph).toBe("ث");
  expect(ductusFor("س", "arabic")?.glyph).toBe("س");
  expect(ductusFor("ش", "arabic")?.glyph).toBe("ش");
  expect(ductusFor("ص", "arabic")?.glyph).toBe("ص");
  expect(ductusFor("ض", "arabic")?.glyph).toBe("ض");
  expect(ductusFor("ع", "arabic")?.glyph).toBe("ع");
  expect(ductusFor("ك", "arabic")?.glyph).toBe("ك");
  expect(ductusFor("ل", "arabic")?.glyph).toBe("ل");
  expect(ductusFor("ه", "arabic")?.glyph).toBe("ه");
  expect(ductusFor("و", "arabic")?.glyph).toBe("و");
  expect(ductusFor("ي", "arabic")?.glyph).toBe("ي");
  expect(ductusFor("ا", "urdu-nastaliq")?.glyph).toBe("ا");
  expect(ductusFor("ج", "urdu-nastaliq")?.glyph).toBe("ج");
  expect(ductusFor("خ", "urdu-nastaliq")?.glyph).toBe("خ");
  expect(ductusFor("خ", "perso-arabic")?.glyph).toBe("خ");
  expect(ductusFor("د", "urdu-nastaliq")?.glyph).toBe("د");
  expect(ductusFor("ج", "perso-arabic")).toBeUndefined();
  expect(ductusFor("ر", "urdu-nastaliq")?.glyph).toBe("ر");
  expect(ductusFor("ر", "perso-arabic")?.glyph).toBe("ر");
  expect(ductusFor("و", "urdu-nastaliq")?.glyph).toBe("و");
  expect(ductusFor("س", "urdu-nastaliq")?.glyph).toBe("س");
  expect(ductusFor("ش", "urdu-nastaliq")?.glyph).toBe("ش");
  expect(ductusFor("ش", "perso-arabic")?.glyph).toBe("ش");
  expect(ductusFor("ق", "urdu-nastaliq")?.glyph).toBe("ق");
  expect(ductusFor("ق", "perso-arabic")?.glyph).toBe("ق");
  expect(ductusFor("ط", "urdu-nastaliq")?.glyph).toBe("ط");
  expect(ductusFor("ط", "perso-arabic")?.glyph).toBe("ط");
  expect(ductusFor("ک", "urdu-nastaliq")?.glyph).toBe("ک");
  expect(ductusFor("ک", "perso-arabic")?.glyph).toBe("ک");
  expect(ductusFor("ل", "urdu-nastaliq")?.glyph).toBe("ل");
  expect(ductusFor("ل", "perso-arabic")?.glyph).toBe("ل");
  expect(ductusFor("م", "urdu-nastaliq")?.glyph).toBe("م");
  expect(ductusFor("م", "perso-arabic")?.glyph).toBe("م");
  expect(ductusFor("ن", "urdu-nastaliq")?.glyph).toBe("ن");
  expect(ductusFor("ن", "perso-arabic")?.glyph).toBe("ن");
  expect(ductusFor("ں", "urdu-nastaliq")?.glyph).toBe("ں");
  expect(ductusFor("ہ", "urdu-nastaliq")?.glyph).toBe("ہ");
  expect(ductusFor("ی", "urdu-nastaliq")?.glyph).toBe("ی");
  expect(ductusFor("ے", "urdu-nastaliq")?.glyph).toBe("ے");
  expect(ductusFor("پ", "perso-arabic")?.script).toBe("perso-arabic");
  expect(ductusFor("پ", "urdu-nastaliq")?.script).toBe("urdu-nastaliq");
});

it("keeps the shared Arabic, Persian, and Urdu خ independently addressable", () => {
  const arabic = ductusFor("خ", "arabic");
  const persian = ductusFor("خ", "perso-arabic");
  const urdu = ductusFor("خ", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu د independently addressable", () => {
  const arabic = ductusFor("د", "arabic");
  const persian = ductusFor("د", "perso-arabic");
  const urdu = ductusFor("د", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu ر independently addressable", () => {
  const arabic = ductusFor("ر", "arabic");
  const persian = ductusFor("ر", "perso-arabic");
  const urdu = ductusFor("ر", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu و independently addressable", () => {
  const arabic = ductusFor("و", "arabic");
  const persian = ductusFor("و", "perso-arabic");
  const urdu = ductusFor("و", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu ا independently addressable", () => {
  const arabic = ductusFor("ا", "arabic");
  const persian = ductusFor("ا", "perso-arabic");
  const urdu = ductusFor("ا", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(arabic?.source.url).not.toBe(persian?.source.url);
  expect(arabic?.source.url).not.toBe(urdu?.source.url);
  expect(persian?.source.url).not.toBe(urdu?.source.url);
});

it("keeps the shared Arabic, Persian, and Urdu ب independently addressable", () => {
  const arabic = ductusFor("ب", "arabic");
  const persian = ductusFor("ب", "perso-arabic");
  const urdu = ductusFor("ب", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu س independently addressable", () => {
  const arabic = ductusFor("س", "arabic");
  const persian = ductusFor("س", "perso-arabic");
  const urdu = ductusFor("س", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(arabic?.source.url).not.toBe(persian?.source.url);
  expect(arabic?.source.url).not.toBe(urdu?.source.url);
  expect(persian?.source.url).not.toBe(urdu?.source.url);
});

it("keeps the shared Arabic, Persian, and Urdu ش independently addressable", () => {
  const arabic = ductusFor("ش", "arabic");
  const persian = ductusFor("ش", "perso-arabic");
  const urdu = ductusFor("ش", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(
    new Set([arabic?.source.url, persian?.source.url, urdu?.source.url]).size,
  ).toBe(3);
});

it("keeps the shared Arabic, Persian, and Urdu م independently addressable", () => {
  const arabic = ductusFor("م", "arabic");
  const persian = ductusFor("م", "perso-arabic");
  const urdu = ductusFor("م", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(arabic?.source.url).not.toBe(persian?.source.url);
  expect(arabic?.source.url).not.toBe(urdu?.source.url);
  expect(persian?.source.url).not.toBe(urdu?.source.url);
});

it("keeps the shared Arabic, Persian, and Urdu ن independently addressable", () => {
  const arabic = ductusFor("ن", "arabic");
  const persian = ductusFor("ن", "perso-arabic");
  const urdu = ductusFor("ن", "urdu-nastaliq");
  expect(arabic?.script).toBe("arabic");
  expect(persian?.script).toBe("perso-arabic");
  expect(urdu?.script).toBe("urdu-nastaliq");
  expect(arabic?.source.url).not.toBe(persian?.source.url);
  expect(arabic?.source.url).not.toBe(urdu?.source.url);
  expect(persian?.source.url).not.toBe(urdu?.source.url);
});

describe("Persian ا — the first cited right-to-left-script filmstrip", () => {
  const steps = ductusSteps(PERSIAN_ALEF);
  const strip = ductusFilmstrip(PERSIAN_ALEF, persianAlefOutline);

  it("keeps the source's top-to-bottom stem in one pen-down run", () => {
    expect(steps).toHaveLength(1);
    expect(steps[0].label).toBe("down");
    expect(steps[0].startsAfterLift).toBe(false);
    expect(steps[0].strokeIndex).toBe(0);
    const path = PERSIAN_ALEF.strokes[0].segments[0].path;
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("reports one movement with no pen lift", () => {
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the vendored Noto Naskh outline behind the complete path", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianAlefOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_ALEF.strokes[0], 1));
  });
});

describe("Arabic ا — an independent, script-scoped filmstrip", () => {
  const steps = ductusSteps(ARABIC_ALEF);
  const strip = ductusFilmstrip(ARABIC_ALEF, arabicAlefOutline);

  it("shows one downward movement with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual(["down"]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(ARABIC_ALEF.strokes[0].segments[0].path[0].y).toBeGreaterThan(
      ARABIC_ALEF.strokes[0].segments[0].path.at(-1)!.y,
    );
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the vendored Noto Naskh outline behind the sourced path", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicAlefOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_ALEF.strokes[0], 1));
  });
});

describe("Arabic ب — a script-scoped bowl-and-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_BAA);
  const strip = ductusFilmstrip(ARABIC_BAA, arabicBaaOutline);

  it("shows the right-to-left bowl before the lifted dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(ARABIC_BAA.strokes[0].segments[0].path[0].x).toBeGreaterThan(
      ARABIC_BAA.strokes[0].segments[0].path.at(-1)!.x,
    );
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicBaaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(ARABIC_BAA.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_BAA.strokes[1], 1));
  });
});

describe("Arabic ت — a script-scoped bowl-and-two-dots filmstrip", () => {
  const steps = ductusSteps(ARABIC_TAA);
  const strip = ductusFilmstrip(ARABIC_TAA, arabicTaaOutline);

  it("shows the shared right-to-left bowl before both separately lifted dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot above",
      "lift again and place the right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(ARABIC_TAA.strokes[0].segments[0].path[0].x).toBeGreaterThan(
      ARABIC_TAA.strokes[0].segments[0].path.at(-1)!.x,
    );
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl and left dot in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicTaaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(ARABIC_TAA.strokes[0], 1),
      penPathD(ARABIC_TAA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_TAA.strokes[2], 1));
  });
});

describe("Arabic ث — a body-first bowl-and-three-dots filmstrip", () => {
  const steps = ductusSteps(ARABIC_THAA);
  const strip = ductusFilmstrip(ARABIC_THAA, arabicThaaOutline);

  it("shows the right-to-left bowl before three separately lifted upper dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the lower-left dot above",
      "lift again and place the lower-right dot",
      "lift a third time and place the centred upper dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Naskh outline and preserves all earlier runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicThaaOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual(
      ARABIC_THAA.strokes.slice(0, 3).map((stroke) => penPathD(stroke, 1)),
    );
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_THAA.strokes[3], 1));
  });
});

describe("Arabic ج — a body-first hook-and-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_JEEM);
  const strip = ductusFilmstrip(ARABIC_JEEM, arabicJeemOutline);

  it("keeps the sourced head and bowl in one stroke before the lifted dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short upper head from left to right",
      "continue down and around the bowl",
      "lift once, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicJeemOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(ARABIC_JEEM.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_JEEM.strokes[1], 1));
  });
});

describe("Arabic ح — a stem-first, dotless filmstrip", () => {
  const steps = ductusSteps(ARABIC_HAA);
  const strip = ductusFilmstrip(ARABIC_HAA, arabicHaaOutline);

  it("keeps the short stem separate from the restarted head-and-bowl run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short left stem downward",
      "lift once and restart near the stem's top",
      "continue down and around the bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the separate stem in the final bowl frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicHaaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(ARABIC_HAA.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_HAA.strokes[1], 1));
  });
});

describe("Persian and Urdu ح — independently sourced body-first filmstrips", () => {
  it.each([
    ["Persian", PERSIAN_HAH],
    ["Urdu", URDU_BARI_HE],
  ])("keeps %s head and bowl in one uninterrupted run", (_name, letter) => {
    const steps = ductusSteps(letter);
    const strip = ductusFilmstrip(letter, arabicHaaOutline);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicHaaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(letter.strokes[0], 2));
  });
});

describe("Arabic خ — a body-first hook-and-upper-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_KHAA);
  const strip = ductusFilmstrip(ARABIC_KHAA, arabicKhaaOutline);

  it("keeps the sourced head and bowl in one stroke before the lifted upper dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short upper head from left to right",
      "continue down and around the bowl",
      "lift once, then place the dot above",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final upper-dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicKhaaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(ARABIC_KHAA.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_KHAA.strokes[1], 1));
  });
});

describe("Arabic د — an unbroken shoulder-and-baseline filmstrip", () => {
  const steps = ductusSteps(ARABIC_DAAL);
  const strip = ductusFilmstrip(ARABIC_DAAL, arabicDaalOutline);

  it("keeps the sourced descent and leftward baseline turn in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend diagonally down and right through the curved shoulder",
      "turn left along the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicDaalOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_DAAL.strokes[0], 1));
  });
});

describe("Arabic ر — an unbroken descending-curve filmstrip", () => {
  const steps = ductusSteps(ARABIC_RAA);
  const strip = ductusFilmstrip(ARABIC_RAA, arabicRaaOutline);

  it("keeps the sourced descent and leftward lower curve in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend through the short stroke",
      "sweep left through the lower curve without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicRaaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_RAA.strokes[0], 1));
  });
});

describe("Arabic س — an unbroken teeth-and-bowl filmstrip", () => {
  const steps = ductusSteps(ARABIC_SEEN);
  const strip = ductusFilmstrip(ARABIC_SEEN, arabicSeenOutline);

  it("keeps the sourced three teeth and final bowl in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicSeenOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_SEEN.strokes[0], 1));
  });
});

describe("Arabic ش — a complete س body followed by three dots", () => {
  const steps = ductusSteps(ARABIC_SHIIN);
  const strip = ductusFilmstrip(ARABIC_SHIIN, arabicShiinOutline);

  it("shows the body first, then lower-left, lower-right, and upper dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again, then place the lower-right dot",
      "lift a third time, then place the centered upper dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws Noto Naskh and preserves completed strokes during the upper dot", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicShiinOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(3);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_SHIIN.strokes[0], 1),
      penPathD(ARABIC_SHIIN.strokes[1], 1),
      penPathD(ARABIC_SHIIN.strokes[2], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_SHIIN.strokes[3], 1));
  });
});

describe("Arabic ص — an oval and shoulder followed by a lifted bowl", () => {
  const steps = ductusSteps(ARABIC_SAAD);
  const strip = ductusFilmstrip(ARABIC_SAAD, arabicSaadOutline);

  it("shows the joined oval and shoulder before restarting for the bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "close the oval clockwise from its lower-left junction",
      "turn left and rise into the short shoulder without lifting",
      "lift, restart at the baseline junction, and sweep through the trailing bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Naskh and preserves the completed body during the bowl", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicSaadOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(ARABIC_SAAD.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_SAAD.strokes[1], 1));
  });
});

describe("Arabic ض — the ص body followed by a separately lifted dot", () => {
  const steps = ductusSteps(ARABIC_DAAD);
  const strip = ductusFilmstrip(ARABIC_DAAD, arabicDaadOutline);

  it("shows the two body runs before placing the upper dot last", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "close the oval clockwise from its lower-left junction",
      "turn left and rise into the short shoulder without lifting",
      "lift, restart at the baseline junction, and sweep through the trailing bowl",
      "lift again, then place the upper dot last",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws Noto Naskh and preserves both completed body strokes during the dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicDaadOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_DAAD.strokes[0], 1),
      penPathD(ARABIC_DAAD.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_DAAD.strokes[2], 1));
  });
});

describe("Arabic ع — an open head flowing into an unbroken lower bowl", () => {
  const steps = ductusSteps(ARABIC_AYN);
  const strip = ductusFilmstrip(ARABIC_AYN, arabicAynOutline);

  it("shows both sourced movements in one unbroken stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left from the upper-right tip and shape the open head",
      "continue down and around the lower bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicAynOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_AYN.strokes[0], 1));
  });
});

describe("Arabic ك — a joined outer body and separately restarted inner arm", () => {
  const steps = ductusSteps(ARABIC_KAF);
  const strip = ductusFilmstrip(ARABIC_KAF, arabicKafOutline);

  it("shows three sourced movements across two pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the main upright",
      "turn left along the baseline without lifting",
      "lift, then draw the inner arm from upper right down-left",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Naskh and keeps the completed outer body behind the inner arm", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicKafOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(ARABIC_KAF.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_KAF.strokes[1], 1));
  });
});

describe("Arabic ل — its upright continues through the leftward base bowl", () => {
  const steps = ductusSteps(ARABIC_LAM);
  const strip = ductusFilmstrip(ARABIC_LAM, arabicLamOutline);

  it("shows two sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the tall upright",
      "continue left through the base bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicLamOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_LAM.strokes[0], 1));
  });
});

describe("Arabic م — its closed head flows into the below-baseline tail", () => {
  const steps = ductusSteps(ARABIC_MEEM);
  const strip = ductusFilmstrip(ARABIC_MEEM, arabicMeemOutline);

  it("shows both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the small closed head in a tight circular movement",
      "continue down and left through the below-baseline tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicMeemOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_MEEM.strokes[0], 1));
  });
});

describe("Arabic ن — its deep bowl is followed by one centred upper dot", () => {
  const steps = ductusSteps(ARABIC_NOON);
  const strip = ductusFilmstrip(ARABIC_NOON, arabicNoonOutline);

  it("shows the body-first bowl and lifted dot in two frames", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep down and around the deep bowl from right to left",
      "lift, then place the dot above the bowl's midpoint",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and preserves the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicNoonOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(ARABIC_NOON.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_NOON.strokes[1], 1));
  });
});

describe("Arabic ه — its two counters flow into one leftward finish", () => {
  const steps = ductusSteps(ARABIC_HEH);
  const strip = ductusFilmstrip(ARABIC_HEH, arabicHehOutline);

  it("shows both counters and the baseline sweep without a lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve down-left and close the lower counter",
      "thread through the centre and close the upper-right counter without lifting",
      "sweep left along the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicHehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_HEH.strokes[0], 1));
  });
});

describe("Arabic و — its closed head flows directly into the leftward tail", () => {
  const steps = ductusSteps(ARABIC_WAW);
  const strip = ductusFilmstrip(ARABIC_WAW, arabicWawOutline);

  it("shows the sourced head and tail without a lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left from the lower-right junction and close the small head loop",
      "continue down and left through the tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicWawOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_WAW.strokes[0], 1));
  });
});

describe("Arabic ي — its independent bowl precedes the two lower dots", () => {
  const steps = ductusSteps(ARABIC_YAA);
  const strip = ductusFilmstrip(ARABIC_YAA, arabicYaaOutline);

  it("shows the sourced body and left-then-right dot order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper right into the independent bowl",
      "sweep left through the bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again, then place the lower-right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws Noto Naskh and keeps the completed body and first dot behind the second", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicYaaOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_YAA.strokes[0], 1),
      penPathD(ARABIC_YAA.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_YAA.strokes[2], 1));
  });
});

describe("Arabic ء — its upper head flows into the lower diagonal", () => {
  const steps = ductusSteps(ARABIC_HAMZA);
  const strip = ductusFilmstrip(ARABIC_HAMZA, arabicHamzaOutline);

  it("shows the sourced one-stroke variant in two movements", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep counterclockwise through the c-shaped upper head",
      "continue through the lower diagonal toward the right without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(arabicHamzaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(ARABIC_HAMZA.strokes[0], 1));
  });
});

describe("Urdu ا — an independent, source-specific filmstrip", () => {
  const steps = ductusSteps(URDU_ALEF);
  const strip = ductusFilmstrip(URDU_ALEF, urduAlefOutline);

  it("shows one downward movement with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual(["down"]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(URDU_ALEF.strokes[0].segments[0].path[0].y).toBeGreaterThan(
      URDU_ALEF.strokes[0].segments[0].path.at(-1)!.y,
    );
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("uses the vendored Noto Naskh fallback outline", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduAlefOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_ALEF.strokes[0], 1));
  });
});

describe("Urdu ج — dot first, then one continuous pointed body", () => {
  const steps = ductusSteps(URDU_JIM);
  const strip = ductusFilmstrip(URDU_JIM, urduJimOutline);

  it("shows the sourced dot-first order and only one lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "place the dot below",
      "lift, then sweep left through the pointed hooked head",
      "continue down and around the bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and keeps the completed dot through the body", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduJimOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(URDU_JIM.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_JIM.strokes[1], 1));
  });
});

describe("Urdu د — its folded shoulder turns left without a lift", () => {
  const steps = ductusSteps(URDU_DAL);
  const strip = ductusFilmstrip(URDU_DAL, urduDalOutline);

  it("shows both sourced movements in one unbroken stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the independent form's upper tip and descend through the folded shoulder",
      "turn left along the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduDalOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_DAL.strokes[0], 1));
  });
});

describe("Urdu ر — one downward line that continues left", () => {
  const steps = ductusSteps(URDU_RE);
  const strip = ductusFilmstrip(URDU_RE, urduReOutline);

  it("shows both sourced movements in one unbroken stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the downward line",
      "continue curving to the left",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduReOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_RE.strokes[0], 1));
  });
});

describe("Urdu و — its looped head flows into the leftward tail", () => {
  const steps = ductusSteps(URDU_WAW);
  const strip = ductusFilmstrip(URDU_WAW, urduWawOutline);

  it("shows both sourced movements in one unbroken stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the independent wāw's looped head",
      "continue down and left through the tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduWawOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_WAW.strokes[0], 1));
  });
});

describe("Urdu س — three close teeth flowing into one final bowl", () => {
  const steps = ductusSteps(URDU_SIN);
  const strip = ductusFilmstrip(URDU_SIN, urduSinOutline);

  it("shows both sourced movements in one right-to-left pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduSinOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_SIN.strokes[0], 1));
  });
});

describe("Urdu ش — a complete س body followed by three dots", () => {
  const steps = ductusSteps(URDU_SHIN);
  const strip = ductusFilmstrip(URDU_SHIN, urduShinOutline);

  it("shows the body first, then lower-left, lower-right, and upper dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again, then place the lower-right dot",
      "lift a third time, then place the centered upper dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("uses Noto Naskh and preserves all completed strokes during the upper dot", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduShinOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(3);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(URDU_SHIN.strokes[0], 1),
      penPathD(URDU_SHIN.strokes[1], 1),
      penPathD(URDU_SHIN.strokes[2], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_SHIN.strokes[3], 1));
  });
});

describe("Persian and Urdu ف — joined head and body before the upper dot", () => {
  for (const [name, ductus, glyphOutline] of [
    ["Persian", PERSIAN_FEH, persianFehOutline],
    ["Urdu", URDU_FE, urduFeOutline],
  ] as const) {
    it(`${name} renders two joined body movements before the lifted dot`, () => {
      const steps = ductusSteps(ductus);
      const strip = ductusFilmstrip(ductus, glyphOutline);
      expect(steps.map((step) => step.startsAfterLift)).toEqual([
        false,
        false,
        true,
      ]);
      expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
      expect(strip.frames).toHaveLength(3);
      expect(strip.penLifts).toBe(1);
      expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
      const paths = byTag(strip.frames[2], "path");
      expect(
        paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
      ).toBe(glyphOutline.path);
      expect(
        paths.filter((path) => path.attrs.class === "ductus__done")[0].attrs.d,
      ).toBe(penPathD(ductus.strokes[0], 1));
      expect(
        paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
      ).toBe(penPathD(ductus.strokes[1], 1));
    });
  }
});

describe("Persian and Urdu ق — joined head and deep bowl before two upper dots", () => {
  for (const [name, ductus, glyphOutline] of [
    ["Persian", PERSIAN_QAF, persianQafOutline],
    ["Urdu", URDU_QAF, urduQafOutline],
  ] as const) {
    it(`${name} renders two joined body movements before two lifted dots`, () => {
      const steps = ductusSteps(ductus);
      const strip = ductusFilmstrip(ductus, glyphOutline);
      expect(steps.map((step) => step.startsAfterLift)).toEqual([
        false,
        false,
        true,
        true,
      ]);
      expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
      expect(strip.frames).toHaveLength(4);
      expect(strip.penLifts).toBe(2);
      expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
      const paths = byTag(strip.frames[3], "path");
      expect(
        paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
      ).toBe(glyphOutline.path);
      expect(
        paths.filter((path) => path.attrs.class === "ductus__done"),
      ).toHaveLength(2);
      expect(
        paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
      ).toBe(penPathD(ductus.strokes[2], 1));
    });
  }
});

describe("Persian and Urdu ط — closed body before the lifted upright", () => {
  for (const [name, ductus, glyphOutline] of [
    ["Persian", PERSIAN_TAH, persianTahOutline],
    ["Urdu", URDU_TOE, urduToeOutline],
  ] as const) {
    it(`${name} renders two joined body movements before the lifted upright`, () => {
      const steps = ductusSteps(ductus);
      const strip = ductusFilmstrip(ductus, glyphOutline);
      expect(steps.map((step) => step.startsAfterLift)).toEqual([
        false,
        false,
        true,
      ]);
      expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
      expect(strip.frames).toHaveLength(3);
      expect(strip.penLifts).toBe(1);
      expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
    });
  }
});

describe("Urdu ک — a main-line body followed by its long slash", () => {
  const steps = ductusSteps(URDU_KAF);
  const strip = ductusFilmstrip(URDU_KAF, urduKafOutline);

  it("shows the stem and flatter hooked bowl before the separately lifted slash", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the independent stem downward",
      "flow right to left through the flatter bowl and finish with the hook without lifting",
      "lift, then draw the long slash down from the upper right toward the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and preserves the completed body during the slash", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduKafOutline.path);
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(URDU_KAF.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_KAF.strokes[1], 1));
  });
});

describe("Urdu گ — the kāf-family body followed by two slashes", () => {
  const steps = ductusSteps(URDU_GAF);
  const strip = ductusFilmstrip(URDU_GAF, urduGafOutline);

  it("places lifts before the long and short slashes", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
  });

  it("reports four movements in three strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });
});

describe("Persian گ — the scoped kāf-family body followed by two slashes", () => {
  const steps = ductusSteps(PERSIAN_GAF);
  const strip = ductusFilmstrip(PERSIAN_GAF, persianGafOutline);

  it("places lifts before the long and short slashes", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
    ]);
  });

  it("reports four movements in three strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });
});

describe("Persian and Urdu ز — a joined re-series body before the lifted dot", () => {
  for (const [name, ductus, glyphOutline] of [
    ["Persian", PERSIAN_ZAY, persianZayOutline],
    ["Urdu", URDU_ZE, urduZeOutline],
  ] as const) {
    it(`${name} renders two joined body movements before the lifted dot`, () => {
      const steps = ductusSteps(ductus);
      const strip = ductusFilmstrip(ductus, glyphOutline);
      expect(steps.map((step) => step.startsAfterLift)).toEqual([
        false,
        false,
        true,
      ]);
      expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
      expect(strip.frames).toHaveLength(3);
      expect(strip.penLifts).toBe(1);
      expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
    });
  }
});

describe("Urdu ل — its upright continues through a below-baseline bowl", () => {
  const steps = ductusSteps(URDU_LAM);
  const strip = ductusFilmstrip(URDU_LAM, urduLamOutline);

  it("keeps the downward upright and leftward bowl in one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the tall independent upright downward",
      "continue below the baseline through the leftward bowl and back up without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduLamOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_LAM.strokes[0], 1));
  });
});

describe("Urdu م — its round head flows into the below-baseline tail", () => {
  const steps = ductusSteps(URDU_MIM);
  const strip = ductusFilmstrip(URDU_MIM, urduMimOutline);

  it("keeps the head and tail in one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the round head",
      "continue down the tail below the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const head = URDU_MIM.strokes[0].segments[0].path;
    const tail = URDU_MIM.strokes[0].segments[1].path;
    expect(tail[0]).toEqual(head.at(-1));
    expect(Math.min(...tail.map((point) => point.y))).toBeLessThan(0);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduMimOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_MIM.strokes[0], 1));
  });
});

describe("Urdu ن — its below-baseline bowl precedes the lifted dot", () => {
  const steps = ductusSteps(URDU_NUN);
  const strip = ductusFilmstrip(URDU_NUN, urduNunOutline);

  it("keeps the bowl together, then marks the sourced lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent bowl right to left below the baseline",
      "lift, then place the dot near the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = URDU_NUN.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduNunOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(URDU_NUN.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_NUN.strokes[1], 1));
  });
});

describe("Urdu ں — its dotless bowl is one unbroken stroke", () => {
  const steps = ductusSteps(URDU_GHUNNA);
  const strip = ductusFilmstrip(URDU_GHUNNA, urduGhunnaOutline);

  it("shows the sourced dotless nūn bowl with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent dotless bowl right to left below the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced bowl", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduGhunnaOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_GHUNNA.strokes[0], 1));
  });
});

describe("Urdu ہ — its independent teardrop is one unbroken loop", () => {
  const steps = ductusSteps(URDU_HE);
  const strip = ductusFilmstrip(URDU_HE, urduHeOutline);

  it("shows one sourced counterclockwise loop with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "loop the independent teardrop counterclockwise without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the Noto Naskh outline and the complete sourced loop", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduHeOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_HE.strokes[0], 1));
  });
});

describe("Urdu ی — its independent S and bowl are one unbroken stroke", () => {
  const steps = ductusSteps(URDU_YE);
  const strip = ductusFilmstrip(URDU_YE, urduYeOutline);

  it("shows the sourced dotless S and bowl with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper right through the independent S curve",
      "continue left around the below-baseline bowl and finish at its rising tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced stroke", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduYeOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_YE.strokes[0], 1));
  });
});

describe("Urdu ے — its broad bowl folds backward in one unbroken stroke", () => {
  const steps = ductusSteps(URDU_BARI_YE);
  const strip = ductusFilmstrip(URDU_BARI_YE, urduBariYeOutline);

  it("shows the sourced upper sweep, curl, and lower fold with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper right and sweep left across the broad bowl",
      "curl back underneath at the far left without lifting",
      "continue right along the lower fold without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced fold", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduBariYeOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_BARI_YE.strokes[0], 1));
  });
});

describe("Persian ب — a right-to-left bowl followed by its dot", () => {
  const steps = ductusSteps(PERSIAN_BEH);
  const strip = ductusFilmstrip(PERSIAN_BEH, persianBehOutline);

  it("keeps the bowl in one right-to-left run, then marks the sourced lift", () => {
    expect(steps).toHaveLength(2);
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = PERSIAN_BEH.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and preserves the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianBehOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(PERSIAN_BEH.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_BEH.strokes[1], 1));
  });
});

describe("Persian پ — the shared bowl followed by three separate dots", () => {
  const steps = ductusSteps(PERSIAN_PEH);
  const strip = ductusFilmstrip(PERSIAN_PEH, persianPehOutline);

  it("keeps the bowl in one run, then preserves all three sourced dot lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot below",
      "lift again and place the right dot below",
      "lift again and place the lower-center dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(PERSIAN_PEH.strokes[1].segments[0].path[0].x).toBeLessThan(
      PERSIAN_PEH.strokes[2].segments[0].path[0].x,
    );
    expect(PERSIAN_PEH.strokes[3].segments[0].path[0].y).toBeLessThan(
      PERSIAN_PEH.strokes[1].segments[0].path[0].y,
    );
  });

  it("reports four movements separated by three pen lifts", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the Noto Naskh outline and preserves prior dots in the final frame", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianPehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(3);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_PEH.strokes[3], 1));
  });
});

describe("Urdu ب — the independent be-series bowl followed by its lower dot", () => {
  const steps = ductusSteps(URDU_BEH);
  const strip = ductusFilmstrip(URDU_BEH, urduBehOutline);

  it("keeps the main line in one run, then places the dot after one lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the single dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and keeps its Urdu source separate", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduBehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(1);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_BEH.strokes[1], 1));
    expect(URDU_BEH.source.url).not.toBe(ARABIC_BAA.source.url);
    expect(URDU_BEH.source.url).not.toBe(PERSIAN_BEH.source.url);
  });
});

describe("Urdu پ — the independent be-series bowl followed by its dot triangle", () => {
  const steps = ductusSteps(URDU_PEH);
  const strip = ductusFilmstrip(URDU_PEH, urduPehOutline);

  it("keeps the bowl in one run, then places the two upper dots before the lower center", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the lower-left dot nearer the main line",
      "after another lift, place the lower-right dot nearer the main line",
      "after a third lift, place the lower-center dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(URDU_PEH.strokes[1].segments[0].path[0].x).toBeLessThan(
      URDU_PEH.strokes[2].segments[0].path[0].x,
    );
    expect(URDU_PEH.strokes[3].segments[0].path[0].y).toBeLessThan(
      URDU_PEH.strokes[1].segments[0].path[0].y,
    );
  });

  it("reports four movements separated by three pen lifts", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the Noto Naskh outline and keeps its Urdu source separate", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduPehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(3);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(URDU_PEH.strokes[3], 1));
    expect(URDU_PEH.source.url).not.toBe(PERSIAN_PEH.source.url);
  });
});

describe("Urdu ت — the independent be-series bowl followed by two upper dots", () => {
  const steps = ductusSteps(URDU_TE);
  const strip = ductusFilmstrip(URDU_TE, urduTeOutline);

  it("keeps the bowl in one run, then preserves both sourced dot lifts", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(URDU_TE.strokes[1].segments[0].path[0].x).toBeLessThan(
      URDU_TE.strokes[2].segments[0].path[0].x,
    );
  });

  it("draws the Noto Naskh outline across three source-backed frames", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(urduTeOutline.path);
  });
});

describe("Persian ت — the shared bowl followed by two separate dots", () => {
  const steps = ductusSteps(PERSIAN_TEH);
  const strip = ductusFilmstrip(PERSIAN_TEH, persianTehOutline);

  it("keeps the bowl in one run, then preserves both sourced dot lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot above",
      "lift again and place the right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    const bowl = PERSIAN_TEH.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(PERSIAN_TEH.strokes[1].segments[0].path[0].x).toBeLessThan(
      PERSIAN_TEH.strokes[2].segments[0].path[0].x,
    );
  });

  it("reports three movements separated by two pen lifts", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("retains the bowl and left dot while the right dot is placed", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianTehOutline.path);
    expect(
      paths
        .filter((path) => path.attrs.class === "ductus__done")
        .map((path) => path.attrs.d),
    ).toEqual([
      penPathD(PERSIAN_TEH.strokes[0], 1),
      penPathD(PERSIAN_TEH.strokes[1], 1),
    ]);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_TEH.strokes[2], 1));
  });
});

describe("Persian د — its folded shoulder turns left without a lift", () => {
  const steps = ductusSteps(PERSIAN_DAL);
  const strip = ductusFilmstrip(PERSIAN_DAL, persianDalOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend through the folded shoulder",
      "turn left along the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline in one continuous path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianDalOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_DAL.strokes[0], 1));
  });
});

describe("Persian ر — its short descent sweeps left without a lift", () => {
  const steps = ductusSteps(PERSIAN_RA);
  const strip = ductusFilmstrip(PERSIAN_RA, persianRaOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend through the short stroke",
      "without lifting, sweep left through the lower curve",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline in one continuous path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianRaOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_RA.strokes[0], 1));
  });
});

describe("Persian س — three teeth flowing into one final bowl", () => {
  const steps = ductusSteps(PERSIAN_SIN);
  const strip = ductusFilmstrip(PERSIAN_SIN, persianSinOutline);

  it("keeps both sourced movements in one right-to-left pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the three teeth from right to left",
      "flow into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const first = PERSIAN_SIN.strokes[0].segments[0].path[0];
    const last = PERSIAN_SIN.strokes[0].segments.at(-1)!.path.at(-1)!;
    expect(first.x).toBeGreaterThan(last.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianSinOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_SIN.strokes[0], 1));
  });
});

describe("Persian ش — one body followed by three dots", () => {
  const steps = ductusSteps(PERSIAN_SHIN);
  const strip = ductusFilmstrip(PERSIAN_SHIN, persianShinOutline);

  it("preserves the sourced body-first order and three dot lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the three teeth from right to left",
      "flow into the final bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again and place the lower-right dot",
      "lift again and place the centered upper dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
  });

  it("reports four strokes, three lifts, and five movements", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });
});

describe("Persian ل — its upright turns directly into the base curve", () => {
  const steps = ductusSteps(PERSIAN_LAM);
  const strip = ductusFilmstrip(PERSIAN_LAM, persianLamOutline);

  it("keeps both sourced movements in one descending pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upright downward",
      "turn into the base curve without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const first = PERSIAN_LAM.strokes[0].segments[0].path[0];
    const last = PERSIAN_LAM.strokes[0].segments.at(-1)!.path.at(-1)!;
    expect(first.y).toBeGreaterThan(last.y);
    expect(first.x).toBeGreaterThan(last.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianLamOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_LAM.strokes[0], 1));
  });
});

describe("Persian م — its round head flows into the descending tail", () => {
  const steps = ductusSteps(PERSIAN_MIM);
  const strip = ductusFilmstrip(PERSIAN_MIM, persianMimOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the round head",
      "continue down the tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const head = PERSIAN_MIM.strokes[0].segments[0].path;
    const tail = PERSIAN_MIM.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianMimOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_MIM.strokes[0], 1));
  });
});

describe("Persian ن — its bowl is followed by a separately placed dot", () => {
  const steps = ductusSteps(PERSIAN_NUN);
  const strip = ductusFilmstrip(PERSIAN_NUN, persianNunOutline);

  it("keeps the bowl in one right-to-left run, then marks the sourced lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the bowl from right to left",
      "lift, then place the dot above",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = PERSIAN_NUN.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and preserves the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianNunOutline.path);
    expect(
      paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d,
    ).toBe(penPathD(PERSIAN_NUN.strokes[0], 1));
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_NUN.strokes[1], 1));
  });
});

describe("Persian و — its small head flows into one leftward tail", () => {
  const steps = ductusSteps(PERSIAN_WAW);
  const strip = ductusFilmstrip(PERSIAN_WAW, persianWawOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the small head loop",
      "flow into the leftward tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const tail = PERSIAN_WAW.strokes[0].segments[1].path;
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianWawOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_WAW.strokes[0], 1));
  });
});

describe("Persian ه — its isolated looping body stays in one pen-down run", () => {
  const steps = ductusSteps(PERSIAN_HEH);
  const strip = ductusFilmstrip(PERSIAN_HEH, persianHehOutline);

  it("keeps the sourced looping body in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "loop the isolated body and finish left without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
  });

  it("reports one movement in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianHehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_HEH.strokes[0], 1));
  });
});

describe("Persian ی — its freehand S and bowl stay in one pen-down run", () => {
  const steps = ductusSteps(PERSIAN_YEH);
  const strip = ductusFilmstrip(PERSIAN_YEH, persianYehOutline);

  it("keeps both sourced movements continuous", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left from the upper right and descend through the S curve",
      "continue around the below-baseline bowl and finish at its rising tip without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline and completes the same sourced stroke", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(
      paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d,
    ).toBe(persianYehOutline.path);
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done"),
    ).toHaveLength(0);
    expect(
      paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d,
    ).toBe(penPathD(PERSIAN_YEH.strokes[0], 1));
  });
});

// ---------------------------------------------------------------------------
// Generic multi-stroke edge cases still use a synthetic ductus so the test can
// vary stroke counts independently of curriculum data. Nothing in this fixture
// is ever shown to a learner, and no letter enters DUCTUS without a citation.
// ---------------------------------------------------------------------------
