// figure-targets.ts — which lessons get a stroke-order filmstrip (HL-C443).
//
// ---------------------------------------------------------------------------
// The problem this closes
// ---------------------------------------------------------------------------
//
// The filmstrip renderer shipped (HL-C300) with three proof targets declared by
// hand in `core/figure-generation.json`, and no lesson referenced any of them.
// So no printed book showed a single filmstrip, while about 380 writing lessons
// already had a CITED stroke order in `script-ductus`. The drawing was never the
// gap. Getting a figure into a book cost three manual steps per letter, and
// nobody took them.
//
// This module removes the manual steps for the case that is always the same: a
// `type: writing` lesson whose headword is ONE letter (or, since the sequence
// strips below, a list of letters or a word whose letters stand apart, or,
// since the shared-headline strips, a Devanagari word), with a `## Writing:`
// block (or, where a track presents its letters that way, a `## Script`
// block), on a track that has been switched on below. Such a lesson is a
// candidate. The candidate becomes a figure only if `script-ductus` holds a
// cited ductus for that letter, and the generated filmstrip ledger is the record
// of that. HL11 §5.2 still holds: no citation, no pen path, no figure. Nothing
// here can draw a letter the ductus data does not source.
//
// ---------------------------------------------------------------------------
// Why deriving from the headword is safe here
// ---------------------------------------------------------------------------
//
// `ScriptFilmstripTarget` names its glyph explicitly. The worry was that a
// figure which followed a headword edit would "redraw itself the next time
// somebody fixed a typo". For a derived target that redraw is LOUD rather than
// silent:
//
//   * the filmstrip ledger is byte-checked by `check:filmstrip-ledger`;
//   * every SVG is hashed in one track owner under `core/generated-figure-hashes.d/`;
//   * every book chapter is hashed as well.
//
// So a headword edit fails three checks until someone regenerates, and the
// regenerated figure is the letter the lesson now teaches, which is the right
// one. An explicit target in the config still wins over a derived one for the
// same lesson, for the lesson that deliberately prints a letter it is not
// named after.
//
// ---------------------------------------------------------------------------
// One track at a time
// ---------------------------------------------------------------------------
//
// `DERIVED_FILMSTRIP_SCRIPTS` is an allowlist, not a heuristic. Each track that
// joins adds pages of figures to its book, and only a real XeLaTeX build (CI's
// books gate) can say the pages still lay out. So tracks join one PR at a time.

import { basename } from "node:path";
import type { FigureTarget, ScriptFilmstripTarget } from "./figure.js";
import type { ParsedLesson } from "./parse.js";

/**
 * Track -> the `script-ductus` script id its writing lessons are drawn in.
 *
 * Tamil goes first because it is the only script whose ductus is sharded one
 * cited file per glyph, and its single-letter lessons (`TA-S01` onward) are the
 * clearest one-letter-at-a-time ramp in the corpus.
 */
export const DERIVED_FILMSTRIP_SCRIPTS: Readonly<Record<string, string>> = {
  tamil: "tamil",
  // HL-C443 second rollout: the four Devanagari tracks share one cited ductus.
  hindi: "devanagari",
  marathi: "devanagari",
  sanskrit: "devanagari",
  marwadi: "devanagari",
  // HL-C443 third rollout: every other track with any cited ductus. A track
  // whose letters are mostly uncited (kannada, malayalam, telugu: vowels and
  // chillus only) still gets exactly the letters that ARE cited, no more.
  gujarati: "gujarati",
  arabic: "arabic",
  persian: "perso-arabic",
  urdu: "urdu-nastaliq",
  russian: "cyrillic",
  chinese: "chinese",
  japanese: "japanese",
  kannada: "kannada",
  malayalam: "malayalam",
  telugu: "telugu",
  // HL-C443 fourth rollout: Bengali, whose first cited letters come from
  // native writers' pen traces (HP Labs India's LipiTk Bangla recognizer). As
  // with kannada, only the letters that carry a cited ductus are drawn.
  bengali: "bengali",
  // HL-C443 fifth rollout: Punjabi, whose first cited letters follow the
  // Apache-2.0 Alphabet Tracing lesson of GNPS's Gurmukhi Sikho app. Only the
  // letters with a cited ductus are drawn, and a Gurmukhi WORD is still never
  // composed (one headline runs across it; see SEPARATE_LETTER_SCRIPTS).
  punjabi: "gurmukhi",
  // HL-C443 sixth rollout: the first Latin-script tracks. Their print letters
  // follow the Grundschrift-App's ordered paths (a school model, cited as
  // facts only), with native Spanish writers' counts from UJIpenchars2, and ñ's
  // tilde, ¿ and ¡ follow those writers. Only Spanish and German join: their
  // lessons are the only Latin ones whose every letter is cited. Noto Sans
  // prints a two-storey a, which no source draws, so French, Italian,
  // Portuguese and Latin, whose writing headwords all hold an a or an
  // uncited mark, stay off until a source covers it.
  spanish: "latin",
  german: "latin",
};

const GRAPHEMES = new Intl.Segmenter("und", { granularity: "grapheme" });

