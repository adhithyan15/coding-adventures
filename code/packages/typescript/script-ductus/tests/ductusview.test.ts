// ---------------------------------------------------------------------------
// ductusview.test.ts — does the picture say what the data says?
// ---------------------------------------------------------------------------
//
// `strokes.test.ts` already proves the pen path is TRUE: every point on real
// ink, every join tight, the whole letter traced. That leaves exactly one thing
// for this file to prove — that the picture is a faithful rendering of that
// already-true data. Concretely:
//
//   • the glyph outline drawn is the FONT's path, character for character;
//   • the letter and the pen share ONE flip, so they cannot disagree on "up";
//   • the frames advance — each shows strictly more of the stroke than the last;
//   • a letter with no authored ductus produces nothing, rather than a guess;
//   • anything that reaches an attribute is escaped.
// ---------------------------------------------------------------------------

import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseFont, boundsOf } from "../src/truetype";
import { DUCTUS, penPathD, type LetterDuctus } from "../src/strokes";
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
} from "../src/ductusview";

const FONT_DIR = resolve(__dirname, "../../../../learning/human-languages/_fonts");
const load = (name: string) => {
  const b = readFileSync(resolve(FONT_DIR, name));
  return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;
};

// The glyph shape comes from the shipped font, exactly as the app gets it.
// Nothing in this file draws a letter.
const tamilOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansTamil-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const naskhOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoNaskhArabic-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const hebrewOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansHebrew-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const chineseOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansSC-Subset.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const devanagariOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansDevanagari-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const cyrillicOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansCyrillic-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const gujaratiOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansGujarati-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const MA = DUCTUS["ம"];
const outline = tamilOutline("ம");
const A = DUCTUS["அ"];
const aOutline = tamilOutline("அ");
const AA = DUCTUS["ஆ"];
const aaOutline = tamilOutline("ஆ");
const I = DUCTUS["இ"];
const iOutline = tamilOutline("இ");
const KA = DUCTUS["க"];
const kaOutline = tamilOutline("க");
const VA = DUCTUS["வ"];
const vaOutline = tamilOutline("வ");
const LA = DUCTUS["ல"];
const laOutline = tamilOutline("ல");
const RRA = DUCTUS["ற"];
const rraOutline = tamilOutline("ற");
const NNA = DUCTUS["ன"];
const nnaOutline = tamilOutline("ன");
const RETROFLEX_NNA = DUCTUS["ண"];
const retroflexNnaOutline = tamilOutline("ண");
const DENTAL_NA = DUCTUS["ந"];
const dentalNaOutline = tamilOutline("ந");
const CHINESE_REN = ductusFor("人", "chinese")!;
const chineseRenOutline = chineseOutline("人");
const CHINESE_PERSON_RADICAL = ductusFor("亻", "chinese")!;
const chinesePersonRadicalOutline = chineseOutline("亻");
const CHINESE_MOUTH = ductusFor("口", "chinese")!;
const chineseMouthOutline = chineseOutline("口");
const CHINESE_WOMAN = ductusFor("女", "chinese")!;
const chineseWomanOutline = chineseOutline("女");
const CHINESE_CHILD = ductusFor("子", "chinese")!;
const chineseChildOutline = chineseOutline("子");
const CHINESE_SUN = ductusFor("日", "chinese")!;
const chineseSunOutline = chineseOutline("日");
const CHINESE_SPEECH_RADICAL = ductusFor("讠", "chinese")!;
const chineseSpeechRadicalOutline = chineseOutline("讠");
const CHINESE_WATER_RADICAL = ductusFor("氵", "chinese")!;
const chineseWaterRadicalOutline = chineseOutline("氵");
const CHINESE_ROOF_RADICAL = ductusFor("宀", "chinese")!;
const chineseRoofRadicalOutline = chineseOutline("宀");
const CHINESE_YOU = ductusFor("你", "chinese")!;
const chineseYouOutline = chineseOutline("你");
const CHINESE_GOOD = ductusFor("好", "chinese")!;
const chineseGoodOutline = chineseOutline("好");
const CHINESE_I = ductusFor("我", "chinese")!;
const chineseIOutline = chineseOutline("我");
const CHINESE_BE = ductusFor("是", "chinese")!;
const chineseBeOutline = chineseOutline("是");
const CHINESE_NOT = ductusFor("不", "chinese")!;
const chineseNotOutline = chineseOutline("不");
const CHINESE_NAME = ductusFor("名", "chinese")!;
const chineseNameOutline = chineseOutline("名");
const CHINESE_CHARACTER = ductusFor("字", "chinese")!;
const chineseCharacterOutline = chineseOutline("字");
const CHINESE_THANK = ductusFor("谢", "chinese")!;
const chineseThankOutline = chineseOutline("谢");
const CHINESE_PLEASE = ductusFor("请", "chinese")!;
const chinesePleaseOutline = chineseOutline("请");
const CHINESE_AGAIN = ductusFor("再", "chinese")!;
const chineseAgainOutline = chineseOutline("再");
const CHINESE_SEE = ductusFor("见", "chinese")!;
const chineseSeeOutline = chineseOutline("见");
const CHINESE_WHAT = ductusFor("什", "chinese")!;
const chineseWhatOutline = chineseOutline("什");
const CHINESE_PARTICLE_ME = ductusFor("么", "chinese")!;
const chineseParticleMeOutline = chineseOutline("么");
const CHINESE_EARLY = ductusFor("早", "chinese")!;
const chineseEarlyOutline = chineseOutline("早");
const CHINESE_UP = ductusFor("上", "chinese")!;
const chineseUpOutline = chineseOutline("上");
const DEVANAGARI_A = ductusFor("अ", "devanagari")!;
const devanagariAOutline = devanagariOutline("अ");
const DEVANAGARI_AA = ductusFor("आ", "devanagari")!;
const devanagariAaOutline = devanagariOutline("आ");
const DEVANAGARI_I = ductusFor("इ", "devanagari")!;
const devanagariIOutline = devanagariOutline("इ");
const DEVANAGARI_II = ductusFor("ई", "devanagari")!;
const devanagariIiOutline = devanagariOutline("ई");
const DEVANAGARI_U = ductusFor("उ", "devanagari")!;
const devanagariUOutline = devanagariOutline("उ");
const DEVANAGARI_UU = ductusFor("ऊ", "devanagari")!;
const devanagariUuOutline = devanagariOutline("ऊ");
const DEVANAGARI_E = ductusFor("ए", "devanagari")!;
const devanagariEOutline = devanagariOutline("ए");
const DEVANAGARI_AI = ductusFor("ऐ", "devanagari")!;
const devanagariAiOutline = devanagariOutline("ऐ");
const DEVANAGARI_O = ductusFor("ओ", "devanagari")!;
const devanagariOOutline = devanagariOutline("ओ");
const DEVANAGARI_AU = ductusFor("औ", "devanagari")!;
const devanagariAuOutline = devanagariOutline("औ");
const DEVANAGARI_KA = ductusFor("क", "devanagari")!;
const devanagariKaOutline = devanagariOutline("क");
const DEVANAGARI_GA = ductusFor("ग", "devanagari")!;
const devanagariGaOutline = devanagariOutline("ग");
const DEVANAGARI_CA = ductusFor("च", "devanagari")!;
const devanagariCaOutline = devanagariOutline("च");
const DEVANAGARI_TA = ductusFor("त", "devanagari")!;
const devanagariTaOutline = devanagariOutline("त");
const DEVANAGARI_DA = ductusFor("द", "devanagari")!;
const devanagariDaOutline = devanagariOutline("द");
const DEVANAGARI_DHA = ductusFor("ध", "devanagari")!;
const devanagariDhaOutline = devanagariOutline("ध");
const DEVANAGARI_NA = ductusFor("न", "devanagari")!;
const devanagariNaOutline = devanagariOutline("न");
const DEVANAGARI_PA = ductusFor("प", "devanagari")!;
const devanagariPaOutline = devanagariOutline("प");
const DEVANAGARI_BA = ductusFor("ब", "devanagari")!;
const devanagariBaOutline = devanagariOutline("ब");
const DEVANAGARI_BHA = ductusFor("भ", "devanagari")!;
const devanagariBhaOutline = devanagariOutline("भ");
const DEVANAGARI_MA = ductusFor("म", "devanagari")!;
const devanagariMaOutline = devanagariOutline("म");
const DEVANAGARI_YA = ductusFor("य", "devanagari")!;
const devanagariYaOutline = devanagariOutline("य");
const DEVANAGARI_RA = ductusFor("र", "devanagari")!;
const devanagariRaOutline = devanagariOutline("र");
const DEVANAGARI_LA = ductusFor("ल", "devanagari")!;
const devanagariLaOutline = devanagariOutline("ल");
const DEVANAGARI_VA = ductusFor("व", "devanagari")!;
const devanagariVaOutline = devanagariOutline("व");
const DEVANAGARI_SHA = ductusFor("श", "devanagari")!;
const devanagariShaOutline = devanagariOutline("श");
const DEVANAGARI_SA = ductusFor("स", "devanagari")!;
const devanagariSaOutline = devanagariOutline("स");
const DEVANAGARI_HA = ductusFor("ह", "devanagari")!;
const devanagariHaOutline = devanagariOutline("ह");
const CYRILLIC_A = ductusFor("а", "cyrillic")!;
const cyrillicAOutline = cyrillicOutline("а");
const CYRILLIC_BE = ductusFor("б", "cyrillic")!;
const cyrillicBeOutline = cyrillicOutline("б");
const CYRILLIC_VE = ductusFor("в", "cyrillic")!;
const cyrillicVeOutline = cyrillicOutline("в");
const CYRILLIC_GE = ductusFor("г", "cyrillic")!;
const cyrillicGeOutline = cyrillicOutline("г");
const CYRILLIC_DE = ductusFor("д", "cyrillic")!;
const cyrillicDeOutline = cyrillicOutline("д");
const CYRILLIC_IE = ductusFor("е", "cyrillic")!;
const cyrillicIeOutline = cyrillicOutline("е");
const CYRILLIC_IO = ductusFor("ё", "cyrillic")!;
const cyrillicIoOutline = cyrillicOutline("ё");
const CYRILLIC_ZHE = ductusFor("ж", "cyrillic")!;
const cyrillicZheOutline = cyrillicOutline("ж");
const CYRILLIC_ZE = ductusFor("з", "cyrillic")!;
const cyrillicZeOutline = cyrillicOutline("з");
const CYRILLIC_I = ductusFor("и", "cyrillic")!;
const cyrillicIOutline = cyrillicOutline("и");
const CYRILLIC_SHORT_I = ductusFor("й", "cyrillic")!;
const cyrillicShortIOutline = cyrillicOutline("й");
const CYRILLIC_KA = ductusFor("к", "cyrillic")!;
const cyrillicKaOutline = cyrillicOutline("к");
const CYRILLIC_EL = ductusFor("л", "cyrillic")!;
const cyrillicElOutline = cyrillicOutline("л");
const CYRILLIC_EM = ductusFor("м", "cyrillic")!;
const cyrillicEmOutline = cyrillicOutline("м");
const CYRILLIC_EN = ductusFor("н", "cyrillic")!;
const cyrillicEnOutline = cyrillicOutline("н");
const CYRILLIC_O = ductusFor("о", "cyrillic")!;
const cyrillicOOutline = cyrillicOutline("о");
const CYRILLIC_PE = ductusFor("п", "cyrillic")!;
const cyrillicPeOutline = cyrillicOutline("п");
const CYRILLIC_ER = ductusFor("р", "cyrillic")!;
const cyrillicErOutline = cyrillicOutline("р");
const CYRILLIC_ES = ductusFor("с", "cyrillic")!;
const cyrillicEsOutline = cyrillicOutline("с");
const CYRILLIC_TE = ductusFor("т", "cyrillic")!;
const cyrillicTeOutline = cyrillicOutline("т");
const CYRILLIC_U = ductusFor("у", "cyrillic")!;
const cyrillicUOutline = cyrillicOutline("у");
const CYRILLIC_EF = ductusFor("ф", "cyrillic")!;
const cyrillicEfOutline = cyrillicOutline("ф");
const CYRILLIC_HA = ductusFor("х", "cyrillic")!;
const cyrillicHaOutline = cyrillicOutline("х");
const CYRILLIC_TSE = ductusFor("ц", "cyrillic")!;
const cyrillicTseOutline = cyrillicOutline("ц");
const CYRILLIC_CHE = ductusFor("ч", "cyrillic")!;
const cyrillicCheOutline = cyrillicOutline("ч");
const CYRILLIC_SHA = ductusFor("ш", "cyrillic")!;
const cyrillicShaOutline = cyrillicOutline("ш");
const CYRILLIC_SHCHA = ductusFor("щ", "cyrillic")!;
const cyrillicShchaOutline = cyrillicOutline("щ");
const CYRILLIC_HARD_SIGN = ductusFor("ъ", "cyrillic")!;
const cyrillicHardSignOutline = cyrillicOutline("ъ");
const CYRILLIC_YERY = ductusFor("ы", "cyrillic")!;
const cyrillicYeryOutline = cyrillicOutline("ы");
const CYRILLIC_SOFT_SIGN = ductusFor("ь", "cyrillic")!;
const cyrillicSoftSignOutline = cyrillicOutline("ь");
const CYRILLIC_E = ductusFor("э", "cyrillic")!;
const cyrillicEOutline = cyrillicOutline("э");
const CYRILLIC_YU = ductusFor("ю", "cyrillic")!;
const cyrillicYuOutline = cyrillicOutline("ю");
const CYRILLIC_YA = ductusFor("я", "cyrillic")!;
const cyrillicYaOutline = cyrillicOutline("я");
const GUJARATI_A = ductusFor("અ", "gujarati")!;
const gujaratiAOutline = gujaratiOutline("અ");
const GUJARATI_AA = ductusFor("આ", "gujarati")!;
const gujaratiAaOutline = gujaratiOutline("આ");
const GUJARATI_I = ductusFor("ઇ", "gujarati")!;
const gujaratiIOutline = gujaratiOutline("ઇ");
const GUJARATI_II = ductusFor("ઈ", "gujarati")!;
const gujaratiIiOutline = gujaratiOutline("ઈ");
const GUJARATI_U = ductusFor("ઉ", "gujarati")!;
const gujaratiUOutline = gujaratiOutline("ઉ");
const GUJARATI_UU = ductusFor("ઊ", "gujarati")!;
const gujaratiUuOutline = gujaratiOutline("ઊ");
const GUJARATI_VOCALIC_R = ductusFor("ઋ", "gujarati")!;
const gujaratiVocalicROutline = gujaratiOutline("ઋ");
const GUJARATI_E = ductusFor("એ", "gujarati")!;
const gujaratiEOutline = gujaratiOutline("એ");
const GUJARATI_AI = ductusFor("ઐ", "gujarati")!;
const gujaratiAiOutline = gujaratiOutline("ઐ");
const GUJARATI_O = ductusFor("ઓ", "gujarati")!;
const gujaratiOOutline = gujaratiOutline("ઓ");
const GUJARATI_AU = ductusFor("ઔ", "gujarati")!;
const gujaratiAuOutline = gujaratiOutline("ઔ");
const GUJARATI_KA = ductusFor("ક", "gujarati")!;
const gujaratiKaOutline = gujaratiOutline("ક");
const GUJARATI_KHA = ductusFor("ખ", "gujarati")!;
const gujaratiKhaOutline = gujaratiOutline("ખ");
const GUJARATI_GA = ductusFor("ગ", "gujarati")!;
const gujaratiGaOutline = gujaratiOutline("ગ");
const GUJARATI_GHA = ductusFor("ઘ", "gujarati")!;
const gujaratiGhaOutline = gujaratiOutline("ઘ");
const GUJARATI_NGA = ductusFor("ઙ", "gujarati")!;
const gujaratiNgaOutline = gujaratiOutline("ઙ");
const GUJARATI_CA = ductusFor("ચ", "gujarati")!;
const gujaratiCaOutline = gujaratiOutline("ચ");
const GUJARATI_CHA = ductusFor("છ", "gujarati")!;
const gujaratiChaOutline = gujaratiOutline("છ");
const GUJARATI_JA = ductusFor("જ", "gujarati")!;
const gujaratiJaOutline = gujaratiOutline("જ");
const GUJARATI_JHA = ductusFor("ઝ", "gujarati")!;
const gujaratiJhaOutline = gujaratiOutline("ઝ");
const GUJARATI_NYA = ductusFor("ઞ", "gujarati")!;
const gujaratiNyaOutline = gujaratiOutline("ઞ");
const GUJARATI_TTA = ductusFor("ટ", "gujarati")!;
const gujaratiTtaOutline = gujaratiOutline("ટ");
const GUJARATI_TTHA = ductusFor("ઠ", "gujarati")!;
const gujaratiTthaOutline = gujaratiOutline("ઠ");
const GUJARATI_DDA = ductusFor("ડ", "gujarati")!;
const gujaratiDdaOutline = gujaratiOutline("ડ");
const GUJARATI_DDHA = ductusFor("ઢ", "gujarati")!;
const gujaratiDdhaOutline = gujaratiOutline("ઢ");
const GUJARATI_NNA = ductusFor("ણ", "gujarati")!;
const gujaratiNnaOutline = gujaratiOutline("ણ");
const GUJARATI_TA = ductusFor("ત", "gujarati")!;
const gujaratiTaOutline = gujaratiOutline("ત");
const GUJARATI_THA = ductusFor("થ", "gujarati")!;
const gujaratiThaOutline = gujaratiOutline("થ");
const GUJARATI_DA = ductusFor("દ", "gujarati")!;
const gujaratiDaOutline = gujaratiOutline("દ");
const GUJARATI_DHA = ductusFor("ધ", "gujarati")!;
const gujaratiDhaOutline = gujaratiOutline("ધ");
const GUJARATI_NA = ductusFor("ન", "gujarati")!;
const gujaratiNaOutline = gujaratiOutline("ન");
const GUJARATI_PA = ductusFor("પ", "gujarati")!;
const gujaratiPaOutline = gujaratiOutline("પ");
const GUJARATI_PHA = ductusFor("ફ", "gujarati")!;
const gujaratiPhaOutline = gujaratiOutline("ફ");
const GUJARATI_BA = ductusFor("બ", "gujarati")!;
const gujaratiBaOutline = gujaratiOutline("બ");
const GUJARATI_BHA = ductusFor("ભ", "gujarati")!;
const gujaratiBhaOutline = gujaratiOutline("ભ");
const GUJARATI_MA = ductusFor("મ", "gujarati")!;
const gujaratiMaOutline = gujaratiOutline("મ");
const GUJARATI_YA = ductusFor("ય", "gujarati")!;
const gujaratiYaOutline = gujaratiOutline("ય");
const GUJARATI_RA = ductusFor("ર", "gujarati")!;
const gujaratiRaOutline = gujaratiOutline("ર");
const GUJARATI_LA = ductusFor("લ", "gujarati")!;
const gujaratiLaOutline = gujaratiOutline("લ");
const GUJARATI_LLA = ductusFor("ળ", "gujarati")!;
const gujaratiLlaOutline = gujaratiOutline("ળ");
const GUJARATI_VA = ductusFor("વ", "gujarati")!;
const gujaratiVaOutline = gujaratiOutline("વ");
const GUJARATI_SHA = ductusFor("શ", "gujarati")!;
const gujaratiShaOutline = gujaratiOutline("શ");
const GUJARATI_SA = ductusFor("સ", "gujarati")!;
const gujaratiSaOutline = gujaratiOutline("સ");
const GUJARATI_HA = ductusFor("હ", "gujarati")!;
const gujaratiHaOutline = gujaratiOutline("હ");
const HEBREW_ALEF = ductusFor("א", "hebrew")!;
const hebrewAlefOutline = hebrewOutline("א");
const HEBREW_BET = ductusFor("ב", "hebrew")!;
const hebrewBetOutline = hebrewOutline("ב");
const HEBREW_GIMEL = ductusFor("ג", "hebrew")!;
const hebrewGimelOutline = hebrewOutline("ג");
const HEBREW_DALET = ductusFor("ד", "hebrew")!;
const hebrewDaletOutline = hebrewOutline("ד");
const HEBREW_HEI = ductusFor("ה", "hebrew")!;
const hebrewHeiOutline = hebrewOutline("ה");
const HEBREW_VAV = ductusFor("ו", "hebrew")!;
const hebrewVavOutline = hebrewOutline("ו");
const HEBREW_ZAYIN = ductusFor("ז", "hebrew")!;
const hebrewZayinOutline = hebrewOutline("ז");
const HEBREW_HEIT = ductusFor("ח", "hebrew")!;
const hebrewHeitOutline = hebrewOutline("ח");
const HEBREW_TET = ductusFor("ט", "hebrew")!;
const hebrewTetOutline = hebrewOutline("ט");
const HEBREW_YOD = ductusFor("י", "hebrew")!;
const hebrewYodOutline = hebrewOutline("י");
const HEBREW_KAF = ductusFor("כ", "hebrew")!;
const hebrewKafOutline = hebrewOutline("כ");
const HEBREW_LAMED = ductusFor("ל", "hebrew")!;
const hebrewLamedOutline = hebrewOutline("ל");
const HEBREW_MEM = ductusFor("מ", "hebrew")!;
const hebrewMemOutline = hebrewOutline("מ");
const HEBREW_NUN = ductusFor("נ", "hebrew")!;
const hebrewNunOutline = hebrewOutline("נ");
const HEBREW_SAMEKH = ductusFor("ס", "hebrew")!;
const hebrewSamekhOutline = hebrewOutline("ס");
const HEBREW_AYIN = ductusFor("ע", "hebrew")!;
const hebrewAyinOutline = hebrewOutline("ע");
const HEBREW_PE = ductusFor("פ", "hebrew")!;
const hebrewPeOutline = hebrewOutline("פ");
const HEBREW_TSADI = ductusFor("צ", "hebrew")!;
const hebrewTsadiOutline = hebrewOutline("צ");
const HEBREW_QOF = ductusFor("ק", "hebrew")!;
const hebrewQofOutline = hebrewOutline("ק");
const HEBREW_RESH = ductusFor("ר", "hebrew")!;
const hebrewReshOutline = hebrewOutline("ר");
const HEBREW_SHIN = ductusFor("ש", "hebrew")!;
const hebrewShinOutline = hebrewOutline("ש");
const HEBREW_TAV = ductusFor("ת", "hebrew")!;
const hebrewTavOutline = hebrewOutline("ת");
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
const URDU_RE = ductusFor("ر", "urdu-nastaliq")!;
const urduReOutline = naskhOutline("ر");
const URDU_SIN = ductusFor("س", "urdu-nastaliq")!;
const urduSinOutline = naskhOutline("س");
const URDU_SHIN = ductusFor("ش", "urdu-nastaliq")!;
const urduShinOutline = naskhOutline("ش");
const URDU_KAF = ductusFor("ک", "urdu-nastaliq")!;
const urduKafOutline = naskhOutline("ک");
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
const PERSIAN_TEH = DUCTUS["ت"];
const persianTehOutline = naskhOutline("ت");
const PERSIAN_SIN = DUCTUS["س"];
const persianSinOutline = naskhOutline("س");
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

