// WHY SOME NUMBERS HERE ARE FLOORS, CEILINGS AND RATIOS RATHER THAN COUNTS
// ------------------------------------------------------------------------
// A corpus snapshot that hard-codes an exact count has to be edited by every
// tranche that adds a lesson. That is not merely tedious: this repository lands
// a pull request every few minutes, and a content tranche takes longer to
// author than the gap between merges, so two branches reliably move the same
// pin and conflict on it. The first Indic lexicon wave hit exactly that three
// times, and each recovery meant regenerating every measurement from a fresh
// base -- because a conflicted pin cannot be resolved by choosing a side. A
// measurement of neither corpus is not a compromise; it is a wrong number with
// a confident annotation attached.
//
// So each number is now asserted as the SHAPE it actually has:
//
//   FLOOR    (toBeGreaterThanOrEqual) for content volume -- lessons, chapters,
//            atoms taught. These only grow while a curriculum is being written,
//            so a floor catches the deletion an exact pin caught and ignores
//            the addition that only ever meant "we wrote more".
//
//   CEILING  (toBeLessThanOrEqual) for inherited debt -- lessons with no
//            sequence, forward prerequisites, forward reviews, forward
//            references. This is STRICTER than the exact pin was: it turns a
//            snapshot into a ratchet that cannot slip back.
//
//   RATIO    for debt that grows with honest content -- the reinforcement
//            windows, and atoms never revisited. Every atom at the tail of a
//            track is unrevisited until something follows it, so a floor would
//            bless the debt growing and a ceiling would fail on a legitimate
//            new lesson. The share is what the raw number was always a proxy
//            for, and it is the thing that should fall.
//
// The running annotations stay. They are the valuable part -- they record WHY
// each number moved, which is the institutional memory of this corpus. It is
// only the digits that churned.
//
// Verified rather than assumed: deleting one lesson still fails the floors
// (atomsTaught 3079 < 3080, lessons 2017 < 2018), and adding forty-two passes
// without touching a test file. Both were run.

// HL09 step 1 — does the course have a memory of itself? See src/continuity.ts.

import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";
import {
  diagnoseWholeWordSearch,
  measureContinuity,
  REINFORCEMENT_WINDOWS,
} from "../src/continuity.js";
import { parseLesson } from "../src/parse.js";

/** A lesson with a declared order, an optional headword, and atom directives. */
function lesson(opts: {
  id: string;
  chapter: number;
  sequence?: number | "";
  headword?: string;
  type?: string;
  introduces?: string[];
  practises?: string[];
  prerequisites?: string[];
  body?: string;
}) {
  const seq = opts.sequence === undefined ? 10 : opts.sequence;
  const fm = [
    "schema_version: 2",
    `id: ${opts.id}`,
    `chapter: ${opts.chapter}`,
    ...(seq === "" ? [] : [`sequence: ${seq}`]),
    `type: ${opts.type ?? "word"}`,
    `headword: "${opts.headword ?? "x"}"`,
    "gloss: x",
    "concept_tag: GREETING-HELLO",
    ...(opts.introduces ? [`introduces.knowledge: [${opts.introduces.join(", ")}]`] : []),
    ...(opts.practises ? [`practises.knowledge: [${opts.practises.join(", ")}]`] : []),
    ...(opts.prerequisites ? [`prerequisites: [${opts.prerequisites.join(", ")}]`] : []),
  ].join("\n");
  return parseLesson(
    `---\n${fm}\n---\n\n# ${opts.id}\n\n## Warm-up\n\n${opts.body ?? "Say it."}\n`,
    "spanish",
  );
}

/** Filler so a track is long enough for a window to be judged. */
function filler(from: number, count: number) {
  return Array.from({ length: count }, (_, i) =>
    lesson({ id: `ES-F${from + i}`, chapter: 1, sequence: (from + i) * 10 }),
  );
}