/** Does this block hold the lesson's writing instructions? */
function isWritingBlock(title: string): boolean {
  return /^Writing\b/.test(title.trim());
}

/** Does this block present the letter itself (the tracks that teach a letter under a `## Script` heading)? */
function isScriptBlock(title: string): boolean {
  return /^Script\b/.test(title.trim());
}

/**
 * Where a lesson's filmstrip belongs: its first Writing block, or, for the
 * tracks that present a letter under `## Script` instead (chinese, japanese,
 * urdu, persian and most russian letter lessons), its first Script block.
 * `-1` when the lesson has neither.
 */
export function filmstripBlockIndex(lesson: ParsedLesson): number {
  const letter = letterBlockIndex(lesson);
  if (letter !== -1) return letter;
  // A DECLARED target may sit on a lesson that teaches its letter inside a word
  // lesson (FA-C03-chist introduces چ in its first explanation block). There the
  // figure goes on the first block that introduces a script atom. Derived
  // candidates never reach this branch: `writingLetterOf` requires a letter
  // block.
  return lesson.blocks.findIndex((block) =>
    (block.knowledge?.introduces ?? []).some((atom) => /-SCRIPT-/.test(atom)),
  );
}

/** The first Writing block, else the first Script block; `-1` when neither. */
export function letterBlockIndex(lesson: ParsedLesson): number {
  const writing = lesson.blocks.findIndex((block) => isWritingBlock(block.title));
  if (writing !== -1) return writing;
  return lesson.blocks.findIndex((block) => isScriptBlock(block.title));
}

/**
 * The one letter a writing lesson teaches, or `undefined` when the lesson is
 * not a single-letter writing lesson with a Writing block. A headword like
 * "வ, க" teaches more than one letter; `writingSequenceOf` below decides
 * whether it becomes a sequence strip, as does a word like "மேசை" whose vowel
 * signs have a cited written order, and so does "வணக்கம்", whose puḷḷi is
 * written after its consonant. "பேசு" does not: ு has neither a cited ductus
 * nor a written-order row (`WRITTEN_SIGN_SIDES`).
 */
export function writingLetterOf(lesson: ParsedLesson): string | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  if (headword === "" || [...GRAPHEMES.segment(headword)].length !== 1) return undefined;
  if (letterBlockIndex(lesson) === -1) return undefined;
  return headword;
}

// ---------------------------------------------------------------------------
// Several letters in one headword (HL-C443, sequence strips)
// ---------------------------------------------------------------------------
//
// About thirty writing lessons teach more than one letter at once and printed
// no filmstrip: a LIST of letters ("வ, க", "ક — ણ — શ", "в, р", "ع ي") or a
// short WORD whose every letter is cited ("はい", "こんにちは"). Each of those
// letters already has a cited ductus, so the strip can be built from parts
// that exist: every letter's own frames, in the order they are written (see
// `renderScriptSequenceFilmstripFigure`). Nothing new is drawn.
//
// The danger is drawing something WRONG out of right parts. Putting cited
// letters side by side asserts that a native writer writes them that way, and
// for a word that is false in several scripts:
//
//   script            what a word does that separate letters do not
//   ----------------  ------------------------------------------------------
//   devanagari        one continuous headline (shirorekha) across the word;
//                     every cited letter ends "lift, then draw the
//                     shirorekha", so a composed word would show N headlines
//                     (a Devanagari word has its own strip instead: its
//                     letters' bodies, then ONE headline; `headlineWordOf`)
//   arabic family     letters join and change shape (initial / medial /
//                     final); the ductus is isolated forms only
//   cyrillic          the cited school hand is connected cursive, whose own
//                     variation notes say words add entry and exit joins
//
// A LIST is different, in every script: each listed letter is written by
// itself, in its isolated form, with its own headline — which is exactly what
// its cited ductus draws. So lists qualify everywhere and words qualify only
// in `SEPARATE_LETTER_SCRIPTS`.

/**
 * Scripts whose letters stand apart inside a word: no shared headline, no
 * joining, no change of shape next to a neighbour, and a pen lift between one
 * letter and the next. A word in one of these, made only of cited base
 * letters, is honestly drawn as its letters one after another.
 *
 * Latin is here for its PRINT letters, the hand the cited school model
 * (the Grundschrift-App) teaches first and the one the bundled outline
 * prints: each letter stands apart. The joined hand that Grundschrift goes on
 * to teach is not drawn. A Latin word with punctuation inside it (¿cómo?) is
 * refused, because ¿ and ? are not base letters; a precomposed ñ is one base
 * letter with a ductus of its own, and n typed with a combining tilde is
 * refused, because Latin has no written-order table.
 *
 * Devanagari, Bengali and Gurmukhi are absent for the reason above (a shared
 * headline). All three have cited letters, so a LIST of them is drawn. A
 * Devanagari WORD is drawn another way (`headlineWordOf`); a Bengali or
 * Gurmukhi word never is: Gurmukhi's cited source draws the headline FIRST,
 * and Bengali's letters do not treat it one way.
 */