/** Walk a node tree, collecting every node the predicate accepts. */
function collect(node: SvgNode, pick: (n: SvgNode) => boolean, out: SvgNode[] = []): SvgNode[] {
  if (pick(node)) out.push(node);
  for (const c of node.children ?? []) collect(c, pick, out);
  return out;
}

const byTag = (node: SvgNode, tag: string) => collect(node, (n) => n.tag === tag);

describe("ductusFor — only cited letters have a ductus", () => {
  it("finds eleven Tamil letters, nine Persian letters, eighteen Arabic letters, and thirteen Urdu letters", () => {
    expect(ductusFor("ம")?.glyph).toBe("ம");
    expect(ductusFor("அ")?.glyph).toBe("அ");
    expect(ductusFor("ஆ")?.glyph).toBe("ஆ");
    expect(ductusFor("இ")?.glyph).toBe("இ");
    expect(ductusFor("க")?.glyph).toBe("க");
    expect(ductusFor("வ")?.glyph).toBe("வ");
    expect(ductusFor("ல")?.glyph).toBe("ல");
    expect(ductusFor("ற")?.glyph).toBe("ற");
    expect(ductusFor("ன")?.glyph).toBe("ன");
    expect(ductusFor("ண")?.glyph).toBe("ண");
    expect(ductusFor("ந")?.glyph).toBe("ந");
    expect(ductusFor("ا")?.glyph).toBe("ا");
    expect(ductusFor("ب")?.glyph).toBe("ب");
    expect(ductusFor("ت")?.glyph).toBe("ت");
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
    expect(ductusFor("ج", "perso-arabic")).toBeUndefined();
    expect(ductusFor("ر", "urdu-nastaliq")?.glyph).toBe("ر");
    expect(ductusFor("ر", "perso-arabic")).toBeUndefined();
    expect(ductusFor("س", "urdu-nastaliq")?.glyph).toBe("س");
    expect(ductusFor("ش", "urdu-nastaliq")?.glyph).toBe("ش");
    expect(ductusFor("ش", "perso-arabic")).toBeUndefined();
    expect(ductusFor("ک", "urdu-nastaliq")?.glyph).toBe("ک");
    expect(ductusFor("ک", "perso-arabic")).toBeUndefined();
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

  it("keeps the shared Arabic and Persian ب independently addressable", () => {
    const arabic = ductusFor("ب", "arabic");
    const persian = ductusFor("ب", "perso-arabic");
    expect(arabic?.script).toBe("arabic");
    expect(persian?.script).toBe("perso-arabic");
    expect(arabic?.source.url).not.toBe(persian?.source.url);
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

  it("keeps the shared Arabic and Urdu ش independently addressable", () => {
    const arabic = ductusFor("ش", "arabic");
    const urdu = ductusFor("ش", "urdu-nastaliq");
    expect(arabic?.script).toBe("arabic");
    expect(urdu?.script).toBe("urdu-nastaliq");
    expect(arabic?.source.url).not.toBe(urdu?.source.url);
    expect(ductusFor("ش", "perso-arabic")).toBeUndefined();
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

  it("returns undefined for a letter nobody has authored a stroke order for", () => {
    // Persian پ is deferred inventory work, not a starter entry or authored
    // pen path. It must come back empty rather than borrow ب's or Tamil ம's.
    expect(ductusFor("پ")).toBeUndefined();
    expect(ductusFor("A")).toBeUndefined();
    expect(ductusFor("")).toBeUndefined();
  });

  it("does not mistake inherited Object properties for letters", () => {
    // DUCTUS is a plain object, so `DUCTUS["toString"]` is a FUNCTION, not a
    // letter. A naive lookup would hand that to the renderer and crash.
    expect(ductusFor("toString")).toBeUndefined();
    expect(ductusFor("constructor")).toBeUndefined();
  });
});

describe("segment fractions — where each part ends along its stroke", () => {
  const fractions = segmentEndFractions(MA.strokes[0]);

  it("has one entry per labelled part", () => {
    expect(fractions).toHaveLength(MA.strokes[0].segments.length);
  });

  it("ascends and finishes at the end of the stroke", () => {
    for (let i = 1; i < fractions.length; i++) {
      expect(fractions[i]).toBeGreaterThan(fractions[i - 1]);
    }
    expect(fractions[0]).toBeGreaterThan(0);
    expect(fractions[fractions.length - 1]).toBeCloseTo(1, 10);
  });

  it("a zero-length stroke reports every part already complete", () => {
    const flat = { segments: [{ label: "nowhere", path: [{ x: 5, y: 5 }, { x: 5, y: 5 }] }] };
    expect(segmentEndFractions(flat)).toEqual([1]);
  });
});

describe("ductusSteps — the frames, in writing order", () => {
  const steps = ductusSteps(MA);

  it("gives one step per labelled part, numbered from 1", () => {
    expect(steps).toHaveLength(5);
    expect(steps.map((s) => s.number)).toEqual([1, 2, 3, 4, 5]);
    expect(steps[0].label).toBe("down the left upright");
    expect(steps[4].label).toBe("down the middle");
  });

  it("marks ம as never lifting the pen", () => {
    expect(steps.every((s) => s.startsAfterLift === false)).toBe(true);
    expect(steps.every((s) => s.strokeIndex === 0)).toBe(true);
  });
});

describe("the drawn frame", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[2]);

  it("draws the FONT's outline, not a redrawn one", () => {
    const glyphPath = byTag(frame, "path").find((p) => p.attrs.class === "ductus__glyph")!;
    expect(glyphPath.attrs.d).toBe(outline.path);
    // Sanity: that really is a font path — quadratics and closed contours.
    expect(String(glyphPath.attrs.d)).toMatch(/^M/);
    expect(String(glyphPath.attrs.d)).toContain("Z");
  });

  it("emits the pen path straight from penPathD at the step's fraction", () => {
    const step = ductusSteps(MA)[2];
    const pen = byTag(frame, "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], step.fraction));
    expect(pen.attrs.fill).toBe("none");
  });

  it("puts a pen dot at the end of what has been drawn", () => {
    const dot = byTag(frame, "circle")[0];
    const d = String(byTag(frame, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs.d);
    const last = d.trim().split(/(?=[ML])/).pop()!.slice(1).trim().split(/\s+/).map(Number);
    expect(Number(dot.attrs.cx)).toBeCloseTo(last[0], 1);
    expect(Number(dot.attrs.cy)).toBeCloseTo(last[1], 1);
  });

  it("labels the step for a screen reader as well as on screen", () => {
    expect(String(frame.attrs["aria-label"])).toContain("up the right side");
    expect(byTag(frame, "title")[0].text).toContain("up the right side");
    expect(byTag(frame, "tspan").map((t) => t.text).join(" ")).toBe("3. up the right side");
  });

  it("carries an intrinsic size matching its viewBox, so it never renders squashed", () => {
    const [, , w, h] = String(frame.attrs.viewBox).split(" ").map(Number);
    expect(Number(frame.attrs.height) / Number(frame.attrs.width)).toBeCloseTo(h / w, 2);
  });
});

describe("captions wrap instead of running off the panel", () => {
  it("breaks on whole words at the width available", () => {
    // 900 units wide at 92-unit text ≈ 18 characters a line.
    expect(wrapCaption("1. down the left upright", 900, 92)).toEqual(["1. down the left", "upright"]);
    expect(wrapCaption("4. over the top", 900, 92)).toEqual(["4. over the top"]);
  });

  it("never chops a word in half, even one too long to fit", () => {
    const lines = wrapCaption("Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr", 200, 92);
    expect(lines).toEqual(["Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"]);
  });

  it("returns one empty line for empty text rather than no lines at all", () => {
    expect(wrapCaption("   ", 900, 92)).toEqual([""]);
  });

  it("makes the box taller when the captions need two lines", () => {
    const short = viewBoxFor(MA, outline, { captionSize: 20 }).height;
    const tall = viewBoxFor(MA, outline, { captionSize: 200 }).height;
    expect(tall).toBeGreaterThan(short);
  });
});

// ---------------------------------------------------------------------------
// The flip. This is the invariant the whole module exists to guarantee: the
// glyph and the pen path are both in font units (y-up), SVG is y-down, and they
// are flipped together exactly once — so they cannot end up disagreeing.
// ---------------------------------------------------------------------------
describe("one shared y-flip", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[4]);

  it("uses exactly one scale(1,-1) group", () => {
    const flips = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"));
    expect(flips).toHaveLength(1);
  });

  it("puts the glyph, every pen path and the pen dot inside that one group", () => {
    const flip = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"))[0];
    expect(byTag(flip, "path").length).toBe(byTag(frame, "path").length);
    expect(byTag(flip, "circle").length).toBe(byTag(frame, "circle").length);
    expect(byTag(flip, "path").length).toBeGreaterThan(1); // glyph + pen
  });

  it("keeps text OUT of the flip, because mirrored text is unreadable", () => {
    const flip = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"))[0];
    expect(byTag(flip, "text")).toHaveLength(0);
    expect(byTag(frame, "text")).toHaveLength(1);
  });

  it("negates the vertical range in the viewBox, as the flip requires", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Top of the letter (largest font y) becomes the SMALLEST svg y.
    expect(box.minY).toBeLessThan(0);
    expect(box.minY).toBeCloseTo(-(b.y1 + 70), 5);
    // The box is wide enough for the ink, and taller by the caption band.
    expect(box.width).toBeGreaterThanOrEqual(b.x1 - b.x0);
    expect(box.height).toBeGreaterThan(b.y1 - b.y0);
  });

  it("keeps the whole flipped glyph inside the viewBox", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Flip every corner of the glyph box and check containment.
    for (const x of [b.x0, b.x1]) {
      for (const y of [b.y0, b.y1]) {
        expect(x).toBeGreaterThanOrEqual(box.minX);
        expect(x).toBeLessThanOrEqual(box.minX + box.width);
        expect(-y).toBeGreaterThanOrEqual(box.minY);
        expect(-y).toBeLessThanOrEqual(box.minY + box.height);
      }
    }
  });

  it("CONTROL: an unflipped viewBox would cut the letter off entirely", () => {
    // If someone "simplified" the box to raw font coordinates, the flipped
    // glyph would sit at negative y and fall completely outside it. This is the
    // failure the negation prevents; assert it really is a failure.
    const b = outline.bounds;
    const naiveTop = b.y0;
    expect(-b.y1).toBeLessThan(naiveTop);
  });
});

describe("the build-up advances", () => {
  const strip = ductusFilmstrip(MA, outline);

  it("has one frame per step", () => {
    expect(strip.frames).toHaveLength(strip.steps.length);
    expect(strip.frames).toHaveLength(5);
  });

  it("draws strictly more of the stroke in each successive frame", () => {
    const drawn = strip.frames.map(
      (f) => String(byTag(f, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs.d).length,
    );
    for (let i = 1; i < drawn.length; i++) {
      expect(drawn[i]).toBeGreaterThan(drawn[i - 1]);
    }
  });

  it("the last frame is the complete stroke", () => {
    const last = strip.frames[strip.frames.length - 1];
    const pen = byTag(last, "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], 1));
  });

  it("says in words how many strokes and lifts there are", () => {
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });
});

describe("அ — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(A);
  const strip = ductusFilmstrip(A, aOutline);

  it("places the only pen lift before the separate right upright", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
  });

  it("reports the source-backed movement, stroke, and lift counts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed body visible while drawing the upright", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(A.strokes[1], 1));
  });
});

