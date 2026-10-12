// HL24 section 9 -- etymology coverage (src/etymology-coverage.ts).
//
// Synthetic lessons only. The corpus ratchet -- every track's two counts
// pinned exactly -- lives in tests/root-ledger.test.ts, beside the other
// etymology measurement on the real corpus, so the ~30,000 lessons are loaded
// once for both rather than once more here.
//
// Every rule gets a firing fixture AND a control, and the documented misses
// of the visible-etymology rule get one each too: a heuristic whose known
// failures are not pinned drifts silently in both directions.

import { describe, expect, it } from "vitest";
import {
  LEXICAL_ATOM,
  hasEtymologyHook,
  hasVisibleEtymology,
  introducesNewWord,
  measureEtymologyCoverage,
  renderEtymologyCoverage,
  statesOrigin,
} from "../src/etymology-coverage.js";
import { parseLesson } from "../src/parse.js";

/**
 * A schema-v2 lesson. `hook: null` omits the field; a string is written
 * QUOTED, so whitespace-only values survive the frontmatter reader exactly.
 */
function lesson(opts: {
  id: string;
  language?: string;
  type?: string | null;
  headword?: string;
  sequence?: number;
  introduces?: string[];
  hook?: string | null;
  body?: string;
}) {
  const type = opts.type === null ? "" : `type: ${opts.type ?? "word"}\n`;
  const hook = opts.hook === undefined || opts.hook === null ? "" : `etymology_hook: "${opts.hook}"\n`;
  const introduces = opts.introduces
    ? `introduces:\n  knowledge: [${opts.introduces.join(", ")}]\n`
    : "";
  return parseLesson(
    `---
schema_version: 2
id: ${opts.id}
chapter: 1
sequence: ${opts.sequence ?? 10}
${type}headword: ${opts.headword ?? "x"}
gloss: x
${hook}${introduces}---

# ${opts.headword ?? "x"} — x

${opts.body ?? "## You'll want to know\n\n**x** — \"x\"."}
`,
    opts.language ?? "spanish",
  );
}

describe("introducesNewWord", () => {
  it("counts a word lesson even when it declares no atoms at all", () => {
    // 632 legacy word lessons name their words under non-LEX atoms, and 23
    // declare none. A headword is a claim to introduce that word.
    expect(introducesNewWord(lesson({ id: "ES-C01-x" }))).toBe(true);
  });

  it("counts a phrase headword", () => {
    // *de nada* from *res nata* is exactly the etymology HL00 asks for.
    expect(introducesNewWord(lesson({ id: "ES-C01-de-nada", type: "phrase", headword: "de nada" }))).toBe(true);
  });

  it("counts a lesson with no `type`, because the parser defaults it to word", () => {
    expect(introducesNewWord(lesson({ id: "ES-C01-untyped", type: null }))).toBe(true);
  });

  it("control: a review lesson that introduces nothing is outside the count", () => {
    expect(introducesNewWord(lesson({ id: "ES-R01", type: "review", introduces: [] }))).toBe(false);
  });

  it("counts a practice lesson that slips a lexical atom in through frontmatter", () => {
    const practice = lesson({ id: "ES-P01", type: "practice", introduces: ["ES-LEX-NEW-01"] });
    expect(introducesNewWord(practice)).toBe(true);
  });

  it("counts a lexical atom introduced only by a block directive", () => {
    // The frontmatter and the block directives are unioned (introducedAtoms);
    // reading only one of them is how a reader silently counts zero.
    const grammar = lesson({
      id: "ES-G01",
      type: "grammar",
      body: [
        "## Grammar Lens: one new word",
        "<!-- hl-knowledge: introduces=[ES-LEX-ONLY-IN-BLOCK]; assesses=[] -->",
        "",
        "Text.",
      ].join("\n"),
    });
    expect(introducesNewWord(grammar)).toBe(true);
  });

  it("control: RETROFLEX, REFLEXIVE and CIRCUMFLEX atoms are not lexical", () => {
    for (const atom of ["HI-SCRIPT-DA-RETROFLEX-01", "ES-GRAMMAR-REFLEXIVE-FIRST-PERSON", "FR-SCRIPT-CIRCUMFLEX-08", "ZH-TONE-LEXICAL"]) {
      expect(LEXICAL_ATOM.test(atom), atom).toBe(false);
      expect(introducesNewWord(lesson({ id: "HI-W01", type: "writing", introduces: [atom] })), atom).toBe(false);
    }
    expect(LEXICAL_ATOM.test("HI-LEX-C242-QUAL242-04")).toBe(true);
  });
});

