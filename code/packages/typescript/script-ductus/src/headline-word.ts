// ---------------------------------------------------------------------------
// headline-word.ts — a Devanagari or Gurmukhi word: the letters' bodies, then ONE headline
// ---------------------------------------------------------------------------
//
// The problem
// -----------
// Every cited Devanagari letter ends the same way:
//
//     ... body strokes ...,  "lift, then draw the shirorekha rightward"
//
// That is right for a letter written by itself. A printed WORD is different:
// its letters hang from one continuous headline (the shirorekhā), so
//
//     म  +  म      is written      मम      — one line on top, not two.
//
// Putting the two letters' strips side by side would draw a headline per
// letter, which is not how the word is written. So a word gets a strip of its
// own, built here out of parts that are already cited:
//
//     letter 1's body strokes            (as letter 1's own strip draws them)
//     letter 2's body strokes            (moved right by letter 1's advance)
//     ...
//     ONE headline, left to right, over the whole word          <- drawn last
//
// Why the headline comes last
// ---------------------------
// Because that is what most native writers do with a single letter. HP Labs
// India's LipiTk 4.0 Devanagari recognizer stores native writers' tablet pen
// traces as prototypes; across its 33 consonant classes, 82% of the 2,706
// prototypes draw the headline as the LAST stroke and about 5% as the first.
// (The model is MIT; the data under it is research-only, so only counts and
// shares are cited, never a trace.) Every cited letter already ends with its
// headline for that reason.
//
// Two honest limits, which the figure's source note repeats:
//
//   * It is a majority, not a rule: some writers draw the headline first.
//   * The traces are single letters. No reachable source records native
//     writers' headline timing across a whole word, so drawing the word's
//     letters first and its one headline last is this book's reading of the
//     per-letter data, not an observation of words.
//
// Nothing new is authored
// -----------------------
// The headline is FOUND, not drawn: in the Devanagari ductus every letter's
// last stroke is one segment labelled exactly `LETTER_HEADLINE_LABEL`, a
// horizontal line travelled left to right at the headline's height.
// `splitHeadline` takes it off and keeps the rest. A letter whose last stroke
// is not that — and any future letter whose headline is part of a body
// stroke — cannot join a word, and the word is refused.
//
// One sign may join: ā (ा)
// ------------------------
// Noto prints ā as a stem with a short piece of headline on top, and its cited
// ductus draws exactly that: the stem, then (after a lift) the piece, labelled
// like any letter's headline. So it splits like a letter, and in a word
//
//     न  ा  म      is written      न's body, ā's stem, म's body, ONE headline
//
// The sign's PLACE is cited separately, on its mark record's
// `compositionSource` (`HEADLINE_WORD_SIGNS` below): the cited आ draws the
// same bar after its body and before its headline, and most native writers'
// आ do too (52 of HP Labs India's 83 stored prototypes: body, bar, headline).
// Nobody's consonant + ā syllables could be reached, so that is a reading of
// आ, at medium confidence, and the sign's citation says so. ā must follow a
// consonant: after a vowel letter the font prints a dotted circle instead.
// Every other sign is refused: none has a cited place against its consonant
// AND the shared headline (ि's side is unresolved; े's flag splits writers).
//
// A phrase: several words, each with its own headline
// ---------------------------------------------------
// `composeHeadlinePhrase` takes words separated by single spaces and composes
// each one as above. Nothing is drawn between them: each word is its own
// ledger entry, printed as one group of a sequence strip, word by word. One
// word that fails refuses the whole phrase, so a phrase is never printed with
// a word missing.
//
// Where each letter goes comes from the font: the next letter starts where
// this one's ADVANCE ends (`hmtx`), exactly as the printed word places it. The
// word's outline is each letter's outline at that offset.
//
// And then it is checked, not believed
// ------------------------------------
// The composed strokes must fit the printed word at the same default
// tolerances every per-letter ductus meets (tests/support/stroke-honesty.ts):
//
//     each stroke      more than 97% of its length on the word's ink
//     the whole word   under 2% of its ink farther than 100 units from a path
//
// A word that fails is refused with the reason, never drawn. The case that
// matters: a letter whose own headline does not reach its left edge (थ ध भ श,
// and अ आ ओ औ, whose left bowl has no bar) breaks the printed line when it is
// not first, so one straight headline would cross blank paper there.
//
// What this module does NOT decide is which headwords are WORDS worth trying:
// that is the book's question (`figure-targets.ts` in human-language-data,
// which also refuses signs and the letters the font splits while shaping).
//
// Gurmukhi: the same word, from letters that draw their headline FIRST
// --------------------------------------------------------------------
// A Gurmukhi word hangs from one headline too (ਸਿਰੋਰੇਖਾ, shirorekha), but its
// cited letters come from a different kind of source. GNPS's tracing lesson
// (strokes/gurmukhi.ts) teaches each letter with its headline FIRST:
//
//     ਰ   =   "draw the headline from left to right",  then the body
//
// and each letter's own strip keeps that order. A WORD does not: by this
// book's convention its letters' bodies come first, in reading order, and the
// one headline comes LAST, left to right, exactly as a Devanagari word is
// drawn:
//
//     ਮਨਨ   ->   ਮ's strokes,  ਨ's body,  ਨ's body,  then ONE headline
//
// That is the order fluent writers are described as using, and the one the
// book already draws Devanagari words in. It is a CONVENTION, and it says so
// wherever it is printed: no reachable Gurmukhi recording shows a writer's
// headline timing in a word, and the cited letter source itself draws the
// headline first. So the word's source note names GNPS for every letter's
// body and names the convention, not a count, for the headline.
//
// Finding the headline works the same way, mirrored: a Gurmukhi letter's
// headline is its FIRST stroke, one segment labelled exactly
// `GURMUKHI_LETTER_HEADLINE_LABEL`, level and travelled left to right
// (`LETTER_HEADLINE` below names, per script, which end and which label).
// `splitHeadline` takes it off from that end.
//
// Five letters print a SPLIT headline (ਅ ਖ ਘ ਪ ਮ): Noto Sans Gurmukhi leaves a
// gap in the bar above the body, and GNPS draws no separate bar for them: the
// bar's left piece opens the first stroke and its right piece opens the stem
// stroke. Those letters have no headline to take off, so they join a word
// WHOLE, their pieces of bar drawn by their own cited strokes
// (`SPLIT_HEADLINE_LETTERS`). The shared headline then runs from the first to
// the last letter whose own headline WAS taken off, and never across a gap:
//
//     ਪਰ    ->  ਪ's two strokes (its bar pieces with them), ਰ's body,
//               then the headline over ਰ
//     ਅਮਨ   ->  ਅ, ਮ whole; ਨ's body; then the headline over ਨ
//
// A split letter BETWEEN two such letters (ਨਪਨ) would put its gap under one
// straight headline, and the ink check below refuses the word, the same check
// that refuses Devanagari मथ. A word of split letters only has no headline to
// share and is refused too.
//
// The layout is the printed word here as well. Read with fontTools, Noto Sans
// Gurmukhi 2.004 has no GSUB or GPOS lookup that acts on a run of bare base
// letters: its `nukt` ligatures need ਼, and every other lookup needs a
// halant, a subjoined form, a vowel sign, addak, bindi or tippi in the run.
// So a word of bare letters is
// its letters' `cmap` outlines at their `hmtx` advances. Signs are not taken:
// none has a cited ductus yet.
//
// One small change of wording: a body that followed the letter's own headline
// starts "lift, then …". When such a body opens the WORD, nothing came before
// it to lift from, so the composed word's very first movement drops that
// "lift, then " (`LIFT_PREFIX`) and keeps the rest of the cited label.
// ---------------------------------------------------------------------------

