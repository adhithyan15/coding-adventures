/**
 * HL24 section 9 -- etymology coverage: does every new word say where it comes from?
 *
 * WHY THIS IS MEASURED
 *
 * HL00 calls etymology "the engine of the whole curriculum": a new word is
 * meant to arrive as a cousin of words the reader already owns, never as a
 * fresh noise to memorise. That promise is made per WORD, so it can be checked
 * per word -- and until this module nobody had. The Root Ledger (root-ledger.ts)
 * asks whether a root, once named, is spent again. It cannot ask the prior
 * question: was the root ever named at all? A lesson with no etymology adds
 * nothing to the ledger, so the ledger simply never sees it.
 *
 * WHAT THE FIRST MEASUREMENT FOUND
 *
 * 25,682 of the 29,572 lessons introduce a word. 22,744 of them carry no
 * `etymology_hook`, 22,378 show the reader no recognisable origin statement
 * anywhere in the lesson body, and 21,697 have neither. Most of the gap is
 * the generated vocabulary waves, whose lessons are four lines of "**valoir**
 * -- 'to be worth'" and nothing else. The hand-written opening chapters are
 * where the etymology lives.
 *
 * THE RULE THIS MEASURES (HL24 section 9)
 *
 *   Every lesson that introduces a word states where that word comes from,
 *   from a cited source. "Unknown" -- or "disputed", with the competing
 *   accounts named -- is an allowed and explicit answer. An invented
 *   etymology never is: a false cognate is worse than none (HL00).
 *
 * So this module never judges whether an etymology is TRUE. That needs a
 * dictionary and a human. It counts the lessons where the question was not
 * answered at all, which is the part a machine can see.
 *
 * TWO MEASURES, BECAUSE THE HOOK IS INVISIBLE TO THE READER
 *
 *   wordLessonsWithoutEtymologyHook      the frontmatter `etymology_hook` is
 *                                        empty or only whitespace.
 *   wordLessonsWithoutVisibleEtymology   the lesson BODY -- what the book
 *                                        prints and the narration reads --
 *                                        carries no recognisable origin
 *                                        statement.
 *
 * Both are needed because neither implies the other. `etymology_hook` is read
 * by the language-ladder app's cards and by nothing that prints: book.ts and
 * narration.ts never render it (tests/standalone-book.test.ts records the same
 * fact from the other side). A lesson can therefore carry a perfect hook and
 * tell the book reader nothing -- GE-C17-kopf's hook traces *Kopf* to Late
 * Latin *cuppa*, and its body never mentions where the word came from. And a
 * lesson can tell the reader everything in prose with an empty hook, as
 * ES-C01-buenos-dias does. 681 lessons are the first kind and 1,047 the
 * second. Measuring one would certify half the promise.
 *
 * WHAT COUNTS AS INTRODUCING A WORD
 *
 * A lesson introduces a word when EITHER
 *
 *   (a) it introduces a lexical knowledge atom -- `<XX>-LEX-...` in
 *       `introduces.knowledge` or in any block's `hl-knowledge introduces=[...]`
 *       (unioned by `introducedAtoms`), OR
 *   (b) its `type` is `word` or `phrase` -- which includes a lesson with no
 *       `type` at all, since the parser defaults that to `word`.
 *
 * (a) is the knowledge graph's own declaration that a word is new, and it is
 * what catches the handful of practice, grammar and listening lessons that
 * slip a new word in. (b) is needed because 632 `word` lessons in older tracks
 * declare their words under legacy atom names (`AR-C01-SALAM-01`,
 * `AR-CONCEPT-SHUKRAN-21`) and would otherwise vanish from the count -- and a
 * headword is itself a claim to introduce that word (lessons.d: a-lesson-
 * headword-is-a-claim-to-introduce-that-word-even-when). The atom test is
 * anchored on a two-letter track prefix, so RETROFLEX, REFLEXIVE and
 * CIRCUMFLEX atoms, which all contain the letters LEX, never match.
 *
 * A review, practice or writing lesson that introduces no lexical atom is
 * outside the count: it re-uses words whose own lessons owe their etymology.
 * A PHRASE headword is inside it. *de nada* from *res nata* and *buenos días*
 * as the fossil of a blessing are exactly the etymology HL00 asks for.
 *
 * WHAT COUNTS AS "VISIBLE" ETYMOLOGY -- deliberately conservative
 *
 * A lesson's body shows etymology when ANY of these holds:
 *
 *   1. it has a block the parser types `etymology` -- a level-two heading
 *      containing "taken apart", the corpus's own etymology section (2,753 of
 *      the word lessons carry one);
 *   2. a block heading names it: "etymology", "origin", "where it comes from";
 *   3. its prose (title, block text, cue lines; HTML comments stripped) holds
 *      an origin statement:
 *        - the derivation arrow `←` (`día ← dies`), the corpus's notation;
 *        - "from", "via" or "through" followed by a source language, with an
 *          optional "the" and period qualifiers: "from Sanskrit",
 *          "through Old French", "via Proto-Dravidian";
 *        - a reconstructed ancestor: "Proto-Germanic", "PIE";
 *        - kinship stated against a named language: "cognate with",
 *          "the same word as English";
 *        - a donor language named as the ancestor without a verb:
 *          "Latin *october*", "**कब** is Sanskrit **कदा**" (never the
 *          track's own language; see `donorPatterns`).
 *
 * Read in place on the corpus, and recorded so the number is read honestly:
 *
 *   FALSE POSITIVES (counted visible, says no origin)
 *     - a "taken apart" block that takes apart SYNTAX: HI-C87-par explains
 *       why postpositions trail their noun and never says where पर came from;
 *     - a letter's origin rather than the word's: "**г**. It is Greek
 *       **gamma**" (RU-C03-govorit, whose word origin is in the hook only);
 *     - a "from English" that is not about a word at all: FA-C03-esm-e-
 *       shoma-chist's question mark "from English **?**".
 *   FALSE NEGATIVES (says an origin, counted invisible)
 *     - an origin told without naming a language: "It grew out of *vuestra
 *       merced*" (ES-C07-hablan), or a native word called "inherited";
 *     - a derivation inside the track's own language: "the first piece is
 *       *yek* worn down" (FA-C18-yazdah) takes eleven apart into one and ten
 *       and never names a source, because there is none to name.
 *   The looser signals that would close the false negatives -- a bare
 *   "borrowed", "inherited", "same word" -- were measured and rejected: on
 *   samples they fired as often on "She inherited the house" and "the same
 *   word covers it" as on etymology.
 *
 * The hook measure has its own false positive, and no mechanical fix: a
 * non-empty hook that is not etymology. JA-C12-kami's reads "Hair adds no new
 * sign", MW-C14-sasta's names the glyphs that spell the word. A string
 * present is not a promise kept, so the hook count is a FLOOR on the debt.
 *
 * LINEAR TIME
 *
 * Every lesson is visited once and every pattern is tested once per block.
 * The only repeated GROUP in any pattern, the period qualifiers, repeats at
 * most three times, and every alternative inside it starts with a fixed
 * word, so no input -- however hostile -- can make a pattern backtrack
 * catastrophically. HTML comments are cut out with an indexOf walk rather
 * than a lazy regex, for the same reason.
 *
 * REPORT-ONLY, AND A RATCHET
 *
 * Nothing here throws. The debt predates the rule, and a gate that fails on
 * recorded debt teaches authors to route around it (the HL05 precedent). The
 * test suite instead pins each track's two counts EXACTLY
 * (tests/root-ledger.test.ts, ETYMOLOGY_GAP_PINS): a new word lesson without
 * etymology raises a count and fails; a lesson that gains one lowers a count
 * and fails until the pin is lowered with it, so cleared debt can never be
 * quietly re-spent.
 */

