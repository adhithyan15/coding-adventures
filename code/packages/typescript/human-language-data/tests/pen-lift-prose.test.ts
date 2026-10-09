// A writing lesson's prose must not count pen lifts the letter does not have.
//
// ---------------------------------------------------------------------------
// The complaint this answers
// ---------------------------------------------------------------------------
//
// "So many pen lifts." Several batches refitted letters to the number of
// strokes native writers really use (HP Labs India's LipiTk counts for
// Telugu, Tamil, Bengali and Devanagari; KanjiVG for kana). Each refit changed
// the letter's inventory record (`penLifts`, `strokeOrder`) and its filmstrip,
// and the strip's heading now says "one unbroken stroke". But the lesson prose
// was written against the OLD paths, and nobody re-read it:
//
//   TE-S117 వ   record: 0 lifts   prose: "Lift, then movement 2 … Lift once
//                                         more for movement 3 … closing the
//                                         two pen lifts"
//   MW-W43 म    record: 1 lift    prose: "**Pen lifts: 2.**"
//   JA-W16 ふ   record: 3 lifts   prose: "counting the four lifts aloud"
//
// A beginner reads the sentence, not the record. When the sentence says "lift"
// and the strip beside it shows one unbroken line, the learner is told two
// different things on one page, and the sentence is the one they obey.
//
// ---------------------------------------------------------------------------
// What this file checks, and why only this
// ---------------------------------------------------------------------------
//
// Free prose about a hand moving is too varied to parse in general ("lift for
// the lower bowl", "make the short lift between them", "each after a lift").
// A checker that tried would either miss most of it or cry wolf, and a test
// that cries wolf gets skipped. So this checks the two shapes that can be read
// EXACTLY, and leaves the rest to an author's eye:
//
//   1. WHOLE-LETTER COUNTS. A sentence that states how many times the pen
//      leaves the paper for the whole letter, in one of the forms the corpus
//      really uses:
//
//        form                              example                      lifts
//        --------------------------------  ---------------------------  -----
//        the step-list label               **Pen lifts: 2.**            2
//        a count opening a sentence        Five pen lifts.              5
//        strokes and lifts together        Six strokes, five lifts.     5
//        counting while tracing            counting the four lifts      4
//        pen-down runs                     two pen-down runs            1
//        a self-check question             How many pen lifts? (Two.)   2
//
//      Each must equal the record's `penLifts`. A run is a stretch of ink
//      between lifts, so N runs are N - 1 lifts; likewise N strokes.
//
//   2. ZERO MEANS ZERO. When the record says the pen never leaves the paper,
//      the block that prints the strip (`stripBlockIndex`) must not tell the
//      learner to lift: no sentence opening "Lift", no numbered step reading
//      "lift, then …", no ", then lift …" that goes on to more ink. A lift at
//      the very END of a letter ("draw it from the top down, then lift.") is
//      not a lift between strokes, and is allowed.
//
// Both rules read only SINGLE-letter writing lessons (`writingLetterOf`): a
// lesson about one glyph, whose prose can only be about that glyph's strokes.
// A warm-up line that recalls ANOTHER letter ("Write ن. The deep bowl, a lift,
// and one dot.") sits outside the strip's block, and none of the six forms in
// rule 1 is used for a recalled letter anywhere in the corpus. If that ever
// changes, this file will say so loudly, and the fix is to name the letter in
// the sentence, not to loosen the pattern.
//
// ---------------------------------------------------------------------------
// Where "the record" comes from
// ---------------------------------------------------------------------------
//
// `data/scripts/<script>.json` (or its sharded `.d/`), read through
// `loadScripts`. A letter is keyed by `glyph`, a vowel sign or diacritic by
// `mark`, and a lesson headword that writes a sign on a dotted circle (◌ौ) is
// matched without the circle. A glyph with no `penLifts` makes no verified
// claim, so its lessons are not checked: absent means "not verified", which is
// not the same as "none" (see `Letter.penLifts` in `types.ts`).

import { describe, expect, it } from "vitest";
import { loadLessons, loadScripts } from "../src/loader.js";
import {
  DERIVED_FILMSTRIP_SCRIPTS,
  stripBlockIndex,
  writingLetterOf,
} from "../src/figure-targets.js";
import type { ParsedLesson } from "../src/parse.js";
import type { ScriptData } from "../src/types.js";