export const SEPARATE_LETTER_SCRIPTS: ReadonlySet<string> = new Set([
  "chinese",
  "japanese",
  "tamil",
  "gujarati",
  "kannada",
  "telugu",
  "malayalam",
  "latin",
]);

/** What separates the items of a list headword: space, commas, dashes, dots. */
const LIST_SEPARATORS = /[\s,\u060C\u3001\u2014\u00B7]+/u;

/**
 * One base letter: a single code point of category L that is not a modifier
 * (Lm, like ー). A word may only be drawn letter by letter when every grapheme
 * is one of these, unless its script has a written-order table below.
 */
const BASE_LETTER = /^(?!\p{Lm})\p{L}$/u;

/** A grapheme made of combining signs alone: a vowel sign taught by itself. */
const SIGNS_ONLY = /^\p{M}+$/u;

// ---------------------------------------------------------------------------
// Vowel signs: drawn in the order they are WRITTEN, not the order they are typed
// ---------------------------------------------------------------------------
//
// Unicode stores a vowel sign AFTER its consonant, but a hand does not always
// write it there. Tamil writes three signs to the LEFT of the consonant, and
// writes them first:
//
//     typed (code points)      written (by hand)        looks like
//     ----------------------   ----------------------   ----------
//     க + ெ                    ெ, then க               கெ
//     க + ே                    ே, then க               கே
//     க + ை                    ை, then க               கை
//     க + ொ  (= ெ + ா)         ெ, then க, then ா       கொ
//     க + ோ  (= ே + ா)         ே, then க, then ா       கோ
//     க + ா / ி / ீ            க, then the sign         கா கி கீ
//     க + ்  (puḷḷi)           க, then the dot          க்
//
// A strip drawn in code-point order would put the left-hand sign's strokes
// AFTER the consonant's. So a script may compose words with signs only through
// a table that says, for each sign, which side of its consonant it is written
// on, and each side must be cited:
//
//   * The Tamil mark records (data/scripts/tamil.d/marks) cite it as their
//     `compositionSource`: Radhakrishnan's Tamil Script Learners Manual,
//     Modules 6 and 7, for ெ and ே ("written before the primary consonant");
//     HP Labs India's Lipi Indic Character Recognizers 4.0 User Manual for
//     ா, ி, ீ and ை (signs written as distinct characters to the left or right
//     of the consonant, and units written from left to right). The puḷḷi
//     cites Varai's recorded drawings of the 18 consonants with puḷḷi: the
//     body is drawn first, then the dot above it, as a second stroke (one
//     writer, so confidence is medium). A test holds this table to those
//     records.
//   * ொ and ோ are not in the table: Unicode decomposes them (NFD) into ெ/ே
//     plus ா, and each half is placed by its own row. ௌ decomposes into ெ
//     plus ௗ, which has no row, so it stays refused.
//
// Gujarati has a table too, and it is simpler: every sign is written AFTER
// its consonant, even િ, which sits to the consonant's LEFT. So in Gujarati the
// written order is the typed order, and the table's job is to say which signs
// have a cited place at all:
//
//     typed (code points)      written (by hand)        looks like
//     ----------------------   ----------------------   ----------
//     ક + િ                    ક, then િ               કિ   (sign sits left)
//     ક + ા / ી / ુ / ૂ        ક, then the sign         કા કી કુ કૂ
//     ક + ે / ૈ / ો / ૌ        ક, then the sign         કે કૈ કો કૌ
//     ક + ં / ઃ                ક, then the mark         કં કઃ
//
//   * The Gujarati mark records (data/scripts/gujarati.json) cite it as their
//     `compositionSource`: KanoAI's hand-made Gujarati barakhadi templates
//     draw the consonant's group before the sign's in every consonant row
//     whose consonant keeps its bare outline (33 of 34 for િ; ઢિ is the
//     exception). A test holds this table to those records.
//   * ો and ૌ have no Unicode decomposition: each is one sign, drawn as its
//     own two or three runs (bar first, then the flags).
//   * The virama ્ and the vocalic-r sign ૃ have no row: no Gujarati source
//     gives their pen path or place, so every word with one stays refused.
//
// Malayalam has a table with one row. Its anusvara ം is written AFTER its
// base, to the right of it:
//
//     typed (code points)      written (by hand)        looks like
//     ----------------------   ----------------------   ----------
//     അ + ം                    അ, then the ring         അം
//     ക + ം                    ക, then the ring         കം
//
//   * The anusvara's mark record (data/scripts/malayalam.json) cites it as
//     its `compositionSource`: in Rodney F. Moag's Malayalam: A University
//     Course and Reference Grammar, Table II numbers the ring of അം as
//     movement 9, after the eight movements of അ, and Table III draws ം to
//     the right of a dash standing for the consonant. A test holds the row
//     to that record. Dumping every GSUB lookup of Noto Sans Malayalam that
//     mentions the anusvara glyph finds only Vedic-sign reorderings, so no
//     consonant + ം pair is fused.
//   * Moag draws each vowel sign beside that dash too, but never numbers the
//     consonant against the sign, so no Malayalam vowel sign has a row: a
//     sign is drawn only by itself, in a lesson that teaches it alone, and a
//     word with a vowel sign stays refused. So does every word with the
//     candrakkala ്, whose Moag table gives no movements at all.
//
// Everything without a row is refused: the Tamil signs ு and ூ (no cited
// ductus, and they fuse with their consonant into shapes of their own),
// Gujarati ્ and ૃ, every Malayalam sign but ം, and every sign in every
// other script (Devanagari ि, the kana voicing mark ゙), because no other
// script has a table yet.
//
// Two signs on the same side of one consonant (Gujarati ાં, a vowel sign and
// then the anusvara) are refused as well: each sign's own place is cited, but
// no source orders the two against each other.
//
// Some consonant-sign pairs fuse into one ligature even though both halves
// have rows. Drawing the consonant and the sign apart would draw a word
// nobody writes, so those pairs are refused too:
//
// Each group of pairs carries its citation in `FUSED_SIGN_PAIR_SOURCES`:
//
//   * Tamil: Unicode 17.0 §12.6.3 (Ligatures with Vowel i, Figure 12-21)
//     shows ட with ி and ீ, and ல with ீ, joined into new shapes.
//   * Gujarati: the bundled Noto Sans Gujarati (read from its GSUB table)
//       - joins ર with ુ and ૂ, and ણ with ુ, into glyphs of their own
//         ('blws');
//       - gives 22 consonants a "stem" form of their own before ુ and ૂ
//         ('blws'): ખ ગ ઘ ચ ઞ ણ ત થ ધ ન પ ફ બ ભ મ ય લ ળ વ શ ષ સ. The printed
//         consonant then grows a stem that neither its own ductus nor the
//         sign's draws;
//       - joins જ and ૹ with ા and ી ('psts'), and, after જ and ૹ, splits ો
//         and ૌ into ા plus ે or ૈ and joins that ા to the consonant too.
//     Only the consonant and the sign are refused together; each still
//     composes with every other partner (રા, જે, નો).