describe("hasEtymologyHook", () => {
  it("is true for an authored hook", () => {
    expect(hasEtymologyHook(lesson({ id: "ES-C01-dia", hook: "día ← dies" }))).toBe(true);
  });

  it("is false when the field is absent or empty", () => {
    expect(hasEtymologyHook(lesson({ id: "ES-C01-a" }))).toBe(false);
    expect(hasEtymologyHook(lesson({ id: "ES-C01-b", hook: "" }))).toBe(false);
  });

  it("treats a whitespace-only hook as empty, not authored", () => {
    // Otherwise a generator could certify a whole wave with `etymology_hook: " "`.
    const blank = lesson({ id: "ES-C01-c", hook: "   " });
    expect(blank.realization.etymologyHook).toBe("   ");
    expect(hasEtymologyHook(blank)).toBe(false);
  });
});

describe("hasVisibleEtymology", () => {
  const visible = (body: string, language = "spanish") =>
    hasVisibleEtymology(lesson({ id: "ES-C01-v", language, body }));

  it("is true for the corpus's own etymology section, a 'taken apart' heading", () => {
    expect(visible("## The word, taken apart\n\n**treize** is three and ten.")).toBe(true);
  });

  it("is true for a heading that names the origin outright", () => {
    expect(visible("## Where it comes from\n\nA long story.")).toBe(true);
    expect(visible("## Etymology\n\nA long story.")).toBe(true);
  });

  it("is false when the origin is only in the hook, which nothing prints", () => {
    const quiet = lesson({ id: "GE-C17-kopf", hook: "Kopf ← Late Latin cuppa", body: "## You'll want to know\n\n**der Kopf** — the head." });
    expect(hasEtymologyHook(quiet)).toBe(true);
    expect(hasVisibleEtymology(quiet)).toBe(false);
  });

  it("reads the title line too, not only the blocks", () => {
    const titled = parseLesson(
      "---\nid: ES-C01-t\ntype: word\nheadword: día\n---\n\n# día ← dies\n\n## You'll want to know\n\nday.\n",
      "spanish",
    );
    expect(hasVisibleEtymology(titled)).toBe(true);
  });
});

