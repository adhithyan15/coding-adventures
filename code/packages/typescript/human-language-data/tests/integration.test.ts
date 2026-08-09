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
    expect(books.books.find((book) => book.language === "spanish")?.chapters.length).toBe(41);
    expect(
      books.books
        .find((book) => book.language === "persian")
        ?.chapters.map((chapter) => chapter.chapter),
      // 6 -> 8: the eight-verb tranche added Chapters 7 and 8 (mind verbs, then
      // taking/asking/helping/loving), split 4+4 to stay inside maxNewAtomsPerChapter.
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
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
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
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
    ).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(books.books.every((book) => book.chapters.every((chapter) => chapter.tex.length > 100))).toBe(true);
  });

  it("produces a machine-readable migration gap baseline", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    expect(report.schemaVersion).toBe(1);
    expect(report.durationModel.version).toBe(2);
    // 20 -> 21 in HL-C39 (Mandarin Chinese) -> 22 in HL-C40 (Japanese). Each joined
    // the registry with its own authored book, so the track, schema, and
    // book-coverage counts all move together and stay equal to the registry.
    // Duration violations stay at zero: Chinese's seven and Japanese's eight
    // Chapter 1 lessons are each under 300 effective seconds.
    expect(report.summary.registeredTracks).toBe(22);
    expect(report.summary.totalLessons).toBe(lessons.length);
    expect(report.summary.authoredBooks).toBe(22);
    expect(report.summary.durationViolations).toBe(0);
    expect(report.summary.unknownPrerequisites).toBe(0);
    expect(report.schemas.tracks).toHaveLength(22);
    expect(report.books.tracks).toHaveLength(22);
  });

  it("compiles the cross-language objective activities from canonical blocks", () => {
    const activities = lessons.flatMap((lesson) => compileLessonActivities(lesson.blocks));
    expect(activities.map((activity) => activity.id).sort()).toEqual([
      "AR-W07-hook-family-ha-kha-dot-position",
      "BN-C10-jol-water",
      "ES-C01-genero-gramatical-class-count",
      "ES-C01-practice-buenos-agreement",
      "ES-W03-question-span-roberto-outside",
      "FA-C02-esm-e-man-sara",
      "FA-C03-chist-fusion",
      "FA-C03-esm-e-shoma-chist-question",
      "FA-C03-khoshvaghtam-close",
      "FA-C03-practice-next-line",
      "FA-C03-shoma-to-first-meeting",
      "FA-C04-chetor-how",
      "FA-C04-hal-e-shoma-chetor-ast-question",
      "FA-C04-hal-meaning",
      "FA-C04-khub-good",
      "FA-C04-khubam-reply",
      "FA-C04-practice-next-line",
      "FA-C05-hafez-meaning",
      "FA-C05-khoda-meaning",
      "FA-C05-khodahafez-goodbye",
      "FA-C05-practice-final-line",
      "FA-C06-amadan-verb",
      "FA-C06-budan-infinitive",
      "FA-C06-danestan-pair",
      "FA-C06-goftan-stem",
      "FA-C06-raftan-stem",
      "FA-C07-fahmidan-stem",
      "FA-C07-fekr-kardan-verb",
      "FA-C07-khandan-silent-vav",
      "FA-C07-neveshtan-stem",
      "FA-C08-dust-dashtan-literal",
      "FA-C08-gereftan-stem",
      "FA-C08-komak-kardan-verb",
      "FA-C08-porsidan-stem",
      "FR-C18-oui-negative",
      "GE-C17-kopf-haupt-compound-word",
      "GU-C06-number-histories-be-source",
      "HI-W01-shirorekha-na-ma-drawing-order",
      "IT-C03-practice-drop-io",
      "JA-C01-arigatou-dakuten",
      "JA-C01-gozaimasu-level",
      "JA-C01-hai-beats",
      "JA-C01-iie-length",
      "JA-C01-konnichiwa-particle",
      "JA-C01-koohii-bar",
      "JA-C01-nihongo-readings",
      "JA-C01-practice-final-line",
      "KA-C06-dative-stacking-agglutinative",
      "LA-C01-practice-vale-root",
      "LA-C19-practice-answer-line",
      "LA-C21-practice-name-reply",
      "LA-C33-practice-soir-family",
      "LA-C36-practice-afternoon-line",
      "ML-C23-naal-survival",
      "MR-C06-number-differences-don-ending",
      "PA-C06-panj-convergence-borrowing",
      "PA-C07-janna-two-js",
      // The Punjabi eight-verb tranche: one activity per lesson across Chapters 8
      // and 9, matching how the Persian and Urdu tranches carry theirs.
      "PA-C08-likhna-four-roots",
      "PA-C08-parhna-subjoined",
      "PA-C08-samajhna-tone",
      "PA-C08-sochna-soch",
      "PA-C09-laina-labhate",
      "PA-C09-madad-karna-root",
      "PA-C09-pasand-dative",
      "PA-C09-puchhna-addak",
      "PA-C10-chaa-tone",
      "PA-C10-paani-please",
      "PA-C10-roti-requests",
      "PA-C11-bhain-cognate",
      "PA-C11-bhara-tone",
      "PA-C11-parivar-split",
      "PA-C12-kann-false-friend",
      "PA-C12-munh-tone",
      "PA-C12-nakk-face",
      "PA-C13-dil-source",
      "PA-C13-sir-cousin",
      "PT-C02-practice-neutral-question",
      "RU-C02-kak-cross-language-what-language",
      "RU-C02-practice-informal-question",
      "RU-C02-vy-formality-safe-default",
      "SA-C06-number-cognates-inheritance",
      "TA-W01-curves-va-ka-writing-surface",
      "TE-C31-subha-madhyahnam-register-source-scope",
      "UR-C02-mera-naam-sara",
      "UR-C03-aap-ka-naam-kya-hai-question",
      "UR-C03-aap-tum-tu-first-meeting",
      "UR-C03-khushi-hui-close",
      "UR-C03-kya-what",
      "UR-C03-practice-next-line",
      "UR-C04-aap-kaise-hain-man",
      "UR-C04-kaise-kaisi-woman",
      "UR-C04-main-hun-frame",
      "UR-C04-main-thik-hun-reply",
      "UR-C04-practice-next-line",
      "UR-C04-thik-well",
      "UR-C05-hafiz-meaning",
      "UR-C05-khuda-hafiz-goodbye",
      "UR-C05-khuda-meaning",
      "UR-C05-practice-final-line",
      "UR-C07-likhna-four-stems",
      "UR-C08-pasand-dative",
      // HL-C41 continuation: the pre-A1 vocabulary tranche added one activity per
      // payoff/deeper lesson across chapters 9-12.
      "UR-C09-bahan-agreement",
      "UR-C09-khandan-sort",
      "UR-C10-munh-nun",
      "UR-C10-naak-reversal",
      "UR-C11-dil-cousin",
      "UR-C12-doodh-false-friend",
      "UR-C12-roti-pasand",
      // HL-C39 added Mandarin Chinese: one activity per Chapter 1 lesson, so the
      // corpus total moves from 51 to 57.
      "ZH-C01-hao-components",
      "ZH-C01-hao-fond-tone",
      "ZH-C01-ni-meaning",
      "ZH-C01-nihao-greeting",
      "ZH-C01-practice-greet",
      "ZH-C01-tone-sandhi-spoken",
      "ZH-C01-tones-count",
    ]);
    expect(activities.every((activity) => activity.assesses.length > 0)).toBe(true);
    expect(activities.every((activity) => activity.acceptedResponses.length > 0)).toBe(true);
  });

  it("keeps the Spanish Chapters 1-3 schema-v2 pilot closed and under five minutes", () => {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    const pilot = lessons.filter(
      (lesson) =>
        lesson.language === "spanish" &&
        lesson.realization.chapter >= 1 &&
        lesson.realization.chapter <= 3,
    );
    // 24 before HL-C18; the tú/usted and cómo splits each added one micro-lesson.
    expect(pilot).toHaveLength(26);
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

  it("keeps the Japanese Chapter 1 mixed-script chain closed and under five minutes", () => {
    // Japanese is the corpus's first track that needs more than one writing system
    // at a time, so this asserts the property rather than only the counts: the same
    // chapter must actually carry hiragana, katakana, and kanji headwords, and the
    // closing exchange must still be reachable without an untaught atom.
    const report = buildCurriculumGapReport({ registry, lessons, books });
    const chapter = lessons.filter(
      (lesson) => lesson.language === "japanese" && lesson.realization.chapter === 1,
    );
    expect(chapter).toHaveLength(8);
    expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
    expect(chapter.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
    expect(
      report.duration.violations.filter((lesson) => lesson.language === "japanese"),
    ).toEqual([]);
    expect(
      report.prerequisites.laterChapterWithoutPrerequisites.filter(
        (lesson) => lesson.language === "japanese",
      ),
    ).toEqual([]);

    const headwords = new Map(
      chapter.map((lesson) => [lesson.realization.lessonId, lesson.realization.headword]),
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
