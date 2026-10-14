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
  TAKEN_APART_ORIGIN_SIGNALS,
  blockStatesOrigin,
  hasEmptyEtymologyBlock,
  hasEtymologyHook,
  hasVisibleEtymology,
  introducesNewWord,
  measureEtymologyCoverage,
  renderEtymologyCoverage,
  statesOrigin,
  takenApartOriginSignals,
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

// ---- The empty "taken apart" block (HL24 section 9.4) -------------------------
//
// Every loose signal gets the sentences it was kept for AND the near-miss it
// must not fire on. The table is checked against TAKEN_APART_ORIGIN_SIGNALS
// itself, so a signal cannot be added to the module without its fixtures.

/** [text, track] -- the track only matters to `donor-word`. */
type SignalCase = string | [string, string];

const SIGNAL_FIXTURES: Record<string, { fires: SignalCase[]; rejects: SignalCase[] }> = {
  inherited: {
    fires: ["*Ōdu* is inherited, not borrowed.", "French, Spanish and Italian all inherited the same word."],
    rejects: ["She was disinherited."],
  },
  borrowed: {
    fires: ["Spanish borrowed the English word entire, island and all."],
    rejects: ["The borrower pays it back."],
  },
  "a-loan": {
    fires: ["**ಮಿಕ್ಸಿ** is a loan from Indian English *mixie*.", "**banana** — a loanword."],
    // A gloss, quoted or emphasised, is the meaning of *préstamo*, not its history.
    rejects: ['**el préstamo** — "a loan".', "**el préstamo** is *a loan*."],
  },
  "taken-over": {
    fires: ["two Sanskrit words taken over whole, tatsamas"],
    rejects: ["They had taken overnight trains."],
  },
  tatsama: {
    fires: ["a genuine **Sanskrit tatsama**", "one of the tatsamas"],
    rejects: ["tatsamatā, the quality of sameness"],
  },
  tadbhava: {
    fires: ["**लड़ना** is a tadbhava, a word inherited and reshaped"],
    rejects: ["tadbhavatā"],
  },
  "learned-word": {
    fires: ["*fábrica* is a learned word, taken straight from the page.", "the learned borrowings"],
    rejects: ["You have learned words like *casa* already."],
  },
  "semi-learned": {
    fires: ["*siglo* is semi-learned: it half wore down."],
    rejects: ["semi learned by heart"],
  },
  "built-on": {
    fires: ["**casar** is built from *casa*, \"house\".", "built on the question base *e-*"],
    rejects: ["The church was built in 1590.", "a building on the corner"],
  },
  "comes-from": {
    fires: ["It comes from *vil*, \"to sell\".", "Both come from a Germanic root."],
    rejects: ["The coming from abroad was late."],
  },
  "goes-back-to": {
    fires: ["It goes back to a bare Slavic particle."],
    // LA-C56-octavus: a return, not a descent.
    rejects: ["the ninth breaks it differently, and the tenth goes back to behaving."],
  },
  "traced-to": {
    fires: ["**चाबी** is usually traced to Portuguese *chave*."],
    rejects: ["She traced the outline with a finger."],
  },
  "descends-from": {
    fires: ["It descends from **vetulus**, the diminutive.", "*Konnte* and *could* descend from one verb."],
    rejects: ["The road descends steeply to the sea."],
  },
  "derived-from": {
    fires: [
      "**dessin** — formed from *dessiner*, \"to draw\".",
      "*porte*, from *porter*, \"to carry\"",
      "\"favour, grace, gratitude,\" from *grātus*",
      "*Nada* came from **res nata**, \"a thing born.\"",
      "*Avē* is the more formal greeting — \"hail\" — from *avēre*",
    ],
    // ES-C436-recoger: a spelling comparison. And a date range is no form.
    rejects: ["because *ge* would be said differently from *go*", "open, from Monday to Friday"],
  },
  "cousin-of": {
    fires: ["*свинья* is a cousin of English *swine*.", "Its relative in Greek is *spondḗ*."],
    rejects: ["**la prima** — the cousin.", "your cousin in Madrid"],
  },
  "is-english": {
    fires: ["**drei** is English **three**.", "**sehr** is English **sore**."],
    // The sound note rule 3 already refuses: italic letters, not bold words.
    rejects: ["German *w* is English *v*."],
  },
  "donor-word": {
    fires: [["Latin said **hodie**, *this day*.", "spanish"], ["**pero** was **two** Latin words.", "spanish"]],
    rejects: [
      // The track's own language: a fact about the word being taught.
      ["Latin had two words here and they are not interchangeable.", "latin"],
      // The bare name, measured and rejected.
      ["named once, in Latin letters, never in Gurmukhi", "punjabi"],
    ],
  },
  "native-family": {
    fires: ["**கரடி** is native Dravidian.", "a native Indo-Aryan adverb"],
    rejects: ["a native speaker would say", "native Japanese speakers"],
  },
  "source-tag": {
    fires: ["(DEDR 3301)", "(Lewis & Short)", "(Lane, s.v.)", "(EtymDB 2.1)"],
    // Case-sensitive, and the name must end there: a lowercase lane or a
    // longer word after the parenthesis is not Lane's Lexicon.
    rejects: ["Keep to the slow lane.", "(lane closed)", "(Laneway entrance)"],
  },
  "source-named": {
    fires: ["DEDR 3376 lists Malayalam *tūṅṅuka*.", "Vasmer derives it from *руб*.", "its origin is not recorded in EtymDB 2.1."],
    // A bare acronym is how a block says the dictionary is silent.
    rejects: ["DEDR has no entry for it."],
  },
  uncertain: {
    fires: ["The origin of **ಹುಬ್ಬು** is uncertain."],
    rejects: ["the uncertainty of the weather"],
  },
  "not-settled": {
    fires: ["its deeper origin is not settled (Vasmer)."],
    // ES-C439-tirar's "unsettled" -- a documented miss, pinned.
    rejects: ["its history is unsettled."],
  },
  "unknown-origin": {
    fires: ["a word of unknown origin", "Its origin is unknown."],
    rejects: ["an unknown word"],
  },
  "no-sure-origin": {
    fires: ["**titulus** has no sure origin."],
    rejects: ["There is no sure way to tell."],
  },
  disputed: {
    fires: ["the details are disputed"],
    rejects: ["an undisputed fact"],
  },
  starred: {
    fires: ["It comes from *\\*ātmanaka*, \"own\".", "the stem **\\*yā-**"],
    rejects: ["5 \\* 3 is fifteen."],
  },
};

