// The CI gate: load the *real* curriculum off disk and assert it stays
// consistent with the taxonomy. If a future lesson drifts (an unknown tag, a
// duplicate realization, a missing required field), this test fails the build.

import { describe, it, expect } from "vitest";
import { loadEverything } from "../src/loader.js";
import { validate, hasErrors } from "../src/validate.js";
import { validateCurriculum } from "../src/curriculum.js";
import { buildCurriculumGapReport } from "../src/report.js";
import { languagesForConcept } from "../src/queries.js";
import { compileLessonActivities } from "../src/activity.js";

const { taxonomy, registry, spine, curricula, books, lessons, scripts, dataset } = loadEverything();

describe("real curriculum", () => {
  it("has zero validation errors", () => {
    const issues = validate({ taxonomy, lessons, scripts });
    const errors = issues.filter((i) => i.level === "error");
    // Surface any error messages so a failure is self-explaining.
    expect(errors.map((e) => e.message)).toEqual([]);
    expect(hasErrors(issues)).toBe(false);
  });

  it("loaded every track (17+ and growing)", () => {
    expect(dataset.languages.length).toBeGreaterThanOrEqual(20);
    for (const t of ["spanish", "telugu", "arabic", "russian", "persian", "urdu"]) {
      expect(dataset.languages).toContain(t);
    }
  });

  it("has a valid shared spine covering every registered language", () => {
    const issues = validateCurriculum({ registry, spine, curricula, taxonomy, lessons, books });
    expect(issues.filter((issue) => issue.level === "error").map((issue) => issue.message)).toEqual([]);
    expect(registry.languages.map((language) => language.id)).toEqual(dataset.languages.sort((a, b) => {
      const order = new Map(registry.languages.map((language, index) => [language.id, index]));
      return order.get(a)! - order.get(b)!;
    }));
  });

  // HL-C10: the spine reaches above A1.
  //
  // This is not bookkeeping. Schema v2 REQUIRES a canonical `spine_node`, and until this
  // tranche existed every node was an A1 social function — greeting, taking leave,
  // counting to five — with nothing covering verbs or tense. A lesson teaching the present
  // tense had no node it could legally declare, so the entire Easy-to-Advanced grammar arc
  // was unauthorable in v2 for all 22 tracks. These five nodes are what unblock it.
  it("carries an A2 tranche, and every track declares where it stands on it", () => {
    const a2 = spine.nodes.filter((node) => node.stage === "A2");
    expect(a2.map((node) => node.id).sort()).toEqual([
      "SPINE-NEGATE-AND-ASK",
      "SPINE-SAY-WHAT-I-DO",
      "SPINE-SAY-WHAT-I-WANT",
      "SPINE-TALK-ABOUT-FUTURE",
      "SPINE-TALK-ABOUT-PAST",
    ]);

    // Every track answers for every A2 node. An unrealized node is declared as such and
    // must name the concepts it is omitting — "we have not built this yet" is a recorded
    // position, never an absent key, so the debt is countable rather than invisible.
    for (const curriculum of curricula) {
      for (const node of a2) {
        const entry = curriculum.spine[node.id];
        expect(entry, `${curriculum.language} omits ${node.id}`).toBeDefined();
        if (entry!.segments.length === 0) {
          expect([...entry!.omits].sort()).toEqual([...node.concepts].sort());
        }
      }
    }
  });

  it("loads one prerequisite-safe realization map for every language", () => {
    expect(curricula.map((curriculum) => curriculum.language).sort())
      .toEqual(registry.languages.map((language) => language.id).sort());
    expect(curricula.flatMap((curriculum) => curriculum.path).length).toBeGreaterThan(300);
    expect(
      curricula.flatMap((curriculum) => curriculum.path.flatMap((segment) => segment.lessons)).length,
    ).toBeGreaterThan(800);
    expect(curricula.every((curriculum) =>
      spine.nodes.every((node) => curriculum.spine[node.id] !== undefined),
    )).toBe(true);

    const spanish = curricula.find((curriculum) => curriculum.language === "spanish")!;
    expect(spanish.spine["SPINE-MEET-GREET"]?.segments.length).toBeGreaterThan(1);
    expect(spanish.spine["SPINE-TAKE-LEAVE"]?.relocates["GREETING-GOODNIGHT"])
      .toBe("SPINE-TIME-OF-DAY");

    for (const language of ["persian", "urdu"]) {
      const curriculum = curricula.find((item) => item.language === language)!;
      expect(curriculum.extensions.some((extension) => extension.category === "script")).toBe(true);
    }
  });

  it("preserves every existing LaTeX book and maps each chapter to short lessons", () => {
    expect(books.books.length).toBeGreaterThanOrEqual(20);
    expect(books.books.reduce((sum, book) => sum + book.chapters.length, 0)).toBeGreaterThanOrEqual(100);
    // 33 -> 35: the second Spanish verb tranche added Chapters 34 and 35, the track's
    // first chapters filed under an A2 spine node (SPINE-SAY-WHAT-I-DO) rather than an
    // A1 social function. Both are generated from schema-v2 lessons like 1-6 and 19-33.
    // 35 -> 37: the third verb tranche added Chapters 36 (oír, dormir, caminar, correr)
    // and 37 (abrir, cerrar, sentarse, levantarse), again split 4+4 to stay inside
    // maxNewAtomsPerChapter, and again on SPINE-SAY-WHAT-I-DO.
    // 38 -> 40: HL-C52 took chapter 38 for the first above-A2 content (narrating), so
    // the final verb tranche renumbered to 39 (traer, conseguir, jugar, conocer) and
    // 40 (esperar, contestar, comprar). Two sessions authored a chapter 38 in parallel;
    // the collision surfaced as a merge conflict on chapters.json rather than silently,
    // because both sides must edit it.
    expect(books.books.find((book) => book.language === "spanish")?.chapters.length).toBe(302); // +4: HL-C98 // +5: HL-C99 splits the four mind-verbs into a chapter each, plus review and synthesis // +3: HL-C88 slice 8 // +1: HL-C88 slice 9 (falsos amigos) // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // spanish pre-A1 round 3: +35 lessons, +7 chapters (chapters 296-302)
    expect(
      books.books
        .find((book) => book.language === "persian")
        ?.chapters.map((chapter) => chapter.chapter),
      // 6 -> 8: the eight-verb tranche added Chapters 7 and 8 (mind verbs, then
      // taking/asking/helping/loving), split 4+4 to stay inside maxNewAtomsPerChapter.
      // 8 -> 11: the pre-A1 vocabulary tranche (HL-C41 continuation). Chapter 9 closes
      // SPINE-POLITE-REQUEST-REPAIR (âb, nân, chây, kelid); Chapter 10 adds family words
      // onto SPINE-EXCHANGE-NAMES (mâdar, pedar, barâdar, dokhtar); Chapter 11 adds body
      // words onto SPINE-CHECK-WELLBEING (cheshm, dast, pâ, zabân).
      // 11 -> 14: the pre-A1 vocabulary tranche's second round (HL-C41 continuation).
      // Chapter 12 adds nâm, del, dar, ketâb onto SPINE-EXCHANGE-NAMES; Chapter 13 adds
      // âsemân, khorshid, mâh, setâre, bârân onto SPINE-CHECK-WELLBEING; Chapter 14 adds
      // khâhar, pesar, mard, zan, dust onto SPINE-EXCHANGE-NAMES, closing the tranche.
      // 14 -> 15: HL-C233, the track's first script chapter. Persian taught no letters
      // at all before it, in 59 lessons across 14 chapters.
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    expect(
      books.books
        .find((book) => book.language === "urdu")
        ?.chapters.map((chapter) => chapter.chapter),
      // 5 -> 6: Chapter 6 is the Urdu track's first verb chapter (HL core verbs),
      // so this is the first Urdu chapter whose spine node is A2 rather than A1.
      // 6 -> 8: chapters 7 and 8 are the eight-verb tranche — think/understand/read/write
      // and take/ask/help/like — split into two four-lesson chapters so neither exceeds
      // the 12-atom chapter budget, both filed under SPINE-SAY-WHAT-I-DO like chapter 6.
      // 8 -> 12: the pre-A1 vocabulary tranche (HL-C41 continuation). Chapters 9-12 drop
      // back to pre-A1 spine nodes (family/friends and face words on EXCHANGE-NAMES and
      // CHECK-WELLBEING, heart as its own chapter, water/tea/milk/bread realizing
      // POLITE-REQUEST-REPAIR for the first time in this track), each staying within the
      // 12-atom chapter budget like every generated chapter before it.
      // 12 -> 15: the second pre-A1 vocabulary tranche (wave 6). Chapter 13 (colors) and
      // 14 (clothing) add further POLITE-REQUEST-REPAIR segments; chapter 15 (weather)
      // returns to CHECK-WELLBEING. All three stay within the 12-atom chapter budget.
      // 15 -> 16: HL-C234, the track's first script chapter. Urdu was the LAST track in
      // the corpus teaching no letters at all, in 59 lessons across 15 chapters.
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    expect(
      books.books
        .find((book) => book.language === "russian")
        ?.chapters.map((chapter) => chapter.chapter),
      // 2 -> 5. Chapters 1-2 stay hand-written; 3, 4 and 5 are the track's first
      // GENERATED chapters, and the first Cyrillic ones the book renderer produces.
      // Chapter 3 (the six core verbs) is generated here rather than left out, because
      // chapters 4-5 build on знать, быть and говорить directly: printing them without
      // chapter 3 would put a forward reference into the standalone PDF.
      // 5 -> 10: the pre-A1 vocabulary tranche (HL-C-russian-vocab). Chapter 6 is
      // water/coffee/tea/bread under SPINE-POLITE-REQUEST-REPAIR; 7-8 are
      // friend/siblings/family under SPINE-EXCHANGE-NAMES; 9-10 are the track's
      // first realization of SPINE-CHECK-WELLBEING, ear/nose/mouth/eye then heart.
      // 10 -> 13: the pre-A1 vocabulary program's second Russian tranche. Chapter
      // 11 is the track's first realization of SPINE-TAKE-LEAVE (all six of the
      // node's concepts); 12 completes the family Chapter 8 gathered with mother
      // and father; 13 extends SPINE-POLITE-REQUEST-REPAIR with milk, cheese,
      // juice and soup. 14 is the script chapter: eleven more Cyrillic letters,
      // sorted into true friends, false friends and shapes with no Latin relative.
      // 14 -> 15: HL-C232, eight more letters on the same three-kinds frame. It exists
      // because а and о -- the two commonest vowels in the language -- were taught by no
      // lesson at all, which 75 lessons and 14 chapters had not surfaced.
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    expect(books.books.every((book) => book.chapters.every((chapter) => chapter.tex.length > 100))).toBe(true);
  });

  it("produces a machine-readable migration gap baseline", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    expect(report.schemaVersion).toBe(1);
    expect(report.durationModel.version).toBe(2);
    // 20 -> 21 in HL-C39 (Mandarin Chinese) -> 22 in HL-C40 (Japanese) -> 23
    // with the writing-first Marwadi starter. Each joined
    // the registry with its own authored book, so the track, schema, and
    // book-coverage counts all move together and stay equal to the registry.
    // Duration violations stay at zero: Chinese's seven and Japanese's eight
    // Chapter 1 lessons are each under 300 effective seconds.
    expect(report.summary.registeredTracks).toBe(23);
    expect(report.summary.totalLessons).toBe(lessons.length);
    expect(report.summary.authoredBooks).toBe(23);
    // HL-C134 made two previously hidden opening cliffs measurable:
    // KA-C01-namaskara (333s) and TE-C01-namaskaram (314s). Both lessons now
    // keep one greeting, one decode, one guided copy, and one etymology payoff
    // in 240 effective seconds. The ceiling was never raised to hide either.
    expect(report.summary.durationViolations).toBe(0);
    expect(report.summary.unknownPrerequisites).toBe(0);
    expect(report.schemas.tracks).toHaveLength(23);
    expect(report.books.tracks).toHaveLength(23);
  });

  it("compiles unique cross-language objective activities from canonical blocks", () => {
    const activities = lessons.flatMap((lesson) => compileLessonActivities(lesson.blocks));
    const ids = activities.map((activity) => activity.id);
    expect(activities.length).toBeGreaterThan(0);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.every((id) => id.length > 0 && id.trim() === id)).toBe(true);
  });

  it("keeps the Spanish Chapters 1-3 schema-v2 pilot closed and under five minutes", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    const pilot = lessons.filter(
      (lesson) =>
        lesson.language === "spanish" &&
        lesson.realization.chapter >= 1 &&
        lesson.realization.chapter <= 4,
    );
    // 24 before HL-C18; the tú/usted and cómo splits each added one micro-lesson.
    // Range widened 1-3 -> 1-4 when HL-C100 inserted `un`/`una` as a new chapter 3:
    // narrowing the count instead would have quietly dropped the original chapter 3
    // (`me llamo`) out of the pilot's coverage, which is the thing this guards.
    // 20 -> 24 when the pre-A1 writing runway (HL19) put ES-W00-hola-observe /
    // -guided-copy / -delayed-copy / -dictation into chapter 1. Spanish had zero
    // `hl-writing-stage` evidence before that, so the four are the whole of its
    // observe-trace -> dictation ladder, not padding.
    expect(pilot).toHaveLength(24) // HL-C94: these chapters are short on purpose;
    expect(pilot.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
    expect(
      report.duration.violations.filter(
        (lesson) => lesson.language === "spanish" && (lesson.chapter ?? 0) <= 3,
      ),
    ).toEqual([]);
    expect(
      report.prerequisites.laterChapterWithoutPrerequisites.filter(
        (lesson) => lesson.language === "spanish" && (lesson.chapter ?? 0) <= 3,
      ),
    ).toEqual([]);
  });

  it("keeps the Persian and Urdu Chapter 3 chains closed, objective, and under five minutes", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    for (const language of ["persian", "urdu"]) {
      const chapter = lessons.filter(
        (lesson) => lesson.language === language && lesson.realization.chapter === 3,
      );
      expect(chapter).toHaveLength(5);
      expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
      expect(chapter.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
      expect(
        report.duration.violations.filter(
          (lesson) => lesson.language === language && lesson.chapter === 3,
        ),
      ).toEqual([]);
      expect(
        report.prerequisites.laterChapterWithoutPrerequisites.filter(
          (lesson) => lesson.language === language && lesson.chapter === 3,
        ),
      ).toEqual([]);
    }
  });

  it("keeps the Persian and Urdu Chapter 4 wellbeing chains closed, objective, and under five minutes", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    for (const language of ["persian", "urdu"]) {
      const chapter = lessons.filter(
        (lesson) => lesson.language === language && lesson.realization.chapter === 4,
      );
      expect(chapter).toHaveLength(6);
      expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
      expect(chapter.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
      expect(
        report.duration.violations.filter(
          (lesson) => lesson.language === language && lesson.chapter === 4,
        ),
      ).toEqual([]);
      expect(
        report.prerequisites.laterChapterWithoutPrerequisites.filter(
          (lesson) => lesson.language === language && lesson.chapter === 4,
        ),
      ).toEqual([]);
    }
  });

  it("keeps the Persian and Urdu Chapter 5 farewell chains closed, objective, and under five minutes", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    for (const language of ["persian", "urdu"]) {
      const chapter = lessons.filter(
        (lesson) => lesson.language === language && lesson.realization.chapter === 5,
      );
      expect(chapter).toHaveLength(4);
      expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
      expect(chapter.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
      expect(
        report.duration.violations.filter(
          (lesson) => lesson.language === language && lesson.chapter === 5,
        ),
      ).toEqual([]);
      expect(
        report.prerequisites.laterChapterWithoutPrerequisites.filter(
          (lesson) => lesson.language === language && lesson.chapter === 5,
        ),
      ).toEqual([]);
    }

    const persianFarewell = lessons.find(
      (lesson) => lesson.realization.lessonId === "FA-C05-khodahafez",
    )!;
    const urduFarewell = lessons.find(
      (lesson) => lesson.realization.lessonId === "UR-C05-khuda-hafiz",
    )!;
    expect(persianFarewell.frontmatter.headword).toBe("خداحافظ");
    expect(urduFarewell.frontmatter.headword).toBe("خدا حافظ");
  });

  it("keeps the Japanese script-before-decoding chain closed and under five minutes", () => {
    // Japanese needs three writing systems, but putting all three in Chapter 1 made
    // the learner decode before the sign lessons. Pin the repaired structure: eight
    // small chapters, one objective activity per lesson, and a final mixed-script
    // exchange only after the hiragana, kanji and katakana ramps.
    const report = buildCurriculumGapReport({ registry, lessons, books });
    const japanese = lessons.filter((lesson) => lesson.language === "japanese");
    expect(japanese).toHaveLength(44);
    expect(new Set(japanese.map((lesson) => lesson.realization.chapter))).toEqual(
      new Set([1, 2, 3, 4, 5, 6, 7, 8]),
    );
    expect(japanese.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
    expect(japanese.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
    expect(
      report.duration.violations.filter((lesson) => lesson.language === "japanese"),
    ).toEqual([]);
    expect(
      report.prerequisites.laterChapterWithoutPrerequisites.filter(
        (lesson) => lesson.language === "japanese",
      ),
    ).toEqual([]);

    const headwords = new Map(
      japanese.map((lesson) => [lesson.realization.lessonId, lesson.realization.headword]),
    );
    expect(headwords.get("JA-C01-konnichiwa")).toBe("こんにちは"); // hiragana
    expect(headwords.get("JA-C01-nihongo")).toBe("日本語"); // kanji
    expect(headwords.get("JA-C01-koohii")).toBe("コーヒー"); // katakana
    // The register field carries two genuinely different grammatical levels here,
    // not two synonyms, so the plain and polite thanks must not collapse into one.
    const plain = lessons.find((lesson) => lesson.realization.lessonId === "JA-C01-arigatou")!;
    const polite = lessons.find((lesson) => lesson.realization.lessonId === "JA-C01-gozaimasu")!;
    expect(plain.frontmatter.register).toBe("plain-casual");
    expect(polite.frontmatter.register).toBe("teineigo-polite");
  });

  it("GREETING-HELLO joins every track (the normalization payoff)", () => {
    // Every track realizes 'hello', so the join size tracks the track count.
    const langs = languagesForConcept(dataset, "GREETING-HELLO").map((r) => r.language);
    expect(new Set(langs).size).toBe(dataset.languages.length);
  });

  it("the self-introduction concepts join many languages", () => {
    expect(languagesForConcept(dataset, "INTRO-MY-NAME-IS").length).toBeGreaterThanOrEqual(8);
    expect(languagesForConcept(dataset, "INTRO-WHATS-YOUR-NAME").length).toBeGreaterThanOrEqual(8);
  });

  it("every concept id is canonical or namespaced", () => {
    const NS = /^[A-Z]{2}-[A-Z0-9-]+$/;
    for (const c of dataset.concepts) {
      const ok = c.id in taxonomy.concepts || NS.test(c.id);
      expect(ok, `bad concept id: ${c.id}`).toBe(true);
    }
  });
});