describe("statesOrigin", () => {
  it("fires on the derivation arrow, and not on the composition arrow", () => {
    expect(statesOrigin("día ← dies")).toBe(true);
    expect(statesOrigin("ಹೇ + ಗೆ → ಹೇಗೆ")).toBe(false);
  });

  it("fires on from / via / through a source language, with qualifiers and emphasis", () => {
    expect(statesOrigin("It comes from Sanskrit घण्टा.")).toBe(true);
    expect(statesOrigin("borrowed from **Persian** صبح")).toBe(true);
    expect(statesOrigin("it reached Spanish through Old French")).toBe(true);
    expect(statesOrigin("via Proto-Dravidian *pac-")).toBe(true);
    expect(statesOrigin("From the Latin for 'right'.")).toBe(true);
  });

  it("follows a statement across a wrapped line", () => {
    // Prose is wrapped at eighty columns, so "from" ends a line and "Latin"
    // starts the next one more often than not.
    expect(statesOrigin("and that word came to Spanish from\nLatin *gratia*")).toBe(true);
  });

  it("control: a possessive or a script is not an origin", () => {
    expect(statesOrigin("Run it through Arabic's patterns.")).toBe(false);
    expect(statesOrigin("What is missing from the Gujarati script?")).toBe(false);
  });

  it("fires on a reconstructed ancestor", () => {
    expect(statesOrigin("Proto-Germanic *namô")).toBe(true);
    expect(statesOrigin("It goes back to PIE *kʷo-teros.")).toBe(true);
  });

  it("fires on kinship against a named language, and not on kinship of meaning", () => {
    expect(statesOrigin("Same word as English **sun**.")).toBe(true);
    expect(statesOrigin("cognate with Hebrew לילה")).toBe(true);
    expect(statesOrigin("a softer broom, and the same word covers it")).toBe(false);
  });

  it("fires on a donor language naming the ancestor without a verb", () => {
    expect(statesOrigin("Latin *october*, from ***octo***", "german")).toBe(true);
    expect(statesOrigin("**कब** is Sanskrit **कदा**, worn down.", "hindi")).toBe(true);
    expect(statesOrigin("सुबह was Persian, borrowed whole.", "hindi")).toBe(true);
  });

  it("control: the track's own language naming its own word is not an origin", () => {
    // In the Sanskrit track "Sanskrit *satya*" names the word being taught.
    expect(statesOrigin("Sanskrit *satya* means true.", "sanskrit")).toBe(false);
    expect(statesOrigin("Sanskrit *satya* means true.", "malayalam")).toBe(true);
  });

  it("control: a German sound note is spelling, not ancestry", () => {
    expect(statesOrigin("German *w* is English *v*.", "german")).toBe(false);
  });

  it("control: ordinary words that only look like etymology", () => {
    expect(statesOrigin("She inherited the house from her aunt.")).toBe(false);
    expect(statesOrigin("Say it from memory, then from Monday to Friday.")).toBe(false);
  });

  it("ignores HTML comments, which the reader is never shown", () => {
    expect(statesOrigin('<!-- hl-activity: {"feedback":"from Latin gratia"} -->\nThank you.')).toBe(false);
    // An unterminated comment hides everything after it, as a renderer would.
    expect(statesOrigin("Thank you. <!-- from Latin gratia")).toBe(false);
    expect(statesOrigin("from Latin <!-- x --> gratia")).toBe(true);
  });

  // The documented misses, pinned so a change to the rule has to say what it
  // did to them. See the module comment.
  it("documented false positive: a letter's origin reads as the word's", () => {
    expect(statesOrigin("**г**. It is Greek **gamma** with the corner rounded off.", "russian")).toBe(true);
  });

  it("documented false negative: an origin that names no language", () => {
    expect(statesOrigin("Usted grew out of *vuestra merced*, 'your grace'.")).toBe(false);
  });

  it("stays linear on hostile input", () => {
    // Bounded repetition only. Each of these used to be a classic backtracking
    // trap shape: a long run of qualifiers with no language after it, and a
    // long run of comment openers with no closer.
    const started = performance.now();
    expect(statesOrigin(`from ${"Old ".repeat(50_000)}nothing`)).toBe(false);
    expect(statesOrigin(`text ${"<!--".repeat(50_000)}`)).toBe(false);
    expect(statesOrigin(`${"from the ".repeat(50_000)}`)).toBe(false);
    expect(performance.now() - started).toBeLessThan(2_000);
  });
});