import { ductusFor, type GlyphOutline } from "./ductusview.ts";
import { distanceToPath, fractionOnInk, inkPoints, makeInInk } from "./ink.ts";
import { penPath, type LetterDuctus, type Point, type Stroke, type StrokeSource } from "./strokes.ts";
import { boundsOf, contoursToPath, type Contour, type Font } from "./truetype.ts";
import devanagari from "../../../../learning/human-languages/data/scripts/devanagari.json";

/** The label every cited Devanagari letter's headline stroke carries, verbatim. */
export const LETTER_HEADLINE_LABEL = "lift, then draw the shirorekha rightward";

/**
 * The caption of a word's last movement. Short on purpose: a printed caption
 * wraps at about 23 characters, so "7. lift, then the" / "word's shirorekha"
 * (and "12. lift, then the" for a long word) stays on two lines.
 * "lift, then one shirorekha over the word" wrapped to three.
 */
export const WORD_HEADLINE_LABEL = "lift, then the word's shirorekha";

/** The label every cited Gurmukhi letter's own (first) headline stroke carries, verbatim. */
export const GURMUKHI_LETTER_HEADLINE_LABEL = "draw the headline from left to right";

/** What a body stroke that followed the letter's own headline opens with. */
export const LIFT_PREFIX = "lift, then ";