function signalsIn(testCase: SignalCase): string[] {
  const [text, language] = typeof testCase === "string" ? [testCase, "spanish"] : testCase;
  return takenApartOriginSignals(text, language);
}

describe("takenApartOriginSignals", () => {
  it("has fixtures for every signal, and for no signal that is gone", () => {
    expect(Object.keys(SIGNAL_FIXTURES).sort()).toEqual(TAKEN_APART_ORIGIN_SIGNALS.map((signal) => signal.name).sort());
  });

  for (const [name, fixture] of Object.entries(SIGNAL_FIXTURES)) {
    it(`${name}: fires on the sentences it was kept for`, () => {
      for (const testCase of fixture.fires) expect(signalsIn(testCase), String(testCase)).toContain(name);
    });

    it(`${name}: control -- rejects its near miss`, () => {
      for (const testCase of fixture.rejects) expect(signalsIn(testCase), String(testCase)).not.toContain(name);
    });
  }

  it("ignores HTML comments, which the reader is never shown", () => {
    expect(takenApartOriginSignals("<!-- inherited, borrowed -->\nIt opens an excuse.")).toEqual([]);
  });

  it("stays linear on hostile input", () => {
    // Long runs of every shape a signal begins with and never finishes.
    const started = performance.now();
    const hostile = [
      `**${"x".repeat(50_000)}`,
      `${"** ".repeat(50_000)}is English`,
      `${", from ".repeat(50_000)}`,
      `goes back to ${"a".repeat(50_000)}`,
      `${"Latin ".repeat(50_000)}`,
      `cousin of ${"Old ".repeat(50_000)}`,
    ];
    for (const text of hostile) takenApartOriginSignals(text, "spanish");
    expect(performance.now() - started).toBeLessThan(2_000);
  });
});