/**
 * Whether a sign is WRITTEN before or after its consonant, per script. In
 * Tamil that is also the side the sign sits on; Gujarati િ sits on the left
 * but is written after, so the value is the order, not the position.
 */
export const WRITTEN_SIGN_SIDES: Readonly<Record<string, Readonly<Record<string, "before" | "after">>>> = {
  tamil: {
    "\u0BC6": "before", // ெ  e
    "\u0BC7": "before", // ே  ē
    "\u0BC8": "before", // ை  ai
    "\u0BBE": "after", //  ா  ā
    "\u0BBF": "after", //  ி  i
    "\u0BC0": "after", //  ீ  ī
    "\u0BCD": "after", //  ்  puḷḷi (the dot above, made after the body)
  },
  gujarati: {
    "\u0ABE": "after", //  ા  ā
    "\u0ABF": "after", //  િ  i (sits LEFT of the consonant, written after it)
    "\u0AC0": "after", //  ી  ī
    "\u0AC1": "after", //  ુ  u
    "\u0AC2": "after", //  ૂ  ū
    "\u0AC7": "after", //  ે  e
    "\u0AC8": "after", //  ૈ  ai
    "\u0ACB": "after", //  ો  o
    "\u0ACC": "after", //  ૌ  au
    "\u0A82": "after", //  ં  anusvara
    "\u0A83": "after", //  ઃ  visarga
  },
  malayalam: {
    "\u0D02": "after", //  ം  anusvara (a ring to the right, after its base)
  },
};

/**
 * One cited reason that some consonant + sign pairs fuse.
 *
 * The citation names the source closely enough to look up (section and
 * figure, or font version and GSUB lookup), and it names every base letter
 * and every sign of its `pairs`: a test holds the two to each other, so a
 * pair cannot be added to or dropped from this table without its source
 * being edited in the same change.
 */
export interface FusedSignPairSource {
  /** Where the fusion is shown, specific enough to check. */
  readonly citation: string;
  /** An HTTPS URL for the source. */
  readonly url: string;
  /** The NFD consonant + sign pairs this source shows fused or reshaped. */
  readonly pairs: readonly string[];
}

/** The bundled Gujarati font the GSUB citations below were read from. */
const NOTO_SANS_GUJARATI =
  "Noto Sans Gujarati Version 2.106, bundled as learning/human-languages/_fonts/NotoSansGujarati-Static.ttf";
const NOTO_SANS_GUJARATI_URL = "https://github.com/notofonts/gujarati";

/**
 * Every fused pair, grouped under the source that shows it. A pair may stand
 * under two sources when both show it (Gujarati ણુ is a stem form AND a
 * ligature of its own); `FUSED_SIGN_PAIRS` is built from this table and
 * nothing else, so there is no fused pair without a citation.
 */