/** Where a script's cited letters draw their own headline, and what it is labelled. */
export interface LetterHeadlineRule {
  /** `last` for Devanagari (native writers' majority); `first` for Gurmukhi (GNPS's teaching order). */
  readonly at: "first" | "last";
  /** The headline stroke's one segment label, verbatim. */
  readonly label: string;
}

/**
 * Per script, which end of a cited letter its own headline stroke sits at.
 *
 *     script       a letter's own headline     a word's one headline
 *     ----------   -------------------------   ---------------------------
 *     devanagari   last  (LipiTk majority)     last
 *     gurmukhi     first (GNPS teaching)       last, by convention
 */
export const LETTER_HEADLINE: Readonly<Record<string, LetterHeadlineRule>> = {
  devanagari: { at: "last", label: LETTER_HEADLINE_LABEL },
  gurmukhi: { at: "first", label: GURMUKHI_LETTER_HEADLINE_LABEL },
};

/** Scripts whose words hang from one shared headline built by this module. */
export const HEADLINE_WORD_SCRIPTS: ReadonlySet<string> = new Set(["devanagari", "gurmukhi"]);

/**
 * Letters whose printed headline is SPLIT, per script: the bundled font leaves
 * a gap in the bar above the body, and the cited source draws no separate bar,
 * so the letter's own strokes draw its pieces of bar. Such a letter joins a
 * word whole, and the word's one headline never crosses its gap.
 *
 *     letter   Noto Sans Gurmukhi 2.004 bar at y 586 (font units)
 *     ਅ        -7..158   and  572..748
 *     ਖ        -7..159   and  428..604
 *     ਘ        -7..159,  343..421 and 587..763
 *     ਪ        -7..159   and  423..599
 *     ਮ        -7..143   and  426..602
 *
 * tests/headline-word-gurmukhi.test.ts measures the gap in the font for each and holds
 * that every OTHER cited Gurmukhi letter has a separate first headline stroke.
 */
export const SPLIT_HEADLINE_LETTERS: Readonly<Record<string, ReadonlySet<string>>> = {
  gurmukhi: new Set(["ਅ", "ਖ", "ਘ", "ਪ", "ਮ"]), // ਅ ਖ ਘ ਪ ਮ
};

/** The default ink tolerances every per-letter ductus is held to. */
export const MINIMUM_INK_FIT = 0.97;
export const MAXIMUM_UNTRACED = 0.02;
/** How far (font units) an ink sample may sit from every path before it counts as untraced. */
export const UNTRACED_DISTANCE = 100;

/**
 * How far apart (font units) two letters' headline heights may be and still
 * be one line. The cited letters sit at 585, except ऋ at 586.
 */
export const HEADLINE_HEIGHT_TOLERANCE = 2;

/** Roughly how far apart the points of the composed headline are, in font units. */
const HEADLINE_POINT_SPACING = 100;

/** The citation for drawing the headline last. Printed in every word strip's footer. */
export const HEADLINE_LAST_SOURCE: Readonly<StrokeSource> = Object.freeze({
  citation:
    "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Devanagari recognizer: " +
    "native writers' stored prototypes of the 33 consonants draw the headline as the last stroke " +
    "in 82% of 2,706 and as the first in about 5% (MIT licence, 2012)",
  url: "https://lipitk.sourceforge.net/lipi-reco.htm",
});

/**
 * What a Gurmukhi word's footer prints for its headline: a CONVENTION, named
 * as one. It points at the Devanagari counts because that is the evidence the
 * convention is carried over from, and says in the same breath that it is not
 * a Gurmukhi observation and that the cited letters draw the headline first.
 */