describe("measureEtymologyCoverage", () => {
  const corpus = [
    // spanish: three word lessons and a review that introduces nothing.
    lesson({ id: "ES-C01-a", sequence: 10, hook: "a ← A", body: "## The word, taken apart\n\nfrom Latin *a*." }),
    lesson({ id: "ES-C01-b", sequence: 20, hook: "b ← B" }),
    lesson({ id: "ES-C01-c", sequence: 30, body: "## You'll want to know\n\n**c** — from Latin *c*." }),
    lesson({ id: "ES-C01-d", sequence: 40, hook: "  " }),
    lesson({ id: "ES-R01", sequence: 50, type: "review", introduces: [] }),
    // alphabetically first, listed second: tracks must come out sorted.
    lesson({ id: "AR-C01-x", language: "arabic", sequence: 10 }),
  ];
  const report = measureEtymologyCoverage(corpus);

  it("counts per track, alphabetically, with review lessons outside the denominator", () => {
    expect(report.tracks).toEqual([
      {
        language: "arabic",
        wordLessons: 1,
        wordLessonsWithoutEtymologyHook: 1,
        wordLessonsWithoutVisibleEtymology: 1,
        wordLessonsWithoutAnyEtymology: 1,
      },
      {
        language: "spanish",
        wordLessons: 4,
        wordLessonsWithoutEtymologyHook: 2,
        wordLessonsWithoutVisibleEtymology: 2,
        wordLessonsWithoutAnyEtymology: 1,
      },
    ]);
  });

  it("sums the tracks and rounds the covered share DOWN", () => {
    expect(report.summary).toEqual({
      wordLessons: 5,
      wordLessonsWithoutEtymologyHook: 3,
      wordLessonsWithoutVisibleEtymology: 3,
      wordLessonsWithoutAnyEtymology: 2,
      hookPercent: 40,
      visiblePercent: 40,
    });
    // 2 of 3 is 66.7%; floor keeps a nearly-done track from printing as done.
    const twoOfThree = measureEtymologyCoverage([
      lesson({ id: "ES-C01-p", hook: "p" }),
      lesson({ id: "ES-C01-q", hook: "q" }),
      lesson({ id: "ES-C01-r" }),
    ]);
    expect(twoOfThree.summary.hookPercent).toBe(66);
  });

  it("lists every gap in reading order, saying which kind is missing", () => {
    expect(report.gaps).toEqual([
      { lessonId: "AR-C01-x", language: "arabic", position: 0, missingHook: true, missingVisible: true },
      { lessonId: "ES-C01-b", language: "spanish", position: 1, missingHook: false, missingVisible: true },
      { lessonId: "ES-C01-c", language: "spanish", position: 2, missingHook: true, missingVisible: false },
      { lessonId: "ES-C01-d", language: "spanish", position: 3, missingHook: true, missingVisible: true },
    ]);
  });

  it("reads an empty corpus as fully covered rather than dividing by zero", () => {
    const empty = measureEtymologyCoverage([]);
    expect(empty.tracks).toEqual([]);
    expect(empty.summary).toMatchObject({ wordLessons: 0, hookPercent: 100, visiblePercent: 100 });
  });
});

describe("renderEtymologyCoverage", () => {
  it("prints the totals and one line per track", () => {
    const text = renderEtymologyCoverage(
      measureEtymologyCoverage([lesson({ id: "ES-C01-a" }), lesson({ id: "ES-C01-b", hook: "b ← B" })]),
    ).join("\n");
    expect(text).toContain("Etymology coverage (HL24 §9)");
    expect(text).toContain("2 word-introducing lessons; 1 carry no etymology_hook (50% covered)");
    expect(text).toContain("spanish: 2 word lessons; 1 no hook, 2 no visible etymology, 1 neither");
  });

  it("strips control characters from a track name before printing it", () => {
    const text = renderEtymologyCoverage({
      tracks: [
        {
          language: "evil\u001b[2Kname",
          wordLessons: 0,
          wordLessonsWithoutEtymologyHook: 0,
          wordLessonsWithoutVisibleEtymology: 0,
          wordLessonsWithoutAnyEtymology: 0,
        },
      ],
      summary: {
        wordLessons: 0,
        wordLessonsWithoutEtymologyHook: 0,
        wordLessonsWithoutVisibleEtymology: 0,
        wordLessonsWithoutAnyEtymology: 0,
        hookPercent: 100,
        visiblePercent: 100,
      },
    }).join("\n");
    expect(text).not.toContain("\u001b");
  });
});