import type { ParsedLesson } from "./parse.js";
import { introducedAtoms, readingOrder } from "./ramp.js";
import { stripControlCharacters as clean } from "./constants.js";

// ---- What counts as introducing a word -------------------------------------

/**
 * A lexical knowledge atom: two-letter track prefix, then `LEX`.
 *
 * Anchored, because a bare `/LEX/` matches `HI-SCRIPT-DA-RETROFLEX-01`,
 * `FR-GRAM-REFLEXIVE-VERB-06` and `ZH-TONE-LEXICAL` -- twelve atoms in the
 * corpus that name a sound, a grammar point or a tone, never a word.
 */
export const LEXICAL_ATOM = /^[A-Z]{2}-LEX-/;

/** Lesson types whose headword is, by declaration, a new word or phrase. */
export const WORD_LESSON_TYPES: ReadonlySet<string> = new Set(["word", "phrase"]);

/**
 * Does this lesson introduce a new word? See the module comment for why the
 * answer is a union of the atom test and the type test.
 */
export function introducesNewWord(lesson: ParsedLesson): boolean {
  if (WORD_LESSON_TYPES.has(lesson.realization.type)) return true;
  return introducedAtoms(lesson).some((atom) => LEXICAL_ATOM.test(atom));
}

// ---- The two "has etymology" tests -----------------------------------------