export const GURMUKHI_HEADLINE_LAST_SOURCE: Readonly<StrokeSource> = Object.freeze({
  citation:
    "by this book's convention for words, not from a Gurmukhi recording: the order fluent writers " +
    "are described as using, and the one this book draws Devanagari words in, after HP Labs India's " +
    "Lipi Toolkit 4.0 Devanagari pen data (headline last in 82% of 2,706 consonant prototypes); " +
    "GNPS's own letters draw the headline first",
  url: HEADLINE_LAST_SOURCE.url,
});

/** Per script: the footer's citation for the headline drawn last, and the note that explains it. */
const HEADLINE_LAST: Readonly<Record<string, { source: StrokeSource; note: string }>> = {
  devanagari: {
    source: HEADLINE_LAST_SOURCE,
    note:
      " The headline is " +
      "then drawn once, left to right, over the whole word. That is the order most native writers use " +
      "for a single letter: in HP Labs India's LipiTk 4.0 Devanagari recognizer the stored prototypes " +
      "of the 33 consonants draw the headline last in 82% of 2,706 and first in about 5%. It is a " +
      "majority, not a rule: some writers draw the headline first. The traces are single letters, so " +
      "carrying their order across a word is this book's reading of them; no reachable source records " +
      "native writers' headline timing in whole words.",
  },
  gurmukhi: {
    source: GURMUKHI_HEADLINE_LAST_SOURCE,
    note:
      " The headline is then drawn once, left to right, along the word's printed bar. GNPS's tracing " +
      "lesson draws every letter's headline FIRST, and each letter's own strip keeps that order; in a " +
      "word this book draws it LAST instead, by convention: the order fluent writers are described as " +
      "using, and the one it draws Devanagari words in, where native writers' consonant prototypes in " +
      "HP Labs India's LipiTk 4.0 Devanagari recognizer draw the headline last in 82% of 2,706. The " +
      "word-level order is not separately sourced: no reachable Gurmukhi recording shows a writer's " +
      "headline timing in a word. It is a convention, not a rule.",
  },
};

/**
 * The signs a word may hold, per script, each with the CITED place it is
 * written in: after its consonant's body and before the shared headline.
 *
 * The place is read from the sign's mark record (`compositionSource` in
 * data/scripts/devanagari.json), never restated here, so the strip's footer
 * and the data cannot drift apart. A sign listed here without that citation
 * is an authoring error, and importing this module throws.
 *
 *     sign   place in a word                         cited to
 *     ----   --------------------------------------  ------------------------------
 *     ा      consonant's body, stem, then headline   the cited आ (Saurmandal), whose
 *                                                    trailing stem is the same bar
 *
 * Gurmukhi has no entry: none of its vowel signs, bindi, tippi, addak or
 * halant has a cited ductus yet, so a Gurmukhi word is bare letters only.
 */
export const HEADLINE_WORD_SIGNS: Readonly<Record<string, ReadonlyMap<string, StrokeSource>>> = {
  devanagari: new Map(
    ["\u093E"].map((sign) => {
      const record = devanagari.marks.find((candidate) => candidate.mark === sign);
      const place = record !== undefined && "compositionSource" in record ? record.compositionSource : undefined;
      if (place === undefined) throw new Error(`Devanagari ${sign} has no cited place in a word`);
      return [sign, place];
    }),
  ),
};

/**
 * The letters a word's sign may follow: Devanagari's consonants, क (U+0915)
 * to ह (U+0939). After an independent vowel (आ is अ's own letter, not अ + ा)
 * the font prints a dotted circle, so ā there is not a word anyone writes.
 * Shaping every consonant + ā (and each of those between two letters) with
 * HarfBuzz gives exactly the `cmap` glyphs at their `hmtx` advances, so the
 * composed outline is the printed word.
 */
export const HEADLINE_WORD_CONSONANTS: Readonly<Record<string, RegExp>> = {
  devanagari: /^[\u0915-\u0939]$/u,
};

/** A letter taken apart: everything but its headline, and the headline itself. */
export interface HeadlineSplit {
  /** The letter's strokes without its headline, in writing order. */
  body: Stroke[];
  /** The headline stroke, as authored. */
  headline: Stroke;
  /** The headline's height, in font units. */
  y: number;
  /** Where the headline starts and ends, left to right, in font units. */
  x0: number;
  x1: number;
}

