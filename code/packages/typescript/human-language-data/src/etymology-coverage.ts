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
 *       why postpositions trail their noun and never says where पर came from.
 *       Rules 1 and 2 cannot see this; the empty-block count below can;
 *     - a letter's origin rather than the word's: "**г**. It is Greek
 *       **gamma**" (RU-C03-govorit, whose word origin is in the hook only);
 *     - a "from English" that is not about a word at all: FA-C03-esm-e-
 *       shoma-chist's question mark "from English **?**";
 *     - a comparison or a translation rather than a descent: "different
 *       from English", "translated from English", "Urdu differs from
 *       Hindi" all read as "from <language>";
 *     - a remark about SCRIPT: "this letter is Arabic" satisfies the
 *       bare-copula ancestor rule.
 *   FALSE NEGATIVES (says an origin, counted invisible)
 *     - an origin told without naming a language: "It grew out of *vuestra
 *       merced*" (ES-C07-hablan), or a native word called "inherited";
 *     - a derivation inside the track's own language: "the first piece is
 *       *yek* worn down" (FA-C18-yazdah) takes eleven apart into one and ten
 *       and never names a source, because there is none to name;
 *     - an origin stated only in a section HEADING ("## From Latin"): the
 *       parser keeps the heading out of the block's markdown, and headings
 *       are matched only against the origin-heading names.
 *   The looser signals that would close the false negatives -- a bare
 *   "borrowed", "inherited", "same word" -- were measured and rejected
 *   CORPUS-WIDE: on samples they fired as often on "She inherited the house"
 *   and "the same word covers it" as on etymology. Inside a "taken apart"
 *   block they are another matter; see the next section.
 *
 * THE EMPTY "TAKEN APART" BLOCK (HL24 section 9.4)
 *
 *   wordLessonsWithEmptyEtymologyBlock   visible on rule 1 or rule 2 ALONE:
 *                                        a "taken apart" block or an origin
 *                                        heading is there, and nothing in the
 *                                        lesson says where the word came from.
 *
 * A review of the Spanish track found 48 "taken apart" blocks that take
 * apart a sentence or list the word's parts and stop. Rule 1 counts every one
 * of them as visible etymology, so the visible count can reach zero over a
 * track that still owes them. This count is disjoint from
 * `wordLessonsWithoutVisibleEtymology` -- a lesson with no structure at all
 * is already there -- and it has its own exact pin.
 *
 * A lesson is counted when it passes on structure, the preamble fails rule 3,
 * and no block passes `blockStatesOrigin`: rule 3, OR -- inside a block the
 * parser types `etymology`, and only there -- one of the loose signals in
 * TAKEN_APART_ORIGIN_SIGNALS. Inside that block the words have had their
 * meaning chosen for them; outside it they have not.
 *
 * Measured on the corpus when the count was added. 4,782 word lessons were
 * visible on structure alone with no rule-3 statement anywhere; the signals
 * read 4,307 of them as origins and left 475. "Fires" is lessons among those
 * 4,782 where the signal matches; "sole" is lessons it alone rescues, the
 * only hits that move the count:
 *
 *     signal          fires  sole     signal          fires  sole
 *     inherited         139    15     derived-from      447    89
 *     borrowed           43    17     cousin-of          19     5
 *     a-loan             17     1     is-english         18    13
 *     taken-over          2     0     donor-word        107    31
 *     tatsama             5     0     native-family   2,165    19
 *     tadbhava           95     0     source-tag      2,387     1
 *     learned-word        0     0     source-named    3,673   120
 *     semi-learned        0     0     uncertain         394   163
 *     built-on          570    74     not-settled        17     1
 *     comes-from        372    21     unknown-origin      2     0
 *     goes-back-to       11     1     no-sure-origin     83    21
 *     traced-to          29    16     disputed            7     0
 *     descends-from      12     4     starred           237     9
 *
 * The left column and the source-tag, uncertain-to-disputed and starred rows
 * are the list HL24 section 9.4 started from. None was dropped; three were
 * TIGHTENED after a misfire or a likely one. `goes-back-to` read "the tenth
 * goes back to behaving" (LA-C56-octavus), so a following gerund is refused.
 * `a-loan` refuses a quoted or emphasised gloss, so *préstamo* -- "a loan" --
 * cannot certify itself (no corpus hit lost). `learned-word` needs its
 * article, so "you have learned words like *casa*" stays out (no corpus hit
 * either way). Six were ADDED, because the list alone left over a thousand
 * blocks the reader plainly sees as origins: `source-named` ("DEDR 3376
 * lists ...", "Vasmer derives ..."), `native-family` ("is native
 * Dravidian"), `derived-from` ("formed from *dessiner*", "*вход* ... from
 * *ходить*"), `donor-word` ("Latin said *hodie*"), `cousin-of` ("a cousin of
 * English *swine*") and `is-english` ("**drei** is English **three**"). A
 * bare donor-language name anywhere in the block was measured and REJECTED:
 * it fired on "in Latin letters", "no Hebrew twin worth the claim" and "like
 * the Dravidian languages to its south", about one hit in seven.
 *
 * About a hundred sole-rescue hits were read in place, every signal's among
 * them. Known misfires, recorded so the count is read honestly:
 *
 *   FALSE POSITIVES (a signal fires, the block names no origin of THIS word)
 *     - a signal about another word: ES-C431-retraso takes *re-* and *tras*
 *       apart and is rescued by "tardar, from a different Latin word";
 *     - a negation: "No English word descends from ك-ت-ب" (AR-C31-kataba),
 *       "not a borrowed verb" (AR-C32-saada). Both blocks also name the root
 *       and its Hebrew cognate, so neither lesson is wrongly cleared -- but a
 *       block saying only the negation would be;
 *     - a descendant, not an ancestor: "English borrowed it late"
 *       (LA-C55-tertius), "English *peril* comes from it". Common in the
 *       Latin track, and harmless only when the block also derives the word,
 *       as every sampled one did;
 *     - "not recorded in EtymDB 2.1" and "uncertain ... DEDR has no entry"
 *       are counted as answered. That is deliberate: HL24 section 9.1 allows
 *       "unknown" stated plainly, and these say it.
 *   FALSE NEGATIVES (counted empty, does say an origin)
 *     - an origin with no signal word and no fixed shape: "Behind it is
 *       **papyrus**" (ES-C301-papel), "The goddess is **Frija**"
 *       (GE-C07-freitag), "on an old Slavic base" (RU-C11-do-vstrechi),
 *       "Latin's general word for apple" (ES-C280-manzana: "general" sits
 *       between the language and the noun);
 *     - a cognate told without a signal: "**werden** is a lost English verb"
 *       (GE-C16-werden), "*-sam*, the ending of English *-some*";
 *     - "unsettled" and "Neither is agreed" (ES-C439-tirar), which
 *       `not-settled` and `disputed` do not cover;
 *     - "coined in Quebec in the 1990s from *courrier électronique*"
 *       (FR-C135-courriel): the verb is too far from "from".
 *   Of 30 of the 475 read in place, 7 were one of these. The rest are what
 *   the count is for: a phrase split into words the reader already owns
 *   (ES-C283-no-hay-de-que), a suffix recipe (ES-C427-vendedor), a sound or
 *   stroke note (ZH-C15-tongxue), a sentence frame (PA-C38-ate-vaak). Parts
 *   in the track's own language with no "from" (RU-C214-nazad, "*на* with
 *   *зад*") are counted empty on purpose: they list the parts and stop. The
 *   count is therefore an upper bound on the empty blocks, and clearing a
 *   false negative costs one origin sentence the block should arguably have
 *   stated plainly anyway.
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
 * catastrophically. The loose signals are fixed phrases with bounded runs
 * between them (`is-english` reads at most 60 characters of headword), and
 * they are tried only on "taken apart" blocks. HTML comments are cut out with an indexOf walk rather
 * than a lazy regex, for the same reason.
 *
 * REPORT-ONLY, AND A RATCHET
 *
 * Nothing here throws. The debt predates the rule, and a gate that fails on
 * recorded debt teaches authors to route around it (the HL05 precedent). The
 * test suite instead pins each track's two counts EXACTLY
 * (tests/root-ledger.test.ts, ETYMOLOGY_GAP_PINS), and its empty-block count
 * beside them (EMPTY_ETYMOLOGY_BLOCK_PINS): a new word lesson without
 * etymology raises a count and fails; a lesson that gains one lowers a count
 * and fails until the pin is lowered with it, so cleared debt can never be
 * quietly re-spent.
 */

import type { ParsedLesson } from "./parse.js";
import type { LessonBodyBlock } from "./types.js";
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

// ---- The empty "taken apart" block -------------------------------------------
//
// Rules 1 and 2 above trust STRUCTURE: a block typed `etymology`, or a heading
// that names an origin, is visible etymology whatever it says. Usually that
// trust is earned. When it is not, the lesson has an etymology-shaped hole --
// a block that promises "the word, taken apart" and then takes apart a
// sentence -- and the visible count cannot see it, because the lesson already
// passed on rule 1. The count below is the second look.
//
// The second look reads the block's PROSE, but it may read it more loosely
// than rule 3 does. Rule 3 runs over every paragraph of every lesson, where
// "borrowed" is as likely to be a library book as a loanword, so rule 3 names
// a language or it says nothing (see "the looser signals ... were measured
// and rejected" above). Inside a block that already announces itself as the
// word taken apart, the same words have had their meaning chosen for them:
//
//     corpus-wide                         inside "taken apart"
//     ---------------------------------   ------------------------------------
//     "She inherited the house"           "*Ōdu* is inherited, not borrowed"
//     "a sentence built on the frame"     "**casar** is built from *casa*"
//     "if you are uncertain, ask"         "The origin of ... is uncertain"
//
// So each signal below is confined to that block, and each was measured on
// the corpus before it was kept; the module comment records the numbers.

/**
 * One loose origin signal: a name for the report and tests, and its pattern.
 * A function when the pattern depends on the track, as `donor-word` does:
 * it must skip the track's own language, like rule 3's verbless shapes.
 */
export interface TakenApartOriginSignal {
  name: string;
  pattern: RegExp | ((language: string) => RegExp);
}

function signalPattern(signal: TakenApartOriginSignal, language: string): RegExp {
  return typeof signal.pattern === "function" ? signal.pattern(language) : signal.pattern;
}

/**
 * The etymological dictionaries the corpus cites, as they appear in prose.
 *
 * DEDR and CDIAL only with an entry number ("DEDR 3376 lists ..."): a bare
 * acronym is how a block says the dictionary has NO entry, and that sentence
 * alone answers nothing. EtymDB, Vasmer and Lewis & Short are named only in
 * the sentence that reports what they say, including "not recorded in EtymDB
 * 2.1", which is the explicit "unknown" HL24 section 9.1 allows.
 */
const SOURCE_NAMED_IN_PROSE = /\b(?:DEDR|CDIAL)\s+\d|\b(?:EtymDB|Vasmer)\b|\bLewis\s+&\s+Short\b/;

/**
 * "formed from *dessiner*", "*porte*, from *porter*", "came from **res nata**".
 *
 * The verb list is closed and the form must be emphasised, so "from Monday"
 * and "differently from *go*" (a spelling comparison in ES-C436-recoger)
 * stay out: neither "differently" nor a bare space before "from" is on it.
 * The comma is the corpus's appositive derivation, which carries most of the
 * Russian track's "*вход* is *в*, with *ход*, from *ходить*" -- and may sit
 * inside a closing quote: `"favour, grace," from *grātus*` (LA-C01-gratias).
 * A dash does the same job: `"hail" — from *avēre*` (LA-C01-ave).
 */
const DERIVED_FROM_FORM = new RegExp(
  "(?:[,—–][\"”]?|\\b(?:formed|made|derived|fused|coined|shortened|cut\\s+(?:short|down)|worn\\s+down|came|grew))" +
    "\\s+from\\s+(?:the\\s+)?[*_]{1,3}[^\\s*_]",
  "i",
);

/**
 * A donor language named as the word's ancestor, by the nouns and verbs the
 * corpus tells that story with: "Latin said *hodie*", "two Latin words: *per
 * hoc*", "Latin's general word for apple", "a Germanic verb meaning to tear",
 * "Two Greek pieces". Built per track, like `donorPatterns`, so the Latin
 * track's "Latin had two words here" -- a fact about the language being
 * taught -- stays out.
 *
 * The bare language name was measured too and rejected: it fired on "in Latin
 * letters", "no Hebrew twin worth the claim" and "like the Dravidian languages
 * to its south". The closed noun list is what keeps those out.
 */
const DONOR_WORD_LANGUAGES = [
  ...DONOR_LANGUAGES, "Slavic", "Gothic", "Frankish", "Norse",
];
const DONOR_WORD_NOUNS =
  "had|said|used|counted|word|words|verb|verbs|noun|phrase|pronoun|root|roots|stem|source|form|forms|pieces|ancestor";

const donorWordCache = new Map<string, RegExp>();

function donorWordPattern(language: string): RegExp {
  const cached = donorWordCache.get(language);
  if (cached !== undefined) return cached;
  const own = language.toLowerCase();
  const donors = DONOR_WORD_LANGUAGES.filter((name) => name.toLowerCase() !== own).join("|");
  const pattern = new RegExp(`\\b(?:${donors})(?:['’]s)?\\s+(?:${DONOR_WORD_NOUNS})\\b`);
  donorWordCache.set(language, pattern);
  return pattern;
}

/**
 * Every loose signal, in the order the HL24 section 9.4 table lists them.
 *
 * Each pattern is a fixed phrase with at most a bounded run of emphasis
 * markers or whitespace in it, so none can backtrack catastrophically; the
 * one lookahead (`goes-back-to`) is tried only after its fixed phrase has
 * matched.
 */
export const TAKEN_APART_ORIGIN_SIGNALS: readonly TakenApartOriginSignal[] = [
  // How the word entered the language.
  { name: "inherited", pattern: /\binherited\b/i },
  { name: "borrowed", pattern: /\bborrowed\b/i },
  // Not after a quote or emphasis mark: `**préstamo** -- "a loan"` is a gloss.
  { name: "a-loan", pattern: /(?<![“"'‘*_])\ba\s+loan/i },
  { name: "taken-over", pattern: /\btaken\s+over\b/i },
  { name: "tatsama", pattern: /\btatsamas?\b/i },
  { name: "tadbhava", pattern: /\btadbhavas?\b/i },
  // The article is required: "you have learned words like ..." is not a term.
  { name: "learned-word", pattern: /\b(?:an?|the)\s+learned\s+(?:word|borrowing)s?\b/i },
  { name: "semi-learned", pattern: /\bsemi-learned\b/i },
  // Where it came from, in a verb.
  { name: "built-on", pattern: /\bbuilt\s+(?:on|from)\b/i },
  { name: "comes-from", pattern: /\bcomes?\s+from\b/i },
  // "the tenth goes back to behaving" (LA-C56-octavus) is a return, not a descent.
  { name: "goes-back-to", pattern: /\bgoes\s+back\s+to\b(?!\s+\w+ing\b)/i },
  { name: "traced-to", pattern: /\btraced\s+to\b/i },
  { name: "descends-from", pattern: /\bdescends?\s+from\b/i },
  { name: "derived-from", pattern: DERIVED_FROM_FORM },
  // Kinship told as family: "a cousin of English *swine*", "its relative in Greek".
  {
    name: "cousin-of",
    pattern: new RegExp(
      `\\b(?:cousins?|twins?|relatives?|kin)\\s+(?:of|to|in)\\s+(?:the\\s+)?[*_]{0,3}` +
        `${PERIOD_QUALIFIERS}(?:${SOURCE_LANGUAGES})\\b`,
      "i",
    ),
  },
  // The German track's cognate line, "**drei** is English **three**": both
  // forms in BOLD. A sound note writes its letters in italics -- "German *w*
  // is English *v*" -- which is why rule 3 leaves English out of its copula.
  { name: "is-english", pattern: /\*\*[^*\n]{1,60}\*\*\s+(?:is|are)\s+English\s+\*\*/ },
  { name: "donor-word", pattern: donorWordPattern },
  { name: "native-family", pattern: /\bnative\s+[*_]{0,3}(?:Dravidian|Indo-Aryan|Indo-European|Germanic|Slavic|Iranian|Semitic|Romance)\b/i },
  // Who says so. Case-sensitive: these are proper names.
  { name: "source-tag", pattern: /\((?:DEDR|CDIAL|EtymDB|Lewis\s+&\s+Short|Vasmer|Lane)\b/ },
  { name: "source-named", pattern: SOURCE_NAMED_IN_PROSE },
  // The explicit "unknown" or "disputed" that HL24 section 9.1 allows.
  { name: "uncertain", pattern: /\buncertain\b/i },
  { name: "not-settled", pattern: /\bnot\s+settled\b/i },
  { name: "unknown-origin", pattern: /\bunknown\s+origin\b|\borigin\s+(?:is|remains)\s+unknown\b/i },
  { name: "no-sure-origin", pattern: /\bno\s+sure\s+origin\b/i },
  { name: "disputed", pattern: /\bdisputed\b/i },
  // A starred reconstruction. The star is escaped in Markdown -- `*\*ātmanaka*`
  // -- and must touch a letter, so a literal "5 \* 3" stays out.
  { name: "starred", pattern: /\\\*[^\s*\\]/ },
];

/**
 * The names of every loose signal that fires in `text`, comments stripped.
 *
 * Exported for the tests and for re-measuring the signals on the corpus;
 * `blockStatesOrigin` only needs to know whether ANY fires.
 */
export function takenApartOriginSignals(text: string, language = ""): string[] {
  const visible = withoutComments(text);
  return TAKEN_APART_ORIGIN_SIGNALS.filter((signal) => signalPattern(signal, language).test(visible)).map(
    (signal) => signal.name,
  );
}

/**
 * Does this block say where its word comes from?
 *
 * Rule 3's `statesOrigin` anywhere; the loose signals only inside a block the
 * parser types `etymology`, the "taken apart" block. A heading that merely
 * NAMES an origin (rule 2) does not earn the loose reading: "## Where it comes
 * from" over "the answer comes from context" would otherwise certify itself.
 */
export function blockStatesOrigin(
  block: Pick<LessonBodyBlock, "type" | "markdown">,
  language = "",
): boolean {
  if (statesOrigin(block.markdown, language)) return true;
  if (block.type !== "etymology") return false;
  const visible = withoutComments(block.markdown);
  return TAKEN_APART_ORIGIN_SIGNALS.some((signal) => signalPattern(signal, language).test(visible));
}

/**
 * Is this lesson's visible etymology an empty heading?
 *
 * True when the lesson passes `hasVisibleEtymology` ONLY on structure (rule 1
 * or rule 2) and nothing in it says where the word came from: no block passes
 * `blockStatesOrigin`, and the preamble fails `statesOrigin`. Every block is
 * read, not only the "taken apart" one -- an origin stated in "You'll want to
 * know" is still an origin the reader sees.
 *
 * A lesson with no structural etymology at all is never counted here; it is
 * already in `wordLessonsWithoutVisibleEtymology`, or it passed on prose.
 */
export function hasEmptyEtymologyBlock(lesson: ParsedLesson): boolean {
  const structural =
    lesson.blocks.some((block) => block.type === "etymology") ||
    lesson.blocks.some((block) => ORIGIN_HEADING.test(block.title));
  if (!structural) return false;
  if (statesOrigin(lesson.preamble, lesson.language)) return false;
  return !lesson.blocks.some((block) => blockStatesOrigin(block, lesson.language));
}

// ---- The report --------------------------------------------------------------

/**
 * One word-introducing lesson missing at least one of the two kinds, or whose
 * visible etymology is an empty "taken apart" block.
 */
export interface EtymologyGap {
  lessonId: string;
  language: string;
  /** Zero-based position in the track's reading order, among ALL its lessons. */
  position: number;
  missingHook: boolean;
  missingVisible: boolean;
  /**
   * Visible only on structure: a "taken apart" block or origin heading that
   * never says where the word came from (`hasEmptyEtymologyBlock`). Never
   * true together with `missingVisible`, which is the lesson with no
   * etymology structure at all.
   */
  emptyEtymologyBlock: boolean;
}

export interface TrackEtymologyCoverage {
  language: string;
  /** Lessons in the track that introduce a word -- the denominator. */
  wordLessons: number;
  wordLessonsWithoutEtymologyHook: number;
  wordLessonsWithoutVisibleEtymology: number;
  /** Neither a hook nor a visible statement: the lessons to work first. */
  wordLessonsWithoutAnyEtymology: number;
  /**
   * Counted visible only because a "taken apart" block or origin heading is
   * there, while nothing in the lesson says where the word came from. A
   * subset of the lessons NOT in `wordLessonsWithoutVisibleEtymology`.
   */
  wordLessonsWithEmptyEtymologyBlock: number;
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
    wordLessonsWithEmptyEtymologyBlock: number;
    /** Share of word lessons with a hook, rounded down so 99.6% never prints as 100. */
    hookPercent: number;
    /** Share of word lessons with visible etymology, rounded down for the same reason. */
    visiblePercent: number;
  };
}

/** Floor percentage; 100 only when nothing is missing, and 100 for an empty denominator. */
function coveredPercent(total: number, missing: number): number {
  // Multiply before dividing: (71/100)*100 is 70.999… in floating point.
  return total === 0 ? 100 : Math.floor(((total - missing) * 100) / total);
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
      wordLessonsWithEmptyEtymologyBlock: 0,
    };
    ordered.forEach((lesson, position) => {
      if (!introducesNewWord(lesson)) return;
      track.wordLessons += 1;
      const missingHook = !hasEtymologyHook(lesson);
      const missingVisible = !hasVisibleEtymology(lesson);
      if (missingHook) track.wordLessonsWithoutEtymologyHook += 1;
      if (missingVisible) track.wordLessonsWithoutVisibleEtymology += 1;
      if (missingHook && missingVisible) track.wordLessonsWithoutAnyEtymology += 1;
      // Only a lesson that PASSED the visible test can have an empty block:
      // one that failed is already counted, and counting it twice would make
      // the two numbers overlap.
      const emptyEtymologyBlock = !missingVisible && hasEmptyEtymologyBlock(lesson);
      if (emptyEtymologyBlock) track.wordLessonsWithEmptyEtymologyBlock += 1;
      if (missingHook || missingVisible || emptyEtymologyBlock) {
        gaps.push({
          lessonId: lesson.realization.lessonId,
          language,
          position,
          missingHook,
          missingVisible,
          emptyEtymologyBlock,
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
      wordLessonsWithEmptyEtymologyBlock: sum((t) => t.wordLessonsWithEmptyEtymologyBlock),
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
    `  ${summary.wordLessonsWithEmptyEtymologyBlock} more count as visible only because a "taken apart" ` +
      "block or origin heading is there, and never say where the word came from",
  ];
  for (const track of report.tracks) {
    lines.push(
      `  ${clean(track.language)}: ${track.wordLessons} word lessons; ` +
        `${track.wordLessonsWithoutEtymologyHook} no hook, ` +
        `${track.wordLessonsWithoutVisibleEtymology} no visible etymology, ` +
        `${track.wordLessonsWithoutAnyEtymology} neither, ` +
        `${track.wordLessonsWithEmptyEtymologyBlock} empty etymology block`,
    );
  }
  return lines;
}