/**
 * Does the frontmatter carry an `etymology_hook`?
 *
 * Whitespace-only is EMPTY. A hook of three spaces is the field left blank by
 * a template, not a memory anchor, and counting it would let a generator
 * certify a whole wave by printing `etymology_hook: " "`.
 */
export function hasEtymologyHook(lesson: ParsedLesson): boolean {
  return lesson.realization.etymologyHook.trim() !== "";
}

/**
 * Source languages an origin statement may name.
 *
 * The 23 track languages and English, plus the historical sources their
 * vocabularies actually draw on. The list is closed on purpose: an open "from <Capital
 * word>" would read "from Monday" and "from Roberto" as etymology.
 */
const SOURCE_LANGUAGES = [
  // Track languages, and English.
  "Arabic", "Bengali", "Chinese", "English", "French", "German", "Gujarati",
  "Hindi", "Italian", "Japanese", "Kannada", "Latin", "Malayalam", "Marathi",
  "Marwari", "Persian", "Portuguese", "Punjabi", "Russian", "Sanskrit",
  "Spanish", "Tamil", "Telugu", "Urdu",
  // Families and stages a word is traced back through.
  "Indo-European", "Indo-Aryan", "Indo-Iranian", "Iranian", "Germanic",
  "Romance", "Dravidian", "Semitic", "Slavic", "Slavonic", "Turkic", "Celtic",
  "Prakrit", "Pali", "Apabhramsha", "Hindustani", "Avestan", "Pahlavi",
  // Neighbours and donors.
  "Greek", "Hebrew", "Aramaic", "Turkish", "Mongolian", "Norse", "Gothic",
  "Frankish", "Gaulish", "Etruscan", "Dutch", "Catalan", "Occitan", "Basque",
  "Mozarabic", "Nahuatl", "Quechua", "Tupi", "Malay", "Korean", "Cantonese",
  "Hokkien", "Manchu",
].join("|");

/**
 * Period qualifiers that may stand between "from" and the language:
 * "from OLD French", "via PROTO-Dravidian", "through VULGAR Latin". At most
 * three, so the repetition is bounded.
 */
const PERIOD_QUALIFIERS =
  "(?:(?:Old|Middle|Classical|Vulgar|Late|Ecclesiastical|Medieval|Ancient|Early|" +
  "Modern|Proto|Common|Biblical|Vedic|High|Low|Church)[\\s-]+){0,3}";

/**
 * "from / via / through" + a source language.
 *
 * `[*_]{0,3}` admits Markdown emphasis between them ("from **Persian**").
 * `\s+` admits a line break, which is why blocks are tested whole rather than
 * line by line: prose wrapped at eighty columns can put "from" at the end of
 * one line and "Latin" at the start of the next.
 *
 * The negative lookahead rejects the two non-etymological readings measured
 * on the corpus: a possessive ("through Arabic's patterns") and a script
 * ("missing from the Gujarati script").
 */