/**
 * Take a letter's headline stroke off, or `undefined` when it has none that
 * can be taken off on its own.
 *
 * Which end it is taken from is the script's (`LETTER_HEADLINE`): the LAST
 * stroke of a Devanagari letter, the FIRST of a Gurmukhi one. A script with
 * no rule is read like Devanagari. The headline must be one segment labelled
 * exactly as the rule says, every point at the same height, and travelled
 * strictly left to right; and something must be left once it is gone. Each
 * condition is a reason a stroke might not be "just the headline": a two-part
 * stroke joins the headline to some other movement, a sloped or backwards one
 * is not the line a word's headline continues.
 */
export function splitHeadline(
  letter: LetterDuctus,
  rule: LetterHeadlineRule = LETTER_HEADLINE[letter.script] ?? LETTER_HEADLINE.devanagari!,
): HeadlineSplit | undefined {
  if (letter.strokes.length < 2) return undefined;
  const index = rule.at === "last" ? letter.strokes.length - 1 : 0;
  const stroke = letter.strokes[index];
  if (stroke.segments.length !== 1) return undefined;
  const [segment] = stroke.segments;
  if (segment.label !== rule.label || segment.path.length < 2) return undefined;
  const y = segment.path[0].y;
  if (segment.path.some((point) => point.y !== y)) return undefined;
  for (let i = 1; i < segment.path.length; i++) {
    if (segment.path[i].x <= segment.path[i - 1].x) return undefined;
  }
  return {
    body: letter.strokes.filter((_, at) => at !== index),
    headline: stroke,
    y,
    x0: segment.path[0].x,
    x1: segment.path[segment.path.length - 1].x,
  };
}

/** A composed word, ready for the filmstrip ledger — or the reason it is refused. */
export type HeadlineWord =
  | { ok: true; ductus: LetterDuctus; outline: GlyphOutline }
  | { ok: false; reason: string };

/** One base letter: a single code point of category L (not Lm) that NFD leaves alone. */
const BASE_LETTER = /^(?!\p{Lm})\p{L}$/u;

const refuse = (reason: string): HeadlineWord => ({ ok: false, reason });

const shifted = (stroke: Stroke, dx: number): Stroke => ({
  segments: stroke.segments.map((segment) => ({
    label: segment.label,
    path: segment.path.map((point) => ({ x: point.x + dx, y: point.y })),
  })),
});

/** A sign that may sit in a word (ā), as opposed to a base letter. */
const SIGN = /^\p{M}$/u;

/**
 * "letter 1", "letters 1 and 2", "letters 1, 3 and 4" — or "sign 2",
 * "signs 2 and 4" for the signs. Positions count every character of the word
 * in reading order, so नाम is letter 1, sign 2, letter 3.
 */
function positions(numbers: readonly number[], noun: "letter" | "sign" = "letter"): string {
  if (numbers.length === 1) return `${noun} ${numbers[0]}`;
  return `${noun}s ${numbers.slice(0, -1).join(", ")} and ${numbers[numbers.length - 1]}`;
}

/**
 * The word's source: each letter's own citation, by position, then the
 * citation for the headline coming last. Letters that share a source share
 * one entry ("letters 1 and 2: …"), in the order they first appear.
 *
 * The printed footer reads "Stroke order after letters 1 and 2: <their
 * source>; the shared headline, drawn last: <HP Labs India>". The `variation`
 * (which the figure keeps in its `<desc>`, and which makes it print "This
 * order is attested, not standardised") says what the strip assumes.
 *
 * A word with a sign (नाम) names it as a sign ("sign 2: …"), and adds the
 * citation for its PLACE, from `HEADLINE_WORD_SIGNS`, before the headline's:
 * "…; the place of sign 2, after its consonant's body: <the cited आ>; the
 * shared headline, drawn last: …". A word of bare letters reads exactly as it
 * did before signs could join.
 *
 * The headline's citation and closing note are the SCRIPT's (`HEADLINE_LAST`):
 * a Devanagari word cites HP Labs India's counts; a Gurmukhi word names the
 * convention it follows (`GURMUKHI_HEADLINE_LAST_SOURCE`) and that its cited
 * letters draw the headline first. A Gurmukhi word with a split-bar letter
 * (ਪਰ) also says, by position, that the letter joins whole and that the
 * shared headline does not cross its gap.
 */