export const FUSED_SIGN_PAIR_SOURCES: Readonly<Record<string, readonly FusedSignPairSource[]>> = {
  tamil: [
    {
      citation:
        "The Unicode Standard, Version 17.0, §12.6.3 Tamil Ligatures, Ligatures with Vowel i and " +
        "Figure 12-21: ட with ி and ீ, and ல with ீ, join into ligatures of their own (2025)",
      url: "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
      pairs: ["\u0B9F\u0BBF", "\u0B9F\u0BC0", "\u0BB2\u0BC0"], // டி டீ லீ
    },
  ],
  gujarati: [
    {
      // Stem forms before u and uu.
      citation:
        `${NOTO_SANS_GUJARATI}, GSUB 'blws' contextual lookup 90: before ુ or ૂ the consonants ` +
        "ખ ગ ઘ ચ ઞ ણ ત થ ધ ન પ ફ બ ભ મ ય લ ળ વ શ ષ સ take a stem form that neither the " +
        "letter's ductus nor the sign's draws",
      url: NOTO_SANS_GUJARATI_URL,
      pairs: [..."ખગઘચઞણતથધનપફબભમયલળવશષસ"].flatMap((consonant) => [
        `${consonant}\u0AC1`, // C + ુ
        `${consonant}\u0AC2`, // C + ૂ
      ]),
    },
    {
      citation:
        `${NOTO_SANS_GUJARATI}, GSUB 'blws' ligature lookup 89: ર with ુ, ર with ૂ, and ણ with ુ ` +
        "each become one glyph of their own",
      url: NOTO_SANS_GUJARATI_URL,
      pairs: ["\u0AB0\u0AC1", "\u0AB0\u0AC2", "\u0AA3\u0AC1"], // રુ રૂ ણુ
    },
    {
      // જ and ૹ join the ā bar, alone (ા, ી) or inside ો and ૌ.
      citation:
        `${NOTO_SANS_GUJARATI}, GSUB 'psts' lookups 94 to 96 and 106: after જ or ૹ, ો and ૌ ` +
        "are split into ા plus ે or ૈ, and ા and ી then join the consonant into one glyph",
      url: NOTO_SANS_GUJARATI_URL,
      pairs: ["\u0A9C", "\u0AF9"].flatMap((consonant) =>
        ["\u0ABE", "\u0AC0", "\u0ACB", "\u0ACC"].map((sign) => `${consonant}${sign}`),
      ),
    },
  ],
};

/** Consonant + sign pairs that fuse into a ligature, or that the font reshapes, per script (NFD). */
export const FUSED_SIGN_PAIRS: Readonly<Record<string, ReadonlySet<string>>> = Object.fromEntries(
  Object.entries(FUSED_SIGN_PAIR_SOURCES).map(([script, sources]) => [
    script,
    new Set(sources.flatMap((source) => source.pairs)),
  ]),
);

// A puḷḷi is a grapheme boundary in Tamil: க்ஷ is two graphemes, க் and ஷ.
// So `writtenPiecesOf`, which looks at one grapheme, cannot see that the font
// joins some letters ACROSS that boundary. Noto Sans Tamil prints every
// consonant + puḷḷi as the unchanged consonant with the unchanged dot above
// it (each such glyph is a composite of exactly those two outlines), so a
// plain க் is honest to draw as க, then ். But two sequences become one new
// shape, and a strip of their parts would draw letters the page does not show:
//
//     typed              printed as
//     ----------------   -----------------------------------
//     க ் ஷ              க்ஷ, one glyph ('akhn')
//     ஸ ் ர ீ / ஶ ் ர ீ    ஸ்ரீ, one glyph ('abvs')
//
// A word containing one is refused, whatever its letters' ductus.

/** One cited reason that a run of letters across graphemes is printed as one shape. */
export interface FusedLetterSequenceSource {
  /** Where the fusion is shown, specific enough to check. */
  readonly citation: string;
  /** An HTTPS URL for the source. */
  readonly url: string;
  /** The NFD letter runs this source shows joined into one glyph. */
  readonly sequences: readonly string[];
}

/** Letter runs, spanning graphemes, that the bundled font prints as one glyph, per script. */
export const FUSED_LETTER_SEQUENCE_SOURCES: Readonly<Record<string, readonly FusedLetterSequenceSource[]>> = {
  tamil: [
    {
      citation:
        "Noto Sans Tamil Version 2.004, bundled as learning/human-languages/_fonts/NotoSansTamil-Static.ttf: " +
        "GSUB 'akhn' ligature lookup 2 joins க, ் and ஷ into one glyph; GSUB 'abvs' ligature lookup 1 joins " +
        "ஸ or ஶ with ், ர and ீ into one glyph",
      url: "https://github.com/notofonts/tamil",
      sequences: ["\u0B95\u0BCD\u0BB7", "\u0BB8\u0BCD\u0BB0\u0BC0", "\u0BB6\u0BCD\u0BB0\u0BC0"], // க்ஷ ஸ்ரீ ஶ்ரீ
    },
  ],
};

/** Whether `word` contains a letter run its script's font prints as one glyph. */
export function hasFusedLetterSequence(word: string, script: string): boolean {
  const nfd = word.normalize("NFD");
  return (FUSED_LETTER_SEQUENCE_SOURCES[script] ?? []).some((source) =>
    source.sequences.some((sequence) => nfd.includes(sequence)),
  );
}