const ORIGIN_FROM_LANGUAGE = new RegExp(
  `\\b(?:[Ff]rom|[Vv]ia|[Tt]hrough)\\s+(?:the\\s+)?[*_]{0,3}${PERIOD_QUALIFIERS}` +
    `(?:${SOURCE_LANGUAGES})\\b(?!['’]s\\b|\\s+script)`,
);

/** A reconstructed ancestor named outright: "Proto-Germanic *namô", "PIE *dʰrewgʰ-". */
const RECONSTRUCTED_ANCESTOR = /\bProto-[A-Z]|\bPIE\b/;

/**
 * Kinship stated against a named language: "cognate with Hebrew ...",
 * "the same word as English sun". The language is required -- "the same word
 * covers it" is a statement about meaning, not ancestry.
 */
const KINSHIP_WITH_LANGUAGE = new RegExp(
  `\\b(?:cognates?\\s+(?:with|of|to)|same\\s+(?:word|root)\\s+as)\\s+[*_]{0,3}` +
    `${PERIOD_QUALIFIERS}(?:${SOURCE_LANGUAGES})\\b`,
  "i",
);

/**
 * The classical and donor languages whose NAME, followed directly by an
 * emphasised form, is how this corpus writes an ancestor without a verb:
 * "Latin *october*, from *octo*", "Sanskrit **कदा**, worn down", "Arabic
 * *faṣl*". Restricted to languages a word is traced BACK to: a modern
 * language followed by an emphasised word is usually a comparison ("like
 * Spanish *hola*"), not an origin.
 */
const DONOR_LANGUAGES = [
  "Latin", "Greek", "Sanskrit", "Prakrit", "Pali", "Persian", "Arabic",
  "Hebrew", "Aramaic", "Avestan", "Germanic", "Dravidian",
];

/**
 * The two verbless ancestor shapes, built per track because each must skip
 * the track's OWN language. In the Latin track "Latin *aqua*" names the word
 * being taught, not where it came from; in the Persian track "Arabic *kitāb*"
 * is exactly a loanword statement.
 *
 *   "<Donor> *form*"            Sanskrit *satya*, built on *sat*
 *   "is / was <Donor>"          **कब** is Sanskrit **कदा**; सुबह was Persian
 *
 * English and French are deliberately absent from the second shape: German
 * sound notes say "German *w* is English *v*" dozens of times, and that is
 * spelling, not ancestry.
 */
interface DonorPatterns {
  namedForm: RegExp;
  copula: RegExp;
}

const donorPatternCache = new Map<string, DonorPatterns>();

function donorPatterns(language: string): DonorPatterns {
  const cached = donorPatternCache.get(language);
  if (cached !== undefined) return cached;
  const own = language.toLowerCase();
  const donors = DONOR_LANGUAGES.filter((name) => name.toLowerCase() !== own).join("|");
  const patterns: DonorPatterns = {
    namedForm: new RegExp(`\\b${PERIOD_QUALIFIERS}(?:${donors})\\s+[*_]{1,3}[^\\s*_]`),
    copula: new RegExp(
      `\\b(?:is|are|was|were)\\s+(?:an?\\s+)?[*_]{0,3}${PERIOD_QUALIFIERS}(?:${donors})\\b(?!['’]s\\b)`,
    ),
  };
  donorPatternCache.set(language, patterns);
  return patterns;
}

/** The corpus's derivation arrow: `día ← dies`. Never `→`, which also writes `a + b → ab`. */
const DERIVATION_ARROW = "←";

/** A heading that announces an origin section in so many words. */
const ORIGIN_HEADING =
  /\b(?:etymology|etymologies|origins?|where (?:it|the word|this word|they|the words) comes? from)\b/i;

