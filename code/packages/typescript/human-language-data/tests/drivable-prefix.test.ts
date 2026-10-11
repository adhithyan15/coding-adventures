// Why a chapter cannot be started in the car — and a ceiling on the part an author
// can move.
//
// The gap report's `chaptersWithoutDrivablePrefix` counts chapters whose FIRST lesson
// a driver cannot do, so the narration has to open them with "The first lesson already
// needs your eyes or your hands, so save this one for when you have stopped." When it
// was first read as a to-do list it was 306, and almost two
// thirds of that was not debt at all: chapters that open on a `type: writing` or
// `type: reading` lesson, whose whole point is the hand or the eye. HL08 forbids
// thinning the book to make those drivable, so the number could never reach zero and
// nobody could tell which part of it was worth working on.
//
// `unstartableCause` (src/modality.ts) splits it:
//
//   lesson-type     the opener is a writing or reading lesson — an honest zero
//   lesson-content  the opener is any other lesson whose CORE still needs eyes or
//                   hands — a pointing phrase, an unspeakable table, a step said
//                   plainly — which rewording, deferral or a detachable `## Writing`
//                   section can often move
//
// This file holds three things, in the shape lessons.d asks for:
//
//   1. The split is a partition: the two causes add up to the total, on fixtures and
//      on the real corpus.
//   2. Each cause can fire, and the remedies the curriculum actually uses really do
//      clear the content cause — demonstrated on SYNTHETIC lessons, never by asserting
//      the real corpus still has a defect (lessons.d/a-test-that-proves-a-measure-is-
//      not-vacuous-by-reading-debt.md: such a test goes red when the debt is paid).
//   3. A ceiling on the real corpus's by-content count that may only FALL. The
//      by-type count gets no ceiling on purpose: HL08 says honest writing instruction
//      may raise the pen count and "no gate here may push back on it", and a chapter
//      that opens by teaching a letter is exactly that.

import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { buildCurriculumGapReport } from "../src/report.js";
import {
  lessonModalities,
  orderChapterLessons,
  summarizeModality,
  unstartableCause,
} from "../src/modality.js";

/** Build a lesson from parts, so each case names only what it is testing. */
function lesson(options: {
  id: string;
  chapter?: number;
  sequence: number;
  type?: string;
  skills?: string;
  body: string;
}): ReturnType<typeof parseLesson> {
  const frontmatter = [
    "schema_version: 2",
    `id: ${options.id}`,
    `chapter: ${options.chapter ?? 1}`,
    `sequence: ${options.sequence}`,
    `type: ${options.type ?? "word"}`,
    "headword: hola",
    "gloss: hello",
    "concept_tag: GREETING-HELLO",
    `skills: [${options.skills ?? "listening, speaking, reading"}]`,
  ];
  return parseLesson(
    `---\n${frontmatter.join("\n")}\n---\n\n# ${options.id}\n\n${options.body}\n`,
    "spanish",
  );
}

/** The cause for a one-chapter corpus, through the same path the report takes. */
function causeOf(lessons: ReturnType<typeof parseLesson>[]) {
  return unstartableCause(orderChapterLessons(lessonModalities(lessons)));
}

const SPOKEN_FOLLOWER = lesson({
  id: "ES-C01-b",
  sequence: 20,
  body: "## Warm-up\n\nSay *hola* out loud.",
});

// The two shapes the corpus's openers were actually blocked by, each in the form it
// had before its fix and the form the fix gave it. The look-at-its-shape sentence is
// the 79-lesson anchor-word template; the writing step is MR-C01-ho's.
const LOOK_PLAIN =
  "## You'll want to know: hola\n\nSay it: *hola*. Look at its shape as you say it: " +
  "the next lesson takes one piece of it.";
const LOOK_DEFERRED =
  "## You'll want to know: hola\n\nSay it: *hola*. [YOU LOOK: at its shape as you say " +
  "it.] The next lesson takes one piece of it.";
const WRITE_IN_PRACTICE =
  "## Guided Practice\n\n- [YOU SAY: *hola*]\n\nNow cover the model. Write *hola* once " +
  "from its spoken name.";
const WRITE_DETACHED =
  "## Guided Practice\n\n- [YOU SAY: *hola*]\n\n## Writing — the first return\n\n" +
  "Cover the model. Write *hola* once from its spoken name.";