/**
 * The pieces one grapheme is written as, in written order, or `undefined`
 * when this module will not draw it.
 *
 *   * a base letter is one piece, itself, in every script;
 *   * in a script with a `WRITTEN_SIGN_SIDES` table, a base letter with signs
 *     is its before-signs, the letter, then its after-signs ("கை" -> ை, க);
 *     a grapheme of signs alone (a sign lesson's headword, "ோ") is its
 *     before-signs then its after-signs (ே, ா);
 *   * anything else (a sign with no row, two signs on the same side of one
 *     consonant, a fused pair, a digit, a mark in a script with no table) is
 *     `undefined`.
 */
export function writtenPiecesOf(grapheme: string, script: string): string[] | undefined {
  if (BASE_LETTER.test(grapheme)) return [grapheme];
  const sides = WRITTEN_SIGN_SIDES[script];
  if (sides === undefined) return undefined;
  const [first, ...rest] = [...grapheme.normalize("NFD")];
  if (first === undefined) return undefined;
  const base = BASE_LETTER.test(first) ? first : undefined;
  const signs = base === undefined ? [first, ...rest] : rest;
  if (base === undefined && !SIGNS_ONLY.test(grapheme)) return undefined;
  if (signs.length === 0 || signs.some((sign) => !Object.prototype.hasOwnProperty.call(sides, sign))) {
    return undefined;
  }
  if (base !== undefined && signs.some((sign) => FUSED_SIGN_PAIRS[script]?.has(`${base}${sign}`))) {
    return undefined;
  }
  const before = signs.filter((sign) => sides[sign] === "before");
  const after = signs.filter((sign) => sides[sign] === "after");
  // No source orders two signs written on the same side of one consonant.
  if (before.length > 1 || after.length > 1) return undefined;
  return [...before, ...(base === undefined ? [] : [base]), ...after];
}

/**
 * The letters a writing lesson's headword spells out, in writing order, or
 * `undefined` when it is not a sequence this module will draw.
 *
 * A sequence is
 *
 *   * a LIST of two or more items, each exactly one grapheme, in any script; or
 *   * one or more WORDS in a `SEPARATE_LETTER_SCRIPTS` script, two or more
 *     pieces in all, where every grapheme is a single base letter or, in a
 *     script with a written-order table, a base letter with cited signs; or
 *   * in a script with a written-order table, ONE grapheme that is written in
 *     two or more pieces: a two-part sign taught by itself ("ோ" -> ே, ா).
 *
 * In a script with a table, each piece of a sign-bearing grapheme is placed in
 * written order (`writtenPiecesOf`). A one-piece headword is not a sequence;
 * `writingLetterOf` owns it. Like a single letter, a sequence needs a Writing
 * or Script block to land in. This function only reads the headword; whether
 * each piece has a CITED ductus is `withDerivedFilmstrips`' question, answered
 * by the ledger.
 */
export function writingSequenceOf(lesson: ParsedLesson, script: string): string[] | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  if (letterBlockIndex(lesson) === -1) return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  const items = headword.split(LIST_SEPARATORS).filter((item) => item !== "");
  const graphemes = items.map((item) => [...GRAPHEMES.segment(item)].map((part) => part.segment));
  const letters = graphemes.flat();
  if (letters.length === 0) return undefined;
  const hasTable = WRITTEN_SIGN_SIDES[script] !== undefined;
  if (letters.length === 1) {
    if (!hasTable) return undefined;
    const pieces = writtenPiecesOf(letters[0]!, script);
    return pieces !== undefined && pieces.length >= 2 ? pieces : undefined;
  }
  if (items.length >= 2 && graphemes.every((item) => item.length === 1)) {
    // A list keeps a grapheme no table can place (a digit, an uncited sign)
    // as it is: the ledger then decides, exactly as before signs were placed.
    return hasTable ? letters.flatMap((letter) => writtenPiecesOf(letter, script) ?? [letter]) : letters;
  }
  if (!SEPARATE_LETTER_SCRIPTS.has(script)) return undefined;
  if (items.some((item) => hasFusedLetterSequence(item, script))) return undefined;
  const pieces = letters.map((letter) => writtenPiecesOf(letter, script));
  if (pieces.some((piece) => piece === undefined)) return undefined;
  return pieces.flat() as string[];
}