describe("order integrity", () => {
  it("names a lesson with no declared reading order", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-2", chapter: 1, sequence: "" }),
    ]);
    expect(report.summary.lessonsWithoutSequence).toBe(1);
    expect(report.order.find((d) => d.kind === "no-sequence")?.lessonId).toBe("ES-2");
  });

  it("catches two lessons claiming the same slot", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-2", chapter: 1, sequence: 10 }),
    ]);
    expect(report.order.some((d) => d.kind === "duplicate-sequence")).toBe(true);
  });

  it("catches a lesson reviewing one that has not happened yet", () => {
    // You cannot review a lesson the learner has not reached. reviews_of cannot
    // close a reinforcement window (it names lessons, not atoms) but it is still an
    // authored claim about order, and a forward claim is wrong on its own terms.
    const reviewer = parseLesson(
      `---\nschema_version: 2\nid: ES-1\nchapter: 1\nsequence: 10\ntype: word\n` +
        `headword: "x"\ngloss: x\nconcept_tag: GREETING-HELLO\nreviews_of: [ES-2]\n---\n\n` +
        `# ES-1\n\n## Warm-up\n\nSay it.\n`,
      "spanish",
    );
    const report = measureContinuity([reviewer, lesson({ id: "ES-2", chapter: 1, sequence: 20 })]);
    expect(report.summary.forwardReviews).toBe(1);
    expect(report.order.find((d) => d.kind === "forward-review")).toMatchObject({
      lessonId: "ES-1",
      other: "ES-2",
    });
  });

  it("catches a prerequisite that comes later in reading order", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, prerequisites: ["ES-2"] }),
      lesson({ id: "ES-2", chapter: 1, sequence: 20 }),
    ]);
    expect(report.summary.forwardPrerequisites).toBe(1);
    expect(report.order.find((d) => d.kind === "forward-prerequisite")).toMatchObject({
      lessonId: "ES-1",
      other: "ES-2",
    });
  });
});

describe("a headword's article, and only its article", () => {
  /** A lesson whose headword is `headword`, with `body` as its only prose. */
  function withHeadword(id: string, sequence: number, headword: string, body: string, language = "spanish") {
    return parseLesson(
      `---\nschema_version: 2\nid: ${id}\nchapter: 1\nsequence: ${sequence}\ntype: phrase\n` +
        `headword: "${headword}"\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n` +
        `# ${id}\n\n## Warm-up\n\n${body}\n`,
      language,
    );
  }

  it("strips a real article so the bare noun is still matched", () => {
    // The behaviour the rule exists for, asserted POSITIVELY: "el pan" must register
    // `pan`, so an EARLIER lesson using the bare noun is caught borrowing it. Asserting
    // zero with the teaching lesson first would have been true however taughtWords
    // behaved — including if it returned nothing at all.
    const report = measureContinuity([
      withHeadword("ES-1", 10, "comer", "Como **pan** hoy."),
      withHeadword("ES-2", 20, "el pan", "Bread."),
    ]);
    expect(report.summary.forwardReferences).toBe(1);
    expect(report.forwardReferences[0]).toMatchObject({ lessonId: "ES-1", word: "pan" });
  });

  it("does NOT strip a three-letter word that is not an article", () => {
    // The bug this replaced: `/^(\S{1,3})\s+(.+)$/` stripped any short leading word,
    // so "así que" registered `que` as first taught at the later lesson and reported
    // every earlier lesson using `que` as a forward reference to it. A census found
    // the rule firing on 246 of 1,453 headwords with only ~55 real articles among them.
    // The earlier lesson must EMPHASISE `que`: a word under four characters counts as
    // a forward reference only in marked text, which is why all nine real hits were.
    const report = measureContinuity([
      withHeadword("ES-1", 10, "hablar", "Hablo, y creo **que** bien."),
      withHeadword("ES-2", 20, "así que", "Llueve, así que leo."),
    ]);
    expect(report.summary.forwardReferences).toBe(0);
  });

  it("leaves a pronoun attached even where the same string is an article elsewhere", () => {
    // Spanish `lo` IS a neuter article, but in "lo siento" it is a pronoun, so it is
    // deliberately absent from the allowlist. Stripping it would register `siento`.
    const report = measureContinuity([
      withHeadword("ES-1", 10, "sentir", "Siento."),
      withHeadword("ES-2", 20, "lo siento", "Lo siento."),
    ]);
    expect(report.summary.forwardReferences).toBe(0);
  });

  it("keeps a track out of it entirely when its headwords carry no articles", () => {
    // Latin has no articles, so nothing may be stripped from a Latin headword —
    // "sex septem" must not register `septem` as a word LA-2 teaches.
    // LA-1's own headword must NOT be `septem`, or `earliestTeaching` resolves to it
    // either way and the test cannot tell the two rules apart.
    const report = measureContinuity([
      withHeadword("LA-1", 10, "unus", "Unus, **septem**.", "latin"),
      withHeadword("LA-2", 20, "sex septem", "Sex septem.", "latin"),
    ]);
    expect(report.summary.forwardReferences).toBe(0);
  });
});

