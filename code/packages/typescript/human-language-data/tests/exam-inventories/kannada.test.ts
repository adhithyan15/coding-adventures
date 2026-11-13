import { describe, expect, it } from "vitest";
import {
  loadCurriculumSpine,
  loadExamInventory,
  loadTrackLanguageCurriculum,
  loadTrackChapters,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { lessonsUpToLevel } from "../../src/levels.js";
import { measureContinuity } from "../../src/continuity.js";

describe("the committed Kannada A1 inventory", () => {
  const inventory = loadExamInventory("kannada", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name. A probe pointing at an id somebody
    // expects a future lesson to introduce resolves to "not introduced" forever
    // and sits in the report indistinguishable from the honest gaps around it.
    // This file was written while Kannada chapters 1, 2 and 4 were still
    // hand-written, so 37 of the atoms it probes did not exist when its first
    // draft was validated; the atom set was re-derived from the merged tree
    // before commit rather than trusted from the working branch.
    const lessons = loadTrackLessons("kannada");
    const taught = trackIntroducedAtoms(lessons, "kannada");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point either derives into some Kannada point or is dropped
    // with a reason, and no point may be both. Fifteen were both in the first
    // draft, because restating a point around Kannada machinery FEELS like not
    // transferring it. Restating is deriving; `notTransferred` is now only the
    // two points that produce no Kannada point at all.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    for (const id of dropped) expect(sourceIds.has(id), `notTransferred names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; kannadaSpecific?: boolean };
      expect(cast.kannadaSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    // Deliberately few, and that is a claim rather than an omission: nearly
    // every Kannada column answers a demand some Spanish point also makes, even
    // where the machinery is completely different -- a case suffix doing what a
    // preposition does, a dative subject doing what gustar does. Only four
    // points have no Spanish question behind them at all.
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id).sort()).toEqual(
      ["KA-A1-CASE-02", "KA-A1-N-06", "KA-A1-P-05", "KA-A1-REG-04"],
    );
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Kannada Sahitya Parishat/);
    expect(inventory.about).toMatch(/Karnataka's state school syllabi/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Kannada has neither mocks nor task shapes nor an assessment contract, so
    // the file must say what it used instead. An unstated substitution is an
    // unauditable one.
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no kannada\/task-shapes\/ and no kannada\/mocks\//);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
    expect(new Set(anchors.map((anchor) => anchor.kind))).toEqual(
      new Set(["sourced-proxy", "external-framework", "project-owned", "editorial"]),
    );
    for (const anchor of anchors) expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(0);
    const known = new Set(anchors.map((anchor) => anchor.id));
    for (const point of inventory.points) {
      const ids = (point as unknown as { anchorIds?: string[] }).anchorIds;
      expect(ids?.length, `${point.id} names no anchor`).toBeGreaterThan(0);
      for (const id of ids ?? []) expect(known.has(id), `${point.id} cites unknown anchor ${id}`).toBe(true);
    }
  });

  it("distinguishes mapped A1 spatial words from the untaught relational frame", () => {
    // Issue #19319 audits the actual spine-derived book level. The extension
    // stage is pre-A1 for all three words, but the spine places two at A1.
    // A chapter number or isolated word is not an assessed composition.
    const lessons = loadTrackLessons("kannada");
    const curriculum = loadTrackLanguageCurriculum("kannada");
    expect(curriculum).toBeDefined();
    const throughPreA1 = new Set(
      lessonsUpToLevel(lessons, [curriculum!], loadCurriculumSpine(), "pre-A1")
        .map((lesson) => lesson.realization.lessonId),
    );
    expect(throughPreA1.has("KA-C100-near")).toBe(true);
    for (const id of ["KA-C107-mele", "KA-C108-kelage"]) {
      expect(throughPreA1.has(id), `${id} is unexpectedly pre-A1`).toBe(false);
    }
    const throughA1 = new Set(
      lessonsUpToLevel(lessons, [curriculum!], loadCurriculumSpine(), "A1")
        .map((lesson) => lesson.realization.lessonId),
    );
    for (const id of ["KA-C100-near", "KA-C107-mele", "KA-C108-kelage"]) {
      expect(throughA1.has(id), `${id} is missing from A1`).toBe(true);
    }
    const spatial = inventory.points.find((point) => point.id === "KA-A1-POST-02");
    expect(spatial?.probe).toBeNull();
    expect(spatial?.note).toMatch(/hattira/);
    expect(spatial?.note).toMatch(/isolated|word/);
    expect(spatial?.note).not.toMatch(/Not one spatial relator/);
  });

  it("teaches the A1 to/from movement contrast after the place vocabulary", () => {
    // Issue #19319 reuses the chapter 103 ablative rather than inventing a
    // second source marker. An isolated go/come or school word is not enough.
    const lessons = loadTrackLessons("kannada");
    const movement = lessons.find(
      (lesson) => lesson.realization.lessonId === "KA-C108-movement",
    );
    expect(movement).toBeDefined();
    expect(movement?.frontmatter["introduces.knowledge"]).toContain(
      "KA-GRAMMAR-C108-DESTINATION-GE-06",
    );
    expect(movement?.frontmatter["practises.knowledge"]).toContain(
      "KA-GRAMMAR-C103-ABLATIVE-INDA-08",
    );
    expect(movement?.body).toContain("ನಾನು ಶಾಲೆಗೆ ಹೋಗುತ್ತೇನೆ");
    expect(movement?.body).toContain("ನಾನು ಬೆಂಗಳೂರಿನಿಂದ ಬಂದಿದ್ದೇನೆ");
    expect(Number(movement?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);

    const point = inventory.points.find((entry) => entry.id === "KA-A1-POST-03");
    expect(point?.probe).toEqual([
      "KA-GRAMMAR-C103-ABLATIVE-INDA-08",
      "KA-GRAMMAR-C108-DESTINATION-GE-06",
    ]);
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.points.find((entry) => entry.id === point?.id)?.covered).toBe(true);
    expect(coverage.covered).toBe(212);
    expect(coverage.unmapped).toBe(46);

    const curriculum = loadTrackLanguageCurriculum("kannada");
    expect(curriculum).toBeDefined();
    expect(lessonsUpToLevel(lessons, [curriculum!], loadCurriculumSpine(), "A1")
      .some((lesson) => lesson.realization.lessonId === "KA-C108-movement")).toBe(true);
    const chapter108 = loadTrackChapters()
      .find((track) => track.language === "kannada")
      ?.chapters.find((chapter) => chapter.chapter === 108);
    expect(chapter108?.payoff?.lesson).toBe("KA-C108-movement");
    expect(chapter108?.payoff?.assesses).toContain("KA-GRAMMAR-C108-DESTINATION-GE-06");

    // Four later retrievals span the R1-R4 windows; a one-off probe should
    // not turn a durable A1 ability into a coverage score.
    for (const id of [
      "KA-C109-setuve", "KA-C110-sante", "KA-C113-yajamana", "KA-C149-seru",
    ]) {
      const review = lessons.find((lesson) => lesson.realization.lessonId === id);
      expect(review?.frontmatter["practises.knowledge"], id).toContain(
        "KA-GRAMMAR-C108-DESTINATION-GE-06",
      );
    }
    expect(measureContinuity(lessons).reinforcement.find(
      (defect) => defect.atom === "KA-GRAMMAR-C108-DESTINATION-GE-06",
    )).toBeUndefined();
  }, 60_000);

  it("teaches the six-part A1 origin cluster before the A1 boundary", () => {
    // The 2026-11-04 source audit found that the old inventory notes were stale:
    // uuru, nagara, deesha and raajya already have direct owners. The missing
    // work is the composition path -- place names, the person/place relation,
    // ablative -inda, and the ask/answer pair. Keep the exact six-point effect
    // pinned so a broad new lesson cannot receive accidental credit here.
    const originIds = [
      "KA-A1-N-02",
      "KA-A1-ADJ-04",
      "KA-A1-CASE-07",
      "KA-A1-Q-09",
      "KA-A1-F-05",
      "KA-A1-LEX-55",
    ];
    const lessons = loadTrackLessons("kannada");
    const coverage = measureExamCoverage(inventory, lessons);
    const origin = coverage.points.filter((point) => originIds.includes(point.id));
    expect(origin.map((point) => point.id)).toEqual(originIds);
    expect(origin.every((point) => point.covered)).toBe(true);
    expect(coverage.covered).toBe(212);
    expect(coverage.unmapped).toBe(46);
  }, 60_000);

  it("teaches the five-part A1 third-person bridge before the A1 boundary", () => {
    // Issue #19104 deliberately joins the five points that make chapter 12's
    // family words usable in a sentence. Keep the effect exact: a future broad
    // lesson must not receive accidental credit for an unrelated A1 point.
    const bridgeIds = [
      "KA-A1-N-05",
      "KA-A1-N-06",
      "KA-A1-POS-03",
      "KA-A1-PRON-03",
      "KA-A1-T-04",
    ];
    const lessons = loadTrackLessons("kannada");
    const coverage = measureExamCoverage(inventory, lessons);
    const bridge = coverage.points.filter((point) => bridgeIds.includes(point.id));
    expect(bridge.map((point) => point.id)).toEqual(bridgeIds);
    expect(bridge.every((point) => point.covered)).toBe(true);
    expect(coverage.covered).toBe(212);
    expect(coverage.unmapped).toBe(46);

    // The plural examples must be words the learner actually owns. The first
    // reviewed draft used banna/bannagaḷu while citing the four colour-word
    // atoms, even though none of those atoms teaches the noun "colour".
    const pluralLesson = lessons.find(
      (lesson) => lesson.realization.lessonId === "KA-C12-plural-galu",
    );
    expect(pluralLesson?.frontmatter["requires.knowledge"]).toEqual(
      expect.arrayContaining(["KA-LEX-C02-HESARU-01", "KA-LEX-C05-MAADU-01"]),
    );
    expect(pluralLesson?.frontmatter["requires.knowledge"]).not.toEqual(
      expect.arrayContaining(["KA-LEX-C11-BANNAGALU-01", "KA-LEX-C11-BANNAGALU-02"]),
    );
    expect(pluralLesson?.body).toContain("ಕೆಲಸ");
    expect(pluralLesson?.body).not.toContain("ಬಣ್ಣ");

    // The chapter payoff is a second public claim about the last lesson. Keep
    // it exact too: stale colour atoms survived the first prose repair because
    // only the lesson frontmatter and body were pinned.
    const chapter12 = loadTrackChapters()
      .find((track) => track.language === "kannada")
      ?.chapters.find((chapter) => chapter.chapter === 12);
    expect(chapter12?.payoff?.lesson).toBe("KA-C12-plural-galu");
    expect(chapter12?.payoff?.assesses).toEqual([
      "KA-ETYMON-C12-KUTUMBA-01",
      "KA-ETYMON-C12-KUTUMBA-02",
      "KA-GRAMMAR-C12-THIRD-PERSON-03",
      "KA-GRAMMAR-C12-HUMAN-NONHUMAN-04",
      "KA-GRAMMAR-C12-PRESENT-THIRD-05",
      "KA-GRAMMAR-C12-THIRD-PERSON-POSSESSIVE-06",
      "KA-GRAMMAR-C12-PLURAL-GALU-07",
      "KA-LEX-C02-HESARU-01",
      "KA-LEX-C05-MAADU-01",
    ]);
    expect(new Set(chapter12?.payoff?.assesses)).toEqual(
      new Set(pluralLesson?.frontmatter["practises.knowledge"]),
    );

    const curriculum = loadTrackLanguageCurriculum("kannada");
    expect(curriculum).toBeDefined();
    const throughA1 = new Set(
      lessonsUpToLevel(
        lessons,
        [curriculum!],
        loadCurriculumSpine(),
        "A1",
      ).map((lesson) => lesson.realization.lessonId),
    );
    expect([
      "KA-C12-third-person",
      "KA-C12-present-endings",
      "KA-C12-third-person-possession",
      "KA-C12-plural-galu",
    ].every((id) => throughA1.has(id))).toBe(true);
  }, 60_000);

  it("teaches the A1 accusative and object-pronoun bridge before the A1 boundary", () => {
    // Issue #19124 follows the dependency written into the inventory itself:
    // object pronouns cannot be owned until the accusative that builds them is
    // owned. Keep the effect exact so a later broad case lesson cannot receive
    // accidental credit for either half of this small bridge.
    const objectIds = ["KA-A1-PRON-07", "KA-A1-CASE-03"];
    const lessons = loadTrackLessons("kannada");
    const coverage = measureExamCoverage(inventory, lessons);
    const objectBridge = coverage.points.filter((point) => objectIds.includes(point.id));
    expect(objectBridge.map((point) => point.id)).toEqual(objectIds);
    expect(objectBridge.every((point) => point.covered)).toBe(true);
    expect(coverage.covered).toBe(212);
    expect(coverage.unmapped).toBe(46);

    const accusative = lessons.find(
      (lesson) => lesson.realization.lessonId === "KA-C104-accusative-annu",
    );
    const pronouns = lessons.find(
      (lesson) => lesson.realization.lessonId === "KA-C104-object-pronouns",
    );
    expect(accusative?.body).toContain("ಪೊಲೀಸರನ್ನು ಕರೆಯಿರಿ");
    expect(accusative?.body).toContain("-ಅನ್ನು");
    expect(pronouns?.body).toContain("ನನ್ನನ್ನು");
    expect(pronouns?.body).toContain("ನಿಮ್ಮನ್ನು");
    expect(pronouns?.body).toContain("ಅವನನ್ನು");
    expect(pronouns?.body).toContain("ಅವಳನ್ನು");
    expect(pronouns?.body).toContain("ಅವರನ್ನು");
    expect(pronouns?.body).toContain("ಅದನ್ನು");
    expect(accusative?.frontmatter["requires.knowledge"]).toContain("KA-LEX-C101-OPEN-04");
    expect(accusative?.frontmatter["practises.knowledge"]).toContain("KA-LEX-C101-OPEN-04");
    expect(pronouns?.frontmatter["requires.knowledge"]).toContain("KA-LEX-C101-OPEN-04");
    expect(pronouns?.frontmatter["practises.knowledge"]).toContain("KA-LEX-C101-OPEN-04");

    const chapter104 = loadTrackChapters()
      .find((track) => track.language === "kannada")
      ?.chapters.find((chapter) => chapter.chapter === 104);
    expect(chapter104?.payoff?.lesson).toBe("KA-C104-object-pronouns");
    expect(new Set(chapter104?.payoff?.assesses)).toEqual(
      new Set(pronouns?.frontmatter["practises.knowledge"]),
    );

    const curriculum = loadTrackLanguageCurriculum("kannada");
    expect(curriculum).toBeDefined();
    const throughA1 = new Set(
      lessonsUpToLevel(
        lessons,
        [curriculum!],
        loadCurriculumSpine(),
        "A1",
      ).map((lesson) => lesson.realization.lessonId),
    );
    expect(
      ["KA-C104-accusative-annu", "KA-C104-object-pronouns"].every((id) =>
        throughA1.has(id),
      ),
    ).toBe(true);
  }, 60_000);

  it("reports a CLOSED joining column and a script still 19 characters short", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("kannada");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(258);
    // 194 -> 197: KA-A1-L-12 (the full stop and the comma), KA-A1-L-13 (the
    // question mark) and KA-A1-L-14 (colon, brackets, quotes, dash). THE CORPUS
    // HAD BEEN PRINTING THESE MARKS SINCE CHAPTER ONE while no lesson named any
    // of them -- every Kannada sentence in every reading passage ends in a Latin
    // full stop -- so chapter 77 opens by pointing at the end of a line in the
    // previous chapter and saying that something is sitting there nothing has
    // named. Kannada borrows the whole Latin set, shape and job together, which
    // is why three points cost one short chapter and nothing in it looks
    // unfamiliar.
    // L-13 CARRIES THE ONE LOAD-BEARING CONTRAST: the Spanish demand it derives
    // from opens a question with a second inverted mark and Kannada does not, so
    // a Kannada reader meets the mark at the end or not at all and the WORDS have
    // to carry the question until then.
    // L-14 IS PROBED AS A RECOGNITION POINT, not a production one: at A1 the
    // demand is knowing what a colon or a bracket signals on a notice, and the
    // recall lesson sorts the set into the three a reader writes and the rest
    // they read. KA-A1-L-15 (abbreviations and symbols) stays open.
    // FOUR OF KANNADA'S UNMAPPED POINTS ARE STRUCTURALLY UNCOVERABLE and are
    // marked untransferable in the inventory: capital letters, written
    // accentuation and superscript abbreviation letters have no Kannada
    // counterpart at all, and neither does Spanish's mid-distance demonstrative.
    // The real ceiling for this track is 254/258, not 258/258.
    // 198 -> 204: the chapter-103 origin tranche closes exactly the six points
    // pinned above. The generic place words were already taught; the new work is
    // the bounded person/place relation, ablative, and ask-answer composition.
    // 204 -> 209: chapter 12 then closes the exact third-person bridge pinned
    // above: pronouns, human/non-human, present endings, possessives and -gaḷu.
    // 209 -> 211: chapter 104 closes the dependent accusative/object-pronoun
    // pair pinned above without claiming any other case or pronoun demand.
    // 211 -> 212: chapter 108 closes movement to/from, not relative position.
    expect(coverage.covered).toBe(212);
    expect(coverage.unmapped).toBe(46);
    expect(coverage.partial).toBe(0);
    // 193 -> 194: the HL-C354 ordinal tranche closed KA-A1-NUM-07 (chapters
    // 74-75). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    expect(coverage.byCategory["Sankhye (numerals and quantity)"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
    // WHAT THIS ASSERTION USED TO SAY, and why it changed. The inventory landed
    // reporting 167/258 and an EMPTY joining column: `mattu`, `athava`,
    // `aadare`, `eekendare` and the quotative `anta`/`endu` occurred ZERO times
    // in 268 lessons and zero times in the generated book, and the only two
    // covered points were the -i participle and the turn-level connectives of
    // chapter 64 -- a chapter literally named JOIN that joins turns and not
    // clauses. Chapters 67 to 73 answer that finding directly: 27 headwords
    // chosen off this file's OWN uncovered list closed 26 points, of which nine
    // are this column. The denominator did not move and `partial` stayed at 0,
    // so nothing was reworded to make the number rise.
    const joining = coverage.byCategory["Samuccaya (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 11 });
    // DO NOT CARRY THE TAMIL SHAPE HERE. Tamil's script column came back 52 of
    // 52 characters taught. Kannada's is the opposite case and was measured
    // twice: 42 characters taught against 69 used in headwords when this
    // inventory was written, and 50 of 69 after chapters 67-73 taught the eight
    // most-used untaught characters. `ma` alone appears in 36 headwords and was
    // never taught. Nineteen characters remain, six of which have a sourced
    // ductus this project has not spent and thirteen of which have none.
    // THE COUNT ONCE MOVED 8 -> 11 WITHOUT THE CHARACTER DEBT SHRINKING BY ONE.
    // Those three points were PUNCTUATION -- the full stop and comma, the
    // question mark, and the colon/brackets/quotes/dash -- which Kannada borrows
    // whole from the Latin alphabet and which this column happens to house
    // alongside the characters. A count over the column is the wrong proxy for
    // "the script is still short", so the assertion named the thing it meant:
    // KA-A1-L-09, whose own label used to read "THE SCRIPT IS NOT CLOSED: N
    // characters are used but never taught", with N going 27, 19, 13.
    // IT IS NOW CLOSED, AND THE ASSERTION IS REWRITTEN RATHER THAN FLIPPED. A
    // bare `.covered` toBe(true) would pass on a probe that had been quietly
    // emptied, so the probe's own size is asserted beside it: the point carries
    // all TWENTY-SEVEN characters it was opened for -- the eight chapters 67-73
    // taught, the six chapter 78 taught as writing from cited stroke-order
    // animations, and the thirteen chapters 79 and 80 taught as recognition
    // because no ductus for them exists anywhere in this project.
    // The character count itself is pinned at an exact zero in
    // tests/corpus/kannada.test.ts, which is where a regression would show.
    const scriptClosed = coverage.points.find((p) => p.id === "KA-A1-L-09")!;
    expect(scriptClosed.covered).toBe(true);
    expect(
      loadExamInventory("kannada", "A1").points.find((p) => p.id === "KA-A1-L-09")!.probe,
    ).toHaveLength(27);
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBe(12);
    // The two columns that carry this track, and they are not the ones French
    // and German lead on.
    expect(coverage.byCategory["Kriyaapada (the verb)"]!.covered).toBeGreaterThan(12);
    expect(coverage.byCategory["Padakosha (lexicon by domain)"]!.covered).toBeGreaterThan(45);
    expect(formatExamCoverage(coverage)).toContain(
      "kannada A1 (partial inventory): 212/258 points covered (82%)",
    );
  }, 60_000);
});
