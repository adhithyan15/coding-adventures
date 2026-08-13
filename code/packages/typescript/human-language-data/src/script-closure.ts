// ---------------------------------------------------------------------------
// script-closure.ts — was the reader ever taught the letters they are shown?
// ---------------------------------------------------------------------------
//
// HL08's `measureScriptRamp` counts how many NEW target-script glyphs a lesson
// puts on the page and caps it at three. That budget is real and it caught real
// spikes. But it is a budget on PACE, and pace is not the thing that was wrong
// with the six Indic tracks.
//
// A track can satisfy it perfectly while teaching no letters at all. Four of the
// six do exactly that: Telugu, Kannada, Malayalam and Sanskrit have no writing
// lessons whatsoever, so every glyph they show is a glyph nobody was taught, and
// the glyph budget reports them as gentle.
//
// This module asks the question the budget cannot: **for each glyph the reader
// is asked to read, had an earlier lesson taught it?**
//
// Exposure, and why it is not a loophole
// --------------------------------------
// HL11's rule is closure on LOAD-BEARING script only. A word the reader is
// merely SHOWN — the headword printed beside its romanization, so the eye starts
// to recognise a shape long before the hand can make it — is exposure. It is
// counted, reported, and never required.
//
// That distinction is what lets the Tamil course open on வணக்கம் while still
// guaranteeing the reader is never asked to decode something untaught. It is
// also honest about what page one actually is: the reader is being shown a word,
// the way a child sees a shop sign years before reading it.
//
// The distinction has to be MECHANICAL or it is worthless, so it is drawn at the
// one place the corpus already records the answer:
//
//   a lesson's HEADWORD is exposure when the lesson declares a `romanization`.
//
// A romanization is the promise that the reader can use this word without
// reading it. Where the promise is made, the headword is exposure; where it is
// not, the headword is something the reader has to decode, and closure applies.
// Everything else in the body is load-bearing either way.
//
// That rule also names its own remediation, which is why it is the right one:
// adding a `romanization` to a lesson converts its headword from load-bearing to
// exposure, and that is a real improvement to the lesson rather than a way of
// hiding from the measurement. The learner genuinely gains something.
//
// What counts as teaching a glyph
// -------------------------------
// A SCRIPT LESSON — `type: writing`, or `delivery: script` — teaches every
// target-script glyph in its body. That is coarser than naming each letter in
// frontmatter, and deliberately so for a first measurement: it credits the
// corpus with everything it could plausibly be teaching, so the debt this
// reports is a LOWER BOUND on the real debt. A number that flatters the corpus
// and is still large is harder to argue with than one that does not.
//
// Report-only, per the HL05 and HL08 precedent.

import type { ParsedLesson } from "./parse.js";
import { SCRIPT_SYSTEMS, belongsToAny, readingOrder } from "./ramp.js";
import { hasOwn } from "./constants.js";

/** One lesson asking the reader to decode glyphs nobody taught them. */
export interface ClosureViolation {
  lessonId: string;
  language: string;
  chapter: number | null;
  /** The untaught glyphs, in sorted order. */
  glyphs: string;
  /** How many. Sorting on this makes the list a work queue. */
  count: number;
}

/** What one track's closure looks like. */
export interface TrackClosure {
  language: string;
  script: string;
  lessonCount: number;
  /** Lessons that teach letters: `type: writing` or `delivery: script`. */
  scriptLessons: number;
  /** Distinct glyphs any script lesson teaches. */
  taughtGlyphs: number;
  /** Distinct glyphs the track shows anywhere. */
  shownGlyphs: number;
  /** Shown but never taught, anywhere in the track. */
  neverTaughtGlyphs: number;
  /** Lessons asking for an untaught glyph in load-bearing text. */
  violations: number;
  /**
   * Lessons whose only untaught glyphs sit in an exempt headword.
   *
   * These are not violations. They are the count of places the exposure rule is
   * doing work, which is worth seeing beside the violations so the rule cannot
   * quietly become the reason the number looks good.
   */
  exposureOnly: number;
  /**
   * GLYPHS the exposure rule removed from a lesson's load-bearing set.
   *
   * `exposureOnly` counts lessons the rule flipped to clean; this counts what it
   * actually took out, including from lessons that still violate. The second
   * number is much larger than the first, and it is the one that would move if
   * an author started laundering script through the headword.
   */
  exposureExemptedGlyphs: number;
  /**
   * Lessons showing a headword in target script with NO romanization declared.
   *
   * The remediation queue: each one is a lesson whose headword would become
   * exposure the moment somebody writes down how to say it.
   */
  headwordsWithoutRomanization: number;
}

