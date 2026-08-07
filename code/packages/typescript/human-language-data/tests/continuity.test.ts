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
    expect(report.summary.missedByWindow.R2).toBe(0);
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
});

describe("the real corpus", () => {
  it("pins what nothing had measured", () => {
    const { lessons } = loadEverything();
    const report = measureContinuity(lessons);

    // ORDER. Until this reaches zero every other number here is provisional: a ramp
    // whose reading order is unknown cannot be verified at all.
    expect(report.summary.lessonsWithoutSequence).toBe(565);
    expect(report.summary.tracksWithUnorderedLessons).toBe(19);
    expect(report.summary.forwardPrerequisites).toBe(271);

    // 331 lessons claim to review material the learner has not reached yet.
    expect(report.summary.forwardReviews).toBe(331);

    // REINFORCEMENT. The founding promise is that the course "constantly
    // re-emphasizes what was learnt previously". Half of it is taught once.
    // The second verb tranche (HL-C43) then added 24 lessons teaching 50 atoms, of which
    // 21 are never revisited — 42%, against the corpus's 51% — so the headline share
    // ticks DOWN a point. New content is not making this worse; it is slightly better
    // than what it joins. That is still 21 more atoms taught once and abandoned.
    expect(report.summary.atomsTaught).toBe(1519);
    expect(report.summary.atomsNeverRevisited).toBe(767);
    expect(report.summary.neverRevisitedPercent).toBe(50);

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
    expect(report.summary.forwardReferences).toBe(517);
  });

  it("reproduces the Spanish audit exactly", () => {
    // These four numbers were measured independently in Python before this module
    // existed. They agreeing is the evidence that the walk is the right one.
    const { lessons } = loadEverything();
    const spanish = measureContinuity(lessons).tracks.find((t) => t.language === "spanish");
    // HL-C43 added 8 Spanish lessons (chapters 34-35), all of them sequenced and
    // prerequisite-closed: `lessonsWithoutSequence` and `forwardPrerequisites` do not
    // move at all, while `atomsTaught` goes 182 -> 199 and `atomsNeverRevisited`
    // 93 -> 102. The independent Python audit these four numbers reproduce was run
    // against the 146-lesson corpus; the walk is unchanged, only its input grew.
    expect(spanish).toMatchObject({
      lessonCount: 154,
      lessonsWithoutSequence: 56,
      forwardPrerequisites: 31,
      atomsTaught: 199,
      atomsNeverRevisited: 102,
    });
  });

  it("finds the forward references a human reviewer found by reading", () => {
    const { lessons } = loadEverything();
    const found = measureContinuity(lessons).forwardReferences;
    const of = (word: string) => found.find((f) => f.language === "spanish" && f.word === word);

    // "Como pan y bebo agua" — in chapter 7. Both words are chapter 26.
    expect(of("pan")).toMatchObject({ lessonId: "ES-C07-beber", taughtBy: "ES-C26-pan" });
    expect(of("agua")).toMatchObject({ lessonId: "ES-C07-beber", taughtBy: "ES-C26-agua" });
    // A chapter that taught 1-10 drilling a chapter-31 number.
    expect(of("diecinueve")?.lessonId).toBe("ES-C08-practice");
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