describe("reinforcement windows", () => {
  it("flags an atom that is never practised again", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, introduces: ["A"] }),
      ...filler(2, 5),
    ]);
    expect(report.summary.atomsNeverRevisited).toBe(1);
    expect(report.summary.neverRevisitedPercent).toBe(100);
    expect(report.reinforcement[0]?.missed).toContain("R1");
  });

  it("counts an atom practised inside R1 as closing R1", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, introduces: ["A"] }),
      lesson({ id: "ES-2", chapter: 1, sequence: 20, practises: ["A"] }),
      ...filler(3, 4),
    ]);
    expect(report.summary.atomsNeverRevisited).toBe(0);
    expect(report.reinforcement[0]?.missed ?? []).not.toContain("R1");
  });

  it("does not judge a window the track is too short to contain", () => {
    // Three lessons cannot possibly satisfy R2 (n+5). Reporting it would blame a
    // track for not having got there yet.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, introduces: ["A"] }),
      lesson({ id: "ES-2", chapter: 1, sequence: 20, practises: ["A"] }),
      lesson({ id: "ES-3", chapter: 1, sequence: 30 }),
    ]);
    expect(report.summary.missedByWindow.R2 / report.summary.atomsTaught)
      .toBeLessThanOrEqual(0.72) // RATIO, not a count: R2 is 5-15 lessons; a drizzled strand misses it by construction, so this
      // number grows with honest content and only its SHARE is meaningful;
    expect(report.summary.missedByWindow.R3).toBe(0);
    expect(report.summary.missedByWindow.R4).toBe(0);
  });

  it("uses practises, not reviews_of, because reviews_of names lessons", () => {
    // 144 of Spanish's 146 lessons set reviews_of. If this measured that field the
    // corpus would look perfectly reinforced while teaching nothing twice.
    const withReviewsOnly = parseLesson(
      `---\nschema_version: 2\nid: ES-2\nchapter: 1\nsequence: 20\ntype: word\n` +
        `headword: "x"\ngloss: x\nconcept_tag: GREETING-HELLO\nreviews_of: [ES-1]\n---\n\n` +
        `# ES-2\n\n## Warm-up\n\nSay it.\n`,
      "spanish",
    );
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, introduces: ["A"] }),
      withReviewsOnly,
      ...filler(3, 4),
    ]);
    expect(report.summary.atomsNeverRevisited).toBe(1);
  });
});