export function headlineWordSource(letters: readonly LetterDuctus[]): StrokeSource {
  const script = letters[0]?.script ?? "devanagari";
  const last = HEADLINE_LAST[script] ?? HEADLINE_LAST.devanagari!;
  const nounOf = (letter: LetterDuctus): "letter" | "sign" => (SIGN.test(letter.glyph) ? "sign" : "letter");
  const groups: Array<{ source: StrokeSource; noun: "letter" | "sign"; at: number[] }> = [];
  letters.forEach((letter, index) => {
    const group = groups.find(
      (candidate) =>
        candidate.source.citation === letter.source.citation &&
        candidate.source.url === letter.source.url &&
        candidate.noun === nounOf(letter),
    );
    if (group === undefined) groups.push({ source: letter.source, noun: nounOf(letter), at: [index + 1] });
    else group.at.push(index + 1);
  });
  // Where each sign is written, grouped the same way: one entry per cited place.
  const places: Array<{ source: StrokeSource; at: number[] }> = [];
  letters.forEach((letter, index) => {
    if (nounOf(letter) !== "sign") return;
    const source = HEADLINE_WORD_SIGNS[letter.script]?.get(letter.glyph);
    if (source === undefined) throw new Error(`${letter.glyph} has no cited place in a word`);
    const place = places.find((candidate) => candidate.source === source);
    if (place === undefined) places.push({ source, at: [index + 1] });
    else place.at.push(index + 1);
  });
  const placeOf = (at: readonly number[]): string =>
    at.length === 1
      ? `the place of ${positions(at, "sign")}, after its consonant's body`
      : `the place of ${positions(at, "sign")}, each after its consonant's body`;
  const citation =
    groups.map((group) => `${positions(group.at, group.noun)}: ${group.source.citation}`).join("; ") +
    places.map((place) => `; ${placeOf(place.at)}: ${place.source.citation}`).join("") +
    `; the shared headline, drawn last: ${last.source.citation}`;
  const signNotes = places
    .map(
      (place) =>
        ` ${placeOf(place.at).replace(/^the/, "The")} and before the headline, after ` +
        `${place.source.citation} <${place.source.url}>; the sign's own piece of headline becomes ` +
        `part of the word's one headline. ${place.source.variation ?? ""}`,
    )
    .join("");
  // Letters whose printed bar is split join whole (`SPLIT_HEADLINE_LETTERS`):
  // say so, by position, so the reader knows why their bar is drawn early.
  const split = letters.flatMap((letter, index) =>
    SPLIT_HEADLINE_LETTERS[letter.script]?.has(letter.glyph) ? [index + 1] : [],
  );
  const splitNote =
    split.length === 0
      ? ""
      : ` ${positions(split).replace(/^l/, "L")} ${split.length === 1 ? "prints its" : "print their"} ` +
        "headline split, with a gap above the body, and the cited source draws no separate bar " +
        `for ${split.length === 1 ? "it" : "them"}: ${split.length === 1 ? "its" : "their"} own strokes, ` +
        "kept whole, draw the pieces of bar, and the shared headline does not cross the gap. It runs " +
        "from the first to the last letter whose own headline stroke was taken off.";
  const variation =
    "The letters are drawn in reading order, each as its own cited strip draws it but without " +
    "its own headline, placed where the bundled font prints it in the word: " +
    groups
      .map((group) => `${positions(group.at, group.noun)} after ${group.source.citation} <${group.source.url}>`)
      .join("; ") +
    ". Each letter's own note on variation is printed with that letter's own strip." +
    signNotes +
    splitNote +
    last.note;
  return { citation, url: last.source.url, variation };
}

/**
 * Compose `word` for `script`: its letters' bodies in reading order, then one
 * headline over the whole word, fitted to the printed word in `font`. A
 * letter in `SPLIT_HEADLINE_LETTERS` joins whole, and the headline runs from
 * the first to the last letter whose own headline was taken off.
 *
 * `lookup` is the cited ductus registry (`ductusFor`); tests pass their own
 * to build letters the registry does not hold.
 */
