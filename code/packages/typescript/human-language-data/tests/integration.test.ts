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
    expect(books.books.find((book) => book.language === "spanish")?.chapters.length).toBe(152); // +4: HL-C98 // +5: HL-C99 splits the four mind-verbs into a chapter each, plus review and synthesis
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
      "ES-C02-concordancia-why-buenas",
      "ES-C02-concordancia-why-buenos",
      "ES-C02-repaso-articulos-an-afternoon",
      "ES-C02-repaso-articulos-the-day",
      "ES-C02-repaso-articulos-where",
      "ES-C02-un-contrast",
      "ES-C02-un-form",
      "ES-C02-un-number",
      "ES-C02-una-tarde",
      "ES-C02-una-which",
      "ES-C02-una-worn",
      "ES-C06-ar-presente-mix-1",
      "ES-C06-ar-presente-mix-2",
      "ES-C06-ar-presente-mix-3",
      "ES-C06-habla-form",
      "ES-C06-habla-usted",
      "ES-C06-hablas-contrast",
      "ES-C06-hablas-form",
      "ES-C06-hablo-ending",
      "ES-C06-hablo-form",
      "ES-C06-pro-drop-count",
      "ES-C06-pro-drop-why",
      "ES-C06-repaso-hablar-friend",
      "ES-C06-repaso-hablar-polite",
      "ES-C06-repaso-hablar-self",
      "ES-C07-beber-coffee",
      "ES-C07-come-form",
      "ES-C07-come-usted",
      "ES-C07-comeis-vivis-count",
      "ES-C07-comeis-vivis-eat",
      "ES-C07-comeis-vivis-live",
      "ES-C07-comemos-form",
      "ES-C07-comemos-part",
      "ES-C07-comen-viven-eat",
      "ES-C07-comen-viven-live",
      "ES-C07-comen-viven-where",
      "ES-C07-comer-family",
      "ES-C07-comes-form",
      "ES-C07-comes-s",
      "ES-C07-como-form",
      "ES-C07-como-same",
      "ES-C07-donde-live-in-madrid",
      "ES-C07-hablais-form",
      "ES-C07-hablais-where",
      "ES-C07-hablais-why",
      "ES-C07-hablamos-form",
      "ES-C07-hablamos-latin",
      "ES-C07-hablan-form",
      "ES-C07-hablan-ustedes",
      "ES-C07-hablan-why",
      "ES-C07-practice-ask-what",
      "ES-C07-practice-live-in-madrid",
      "ES-C07-practice-yo-forms",
      "ES-C07-que-ask-drinking",
      "ES-C07-repaso-comer-diff",
      "ES-C07-repaso-comer-i",
      "ES-C07-repaso-comer-you",
      "ES-C07-repaso-presente-completo-count",
      "ES-C07-repaso-presente-completo-they",
      "ES-C07-repaso-presente-completo-we",
      "ES-C07-repaso-tres-familias-diff",
      "ES-C07-repaso-tres-familias-live",
      "ES-C07-repaso-tres-familias-yo",
      "ES-C07-sintesis-dos-continentes-bogota",
      "ES-C07-sintesis-dos-continentes-madrid",
      "ES-C07-sintesis-dos-continentes-which",
      "ES-C07-sintesis-preguntar-families",
      "ES-C07-sintesis-preguntar-what",
      "ES-C07-sintesis-preguntar-where",
      "ES-C07-vivimos-contrast",
      "ES-C07-vivimos-form",
      "ES-C07-vivir-singular-forms",
      "ES-C08-cuantos-anos-ask",
      "ES-C08-cuantos-anos-eight",
      "ES-C08-numeros-1-5-coffee",
      "ES-C08-numeros-1-5-count",
      "ES-C08-numeros-6-10-count",
      "ES-C08-numeros-6-10-month-change",
      "ES-C08-practice-age-nine",
      "ES-C08-practice-count-ten",
      "ES-C08-practice-tener-singular",
      "ES-C08-tenemos-form",
      "ES-C08-tenemos-why",
      "ES-C08-tener-singular-forms",
      "ES-C08-tienen-boot",
      "ES-C08-tienen-form",
      "ES-C08-tienen-rule",
      "ES-C09-esta-en-ana",
      "ES-C09-esta-en-ask-formal",
      "ES-C09-estais-form",
      "ES-C09-estais-where",
      "ES-C09-estamos-form",
      "ES-C09-estamos-why",
      "ES-C09-estan-accent",
      "ES-C09-estan-form",
      "ES-C09-practice-identity",
      "ES-C09-practice-location",
      "ES-C09-practice-origin",
      "ES-C09-practice-state",
      "ES-C09-repaso-ser-estar-they",
      "ES-C09-repaso-ser-estar-we",
      "ES-C09-repaso-ser-estar-why",
      "ES-C09-repaso-ser-odd",
      "ES-C09-repaso-ser-they",
      "ES-C09-repaso-ser-we",
      "ES-C09-ser-identity",
      "ES-C09-ser-singular-forms",
      "ES-C09-ser-vs-estar-identify",
      "ES-C09-ser-vs-estar-state",
      "ES-C09-sois-form",
      "ES-C09-sois-where",
      "ES-C09-somos-form",
      "ES-C09-somos-why",
      "ES-C09-son-form",
      "ES-C09-son-latin",
      "ES-C09-soy-de-ask-origin",
      "ES-C09-soy-de-madrid",
      "ES-C10-ir-a-futuro-vas-comer",
      "ES-C10-ir-a-futuro-voy-hablar",
      "ES-C10-ir-formal-question",
      "ES-C10-ir-singular-forms",
      "ES-C10-mi-tu-su-formal-your-coffee",
      "ES-C10-mi-tu-su-my-coffee",
      "ES-C10-practice-formal-going",
      "ES-C10-practice-ir-forms",
      "ES-C10-practice-near-future",
      "ES-C10-practice-possessive",
      "ES-C10-vamos-english",
      "ES-C10-vamos-form",
      "ES-C10-vamos-why",
      "ES-C10-van-form",
      "ES-C10-van-four",
      "ES-C10-van-lesson",
      "ES-C11-nuestro-our-day",
      "ES-C11-nuestro-our-night",
      "ES-C11-poder-can-eat",
      "ES-C11-poder-singular-forms",
      "ES-C11-practice-can-eat",
      "ES-C11-practice-our-night",
      "ES-C11-practice-poder-forms",
      "ES-C11-practice-querer-forms",
      "ES-C11-practice-want-to-speak",
      "ES-C11-querer-singular-forms",
      "ES-C11-querer-want-to-speak",
      "ES-C11-stem-changes-tu-contrast",
      "ES-C11-stem-changes-yo-contrast",
      "ES-C12-decir-how-do-you-say",
      "ES-C12-decir-say-hello",
      "ES-C12-decir-singular-forms",
      "ES-C12-hacer-make-coffee",
      "ES-C12-hacer-singular-forms",
      "ES-C12-practice-decir-forms",
      "ES-C12-practice-hacer-forms",
      "ES-C12-practice-how-say-hello",
      "ES-C12-practice-make-coffee",
      "ES-C12-practice-yo-go-order",
      "ES-C12-yo-go-hacer-choice",
      "ES-C12-yo-go-learned-order",
      "ES-C13-plurales-cambio-can",
      "ES-C13-plurales-cambio-derive",
      "ES-C13-plurales-cambio-want",
      "ES-C13-plurales-yo-go-out",
      "ES-C13-plurales-yo-go-say",
      "ES-C13-plurales-yo-go-we",
      "ES-C13-poner-meaning",
      "ES-C13-poner-singular-forms",
      "ES-C13-practice-leave-come",
      "ES-C13-practice-poner-forms",
      "ES-C13-practice-root-families",
      "ES-C13-practice-salir-forms",
      "ES-C13-practice-six-yo-forms",
      "ES-C13-practice-venir-forms",
      "ES-C13-repaso-yo-go-count",
      "ES-C13-repaso-yo-go-slot",
      "ES-C13-repaso-yo-go-why",
      "ES-C13-salir-leave-madrid",
      "ES-C13-salir-singular-forms",
      "ES-C13-sintesis-derivar-count",
      "ES-C13-sintesis-derivar-rules",
      "ES-C13-sintesis-derivar-they",
      "ES-C13-sintesis-yo-go-me",
      "ES-C13-sintesis-yo-go-them",
      "ES-C13-sintesis-yo-go-which",
      "ES-C13-venir-come-from-madrid",
      "ES-C13-venir-singular-forms",
      "ES-C13-venir-tener-contrast",
      "ES-C14-hablar-preterite-hable-espanol",
      "ES-C14-hablar-preterite-present-past",
      "ES-C14-hablar-preterite-singular-forms",
      "ES-C14-practice-ar-forms",
      "ES-C14-practice-context-ser-ir",
      "ES-C14-practice-present-past",
      "ES-C14-practice-ser-ir-forms",
      "ES-C14-practice-written-accents",
      "ES-C14-ser-ir-preterite-context",
      "ES-C14-ser-ir-preterite-fui-madrid",
      "ES-C14-ser-ir-preterite-singular-forms",
      "ES-C15-comer-preterite-history",
      "ES-C15-comer-preterite-past-choice",
      "ES-C15-comer-preterite-singular-forms",
      "ES-C15-comer-vivir-preterite-shared-endings",
      "ES-C15-comer-vivir-preterite-vivi-madrid",
      "ES-C15-comer-vivir-preterite-vivir-forms",
      "ES-C15-hacer-preterite-hice-cafe",
      "ES-C15-hacer-preterite-history",
      "ES-C15-hacer-preterite-singular-forms",
      "ES-C15-practice-accent-contrast",
      "ES-C15-practice-estuve-bien",
      "ES-C15-practice-hice-cafe",
      "ES-C15-practice-history-match",
      "ES-C15-practice-regular-yo-forms",
      "ES-C15-practice-strong-yo-forms",
      "ES-C15-practice-vivi-madrid",
      "ES-C15-preterite-fuertes-analogy",
      "ES-C15-preterite-fuertes-estar-forms",
      "ES-C15-preterite-fuertes-estuve-bien",
      "ES-C15-sintesis-preterito-accent",
      "ES-C15-sintesis-preterito-families",
      "ES-C15-sintesis-preterito-stress",
      "ES-C15-tener-preterite-history",
      "ES-C15-tener-preterite-singular-forms",
      "ES-C15-tener-preterite-tuve-cafe",
      "ES-C16-comer-imperfecto-comi-comia",
      "ES-C16-comer-imperfecto-singular-forms",
      "ES-C16-comer-imperfecto-syllables",
      "ES-C16-imperfecto-hable-hablaba",
      "ES-C16-imperfecto-history",
      "ES-C16-imperfecto-singular-forms",
      "ES-C16-ir-imperfecto-history",
      "ES-C16-ir-imperfecto-iba-madrid",
      "ES-C16-ir-imperfecto-singular-forms",
      "ES-C16-practice-comi-comia",
      "ES-C16-practice-hablaba-espanol",
      "ES-C16-practice-history-match",
      "ES-C16-practice-iba-madrid",
      "ES-C16-practice-regular-yo-forms",
      "ES-C16-practice-short-yo-forms",
      "ES-C16-practice-vivia-madrid",
      "ES-C16-ser-imperfecto-era-fue",
      "ES-C16-ser-imperfecto-history",
      "ES-C16-ser-imperfecto-singular-forms",
      "ES-C16-sintesis-imperfecto-count",
      "ES-C16-sintesis-imperfecto-which",
      "ES-C16-sintesis-imperfecto-why",
      "ES-C16-ver-history",
      "ES-C16-ver-imperfecto-extra-e",
      "ES-C16-ver-imperfecto-singular-forms",
      "ES-C16-ver-imperfecto-veo-veia",
      "ES-C16-ver-meaning",
      "ES-C16-ver-present-singular",
      "ES-C16-vivir-imperfecto-shared-row",
      "ES-C16-vivir-imperfecto-singular-forms",
      "ES-C16-vivir-imperfecto-vivia-madrid",
      "ES-C17-comer-condicional-future-pair",
      "ES-C17-comer-condicional-shared-endings",
      "ES-C17-comer-condicional-singular-forms",
      "ES-C17-comer-futuro-cafe",
      "ES-C17-comer-futuro-shared-endings",
      "ES-C17-comer-futuro-singular-forms",
      "ES-C17-condicional-hablaria-espanol",
      "ES-C17-condicional-singular-forms",
      "ES-C17-condicional-syllables",
      "ES-C17-futuro-hablare-espanol",
      "ES-C17-futuro-history",
      "ES-C17-futuro-singular-forms",
      "ES-C17-irregulares-hacer-pair",
      "ES-C17-irregulares-poder-pair",
      "ES-C17-irregulares-tener-pair",
      "ES-C17-practice-hablare-espanol",
      "ES-C17-practice-hacer-pair",
      "ES-C17-practice-history",
      "ES-C17-practice-regular-conditional-yo",
      "ES-C17-practice-regular-future-yo",
      "ES-C17-practice-three-stems",
      "ES-C17-practice-viviria-madrid",
      "ES-C17-sintesis-futuro-condicional-promise",
      "ES-C17-sintesis-futuro-condicional-suppose",
      "ES-C17-sintesis-futuro-condicional-why",
      "ES-C17-vivir-condicional-madrid",
      "ES-C17-vivir-condicional-singular-forms",
      "ES-C17-vivir-condicional-three-yo",
      "ES-C17-vivir-futuro-madrid",
      "ES-C17-vivir-futuro-singular-forms",
      "ES-C17-vivir-futuro-three-yo",
      "ES-C18-comer-subjuntivo-fact-wish",
      "ES-C18-comer-subjuntivo-forms",
      "ES-C18-comer-subjuntivo-wanted",
      "ES-C18-hacer-subjuntivo-forms",
      "ES-C18-hacer-subjuntivo-stem",
      "ES-C18-hacer-subjuntivo-wanted",
      "ES-C18-ojala-no-que",
      "ES-C18-ojala-source",
      "ES-C18-ojala-trigger",
      "ES-C18-poder-subjuntivo-forms",
      "ES-C18-poder-subjuntivo-stem",
      "ES-C18-poder-subjuntivo-wanted",
      "ES-C18-practice-fact-wanted",
      "ES-C18-practice-hacer",
      "ES-C18-practice-ojala",
      "ES-C18-practice-ojala-source",
      "ES-C18-practice-one-two",
      "ES-C18-practice-poder",
      "ES-C18-practice-querer",
      "ES-C18-practice-regular-rows",
      "ES-C18-practice-subjunctive-name",
      "ES-C18-practice-yo-stem",
      "ES-C18-querer-subjuntivo-forms",
      "ES-C18-querer-subjuntivo-stem",
      "ES-C18-querer-subjuntivo-wanted",
      "ES-C18-quiero-que-meaning",
      "ES-C18-quiero-que-one-doer",
      "ES-C18-quiero-que-two-doers",
      "ES-C18-sintesis-subjuntivo-assert",
      "ES-C18-sintesis-subjuntivo-ojala",
      "ES-C18-sintesis-subjuntivo-wish",
      "ES-C18-subjuntivo-forms",
      "ES-C18-subjuntivo-name",
      "ES-C18-subjuntivo-recipe",
      "ES-C18-vivir-subjuntivo-er-ir-match",
      "ES-C18-vivir-subjuntivo-forms",
      "ES-C18-vivir-subjuntivo-wanted",
      "ES-C34-repaso-mente-gather",
      "ES-C34-repaso-mente-scratch",
      "ES-C34-repaso-mente-weigh",
      "ES-C34-sintesis-mente-literal",
      "ES-C34-sintesis-mente-now",
      "ES-C35-repaso-tres-aid",
      "ES-C35-repaso-tres-nostory",
      "ES-C35-repaso-tres-who",
      "ES-C35-sintesis-gustar-joke",
      "ES-C35-sintesis-gustar-subject",
      "ES-C35-sintesis-gustar-taste",
      "ES-C39-repaso-cambios-count",
      "ES-C39-repaso-cambios-ei",
      "ES-C39-repaso-cambios-only",
      "ES-C39-sintesis-elegir-ask",
      "ES-C39-sintesis-elegir-bring",
      "ES-C39-sintesis-elegir-know",
      "ES-C40-sintesis-describir-article",
      "ES-C40-sintesis-describir-have",
      "ES-C40-sintesis-describir-weather",
      "ES-C42-la-agree",
      "ES-C42-la-form",
      "ES-C42-la-why",
      "ES-C42-lo-origin",
      "ES-C42-lo-place",
      "ES-C42-lo-replace",
      "ES-C42-me-objeto-difference",
      "ES-C42-me-objeto-new",
      "ES-C42-me-objeto-say",
      "ES-C42-repaso-objeto-four",
      "ES-C42-repaso-objeto-hard",
      "ES-C42-repaso-objeto-question",
      "ES-C42-sintesis-decirlo-una-vez-book",
      "ES-C42-sintesis-decirlo-una-vez-meal",
      "ES-C42-sintesis-decirlo-una-vez-people",
      "ES-C42-sintesis-decirlo-una-vez-point",
      "ES-C42-te-english",
      "ES-C42-te-meaning",
      "ES-C42-te-say",
      "ES-C43-casa-article",
      "ES-C43-casa-french",
      "ES-C43-casa-have",
      "ES-C43-comida-build",
      "ES-C43-comida-gender",
      "ES-C43-comida-make",
      "ES-C43-libro-article",
      "ES-C43-libro-bark",
      "ES-C43-libro-have",
      "ES-C44-las-houses",
      "ES-C44-las-meals",
      "ES-C44-las-rule",
      "ES-C44-los-books",
      "ES-C44-los-four",
      "ES-C44-los-why",
      "ES-C44-plural-es-already",
      "ES-C44-plural-es-espanol",
      "ES-C44-plural-es-why",
      "ES-C44-repaso-articulos-four",
      "ES-C44-repaso-articulos-grid",
      "ES-C44-repaso-articulos-rule",
      "ES-C44-sintesis-mas-de-uno-bare",
      "ES-C44-sintesis-mas-de-uno-contrast",
      "ES-C44-sintesis-mas-de-uno-definite",
      "ES-C44-sintesis-mas-de-uno-meals",
      "ES-C45-los-las-books",
      "ES-C45-los-las-meals",
      "ES-C45-los-las-tidy",
      "ES-C45-nos-ending",
      "ES-C45-nos-say",
      "ES-C45-nos-word",
      "ES-C45-os-other",
      "ES-C45-os-say",
      "ES-C45-os-vos",
      "ES-C45-repaso-ocho-all",
      "ES-C45-repaso-ocho-free",
      "ES-C45-repaso-ocho-quarter",
      "ES-C45-sintesis-sin-repetir-books",
      "ES-C45-sintesis-sin-repetir-cost",
      "ES-C45-sintesis-sin-repetir-meals",
      "ES-C45-sintesis-sin-repetir-us",
      "ES-C46-cual-pide-have",
      "ES-C46-cual-pide-leismo",
      "ES-C46-cual-pide-speak",
      "ES-C46-cual-pide-test",
      "ES-C46-le-case",
      "ES-C46-le-gender",
      "ES-C46-le-say",
      "ES-C46-les-count",
      "ES-C46-les-same",
      "ES-C46-les-say",
      "ES-C46-repaso-dos-sistemas-count",
      "ES-C46-repaso-dos-sistemas-indirect",
      "ES-C46-repaso-dos-sistemas-row",
      "ES-C46-sintesis-dar-y-decir-book",
      "ES-C46-sintesis-dar-y-decir-person",
      "ES-C46-sintesis-dar-y-decir-plural",
      "ES-C46-sintesis-dar-y-decir-what",
      "ES-C47-me-lo-meal",
      "ES-C47-me-lo-order",
      "ES-C47-me-lo-say",
      "ES-C47-por-que-se-old",
      "ES-C47-por-que-se-two",
      "ES-C47-por-que-se-why",
      "ES-C47-repaso-dobles-new",
      "ES-C47-repaso-dobles-two",
      "ES-C47-repaso-dobles-us",
      "ES-C47-se-lo-ambiguous",
      "ES-C47-se-lo-say",
      "ES-C47-se-lo-when",
      "ES-C47-sintesis-dos-a-la-vez-cost",
      "ES-C47-sintesis-dos-a-la-vez-count",
      "ES-C47-sintesis-dos-a-la-vez-her",
      "ES-C47-sintesis-dos-a-la-vez-you",
      "ES-C48-cuando-family",
      "ES-C48-cuando-say",
      "ES-C48-cuando-which",
      "ES-C48-dos-miradas-inside",
      "ES-C48-dos-miradas-outside",
      "ES-C48-dos-miradas-which",
      "ES-C48-repaso-dos-pasados-question",
      "ES-C48-repaso-dos-pasados-scene",
      "ES-C48-repaso-dos-pasados-state",
      "ES-C48-sintesis-una-historia-layers",
      "ES-C48-sintesis-una-historia-tell",
      "ES-C48-sintesis-una-historia-third",
      "ES-C48-sintesis-una-historia-why",
      "ES-C48-tenia-tuve-got",
      "ES-C48-tenia-tuve-had",
      "ES-C48-tenia-tuve-rule",
      "ES-C49-haber-completo-all",
      "ES-C49-haber-completo-participle",
      "ES-C49-haber-completo-they",
      "ES-C49-haber-completo-why",
      "ES-C49-he-hablado-english",
      "ES-C49-he-hablado-mean",
      "ES-C49-he-hablado-say",
      "ES-C49-participio-ar",
      "ES-C49-participio-erir",
      "ES-C49-participio-person",
      "ES-C49-participios-irregulares-fact",
      "ES-C49-participios-irregulares-four",
      "ES-C49-participios-irregulares-use",
      "ES-C49-participios-irregulares-why",
      "ES-C49-repaso-perfecto-build",
      "ES-C49-repaso-perfecto-cost",
      "ES-C49-repaso-perfecto-irreg",
      "ES-C49-repaso-perfecto-pieces",
      "ES-C49-sintesis-dos-orillas-build",
      "ES-C49-sintesis-dos-orillas-now",
      "ES-C49-sintesis-dos-orillas-split",
      "ES-C49-sintesis-dos-orillas-wrong",
      "ES-C50-habla-emperor",
      "ES-C50-habla-rule",
      "ES-C50-habla-say",
      "ES-C50-hable-usted-grid",
      "ES-C50-hable-usted-say",
      "ES-C50-hable-usted-why",
      "ES-C50-no-hables-eat",
      "ES-C50-no-hables-say",
      "ES-C50-no-hables-why",
      "ES-C50-ocho-cortos-come",
      "ES-C50-ocho-cortos-list",
      "ES-C50-ocho-cortos-why",
      "ES-C50-repaso-mandatos-cost",
      "ES-C50-repaso-mandatos-grid",
      "ES-C50-repaso-mandatos-odd",
      "ES-C50-sintesis-pedir-bien-friend",
      "ES-C50-sintesis-pedir-bien-mistake",
      "ES-C50-sintesis-pedir-bien-stranger",
      "ES-C50-sintesis-pedir-bien-where",
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
      "UR-C09-bahan-agreement",
      "UR-C09-khandan-sort",
      "UR-C10-munh-nun",
      "UR-C10-naak-reversal",
      "UR-C11-dil-cousin",
      "UR-C12-doodh-false-friend",
      "UR-C12-roti-pasand",
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
        lesson.realization.chapter <= 4,
    );
    // 24 before HL-C18; the tú/usted and cómo splits each added one micro-lesson.
    // Range widened 1-3 -> 1-4 when HL-C100 inserted `un`/`una` as a new chapter 3:
    // narrowing the count instead would have quietly dropped the original chapter 3
    // (`me llamo`) out of the pilot's coverage, which is the thing this guards.
    expect(pilot).toHaveLength(20) // HL-C94: these chapters are short on purpose;
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
