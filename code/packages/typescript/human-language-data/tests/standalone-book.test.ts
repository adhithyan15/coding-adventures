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

// Chapters PLUS lesson sources. Frontmatter never reaches the .tex — which is
// exactly where the `ES-C14` in GE-C16 hid — but it does reach narration and the app,
// and it is the authored source of truth, so it is held to the same rule.
const SURFACES = [
  ...CHAPTERS,
  ...loadEverything().lessons.map((lesson) => ({
    source: `${lesson.language}/lessons/${lesson.realization.lessonId}.md`,
    // canonicalLessonSource returns JSON, where a line break is the two-character
    // escape `\n`, not a newline. Un-escaping matters twice: `\\s+` in the patterns
    // below could not cross a wrap (a pointer split as "the Spanish\ntrack" was
    // invisible, and one real defect hid there), and `[^.?!\\n]` silently bounded
    // nothing, so the 60-char window could jump paragraphs on this surface alone.
    tex: canonicalLessonSource(lesson).replace(/\\n/g, "\n"),
  })),
];

const TRACK_PREFIX: Record<string, string> = {
  arabic: "AR", bengali: "BN", chinese: "ZH", french: "FR", german: "GE",
  gujarati: "GU", hindi: "HI", italian: "IT", japanese: "JA", kannada: "KA",
  latin: "LA", malayalam: "ML", marathi: "MR", marwadi: "MW", persian: "FA", portuguese: "PT",
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
    // Two things decide a match, because the phrase alone cannot: ~86 chapters say
    // "already met in Chapter 24" about their OWN earlier lessons, which is the
    // callback the gentle ramp is built on and must keep passing. What fails is a
    // second-person memory claim aimed at ANOTHER language, or a pointer at another
    // language's MATERIAL ("the Spanish track", "Latin's colour lesson").
    //
    // Four things this learned the hard way, each proven by execution:
    //  - `[^.?!\n]` not `[^.?!]`, or a 60-char window jumps a paragraph break and
    //    pairs a claim with an unrelated locator two paragraphs down;
    //  - read the language from the CAPTURED locator, not by searching the whole
    //    match — searching found "Sanskrit" in "already met alongside Sanskrit, in
    //    Telugu" and flagged the Telugu book for citing itself;
    //  - no bare `from <language>`: "from Sanskrit ucca" is an etymology, not a
    //    pointer, so `from` is accepted only before "the <lang> book/arc/…";
    //  - `track` is also a VERB. "unlike how closely Kannada and Telugu track Tamil"
    //    is prose, not a pointer, so the material nouns require a possessive or an
    //    article rather than matching a bare "<language> track".
    const L = Object.keys(TRACK_PREFIX).join("|");
    // Plural too: "the Spanish, Italian, French, and Portuguese tracks" slipped a
    // singular-only pattern, as did "in the Spanish and Latin lessons".
    const MATERIAL = `(?:lesson|chapter|book|track|arc|volume|curriculum|series)s?`;
    const PATTERNS = [
      // "you met in Tamil", "you may remember from Latin", "you've now seen in Spanish"
      new RegExp(
        `\\byou(?:'ve| have| may| already|'ll| will)?\\s+(?:\\w+\\s+){0,3}?` +
          `(?:meet|met|learn|learned|learnt|see|saw|seen|remember|encountered)\\b[^.?!\\n]{0,60}?` +
          `\\bin\\s+(?:the\\s+)?(${L})\\b`,
        "gi",
      ),
      // "the Spanish track", "Latin's colour lesson", "in the Tamil book"
      new RegExp(`\\b(?:the|in the|from the)\\s+(${L})\\s+${MATERIAL}\\b`, "gi"),
      new RegExp(`\\b(${L})'s\\s+(?:\\w+\\s+){0,2}?${MATERIAL}\\b`, "gi"),
      // "already taught in this course", "earlier in this arc"
      new RegExp(
        `(?:already|genuinely)\\s+(?:met|taught|learned|established|found|been)` +
          `\\b[^.?!\\n]{0,60}?\\bin this (?:course|track|arc)\\b()`,
        "gi",
      ),
      // "earlier in this arc", "elsewhere in this course" -- no memory verb needed.
      // "earlier in this arc", "elsewhere in this course" -- no memory verb needed.
      // An adjective may intervene: "elsewhere in this ENTIRE arc" defeated a pattern
      // that demanded the noun come straight after "this", and the Malayalam sibling
      // of a phrase fixed in Kannada was sitting in that gap.
      new RegExp(
        `\\b(?:earlier|elsewhere|later)\\s+in this\\s+(?:\\w+\\s+)?` +
          `(?:course|track|arc|curriculum|volume|series)\\b()`,
        "gi",
      ),
      // A pointer needs no language and no verb: "the daughter lessons", "its own
      // track", "the companion volumes".
      new RegExp(
        `\\b(?:the (?:daughter|companion|sibling|other) (?:lesson|chapter|book|track|arc|volume)s?` +
          `|its own (?:track|book|volume|arc))\\b()`,
        "gi",
      ),
      // Bare "in this course" / "in this curriculum" / "in this series". The previous
      // change left this off on purpose and said why: 31 sites carried it, and a guard
      // muted on its own corpus is worth less than one that holds. They are gone now,
      // so it holds.
      //
      // "in this BOOK" is deliberately absent from the alternation. A reader is
      // holding the book, so an ordinal about it ("the first verb in this book whose
      // vowel changes") is answerable -- which is what several of these became.
      new RegExp(`\\bin this\\s+(?:course|curriculum)\\b()`, "gi"),
      // The same set named without "in": "the course keeps finding connections", "the
      // curriculum treats Latin as a taproot", "the course's first taste of case".
      new RegExp(
        `\\b(?:the|this)\\s+(?:whole|entire|single|full)?\\s*(?:course|curriculum)\\b()`,
        "gi",
      ),
      // NOT "series": in this corpus that is a conjugation or letter paradigm --
      // "learn this series whole", "the retroflex ट-ठ-ड series" -- not the book set.
      new RegExp(`\\b(?:course|curriculum)'s\\b()`, "gi"),
      // "in this arc" for a set that spans volumes -- "every other language in this
      // arc". An arc INSIDE one book ("the numbers arc") is fine and stays; what
      // fails is the phrase used as a stand-in for the whole series.
      new RegExp(`\\b(?:in|across|throughout)\\s+this\\s+arc\\b()`, "gi"),
      // "that's the fifth language to reach the full arc" -- an ordinal over a set
      // the reader cannot see, and cannot check.
      new RegExp(
        `\\bthe (?:first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth) language\\b()`,
        "gi",
      ),
      new RegExp(`\\bthis course covers\\b|\\bevery track\\b|\\bthe tracks\\b()`, "gi"),
    ];
    const offenders: string[] = [];
    for (const { source, tex } of SURFACES) {
      const own = source.split("/")[0]!;
      for (const pattern of PATTERNS) {
        for (const match of tex.matchAll(pattern)) {
          // A book naming its OWN language points nowhere else.
          const named = match[1]?.toLowerCase();
          if (named !== own) offenders.push(`${source}: ${match[0].replace(/\s+/g, " ")}`);
        }
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