export function composeHeadlineWord(
  word: string,
  script: string,
  font: Font,
  lookup: (glyph: string, script: string) => LetterDuctus | undefined = ductusFor,
): HeadlineWord {
  if (!HEADLINE_WORD_SCRIPTS.has(script)) {
    return refuse(`${script} words are not composed with a shared headline`);
  }
  const characters = [...word];
  if (characters.length < 2) return refuse("a word needs two or more characters");

  // 1. Every character a cited base letter with a headline of its own — or
  //    a sign with a cited place in a word (ā), straight after a consonant.
  //    Unicode stores ā after its consonant, and it is written after the
  //    consonant's body, so reading order is writing order here.
  //    A letter whose printed bar is split (`SPLIT_HEADLINE_LETTERS`) has no
  //    headline to take off: it joins whole, and adds nothing to the span of
  //    the shared headline (`bar` is undefined).
  const letters: LetterDuctus[] = [];
  const parts: Array<{ body: Stroke[]; bar?: HeadlineSplit }> = [];
  let height: number | undefined;
  const signs = HEADLINE_WORD_SIGNS[script];
  const consonant = HEADLINE_WORD_CONSONANTS[script];
  const splitBar = SPLIT_HEADLINE_LETTERS[script];
  for (let index = 0; index < characters.length; index++) {
    const character = characters[index];
    if (signs?.has(character)) {
      const before = characters[index - 1];
      if (before === undefined || consonant === undefined || !consonant.test(before)) {
        return refuse(`${character} must follow a consonant`);
      }
    } else if (!BASE_LETTER.test(character) || character.normalize("NFD") !== character) {
      return refuse(`${character} is neither a single base letter nor a sign with a cited place in a word`);
    }
    const letter = lookup(character, script);
    if (letter === undefined) return refuse(`${character} has no cited ductus`);
    letters.push(letter);
    if (splitBar?.has(character)) {
      parts.push({ body: letter.strokes });
      continue;
    }
    const split = splitHeadline(letter, LETTER_HEADLINE[script]);
    if (split === undefined) return refuse(`${character} has no separate headline stroke`);
    if (Math.abs(split.y - (height ?? split.y)) > HEADLINE_HEIGHT_TOLERANCE) {
      return refuse(`${character}'s headline is not at the height of the first letter's`);
    }
    height ??= split.y;
    parts.push({ body: split.body, bar: split });
  }
  if (height === undefined) return refuse("no letter of the word has a headline stroke of its own to share");

  // 2. Lay the letters out the way the font prints them: each starts where the
  //    advances before it end. Body strokes and outlines move together.
  const strokes: Stroke[] = [];
  const contours: Contour[] = [];
  let x0 = Infinity;
  let x1 = -Infinity;
  let dx = 0;
  for (let i = 0; i < characters.length; i++) {
    const character = characters[i];
    const glyph = font.glyphFor(character);
    const advance = font.advanceFor(character);
    if (glyph === undefined || advance === undefined) {
      return refuse(`the font has no outline or advance for ${character}`);
    }
    strokes.push(...parts[i].body.map((stroke) => shifted(stroke, dx)));
    for (const contour of glyph.contours) {
      contours.push(contour.map((point) => ({ x: point.x + dx, y: point.y, on: point.on })));
    }
    const bar = parts[i].bar;
    if (bar !== undefined) {
      x0 = Math.min(x0, dx + bar.x0);
      x1 = Math.max(x1, dx + bar.x1);
    }
    dx += advance;
  }

  // The word's first movement had nothing before it to lift from: a body
  // that followed its letter's own headline (Gurmukhi's "lift, then come down
  // the stem") opens the word without the "lift, then ".
  const opening = strokes[0]?.segments[0];
  if (opening !== undefined && opening.label.startsWith(LIFT_PREFIX) && opening.label.length > LIFT_PREFIX.length) {
    strokes[0] = {
      segments: [{ ...opening, label: opening.label.slice(LIFT_PREFIX.length) }, ...strokes[0].segments.slice(1)],
    };
  }

  // 3. The one headline, left to right, at the first headline's height.
  const y = height;
  const steps = Math.max(2, Math.ceil((x1 - x0) / HEADLINE_POINT_SPACING));
  const path: Point[] = [];
  for (let s = 0; s <= steps; s++) path.push({ x: Math.round(x0 + ((x1 - x0) * s) / steps), y });
  strokes.push({ segments: [{ label: WORD_HEADLINE_LABEL, path }] });

  // 4. Check the composed path against the printed word, as every letter is
  //    checked against its own glyph.
  const inInk = makeInInk(contours);
  for (let s = 0; s < strokes.length; s++) {
    const fit = fractionOnInk(penPath(strokes[s]), inInk);
    if (!(fit > MINIMUM_INK_FIT)) {
      const which = s === strokes.length - 1 ? "the shared headline" : `stroke ${s + 1}`;
      return refuse(
        `${which} is only ${(fit * 100).toFixed(1)}% on the printed word's ink ` +
          `(the default tolerance is over ${MINIMUM_INK_FIT * 100}%)`,
      );
    }
  }
  const samples = inkPoints(contours);
  const paths = strokes.map((stroke) => penPath(stroke));
  const untraced = samples.filter(([x, py]) =>
    paths.every((candidate) => distanceToPath(x, py, candidate) > UNTRACED_DISTANCE),
  ).length / Math.max(1, samples.length);
  if (!(untraced < MAXIMUM_UNTRACED)) {
    return refuse(
      `${(untraced * 100).toFixed(1)}% of the printed word is never traced ` +
        `(the default tolerance is under ${MAXIMUM_UNTRACED * 100}%)`,
    );
  }

  return {
    ok: true,
    ductus: { script, glyph: word, strokes, source: headlineWordSource(letters) },
    outline: { path: contoursToPath(contours), bounds: boundsOf(contours) },
  };
}