describe("ஆ — the upright and long-vowel loop stay connected", () => {
  const steps = ductusSteps(AA);
  const strip = ductusFilmstrip(AA, aaOutline);

  it("places one lift before the upright and none before its loop", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("finishes the connected upright-and-loop stroke in the last frame", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(AA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(AA.strokes[1], 1));
  });
});

describe("இ — a real cited seven-movement filmstrip", () => {
  const steps = ductusSteps(I);
  const strip = ductusFilmstrip(I, iOutline);

  it("places one lift before the outer climb and joins that climb to the arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1, 1]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("finishes the joined outer climb-and-arch stroke in the last frame", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(I.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(I.strokes[1], 1));
  });
});

describe("க — a real cited three-stroke filmstrip", () => {
  const steps = ductusSteps(KA);
  const strip = ductusFilmstrip(KA, kaOutline);

  it("places lifts before each lower bowl", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible while drawing the right bowl", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([penPathD(KA.strokes[0], 1), penPathD(KA.strokes[1], 1)]);
    expect(pen.attrs.d).toBe(penPathD(KA.strokes[2], 1));
  });
});

describe("வ — a real cited unbroken five-movement filmstrip", () => {
  const steps = ductusSteps(VA);
  const strip = ductusFilmstrip(VA, vaOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
  });

  it("reports five movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(VA.strokes[0], 1));
  });
});

describe("ல — a real cited unbroken four-movement filmstrip", () => {
  const steps = ductusSteps(LA);
  const strip = ductusFilmstrip(LA, laOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(LA.strokes[0], 1));
  });
});

describe("ற — a real cited three-stroke five-movement filmstrip", () => {
  const steps = ductusSteps(RRA);
  const strip = ductusFilmstrip(RRA, rraOutline);

  it("marks exactly the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 2]);
  });

  it("reports five movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps both completed strokes visible while drawing the joined sweep", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([penPathD(RRA.strokes[0], 1), penPathD(RRA.strokes[1], 1)]);
    expect(pen.attrs.d).toBe(penPathD(RRA.strokes[2], 1));
  });
});

describe("ன — a real cited two-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(NNA);
  const strip = ductusFilmstrip(NNA, nnaOutline);

  it("joins the loop, inner arch, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed loop-and-bar stroke visible while drawing the upright", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(NNA.strokes[1], 1));
  });
});

describe("ண — a real cited two-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(RETROFLEX_NNA);
  const strip = ductusFilmstrip(RETROFLEX_NNA, retroflexNnaOutline);

  it("joins the loop, both inner arches, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 1]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the completed double-arch stroke visible while drawing the upright", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[1], 1));
  });
});

describe("ந — a real cited three-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(DENTAL_NA);
  const strip = ductusFilmstrip(DENTAL_NA, dentalNaOutline);

  it("marks the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible during the right-hand descent", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(DENTAL_NA.strokes[0], 1),
      penPathD(DENTAL_NA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(DENTAL_NA.strokes[2], 1));
  });
});

describe("Chinese 人 — two cited falling strokes in PRC order", () => {
  const steps = ductusSteps(CHINESE_REN);
  const strip = ductusFilmstrip(CHINESE_REN, chineseRenOutline);

  it("shows the left-falling stroke before restarting for the right-falling stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left-falling piě stroke from the upper centre",
      "lift, then draw the right-falling nà stroke from the junction",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans SC glyph with the first stroke settled behind the second", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseRenOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(CHINESE_REN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_REN.strokes[1], 1),
    );
  });
});

describe("Chinese 亻 — a cited falling stroke followed by a vertical", () => {
  const steps = ductusSteps(CHINESE_PERSON_RADICAL);
  const strip = ductusFilmstrip(CHINESE_PERSON_RADICAL, chinesePersonRadicalOutline);

  it("shows the left-falling stroke before restarting for the vertical", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left-falling piě stroke from upper right to lower left",
      "lift, then draw the vertical shù stroke from the junction to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans SC radical with the falling stroke settled behind the vertical", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chinesePersonRadicalOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(CHINESE_PERSON_RADICAL.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_PERSON_RADICAL.strokes[1], 1),
    );
  });
});

describe("Chinese 口 — a cited three-run box that closes last", () => {
  const steps = ductusSteps(CHINESE_MOUTH);
  const strip = ductusFilmstrip(CHINESE_MOUTH, chineseMouthOutline);

  it("shows the joined top-right corner before the separately closing bottom", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left vertical shù stroke from top to bottom",
      "lift, then draw the top bar from left to right",
      "turn the corner without lifting and descend the right side",
      "lift, then close the bottom from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC box with the first two runs behind the closing bottom", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseMouthOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(CHINESE_MOUTH.strokes[0], 1), penPathD(CHINESE_MOUTH.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_MOUTH.strokes[2], 1),
    );
  });
});

describe("Chinese 女 — a cited bent first run followed by two lifted strokes", () => {
  const steps = ductusSteps(CHINESE_WOMAN);
  const strip = ductusFilmstrip(CHINESE_WOMAN, chineseWomanOutline);

  it("keeps the first bend joined before the falling and horizontal strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the first piědiǎn stroke down and left",
      "turn without lifting and sweep down to the lower right",
      "lift, then draw the left-falling piě stroke from upper right to lower left",
      "lift, then draw the middle horizontal héng from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC glyph with both earlier runs behind the middle horizontal", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseWomanOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(CHINESE_WOMAN.strokes[0], 1), penPathD(CHINESE_WOMAN.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_WOMAN.strokes[2], 1),
    );
  });
});

describe("Chinese 子 — two cited joined turns followed by a final horizontal", () => {
  const steps = ductusSteps(CHINESE_CHILD);
  const strip = ductusFilmstrip(CHINESE_CHILD, chineseChildOutline);

  it("keeps each turn joined inside its stroke before the final horizontal", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top horizontal héng from left to right",
      "turn without lifting and sweep down-left",
      "lift, then descend the central vertical",
      "hook left at the base without lifting",
      "lift, then draw the middle horizontal héng from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 2]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC glyph with both hooked runs behind the final horizontal", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseChildOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(CHINESE_CHILD.strokes[0], 1), penPathD(CHINESE_CHILD.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_CHILD.strokes[2], 1),
    );
  });
});

describe("Chinese 日 — a cited joined corner with an inside-before-close order", () => {
  const steps = ductusSteps(CHINESE_SUN);
  const strip = ductusFilmstrip(CHINESE_SUN, chineseSunOutline);

  it("draws the left side, joined top-right corner, inside bar, then closing bottom", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left vertical shù from top to bottom",
      "lift, then draw the top horizontal héng from left to right",
      "turn without lifting and descend the right side",
      "lift, then draw the middle horizontal héng from left to right",
      "lift, then close the bottom horizontal héng from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans SC glyph with the inside bar behind the closing bottom", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseSunOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([
      penPathD(CHINESE_SUN.strokes[0], 1),
      penPathD(CHINESE_SUN.strokes[1], 1),
      penPathD(CHINESE_SUN.strokes[2], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_SUN.strokes[3], 1),
    );
  });
});

describe("Chinese 讠 — a cited dot followed by one double-turning stroke", () => {
  const steps = ductusSteps(CHINESE_SPEECH_RADICAL);
  const strip = ductusFilmstrip(CHINESE_SPEECH_RADICAL, chineseSpeechRadicalOutline);

  it("keeps the horizontal, descent, and rising finish joined after the dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top dot down and right",
      "lift, then draw the short horizontal from left to right",
      "turn without lifting and descend the vertical",
      "turn without lifting and rise to the upper right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with the completed dot behind the joined body", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseSpeechRadicalOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(CHINESE_SPEECH_RADICAL.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_SPEECH_RADICAL.strokes[1], 1),
    );
  });
});

describe("Chinese 氵 — two falling dots above one rising bottom stroke", () => {
  const steps = ductusSteps(CHINESE_WATER_RADICAL);
  const strip = ductusFilmstrip(CHINESE_WATER_RADICAL, chineseWaterRadicalOutline);

  it("keeps all three sourced strokes separate while joining the final rise", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upper dot down and right",
      "lift, then draw the middle dot down and right",
      "lift, then begin the bottom stroke with a slight rise left",
      "continue without lifting in a long rise to the upper right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with both completed dots behind the rising stroke", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseWaterRadicalOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(CHINESE_WATER_RADICAL.strokes[0], 1),
      penPathD(CHINESE_WATER_RADICAL.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_WATER_RADICAL.strokes[2], 1),
    );
  });
});

describe("Chinese 宀 — two separate marks before a joined roof hook", () => {
  const steps = ductusSteps(CHINESE_ROOF_RADICAL);
  const strip = ductusFilmstrip(CHINESE_ROOF_RADICAL, chineseRoofRadicalOutline);

  it("keeps the horizontal and down-left hook joined after two lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top dot down and right",
      "lift, then draw the left-side stroke down and left",
      "lift, then draw the horizontal roof from left to right",
      "hook down and left without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC radical with both completed marks behind the roof hook", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseRoofRadicalOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(CHINESE_ROOF_RADICAL.strokes[0], 1),
      penPathD(CHINESE_ROOF_RADICAL.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_ROOF_RADICAL.strokes[2], 1),
    );
  });
});