/**
 * The text with every `<!-- ... -->` removed.
 *
 * Directive comments carry atom ids and activity JSON; an activity's feedback
 * string can say "from Latin" without the reader ever being shown it as
 * prose. An indexOf walk rather than `/<!--[\s\S]*?-->/g`, so an unterminated
 * comment costs one pass, not a rescan per opener. An unterminated comment
 * swallows the rest of the text -- the same reading a Markdown renderer gives
 * it, since nothing after an open comment is displayed.
 */
function withoutComments(text: string): string {
  const parts: string[] = [];
  let cursor = 0;
  while (cursor < text.length) {
    const open = text.indexOf("<!--", cursor);
    if (open === -1) {
      parts.push(text.slice(cursor));
      break;
    }
    parts.push(text.slice(cursor, open));
    const close = text.indexOf("-->", open + 4);
    if (close === -1) break;
    cursor = close + 3;
  }
  return parts.join("");
}

/**
 * Does this prose state an origin? Rule 3 of the module comment.
 *
 * `language` is the track id (`"sanskrit"`), used only to skip the track's
 * own language in the verbless shapes; see `donorPatterns`.
 */
export function statesOrigin(prose: string, language = ""): boolean {
  const text = withoutComments(prose);
  if (text.includes(DERIVATION_ARROW)) return true;
  if (ORIGIN_FROM_LANGUAGE.test(text)) return true;
  if (RECONSTRUCTED_ANCESTOR.test(text)) return true;
  if (KINSHIP_WITH_LANGUAGE.test(text)) return true;
  const donors = donorPatterns(language);
  return donors.namedForm.test(text) || donors.copula.test(text);
}

/**
 * Does the learner-visible body say where the word comes from?
 *
 * Rules 1-3 of the module comment, in that order, because the structural
 * tests are cheaper than the prose scan and settle most lessons that pass.
 */
export function hasVisibleEtymology(lesson: ParsedLesson): boolean {
  if (lesson.blocks.some((block) => block.type === "etymology")) return true;
  if (lesson.blocks.some((block) => ORIGIN_HEADING.test(block.title))) return true;
  if (statesOrigin(lesson.preamble, lesson.language)) return true;
  return lesson.blocks.some((block) => statesOrigin(block.markdown, lesson.language));
}

// ---- The report --------------------------------------------------------------

/** One word-introducing lesson missing at least one of the two kinds. */
export interface EtymologyGap {
  lessonId: string;
  language: string;
  /** Zero-based position in the track's reading order, among ALL its lessons. */
  position: number;
  missingHook: boolean;
  missingVisible: boolean;
}

export interface TrackEtymologyCoverage {
  language: string;
  /** Lessons in the track that introduce a word -- the denominator. */
  wordLessons: number;
  wordLessonsWithoutEtymologyHook: number;
  wordLessonsWithoutVisibleEtymology: number;
  /** Neither a hook nor a visible statement: the lessons to work first. */
  wordLessonsWithoutAnyEtymology: number;
}

export interface EtymologyCoverageReport {
  /** Per track, alphabetical. */
  tracks: TrackEtymologyCoverage[];
  /**
   * Every gap, per track in reading order. Large -- tens of thousands of
   * entries today -- so the gap report publishes `tracks` and `summary` only;
   * the ratchet test reads this to name the lessons behind a moved count.
   */
  gaps: EtymologyGap[];
  summary: {
    wordLessons: number;
    wordLessonsWithoutEtymologyHook: number;
    wordLessonsWithoutVisibleEtymology: number;
    wordLessonsWithoutAnyEtymology: number;
    /** Share of word lessons with a hook, rounded down so 99.6% never prints as 100. */
    hookPercent: number;
    /** Share of word lessons with visible etymology, rounded down for the same reason. */
    visiblePercent: number;
  };
}

/** Floor percentage; 100 only when nothing is missing, and 100 for an empty denominator. */
function coveredPercent(total: number, missing: number): number {
  return total === 0 ? 100 : Math.floor(((total - missing) / total) * 100);
}