// ---------------------------------------------------------------------------
// A phrase: words separated by single spaces, each composed on its own
// ---------------------------------------------------------------------------
//
//     "मम नाम"   ->   word 1: म's body, म's body, then मम's headline
//                     word 2: न's body, ā's stem, म's body, then नाम's headline
//
// Each word keeps its OWN headline: the space between words breaks the line,
// exactly as the printed phrase shows it. So a phrase is not one ledger entry
// but one per word, printed as the groups of a sequence strip ("Word 1 of 2"),
// each word at its own scale. A word of ONE letter is that letter's own strip,
// which already draws its body and then its own headline.
//
// The separator is a single space and nothing else. "नाम: मीरा" is a label
// whose colon no source draws, and "नमस्कार मीरा." ends in a full stop: each
// is refused here, because a character with no cited ductus refuses its word.

/** One word of a composed phrase, ready for the filmstrip ledger. */
export interface PhraseWord {
  /** The word, which is also its ledger key (`devanagari:नाम`). */
  glyph: string;
  ductus: LetterDuctus;
  outline: GlyphOutline;
}

/** A composed phrase, word by word — or the reason it is refused. */
export type HeadlinePhrase = { ok: true; words: PhraseWord[] } | { ok: false; reason: string };

/**
 * Compose every word of `phrase`, or refuse the whole phrase with the first
 * word that fails and why ("word 2 (नामः): …").
 */
export function composeHeadlinePhrase(
  phrase: string,
  script: string,
  font: Font,
  lookup: (glyph: string, script: string) => LetterDuctus | undefined = ductusFor,
): HeadlinePhrase {
  const no = (reason: string): HeadlinePhrase => ({ ok: false, reason });
  if (!HEADLINE_WORD_SCRIPTS.has(script)) return no(`${script} words are not composed with a shared headline`);
  const words = phrase.split(" ");
  if (words.length < 2) return no("a phrase needs two or more words");
  if (words.some((word) => word === "" || /\s/u.test(word))) {
    return no("a phrase's words are separated by single spaces and nothing else");
  }
  const composed: PhraseWord[] = [];
  for (const [index, word] of words.entries()) {
    const which = `word ${index + 1} (${word})`;
    if ([...word].length === 1) {
      // A one-letter word: the letter's own cited strip, headline and all.
      const letter = BASE_LETTER.test(word) && word.normalize("NFD") === word ? lookup(word, script) : undefined;
      const glyph = font.glyphFor(word);
      if (letter === undefined || glyph === undefined) return no(`${which}: ${word} is not a cited base letter`);
      composed.push({ glyph: word, ductus: letter, outline: { path: glyph.path, bounds: boundsOf(glyph.contours) } });
      continue;
    }
    const result = composeHeadlineWord(word, script, font, lookup);
    if (!result.ok) return no(`${which}: ${result.reason}`);
    composed.push({ glyph: word, ductus: result.ductus, outline: result.outline });
  }
  return { ok: true, words: composed };
}
