// HL-C50 — the book is a standalone artifact. See HL09 §8.

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { defaultCurriculumRoot, loadBookCorpus, loadEverything } from "../src/loader.js";
import { canonicalLessonSource } from "../src/hash.js";

const BOOKS = loadBookCorpus().books;

// Chapters AND the book shell. `loadBookCorpus` records `entrypoint` as a path and
// never reads it, so a test over `book.chapters` alone passes green while the TITLE
// PAGE prints a repo path — which is exactly what Japanese and Chinese were doing,
// more prominently than any chapter clause.
const CHAPTERS = [
  ...BOOKS.flatMap((book) =>
    book.chapters.map((chapter) => ({ source: chapter.source, tex: chapter.tex })),
  ),
  ...BOOKS.map((book) => ({
    source: book.entrypoint,
    tex: readFileSync(join(defaultCurriculumRoot(), book.entrypoint), "utf8"),
  })),
];

const TRACK_PREFIX: Record<string, string> = {
  arabic: "AR", bengali: "BN", chinese: "ZH", french: "FR", german: "GE",
  gujarati: "GU", hindi: "HI", italian: "IT", japanese: "JA", kannada: "KA",
  latin: "LA", malayalam: "ML", marathi: "MR", persian: "FA", portuguese: "PT",
  punjabi: "PA", russian: "RU", sanskrit: "SA", spanish: "ES", tamil: "TA",
  telugu: "TE", urdu: "UR",
};

describe("a reader holding the PDF", () => {
  it("is never cited a lesson from a DIFFERENT language's book", () => {
    // Malayalam chapters cited TE-C27/28/29; Hindi cited AR-C27. A reader with one
    // volume cannot follow any of them. The cross-language FACT beside such a
    // citation is the point of the book and always stays — only the pointer to
    // another volume goes.
    const offenders: string[] = [];
    for (const { source, tex } of CHAPTERS) {
      const own = TRACK_PREFIX[source.split("/")[0]!];
      // Not `continue` — an unmapped track would score zero offenders and pass green.
      expect(own, `${source}: no TRACK_PREFIX entry`).toBeDefined();
      for (const [, prefix] of tex.matchAll(/\b([A-Z]{2})-C\d+/g)) {
        if (prefix !== own && Object.values(TRACK_PREFIX).includes(prefix!)) {
          offenders.push(`${source}: ${prefix}-C…`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("is never cited one from a lesson's FRONTMATTER either", () => {
    // The `ES-C14` in GE-C16 lived in `etymology_hook`, which the .tex never renders —
    // so the guard above would have missed one of the nine defects it was written for.
    // Frontmatter still reaches the narration export and the app, and it is the
    // authored source of truth, so it is held to the same rule.
    const offenders: string[] = [];
    for (const lesson of loadEverything().lessons) {
      const own = TRACK_PREFIX[lesson.language];
      expect(own, `${lesson.language}: no TRACK_PREFIX entry`).toBeDefined();
      for (const [, prefix] of canonicalLessonSource(lesson).matchAll(/\b([A-Z]{2})-C\d+/g)) {
        if (prefix !== own && Object.values(TRACK_PREFIX).includes(prefix!)) {
          offenders.push(`${lesson.realization.lessonId}: ${prefix}-C…`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("is never told it already learned something that lives in another volume", () => {
    // "What Arabic phrase, already taught in this course, shares सुबह's root?" was a
    // RECALL question in the Hindi book quizzing the Arabic one. Unanswerable.
    //
    // The first version of this guard demanded the literal words "in this course" and
    // passed green while the SAME chapter's printed heading still said "a root you've
    // already met, in Arabic". So match the claim, then decide by CONTEXT: ~86
    // chapters say "already met" about their OWN earlier lessons ("already met in
    // Chapter 24"), which is the callback the ramp is built on and must keep passing.
    // What fails is the claim sitting next to ANOTHER language, or next to an appeal
    // to a different language in the abstract.
    // A legitimate callback carries an IN-volume locator: "already met in Chapter 24",
    // "already met last lesson". The defect carries an OUT-of-volume one: "already met
    // in Arabic", "already met in the Arabic arc", "already taught in this course".
    // So match the claim plus the locator that follows it, and let the locator decide.
    //
    // Four things this regex learned the hard way, each proven by execution:
    //  - `[^.?!\n]` not `[^.?!]`, or a 60-char window jumps a paragraph break and
    //    pairs a claim with an unrelated locator two paragraphs down;
    //  - the language must be read from the CAPTURED locator, not searched for across
    //    the whole match — searching found "Sanskrit" in "already met alongside
    //    Sanskrit, in Telugu" and flagged the Telugu book for citing itself;
    //  - no bare `from <language>`: in this corpus "from Sanskrit ucca" is an etymology,
    //    not a pointer, so `from` is accepted only before "the <lang> book/arc/…";
    //  - scoped to .tex. Extending it to frontmatter is right and is the next step, but
    //    the corpus has ~20 cross-volume pointers phrased in ways this pattern cannot
    //    see at all ("you learned in Hindi", "Latin's colour lesson"), so a narrow
    //    guard that holds is worth more here than a broad one that must be muted.
    const LOCATOR =
      `(?:in this course|in this track|in (?:a|another|one) (?:completely )?different language` +
      `|in (?:the )?(${Object.keys(TRACK_PREFIX).join("|")})(?:\\b| (?:arc|book|track))` +
      `|from the (${Object.keys(TRACK_PREFIX).join("|")}) (?:arc|book|track|lesson|chapter))`;
    const OFFENDING = new RegExp(
      `(?:already|genuinely) (?:met|taught|learned|established|found|been)\\b[^.?!\\n]{0,60}?\\b${LOCATOR}`,
      "gi",
    );
    const offenders: string[] = [];
    for (const { source, tex } of CHAPTERS) {
      const own = source.split("/")[0]!;
      for (const match of tex.matchAll(OFFENDING)) {
        // Read the language out of the locator the regex actually matched. A book
        // naming its OWN language points nowhere else.
        const named = (match[1] ?? match[2])?.toLowerCase();
        if (named !== own) offenders.push(`${source}: ${match[0].replace(/\s+/g, " ")}`);
      }
    }
    expect(offenders).toEqual([]);
  });

  it("is never sent to a file in the git repository", () => {
    // 99 chapters ended a paragraph with "Practice lessons: lessons/AR-C01-*.md".
    // That sentence is addressed to somebody with a checkout. `book-cli.ts` already
    // states the principle — "a reader holding the PDF cannot follow a link into a
    // Git repository" — and drops relative links for exactly this reason; the
    // handwritten chapters had never been held to it.
    const offenders = CHAPTERS.filter(({ tex }) =>
      /lessons\/[A-Z]{2}-C\d|(?:Practice|Companion)\s+(?:practice\s+)?lessons?:?/.test(tex),
    ).map(({ source }) => source);
    expect(offenders).toEqual([]);
  });

  it("is never pointed at a path in the curriculum tree at all", () => {
    // Broader than the line that prompted this: any `lessons/`, `curriculum.json`,
    // `chapters.json` or `.md` path printed to a reader is the same mistake.
    const offenders = CHAPTERS.filter(({ tex }) =>
      /\\(?:texttt|nolinkurl|path)\{[^}]*(?:lessons\/|\.md|chapters\.json|curriculum\.json)[^}]*\}/.test(
        tex,
      ),
    ).map(({ source }) => source);
    expect(offenders).toEqual([]);
  });
});