describe("blockStatesOrigin", () => {
  it("passes any block whose prose passes rule 3", () => {
    expect(blockStatesOrigin({ type: "input", markdown: "**día** — from Latin *dies*." }, "spanish")).toBe(true);
  });

  it("reads the loose signals inside a 'taken apart' block", () => {
    expect(blockStatesOrigin({ type: "etymology", markdown: "**casar** is built from *casa*." }, "portuguese")).toBe(true);
  });

  it("control: the same sentence in any other block is not an origin statement", () => {
    // Corpus-wide, "built from" and "inherited" are as often about a house or
    // a sentence as a word; see the module comment.
    expect(blockStatesOrigin({ type: "input", markdown: "**casar** is built from *casa*." }, "portuguese")).toBe(false);
    expect(blockStatesOrigin({ type: "culture-pragmatics", markdown: "She inherited the house." }, "spanish")).toBe(false);
  });

  it("control: a 'taken apart' block that takes apart a sentence states nothing", () => {
    expect(
      blockStatesOrigin(
        { type: "etymology", markdown: "A postposition trails its noun: *ghar par*, 'on the house'." },
        "hindi",
      ),
    ).toBe(false);
  });
});

describe("hasEmptyEtymologyBlock", () => {
  const empty = (body: string, opts: { language?: string; title?: string } = {}) =>
    hasEmptyEtymologyBlock(lesson({ id: "ES-C01-e", language: opts.language, headword: opts.title, body }));

  it("is true for a 'taken apart' block that takes apart syntax and never says where the word came from", () => {
    // HI-C87-par's shape: why the postposition trails, not where पर came from.
    expect(
      empty(
        "## The word, taken apart\n\n**पर** comes after its noun, never before: *ghar par*, \"on the house\".",
        { language: "hindi" },
      ),
    ).toBe(true);
  });

  it("is true for a block that only lists parts", () => {
    // ES-C432-lavadora's shape: the parts, and no word about where they came from.
    expect(empty("## The word, taken apart\n\n**lavar** plus **-dora**: the machine that washes.")).toBe(true);
  });

  it("is false for a 'built on X' block", () => {
    expect(empty("## The word, taken apart\n\n**casar** is built from *casa*, \"house\".", { language: "portuguese" })).toBe(false);
  });

  it("is false when another block or the preamble says it instead", () => {
    const syntax = "## The word, taken apart\n\nIt opens an excuse, not an answer.";
    expect(empty(`${syntax}\n\n## You'll want to know\n\n**x** — from Latin *x*.`)).toBe(false);
    expect(empty(syntax, { title: "día ← dies" })).toBe(false);
  });

  it("counts an origin HEADING over nothing, and reads no loose signal under it", () => {
    // The loose reading belongs to the "taken apart" block only. Under
    // "Where it comes from", "inherited" gets no more trust than anywhere else.
    expect(empty("## Where it comes from\n\nA long story.")).toBe(true);
    expect(empty("## Where it comes from\n\nIt is inherited.")).toBe(true);
    expect(empty("## Where it comes from\n\nIt comes from Latin *x*.")).toBe(false);
  });

  it("is false for a lesson with no etymology structure at all, which the visible count already holds", () => {
    const bare = lesson({ id: "ES-C01-b" });
    expect(hasVisibleEtymology(bare)).toBe(false);
    expect(hasEmptyEtymologyBlock(bare)).toBe(false);
  });

  // The documented misses, pinned so a change to the signals has to say what
  // it did to them. See the module comment.
  it("documented false negative: an origin told without a signal word is counted empty", () => {
    // ES-C301-papel: "Behind it is papyrus" names no language and uses no signal.
    expect(empty("## The word, taken apart\n\nBehind it is **papyrus**, the Egyptian reed.")).toBe(true);
  });

  it("documented false positive: a signal about a different word rescues the block", () => {
    // ES-C431-retraso: the parts are re- and tras; the Latin word is tardar's.
    expect(
      empty("## The word, taken apart\n\n**re-** plus **tras**. Spanish also has **tardar**, from a different Latin word for *slow*."),
    ).toBe(false);
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
    // Visible on structure alone: the block takes a sentence apart.
    lesson({
      id: "ES-C01-e",
      sequence: 60,
      hook: "e ← E",
      body: "## The word, taken apart\n\n*Es que* opens an excuse, not an answer.",
    }),
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
        wordLessonsWithEmptyEtymologyBlock: 0,
      },
      {
        language: "spanish",
        wordLessons: 5,
        wordLessonsWithoutEtymologyHook: 2,
        wordLessonsWithoutVisibleEtymology: 2,
        wordLessonsWithoutAnyEtymology: 1,
        wordLessonsWithEmptyEtymologyBlock: 1,
      },
    ]);
  });

  it("sums the tracks and rounds the covered share DOWN", () => {
    expect(report.summary).toEqual({
      wordLessons: 6,
      wordLessonsWithoutEtymologyHook: 3,
      wordLessonsWithoutVisibleEtymology: 3,
      wordLessonsWithoutAnyEtymology: 2,
      wordLessonsWithEmptyEtymologyBlock: 1,
      hookPercent: 50,
      visiblePercent: 50,
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
      { lessonId: "AR-C01-x", language: "arabic", position: 0, missingHook: true, missingVisible: true, emptyEtymologyBlock: false },
      { lessonId: "ES-C01-b", language: "spanish", position: 1, missingHook: false, missingVisible: true, emptyEtymologyBlock: false },
      { lessonId: "ES-C01-c", language: "spanish", position: 2, missingHook: true, missingVisible: false, emptyEtymologyBlock: false },
      { lessonId: "ES-C01-d", language: "spanish", position: 3, missingHook: true, missingVisible: true, emptyEtymologyBlock: false },
      // Hook and visible both present, and still listed: the block is empty.
      { lessonId: "ES-C01-e", language: "spanish", position: 5, missingHook: false, missingVisible: false, emptyEtymologyBlock: true },
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
    expect(text).toContain("spanish: 2 word lessons; 1 no hook, 2 no visible etymology, 1 neither, 0 empty etymology block");
  });

  it("prints the empty-block count in the totals and per track", () => {
    const text = renderEtymologyCoverage(
      measureEtymologyCoverage([
        lesson({ id: "ES-C01-a", hook: "a", body: "## The word, taken apart\n\nIt opens an excuse." }),
      ]),
    ).join("\n");
    expect(text).toContain('1 more count as visible only because a "taken apart" block or origin heading is there');
    expect(text).toContain("spanish: 1 word lessons; 0 no hook, 0 no visible etymology, 0 neither, 1 empty etymology block");
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
          wordLessonsWithEmptyEtymologyBlock: 0,
        },
      ],
      summary: {
        wordLessons: 0,
        wordLessonsWithoutEtymologyHook: 0,
        wordLessonsWithoutVisibleEtymology: 0,
        wordLessonsWithoutAnyEtymology: 0,
        wordLessonsWithEmptyEtymologyBlock: 0,
        hookPercent: 100,
        visiblePercent: 100,
      },
    }).join("\n");
    expect(text).not.toContain("\u001b");
  });
});