describe("Chinese 你 — seven cited strokes with two joined hooks", () => {
  const steps = ductusSteps(CHINESE_YOU);
  const strip = ductusFilmstrip(CHINESE_YOU, chineseYouOutline);

  it("writes 亻 first, keeps both hooks joined, and places both dots last", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3, 3, 4, 4, 5, 6]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true, false, true, false, true, true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character with six completed strokes behind the final dot", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseYouOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual(
      CHINESE_YOU.strokes.slice(0, 6).map((stroke) => penPathD(stroke, 1)),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_YOU.strokes[6], 1),
    );
  });
});

describe("Chinese 好 — six cited strokes with 女 before 子", () => {
  const steps = ductusSteps(CHINESE_GOOD);
  const strip = ductusFilmstrip(CHINESE_GOOD, chineseGoodOutline);

  it("keeps all three internal turns joined across six component-ordered strokes", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 3, 4, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, false, true, true, true, false, true, false, true,
    ]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character with five completed strokes behind the final bar", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseGoodOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual(
      CHINESE_GOOD.strokes.slice(0, 5).map((stroke) => penPathD(stroke, 1)),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_GOOD.strokes[5], 1),
    );
  });
});

describe("Chinese 我 — seven cited strokes with one joined hook", () => {
  const steps = ductusSteps(CHINESE_I);
  const strip = ductusFilmstrip(CHINESE_I, chineseIOutline);

  it("preserves seven strokes, one internal join, and six lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2, 3, 4, 4, 5, 6]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseIOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_I.strokes[6], 1),
    );
  });
});

describe("Chinese 是 — nine cited strokes with 日 first", () => {
  const steps = ductusSteps(CHINESE_BE);
  const strip = ductusFilmstrip(CHINESE_BE, chineseBeOutline);

  it("closes 日 before the lower body and preserves eight lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3, 4, 5, 6, 7, 8]);
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(8);
    expect(strip.summary).toBe("9 strokes · 8 pen lifts · 10 movements");
  });

  it("draws the exact Noto Sans SC character behind the final sweep", () => {
    const paths = byTag(strip.frames[9], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseBeOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_BE.strokes[8], 1),
    );
  });
});

describe("Chinese 不 — four separately placed cited strokes", () => {
  const steps = ductusSteps(CHINESE_NOT);
  const strip = ductusFilmstrip(CHINESE_NOT, chineseNotOutline);

  it("keeps all four source strokes separate with three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseNotOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_NOT.strokes[3], 1),
    );
  });
});

describe("Chinese 名 — 夕 before 口 in six cited strokes", () => {
  const steps = ductusSteps(CHINESE_NAME);
  const strip = ductusFilmstrip(CHINESE_NAME, chineseNameOutline);

  it("preserves both joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3, 4, 4, 5]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind 口's closing stroke", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseNameOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_NAME.strokes[5], 1),
    );
  });
});

describe("Chinese 字 — 宀 before 子 in six cited strokes", () => {
  const steps = ductusSteps(CHINESE_CHARACTER);
  const strip = ductusFilmstrip(CHINESE_CHARACTER, chineseCharacterOutline);

  it("preserves all three joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2, 3, 3, 4, 4, 5]);
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 9 movements");
  });

  it("draws the exact Noto Sans SC character behind 子's final horizontal", () => {
    const paths = byTag(strip.frames[8], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseCharacterOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_CHARACTER.strokes[5], 1),
    );
  });
});

describe("Chinese 谢 — 讠 before 身 before 寸 in twelve cited strokes", () => {
  const steps = ductusSteps(CHINESE_THANK);
  const strip = ductusFilmstrip(CHINESE_THANK, chineseThankOutline);

  it("preserves all five joined turns and eleven lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 1, 2, 3, 4, 4, 4, 5, 6, 7, 8, 9, 10, 10, 11,
    ]);
    expect(strip.frames).toHaveLength(17);
    expect(strip.penLifts).toBe(11);
    expect(strip.summary).toBe("12 strokes · 11 pen lifts · 17 movements");
  });

  it("draws the exact Noto Sans SC character behind 寸's final dot", () => {
    const paths = byTag(strip.frames[16], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseThankOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_THANK.strokes[11], 1),
    );
  });
});

describe("Chinese 请 — 讠 before 青 in ten cited strokes", () => {
  const steps = ductusSteps(CHINESE_PLEASE);
  const strip = ductusFilmstrip(CHINESE_PLEASE, chinesePleaseOutline);

  it("preserves all four joined turns and nine lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([
      0, 1, 1, 1, 2, 3, 4, 5, 6, 7, 7, 7, 8, 9,
    ]);
    expect(strip.frames).toHaveLength(14);
    expect(strip.penLifts).toBe(9);
    expect(strip.summary).toBe("10 strokes · 9 pen lifts · 14 movements");
  });

  it("draws the exact Noto Sans SC character behind 青's final inner horizontal", () => {
    const paths = byTag(strip.frames[13], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chinesePleaseOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_PLEASE.strokes[9], 1),
    );
  });
});

describe("Chinese 再 — central frame before the closing bottom bar", () => {
  const steps = ductusSteps(CHINESE_AGAIN);
  const strip = ductusFilmstrip(CHINESE_AGAIN, chineseAgainOutline);

  it("preserves both joined turns and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 2, 2, 3, 4, 5]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans SC character behind the closing horizontal", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseAgainOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_AGAIN.strokes[5], 1),
    );
  });
});

describe("Chinese 见 — open upper frame before the two lower runs", () => {
  const steps = ductusSteps(CHINESE_SEE);
  const strip = ductusFilmstrip(CHINESE_SEE, chineseSeeOutline);

  it("preserves all three joined turns and three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3, 3, 3]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the hooked second leg", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(chineseSeeOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_SEE.strokes[3], 1),
    );
  });
});

describe("Chinese 什 — complete 亻 before writing 十", () => {
  const steps = ductusSteps(CHINESE_WHAT);
  const strip = ductusFilmstrip(CHINESE_WHAT, chineseWhatOutline);

  it("shows four separate source strokes with three lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true, true]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind 十's final vertical", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseWhatOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_WHAT.strokes[3], 1),
    );
  });
});

describe("Chinese 么 — joined second fall and rightward base sweep", () => {
  const steps = ductusSteps(CHINESE_PARTICLE_ME);
  const strip = ductusFilmstrip(CHINESE_PARTICLE_ME, chineseParticleMeOutline);

  it("preserves the joined turn and two lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false, true]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans SC character behind the final dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseParticleMeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_PARTICLE_ME.strokes[2], 1),
    );
  });
});

describe("Chinese 早 — complete 日 before writing 十 below", () => {
  const steps = ductusSteps(CHINESE_EARLY);
  const strip = ductusFilmstrip(CHINESE_EARLY, chineseEarlyOutline);

  it("preserves the joined top-right corner and five lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 2, 3, 4, 5]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, false, true, true, true, true,
    ]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans SC character behind the final vertical", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseEarlyOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_EARLY.strokes[5], 1),
    );
  });
});

describe("Chinese 上 — vertical before short and long horizontals", () => {
  const steps = ductusSteps(CHINESE_UP);
  const strip = ductusFilmstrip(CHINESE_UP, chineseUpOutline);

  it("preserves three separate sourced strokes and two lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans SC character behind the long base", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      chineseUpOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CHINESE_UP.strokes[2], 1),
    );
  });
});

describe("Devanagari अ — joined left body before shoulder, stem, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_A);
  const strip = ductusFilmstrip(DEVANAGARI_A, devanagariAOutline);

  it("shows five movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariAOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_A.strokes[3], 1),
    );
  });
});

describe("Devanagari आ — joined left body before shoulder, two stems, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AA);
  const strip = ductusFilmstrip(DEVANAGARI_AA, devanagariAaOutline);

  it("shows six movements across five sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 4]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the full headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariAaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_AA.strokes[4], 1),
    );
  });
});

describe("Devanagari इ — continuous double-bowl body before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_I);
  const strip = ductusFilmstrip(DEVANAGARI_I, devanagariIOutline);

  it("shows five movements across two sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the upright from the headline",
      "turn left and curve around the upper bowl without lifting",
      "sweep right through the waist and around the lower bowl",
      "finish down-right through the tail without lifting",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariIOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_I.strokes[1], 1),
    );
  });
});

describe("Devanagari ई — shared double-bowl body before curl and headline", () => {
  const steps = ductusSteps(DEVANAGARI_II);
  const strip = ductusFilmstrip(DEVANAGARI_II, devanagariIiOutline);

  it("shows six movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the upright from the headline",
      "turn left and curve around the upper bowl without lifting",
      "sweep right through the waist and around the lower bowl",
      "finish down-right through the tail without lifting",
      "lift, then sweep the upper curl upward and around to the right",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariIiOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_II.strokes[2], 1),
    );
  });
});

describe("Devanagari उ — joined upper bowl and lower loop before the headline", () => {
  const steps = ductusSteps(DEVANAGARI_U);
  const strip = ductusFilmstrip(DEVANAGARI_U, devanagariUOutline);

  it("shows three movements across two sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve down and left around the upper bowl",
      "sweep back through the waist and around the lower loop without lifting",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariUOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_U.strokes[1], 1),
    );
  });
});

describe("Devanagari ऊ — shared body before the right loop and headline", () => {
  const steps = ductusSteps(DEVANAGARI_UU);
  const strip = ductusFilmstrip(DEVANAGARI_UU, devanagariUuOutline);

  it("shows four movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve down and left around the upper bowl",
      "sweep back through the waist and around the lower loop without lifting",
      "lift, then sweep the right-hand loop up, around, and down-left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariUuOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_UU.strokes[2], 1),
    );
  });
});

describe("Devanagari ए — long stem and tail before short stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_E);
  const strip = ductusFilmstrip(DEVANAGARI_E, devanagariEOutline);

  it("shows four movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long left stem from the headline",
      "curve right through the lower shoulder and sweep down the tail without lifting",
      "lift, then descend the shorter right stem into its inward hook",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariEOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_E.strokes[2], 1),
    );
  });
});

describe("Devanagari ऐ — shared ए base before upper arc and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AI);
  const strip = ductusFilmstrip(DEVANAGARI_AI, devanagariAiOutline);

  it("shows five movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long left stem from the headline",
      "curve right through the lower shoulder and sweep down the tail without lifting",
      "lift, then descend the shorter right stem into its inward hook",
      "lift, then sweep the upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariAiOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_AI.strokes[3], 1),
    );
  });
});

describe("Devanagari ओ — shared आ base before upper arc and headline", () => {
  const steps = ductusSteps(DEVANAGARI_O);
  const strip = ductusFilmstrip(DEVANAGARI_O, devanagariOOutline);

  it("shows seven movements across six sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, false, true, true, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 4, 5]);
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(5);
    expect(strip.summary).toBe("6 strokes · 5 pen lifts · 7 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[6], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariOOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_O.strokes[5], 1),
    );
  });
});

describe("Devanagari औ — shared आ base before two upper arcs and headline", () => {
  const steps = ductusSteps(DEVANAGARI_AU);
  const strip = ductusFilmstrip(DEVANAGARI_AU, devanagariAuOutline);

  it("shows eight movements across seven sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve right around the upper bowl",
      "continue down and around the lower bowl without lifting",
      "lift, then sweep the middle shoulder right",
      "lift, then descend the inner stem",
      "lift, then descend the trailing stem",
      "lift, then sweep the lower upper arc upward and left",
      "lift, then sweep the taller upper arc upward and left",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, false, true, true, true, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 4, 5, 6]);
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(6);
    expect(strip.summary).toBe("7 strokes · 6 pen lifts · 8 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[7], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariAuOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_AU.strokes[6], 1),
    );
  });
});

describe("Devanagari क — counterclockwise bowl before stem, arch, and headline", () => {
  const steps = ductusSteps(DEVANAGARI_KA);
  const strip = ductusFilmstrip(DEVANAGARI_KA, devanagariKaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left over the top and around the bowl",
      "lift, then descend the central stem",
      "lift, then sweep the right-hand arch clockwise",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariKaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_KA.strokes[3], 1),
    );
  });
});

describe("Devanagari ग — continuous loop and ascending stem before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_GA);
  const strip = ductusFilmstrip(DEVANAGARI_GA, devanagariGaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep counterclockwise around the loop and up the joined stem",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariGaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_GA.strokes[2], 1),
    );
  });
});

describe("Devanagari च — upper bar and rounded body before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_CA);
  const strip = ductusFilmstrip(DEVANAGARI_CA, devanagariCaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upper bar right and curve around the open body",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariCaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_CA.strokes[2], 1),
    );
  });
});

describe("Devanagari त — right-to-left shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_TA);
  const strip = ductusFilmstrip(DEVANAGARI_TA, devanagariTaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder and curve down to the open tip",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariTaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_TA.strokes[2], 1),
    );
  });
});

describe("Devanagari द — short stem before the joined outer body, curl, and tail", () => {
  const steps = ductusSteps(DEVANAGARI_DA);
  const strip = ductusFilmstrip(DEVANAGARI_DA, devanagariDaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the short stem",
      "lift, then sweep around the body, inner curl, and tail",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariDaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_DA.strokes[2], 1),
    );
  });
});

describe("Devanagari ध — upper spiral before the lower bowl and lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_DHA);
  const strip = ductusFilmstrip(DEVANAGARI_DHA, devanagariDhaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curl around the upper spiral and sweep right through the shoulder",
      "lift, then sweep down and around the lower bowl",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariDhaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_DHA.strokes[3], 1),
    );
  });
});

describe("Devanagari न — clockwise loop and shoulder before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_NA);
  const strip = ductusFilmstrip(DEVANAGARI_NA, devanagariNaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise around the left loop and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariNaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_NA.strokes[2], 1),
    );
  });
});

describe("Devanagari प — descending left stem curves through the bowl before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_PA);
  const strip = ductusFilmstrip(DEVANAGARI_PA, devanagariPaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem and curve right around the lower bowl",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariPaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_PA.strokes[2], 1),
    );
  });
});

describe("Devanagari ब — counterclockwise oval before the lifted stem and inner diagonal", () => {
  const steps = ductusSteps(DEVANAGARI_BA);
  const strip = ductusFilmstrip(DEVANAGARI_BA, devanagariBaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the oval body",
      "lift, then descend the right stem",
      "lift, then cross the body down and right",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariBaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_BA.strokes[3], 1),
    );
  });
});

