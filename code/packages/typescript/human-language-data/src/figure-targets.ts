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
 * whether it becomes a sequence strip. "வணக்கம்" does not yet: it carries a
 * pulli, and a mark is not drawn in a sequence until its written order is
 * modelled.
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
 * A word may only be drawn letter by letter when every grapheme is ONE base
 * letter: a single code point of category L that is not a modifier (Lm, like
 * ー). That leaves out every vowel sign, virama and nasal mark, and it does so
 * on purpose — some of those are WRITTEN before the consonant they follow in
 * Unicode (Tamil ெ, Devanagari ि), so drawing a word in code-point order would
 * put strokes in the wrong order once marks have a ductus. Until a design that
 * knows the written order exists, a word with a mark is not a sequence.
 */
const BASE_LETTER = /^(?!\p{Lm})\p{L}$/u;

/**
 * The letters a writing lesson's headword spells out, in writing order, or
 * `undefined` when it is not a sequence this module will draw.
 *
 * A sequence is
 *
 *   * a LIST of two or more items, each exactly one grapheme, in any script; or
 *   * one or more WORDS in a `SEPARATE_LETTER_SCRIPTS` script, two or more
 *     letters in all, where every grapheme is a single base letter.
 *
 * A one-letter headword is not a sequence; `writingLetterOf` owns it. Like a
 * single letter, a sequence needs a Writing or Script block to land in. This
 * function only reads the headword; whether each letter has a CITED ductus is
 * `withDerivedFilmstrips`' question, answered by the ledger.
 */
export function writingSequenceOf(lesson: ParsedLesson, script: string): string[] | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  if (letterBlockIndex(lesson) === -1) return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  const items = headword.split(LIST_SEPARATORS).filter((item) => item !== "");
  const graphemes = items.map((item) => [...GRAPHEMES.segment(item)].map((part) => part.segment));
  const letters = graphemes.flat();
  if (letters.length < 2) return undefined;
  if (items.length >= 2 && graphemes.every((item) => item.length === 1)) return letters;
  if (!SEPARATE_LETTER_SCRIPTS.has(script)) return undefined;
  return letters.every((letter) => BASE_LETTER.test(letter)) ? letters : undefined;
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
    const glyph = writingLetterOf(lesson);
    if (glyph !== undefined) {
      candidates.push({ kind: "script-filmstrip", lessonId, script, glyph, output });
      continue;
    }
    const letters = writingSequenceOf(lesson, script);
    if (letters === undefined) continue;
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
 * the separators are gone.
 */
export function filmstripImageMarkdown(target: ScriptFilmstripTarget): string {
  const file = `figures/${basename(target.output)}`;
  if (target.letters === undefined) return `![How ${target.glyph} is written, stroke by stroke](${file})`;
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