// ---------------------------------------------------------------------------
// Devanagari words: the letters' bodies, then ONE shared headline
// ---------------------------------------------------------------------------
//
// A Devanagari word hangs from one headline, so it is not its letters' strips
// side by side (see the table above). It gets a strip of its own instead:
//
//     मम   ->   म's body,  म's body (moved right by the first म's advance),
//               then ONE headline, left to right, over the whole word
//
// The headline comes last because most native writers draw it last: in HP Labs
// India's LipiTk 4.0 Devanagari recognizer, 82% of the 2,706 stored consonant
// prototypes end with the headline and about 5% begin with it. Some writers do
// begin with it, and the traces are single letters, not words, so the strip
// prints "attested, not standardised" and its source note says both things.
//
// The composing is done in `script-ductus` (`composeHeadlineWord`), which has
// the cited letters and the font: it takes each letter's last stroke off (its
// "lift, then draw the shirorekha rightward"), places the bodies at the font's
// advances, draws the one headline, and refuses any word whose result does not
// fit the printed word's ink at the default tolerances. That last check is
// what refuses, say, मथ: थ's own headline does not reach its left edge, so the
// printed line is broken there and one straight headline would cross paper.
//
// THIS module decides which headwords are worth trying, from the text alone:
//
//   * one word (no space, comma, dash or dot) of two or more letters;
//   * every letter a single base letter of the script, one code point that
//     NFD leaves alone. So EVERY sign is refused: no Devanagari sign has a
//     cited place against its consonant or the shared headline (the signs'
//     traces were written alone); ि's place is unresolved; ा has no cited
//     ductus; the virama makes the conjuncts and half forms the font fuses;
//     and a precomposed nukta letter (क़, U+0958) decomposes into क + ़;
//   * no letter the bundled font SPLITS while shaping, listed with its
//     citation in `HEADLINE_WORD_SPLIT_LETTER_SOURCES` (ई and ऐ): the strip
//     lays out each letter's own outline at its advance, which is what the
//     printed word shows only when the font prints that outline unchanged.
//
// Shaping every two- and three-letter string of the other cited letters with
// HarfBuzz gave exactly their `cmap` glyphs at their `hmtx` advances, with no
// offsets, so for them the composed outline IS the printed word.

/**
 * Scripts whose words are drawn as their letters' bodies and one shared
 * headline, with the Unicode script every letter must belong to.
 * `HEADLINE_WORD_SCRIPTS` in script-ductus names the same scripts; a test
 * there holds the two together.
 */
export const HEADLINE_WORD_SCRIPTS: Readonly<Record<string, RegExp>> = {
  devanagari: /^\p{Script=Devanagari}$/u,
};

/** One cited reason that the bundled font prints some letters as other glyphs. */
export interface SplitLetterSource {
  /** Where the substitution is, specific enough to check (font version, table, lookup). */
  readonly citation: string;
  /** An HTTPS URL for the source. */
  readonly url: string;
  /** The letters it splits. */
  readonly letters: readonly string[];
}

/**
 * Letters the bundled font replaces while shaping, so a word containing one is
 * not its letters' outlines at their advances. A test checks that the bundled
 * Devanagari font is still the version cited.
 */
export const HEADLINE_WORD_SPLIT_LETTER_SOURCES: Readonly<Record<string, readonly SplitLetterSource[]>> = {
  devanagari: [
    {
      citation:
        "Noto Sans Devanagari Version 2.006, bundled as learning/human-languages/_fonts/" +
        "NotoSansDevanagari-Static.ttf: GSUB 'abvs' multiple-substitution lookup 179 replaces ई " +
        "with इ plus a reph-shaped mark (glyph uni0930094D) and ऐ with ए plus े, each placed by GPOS",
      url: "https://github.com/notofonts/devanagari",
      letters: ["\u0908", "\u0910"], // ई ऐ
    },
  ],
};

/**
 * The word a writing lesson's headword is, when it is a word this module asks
 * `script-ductus` to compose with one shared headline; `undefined` otherwise.
 * Like every candidate, it needs a Writing or Script block to land in, and
 * whether it is DRAWN is the ledger's answer (`withDerivedFilmstrips`).
 */
export function headlineWordOf(lesson: ParsedLesson, script: string): string | undefined {
  const letterScript = HEADLINE_WORD_SCRIPTS[script];
  if (letterScript === undefined) return undefined;
  if (lesson.realization.type !== "writing") return undefined;
  if (letterBlockIndex(lesson) === -1) return undefined;
  const word = (lesson.realization.headword ?? "").trim();
  if (word === "" || LIST_SEPARATORS.test(word)) return undefined;
  const letters = [...word];
  if (letters.length < 2) return undefined;
  const split = new Set((HEADLINE_WORD_SPLIT_LETTER_SOURCES[script] ?? []).flatMap((source) => source.letters));
  const fits = letters.every(
    (letter) =>
      BASE_LETTER.test(letter) &&
      letterScript.test(letter) &&
      letter.normalize("NFD") === letter &&
      !split.has(letter),
  );
  return fits ? word : undefined;
}