describe("Devanagari भ — joined clockwise loops before the lifted right stem", () => {
  const steps = ductusSteps(DEVANAGARI_BHA);
  const strip = ductusFilmstrip(DEVANAGARI_BHA, devanagariBhaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise through both loops and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariBhaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_BHA.strokes[2], 1),
    );
  });
});

describe("Devanagari म — descending left stem joins the clockwise lower loop", () => {
  const steps = ductusSteps(DEVANAGARI_MA);
  const strip = ductusFilmstrip(DEVANAGARI_MA, devanagariMaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem, circle clockwise through the loop, and sweep right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariMaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_MA.strokes[2], 1),
    );
  });
});

describe("Devanagari य — inner curl precedes the restarted lower bowl", () => {
  const steps = ductusSteps(DEVANAGARI_YA);
  const strip = ductusFilmstrip(DEVANAGARI_YA, devanagariYaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve clockwise around the inner curl",
      "lift, then curve around the lower bowl to the right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariYaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_YA.strokes[3], 1),
    );
  });
});

describe("Devanagari र — looped stem precedes the restarted diagonal tail", () => {
  const steps = ductusSteps(DEVANAGARI_RA);
  const strip = ductusFilmstrip(DEVANAGARI_RA, devanagariRaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend and curl clockwise around the lower loop",
      "lift, then draw the diagonal tail down-right",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariRaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_RA.strokes[2], 1),
    );
  });
});

describe("Devanagari ल — open loop precedes the restarted diagonal arm", () => {
  const steps = ductusSteps(DEVANAGARI_LA);
  const strip = ductusFilmstrip(DEVANAGARI_LA, devanagariLaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve up and clockwise around the open left loop",
      "lift, then sweep the diagonal arm up-right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariLaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_LA.strokes[3], 1),
    );
  });
});

describe("Devanagari व — counterclockwise loop before stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_VA);
  const strip = ductusFilmstrip(DEVANAGARI_VA, devanagariVaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the left loop",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariVaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_VA.strokes[2], 1),
    );
  });
});

describe("Devanagari श — joined double-loop body before stem and headline", () => {
  const steps = ductusSteps(DEVANAGARI_SHA);
  const strip = ductusFilmstrip(DEVANAGARI_SHA, devanagariShaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "trace the joined double-loop body and diagonal tail",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariShaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_SHA.strokes[2], 1),
    );
  });
});

describe("Devanagari स — joined hook and tail before crossbar and stems", () => {
  const steps = ductusSteps(DEVANAGARI_SA);
  const strip = ductusFilmstrip(DEVANAGARI_SA, devanagariSaOutline);

  it("shows four movements across four sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend through the hook and diagonal tail",
      "lift, then draw the middle crossbar left-to-right",
      "lift, then descend the right stem",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariSaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_SA.strokes[3], 1),
    );
  });
});

describe("Devanagari ह — joined stem and hooked body before the outer tail", () => {
  const steps = ductusSteps(DEVANAGARI_HA);
  const strip = ductusFilmstrip(DEVANAGARI_HA, devanagariHaOutline);

  it("shows three movements across three sourced strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend, sweep left, and curve around the hooked body",
      "lift, then sweep down-left and through the diagonal tail",
      "lift, then draw the shirorekha left-to-right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false, true, true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Devanagari character behind the headline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      devanagariHaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(DEVANAGARI_HA.strokes[2], 1),
    );
  });
});

describe("Cyrillic а — one joined body and finishing stem", () => {
  const steps = ductusSteps(CYRILLIC_A);
  const strip = ductusFilmstrip(CYRILLIC_A, cyrillicAOutline);

  it("shows two movements within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep over the shoulder and around the round body",
      "continue down the right-hand finishing stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the finishing stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicAOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_A.strokes[0], 1),
    );
  });
});

describe("Cyrillic б — one joined lower body and top flag", () => {
  const steps = ductusSteps(CYRILLIC_BE);
  const strip = ductusFilmstrip(CYRILLIC_BE, cyrillicBeOutline);

  it("shows the body and top flag within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the rounded lower body",
      "continue through the rising shoulder and sweep the top flag right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the top flag", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicBeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_BE.strokes[0], 1),
    );
  });
});

describe("Cyrillic в — one joined upper loop and lower bowl", () => {
  const steps = ductusSteps(CYRILLIC_VE);
  const strip = ductusFilmstrip(CYRILLIC_VE, cyrillicVeOutline);

  it("shows the upper loop and lower bowl within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb through the upper loop and descend to the baseline",
      "continue counterclockwise around the rounded lower bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the lower bowl", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicVeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_VE.strokes[0], 1),
    );
  });
});

describe("Cyrillic г — one zero-lift printed fit for the cursive humps", () => {
  const steps = ductusSteps(CYRILLIC_GE);
  const strip = ductusFilmstrip(CYRILLIC_GE, cyrillicGeOutline);

  it("shows the outward and returning paths within one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the upright and sweep the top bar right",
      "reverse along the top and descend to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the returning path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicGeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_GE.strokes[0], 1),
    );
  });
});

describe("Cyrillic д — one zero-lift body and retraced printed base", () => {
  const steps = ductusSteps(CYRILLIC_DE);
  const strip = ductusFilmstrip(CYRILLIC_DE, cyrillicDeOutline);

  it("shows the closed body before the joined base-and-feet movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle counterclockwise around the closed body",
      "descend, retrace both feet, and finish along the base shelf",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicDeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_DE.strokes[0], 1),
    );
  });
});

describe("Cyrillic е — one zero-lift upper loop and lower bowl", () => {
  const steps = ductusSteps(CYRILLIC_IE);
  const strip = ductusFilmstrip(CYRILLIC_IE, cyrillicIeOutline);

  it("shows the upper bowl and middle crossing before the lower bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve around the upper bowl and sweep through the middle",
      "reverse through the middle and circle the lower bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicIeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_IE.strokes[0], 1),
    );
  });
});

describe("Cyrillic ё — looped body followed by two lifted dots", () => {
  const steps = ductusSteps(CYRILLIC_IO);
  const strip = ductusFilmstrip(CYRILLIC_IO, cyrillicIoOutline);

  it("shows the joined body before the left and right dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve around the upper bowl and sweep through the middle",
      "reverse through the middle and circle the lower bowl",
      "lift and place the left dot",
      "lift again and place the right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact dotted Noto Sans Cyrillic glyph behind all three runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicIoOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(2);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_IO.strokes[2], 1),
    );
  });
});

describe("Cyrillic ж — one continuous left-centre-right run", () => {
  const steps = ductusSteps(CYRILLIC_ZHE);
  const strip = ductusFilmstrip(CYRILLIC_ZHE, cyrillicZheOutline);

  it("shows the left wings and centre before the joined right wings", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "trace the left wings and rise through the centre",
      "retrace the centre and trace the right wings",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicZheOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_ZHE.strokes[0], 1),
    );
  });
});

describe("Cyrillic з — one continuous double-lobe run", () => {
  const steps = ductusSteps(CYRILLIC_ZE);
  const strip = ductusFilmstrip(CYRILLIC_ZE, cyrillicZeOutline);

  it("shows the smaller upper lobe before the joined larger lower lobe", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the smaller upper lobe and descend through the middle",
      "circle the larger lower lobe and finish at the lower right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicZeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_ZE.strokes[0], 1),
    );
  });
});

describe("Cyrillic и — one continuous stem-diagonal-stem run", () => {
  const steps = ductusSteps(CYRILLIC_I);
  const strip = ductusFilmstrip(CYRILLIC_I, cyrillicIOutline);

  it("shows the left stem, rising diagonal, and right stem without a lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise diagonally to the upper right",
      "descend the right stem and finish at the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicIOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_I.strokes[0], 1),
    );
  });
});

describe("Cyrillic й — joined body followed by a lifted breve", () => {
  const steps = ductusSteps(CYRILLIC_SHORT_I);
  const strip = ductusFilmstrip(CYRILLIC_SHORT_I, cyrillicShortIOutline);

  it("shows the three-part body before the separately drawn breve", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise diagonally to the upper right",
      "descend the right stem and finish at the baseline",
      "lift, then draw the breve from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the joined body visible over the exact breve-bearing glyph", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicShortIOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(CYRILLIC_SHORT_I.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_SHORT_I.strokes[1], 1),
    );
  });
});

describe("Cyrillic к — one joined stem-and-arms school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_KA);
  const strip = ductusFilmstrip(CYRILLIC_KA, cyrillicKaOutline);

  it("shows the descending stem before the upper and lower arms", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "rise through the upper arm and return to the middle junction",
      "continue down-right through the lower arm to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicKaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_KA.strokes[0], 1),
    );
  });
});

describe("Cyrillic л — one joined hook-to-legs school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EL);
  const strip = ductusFilmstrip(CYRILLIC_EL, cyrillicElOutline);

  it("shows the hooked left leg before the top shoulder and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve from the baseline hook up the left leg",
      "sweep right along the top shoulder",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicElOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_EL.strokes[0], 1),
    );
  });
});

describe("Cyrillic м — one joined two-arch school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EM);
  const strip = ductusFilmstrip(CYRILLIC_EM, cyrillicEmOutline);

  it("shows the left stem before the central valley, second apex, and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "rise from the baseline through the left stem",
      "descend diagonally to the central valley",
      "rise diagonally to the second apex",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicEmOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_EM.strokes[0], 1),
    );
  });
});

describe("Cyrillic н — one joined middle-bridge school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_EN);
  const strip = ductusFilmstrip(CYRILLIC_EN, cyrillicEnOutline);

  it("shows the left stem before the middle bridge and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace to the middle bridge and rise to the upper right",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicEnOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_EN.strokes[0], 1),
    );
  });
});

describe("Cyrillic о — one closed counterclockwise school-hand oval", () => {
  const steps = ductusSteps(CYRILLIC_O);
  const strip = ductusFilmstrip(CYRILLIC_O, cyrillicOOutline);

  it("shows the top and left side before the bottom, right side, and closure", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve left over the top and descend the left side",
      "sweep through the bottom and rise to close the oval",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the closed path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicOOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_O.strokes[0], 1),
    );
  });
});

describe("Cyrillic п — one joined top-shoulder school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_PE);
  const strip = ductusFilmstrip(CYRILLIC_PE, cyrillicPeOutline);

  it("shows the left stem before the top shoulder and right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace to the top shoulder and sweep right",
      "descend the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicPeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_PE.strokes[0], 1),
    );
  });
});

describe("Cyrillic р — one joined descender-and-bowl school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_ER);
  const strip = ductusFilmstrip(CYRILLIC_ER, cyrillicErOutline);

  it("shows the descender before the retraced shoulder and closed bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the stem below the baseline",
      "retrace to the upper shoulder and curve right",
      "sweep around the bowl and return to the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicErOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_ER.strokes[0], 1),
    );
  });
});

describe("Cyrillic с — one open counterclockwise school-hand run", () => {
  const steps = ductusSteps(CYRILLIC_ES);
  const strip = ductusFilmstrip(CYRILLIC_ES, cyrillicEsOutline);

  it("shows the upper-left sweep before the lower-right exit", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curve left over the top and descend the left side",
      "sweep through the bottom and rise to the lower-right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the open curve", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicEsOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_ES.strokes[0], 1),
    );
  });
});

describe("Cyrillic т — one joined central-stem-and-top-bar run", () => {
  const steps = ductusSteps(CYRILLIC_TE);
  const strip = ductusFilmstrip(CYRILLIC_TE, cyrillicTeOutline);

  it("shows the central descent before both halves of the top bar", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the central stem to the baseline",
      "retrace to the top junction and sweep left",
      "retrace through the junction and sweep to the right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicTeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_TE.strokes[0], 1),
    );
  });
});

describe("Cyrillic у — one joined upper-body-and-descender run", () => {
  const steps = ductusSteps(CYRILLIC_U);
  const strip = ductusFilmstrip(CYRILLIC_U, cyrillicUOutline);

  it("shows both upper arms before the long left-curving terminal", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left arm to the middle junction",
      "turn and rise through the right arm",
      "retrace to the junction and descend below the baseline",
      "curve left through the descender terminal",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicUOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_U.strokes[0], 1),
    );
  });
});

describe("Cyrillic ф — stem first, then one joined two-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_EF);
  const strip = ductusFilmstrip(CYRILLIC_EF, cyrillicEfOutline);

  it("shows the long stem before the lifted left-to-right bowl sequence", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long central stem below the baseline",
      "lift and curve over and around the left bowl",
      "sweep through the lower-left curve to the centre",
      "continue through the lower-right curve",
      "rise over the right bowl to the upper junction",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1, 1, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind both runs", () => {
    const paths = byTag(strip.frames[4], "path");
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    const pen = paths.find((path) => path.attrs.class === "ductus__pen")!;
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicEfOutline.path,
    );
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(CYRILLIC_EF.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(CYRILLIC_EF.strokes[1], 1));
  });
});

describe("Cyrillic х — two facing curves fitted through one printed crossing", () => {
  const steps = ductusSteps(CYRILLIC_HA);
  const strip = ductusFilmstrip(CYRILLIC_HA, cyrillicHaOutline);

  it("shows the complete left run before the lifted right run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper-left tip to the central crossing",
      "sweep down-left from the crossing to the lower-left tip",
      "lift and descend from the upper-right tip to the crossing",
      "sweep down-right from the crossing to the lower-right tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    const pen = paths.find((path) => path.attrs.class === "ductus__pen")!;
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicHaOutline.path,
    );
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(CYRILLIC_HA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(CYRILLIC_HA.strokes[1], 1));
  });
});

describe("Cyrillic ц — one joined stem-to-stem-to-tail run", () => {
  const steps = ductusSteps(CYRILLIC_TSE);
  const strip = ductusFilmstrip(CYRILLIC_TSE, cyrillicTseOutline);

  it("keeps the square printed body and descender in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "sweep along the base and rise through the right stem",
      "retrace the right stem and cross the tail shoulder",
      "descend the short tail below the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicTseOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_TSE.strokes[0], 1),
    );
  });
});

describe("Cyrillic ч — one joined short-stem-to-bowl-to-long-stem run", () => {
  const steps = ductusSteps(CYRILLIC_CHE);
  const strip = ductusFilmstrip(CYRILLIC_CHE, cyrillicCheOutline);

  it("keeps the shorter left stem, bowl, and full right stem in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the short left stem to the middle join",
      "sweep through the bowl and rise along the right stem",
      "descend the full right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicCheOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_CHE.strokes[0], 1),
    );
  });
});