export interface ScriptClosureReport {
  tracks: TrackClosure[];
  /** Every violation, steepest first, as a work queue. */
  violations: ClosureViolation[];
  /**
   * Tracks whose declared script is not one this module knows.
   *
   * Named rather than counted, because the fix is per-track. These are NOT
   * reported as clean: they are reported as unmeasured, which is a different
   * claim and the only honest one.
   */
  unknownScriptTracks: string[];
  summary: {
    tracksWithScript: number;
    tracksTeachingNothing: number;
    violations: number;
    exposureOnly: number;
    exposureExemptedGlyphs: number;
    headwordsWithoutRomanization: number;
    /** Tracks skipped because their declared script is not one we know. */
    tracksWithUnknownScript: number;
  };
}

/** Does this lesson teach letters, rather than merely use them? */
function isScriptLesson(lesson: ParsedLesson): boolean {
  if (lesson.realization.type === "writing") return true;
  const delivery = lesson.frontmatter["delivery"];
  return typeof delivery === "string" && delivery.trim() === "script";
}

/**
 * Measure closure: glyphs asked for against glyphs taught, in reading order.
 *
 * Latin-script tracks are skipped entirely. Their reader arrives already knowing
 * the alphabet, which is the whole reason this module exists for the others and
 * not for Spanish.
 */