describe("unstartableCause — the truth table", () => {
  it("is null when the chapter opens on a lesson whose core is voice", () => {
    expect(causeOf([lesson({ id: "ES-C01-a", sequence: 10, body: "## Warm-up\n\nSay *hola*." })]))
      .toBeNull();
  });

  it("is null for an empty chapter — there is no lesson to blame", () => {
    expect(unstartableCause([])).toBeNull();
  });

  it("names the lesson TYPE when a writing lesson opens the chapter", () => {
    const opener = lesson({ id: "ES-C01-a", sequence: 10, type: "writing", body: "## Warm-up\n\nTrace it." });
    expect(causeOf([opener, SPOKEN_FOLLOWER])).toBe("lesson-type");
  });

  it("names the lesson TYPE when a reading lesson opens the chapter", () => {
    const opener = lesson({ id: "ES-C01-a", sequence: 10, type: "reading", body: "## Reading\n\nSay *hola*." });
    expect(causeOf([opener, SPOKEN_FOLLOWER])).toBe("lesson-type");
  });

  it("names the CONTENT when an ordinary lesson's core needs eyes", () => {
    const opener = lesson({ id: "ES-C01-a", sequence: 10, body: LOOK_PLAIN });
    expect(causeOf([opener, SPOKEN_FOLLOWER])).toBe("lesson-content");
  });

  it("names the CONTENT when an ordinary lesson's core asks for a hand", () => {
    const opener = lesson({ id: "ES-C01-a", sequence: 10, body: WRITE_IN_PRACTICE });
    expect(causeOf([opener, SPOKEN_FOLLOWER])).toBe("lesson-content");
  });

  it("reads the FIRST lesson in authored order, not in array order", () => {
    // The blocker is passed first but sequenced second, so the chapter opens on voice.
    const late = lesson({ id: "ES-C01-z", sequence: 30, body: LOOK_PLAIN });
    expect(causeOf([late, SPOKEN_FOLLOWER])).toBeNull();
  });
});

describe("the remedies the curriculum uses really clear the content cause", () => {
  // Non-vacuity on fixtures: the measure fires on the plain form and stops firing on
  // the fixed form, so a ceiling built on it moves when an author does the work.
  it("a pointing step deferred as a [YOU LOOK: …] cue makes the chapter startable", () => {
    expect(causeOf([lesson({ id: "ES-C01-a", sequence: 10, body: LOOK_PLAIN })])).toBe(
      "lesson-content",
    );
    expect(causeOf([lesson({ id: "ES-C01-a", sequence: 10, body: LOOK_DEFERRED })])).toBeNull();
  });

  it("a writing step moved into a detachable ## Writing section makes it startable", () => {
    expect(causeOf([lesson({ id: "ES-C01-a", sequence: 10, body: WRITE_IN_PRACTICE })])).toBe(
      "lesson-content",
    );
    const detached = lesson({
      id: "ES-C01-a",
      sequence: 10,
      skills: "listening, speaking, writing",
      body: WRITE_DETACHED,
    });
    // The book still prints the writing (full modality pen); the car skips it.
    const [entry] = lessonModalities([detached]);
    expect(entry?.modality).toBe("pen");
    expect(entry?.coreModality).toBe("voice");
    expect(causeOf([detached])).toBeNull();
  });

  it("no rewording clears the TYPE cause — a writing lesson stays an honest zero", () => {
    const opener = lesson({ id: "ES-C01-a", sequence: 10, type: "writing", body: LOOK_DEFERRED });
    expect(causeOf([opener])).toBe("lesson-type");
  });
});

describe("the gap report's split", () => {
  const registry = {
    version: 1,
    languages: [
      { id: "spanish", name: "Spanish", family: "Romance", script: "latin", status: "active", bridges: [] },
    ],
  };

  it("partitions chaptersWithoutDrivablePrefix into by-type and by-content", () => {
    const report = buildCurriculumGapReport({
      registry,
      lessons: [
        lesson({ id: "ES-C01-a", chapter: 1, sequence: 10, type: "writing", body: "## Warm-up\n\nTrace it." }),
        lesson({ id: "ES-C02-a", chapter: 2, sequence: 20, body: LOOK_PLAIN }),
        lesson({ id: "ES-C03-a", chapter: 3, sequence: 30, body: WRITE_IN_PRACTICE }),
        lesson({ id: "ES-C04-a", chapter: 4, sequence: 40, body: LOOK_DEFERRED }),
      ],
      books: { books: [] },
    });
    expect(report.summary.chaptersWithoutDrivablePrefix).toBe(3);
    expect(report.summary.chaptersWithoutDrivablePrefixByType).toBe(1);
    expect(report.summary.chaptersWithoutDrivablePrefixByContent).toBe(2);
  });
});