describe("Cyrillic ш — one joined three-stem run", () => {
  const steps = ductusSteps(CYRILLIC_SHA);
  const strip = ductusFilmstrip(CYRILLIC_SHA, cyrillicShaOutline);

  it("keeps all three stems and both base joins in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "cross the first base join and rise through the middle stem",
      "retrace the middle stem to the baseline",
      "cross the second base join and rise through the right stem",
      "retrace the right stem to the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicShaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_SHA.strokes[0], 1),
    );
  });
});

describe("Cyrillic щ — one joined three-stem-to-tail run", () => {
  const steps = ductusSteps(CYRILLIC_SHCHA);
  const strip = ductusFilmstrip(CYRILLIC_SHCHA, cyrillicShchaOutline);

  it("keeps all three stems, both joins, and the tail in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "cross the first base join and rise through the middle stem",
      "retrace the middle stem to the baseline",
      "cross the second base join and rise through the right stem",
      "retrace the right stem and cross the tail shoulder",
      "descend the short tail below the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 6 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicShchaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_SHCHA.strokes[0], 1),
    );
  });
});

describe("Cyrillic ъ — one joined flag-to-stem-to-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_HARD_SIGN);
  const strip = ductusFilmstrip(CYRILLIC_HARD_SIGN, cyrillicHardSignOutline);

  it("keeps the top flag, descending stem, and lower bowl in source order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right along the broad top flag",
      "descend the main stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined path", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicHardSignOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_HARD_SIGN.strokes[0], 1),
    );
  });
});

describe("Cyrillic ы — joined left body before a lifted right stem", () => {
  const steps = ductusSteps(CYRILLIC_YERY);
  const strip = ductusFilmstrip(CYRILLIC_YERY, cyrillicYeryOutline);

  it("keeps the left stem and bowl together before the separate right stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
      "lift, then descend the separate right stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the final stem", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicYeryOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(CYRILLIC_YERY.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_YERY.strokes[1], 1),
    );
  });
});

describe("Cyrillic ь — one joined stem-and-bowl run", () => {
  const steps = ductusSteps(CYRILLIC_SOFT_SIGN);
  const strip = ductusFilmstrip(CYRILLIC_SOFT_SIGN, cyrillicSoftSignOutline);

  it("keeps the descending stem joined to the counterclockwise lower bowl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the stem to the baseline",
      "sweep right along the lower bowl",
      "curve upward around the bowl's right side",
      "return left through the upper bowl to close against the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the closed bowl", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicSoftSignOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_SOFT_SIGN.strokes[0], 1),
    );
  });
});

describe("Cyrillic э — outer curve before a lifted middle tongue", () => {
  const steps = ductusSteps(CYRILLIC_E);
  const strip = ductusFilmstrip(CYRILLIC_E, cyrillicEOutline);

  it("keeps the backwards-C run before the right-to-left tongue", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right across the upper curve",
      "continue down around the outer right side",
      "sweep left through the lower curve",
      "lift, then draw the middle tongue right-to-left",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the final tongue", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicEOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(CYRILLIC_E.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_E.strokes[1], 1),
    );
  });
});

describe("Cyrillic ю — one joined stem-to-oval run", () => {
  const steps = ductusSteps(CYRILLIC_YU);
  const strip = ductusFilmstrip(CYRILLIC_YU, cyrillicYuOutline);

  it("keeps the left stem and connector joined to the clockwise oval", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left stem to the baseline",
      "retrace upward and sweep right along the middle bar",
      "curve upward around the oval and across its top",
      "continue down around the oval's right side",
      "sweep left through the bottom and rise to close",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the closed oval", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicYuOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_YU.strokes[0], 1),
    );
  });
});

describe("Cyrillic я — one joined rise-to-loop-to-leg run", () => {
  const steps = ductusSteps(CYRILLIC_YA);
  const strip = ductusFilmstrip(CYRILLIC_YA, cyrillicYaOutline);

  it("keeps the rising stem, counterclockwise bowl, and diagonal leg joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "climb the right stem from the baseline to the top",
      "curve counterclockwise around the upper bowl",
      "sweep left through the bowl's lower join",
      "descend the diagonal leg to the lower-left tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Cyrillic character behind the joined run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      cyrillicYaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(CYRILLIC_YA.strokes[0], 1),
    );
  });
});

describe("Gujarati અ — joined body before the lifted right stem", () => {
  const steps = ductusSteps(GUJARATI_A);
  const strip = ductusFilmstrip(GUJARATI_A, gujaratiAOutline);

  it("shows the three-part body before the lifted stem and foot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep clockwise around the open left curve",
      "continue through the lower body and rise into the middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the right stem into its foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiAOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_A.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_A.strokes[1], 1),
    );
  });
});

describe("Gujarati આ — complete અ before the lifted trailing ā stem", () => {
  const steps = ductusSteps(GUJARATI_AA);
  const strip = ductusFilmstrip(GUJARATI_AA, gujaratiAaOutline);

  it("shows the joined body before two separately descended stems", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep clockwise around the open left curve",
      "continue through the lower body and rise into the middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing ā stem into its foot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiAaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_AA.strokes[0], 1),
      penPathD(GUJARATI_AA.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_AA.strokes[2], 1),
    );
  });
});

describe("Gujarati ઇ — two loops flow into the rising hook without a lift", () => {
  const steps = ductusSteps(GUJARATI_I);
  const strip = ductusFilmstrip(GUJARATI_I, gujaratiIOutline);

  it("shows the upper loop, crossing, lower loop, and hook as one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper-left loop down to the middle crossing",
      "continue through the narrow crossing",
      "sweep clockwise around the broad lower loop",
      "rise along the right side into the upper hook",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiIOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_I.strokes[0], 1),
    );
  });
});

describe("Gujarati ઈ — the ઇ run rises into a taller clockwise curl", () => {
  const steps = ductusSteps(GUJARATI_II);
  const strip = ductusFilmstrip(GUJARATI_II, gujaratiIiOutline);

  it("shows both loops before the extended top curl in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper-left loop down to the middle crossing",
      "continue through the narrow crossing",
      "sweep clockwise around the broad lower loop",
      "rise and curl clockwise around the extended top hook",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiIiOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_II.strokes[0], 1),
    );
  });
});

describe("Gujarati ઉ — two bowls return around one tall outer curve", () => {
  const steps = ductusSteps(GUJARATI_U);
  const strip = ductusFilmstrip(GUJARATI_U, gujaratiUOutline);

  it("shows the upper bowl, lower bowl, and returning curve in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise through the small upper bowl to the middle cusp",
      "continue right and sweep clockwise around the broad lower bowl",
      "climb around the tall outer-left curve and finish at the upper right",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiUOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_U.strokes[0], 1),
    );
  });
});

describe("Gujarati ઊ — the complete ઉ run descends a long right tail", () => {
  const steps = ductusSteps(GUJARATI_UU);
  const strip = ductusFilmstrip(GUJARATI_UU, gujaratiUuOutline);

  it("shows the complete ઉ body before its extended tail in one run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write ઉ through its upper bowl, middle cusp, and lower bowl",
      "continue around the tall outer-left curve",
      "cross the high shoulder and descend the long right tail into its foot",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiUuOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_UU.strokes[0], 1),
    );
  });
});

describe("Gujarati ઋ — bent body, central stem, then right loop and tail", () => {
  const steps = ductusSteps(GUJARATI_VOCALIC_R);
  const strip = ductusFilmstrip(GUJARATI_VOCALIC_R, gujaratiVocalicROutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep right along the upper body, then turn diagonally down-left",
      "lift, then descend the central stem into its foot",
      "lift again, circle the right loop, and descend through the tail",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiVocalicROutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(2);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_VOCALIC_R.strokes[2], 1),
    );
  });
});

describe("Gujarati એ — joined body, right stem, then high arc", () => {
  const steps = ductusSteps(GUJARATI_E);
  const strip = ductusFilmstrip(GUJARATI_E, gujaratiEOutline);

  it("shows four movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle clockwise around the left bowl",
      "continue through the lower body and small right arch",
      "lift, then descend the full-height right stem into its foot",
      "lift again and sweep the high arcing mark from left to right",
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiEOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(2);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_E.strokes[2], 1),
    );
  });
});

describe("Gujarati ઐ — the એ sequence gains a second high arc", () => {
  const steps = ductusSteps(GUJARATI_AI);
  const strip = ductusFilmstrip(GUJARATI_AI, gujaratiAiOutline);

  it("shows the body, stem, lower arc, then higher arc as four runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write એ through its joined bowl, lower body, and right arch",
      "lift, then descend the full-height right stem into its foot",
      "lift again and sweep the lower high arc from left to right",
      "lift once more and sweep the higher arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all four runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiAiOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(3);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_AI.strokes[3], 1),
    );
  });
});

describe("Gujarati ઓ — the complete આ sequence gains a high arc", () => {
  const steps = ductusSteps(GUJARATI_O);
  const strip = ductusFilmstrip(GUJARATI_O, gujaratiOOutline);

  it("shows the body, two stems, then high arc as four runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write આ through its open left curve",
      "continue through the lower body and middle shoulder",
      "retrace down and sweep through the small right arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing stem into its foot",
      "lift once more and sweep the high arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 6 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all four runs", () => {
    const paths = byTag(strip.frames[5], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiOOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(3);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_O.strokes[3], 1),
    );
  });
});

describe("Gujarati ઔ — the ઓ sequence gains a second high arc", () => {
  const steps = ductusSteps(GUJARATI_AU);
  const strip = ductusFilmstrip(GUJARATI_AU, gujaratiAuOutline);

  it("shows the body, two stems, lower arc, then higher arc as five runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "write ઓ through its open left curve, lower body, and arch",
      "lift, then descend the first right stem into its foot",
      "lift again, then descend the trailing stem into its foot",
      "lift once more and sweep the lower high arc left to right",
      "lift again and sweep the higher arc from left to right",
    ]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 5 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all five runs", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiAuOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(4);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_AU.strokes[4], 1),
    );
  });
});

describe("Gujarati ક — joined loop-body before the crossing diagonal", () => {
  const steps = ductusSteps(GUJARATI_KA);
  const strip = ductusFilmstrip(GUJARATI_KA, gujaratiKaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper loop and continue through the rounded lower body",
      "lift, then sweep the diagonal cross-stroke lower-left to upper-right",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiKaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_KA.strokes[1], 1),
    );
  });
});

describe("Gujarati ખ — joined left body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_KHA);
  const strip = ductusFilmstrip(GUJARATI_KHA, gujaratiKhaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend through the left lobe and curl right through the middle",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiKhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_KHA.strokes[1], 1),
    );
  });
});

describe("Gujarati ગ — rounded body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_GA);
  const strip = ductusFilmstrip(GUJARATI_GA, gujaratiGaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded body from upper left to lower left",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiGaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_GA.strokes[1], 1),
    );
  });
});

describe("Gujarati ઘ — joined double body before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_GHA);
  const strip = ductusFilmstrip(GUJARATI_GHA, gujaratiGhaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper lobe, turn through the middle, and round the lower body",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiGhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_GHA.strokes[1], 1),
    );
  });
});

describe("Gujarati ઙ — S-like body before the separate upper-right dot", () => {
  const steps = ductusSteps(GUJARATI_NGA);
  const strip = ductusFilmstrip(GUJARATI_NGA, gujaratiNgaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep from the upper right through the S-like body to the lower left",
      "lift, then circle the separate upper-right dot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiNgaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_NGA.strokes[1], 1),
    );
  });
});

describe("Gujarati ચ — joined bowls before the separate right spine", () => {
  const steps = ductusSteps(GUJARATI_CA);
  const strip = ductusFilmstrip(GUJARATI_CA, gujaratiCaOutline);

  it("shows two movements across two ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper bowl, turn through the middle loop, and round the lower body",
      "lift, then descend the right spine and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiCaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(1);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_CA.strokes[1], 1),
    );
  });
});

describe("Gujarati છ — both upper lobes join through one continuous body", () => {
  const steps = ductusSteps(GUJARATI_CHA);
  const strip = ductusFilmstrip(GUJARATI_CHA, gujaratiChaOutline);

  it("shows three connected movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper-left lobe and turn back through the middle",
      "continue around the broad lower body and climb the outer right curve",
      "circle the upper-right lobe and finish beside the outer curve",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiChaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_CHA.strokes[0], 1),
    );
  });
});

describe("Gujarati જ — both loops join through the crossing and exit", () => {
  const steps = ductusSteps(GUJARATI_JA);
  const strip = ductusFilmstrip(GUJARATI_JA, gujaratiJaOutline);

  it("shows three connected movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper-left loop",
      "continue diagonally through the crossing body",
      "circle the lower-right loop and sweep into the upper-right exit",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiJaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_JA.strokes[0], 1),
    );
  });
});

describe("Gujarati ઝ — left body before right loop and upper stem", () => {
  const steps = ductusSteps(GUJARATI_JHA);
  const strip = ductusFilmstrip(GUJARATI_JHA, gujaratiJhaOutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded left body from upper left to lower left",
      "lift, then circle the right loop and finish through its lower tail",
      "lift again, then descend the short upper stem",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiJhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_JHA.strokes[0], 1),
      penPathD(GUJARATI_JHA.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_JHA.strokes[2], 1),
    );
  });
});

describe("Gujarati ઞ — left body before shoulder and tall spine", () => {
  const steps = ductusSteps(GUJARATI_NYA);
  const strip = ductusFilmstrip(GUJARATI_NYA, gujaratiNyaOutline);

  it("shows three movements across three ordered pen-down runs", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded left body from upper left to lower left",
      "lift, then sweep the short rightward shoulder",
      "lift again, then descend the tall spine and curl through its terminal",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiNyaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_NYA.strokes[0], 1),
      penPathD(GUJARATI_NYA.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_NYA.strokes[2], 1),
    );
  });
});

describe("Gujarati ટ — upper turn and lower bowl stay joined", () => {
  const steps = ductusSteps(GUJARATI_TTA);
  const strip = ductusFilmstrip(GUJARATI_TTA, gujaratiTtaOutline);

  it("shows the complete joined form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the upper turn, bend down-left, and circle the lower bowl",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiTtaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_TTA.strokes[0], 1),
    );
  });
});

describe("Gujarati ઠ — high shoulder, outer bowl, and inward curl stay joined", () => {
  const steps = ductusSteps(GUJARATI_TTHA);
  const strip = ductusFilmstrip(GUJARATI_TTHA, gujaratiTthaOutline);

  it("shows the complete joined form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder, circle the lower bowl, and curl inward",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiTthaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_TTHA.strokes[0], 1),
    );
  });
});