/** Every lesson on a switched-on track that COULD carry a filmstrip. */
export function filmstripCandidates(
  lessons: readonly ParsedLesson[],
  scripts: Readonly<Record<string, string>> = DERIVED_FILMSTRIP_SCRIPTS,
): ScriptFilmstripTarget[] {
  const candidates: ScriptFilmstripTarget[] = [];
  for (const lesson of lessons) {
    const script = scripts[lesson.language];
    if (script === undefined) continue;
    const lessonId = lesson.realization.lessonId;
    const output = `${lesson.language}/book/figures/${lessonId}-filmstrip.svg`;
    // A sequence first: a one-grapheme headword that is written in two pieces
    // (the two-part sign ோ) is a sequence, not a letter.
    const letters = writingSequenceOf(lesson, script);
    if (letters === undefined) {
      // A Devanagari word: one entry, composed from its letters and one
      // shared headline by script-ductus.
      const word = headlineWordOf(lesson, script);
      if (word !== undefined) {
        candidates.push({
          kind: "script-filmstrip",
          lessonId,
          script,
          glyph: word,
          composition: "shared-headline",
          output,
        });
        continue;
      }
      const glyph = writingLetterOf(lesson);
      if (glyph !== undefined) candidates.push({ kind: "script-filmstrip", lessonId, script, glyph, output });
      continue;
    }
    candidates.push({
      kind: "script-filmstrip",
      lessonId,
      script,
      glyph: (lesson.realization.headword ?? "").trim(),
      letters,
      output,
    });
  }
  return candidates.sort((left, right) => left.lessonId.localeCompare(right.lessonId));
}

/** The letters a filmstrip target draws: its `letters`, else its one `glyph`. */
export function filmstripLetters(target: ScriptFilmstripTarget): readonly string[] {
  return target.letters ?? [target.glyph];
}

/**
 * The explicit targets plus every candidate whose letter (or, for a sequence,
 * every letter) has a cited ductus.
 *
 * `hasDuctus` is the caller's source of truth for "cited": `script-ductus`
 * asks its own stroke registry, and the figure and book generators ask the
 * generated filmstrip ledger, which `script-ductus` writes from that registry.
 */
export function withDerivedFilmstrips(
  explicit: readonly FigureTarget[],
  candidates: readonly ScriptFilmstripTarget[],
  hasDuctus: (script: string, glyph: string) => boolean,
): FigureTarget[] {
  const declared = new Set(
    explicit
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );
  // A sequence is drawn only when EVERY letter in it is cited: one uncited
  // letter would leave a hole in the word, and a strip that skipped it would
  // teach the word misspelled.
  return [
    ...explicit,
    ...candidates.filter(
      (candidate) =>
        !declared.has(candidate.lessonId) &&
        filmstripLetters(candidate).every((glyph) => hasDuctus(candidate.script, glyph)),
    ),
  ];
}

/**
 * The Markdown image a book prints for a filmstrip target.
 *
 * A Devanagari word reads "How मम is written, letter by letter, then one
 * headline": its strip draws the letters' bodies and then the one headline.
 * A sequence reads "How はい is written" when its letters spell the headword
 * as one word, and "How the letters வ, க are written" when the headword is a
 * LIST — the letters joined back together are then not the headword, because
 * the separators are gone. A strip that holds a vowel sign reads "How மேசை is
 * written, part by part": its pieces are in WRITTEN order, so they never spell
 * the headword back.
 */
export function filmstripImageMarkdown(target: ScriptFilmstripTarget): string {
  const file = `figures/${basename(target.output)}`;
  // A shared-headline word: its letters' bodies, then one headline across it.
  if (target.composition === "shared-headline") {
    return `![How ${target.glyph} is written, letter by letter, then one headline, stroke by stroke](${file})`;
  }
  if (target.letters === undefined) return `![How ${target.glyph} is written, stroke by stroke](${file})`;
  // A strip with a vowel sign in it draws PIECES in written order ("மேசை" is
  // ே, ம, ை, ச), which never spell the headword back, so it is captioned by
  // its parts.
  if (target.letters.some((letter) => SIGNS_ONLY.test(letter))) {
    return `![How ${target.glyph} is written, part by part, stroke by stroke](${file})`;
  }
  const what = target.letters.join("") === target.glyph
    ? `How ${target.glyph} is written`
    : `How the letters ${target.glyph} are written`;
  return `![${what}, letter by letter, stroke by stroke](${file})`;
}

/**
 * Lessons as the BOOK sees them: each filmstrip target's image placed at the
 * top of its lesson's first Writing block (else its first Script block), above
 * the numbered strokes it draws.
 *
 * Only the book gets this view. Narration and the app keep the authored lesson,
 * because a listener cannot see a figure. A lesson that already references its
 * figure somewhere is left alone, so an author can still place one by hand.
 */
export function withFilmstripImages(
  lessons: readonly ParsedLesson[],
  targets: readonly FigureTarget[],
): ParsedLesson[] {
  const byLesson = new Map<string, ScriptFilmstripTarget>();
  for (const target of targets) {
    if (target.kind === "script-filmstrip") byLesson.set(target.lessonId, target);
  }
  return lessons.map((lesson) => {
    const target = byLesson.get(lesson.realization.lessonId);
    if (target === undefined) return lesson;
    const file = `figures/${basename(target.output)}`;
    if (lesson.blocks.some((block) => block.markdown.includes(file))) return lesson;
    const at = filmstripBlockIndex(lesson);
    if (at === -1) return lesson;
    const blocks = lesson.blocks.map((block, index) =>
      index === at
        ? { ...block, markdown: `${filmstripImageMarkdown(target)}\n\n${block.markdown.replace(/^\s+/, "")}` }
        : block,
    );
    return { ...lesson, blocks };
  });
}