// ---------------------------------------------------------------------------
// Reading numbers the way the lessons write them
// ---------------------------------------------------------------------------

/** Number words up to fifteen, plus the ways a lesson says "none" or "once". */
const NUMBER_WORDS: Readonly<Record<string, number>> = {
  no: 0, none: 0, zero: 0, not: 0,
  one: 1, once: 1,
  two: 2, twice: 2,
  three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10,
  eleven: 11, twelve: 12, thirteen: 13, fourteen: 14, fifteen: 15,
};

/** The alternation every pattern below uses for "a count". */
const COUNT = "(\\d+|no|zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen)";

/** "Five" → 5, "12" → 12, anything else → undefined. */
function numberOf(word: string): number | undefined {
  if (/^\d+$/.test(word)) return Number(word);
  return NUMBER_WORDS[word.toLowerCase()];
}

// ---------------------------------------------------------------------------
// Rule 1: the whole-letter count forms
// ---------------------------------------------------------------------------

/** One stated count, and the sentence fragment that stated it. */
interface LiftClaim {
  form: string;
  /** The number of pen lifts the sentence implies, or NaN if it contradicts itself. */
  lifts: number;
  text: string;
}

/**
 * Each form: a name for the failure message, a pattern, and how to turn a
 * match into a number of LIFTS (not strokes, not runs).
 *
 * Every pattern is anchored to wording that only ever describes a whole
 * letter. That is why "one stroke, no lift" is NOT a form: Chinese lessons use
 * it for a single turning stroke inside a character (见's frame, 对's héngpiě),
 * and reading it as the character's count would fail four correct lessons. The
 * strokes-and-lifts form therefore insists on the plural "strokes".
 */