describe("Gujarati ડ — high shoulder and lower bowl stay joined", () => {
  const steps = ductusSteps(GUJARATI_DDA);
  const strip = ductusFilmstrip(GUJARATI_DDA, gujaratiDdaOutline);

  it("shows the complete descending form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the shoulder, descend through the middle, and round the lower bowl",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiDdaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_DDA.strokes[0], 1),
    );
  });
});

describe("Gujarati ઢ — outer bowl flows into the inner loop", () => {
  const steps = ductusSteps(GUJARATI_DDHA);
  const strip = ductusFilmstrip(GUJARATI_DDHA, gujaratiDdhaOutline);

  it("shows the complete looped form as one movement", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the upper shoulder, round the outer bowl, and circle the inner loop",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the exact Noto Sans Gujarati character behind the continuous run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiDdhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_DDHA.strokes[0], 1),
    );
  });
});

describe("Gujarati ણ — hooked body before bowl and right spine", () => {
  const steps = ductusSteps(GUJARATI_NNA);
  const strip = ductusFilmstrip(GUJARATI_NNA, gujaratiNnaOutline);

  it("shows the three source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the left spine and sweep through the hooked lower tail",
      "lift, then circle the separate middle bowl",
      "lift again, descend the tall right spine, and turn through its foot",
    ]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the exact Noto Sans Gujarati character behind all three runs", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiNnaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_NNA.strokes[0], 1),
      penPathD(GUJARATI_NNA.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_NNA.strokes[2], 1),
    );
  });
});

describe("Gujarati ત — open body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_TA);
  const strip = ductusFilmstrip(GUJARATI_TA, gujaratiTaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep from the lower terminal around the open body and across the upper shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiTaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_TA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_TA.strokes[1], 1),
    );
  });
});

describe("Gujarati થ — looped body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_THA);
  const strip = ductusFilmstrip(GUJARATI_THA, gujaratiThaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small upper loop, descend, and sweep around the broad body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiThaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_THA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_THA.strokes[1], 1),
    );
  });
});

describe("Gujarati દ — one continuous upper and lower body", () => {
  const steps = ductusSteps(GUJARATI_DA);
  const strip = ductusFilmstrip(GUJARATI_DA, gujaratiDaOutline);

  it("shows the single source run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the upper body, narrow through the middle, and sweep around the lower body into its terminal",
    ]);
    expect(strip.frames).toHaveLength(1);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
  });

  it("draws the exact Noto Sans Gujarati character behind the run", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiDaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_DA.strokes[0], 1),
    );
  });
});

describe("Gujarati ધ — joined body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_DHA);
  const strip = ductusFilmstrip(GUJARATI_DHA, gujaratiDhaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the high entry through the turns and sweep around the broad body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiDhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_DHA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_DHA.strokes[1], 1),
    );
  });
});

describe("Gujarati ન — loop and shoulder before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_NA);
  const strip = ductusFilmstrip(GUJARATI_NA, gujaratiNaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the small left loop and continue across the long rightward shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiNaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_NA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_NA.strokes[1], 1),
    );
  });
});

describe("Gujarati પ — hooked lower body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_PA);
  const strip = ductusFilmstrip(GUJARATI_PA, gujaratiPaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "curl over the high left hook, descend, and sweep around the broad lower body into the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiPaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_PA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_PA.strokes[1], 1),
    );
  });
});

describe("Gujarati ફ — winding body before the diagonal cross-stroke", () => {
  const steps = ductusSteps(GUJARATI_PHA);
  const strip = ductusFilmstrip(GUJARATI_PHA, gujaratiPhaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep left across the high cap, wind around the body and lower-left loop, then exit through the tail",
      "lift and draw the diagonal cross-stroke from lower left to upper right",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiPhaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_PHA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_PHA.strokes[1], 1),
    );
  });
});

describe("Gujarati બ — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_BA);
  const strip = ductusFilmstrip(GUJARATI_BA, gujaratiBaOutline);

  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "circle the rounded body, wind through the inner turn, and exit across the right shoulder",
      "lift, descend the tall right spine, and turn through its lower foot",
    ]);
    expect(strip.frames).toHaveLength(2);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      gujaratiBaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d)).toEqual([
      penPathD(GUJARATI_BA.strokes[0], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(GUJARATI_BA.strokes[1], 1),
    );
  });
});

describe("Gujarati ભ — broad loop before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_BHA);
  const strip = ductusFilmstrip(GUJARATI_BHA, gujaratiBhaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character behind both runs", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiBhaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_BHA.strokes[1], 1));
  });
});

describe("Gujarati મ — left body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_MA);
  const strip = ductusFilmstrip(GUJARATI_MA, gujaratiMaOutline);
  it("shows two source runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiMaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_MA.strokes[1], 1));
  });
});

describe("Gujarati ય — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_YA);
  const strip = ductusFilmstrip(GUJARATI_YA, gujaratiYaOutline);
  it("shows two source runs", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiYaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_YA.strokes[1], 1));
  });
});

describe("Gujarati ર — upper body, middle loop, and tail stay joined", () => {
  const steps = ductusSteps(GUJARATI_RA);
  const strip = ductusFilmstrip(GUJARATI_RA, gujaratiRaOutline);
  it("shows one continuous source run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiRaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_RA.strokes[0], 1));
  });
});

describe("Gujarati લ — broad body before shoulder and tall spine", () => {
  const steps = ductusSteps(GUJARATI_LA);
  const strip = ductusFilmstrip(GUJARATI_LA, gujaratiLaOutline);
  it("shows the three source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
    expect(strip.frames).toHaveLength(3);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiLaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_LA.strokes[2], 1));
  });
});

describe("Gujarati ળ — left bowl flows through the arch into the tall spine", () => {
  const steps = ductusSteps(GUJARATI_LLA);
  const strip = ductusFilmstrip(GUJARATI_LLA, gujaratiLlaOutline);
  it("shows one continuous source run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiLlaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_LLA.strokes[0], 1));
  });
});

describe("Gujarati વ — rounded body before the separate tall spine", () => {
  const steps = ductusSteps(GUJARATI_VA);
  const strip = ductusFilmstrip(GUJARATI_VA, gujaratiVaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiVaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_VA.strokes[1], 1));
  });
});

describe("Gujarati શ — upper loop and lower body before the tall spine", () => {
  const steps = ductusSteps(GUJARATI_SHA);
  const strip = ductusFilmstrip(GUJARATI_SHA, gujaratiShaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiShaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_SHA.strokes[1], 1));
  });
});

describe("Gujarati સ — rounded loop and shoulder before the tall spine", () => {
  const steps = ductusSteps(GUJARATI_SA);
  const strip = ductusFilmstrip(GUJARATI_SA, gujaratiSaOutline);
  it("shows the two source runs in order", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(strip.frames).toHaveLength(2);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiSaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_SA.strokes[1], 1));
  });
});

describe("Gujarati હ — upper loop flowing into the broad lower bowl", () => {
  const steps = ductusSteps(GUJARATI_HA);
  const strip = ductusFilmstrip(GUJARATI_HA, gujaratiHaOutline);
  it("shows the single source run without a lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
  });
  it("draws the exact Noto Sans Gujarati character", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(gujaratiHaOutline.path);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(penPathD(GUJARATI_HA.strokes[0], 1));
  });
});

describe("Hebrew א — two crossed handwritten runs fitted to the block outline", () => {
  const steps = ductusSteps(HEBREW_ALEF);
  const strip = ductusFilmstrip(HEBREW_ALEF, hebrewAlefOutline);

  it("shows the main diagonal before the lifted opposing run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the main diagonal down and right",
      "lift, then descend from the upper-right arm to the crossing",
      "continue through the crossing and down the lower-left leg",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("keeps the first run visible over the vendored Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_ALEF.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_ALEF.strokes[1], 1),
    );
  });
});

describe("Hebrew ב — its top and right side precede the lifted baseline", () => {
  const steps = ductusSteps(HEBREW_BET);
  const strip = ductusFilmstrip(HEBREW_BET, hebrewBetOutline);

  it("shows the sourced three movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the baseline from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("keeps the joined top-and-right stroke over the Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewBetOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_BET.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_BET.strokes[1], 1),
    );
  });
});

describe("Hebrew ג — its joined top and right leg precede the lifted left leg", () => {
  const steps = ductusSteps(HEBREW_GIMEL);
  const strip = ductusFilmstrip(HEBREW_GIMEL, hebrewGimelOutline);

  it("shows the sourced four movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short top bar from left to right",
      "continue down the right stem without lifting",
      "continue into the short lower-right leg",
      "lift, restart at the lower junction, and draw the longer leg down-left",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("keeps the first angular run visible over the Noto Sans Hebrew outline", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewGimelOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_GIMEL.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_GIMEL.strokes[1], 1),
    );
  });
});

describe("Hebrew ד — one sourced curve fitted to the angular block outline", () => {
  const steps = ductusSteps(HEBREW_DALET);
  const strip = ductusFilmstrip(HEBREW_DALET, hebrewDaletOutline);

  it("keeps the top bar and right descent in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue around the sharp right corner and down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the continuous path over Noto Sans Hebrew without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewDaletOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_DALET.strokes[0], 2),
    );
  });
});

describe("Hebrew ה — joined top and right body plus a detached left leg", () => {
  const steps = ductusSteps(HEBREW_HEI);
  const strip = ductusFilmstrip(HEBREW_HEI, hebrewHeiOutline);

  it("keeps the top and right side joined before restarting the left leg", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the detached left leg from top to bottom",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Sans Hebrew and preserves the completed body behind the detached leg", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewHeiOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_HEI.strokes[0], 2));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_HEI.strokes[1], 1),
    );
  });
});

describe("Hebrew ו — one joined head-and-stem stroke", () => {
  const steps = ductusSteps(HEBREW_VAV);
  const strip = ductusFilmstrip(HEBREW_VAV, hebrewVavOutline);

  it("keeps the small head joined to the top-to-bottom stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the small head from left to right",
      "continue straight down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws Noto Sans Hebrew with no completed-stroke overlay before the stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewVavOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_VAV.strokes[0], 2),
    );
  });
});

describe("Hebrew ז — one joined head-and-curved-stem stroke", () => {
  const steps = ductusSteps(HEBREW_ZAYIN);
  const strip = ductusFilmstrip(HEBREW_ZAYIN, hebrewZayinOutline);

  it("keeps the short head joined to the curved descent", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short head from left to right",
      "continue down through the curved stem without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws Noto Sans Hebrew with no completed-stroke overlay before the stem", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewZayinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_ZAYIN.strokes[0], 2),
    );
  });
});

describe("Hebrew ח — joined top and right body plus a joined left leg", () => {
  const steps = ductusSteps(HEBREW_HEIT);
  const strip = ductusFilmstrip(HEBREW_HEIT, hebrewHeitOutline);

  it("keeps the top and right side joined before restarting the left leg", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then draw the joined left leg from top to bottom",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Sans Hebrew and preserves the completed body behind the left leg", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewHeitOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_HEIT.strokes[0], 2));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_HEIT.strokes[1], 1),
    );
  });
});

describe("Hebrew ט — left-and-base body plus a bottom-up hooked side", () => {
  const steps = ductusSteps(HEBREW_TET);
  const strip = ductusFilmstrip(HEBREW_TET, hebrewTetOutline);

  it("keeps each body pair joined with one restart at the lower right", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the left side from top to bottom",
      "continue around the bottom from left to right without lifting",
      "lift, restart at the lower-right, and climb the right side",
      "turn down-left into the inward hook without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws Noto Sans Hebrew and preserves the first body behind the hooked side", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewTetOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(HEBREW_TET.strokes[0], 2));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_TET.strokes[1], 2),
    );
  });
});

describe("Hebrew י — one tiny joined head-and-stem stroke", () => {
  const steps = ductusSteps(HEBREW_YOD);
  const strip = ductusFilmstrip(HEBREW_YOD, hebrewYodOutline);

  it("keeps the tiny head joined to its short stem", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the small head from left to right",
      "continue down through the short angled stem without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact compact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewYodOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_YOD.strokes[0], 2),
    );
  });
});

describe("Hebrew כ — one continuous sharp-cornered half-circle", () => {
  const steps = ductusSteps(HEBREW_KAF);
  const strip = ductusFilmstrip(HEBREW_KAF, hebrewKafOutline);

  it("keeps the top, rounded side, and base in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the rounded right side without lifting",
      "turn left along the base without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew Kaf in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewKafOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_KAF.strokes[0], 2),
    );
  });
});

describe("Hebrew ל — one tall angular run", () => {
  const steps = ductusSteps(HEBREW_LAMED);
  const strip = ductusFilmstrip(HEBREW_LAMED, hebrewLamedOutline);

  it("keeps the tall stroke, middle bar, and diagonal lower stroke joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the tall left stroke from top to bottom",
      "continue right along the middle bar without lifting",
      "turn diagonally down-left through the lower stroke without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact tall Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewLamedOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_LAMED.strokes[0], 2),
    );
  });
});

describe("Hebrew מ — detached angled part, then one joined angular body", () => {
  const steps = ductusSteps(HEBREW_MEM);
  const strip = ductusFilmstrip(HEBREW_MEM, hebrewMemOutline);

  it("shows the source's five movements across two strokes", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the detached left part from its lower tip up to the corner",
      "turn down-right through its short inner leg without lifting",
      "lift, then climb diagonally right through the upper shoulder",
      "turn down the right side without lifting",
      "turn left along the base without lifting, stopping before the left part",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 1]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("draws the exact open Noto Sans Hebrew glyph and preserves the diagonal", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewMemOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_MEM.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_MEM.strokes[1], 2),
    );
  });
});

describe("Hebrew נ — one joined printed hook", () => {
  const steps = ductusSteps(HEBREW_NUN);
  const strip = ductusFilmstrip(HEBREW_NUN, hebrewNunOutline);

  it("keeps the head, right descent, and leftward base joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short top head from left to right",
      "continue down the right side without lifting",
      "turn left along the base without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewNunOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_NUN.strokes[0], 2),
    );
  });
});

describe("Hebrew ס — one closed clockwise printed loop", () => {
  const steps = ductusSteps(HEBREW_SAMEKH);
  const strip = ductusFilmstrip(HEBREW_SAMEKH, hebrewSamekhOutline);

  it("keeps the top, right side, base, and closing left side joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the flat top from left to right",
      "round down the right side without lifting",
      "sweep left along the base without lifting",
      "climb the left side and close the loop without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("draws the exact closed Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewSamekhOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_SAMEKH.strokes[0], 3),
    );
  });
});

