// ---------------------------------------------------------------------------
// letter-anchoring.ts — is every letter written from a word the reader knows?
// ---------------------------------------------------------------------------
//
// HL-C443's rule for gentle writing, in the words it was asked for:
//
//   "You can teach letters from words that the reader has already learned and
//    at some point the reader would have learned all the letters."
//
// That is two claims, and this module measures each one per track.
//
//   1. ANCHORING. A letter lesson (one letter, written stroke by stroke) should
//      come AFTER a word lesson whose headword holds that letter. The reader
//      then meets the shape as a piece of something they can already say, and
//      writing it is taking a known word apart. A letter taught cold is a shape
//      with nothing to hang it on.
//
//   2. COMPLETENESS. Every letter the reader reads in a word headword should,
//      at some point, get its own letter lesson. Otherwise the reader can read
//      the letter but was never shown how to make it.
//
// ---------------------------------------------------------------------------
// Three kinds of letter lesson
// ---------------------------------------------------------------------------
//
// Walking a track in reading order, each letter lesson is exactly one of:
//
//   anchored        every glyph of the letter appeared in an EARLIER word
//                   headword.  This is the rule.
//   builds-toward   not anchored, but a word LATER IN THE SAME CHAPTER holds
//                   it.  Marwadi opens this way: र, then ा, then the word राम.
//                   The word is close, but the letter still comes first.
//   cold            neither.  No word the reader knows, or is about to meet in
//                   this chapter, holds the letter.
//
// "builds-toward" is split from "cold" because the two need different fixes.
// Builds-toward is usually a resequencing inside one chapter. Cold needs a
// word authored first.
//
// ---------------------------------------------------------------------------
// What this does not measure yet: Han components
// ---------------------------------------------------------------------------
//
// A Chinese track teaches 人 and 亻 on the way to 你. That IS anchoring: the
// component is part of a character the reader knows. But you cannot see it in
// code points. 你 is one code point, and 亻 is not inside it. Telling whether a
// component belongs to a known character needs decomposition data this package
// does not hold. So a letter lesson whose unanchored glyphs are all Han is
// counted as `unmeasured`, never as cold. Reporting it as cold would blame the
// tracks for a gap in the measure itself.
//
// ---------------------------------------------------------------------------
// What counts as a word and as a letter lesson
// ---------------------------------------------------------------------------
//
// A letter lesson is exactly what gets a stroke-order filmstrip:
// `writingLetterOf` from figure-targets.ts. That means `type: writing`, a
// one-grapheme headword, and a Writing or Script block. A LETTER SET counts
// too: a writing lesson whose headword lists single letters, like "வ, க" or
// "௧ ௨ ௩". It writes a few letters side by side rather than one, and each of
// its letters counts as written. Other writing lessons (a whole word copied, a
// dictation) are neither letters nor words here. They practise writing, but
// they do not make a word "known" to read.
//
// A word is the headword of every lesson that is not `type: writing`. Only the
// headword counts. It is the one thing the lesson guarantees the reader
// learned. Glyphs that only appear in body text are the business of
// script-closure.ts.
//
// letter-ledger.ts asks a neighbouring question: does a script's AUTHORED
// letter order (data/scripts/<script>-ledger.json) still match the corpus? This
// module needs no ledger. It reads the order the lessons actually run in, so it
// measures every track, including the ones that have no ledger yet.
//
// Latin-script tracks are skipped, as in script-closure.ts: their reader
// arrives already able to write the alphabet.
//
// Report-only. The corpus test pins a per-track ceiling, which is a ratchet: a
// number may fall, and must not rise.

import { letterBlockIndex, writingLetterOf } from "./figure-targets.js";
import { hasOwn } from "./constants.js";
import type { ParsedLesson } from "./parse.js";
import { SCRIPT_SYSTEMS, belongsToAny, readingOrder, systemOf } from "./ramp.js";

export type LetterAnchoring = "anchored" | "builds-toward" | "cold" | "unmeasured";

/** One letter lesson and what, if anything, it was anchored in. */
export interface LetterLessonAnchor {
  lessonId: string;
  /** The letter, or a letter set's letters joined by single spaces. */
  letter: string;
  chapter: number | null;
  anchoring: LetterAnchoring;
}

/** One track's writing ramp. */
export interface TrackLetterAnchoring {
  language: string;
  script: string;
  /** Every letter lesson, in reading order. */
  letterLessons: LetterLessonAnchor[];
  anchored: number;
  buildsToward: number;
  cold: number;
  unmeasured: number;
  /** Distinct glyphs any word headword shows. */
  lettersRead: number;
  /** Distinct glyphs any letter lesson writes. */
  lettersWritten: number;
  /**
   * Glyphs read in a word headword but never given a letter lesson, sorted.
   * This is the completeness debt: "at some point the reader would have
   * learned all the letters" is true exactly when this is empty.
   */
  unwritten: string[];
}

