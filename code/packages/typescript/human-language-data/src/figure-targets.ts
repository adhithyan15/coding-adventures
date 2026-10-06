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
// strips below, a list of letters or a word whose letters stand apart), with a `## Writing:`
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
 * signs have a cited written order. "வணக்கம்" does not yet: its puḷḷi has
 * neither a cited ductus nor a written-order row (`WRITTEN_SIGN_SIDES`).
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
 * Bengali and Gurmukhi are absent for the Devanagari reason above (a shared
 * headline), as well as for having no ductus at all yet.
 */
export const SEPARATE_LETTER_SCRIPTS: ReadonlySet<string> = new Set([
  "chinese",
  "japanese",
  "tamil",
  "gujarati",
  "kannada",
  "telugu",
  "malayalam",
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
//     of the consonant, and units written from left to right). A test holds
//     this table to those records.
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
// Everything without a row is refused: the Tamil puḷḷi ் and the signs ு and
// ூ (no cited ductus, and ு and ூ fuse with their consonant into shapes of
// their own), Gujarati ્ and ૃ, and every sign in every other script
// (Devanagari ि, the kana voicing mark ゙), because no other script has a
// table yet.
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
  const pieces = letters.map((letter) => writtenPiecesOf(letter, script));
  if (pieces.some((piece) => piece === undefined)) return undefined;
  return pieces.flat() as string[];
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
 * A sequence reads "How はい is written" when its letters spell the headword
 * as one word, and "How the letters வ, க are written" when the headword is a
 * LIST — the letters joined back together are then not the headword, because
 * the separators are gone. A strip that holds a vowel sign reads "How மேசை is
 * written, part by part": its pieces are in WRITTEN order, so they never spell
 * the headword back.
 */
export function filmstripImageMarkdown(target: ScriptFilmstripTarget): string {
  const file = `figures/${basename(target.output)}`;
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