/** Measure etymology coverage across every track in `lessons`. */
export function measureEtymologyCoverage(lessons: ParsedLesson[]): EtymologyCoverageReport {
  const byTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    const track = byTrack.get(lesson.language);
    if (track === undefined) byTrack.set(lesson.language, [lesson]);
    else track.push(lesson);
  }

  const tracks: TrackEtymologyCoverage[] = [];
  const gaps: EtymologyGap[] = [];
  for (const language of [...byTrack.keys()].sort()) {
    // Reading order, so a ratchet failure can name the LATEST lessons first:
    // new work is usually appended, and the newest gap is the likeliest culprit.
    const ordered = [...byTrack.get(language)!].sort(readingOrder);
    const track: TrackEtymologyCoverage = {
      language,
      wordLessons: 0,
      wordLessonsWithoutEtymologyHook: 0,
      wordLessonsWithoutVisibleEtymology: 0,
      wordLessonsWithoutAnyEtymology: 0,
    };
    ordered.forEach((lesson, position) => {
      if (!introducesNewWord(lesson)) return;
      track.wordLessons += 1;
      const missingHook = !hasEtymologyHook(lesson);
      const missingVisible = !hasVisibleEtymology(lesson);
      if (missingHook) track.wordLessonsWithoutEtymologyHook += 1;
      if (missingVisible) track.wordLessonsWithoutVisibleEtymology += 1;
      if (missingHook && missingVisible) track.wordLessonsWithoutAnyEtymology += 1;
      if (missingHook || missingVisible) {
        gaps.push({
          lessonId: lesson.realization.lessonId,
          language,
          position,
          missingHook,
          missingVisible,
        });
      }
    });
    tracks.push(track);
  }

  const sum = (pick: (track: TrackEtymologyCoverage) => number) =>
    tracks.reduce((total, track) => total + pick(track), 0);
  const wordLessons = sum((t) => t.wordLessons);
  const withoutHook = sum((t) => t.wordLessonsWithoutEtymologyHook);
  const withoutVisible = sum((t) => t.wordLessonsWithoutVisibleEtymology);
  return {
    tracks,
    gaps,
    summary: {
      wordLessons,
      wordLessonsWithoutEtymologyHook: withoutHook,
      wordLessonsWithoutVisibleEtymology: withoutVisible,
      wordLessonsWithoutAnyEtymology: sum((t) => t.wordLessonsWithoutAnyEtymology),
      hookPercent: coveredPercent(wordLessons, withoutHook),
      visiblePercent: coveredPercent(wordLessons, withoutVisible),
    },
  };
}

/** The gap-report view: everything except the per-lesson gap list. */
export type EtymologyCoverageSummary = Omit<EtymologyCoverageReport, "gaps">;

/** Human-readable lines for the gap report. Accepts the full report or its summary view. */
export function renderEtymologyCoverage(report: EtymologyCoverageSummary): string[] {
  const { summary } = report;
  const lines = [
    "Etymology coverage (HL24 §9) — every lesson that introduces a word says where it comes from:",
    `  ${summary.wordLessons} word-introducing lessons; ` +
      `${summary.wordLessonsWithoutEtymologyHook} carry no etymology_hook (${summary.hookPercent}% covered), ` +
      `${summary.wordLessonsWithoutVisibleEtymology} show the reader no origin in the body ` +
      `(${summary.visiblePercent}% covered), ${summary.wordLessonsWithoutAnyEtymology} have neither`,
    "  the hook is read by the app only; the book and narration print the body, so both are counted",
  ];
  for (const track of report.tracks) {
    lines.push(
      `  ${clean(track.language)}: ${track.wordLessons} word lessons; ` +
        `${track.wordLessonsWithoutEtymologyHook} no hook, ` +
        `${track.wordLessonsWithoutVisibleEtymology} no visible etymology, ` +
        `${track.wordLessonsWithoutAnyEtymology} neither`,
    );
  }
  return lines;
}