describe("Hebrew ע — one joined branch-and-base run", () => {
  const steps = ductusSteps(HEBREW_AYIN);
  const strip = ductusFilmstrip(HEBREW_AYIN, hebrewAyinOutline);

  it("keeps the right descent, base, and left climb joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the right branch and curve left into the base",
      "sweep left along the base without lifting",
      "turn back and climb the left branch without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewAyinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_AYIN.strokes[0], 2),
    );
  });
});

describe("Hebrew פ — an outer body followed by a lifted inner curl", () => {
  const steps = ductusSteps(HEBREW_PE);
  const strip = ductusFilmstrip(HEBREW_PE, hebrewPeOutline);

  it("keeps the top, side, and base joined before the inner curl", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the outer top from left to right",
      "turn down the right side without lifting",
      "return left along the base without lifting",
      "lift, then draw the short inner curl from left to right",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph and preserves the outer body", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewPeOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_PE.strokes[0], 2),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_PE.strokes[1], 1),
    );
  });
});

describe("Hebrew צ — a joined diagonal and base followed by a lifted arm", () => {
  const steps = ductusSteps(HEBREW_TSADI);
  const strip = ductusFilmstrip(HEBREW_TSADI, hebrewTsadiOutline);

  it("keeps the long diagonal joined to the base before the upper-right arm", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the long diagonal from the upper left",
      "turn left along the base without lifting",
      "lift, then curve the upper-right arm down-left into the junction",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph and preserves the first run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewTsadiOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_TSADI.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_TSADI.strokes[1], 1),
    );
  });
});

describe("Hebrew ק — a joined top and right body followed by a lifted stem", () => {
  const steps = ductusSteps(HEBREW_QOF);
  const strip = ductusFilmstrip(HEBREW_QOF, hebrewQofOutline);

  it("keeps the top joined to the right body before the separate descender", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "turn down-left through the right body without lifting",
      "lift, then descend the separate inner-left stem below the line",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph and preserves the first run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewQofOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_QOF.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_QOF.strokes[1], 1),
    );
  });
});

describe("Hebrew ר — one rounded top-and-right run", () => {
  const steps = ductusSteps(HEBREW_RESH);
  const strip = ductusFilmstrip(HEBREW_RESH, hebrewReshOutline);

  it("keeps the top bar and rounded right descent joined", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "round the top-right corner and continue down without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph with no completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewReshOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_RESH.strokes[0], 1),
    );
  });
});

describe("Hebrew ש — an outer bowl followed by a lifted middle branch", () => {
  const steps = ductusSteps(HEBREW_SHIN);
  const strip = ductusFilmstrip(HEBREW_SHIN, hebrewShinOutline);

  it("keeps the outer bowl joined before the separate middle branch", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend the right branch and round left along the base",
      "continue up the left branch without lifting",
      "lift, then descend the middle branch into the base",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph and preserves the outer run", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewShinOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_SHIN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_SHIN.strokes[1], 1),
    );
  });
});

describe("Hebrew ת — a joined top and right side, then a lifted left leg", () => {
  const steps = ductusSteps(HEBREW_TAV);
  const strip = ductusFilmstrip(HEBREW_TAV, hebrewTavOutline);

  it("keeps the top and right side joined before the separate left leg and foot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the top bar from left to right",
      "continue down the right side without lifting",
      "lift, then descend the separate left leg",
      "curve left into the small foot without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });

  it("draws the exact Noto Sans Hebrew glyph and preserves both runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      hebrewTavOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(HEBREW_TAV.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(HEBREW_TAV.strokes[1], 1),
    );
  });
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_ALEF.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_ALEF.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicBaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_BAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_BAA.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicTaaOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(ARABIC_TAA.strokes[0], 1), penPathD(ARABIC_TAA.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_TAA.strokes[2], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true, true]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 4 movements");
  });

  it("draws the exact Noto Naskh outline and preserves all earlier runs", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicThaaOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual(ARABIC_THAA.strokes.slice(0, 3).map((stroke) => penPathD(stroke, 1)));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_THAA.strokes[3], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicJeemOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_JEEM.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_JEEM.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the separate stem in the final bowl frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicHaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_HAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_HAA.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final upper-dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicKhaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_KHAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_KHAA.strokes[1], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicDaalOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_DAAL.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicRaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_RAA.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicSeenOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_SEEN.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("draws Noto Naskh and preserves completed strokes during the upper dot", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicShiinOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(3);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_SHIIN.strokes[0], 1),
      penPathD(ARABIC_SHIIN.strokes[1], 1),
      penPathD(ARABIC_SHIIN.strokes[2], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_SHIIN.strokes[3], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Naskh and preserves the completed body during the bowl", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicSaadOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(ARABIC_SAAD.strokes[0], 1));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_SAAD.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws Noto Naskh and preserves both completed body strokes during the dot", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicDaadOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_DAAD.strokes[0], 1),
      penPathD(ARABIC_DAAD.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_DAAD.strokes[2], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicAynOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_AYN.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("draws Noto Naskh and keeps the completed outer body behind the inner arm", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicKafOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(ARABIC_KAF.strokes[0], 1));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_KAF.strokes[1], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicLamOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_LAM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicMeemOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_MEEM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicNoonOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_NOON.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_NOON.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicHehOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_HEH.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicWawOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_WAW.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2]);
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 4 movements");
  });

  it("draws Noto Naskh and keeps the completed body and first dot behind the second", () => {
    const paths = byTag(strip.frames[3], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicYaaOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(ARABIC_YAA.strokes[0], 1),
      penPathD(ARABIC_YAA.strokes[1], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_YAA.strokes[2], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicHamzaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_HAMZA.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_ALEF.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and keeps the completed dot through the body", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduJimOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(URDU_JIM.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_JIM.strokes[1], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduReOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_RE.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduSinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_SIN.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("uses Noto Naskh and preserves all completed strokes during the upper dot", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduShinOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(3);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(URDU_SHIN.strokes[0], 1),
      penPathD(URDU_SHIN.strokes[1], 1),
      penPathD(URDU_SHIN.strokes[2], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_SHIN.strokes[3], 1),
    );
  });
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and preserves the completed body during the slash", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduKafOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(URDU_KAF.strokes[0], 1));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_KAF.strokes[1], 1),
    );
  });
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduLamOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_LAM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduMimOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_MIM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduNunOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(URDU_NUN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_NUN.strokes[1], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduGhunnaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_GHUNNA.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduHeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_HE.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduYeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_YE.strokes[0], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced fold", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduBariYeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_BARI_YE.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianBehOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(PERSIAN_BEH.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_BEH.strokes[1], 1),
    );
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
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianTehOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(PERSIAN_TEH.strokes[0], 1), penPathD(PERSIAN_TEH.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_TEH.strokes[2], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianSinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_SIN.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianLamOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_LAM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianMimOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_MIM.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianNunOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(PERSIAN_NUN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_NUN.strokes[1], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianWawOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_WAW.strokes[0], 1),
    );
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
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianHehOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_HEH.strokes[0], 1),
    );
  });
});

// ---------------------------------------------------------------------------
// Generic multi-stroke edge cases still use a synthetic ductus so the test can
// vary stroke counts independently of curriculum data. Nothing in this fixture
// is ever shown to a learner, and no letter enters DUCTUS without a citation.
// ---------------------------------------------------------------------------
describe("a letter written in more than one stroke", () => {
  const twoStroke: LetterDuctus = {
    script: "test",
    glyph: "✚",
    strokes: [
      { segments: [{ label: "the upright", path: [{ x: 100, y: 0 }, { x: 100, y: 400 }] }] },
      {
        segments: [
          { label: "the crossbar, left half", path: [{ x: 0, y: 200 }, { x: 100, y: 200 }] },
          { label: "the crossbar, right half", path: [{ x: 100, y: 200 }, { x: 200, y: 200 }] },
        ],
      },
    ],
    source: { citation: "test fixture, not curriculum data", url: "https://example.invalid/fixture" },
  };
  const fakeOutline: GlyphOutline = { path: "M0 0L1 1Z", bounds: { x0: 0, y0: 0, x1: 200, y1: 400 } };

  it("renders every frame without throwing", () => {
    expect(() => ductusFilmstrip(twoStroke, fakeOutline)).not.toThrow();
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("flags the frame where the hand leaves the paper", () => {
    const steps = ductusSteps(twoStroke);
    expect(steps.map((s) => s.startsAfterLift)).toEqual([false, true, false]);
  });

  it("keeps already-finished strokes on screen, in a settled colour", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    const first = byTag(strip.frames[0], "path").filter((p) => p.attrs.class === "ductus__done");
    const later = byTag(strip.frames[2], "path").filter((p) => p.attrs.class === "ductus__done");
    expect(first).toHaveLength(0); // nothing finished yet
    expect(later).toHaveLength(1); // the upright, done, behind the crossbar
    expect(later[0].attrs.d).toBe(penPathD(twoStroke.strokes[0], 1));
  });

  it("counts lifts and movements in plain English", () => {
    const strip = (n: number) =>
      ductusFilmstrip(
        {
          script: "test",
          glyph: "?",
          strokes: Array.from({ length: n }, (_, i) => ({
            segments: [{ label: `part ${i}`, path: [{ x: 0, y: 0 }, { x: 10, y: 0 }] }],
          })),
          source: twoStroke.source,
        },
        fakeOutline,
      ).summary;
    expect(strip(1)).toBe("one unbroken stroke · 1 movement");
    expect(strip(3)).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("honours caller-supplied sizes and colours", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline, { padding: 0, penColor: "#ff0000" });
    const pen = byTag(strip.frames[0], "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.stroke).toBe("#ff0000");
    // With no padding the box hugs the ink exactly sideways, and is taller than
    // the letter by exactly the caption band.
    const box = viewBoxFor(twoStroke, fakeOutline, { padding: 0 });
    expect(box.minX).toBe(0);
    expect(box.minY).toBe(-400);
    expect(box.width).toBe(200);
    expect(box.height).toBeGreaterThan(400);
  });

  it("gives every frame of a letter the SAME box, so the strip reads as one picture", () => {
    const boxes = ductusFilmstrip(twoStroke, fakeOutline).frames.map((f) => f.attrs.viewBox);
    expect(new Set(boxes).size).toBe(1);
  });

  it("draws nothing rather than an empty path when a part has no points", () => {
    const hollow: LetterDuctus = {
      script: "test",
      glyph: "␀",
      strokes: [{ segments: [{ label: "nothing at all", path: [] }] }],
      source: twoStroke.source,
    };
    const frame = ductusFrame(hollow, fakeOutline, ductusSteps(hollow)[0]);
    // Only the glyph outline survives; no zero-length pen path is emitted.
    expect(byTag(frame, "path")).toHaveLength(1);
    expect(byTag(frame, "path")[0].attrs.class).toBe("ductus__glyph");
    // The box still falls back to the glyph's own bounds when the pen has none.
    expect(viewBoxFor(hollow, fakeOutline).width).toBeGreaterThan(200);
  });

  it("survives degenerate input rather than emitting a zero-size picture", () => {
    const empty: LetterDuctus = { script: "test", glyph: "␣", strokes: [], source: twoStroke.source };
    const nowhere: GlyphOutline = { path: "", bounds: { x0: 0, y0: 0, x1: 0, y1: 0 } };
    const strip = ductusFilmstrip(empty, nowhere);
    expect(strip.frames).toHaveLength(0);
    expect(strip.penLifts).toBe(0);
    const box = viewBoxFor(empty, nowhere);
    expect(box.width).toBeGreaterThan(0);
    expect(box.height).toBeGreaterThan(0);
  });
});

describe("serialising to markup", () => {
  it("produces well-formed SVG with the path data intact", () => {
    const svg = svgMarkup(ductusFrame(MA, outline, ductusSteps(MA)[0]));
    expect(svg.startsWith("<svg ")).toBe(true);
    expect(svg.endsWith("</svg>")).toBe(true);
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"');
    expect(svg).toContain(`d="${outline.path}"`);
    expect(svg).toContain('transform="scale(1,-1)"');
    // A DOM parser is the real test of well-formedness.
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelectorAll("path").length).toBeGreaterThan(1);
  });

  it("escapes every XML metacharacter", () => {
    expect(escapeXml(`<&>"'`)).toBe("&lt;&amp;&gt;&quot;&apos;");
    expect(escapeXml("plain")).toBe("plain");
  });

  it("cannot be escaped out of an attribute by a hostile label", () => {
    // A label is authored today, but escaping is a property of the writer, not
    // of the data. Feed it markup and check none survives as markup.
    const nasty: LetterDuctus = {
      script: "test",
      glyph: "x",
      strokes: [
        {
          segments: [
            {
              label: `"><script>alert(1)</script>`,
              path: [{ x: 0, y: 0 }, { x: 10, y: 10 }],
            },
          ],
        },
      ],
      source: { citation: "test fixture", url: "https://example.invalid/fixture" },
    };
    const svg = svgMarkup(
      ductusFrame(nasty, { path: "M0 0Z", bounds: { x0: 0, y0: 0, x1: 10, y1: 10 } }, ductusSteps(nasty)[0]),
    );
    expect(svg).not.toContain("<script>");
    expect(svg).toContain("&lt;script&gt;");
    expect(svg).toContain("&quot;");
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelector("script")).toBeNull();
  });

  it("self-closes childless elements", () => {
    expect(svgMarkup({ tag: "circle", attrs: { r: 3 } })).toBe('<circle r="3"/>');
  });

  // An attribute NAME cannot be escaped — there is nowhere to put the entity —
  // so a name is either legal or dropped. Every name this module emits is a
  // literal, but `SvgNode` is public and the serialiser is meant to be reused.
  it("drops attribute names that are not legal XML names", () => {
    expect(isSafeName("stroke-width")).toBe(true);
    expect(isSafeName("xlink:href")).toBe(true);
    expect(isSafeName(`x" onload="alert(1)`)).toBe(false);
    expect(isSafeName("2bad")).toBe(false);
    const svg = svgMarkup({ tag: "rect", attrs: { [`x" onload="alert(1)`]: "1", width: 4 } });
    expect(svg).toBe('<rect width="4"/>');
  });

  it("refuses event-handler attributes outright, prefix and all", () => {
    // `onload` is a perfectly legal XML name AND a script. Reject the prefix
    // rather than chase a list of handler names that keeps growing.
    expect(isSafeName("onload")).toBe(false);
    expect(isSafeName("OnClick")).toBe(false);
    expect(svgMarkup({ tag: "svg", attrs: { onload: "alert(1)", onclick: "x" } })).toBe("<svg/>");
  });

  it("neutralises a hostile tag name rather than emitting it", () => {
    expect(svgMarkup({ tag: "svg><script", attrs: {} })).toBe("<g/>");
  });
});