// The whole corpus, parsed ONCE at import: whole-corpus work in a test body is a
// timeout under full-suite load (lessons.d/whole-corpus-work-in-a-test-body-is-a-
// timeout-on-a.md). The tests below only read it.
const CORPUS = summarizeModality(loadEverything().lessons);
const CORPUS_CHAPTERS = CORPUS.tracks.flatMap((track) => track.chapters);

/**
 * Chapters that open on a non-writing, non-reading lesson whose core needs eyes or
 * hands. A CEILING THAT MAY ONLY FALL — lower it when you clear one; never raise it.
 *
 * History, so the next reader knows what the number has already absorbed:
 *
 *   123  measured when the split was introduced (306 in all, 183 of them by type).
 *    80  after the drive-prefix pass: the 79-lesson anchor-word template deferred its
 *        "Look at its shape as you say it" as a [YOU LOOK: …] cue; five Gujarati R4
 *        reviews deferred "then read **…** once" as [YOU READ: …]; MR-C01-ho moved
 *        its writing return into its own `## Writing` section; and 24 pointing
 *        phrases that were never about the page ("look at your collarbone", "the
 *        Persian for to look at", "see the Hindi color lesson") or whose point is
 *        as audible as it is visible were said for the ear or quoted as glosses.
 *
 *     7  after the second pass, which read every one of the 80 openers. 73 lessons
 *        moved: 19 four-or-five-column tables whose extra column was derivable or
 *        foldable (a bare numeral beside its word, "planet-god" beside "day of
 *        Mars") became three LABELLED columns the narrator reads, with no value
 *        dropped and the linearisable width still 3; references that were never
 *        about the page ("Before the table:", "the whole first column", "opening a
 *        column this book has left empty", "can you see the seam?") were said for
 *        the ear; written-accent and letter-shape observations were deferred as
 *        [YOU LOOK: …] with the fact still said aloud; and one writing warm-up
 *        (ES-C422-pagina-web) became [YOU WRITE: …].
 *
 * What is left is honest, and each was read: four Punjabi date-card reviews
 * (PA-R168..R171) whose every step handles a printed card; Spanish chapter 46's
 * eighteen-form -ar/-er/-ir grid, whose drills run "down a column, then across a
 * row"; and two reading-practice openers (GU-C25-savaar, UR-C16-practice) that are
 * not `type: reading` but are reading all the same. If a NEW lesson raises this,
 * the fix is in the lesson: defer the step ([YOU READ: …], [YOU WRITE: …],
 * [YOU LOOK: …]), move a writing step into a `## Writing` section, fold a derivable
 * table column, or say the point for the ear — not here.
 *
 * 7 -> 8, deliberately: PA-R172-form-order-again (#17162) is a fifth Punjabi
 * printed-form review — "the learner must inspect and copy printed Gurmukhi form
 * labels", its own authored reason — and so honest in exactly the way R168..R171
 * are. It merged with a modality record derived before the driver-safety rules
 * (#17203/#17204) and announced itself as drivable; regenerating it says `pen`,
 * which is the truth, and opens its chapter. Nothing in the lesson can be deferred
 * without leaving it nothing to do.
 */
const BY_CONTENT_CEILING = 8;

describe("corpus: chapters a driver cannot start", () => {
  it("starts Punjabi Chapter 172 by ear but stops before the printed form", () => {
    const chapter = CORPUS_CHAPTERS.find(
      (entry) => entry.language === "punjabi" && entry.chapter === 172,
    );
    expect(chapter).toBeDefined();
    expect(chapter?.lessonCount).toBe(2);
    expect(chapter?.drivablePrefix).toBe(1);
    expect(chapter?.firstNonVoiceLesson).toBe("PA-R172-form-order-again");
    expect(chapter?.unstartable).toBeNull();
  });

  it("splits every unstartable chapter into exactly one cause", () => {
    const unstartable = CORPUS_CHAPTERS.filter((chapter) => chapter.drivablePrefix === 0);
    expect(unstartable.length).toBeGreaterThan(0);
    for (const chapter of CORPUS_CHAPTERS) {
      // A cause exactly when the prefix is 0 — never on a startable chapter, never
      // missing on an unstartable one.
      expect(chapter.unstartable === null, `${chapter.language} ch${chapter.chapter}`).toBe(
        chapter.drivablePrefix > 0,
      );
    }
  });

  it("holds the by-content count under its ceiling (a ceiling that may only fall)", () => {
    const byContent = CORPUS_CHAPTERS.filter((chapter) => chapter.unstartable === "lesson-content");
    const listing = byContent
      .map((chapter) => `${chapter.language} ch${chapter.chapter} (${chapter.firstNonVoiceLesson})`)
      .join(", ");
    expect(byContent.length, listing).toBeLessThanOrEqual(BY_CONTENT_CEILING);
  });
});