const COUNT_FORMS: ReadonlyArray<{
  form: string;
  pattern: RegExp;
  lifts: (match: RegExpMatchArray) => number | undefined;
}> = [
  {
    form: "the step-list label",
    pattern: /\bPen lifts?:\s*(\d+)\b/gi,
    lifts: (m) => numberOf(m[1]!),
  },
  {
    // "Five pen lifts." / "**Three pen lifts, and every stroke stands alone.**"
    // / "That is two pen lifts in all." The count must OPEN the sentence (or a
    // bold run, or follow "That is"), so "closing the two pen lifts" in the
    // middle of a sentence is not read as a count.
    form: "a count opening a sentence",
    pattern: new RegExp(`(?:^|[.!?]\\s+|\\*\\*|\\bThat is )${COUNT} pen lifts?\\b`, "gim"),
    lifts: (m) => numberOf(m[1]!),
  },
  {
    // "Six strokes, five lifts." Both numbers are read, and they must agree
    // with each other before either is compared with the record.
    form: "strokes and lifts together",
    pattern: new RegExp(`\\b${COUNT} strokes[,;] ${COUNT} (?:pen )?lifts?\\b`, "gi"),
    lifts: (m) => {
      const strokes = numberOf(m[1]!);
      const lifts = numberOf(m[2]!);
      if (strokes === undefined || lifts === undefined) return undefined;
      return strokes - 1 === lifts ? lifts : Number.NaN;
    },
  },
  {
    form: "counting while tracing",
    pattern: new RegExp(`\\bcounting (?:the )?${COUNT} (?:pen )?lifts\\b`, "gi"),
    lifts: (m) => numberOf(m[1]!),
  },
  {
    form: "pen-down runs",
    pattern: new RegExp(`\\b${COUNT} pen-down runs?\\b`, "gi"),
    lifts: (m) => {
      const runs = numberOf(m[1]!);
      return runs === undefined ? undefined : runs - 1;
    },
  },
  {
    // "How many pen lifts? (**Two.**)" — the answer is the first word inside
    // the parentheses. A question about pen-down RUNS answers in runs.
    form: "a self-check question",
    pattern: /How many (pen lifts|pen-down runs|times do you lift the pen)[^?]*\?\s*\((?:\*\*)?([A-Za-z0-9]+)/gi,
    lifts: (m) => {
      const answer = numberOf(m[2]!);
      if (answer === undefined) return undefined;
      return /runs/i.test(m[1]!) ? answer - 1 : answer;
    },
  },
];

/** Every whole-letter count stated anywhere in a piece of prose. */
function liftClaims(prose: string): LiftClaim[] {
  const claims: LiftClaim[] = [];
  for (const { form, pattern, lifts } of COUNT_FORMS) {
    for (const match of prose.matchAll(new RegExp(pattern.source, pattern.flags))) {
      const value = lifts(match);
      // A count we cannot read is reported, not skipped: a silent skip is how
      // a check goes vacuous without anyone noticing.
      claims.push({ form, lifts: value ?? Number.NaN, text: match[0].replace(/\s+/g, " ").trim() });
    }
  }
  return claims;
}

// ---------------------------------------------------------------------------
// Rule 2: an instruction to lift, in a letter that has none
// ---------------------------------------------------------------------------

/**
 * The imperative shapes of "lift", after bold markers are removed:
 *
 *   "… upper bowl. Lift, then movement 2 …"       a sentence opening "Lift"
 *   "4. Lift once more, then draw …"               a numbered step
 *   "2. lift, then descend the right stem"         a step-list item
 *   "… the loop, then lift and draw the bar"       ", then lift" going on
 *
 * The last is refused only when more writing FOLLOWS: ", then lift." with a
 * full stop ends the letter, which is what the pen does after any letter.
 * "Lift" is matched with a capital L at a sentence start on purpose: "nearly
 * as many lift once" describes other writers, not this learner's pen.
 */
const LIFT_INSTRUCTIONS: readonly RegExp[] = [
  /(?:^|[.;:!?]\s+|\d+\.\s+|—\s+)Lift\b/g,
  /\d+\.\s+lift\b/g,
  /,\s*then lift\b(?!\.)/g,
];

/** Every instruction to lift in a piece of prose, with a little context. */
function liftInstructions(prose: string): string[] {
  const flat = prose.replace(/\*\*/g, "").replace(/\s+/g, " ");
  const found: string[] = [];
  for (const pattern of LIFT_INSTRUCTIONS) {
    for (const match of flat.matchAll(new RegExp(pattern.source, pattern.flags))) {
      const at = match.index ?? 0;
      found.push(flat.slice(Math.max(0, at - 40), at + match[0].length + 30).trim());
    }
  }
  return found;
}

// ---------------------------------------------------------------------------
// The records
// ---------------------------------------------------------------------------

/** script id → (glyph or mark, with and without ◌) → penLifts. */
function penLiftIndex(scripts: Record<string, ScriptData>): Map<string, Map<string, number>> {
  const index = new Map<string, Map<string, number>>();
  for (const [id, data] of Object.entries(scripts)) {
    const byGlyph = new Map<string, number>();
    // Letters, independent vowels, digits, final consonants and marks all
    // carry `penLifts` the same way; only the key's field name differs.
    for (const value of Object.values(data)) {
      if (!Array.isArray(value)) continue;
      for (const row of value as Array<Record<string, unknown>>) {
        if (row === null || typeof row !== "object" || typeof row.penLifts !== "number") continue;
        const key = typeof row.glyph === "string" ? row.glyph : typeof row.mark === "string" ? row.mark : undefined;
        if (key === undefined) continue;
        byGlyph.set(key, row.penLifts);
        byGlyph.set(key.replace(/◌/g, ""), row.penLifts);
      }
    }
    index.set(id, byGlyph);
  }
  return index;
}

interface CheckedLesson {
  lesson: ParsedLesson;
  glyph: string;
  penLifts: number;
}

/** Every single-letter writing lesson whose letter has a verified lift count. */
function checkedLessons(): CheckedLesson[] {
  const index = penLiftIndex(loadScripts());
  const out: CheckedLesson[] = [];
  for (const lesson of loadLessons()) {
    const script = DERIVED_FILMSTRIP_SCRIPTS[lesson.language];
    if (script === undefined) continue;
    const glyph = writingLetterOf(lesson);
    if (glyph === undefined) continue;
    const records = index.get(script);
    const penLifts = records?.get(glyph) ?? records?.get(glyph.replace(/◌/g, ""));
    if (penLifts === undefined) continue;
    out.push({ lesson, glyph, penLifts });
  }
  return out;
}

/** Learner-facing prose: each block's display Markdown, directives removed. */
function proseOf(lesson: ParsedLesson): string {
  return lesson.blocks.map((block) => block.markdown ?? "").join("\n\n");
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

describe("the claim readers, on prose small enough to check by eye", () => {
  it("reads each whole-letter form as a number of lifts", () => {
    const lifts = (text: string) => liftClaims(text).map((claim) => claim.lifts);
    expect(lifts("**Pen lifts: 2.**")).toEqual([2]);
    expect(lifts("Down, round, up. Five pen lifts.")).toEqual([5]);
    expect(lifts("**Three pen lifts, and every stroke stands alone.**")).toEqual([3]);
    expect(lifts("Six strokes, five lifts.")).toEqual([5]);
    expect(lifts("Trace it, counting the four lifts aloud.")).toEqual([4]);
    expect(lifts("Use **two pen-down runs**.")).toEqual([1]);
    expect(lifts("How many pen lifts? (**None.**)")).toEqual([0]);
    expect(lifts("How many pen-down runs? (**Two.**)")).toEqual([1]);
  });

  it("flags a strokes-and-lifts pair that disagrees with itself", () => {
    expect(liftClaims("Six strokes, six lifts.")[0]?.lifts).toBeNaN();
  });

  it("does not read a part of a letter as the whole letter", () => {
    // A turning stroke inside 见, and a count in the middle of a sentence.
    expect(liftClaims("the frame's top, turning down the right side — one stroke, no lift")).toEqual([]);
    expect(liftClaims("keep the three movements clear instead of closing the two pen lifts")).toEqual([]);
  });

  it("finds an instruction to lift, and lets a letter end", () => {
    expect(liftInstructions("Movement 1 loops the bowl. Lift, then movement 2 sweeps.")).toHaveLength(1);
    expect(liftInstructions("3. Sweep right.\n4. Lift once more, then draw the chevron.")).toHaveLength(1);
    expect(liftInstructions("- **2.** lift, then descend the right stem")).toHaveLength(1);
    expect(liftInstructions("Draw its one straight stroke from top to bottom, then lift.")).toEqual([]);
    expect(liftInstructions("Without lifting, sweep on. No pen lift.")).toEqual([]);
    expect(liftInstructions("nearly as many lift once before the flourish")).toEqual([]);
  });
});

describe("pen lifts in single-letter writing lessons", () => {
  const lessons = checkedLessons();

  it("checks the whole corpus, not a corner of it", () => {
    // Anti-vacuity. Measured when this file was written: 753 single-letter
    // writing lessons with a verified count (657 letters, 96 signs), 225
    // whole-letter counts among them, and 252 lessons whose letter has no lift
    // at all. Run against the corpus as it stood before this file, the two
    // rules below found 13 lessons: four wrong counts (MW-W43 म, JA-W16 ふ,
    // JA-W17 む, TE-S124 ఎ) and ten Telugu lessons, ఎ among them, that told
    // the learner to lift in a letter native writers draw in one stroke. A refactor that silently stopped finding lessons, records or
    // claims would pass both rules below while checking nothing; these floors
    // fail it instead. They sit a little under the measurement so that a
    // lesson merged or retired does not break the build.
    expect(lessons.length).toBeGreaterThan(700);
    const claims = lessons.flatMap(({ lesson }) => liftClaims(proseOf(lesson)));
    expect(claims.length).toBeGreaterThan(200);
    expect(lessons.filter(({ penLifts }) => penLifts === 0).length).toBeGreaterThan(200);
  });

  it("states no whole-letter lift count that the record contradicts", () => {
    const wrong: string[] = [];
    for (const { lesson, glyph, penLifts } of lessons) {
      for (const claim of liftClaims(proseOf(lesson))) {
        if (claim.lifts === penLifts) continue;
        wrong.push(
          `${lesson.realization.lessonId} (${glyph}): "${claim.text}" [${claim.form}] ` +
            `says ${Number.isNaN(claim.lifts) ? "something unreadable" : claim.lifts}, ` +
            `the record says ${penLifts}`,
        );
      }
    }
    // The fix is the sentence. The record follows a cited pen path and a
    // native-writer count; if you believe the RECORD is wrong, change it in
    // script-ductus with its evidence, and the filmstrip changes with it.
    expect(wrong).toEqual([]);
  });

  it("never tells the learner to lift where the record draws one unbroken stroke", () => {
    const wrong: string[] = [];
    for (const { lesson, glyph, penLifts } of lessons) {
      if (penLifts !== 0) continue;
      const block = lesson.blocks[stripBlockIndex(lesson)];
      if (block === undefined) continue;
      for (const instruction of liftInstructions(block.markdown ?? "")) {
        wrong.push(`${lesson.realization.lessonId} (${glyph}): "…${instruction}…"`);
      }
    }
    expect(wrong).toEqual([]);
  });
});