export interface LetterAnchoringReport {
  tracks: TrackLetterAnchoring[];
  summary: {
    tracks: number;
    letterLessons: number;
    anchored: number;
    buildsToward: number;
    cold: number;
    unmeasured: number;
    unwritten: number;
  };
}

const GRAPHEMES = new Intl.Segmenter("und", { granularity: "grapheme" });

/**
 * The letters a writing lesson writes one at a time: its single letter, or
 * every letter of a letter set. `undefined` for any other lesson.
 */
export function writtenLettersOf(lesson: ParsedLesson): string[] | undefined {
  const letter = writingLetterOf(lesson);
  if (letter !== undefined) return [letter];
  if (lesson.realization.type !== "writing" || letterBlockIndex(lesson) === -1) return undefined;
  const pieces = (lesson.realization.headword ?? "").split(/[\s,\u00B7\u060C\u3001]+/u).filter(Boolean);
  if (pieces.length < 2) return undefined;
  if (pieces.some((piece) => [...GRAPHEMES.segment(piece)].length !== 1)) return undefined;
  return pieces;
}

function chapterOf(lesson: ParsedLesson): number | null {
  const chapter = lesson.realization.chapter;
  return typeof chapter === "number" && Number.isFinite(chapter) ? chapter : null;
}

/** The target-script glyphs in a string, in order, deduplicated. */
function glyphsIn(text: string, target: ReadonlySet<string>): string[] {
  return [...new Set([...text].filter((ch) => belongsToAny(ch, target)))];
}

/** Measure both halves of the rule for every non-Latin track. */
export function measureLetterAnchoring(lessons: readonly ParsedLesson[]): LetterAnchoringReport {
  const byTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    let group = byTrack.get(lesson.language);
    if (!group) byTrack.set(lesson.language, (group = []));
    group.push(lesson);
  }

  const tracks: TrackLetterAnchoring[] = [];
  for (const [language, group] of [...byTrack].sort((a, b) => a[0].localeCompare(b[0]))) {
    const script = group[0]!.script;
    // An unknown script is script-closure.ts's finding to report. Here it has
    // no glyph set to measure against, so it is skipped rather than zeroed.
    if (!hasOwn(SCRIPT_SYSTEMS, script)) continue;
    const target = new Set(SCRIPT_SYSTEMS[script]!);
    if (target.has("Latin")) continue;

    const ordered = [...group].sort(readingOrder);

    // Every word headword's glyphs by chapter, for the builds-toward check.
    const wordGlyphsByChapter = new Map<number | null, Set<string>>();
    for (const lesson of ordered) {
      if (lesson.realization.type === "writing") continue;
      const chapter = chapterOf(lesson);
      let set = wordGlyphsByChapter.get(chapter);
      if (!set) wordGlyphsByChapter.set(chapter, (set = new Set()));
      for (const ch of glyphsIn(lesson.realization.headword ?? "", target)) set.add(ch);
    }

    const read = new Set<string>();
    const written = new Set<string>();
    const letterLessons: LetterLessonAnchor[] = [];
    for (const lesson of ordered) {
      const letters = writtenLettersOf(lesson);
      if (letters !== undefined) {
        const letter = letters.join(" ");
        const glyphs = glyphsIn(letters.join(""), target);
        const missing = glyphs.filter((ch) => !read.has(ch));
        const chapter = chapterOf(lesson);
        const sameChapter = wordGlyphsByChapter.get(chapter) ?? new Set<string>();
        let anchoring: LetterAnchoring;
        if (missing.length === 0) anchoring = "anchored";
        else if (missing.every((ch) => sameChapter.has(ch))) anchoring = "builds-toward";
        else if (missing.every((ch) => systemOf(ch) === "Han")) anchoring = "unmeasured";
        else anchoring = "cold";
        letterLessons.push({ lessonId: lesson.realization.lessonId, letter, chapter, anchoring });
        for (const ch of glyphs) written.add(ch);
        continue;
      }
      if (lesson.realization.type === "writing") continue;
      for (const ch of glyphsIn(lesson.realization.headword ?? "", target)) read.add(ch);
    }

    const count = (kind: LetterAnchoring) =>
      letterLessons.filter((entry) => entry.anchoring === kind).length;
    tracks.push({
      language,
      script,
      letterLessons,
      anchored: count("anchored"),
      buildsToward: count("builds-toward"),
      cold: count("cold"),
      unmeasured: count("unmeasured"),
      lettersRead: read.size,
      lettersWritten: written.size,
      unwritten: [...read].filter((ch) => !written.has(ch)).sort(),
    });
  }

  const total = (pick: (track: TrackLetterAnchoring) => number) =>
    tracks.reduce((sum, track) => sum + pick(track), 0);
  return {
    tracks,
    summary: {
      tracks: tracks.length,
      letterLessons: total((track) => track.letterLessons.length),
      anchored: total((track) => track.anchored),
      buildsToward: total((track) => track.buildsToward),
      cold: total((track) => track.cold),
      unmeasured: total((track) => track.unmeasured),
      unwritten: total((track) => track.unwritten.length),
    },
  };
}