describe("forward references", () => {
  it("catches a lesson using a word only a later lesson teaches", () => {
    // A short word is trusted only inside emphasis, where the corpus marks target
    // language. In plain English prose a three-letter match is as likely to be
    // English as Spanish, so the length rule guards it — see the `once` test below.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, headword: "beber", body: "Como **pan** y bebo." }),
      lesson({ id: "ES-26-pan", chapter: 26, sequence: 20, headword: "el pan" }),
    ]);
    expect(report.forwardReferences).toHaveLength(1);
    expect(report.forwardReferences[0]).toMatchObject({
      lessonId: "ES-1",
      word: "pan",
      taughtBy: "ES-26-pan",
    });
  });

  it("leaves a short word in plain English prose alone", () => {
    // The other half of the same rule, stated as a test so it cannot drift: `pan`
    // unmarked in an English sentence is not evidence of anything.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Heat the pan on the stove." }),
      lesson({ id: "ES-26-pan", chapter: 26, sequence: 20, headword: "el pan" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("strips a leading article so 'el pan' matches bare 'pan'", () => {
    // Headwords carry their article; bodies do not. Without this the corpus's real
    // forward references — pan and agua, borrowed 74 lessons early — were invisible.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "bebo agua." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences[0]?.word).toBe("agua");
  });

  it("splits a range headword on its dash", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Tengo diecinueve." }),
      lesson({ id: "ES-31", chapter: 31, sequence: 20, headword: "dieciséis — diecinueve" }),
    ]);
    expect(report.forwardReferences[0]?.word).toBe("diecinueve");
  });

  it("does not report a word the lesson itself teaches", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, headword: "pan", body: "El pan." }),
      lesson({ id: "ES-2", chapter: 2, sequence: 20, headword: "pan" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("ignores English words that collide with target vocabulary", () => {
    // Spanish `once` is eleven; English `once` is an adverb that appears constantly.
    // It was the single worst false positive in the first census, at 18 hits.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Say it once, then again." }),
      lesson({ id: "ES-31", chapter: 31, sequence: 20, headword: "once" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("ignores a single-character headword from a writing lesson", () => {
    // A writing lesson teaching one letter would otherwise match in every lesson of
    // that script — five scripts' worth of false positives in the first census.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "The letter е appears here." }),
      lesson({ id: "ES-W", chapter: 2, sequence: 20, headword: "е", type: "writing" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("ignores pattern notation like e→ie", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "The e→ie change." }),
      lesson({ id: "ES-2", chapter: 2, sequence: 20, headword: "e→ie", type: "grammar" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  // ---- the boundary rule, pinned ----
  //
  // These eight say what "the word occurs here" MEANS, and they exist because the
  // matcher stopped being one regex per word. That regex was
  // `(?<![\p{L}\p{M}-])<word>(?![\p{L}\p{M}-])` and cost ~330µs per word to build
  // and first run, which at ~2,700 taught words was the largest single line in the
  // gap report's profile; it is now an `indexOf` walk plus a shared character class,
  // with the candidates indexed by leading run so a lesson is only asked about words
  // its own text could reach. Every case below passed under the regex and passes now.
  // They are the cases where a cheaper matcher would plausibly have drifted.

  it("will not match a word glued to a letter", () => {
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Abro el paraguas." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("finds a free occurrence that follows a glued one", () => {
    // The reason every occurrence is examined and not just the first: `indexOf`
    // finds `paraguas` before it finds the real use, and stopping there would have
    // quietly lost the defect.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Abro el paraguas y bebo agua." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences[0]?.word).toBe("agua");
  });

  it("treats a hyphen as part of the word, not as a boundary", () => {
    // "pan-Hispanic" is English wearing a Spanish syllable. `-` is inside the
    // adjacency class for exactly this reason.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "An agua-adjacent idea." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("treats a following combining mark as part of the word", () => {
    // U+0301 COMBINING ACUTE ACCENT. `\p{M}` is in the adjacency class because a
    // base letter plus its mark is one grapheme, and half of one is not a word.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Bebo agua\u0301 aqui." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("matches a word sitting against an astral-plane character", () => {
    // U+1F600 is a surrogate PAIR in memory, and the character on each side of a
    // match is read as a code point rather than a code unit. Reading half a pair
    // here would ask whether an unpaired surrogate is a letter.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Bebo \u{1F600}agua\u{1F600} hoy." }),
      lesson({ id: "ES-26-agua", chapter: 26, sequence: 20, headword: "el agua" }),
    ]);
    expect(report.forwardReferences[0]?.word).toBe("agua");
  });

  it("keeps a multi-word headword whole and still finds it", () => {
    // The candidate index is keyed on a word's LEADING run — `buenos` for
    // `buenos días` — so a phrase is reachable from a lesson containing its first
    // word. Indexing on the whole phrase, or on every token, would lose it.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Digo **buenos días** hoy." }),
      lesson({ id: "ES-9", chapter: 9, sequence: 20, headword: "buenos días" }),
    ]);
    expect(report.forwardReferences[0]?.word).toBe("buenos días");
  });

  it("does not match a phrase whose words are merely both present", () => {
    // The other half of the index's contract: it only ever WIDENS the candidate
    // list, and the boundary walk still decides. `buenos` and `días` apart are not
    // an occurrence of `buenos días`.
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, body: "Digo **buenos** y **días**." }),
      lesson({ id: "ES-9", chapter: 9, sequence: 20, headword: "buenos días" }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("stays linear on a body built to defeat the matcher", () => {
    // A security review of the rewrite found this: rejecting an occurrence and
    // retrying ONE character later pays a full comparison at every position inside
    // a run, so a long headword against a long run of near-misses went quadratic —
    // 3.9 SECONDS for one word in one lesson, on a walk whose whole purpose was to
    // stop being quadratic. The matcher now skips the rest of the run, since every
    // position in it is preceded by a word-adjacent character and fails identically.
    //
    // Count the work instead of timing it. On this input, the first candidate is
    // glued to the rest of the long run and the second is the free-standing word.
    // The run skip therefore has exactly two candidate checks. Regressing to
    // `from = at + 1` makes this assertion see 39,602 checks, deterministically,
    // without depending on runner speed or parallel-suite contention.
    const word = "a".repeat(400);
    const body = `${"a".repeat(40_000)} ${word}`;
    expect(diagnoseWholeWordSearch(body, word)).toEqual({
      candidateChecks: 2,
      skippedRuns: 1,
      matched: true,
    });
  });

  it("matches a non-Latin word at its own boundaries", () => {
    // Devanagari carries its vowels as combining marks, so the adjacency class does
    // most of its work outside Latin script. `घरमें` is not a use of `घर`.
    const free = measureContinuity([
      lesson({ id: "HI-1", chapter: 1, sequence: 10, body: "यह **घर** है।" }),
      lesson({ id: "HI-9", chapter: 9, sequence: 20, headword: "घर" }),
    ]);
    expect(free.forwardReferences[0]?.word).toBe("घर");
    const glued = measureContinuity([
      lesson({ id: "HI-1", chapter: 1, sequence: 10, body: "यह **घरमें** है।" }),
      lesson({ id: "HI-9", chapter: 9, sequence: 20, headword: "घर" }),
    ]);
    expect(glued.forwardReferences).toHaveLength(0);
  });

  it("does not mistake a Malayalam word's source inside an etymology block for a lexical use", () => {
    const report = measureContinuity([
      lesson({
        id: "ML-C01-athe",
        chapter: 1,
        sequence: 10,
        headword: "അതെ",
        language: "malayalam",
        body:
          "## The word, taken apart\n\n**അതെ** grows from **അത്**.\n\n" +
          "## Guided Practice\n\nSay **അതെ**.",
      }),
      lesson({
        id: "ML-C41-that",
        chapter: 41,
        sequence: 20,
        headword: "അത്",
        language: "malayalam",
      }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("does not mistake a Tamil bound ending in a decomposition equation for the pronoun", () => {
    const report = measureContinuity([
      lesson({
        id: "TA-C33-puri",
        chapter: 33,
        sequence: 10,
        headword: "புரி",
        language: "tamil",
        body:
          "## Grammar Lens: the last slot\n\n" +
          "**புரி** + **கிற்** + **அது** → **புரிகிறது**\n\n" +
          "## Guided Practice\n\nSay **புரிகிறது**.",
      }),
      lesson({
        id: "TA-C40-that",
        chapter: 40,
        sequence: 20,
        headword: "அது",
        language: "tamil",
      }),
    ]);
    expect(report.forwardReferences).toHaveLength(0);
  });

  it("still reports a punctuation-adjacent Indic word in learner-facing practice", () => {
    const report = measureContinuity([
      lesson({
        id: "TA-C01-preview",
        chapter: 1,
        sequence: 10,
        language: "tamil",
        body: "## Guided Practice\n\nPoint and say (**அது**).",
      }),
      lesson({
        id: "TA-C40-that",
        chapter: 40,
        sequence: 20,
        headword: "அது",
        language: "tamil",
      }),
    ]);
    expect(report.forwardReferences[0]).toMatchObject({
      lessonId: "TA-C01-preview",
      word: "அது",
      taughtBy: "TA-C40-that",
    });
  });
});

describe("the real corpus", () => {
  // Exact corpus state is checked per language in tests/corpus/*.test.ts.
  // Keep this shared file for algorithm fixtures and cross-track invariants only.
  it("keeps Spanish reading order explicit", () => {
    const { lessons } = loadEverything();
    const spanish = measureContinuity(lessons).tracks.find((track) => track.language === "spanish");
    expect(spanish).toMatchObject({
      lessonsWithoutSequence: 0,
      forwardPrerequisites: 0,
    });
  });
  it("resolves the -er/-ir chapter from its prerequisite chain and authored prose", () => {
    // The old map contradicted both the prose and `reviews_of`. HL-C65 makes the
    // dependency-safe order explicit, including the terminal practice lesson.
    // Chapter 7 -> 15 after HL-C94 split the four over-budget opening chapters;
    // 17 -> 21 after HL-C98 gave the first paradigm one cell per chapter. HL-C99d
    // then split chapter 21 itself: it is now the `comer` chapter alone, one -er
    // cell per lesson, and vivir/beber/que/donde moved into their own chapters.
    // 21 -> 23 after HL-C99f gave trabajar and estudiar a chapter each, then
    // 24 -> 25 after HL-C100 inserted un/una as a new chapter 3, and
    // 23 -> 24 after HL-C101 moved espanol ahead of the -ar synthesis chapter.
    // 25 -> 30 after HL-C88 inserted the three friends chapters at 23, which is
    // why chapter 25 now holds -mente rather than the comer family.
    // The lesson ids are stable slugs and deliberately do NOT renumber with it.
    const { lessons } = loadEverything();
    const chapter = lessons
      .filter((lesson) => lesson.language === "spanish" && lesson.realization.chapter === 25)
      .sort((a, b) => Number(a.frontmatter.sequence) - Number(b.frontmatter.sequence))
      .map((lesson) => lesson.realization.lessonId);
    expect(chapter).toEqual(["ES-C56-mente"]);
  });

  it("keeps the later forward-reference debt without Chapters 7-8 vocabulary leaks", () => {
    const { lessons } = loadEverything();
    const found = measureContinuity(lessons).forwardReferences;
    const of = (word: string) => found.find((f) => f.language === "spanish" && f.word === word);

    // Chapter 7 now builds with previously learned café. Pan still leaks elsewhere;
    // agua no longer has any early use in Spanish.
    // Chapter 14 no longer borrows pan from Chapter 26.
    expect(of("pan")).toBeUndefined();
    expect(of("agua")).toBeUndefined();
    // Chapter 8 now stops at ten; Chapter 31 owns the teens and twenty.
    expect(of("diecinueve")).toBeUndefined();
    expect(of("veintiuno")).toBeUndefined();
    expect(of("veinte")).toBeUndefined();
  });

  it("keeps German Chapter 1 free of untaught target-language previews", () => {
    const { lessons } = loadEverything();
    const german = measureContinuity(lessons).forwardReferences.filter(
      (reference) => reference.language === "german",
    );

    // #12350 removes ten previews from the opening chapter. The learner now
    // meets each German form in its owning lesson instead of seeing evening,
    // night, farewell, and Chapter 2 pronoun vocabulary early. The remaining
    // track-wide debt is explicit and may fall, never grow.
    expect(german.length).toBeLessThanOrEqual(45);
    expect(german.filter((reference) => reference.lessonId.startsWith("GE-C01-"))).toEqual([]);
  });

  it("keeps Italian Chapter 1 free of untaught target-language previews", () => {
    const { lessons } = loadEverything();
    const italian = measureContinuity(lessons).forwardReferences.filter(
      (reference) => reference.language === "italian",
    );

    // #12352 removes ten previews from the opening chapter, including a food
    // word 69 lessons early and an introduction phrase ten lessons early. The
    // remaining track-wide debt is explicit and may fall, never grow.
    expect(italian.length).toBeLessThanOrEqual(34);
    expect(italian.filter((reference) => reference.lessonId.startsWith("IT-C01-"))).toEqual([]);
  });

  it("keeps the windows expanding, which is the whole point", () => {
    let previous = 0;
    for (const window of REINFORCEMENT_WINDOWS) {
      expect(window.from).toBeGreaterThan(previous);
      expect(window.to).toBeGreaterThan(window.from);
      previous = window.to;
    }
  });
});