export function measureScriptClosure(lessons: ParsedLesson[]): ScriptClosureReport {
  const tracks: TrackClosure[] = [];
  const violations: ClosureViolation[] = [];
  const unknownScriptTracks: string[] = [];

  const byTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    let group = byTrack.get(lesson.language);
    if (!group) byTrack.set(lesson.language, (group = []));
    group.push(lesson);
  }

  for (const [language, group] of [...byTrack].sort((a, b) => a[0].localeCompare(b[0]))) {
    const script = group[0]!.script;
    // `hasOwn`, not `??`: an unknown script name reaching Object.prototype is
    // not nullish, so the fallback never fires and the Set constructor throws.
    // The same trap `measureScriptRamp` already documents.
    const known = hasOwn(SCRIPT_SYSTEMS, script);

    // "Genuinely Latin" and "we do not recognise this script" must not look the
    // same. Both used to `continue`, so a track with a mistyped or unregistered
    // script simply vanished from the report -- its lessons uncounted, its debt
    // unreported, and nothing anywhere saying so. That is the silent zero this
    // module exists to prevent, reached through the module itself.
    //
    // This is not hypothetical. `constants.ts` records that exact bug already
    // having shipped once, for Gujarati: an unknown script read as having no
    // script to learn.
    if (!known) {
      unknownScriptTracks.push(language);
      continue;
    }

    const target = new Set(SCRIPT_SYSTEMS[script]!);
    // A Latin-script track is skipped on purpose: its reader arrives already
    // knowing the alphabet, which is the whole reason this exists for the rest.
    if (target.has("Latin")) continue;

    const ordered = [...group].sort(readingOrder);
    const taught = new Set<string>();
    const shown = new Set<string>();
    const track: TrackClosure = {
      language,
      script,
      lessonCount: ordered.length,
      scriptLessons: 0,
      taughtGlyphs: 0,
      shownGlyphs: 0,
      neverTaughtGlyphs: 0,
      violations: 0,
      exposureOnly: 0,
      exposureExemptedGlyphs: 0,
      headwordsWithoutRomanization: 0,
    };

    for (const lesson of ordered) {
      const teaching = isScriptLesson(lesson);
      if (teaching) track.scriptLessons += 1;

      const headword = lesson.realization.headword ?? "";
      const romanization = (lesson.realization.romanization ?? "").trim();
      // The exemption, and the one place the whole measurement turns.
      const headwordIsExposure = romanization.length > 0;

      const headwordGlyphs = new Set<string>();
      for (const ch of headword) {
        if (belongsToAny(ch, target)) headwordGlyphs.add(ch);
      }
      if (headwordGlyphs.size > 0 && !headwordIsExposure) {
        track.headwordsWithoutRomanization += 1;
      }

      const bodyGlyphs = new Set<string>();
      for (const ch of new Set(lesson.body)) {
        if (belongsToAny(ch, target)) bodyGlyphs.add(ch);
      }
      for (const ch of headwordGlyphs) shown.add(ch);
      for (const ch of bodyGlyphs) shown.add(ch);

      // A script lesson is where letters come FROM, so it cannot be in debt to
      // itself. Its glyphs become taught for everything after it.
      if (teaching) {
        for (const ch of bodyGlyphs) taught.add(ch);
        for (const ch of headwordGlyphs) taught.add(ch);
        continue;
      }

      // Load-bearing = everything the lesson puts in front of the reader, MINUS
      // the headword when the headword is exempt.
      //
      // The headword has to be seeded in, not just subtracted out. An earlier
      // version built this set from the body alone, which silently dropped the
      // debt of any lesson whose headword glyphs do not also appear verbatim in
      // its body -- and then, worse, counted that lesson as clean BECAUSE of an
      // exemption it had never claimed.
      const loadBearing = new Set(bodyGlyphs);
      if (headwordIsExposure) {
        for (const ch of headwordGlyphs) loadBearing.delete(ch);
      } else {
        for (const ch of headwordGlyphs) loadBearing.add(ch);
      }

      // What the exemption actually removed, in glyphs rather than in lessons.
      //
      // `exposureOnly` counts lessons the exemption FLIPPED from violating to
      // clean, and that turns out to be a small number sitting on top of a much
      // larger one: the exemption also shaves glyphs off lessons that violate
      // anyway, and those are invisible in a per-lesson count. A lesson
      // reporting five untaught glyphs while fifteen more were exempted is not
      // a lesson with five problems. Counting the glyphs is what makes the
      // exemption's real size visible -- and it is the number that matters once
      // 931 becomes a burn-down target, because moving text into the headword
      // is the cheapest way to make the count fall without improving anything.
      if (headwordIsExposure) {
        // The whole untaught headword set, not just its intersection with the
        // body. Under the symmetric construction above, a non-exempt headword is
        // ADDED to the load-bearing set -- so what the exemption removes is
        // everything in that set, including glyphs that appear nowhere else in
        // the lesson. Guarding on `bodyGlyphs` would suppress exactly the case
        // whose omission was the bug fixed directly above it.
        for (const ch of headwordGlyphs) {
          if (!taught.has(ch)) track.exposureExemptedGlyphs += 1;
        }
      }

      const untaughtLoadBearing = [...loadBearing].filter((ch) => !taught.has(ch)).sort();
      const untaughtAnywhere = [...bodyGlyphs, ...headwordGlyphs]
        .filter((ch) => !taught.has(ch));

      if (untaughtLoadBearing.length > 0) {
        track.violations += 1;
        violations.push({
          lessonId: lesson.realization.lessonId,
          language,
          chapter:
            typeof lesson.realization.chapter === "number" &&
            Number.isFinite(lesson.realization.chapter)
              ? lesson.realization.chapter
              : null,
          glyphs: untaughtLoadBearing.join(""),
          count: untaughtLoadBearing.length,
        });
      } else if (headwordIsExposure && untaughtAnywhere.length > 0) {
        // Untaught glyphs, every one of them behind the exposure rule. Gated on
        // the exemption having actually been claimed: a lesson with no
        // romanization has nothing to be exempt BY, and counting it here read as
        // "the exposure rule saved this lesson" when the rule never applied.
        track.exposureOnly += 1;
      }
    }

    track.taughtGlyphs = taught.size;
    track.shownGlyphs = shown.size;
    track.neverTaughtGlyphs = [...shown].filter((ch) => !taught.has(ch)).length;
    tracks.push(track);
  }

  // Steepest first, then by id, so the list is a stable work queue.
  violations.sort((a, b) => b.count - a.count || a.lessonId.localeCompare(b.lessonId));

  return {
    tracks,
    violations,
    summary: {
      tracksWithScript: tracks.length,
      tracksTeachingNothing: tracks.filter((t) => t.scriptLessons === 0).length,
      violations: violations.length,
      exposureOnly: tracks.reduce((n, t) => n + t.exposureOnly, 0),
      exposureExemptedGlyphs: tracks.reduce((n, t) => n + t.exposureExemptedGlyphs, 0),
      headwordsWithoutRomanization: tracks.reduce(
        (n, t) => n + t.headwordsWithoutRomanization, 0),
      tracksWithUnknownScript: unknownScriptTracks.length,
    },
    unknownScriptTracks: unknownScriptTracks.sort(),
  };
}
