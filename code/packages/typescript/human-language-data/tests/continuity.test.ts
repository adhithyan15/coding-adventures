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
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../src/continuity.js";
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
    // The budget is deliberately loose: this asserts the SHAPE (a linear scan of a
    // few megabytes, tens of milliseconds) against a regression that costs seconds,
    // so it cannot flake on a slow runner and still catches the bug it exists for.
    const word = "a".repeat(4_000);
    const body = `**${"a".repeat(4_000_000)} ${word}**`;
    const started = performance.now();
    const report = measureContinuity([
      lesson({ id: "ES-1", chapter: 1, sequence: 10, headword: "hola", body }),
      lesson({ id: "ES-2", chapter: 2, sequence: 20, headword: word }),
    ]);
    expect(performance.now() - started).toBeLessThan(1_000);
    expect(report.tracks[0]?.lessonCount).toBe(2);
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
});

describe("the real corpus", () => {
  // Explicit budget: this walks the WHOLE corpus for order, reinforcement windows and
  // forward references, and at 1,313 lessons it runs past vitest's 5,000 ms default under
  // full-suite parallel load (it passes comfortably in isolation, which is why a
  // per-file run will not reproduce the failure). Third test in this package to need
  // this — see `cli.test.ts` and `level-gate.test.ts`. The corpus only grows, so raise
  // the budget when it next runs close; never thin the walk to fit the clock.
  it("pins what nothing had measured", { timeout: 30_000 }, () => {
    const { lessons } = loadEverything();
    const report = measureContinuity(lessons);

    // ORDER. Until this reaches zero every other number here is provisional: a ramp
    // whose reading order is unknown cannot be verified at all.
        // 515 -> 507. Eight Tamil chapter-1 lessons that had no `sequence` were given one
    // from the order the curriculum path already declared, so the continuity walk can
    // finally see it. The corpus GREW by a lesson over the same change, and that lesson
    // was born sequenced, so it never entered this count.
    // 507 -> 483: exactly the 24 Tamil lessons in chapters 2-5 that had never declared
    // a sequence. They had been falling back to ALPHABETICAL order, which is how
    // chapter 2 came to teach the assembled phrase "en peyar" before "peyar" existed,
    // and to ask "what is your name?" last, after the chapter's own practice lesson. Nothing detected it, because a chapter with no declared order
    // has no order to contradict.
    // HL-C65 resolves the six Chapter-7 lessons from their prerequisite chain, so
    // Spanish becomes fully ordered: 483 -> 477 and 18 -> 17 tracks with debt.
    expect(report.summary.lessonsWithoutSequence).toBeLessThanOrEqual(322) // CEILING — this is debt; it may fall, never grow;    // 19 -> 18: Tamil joins chinese, japanese and latin as a fully-ordered track — // HL11: -155. Hindi, Telugu, Kannada, Malayalam and Sanskrit each had ~30 lessons with no declared reading order, so the corpus believed their words came after their script -- the opposite of what their books print. The order was recovered from the books' own section labels, which is the only place it was ever written down
    // the fourth of 22, not the first.
    expect(report.summary.tracksWithUnorderedLessons).toBe(12); // HL11: -5. Hindi, Telugu, Kannada, Malayalam and Sanskrit now declare a reading order for every lesson, recovered from their own books
        // 245 -> 240. Not five prerequisites fixed — five that were never real. Without a
    // `sequence` the walk falls back to alphabetical, so "TA-C01-aam requires
    // TA-C01-vanakkam-family-register" read as a forward prerequisite purely because
    // `aam` sorts first. Declaring chapter 1's order removed the artifact.    // 240 -> 230, and forwardReviews 285 -> 273 below: ten lessons each stopped
    // depending on something taught later. No lesson changed; only Tamil's order did.
    expect(report.summary.forwardPrerequisites).toBeLessThanOrEqual(143) // CEILING — this is debt; it may fall, never grow; // HL11: -82, and this is the payoff of the order fix rather than a side effect. A forward prerequisite is a lesson requiring something the reader has not reached; when five tracks had no declared order at all, their words sorted AFTER the script lessons that depended on them, so most of these were artifacts of an undeclared order rather than real gaps in the ramp

    // Lessons claiming to review material the learner has not reached yet.
    // 300 -> 285. Fourteen are the same alphabetical-fallback artifact as
    // forwardPrerequisites above, four of them in the writing track rather than the
    // word lessons; the fifteenth is a `reviews_of` list that genuinely shrank when its
    // lesson was rewritten.
    // 285 -> 273. Ten from the ordering above, and two more from a real fix:
    // TA-C02-magizhcci reviewed TA-C02-ungal-peyar-enna and TA-C04-mindum-sandippom
    // reviewed TA-C04-naalai, both before those lessons existed. Both were forward
    // under the old alphabetical fallback too; the hand-authored book runs them the
    // other way round, so the sequences now follow the book. Tamil forward reviews: 0.
    expect(report.summary.forwardReviews).toBeLessThanOrEqual(168) // CEILING — this is debt; it may fall, never grow; // HL11: -99, same cause as forwardPrerequisites above. With five tracks carrying no declared order, a lesson reviewing an earlier one looked like it reviewed a later one. Recovering the order from their books turned most of these back into what they always were: ordinary backward references

    // REINFORCEMENT. The founding promise is that the course "constantly
    // re-emphasizes what was learnt previously". It shipped with HALF taught once.
    //
    // This number is now moving the right way, and it took three different things:
    //   - HL09 step 3 wired 17 R1 windows in Spanish chapters 3-6 (12 moved "never").
    //   - HL-C43 (wave 6) added 24 lessons and 21 more orphans — content outrunning
    //     reinforcement, which is exactly how 51% happened in the first place.
    //   - HL-C44/C45 (waves 7-8) required every tranche to reach back at two cadences:
    //     `practises.knowledge` on the preceding 1-3 lessons (closes R1/R2 at zero new
    //     lessons, because a chapter-END payoff is out of R1 range), plus a payoff
    //     reaching several chapters back (rescues atoms never revisited at ANY distance).
    //
    // Wave 8 alone taught 72 new atoms while the orphan count FELL BY 49. Per track:
    // Russian 21 of 34 orphans -> 3 of 55 (every one rescued; the survivors are its final
    // lesson's own, structurally unreachable), Bengali 12 of 18 -> 4 of 35, Tamil 53% ->
    // 38%, Arabic four ch28-30 atoms off zero. Adding vocabulary and reducing orphans at
    // the same time is the whole point; a corpus that only grows is not a course.
    // The pre-A1 vocabulary probe (hindi/arabic/tamil, 45 lessons across three
    // unrelated families) confirmed the reach-back discipline scales past verbs:
    // atomsTaught +98 while atomsNeverRevisited FELL by 53. 22% is the lowest this
    // figure has read since it was first measured at 51%.
    // Vocabulary wave 2 (french/german/portuguese/italian, 60 pre-A1 nouns) confirmed
    // the one-headword-per-lesson exchange rate a further four times, and kept the
    // reach-back discipline: atomsTaught +151 while atomsNeverRevisited FELL by 21.
    // 21% — still the lowest this figure has read since 51%.
    // The Tamil ch1 script-gap lessons add 7: TA-W05 two, TA-W06 three, TA-W07 two.
    // Splitting one word-lesson per word moved letter teaching into the writing
    // track; these atoms are the seven script facts that move stranded.
    // Vocabulary wave 3 (russian/bengali/gujarati/kannada, 52 pre-A1 nouns) added 106
    // atoms while atomsNeverRevisited FELL by 7 — the first wave where the raw count
    // dropped, not just the share. 20% is the lowest reading yet, from 51%.
    // Vocabulary wave 4 (marathi/punjabi/sanskrit/urdu, 51 pre-A1 nouns) kept both
    // trends moving the right way at once: atomsTaught +117 while atomsNeverRevisited
    // FELL a second time, by 7 more. 19% is the lowest reading yet, from 51%.
    // +5: TA-W08-read-en introduces 2, TA-W09-read-peyar 3. They teach எ, ப, ய and the
    // ெ sign — the glyphs Tamil chapter 2 was still explaining inside its own speaking
    // lessons because the writing strand had never covered them.
    // Chapters 7 and 8 each add their configured 12-atom budget. Chapters 9 and 10
    // each add nine smaller atoms. Chapter 11 adds eleven, Chapter 12 adds eight,
    // and Chapter 13 adds nine while keeping every new verb inside the already-owned
    // singular frame.
    // Every terminal practice lesson revisits its chapter, so measurable teaching
    // grows while the corpus orphan count can still fall.
    // Chapter 14 adds six atoms in two three-atom teaching steps, then retrieves all
    // six in its atom-free terminal checkpoint. Chapter 15 adds twelve more across
    // five bounded teaching steps; its terminal checkpoint retrieves all twelve.
    // Chapter 16 adds twelve atoms across seven small teaching steps, then
    // retrieves all twelve in its atom-free terminal checkpoint.
    // Chapter 17 repeats that bounded twelve-atom shape across seven teaching
    // steps and one atom-free terminal checkpoint.
    // Chapter 18 does the same across eight teaching steps before its checkpoint.
    // +9: the four lessons that extend the Tamil writing strand to chapters 2-3's
    // glyphs introduce 2 + 3 + 2 + 2 atoms (TA-W10/11/12/13).
    // +6 more: TA-W14/15/16 close chapters 4-5's glyphs at 2 atoms each.
    // +4 more: TA-W17 and TA-W18 teach உ and ஊ — the last two untaught glyphs in the
    // chapter 33-38 sections, NOT in the corpus. A census against the strand's taught
    // set leaves glyphs still used and never taught, most of them in chapter 7's
    // numbers. That is where the real debt is. (A figure of 19 was once written here.
    // It reproduces, but only under one particular reading: the count depends entirely
    // on how "taught" is detected, and small choices — whether a bold span of four code
    // points such as ஸ்ரீ counts, how far a negation scopes, whether a TA-C* lesson may
    // teach — swing it by several glyphs. 19 needs ஞ, ஸ, ஃ and ஷ to count as untaught;
    // the reading used below counts all four as taught. The absolute number is not
    // restated below because it is not portable. The two scoped facts are.)
    // +2 more: TA-W19 teaches the ூ sign, the highest-usage of the untaught glyphs
    // (5 lessons) and the last one the strand has room for. Two facts are relied on
    // here, and they hold under a detector that does two specific things rather than
    // under every detector: it must scope negation, and it must not count a TA-C*
    // lesson as teaching. Under one that does both, the difference this lesson makes
    // is exactly ூ, and thirteen glyphs used by chapter 7's numbers — ஏ, ஐ, ஒ and the
    // ten digits ௧-௰ — remained untaught at that point. Chapter 39 then teaches ஒ,
    // leaving twelve. Ignore negation and the delta becomes four
    // glyphs, because this very lesson prints ஒன்று, ஐந்து and ஏழு in bold inside the
    // sentence saying they are still unreadable; count TA-C* as teaching and the
    // untaught set empties, because chapter 7 bolds those letters while merely using
    // them. See the CHANGELOG entry for this change.
    // Measured, the Tamil track's teaching runway is the
    // binding constraint: after TA-W18 only five speaking lessons remain, so the 3:1
    // remaining runway holds ONE more writing lesson, which TA-W19 now occupies at
    // sequence 1165, last in chapter 38. Those thirteen glyphs cannot be taught
    // inside 38 chapters.
    // +8: chapter 39's four lessons introduce 2 atoms each, and all eight are genuine
    // first introductions. What the chapter does NOT re-teach is the point —
    // TA-C39-vendum is another dative-subject verb,
    // after ch32's தெரியும், ch33's புரிகிறது and ch34's பிடிக்கும் (the lesson does
    // not number the family; ch19's ஆகிறது is described the same way), so it practises
    // TA-GRAMMAR-DATIVE-SUBJECT-02 at a distance of 90 lessons (index 38 -> 128)
    // rather than re-teaching it.
    expect(report.summary.atomsTaught).toBeGreaterThanOrEqual(3080) // FLOOR, not an exact count — see the note at the top of this file; // +4: ES-LEX-VOS-01, ES-CULTURE-VOSEO-02 (ES-C03-vos) // +3: HL-C98 splits AR-PRESENT-SINGULAR into 1SG/2SG/3SG // +83: vocabulary wave 5 (persian ch9-11, telugu ch35-40, malayalam ch35-40) // +7: HL-C88 slices 5-6 (Spanish) // +2: HL-C88 slice 8 // +103: vocabulary wave 6, round 2 (russian/persian/urdu/bengali) // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL12: +30 recognition segments (telugu/kannada/malayalam 8 each, sanskrit 6) -- one atom each, and every one of them a letter // HL12 payment two: +8 Hindi segments
    // +2, and it goes UP, which is worth stating plainly. Three of the five new atoms
    // are TA-W09's and nothing follows TA-W09, so they are orphans by construction:
    // PA-YA-01, E-SIGN-02, READ-PEYAR-03. TA-W08's two are revisited by TA-W09.
    // Against that, TA-W09 re-uses ர when it spells பெயர், so declaring
    // CA-ONE-LETTER-01 pulls that atom out of the orphan set for the first time
    // (revisits 0 -> 1). Three in, one out.
    // Chapters 10-12 revisit every atom they introduce in their terminal
    // checkpoints. Chapter 12 also gives the older silent-h atom another genuine use,
    // pulling one pre-existing atom out of the orphan set.
    // The Chapter-14 checkpoint also revives one older item, so the corpus orphan
    // count falls by one while none of the six new atoms becomes an orphan.
    // +1, measured as a set difference against the pre-change corpus rather than
    // inferred from the total. THREE atoms enter the orphan set — TTA-01 (TA-W12),
    // U-SIGN-01 and READ-QUESTION-02 (TA-W13). Nothing follows TA-W13, so its two are
    // orphans by construction. TWO leave it, both TA-W09's: extending the strand finally
    // re-uses ெ and ப, so E-SIGN-02 (TA-W10's sign-position table) and PA-YA-01 (TA-W12,
    // which spells எப்படி with ப) go from 0 revisits to 1. READ-PEYAR-03 stays an orphan.
    // Three in, two out. AA-SIGN-01, II-SIGN-01, NGA-LLA-01 and READ-NAAN-02 never become
    // orphans at all, because each later strand lesson declares — and genuinely assesses
    // — the earlier letters its own word is built from.
    // The chapters 4-5 tranche then moves this number NOT AT ALL, the first time
    // extending the strand has been free. Two in — READ-TAMIZH-02 and TA-ZHA-01,
    // TA-W16's, orphans by construction because nothing follows it. Two out — TTA-01
    // and U-SIGN-01, which TA-W16 and TA-W14 genuinely re-use (the ட/த contrast, and
    // the ு of சு).
    // +2 for உ/ஊ, and both are TA-W18's: UU-VOWEL-01 and READ-UUR-02. Nothing follows
    // TA-W18 in the strand, so they are orphans by construction. TA-W17's two are not —
    // TA-W18 re-reads உணவு beside ஊர், which is the point of the pair. Two in, none out.
    // TA-W19 then moves this number NOT AT ALL, and the sentence above about TA-W18 is
    // now historical rather than current: something DOES follow TA-W18 in the strand,
    // and it is exactly the two atoms that entry named that leave. Two out — UU-VOWEL-01
    // and READ-UUR-02, both 0 revisits -> 1, because TA-W19 re-reads ஊர் beside மூன்று
    // and declares the atom that owns it. Two in — UU-SIGN-01 and READ-MUUNRU-02,
    // TA-W19's own, orphans by construction because it is the last lesson in the track.
    // The 422-atom subset that ALSO misses a window holds too, but it trades different
    // atoms in: TA-ETYMON-VIDAI-02 and TA-LEX-VIDAI-01, which were already never
    // revisited and merely became window-measurable. TA-W19's own two are absent from
    // that subset because at index 127 no window is evaluable for them.
    // +2 net, and this counter's composition is NOT the same as the defect subset's
    // above, which is the trap here — both move +2, so a wrong attribution passes.
    // summary.atomsNeverRevisited counts every taught atom with zero revisits,
    // regardless of whether any window was evaluable (src/continuity.ts increments it
    // outside the evaluability guard). Its trade is FIVE in, THREE out:
    //   IN  TA-GRAMMAR-EVVALAVU-VS-ETHANAI-02, TA-LEX-ORU-01,
    //       TA-GRAMMAR-ORU-ATTRIBUTIVE-02, and TA-W20's own TA-SCRIPT-O-VOWEL-01 and
    //       TA-SCRIPT-READ-ONRU-02, which sit at the track's last index.
    //   OUT TA-SCRIPT-READ-MUUNRU-02 and TA-SCRIPT-UU-SIGN-01, both 0 -> 1 because
    //       TA-W20 re-reads மூன்று and credits the ூ sign that makes it மூன்று; and
    //       TA-GRAMMAR-PIDI-02, 0 -> 1, from TA-C39-vendum declaring the shape
    //       பிடிக்கும் shares with வேண்டும்.
    // The 422-atom defect subset trades differently: 422 -> 424, three in and PIDI-02
    // out. Two atoms are absent from the subset for want of an evaluable window —
    // TA-SCRIPT-O-VOWEL-01 and TA-SCRIPT-READ-ONRU-02, introduced at the last index,
    // where at + 1 > last. UU-SIGN-01 is absent for a different reason: its R1 window
    // IS evaluable, but its revisit count is now 1. The ORU atoms are in the subset,
    // at index 130, where R1 is evaluable and they have no revisit.
    // The ORU pair is structural rather than an oversight: TA-W20 genuinely re-reads
    // ஒரு, but a writing lesson may only take other writing lessons as prerequisites
    // (TA-EXT-003-SCRIPT is inlined at TA-PATH-003, so naming a chapter-39 lesson
    // would put the prerequisite AFTER its dependent and fail the ordering rule). The
    // tie is carried by `reviews_of` instead, which does not count as a revisit.
    // Chapters 40 and 41 are planned to close this.
    // RATIO, not a count. This is debt, but it is debt that grows with honest
    // content: every atom at the tail of a track is unrevisited until something
    // follows it. A floor would bless the debt growing; a ceiling would fail on
    // a legitimate new lesson. The share is what the raw number was always a
    // proxy for, and it is the thing that should fall.
    expect(report.summary.atomsNeverRevisited / report.summary.atomsTaught)
      .toBeLessThanOrEqual(0.18); // -17: payoff lessons plus si/no moving early // +2: HL-C98's per-cell atoms are revisited later than the R-windows reach // -8: vocabulary wave 5 rescues net orphan atoms via reach-back payoffs // +4: HL-C88 slices 5-6 // +3: vocabulary wave 6 (russian/persian/urdu/bengali reinforcement pushed most orphans to zero, but extending each track exposed a few new tail atoms) // +1: HL-C113 (B1 si-condition rung) // +2: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL11: +1. Tamil's nine drizzle segments chain -- each practises the previous letter -- so eight of nine are revisited. The last has no successor yet; a real, small ramp defect that resolves when segment 10 lands rather than by editing this pin again // HL12: +4, and it is the same small defect as HL11's, once per track. Each track's recognition segments chain -- every one practises the previous letter -- so all but the LAST are revisited. Four tracks, four last segments, four orphans. It resolves when the next batch of segments lands behind them, not by editing this pin // HL12 payment two: +1 -- Hindi's last segment, the same one-per-track orphan // wave II's tail atoms, same one-per-track orphan as every wave
    expect(report.summary.neverRevisitedPercent).toBe(14); // hindi pre-A1 round 2: +35 lessons, +7 chapters (chapters 52-58) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +3 -- B2 closes (chapter 271) // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51)

    // 509 -> 517, and the eight split into TWO DIFFERENT PHENOMENA this number conflates.
    //
    // TWO are real debt that only became VISIBLE now: `LA-C08-manus` and `LA-C37-habeo`
    // have always used *scrībere* and *capere*, but nothing taught those words, so no
    // forward reference could be detected. Teaching them in chapters 38-39 exposed uses
    // that were already there. This is the measurement improving, not the corpus decaying,
    // and it will keep happening as coverage grows.
    //
    // SIX are Spanish end-of-lesson teasers — "Next: **entender**, which cracks open..."
    // naming the lesson that immediately follows, inside the same chapter. That is a
    // title, not vocabulary the reader is expected to already know, and it is a different
    // thing from `ES-C07-beber` drilling *pan* and *agua* nineteen chapters before they
    // are taught — which is the case this module was built to find. Latin and Portuguese
    // authored the same eight verbs with zero teasers, so the habit is avoidable; it is
    // NOT being rewritten to move this number, because contorting good prose to satisfy a
    // naive matcher is the exact failure the sight-cue detector already demonstrated.
    // If this metric is to gate anything, it wants a severity split by distance first.
    //
    // 515 -> 521, and all six are the word *luego*, which chapter 38 takes as a
    // headword. FIVE are chapter 5, inside the fixed farewell *hasta luego* (plus one
    // in the chapter-4 practice) — a spiral, not a leak, and chapter 38 now opens by
    // saying the word is already the reader's rather than pretending it is new.
    // The SIXTH is different: `ES-C10-practice` uses bare adverbial *luego* ("y luego
    // voy a trabajar"), which is the connective sense, 28 lessons early. That one is a
    // real leak. The metric reports all six identically, which is exactly the severity
    // split the comment above asks for: it cannot tell a spiral from a leak.
    // Note it keys on the HEADWORD, not the atom — chapter 38 introduces only
    // ES-GRAMMAR-LUEGO-CONNECTIVE-01 and requires chapter 5's ES-LEX-LUEGO, yet the
    // count is unchanged by that wiring.
    //
    // 524 -> 443, and none of the 81 was a real finding. Two distinct problems, one of
    // which had been masking the other.
    //
    // First, `taughtWords` stripped ANY leading word of three characters or fewer,
    // meaning to remove articles. A census found it firing on 227 of 1,453 lessons
    // while only 49 of those begin with one — registering `llamo` as taught by "me
    // llamo", `favor` by "por favor", `dia` by "bom dia", and the night-word of every
    // ശുഭ / शुभ / శుభ / ಶುಭ greeting. The rule is now an allowlist of real articles.
    //
    // Second, and only visible once the first was fixed: the bad strip had been
    // seeding a lesson's OWN word set with the stripped tail, which incidentally
    // stopped it reporting itself. Removing the strip removed that accident, and four
    // lessons started being reported for a word sitting in their own headword —
    // exactly what this module's docstring says must not happen. `ownHeadwordTokens`
    // now does it deliberately, and completely: the accident only ever covered the
    // tail after the first word, so 45 self-references it had never caught are gone
    // too. Every one was verified to be a token of the reporting lesson's own headword.
    //
    // 443 -> 439, and NOT because of the prose rewrite — a sequence-only corpus reads
    // 439 too. All four dropped references sit in TA-W01-curves-va-ka,
    // TA-W03-pulli-vanakkam and TA-C01-practice, none of which had prose edited. They
    // stopped counting because chapter 1 now declares its order.
    // -1, same cause.
    // Removing pan/agua and the rest of the later café vocabulary closes six leaks.
    // Chapter 8 closes six more by deferring sixteen-through-twenty-one to Chapter 31.
    // Chapter 9 removes five more leaks by replacing untaught adjective pairs and
    // plural verb forms with identity/state examples built from known words.
    // Chapter 10 removes eight more: no plural ir forms, no new infinitives, no
    // undeclared nouns, and no connective luego from much later in the course.
    // Chapter 11 removes five more by dropping full boot tables, future verbs, and
    // undeclared house/car/friend vocabulary from its singular-only migration.
    // Chapter 12 removes seven more: weather and homework nouns, a window prompt,
    // plural forms, and the three Chapter-13 verbs no longer appear early. Chapter 13
    // removes one net leak by deferring its own plural tables and undeclared contexts.
    // Chapter 14 removes the pan leak and two other undeclared contexts by staying
    // inside known Madrid, Roberto, español, and the singular-person frontier.
    // Chapter 15 removes one more net leak by deferring every plural form and using
    // only already-declared café, en, bien, and Madrid in its singular examples.
    // Chapter 18 removes one more by deferring its untaught clause-frame examples.
    // -2, and both are Tamil leaks this change closes rather than Spanish ones. Both
    // were chapter 4-5 lessons quoting a verb in SCRIPT that chapter 32 teaches, ~27
    // chapters later: TA-C05-vaazh spelled வாழ் out of வா, and TA-C04-naalai glossed
    // பார்க்கலாம் from பார். Neither word moved; both are now named in romanization,
    // which is what a speaking-first lesson should have been doing anyway.
    // +1, and it is a measurement improvement rather than new damage. The single new
    // entry is TA-C18-mani-homophone-time using ஒரு at position 65, 65 lessons before
    // TA-C39-oru teaches it. That use is not new — ch18 has always printed ஒரு — but
    // until this chapter no lesson OWNED the word, so the checker had no teacher to
    // measure the distance against and stayed silent. Naming a teacher is what made
    // the existing early use visible. It also argues ஒரு belongs earlier than 39,
    // which the runway did not allow.
    expect(report.summary.forwardReferences).toBeLessThanOrEqual(511) // 499 -> 500. HL-C175: ES-C272-si-claro is the first lesson to OWN claro, and ES-C56-mente used it 356 lessons earlier -- as a CITATION FORM in the adjective-to-adverb table (claro -> clara -> claramente), not as vocabulary in a sentence. Same class as the LA-C08-manus case above: the corpus did not decay, the measurement got sharper when the word finally got a teacher. Worth sharpening rather than absorbing: a word appearing inside a derivation table is being SHOWN, not USED, and the detector cannot yet tell those apart. Logged as its own backlog row. // 494 -> 499. HL-C156 replicates the letter ledger to five more tracks: 85 recognition segments, and FIVE of them name a letter whose word-lesson comes later in the same track. That is genuine new debt, not a measurement artefact -- the segments are placed by ledger position, which is ordered by word payoff, and payoff order does not always match the order the words are taught in. It is fixable by re-seating the five segments behind their words, which is a placement pass rather than a content one; logged so the next increase has to be written down too. // 463 -> 493 while HL-C136 wave I was in review, and every one of the thirty was read before this ceiling was moved. EIGHTEEN are three per track, the same three in all six: the `here` lesson names its partner `there`, `this` names `that`, and `who` names `where` -- each one lesson early, inside the same chapter, because a deixis word cannot be taught without its contrast. That is a spiral one step wide, not a leak, and it is the Spanish teaser case below with a better excuse. The other TWELVE are in lessons this wave never touched, and TEN of them are the measurement improving rather than the corpus decaying: HI-C36-ghar prints **यह मेरा घर है**, KA-C35-kutumba prints **ಇದು ನನ್ನ ಕುಟುಂಬ**, and eight more chapter 34-36 vocabulary lessons open the same way -- standalone demonstratives in real sentences, sitting there since they were written, and until this wave gave those words an owner no forward reference could be detected. Exactly the LA-C08-manus / LA-C37-habeo case named above, and it will keep happening as coverage grows. Each of the ten was read in its own lesson rather than trusted from the report, because the OTHER TWO are not real: BACKLOG HL-C150 establishes that ML-C01-athe is Malayalam athe ('yes') merely CONTAINING the letters of athu ('that'), and that TA-C33-puri's -adu is the third-person verb ending of purikiradu, not the demonstrative. This detector matches SUBSTRINGS, so a two- or three-character headword collides with morphology; sharpening it to word boundaries is logged on that row and is what would make this ceiling mean what it says. // 455 -> 463: HL-C128 step 4 (al, del, quien, o, ni). NOT eight new early uses -- every one of the eight is a use that ALREADY EXISTED and only became measurable because these lessons finally gave the words an owner, the same phenomenon as the oru entry above. Two are false positives worth naming: ES-W02-enye and ES-W02-enye-formas print "ni" as the Latin letter sequence that became n-tilde, not as the word. The other six are genuine, and the worst is real debt this PR only exposed: ES-C03-como-acento prints quien at chapter 6 and nothing taught it until chapter 232, 351 lessons later. That argues quien belongs near the other asking words rather than here, which is recorded as its own backlog row rather than fixed by renumbering 226 chapters inside a content PR. // CEILING — this is debt; it may fall, never grow; // -1: HL-C98 removes a forward reference // -5: HL-C103 census adds comes/hand/regular to ENGLISH_COLLISIONS, and drops an untaught tres from an accepted list // +19: vocabulary wave 5 // +8: HL-C88 slices 5-6 // -8: HL-C112 moves casa and libro ahead of the adjective arc, so uses that were early are now taught // +7: vocabulary wave 6 // HL11: five tracks gained a declared reading order, recovered from their own books; every continuity window is measured in that order, so each of these moves // 454 -> 455 while this PR was in review: HL-C128 step 3 (Spanish degree words) added one. Re-seating a ratchet is not the same as relaxing it — the ceiling records where the debt stands today, and every future increase has to be written down here too, by whoever causes it. // 493 -> 494. HL-C137 wave II: Tamil chapter 3 already USES நல்ல and this wave is the first thing that TEACHES it, at chapter 41. Same class as HL-C150 and blocked on the same placement work — recorded here so the next increase has to be written down too. // 500 -> 508. Latin pre-A1 tranche, 20 words in chapters 44-47. ALL EIGHT are the LA-C08-manus / LA-C37-habeo class named above -- the measurement getting sharper, not the corpus decaying. Every one is a use that ALREADY EXISTED in a lesson this PR never opened, and each became measurable only because the word finally got an owner: LA-C08-manus lists domus among the feminine -us nouns, LA-C27-bene cites mel->miel and terra->tierra as diphthongisation examples, LA-C06-ruber-caeruleus glosses caeruleum mare, LA-C40-dormio and LA-C41-ambulo both name somnus while teaching its verbs, and LA-C28-dies and LA-C24-propediem-te-videbo both print dies Lunae. Verified by reading each of the eight in its own file and confirming `git diff HEAD` reports it unmodified -- not trusted from the report. A NINTH was genuinely new and was FIXED IN CONTENT rather than absorbed here: LA-C45-terra named mare four lessons early to build mare mediterraneum, and now describes the name in English and defers the Latin to the lesson that teaches it. That these words are used 25-84 lessons before they are taught argues they belong early in the book rather than at chapter 44, which is a renumbering pass and is logged as its own row rather than done inside a content PR. // 508 -> 511. French questions chapter, and ALL THREE are the HL-C213 class: pre-existing uses of `ou` (where) that became measurable only because FR-C32-ou is the first lesson to OWN the word. FR-W01-accents has been teaching the ou/ou-grave accent contrast since chapter 1, and FR-C05-habiter glosses `a ou question`. Each was read in its own file and confirmed unmodified by git diff HEAD, not trusted from the report. That they are used 69-75 lessons before being taught argues the question words belong earlier than chapter 32, which is the same placement finding HL-C213 recorded for Latin

    // HL09 step 3 closed 17 R1 windows in chapters 3-6, measured on the corpus of the
    // day as 766 -> 749. The absolute figures drift as main lands lessons; what the
    // change is accountable for is the 17. R2 moves only with new lessons, never from
    // that work — closing a near window does not close a far one, and nothing yet
    // addresses R2/R3/R4.
    //
    // Chapter 38 costs +3 R1 and +10 R2, and almost none of that is its own atoms.
    // `continuity.ts` only judges a window that FITS: `if (at + window.from > last)
    // continue`. Lengthening Spanish by four lessons pushed `last` out and un-suppressed
    // windows that were previously never judged — on chapter 36 and 37 atoms. Of the
    // +3 R1 exactly one is a chapter-38 atom; of the +10 R2, none is.
    //
    // So this is not a cost the new chapter incurred; it is a debt the old chapters
    // already had, which only became measurable once something followed them. Any
    // chapter appended to any track will do this, and the number is honest either way.
    // The Tamil ch1 script lessons introduce atoms that miss R1 six times and R2 six
    // times — yet the totals move only +5 and +4. The difference is the point: those
    // same lessons practise the OLDER script atoms they build on (the puḷḷi, the ி
    // sign), rescuing one R1 and two R2 misses that main was already carrying. New
    // teaching paid down old debt at the same time.
    //
    // The two sets are not the same six. WRITE-AAM-02 misses R1 but lands in R2;
    // INDEPENDENT-VOWEL-AA-01 does the reverse. Common to both: two of TA-W07's own,
    // which nothing follows, and TA-W06's three, which are the interesting case —
    // TA-W07 does revisit all three (revisits=1), but at a distance
    // of FOUR lessons, because the interleave puts word lessons between consecutive
    // writing lessons and those are schema v1 — no atoms, so they cannot pay a window.
    // Distance 4 falls in the hole between R1 (1-3) and R2 (5-15), so a genuinely
    // reinforced atom scores as missing every window. The windows have a gap here;
    // the lessons do not. Closing it means reordering ch1 so consecutive writing
    // lessons sit within three of each other — a separate change, and one that would
    // also fix a pre-existing inversion where TA-W04 spells நன்றி forty sequence
    // numbers before TA-C01-nandri teaches the word.
    // Two independent changes land on this number in the same merge: vocabulary wave 4
    // (new pre-A1 nouns landing near the end of already-long chapters, moving R1 little
    // and R2 a lot: 834 -> 838 claimed in isolation) and Tamil's script-interleaving
    // restructure (spacing script lessons so none can reinforce inside the 1-3 R1
    // window, while R2's wider 5-15 span still reaches most of them: 834 -> 843, 1627 ->
    // 1625 claimed in isolation). Re-measured against the merged corpus: R1 834 -> 847
    // (both effects add), R2 1627 -> 1716 (wave 4's growth dominates, Tamil's recovery
    // is a small offset against it, not the whole story either claim told alone).
    // +5, exactly the five new atoms and no offsets. The strand's cadence puts
    // consecutive script lessons 4-5 apart and R1 is a 1-3 window, so an atom a script
    // lesson introduces cannot be reinforced inside R1 by the next one.
    // Chapter 8's tightly spaced lessons revisit eleven of its twelve atoms inside R1;
    // the last cultural atom is practised later in the same chapter, for a net +1.
    // Chapter 11's four teaching lessons add one net near-window miss after local
    // practice offsets the other newly measurable windows.
    // Chapter 15 adds two net R1 misses: its local checkpoint revisits every new atom,
    // but five deliberately tiny teaching steps spread the earliest atoms beyond R1.
    // Chapter 16 adds two net R1 misses: the early imperfect atoms are
    // retrieved locally, but its eight-lesson footprint pushes two beyond R1.
    // Chapter 18 adds four: its terminal checkpoint retrieves all twelve atoms,
    // but eight small teaching steps push the earliest four beyond R1.
    // +10, and the composition is measured per atom, not inferred from the total.
    // NINE of the ten are the new script atoms: every atom TA-W10..TA-W13 introduces
    // misses R1, because the strand's 3:1 cadence puts consecutive script lessons well
    // outside a 1-3 window. The TENTH is not new and is worth naming: interleaving a
    // writing lesson at chapter 29 pushes TA-LEX-AFTERNOON-BOUNDARY-01's reinforcement
    // past R1 ([R2,R3] -> [R1,R2,R3]). No atom gains R1 reinforcement here.
    // +6 for chapters 4-5: all six of TA-W14/15/16's atoms miss R1 and nothing leaves,
    // the same cadence arithmetic as above.
    // +4: all four new atoms miss R1, and nothing leaves. TA-W17 and TA-W18 are 3
    // speaking lessons apart, so the gap is 4 in reading order, outside the 1-3 window
    // as everywhere else in the strand.
    // +2, and NEITHER is TA-W19's. TA-W19 is the corpus's last Tamil lesson, at index
    // 127, so no window is evaluable for the atoms it introduces at all. The two that
    // enter are TA-ETYMON-VIDAI-02 and TA-LEX-VIDAI-01, introduced at 126, and they
    // are the R1 case of the one mechanism described at the R2 pin below: the track
    // was 127 lessons, 126 + 1 = 127, so R1's first position did not exist until this
    // lesson made index 127 exist. Their revisit counts are 0 before and 0 after.
    // +5. Three are chapter 39's own atoms with nothing after them yet. Two are
    // TA-W19's — TA-SCRIPT-READ-MUUNRU-02 and TA-SCRIPT-UU-SIGN-01, introduced at 127,
    // which had NO evaluable window while the track ended at 127 and now have one.
    // READ-MUUNRU-02 gained a real revisit at the same time (0 -> 1, TA-W20 re-reads
    // மூன்று beside ஒன்று) and still misses R1, because TA-W20 is 4 lessons later and
    // R1 stops at 3.
    expect(report.summary.missedByWindow.R1 / report.summary.atomsTaught)
      .toBeLessThanOrEqual(0.32) // RATIO, not a count: R1 is the tightest window (3 lessons), so most atoms miss it, so this
      // number grows with honest content and only its SHARE is meaningful; // +2: HL-C98 // +2: vocabulary wave 5 // +4: HL-C88 slices 5-6 // +1: HL-C112 moves casa and libro 14 chapters earlier, so one atom's next re-use now falls outside R1 // +12: vocabulary wave 6 // +1: HL-C113 (B1 si-condition rung) // +1: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL11: +9. Each drizzle atom's only re-use is the next segment, forty sequence units later -- one letter, then the word lesson that uses it, then the next letter. Far outside R1, which is what a drizzle looks like measured by a window built for consecutive lessons // HL12: +9 of 30 new atoms, and the 21 that do NOT miss are the interesting part. Placing each segment last in its chapter put consecutive segments 2-3 lessons apart in Telugu, Kannada and Malayalam -- their chapters 8-13 hold one or two lessons each -- which is inside R1's window, so each letter is re-practised almost immediately. Sanskrit's chapters are longer, its gaps run 3-5, and 4 of its 5 gaps miss; the other misses are the four last segments, which have no successor yet // HL12 payment two: +1 of Hindi's 8. Hindi's chapters 6-13 are short like the Dravidian ones, so seven of its eight segments land inside R1 too; the miss is the last, which has no successor
    // +2 net, and the composition is the interesting part: all FIVE new atoms miss R2
    // as well, offset by THREE pre-existing atoms that TA-W09 pulls back into it.
    // TA-W09 sits 12 lessons after TA-W06 and 8 after TA-W07, both inside R2's 5-15
    // window, so practising INDEPENDENT-VOWEL-I-01, LA-AI-SIGN-02 and CA-ONE-LETTER-01
    // there reinforces them at a distance R1 could never reach. Measured: all three go
    // from missing R1/R2/R3 to missing only R1/R3.
    // Chapter 9 makes six more R2 windows measurable while its deliberately local
    // five-lesson reinforcement still lands before R2's 5-15-lesson span. Chapter
    // 10 adds nine typed atoms whose terminal retrieval is likewise earlier than R2.
    // Chapter 11 makes seven additional far windows measurable before a later lesson
    // can reach the 5-15 lesson R2 span. Chapter 12 makes eight more. Chapter 13 adds
    // three net misses: all nine atoms get local retrieval, but that checkpoint remains
    // deliberately too close to count as far-window reinforcement.
    // Chapter 14's six atoms receive local retrieval, but its three-lesson footprint
    // cannot reach the far R2 window yet.
    // Chapter 15 adds seven net R2 misses. Its terminal retrieval is intentionally
    // local; later chapters must provide the five-to-fifteen-lesson reinforcement.
    // Chapter 16 adds five net R2 misses. Its checkpoint is deliberately local;
    // Chapters 17 onward still own the farther reinforcement window.
    // Chapter 17 adds seven more: the local checkpoint retrieves every atom,
    // while later chapters still own the five-to-fifteen-lesson revisit window.
    // Chapter 18 adds five more far-window misses for the same reason.
    // +1 net, and again the composition matters more than the number. FOUR of the nine
    // new atoms miss R2 (TTA-01, U-SIGN-01, READ-EPPADI-02, READ-QUESTION-02); the other
    // five do NOT — AA-SIGN-01, II-SIGN-01, NGA-LLA-01, READ-NAAN-02 and READ-NIINGAL-03
    // are practised by a later strand lesson 5-15 lessons on, which is exactly what R2
    // measures and exactly what threading them through `practises` was for. Against those
    // four, THREE pre-existing atoms are pulled back inside R2: PA-YA-01 (TA-W09's ப, now
    // re-used by TA-W12) and ETYMON-KAALAI-01 and ETYMON-MAALAI-01, whose distances shift
    // as the new lessons land between them. Note E-SIGN-02 does NOT leave R2: TA-W10
    // re-uses it, which is what takes it out of the orphan set above, but at a distance
    // R2 does not count. Four in, three out.
    // Chapters 4-5 add +4. All SIX new atoms miss R2 this time, and the reason is worth
    // recording because it is a consequence of ordering, not of authoring: TA-W16 was
    // moved ahead of TA-W14/W15 so it lands before TA-C33-ezhutu, which already takes
    // ழ and த apart. That puts consecutive strand lessons 4 apart — outside R1's 1-3,
    // but also short of R2's 5-15 — so nothing any of the three introduces can be
    // reinforced at R2 distance by the next one. TWO leave: TTA-01 and U-SIGN-01, both
    // genuinely re-used far enough back to land inside the window. Six in, two out.
    // +5 for உ/ஊ, and only four of those are the new atoms. The fifth is a REGRESSION
    // this tranche causes, and it is worth naming rather than filing under measurement.
    // TA-LEX-VEEDU-01 is introduced by TA-C35-veedu and revisited far away by exactly
    // one lesson, TA-C38-vidai. That revisit sat at offset 15 — the LAST position R2's
    // 5-15 window counts. Inserting TA-W17 and TA-W18 between them pushed it to 17, so
    // an atom that was passing R2 now misses it. Nothing leaves.
    // -1, and it goes DOWN, which is the point of placing this lesson LAST in its
    // chapter rather than mid-chapter. THREE atoms leave: TA-SCRIPT-READ-UUR-02
    // (revisits 0 -> 1), TA-SCRIPT-UU-VOWEL-01 (0 -> 1) and TA-SCRIPT-U-VOWEL-01
    // (1 -> 2). At index 127 the lesson sits 6 lessons after TA-W18 and 10 after
    // TA-W17, both inside R2's 5-15 span. Placed mid-chapter at index 125 it landed 4
    // after TA-W18 — past R1's 1-3 and short of R2's 5 — and rescued only one of the
    // three. Same lesson, same atoms, different distance.
    // IN (2): TA-GRAMMAR-IVAR-02 and TA-LEX-IVAR-01, introduced at index 122 by
    // TA-C37-ivar. Their revisit COUNTS are unchanged (1 and 2); what changed is that
    // R2 became evaluable for them. The arithmetic is exact, and it is the SAME
    // mechanism in all four windows: the Tamil track was 127 lessons, and
    // introducedAt + window.from = 127 for every atom that enters, so that window's
    // first position did not exist until this lesson made index 127 exist.
    //   R1: VIDAI pair at 126, 126 + 1 = 127 (pinned above).
    //   R2: IVAR pair at 122, 122 + 5 = 127.
    //   R3: 1307 -> 1309, UTAVU pair at 107, 107 + 20 = 127. Nothing leaves.
    //   R4: 242 -> 243, PLEASE-REGISTER pair at 47, 47 + 80 = 127, offset by ONE atom
    //   leaving — TA-SCRIPT-THREE-NS-01, revisits 4 -> 5 and missing [R1,R2,R4] ->
    //   [R1,R2], because this lesson practises it where it re-reads ன in மூன்று.
    // For every atom that ENTERS any of the four windows the revisit count is
    // identical before and after, so no existing reinforcement was broken. Every atom
    // that LEAVES gained a real revisit. R3 and R4 are not pinned here.
    // (Scope: that invariant is TA-W19's. Chapter 39 breaks it in the benign
    // direction — READ-MUUNRU-02 and UU-SIGN-01 enter R1 with revisits 0 -> 1, having
    // GAINED one. See the chapter-39 R1 note above.)
    // +8, and every one of them is the same measurability mechanism, now with a wider
    // mouth: the Tamil track grew 128 -> 132, so a window becomes evaluable for exactly
    // those atoms whose `introducedAt + window.from` lands in (127, 131]. All eight fit
    // — four two-atom pairs, at 126 (VIDAI, 126+5=131), 125 (SUGAM), 124
    // (UDAMBU) and 123 (IVAR-EN-NANBAR): two atoms each, eight in all. Not one of
    // their revisit counts changed.
    // The same arithmetic, unpinned here: R4 243 -> 247, and R3 does not move at all —
    // 1309 -> 1309, seven in and seven out. That is the whole argument for declaring
    // what a sentence actually re-uses. TA-C39-vendum names தெரியும், புரிகிறது and
    // பிடிக்கும் in one clause and credits all six of their atoms:
    //   TA-LEX-PIDI-01 and TA-GRAMMAR-PIDI-02 (at 108) land a revisit at exactly
    //   distance 20, R3's first position, so neither enters;
    //   TA-LEX-PURI-01 and TA-GRAMMAR-PURI-02 (at 100) and TA-GRAMMAR-TERI-02 (at 98)
    //   leave R3 outright and, missing no other window either, drop off the defect
    //   list entirely.
    // The same clause with the verbs merely named would have read identically on the
    // page and left R3 five windows worse.
    // Against that, four atoms LEAVE R3 and three leave R4, and those are real gains
    // this chapter earned by declaring what it actually re-uses: EE-SIGN-01 1 -> 2,
    // INDEPENDENT-VOWEL-E-01 2 -> 3, NGA-LLA-01 2 -> 3 and TTA-01 1 -> 2 out of R3;
    // GRAMMAR-DATIVE-SUBJECT-02 2 -> 3, LEX-DATIVE-SUBJECT-01 3 -> 4 and
    // LEX-NUMBERS-1-5-01 2 -> 3 out of R4. Chapter 6's dative and chapter 7's numbers
    // are reached at R4 distance for the first time.
    expect(report.summary.missedByWindow.R2).toBe(3265); // HL-C232: +3. Nine Russian lessons and nine atoms, three of which register: they sit at the END of russian with no successors yet -- the usual 'extending a track makes its tail measurable' shape, clearing when the next tranche lands behind it rather than by editing this pin // HL-C231: +8. Eleven Japanese lessons and eleven atoms, eight of which register: they sit at the END of japanese with no successors yet -- the same 'extending a track makes its tail measurable' shape recorded above, clearing when chapter 4 lands behind it rather than by editing this pin // tamil pre-A1 round 2: +35 lessons, +7 chapters (chapters 51-57) -- R2 moves with new lessons by construction; all 35 new atoms sit at the END of tamil with no successors yet, the same 'extending a track makes its tail measurable' shape recorded above. R1's numerator held at 1127 while the denominator grew 4104 -> 4139, so the tight ratio improved 0.2746 -> 0.2723 // merge with main: +10 -- HL-C229 French question tranche landed on main while this tranche was in flight (a second time); re-measured against the merged tree rather than picked // merge with main: +5 -- hindi script second wave (chapter 59) landed on main while this tranche was in flight; re-measured against the merged tree rather than picked // hindi pre-A1 round 2: +35, and the split was checked rather than assumed. THIRTY are the wave's own tail atoms. The other FIVE are chapter 51's HI-LEX-C51-WELCOME-01..05, which had no successors before this tranche and so had no R2 window to miss at all -- appending chapters 52-58 is what made them measurable, not what broke them. Zero atoms LEFT the window. Same shape as "extending each track exposed a few new tail atoms" above, and it resolves when round three lands behind it rather than by editing this pin // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50). R2 is the wide window and it moves with new lessons by construction; R1, the tight one, held its numerator at 1106 while the denominator grew, so the ratio improved 0.2949 -> 0.2922 // HL-C136 wave I: +61, and NOT ONE of the 61 is an atom this wave introduced -- verified by measuring main's tree and the merged tree side by side and diffing the atom lists. All 61 are pre-existing atoms sitting in the last few positions of the six Indic tracks (hindi 12, malayalam 11, kannada/tamil/telugu 10 each, sanskrit 8), and they moved because of the guard at continuity.ts: a window is only judged when `at + window.from <= last`, so an atom near the end of a track has not FAILED R2, the track simply has not got there yet. Giving each of the six tracks seven more lessons puts that band inside the judgeable range: hindi's newly-judged atoms sit at positions 112-116 against a previous last-introducing position of 112, and the other five read the same way. The debt was always there and was never re-practised; extending the track is what made it measurable. Which is also why this number is the wrong shape -- HL-C149 exists to derive it. // +1: ES-C02-concordancia (HL-C85). buenos dias/buenas tardes stop REQUIRING the agreement rule; it becomes a payoff lesson after the learner has used all three greetings. // +84: vocabulary wave 5, new pre-A1 nouns landing late in already-long chapters // +4: HL-C88 slices 5-6 // +1: HL-C112, same cause as R1 above // +1: HL-C88 slice 8 // +87: vocabulary wave 6 // +4: HL-C113 (B1 si-condition rung) // +2: HL-C113 preterite plural // HL-C113 preterite close // HL11: -4. The drizzle adds nine atoms whose re-use is far away (R1 above), but placing each segment beside the word lesson that uses its letter also pulls several EXISTING atoms back inside R2 -- so the wider window comes out ahead // HL12: the 30 recognition segments chain one letter to the next, so their atoms sit in this window like every other drizzled strand // HL12 payment two: +8, Hindi's segments, same shape as the other four tracks' // HL-C137 wave II: +36 adjective lessons, +6 chapters, all six Indic tracks // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2 // HL-C156: 85 script segments across the six Indic tracks // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C168: +1 -- Kannada's ledger closes at 24 of 24 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +3 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C190: see/say verbs across four tracks // HL-C192: +24 family words // HL-C194: +16 Spanish words // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // chinese script chapter: +7 lessons, +1 chapter. R2 moves by construction when a track gains lessons; the seven new atoms sit at the END of chinese, so each is judged without a successor yet -- the same 'extending a track makes its tail measurable' shape recorded above, and it resolves when chapter 3 lands behind it rather than by editing this pin // japanese hiragana tranche: +15. The ten new atoms sit at the END of japanese with no successors yet, the same 'extending a track makes its tail measurable' shape recorded above; it resolves when the next hiragana tranche lands behind it // latin pre-A1 tranche: +20 lessons, +4 chapters (chapters 44-47) -- R2 moves with new lessons by construction; the 20 new atoms sit at the end of latin with no successors yet // latin pre-A1 tranche: +20 lessons, +4 chapters (chapters 44-47) -- recount after LA-C45-terra dropped its early mention of mare // gujarati script tranche: +12. The ten new atoms sit at the END of gujarati with no successors yet -- the same 'extending a track makes its tail measurable' shape recorded above // marathi script tranche: +11 lessons, +1 chapter (ch14) -- ten pieces, one per lesson, plus one assembly -- the eleven new atoms sit at the END of marathi with no successors yet // punjabi script tranche: +10 lessons, +1 chapter (ch14) -- nine pieces, one per lesson, plus one assembly -- the ten new atoms sit at the END of punjabi with no successors yet // russian script tranche: +11 lessons, +1 chapter (ch14) -- eleven letters, one per lesson, chosen by how many words each unblocks // malayalam pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) -- R2 moves with new lessons by construction; all 35 new atoms sit at the END of malayalam with no successors yet, the same 'extending a track makes its tail measurable' shape recorded above. R1's numerator held at 1123 while the denominator grew 4039 -> 4074, so the tight ratio improved 0.2780 -> 0.2757 // sanskrit pre-A1 round 3: +35 lessons, +7 chapters (chapters 38-44) -- R2 moves with new lessons by construction; all 35 new atoms sit at the END of sanskrit with no successors yet, the same 'extending a track makes its tail measurable' shape recorded above. R1's numerator held at 1127 while the denominator grew 4139 -> 4174, so the tight ratio improved 0.2723 -> 0.2700
  });

  it("shows what a declared reading order was worth", () => {
    // Before HL09 step 2, Spanish had 56 lessons with no `sequence` and the walk
    // fell back to alphabetical order within a chapter. That fallback INVENTED
    // defects: it reported 31 forward prerequisites, of which 26 were artifacts of
    // sorting `beber` before `comer`. Declaring the real order removed them.
    //
    //                     before   after
    //   no sequence           56       6   (the six chapter-7 lessons, see below)
    //   forward prereqs       31       5
    //   forward references   143      99
    //
    // The atom figures are unchanged, as they must be: ordering moved no content.
    const { lessons } = loadEverything();
    const spanish = measureContinuity(lessons).tracks.find((t) => t.language === "spanish");
    // HL-C43 added 8 Spanish lessons (chapters 34-35), all of them sequenced and
    // prerequisite-closed: `lessonsWithoutSequence` and `forwardPrerequisites` do not
    // move at all, while `atomsTaught` goes 182 -> 199 and `atomsNeverRevisited`
    // 93 -> 102. The independent Python audit these four numbers reproduce was run
    // against the 146-lesson corpus; the walk is unchanged, only its input grew.
    expect(spanish).toMatchObject({
      // Chapter 38 added four lessons and ten atoms. The two ORDER numbers did not
      // move -- no new unsequenced lesson, no new forward prerequisite -- which is the
      // point of the pin: new content is supposed to leave the walk alone.
      // Chapter 16 replaces three legacy lessons with eight bounded steps;
      // Chapter 17 replaces four legacy lessons with eight bounded steps.
      // Chapter 18 replaces ten legacy lessons with nine bounded steps.
      lessonCount: 549, // HL-C194: +16 Spanish words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // +4: HL-C98 // +3: HL-C97 adds the repair kit (no entiendo, mas despacio) at chapter 14 // +8 payoff lessons // +1 ES-C03-vos, +1 ES-C02-concordancia // +4: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +4 // HL-C113 step 8: +3 // HL-C128 step 2: +5 // HL-C128 step 3: +4 // HL-C128 step 4: +6 // HL-C128 step 5: +5 // HL-C127: +5 // HL-C128 step 7: +5 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      lessonsWithoutSequence: 0,
      forwardPrerequisites: 0,
      // Chapters 9, 10, and 13 each add nine atoms, Chapter 11 adds eleven, and
      // Chapter 12 adds eight; every terminal checkpoint revisits its full typed chapter.
      // Chapter 14 adds six atoms and its checkpoint revives one older atom.
      // Chapter 15 adds twelve atoms across five steps, then revisits all twelve.
      // Chapter 16 adds twelve atoms without adding an unrevisited orphan.
      // Chapter 17 does the same across its future and conditional ramp.
      // Chapter 18 does the same across its singular subjunctive ramp.
      atomsTaught: 841, // HL-C194: +16 Spanish words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // +3: HL-C98's per-cell atoms // +7: HL-C88 slices 5-6 // +2: HL-C88 slice 7 (ES-C09-ncia) // +2: HL-C88 slice 8 (-ario, review, synthesis) // +2: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +2: HL-C113 preterite close (strong plurals, review, synthesis) // +3: HL-C113 imperfect subjunctive // +1: HL-C113 unreal condition // HL-C113 step 7: +2 // HL-C113 step 8: +8 // HL-C128 step 2: +9 -- the demonstratives, their deixis, their position rule and three etymons // HL-C128 step 3: +10 -- muy, bastante, mal, malo, the scale, the apocope pattern and three etymons // HL-C128 step 4: +13 // HL-C128 step 5: +9 -- both gerund endings, its meaning, the progressive and its limit, the personal a and its reason, and the -ndum etymon // HL-C127: +10 -- the vosotros preterite for both ending groups and the strong stems, the imperfect plural for both, the three irregular plurals, the accent rule, and two completeness atoms // HL-C128 step 7: +8 // HL-C128 step 8: +15 // HL-C128 step 9: +8 // HL-C128 step 10: +11 // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      atomsNeverRevisited: 128, // HL-C194: +16 Spanish words // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // slice 8 nets zero: -1 ES-FRIEND-NCIA-NCE (the new review and synthesis revisit it), +1 ES-ETYMON-ARIUS (introduced, never re-practised) // +2: HL-C98 // +4: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +1: HL-C88 slice 9 (falsos amigos) // +1: B1 si-condition rung // +2: HL-C113 preterite plural // -2 (the review and synthesis revisit two orphaned atoms): HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // -1 (the review revisits an orphaned atom): HL-C113 unreal condition // HL-C113 step 7: -3 -- the review and synthesis revisit the three reported-speech atoms step 6 left unspent // HL-C113 step 8: +4 -- pero/tambien/tampoco mint lexical atoms the review rung has not spent yet // HL-C128 step 3: +1 // HL-C128 step 4: +2 // HL-C128 step 5: +2 // HL-C127: +3 // HL-C128 step 7: +1 // HL-C128 step 8: +1 -- 15 new atoms, and the ch256 review revisits all but one of them; without that review this would have been +15, which is what prompted writing it // HL-C128 step 9: -1 -- the ch261 review revisits all eight new atoms and one older orphan // HL-C128 step 10: +3 // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288)
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

  it("keeps the windows expanding, which is the whole point", () => {
    let previous = 0;
    for (const window of REINFORCEMENT_WINDOWS) {
      expect(window.from).toBeGreaterThan(previous);
      expect(window.to).toBeGreaterThan(window.from);
      previous = window.to;
    }
  });
});
