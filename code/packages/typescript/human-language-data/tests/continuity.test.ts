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
    expect(report.summary.lessonsWithoutSequence).toBe(507);
    expect(report.summary.tracksWithUnorderedLessons).toBe(19);
        // 245 -> 240. Not five prerequisites fixed — five that were never real. Without a
    // `sequence` the walk falls back to alphabetical, so "TA-C01-aam requires
    // TA-C01-vanakkam-family-register" read as a forward prerequisite purely because
    // `aam` sorts first. Declaring chapter 1's order removed the artifact.
    expect(report.summary.forwardPrerequisites).toBe(240);

    // Lessons claiming to review material the learner has not reached yet.
    // 300 -> 285. Fourteen are the same alphabetical-fallback artifact as
    // forwardPrerequisites above, four of them in the writing track rather than the
    // word lessons; the fifteenth is a `reviews_of` list that genuinely shrank when its
    // lesson was rewritten.
    expect(report.summary.forwardReviews).toBe(285);

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
    expect(report.summary.atomsTaught).toBe(2502);
    expect(report.summary.atomsNeverRevisited).toBe(472);
    expect(report.summary.neverRevisitedPercent).toBe(19);

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
    expect(report.summary.forwardReferences).toBe(469);

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
    // Vocabulary wave 4 pushed both further out: R1 834 -> 838, R2 1627 -> 1718. New
    // pre-A1 nouns keep landing near the end of already-long chapters, so the near
    // window (R1) barely moves while the far one (R2) absorbs most of the growth --
    // the same shape every vocabulary wave has had.
    expect(report.summary.missedByWindow.R1).toBe(838);
    expect(report.summary.missedByWindow.R2).toBe(1718);
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
      lessonCount: 177,
      lessonsWithoutSequence: 6,
      forwardPrerequisites: 5,
      atomsTaught: 260,
      atomsNeverRevisited: 70,
    });
  });

  it("leaves chapter 7 unsequenced, because its sources contradict", () => {
    // curriculum.json says comer -> beber -> que -> vivir -> donde; the lesson prose
    // `Next:` chain AND ES-C07-beber's own reviews_of say comer -> vivir -> beber ->
    // que -> donde. Under the ledger's order, beber reviews a lesson that has not
    // happened. Guessing would bake a false ramp into every later measurement, so
    // these six stay unsequenced until the project owner rules.
    const { lessons } = loadEverything();
    const unsequenced = measureContinuity(lessons)
      .order.filter((d) => d.language === "spanish" && d.kind === "no-sequence")
      .map((d) => d.lessonId)
      .sort();
    expect(unsequenced).toEqual([
      "ES-C07-beber",
      "ES-C07-comer",
      "ES-C07-donde",
      "ES-C07-practice",
      "ES-C07-que",
      "ES-C07-vivir",
    ]);
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
