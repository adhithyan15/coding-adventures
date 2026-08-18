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
    expect(books.books.find((book) => book.language === "spanish")?.chapters.length).toBe(295); // +4: HL-C98 // +5: HL-C99 splits the four mind-verbs into a chapter each, plus review and synthesis // +3: HL-C88 slice 8 // +1: HL-C88 slice 9 (falsos amigos) // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
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
    // 20 -> 21 in HL-C39 (Mandarin Chinese) -> 22 in HL-C40 (Japanese). Each joined
    // the registry with its own authored book, so the track, schema, and
    // book-coverage counts all move together and stay equal to the registry.
    // Duration violations stay at zero: Chinese's seven and Japanese's eight
    // Chapter 1 lessons are each under 300 effective seconds.
    expect(report.summary.registeredTracks).toBe(22);
    expect(report.summary.totalLessons).toBe(lessons.length);
    expect(report.summary.authoredBooks).toBe(22);
    // 0 -> 2. HL-C134 carried the hand-written prose back into the lessons, and
    // KA-C01-namaskara (333s) and TE-C01-namaskaram (314s) crossed the
    // five-minute line. Worth being exact about what changed: NOT the lessons.
    // A reader of the book always read every one of those words -- the
    // `sounds`, `cousinweb` and `cognates` blocks were on the page all along.
    // What changed is that the markdown now knows about them, so the gate can
    // finally see a lesson it could not measure before.
    //
    // So this is not new debt, it is newly VISIBLE debt, and the fix is the one
    // the owner's own rule prescribes: where a lesson is too big it splits, and
    // it never gets compressed to fit. Logged as HL-C151 rather than absorbed
    // by quietly raising a threshold.
    expect(report.summary.durationViolations).toBe(2);
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
      "ES-C09-ario-decode",
      "ES-C09-ario-place",
      "ES-C09-ario-shift",
      "ES-C09-cansado-nocousin",
      "ES-C09-cansado-say",
      "ES-C09-cansado-which",
      "ES-C09-esta-en-ana",
      "ES-C09-esta-en-ask-formal",
      "ES-C09-estais-form",
      "ES-C09-estais-where",
      "ES-C09-estamos-form",
      "ES-C09-estamos-why",
      "ES-C09-estan-accent",
      "ES-C09-estan-form",
      "ES-C09-estudiante-build",
      "ES-C09-estudiante-decode",
      "ES-C09-estudiante-we",
      "ES-C09-falsos-amigos-actual",
      "ES-C09-falsos-amigos-exito",
      "ES-C09-falsos-amigos-method",
      "ES-C09-grande-cousin",
      "ES-C09-grande-invariant",
      "ES-C09-grande-order",
      "ES-C09-ista-decode",
      "ES-C09-ista-root",
      "ES-C09-ista-say",
      "ES-C09-ncia-article",
      "ES-C09-ncia-build",
      "ES-C09-ncia-decode",
      "ES-C09-oso-agree",
      "ES-C09-oso-decode",
      "ES-C09-oso-verb",
      "ES-C09-practice-identity",
      "ES-C09-practice-location",
      "ES-C09-practice-origin",
      "ES-C09-practice-state",
      "ES-C09-profesor-cousin",
      "ES-C09-profesor-say",
      "ES-C09-profesor-she",
      "ES-C09-repaso-ocho-gender",
      "ES-C09-repaso-ocho-reach",
      "ES-C09-repaso-ocho-sort",
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
      "ES-C09-sintesis-describir-both",
      "ES-C09-sintesis-describir-order",
      "ES-C09-sintesis-describir-shift",
      "ES-C09-sintesis-ocho-mix",
      "ES-C09-sintesis-ocho-new",
      "ES-C09-sintesis-ocho-read",
      "ES-C09-sintesis-trabajo-read",
      "ES-C09-sintesis-trabajo-turn",
      "ES-C09-sintesis-trabajo-we",
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
      "ES-C51-la-flecha-cause",
      "ES-C51-la-flecha-gift",
      "ES-C51-la-flecha-test",
      "ES-C51-la-flecha-why",
      "ES-C51-para-arrow",
      "ES-C51-para-when",
      "ES-C51-para-you",
      "ES-C51-por-favor",
      "ES-C51-por-per",
      "ES-C51-por-thanks",
      "ES-C51-repaso-flecha-both",
      "ES-C51-repaso-flecha-old",
      "ES-C51-repaso-flecha-two",
      "ES-C51-sintesis-regalo-motivo-cause",
      "ES-C51-sintesis-regalo-motivo-gift",
      "ES-C51-sintesis-regalo-motivo-thanks",
      "ES-C51-sintesis-regalo-motivo-wrong",
      "ES-C52-como-se-dice-ask",
      "ES-C52-como-se-dice-full",
      "ES-C52-como-se-dice-why",
      "ES-C52-se-habla-eat",
      "ES-C52-se-habla-sign",
      "ES-C52-se-habla-who",
      "ES-C52-se-venden-agree",
      "ES-C52-se-venden-origin",
      "ES-C52-se-venden-plural",
      "ES-C52-sintesis-el-letrero-authority",
      "ES-C52-sintesis-el-letrero-eat",
      "ES-C52-sintesis-el-letrero-read",
      "ES-C52-sintesis-el-letrero-unknown",
      "ES-C52-tres-ses-test",
      "ES-C52-tres-ses-two",
      "ES-C52-tres-ses-which",
      "ES-C53-lo-que-contrast",
      "ES-C53-lo-que-new",
      "ES-C53-lo-que-say",
      "ES-C53-que-no-se-cae-book",
      "ES-C53-que-no-se-cae-habit",
      "ES-C53-que-no-se-cae-house",
      "ES-C53-que-no-se-cae-why",
      "ES-C53-que-relativo-family",
      "ES-C53-que-relativo-join",
      "ES-C53-que-relativo-what",
      "ES-C53-repaso-que-drop",
      "ES-C53-repaso-que-hole",
      "ES-C53-repaso-que-none",
      "ES-C53-sintesis-frases-largas-ask",
      "ES-C53-sintesis-frases-largas-full",
      "ES-C53-sintesis-frases-largas-hole",
      "ES-C53-sintesis-frases-largas-why",
      "ES-C54-ha-mas-y-fossil",
      "ES-C54-ha-mas-y-french",
      "ES-C54-ha-mas-y-parts",
      "ES-C54-hay-many",
      "ES-C54-hay-one",
      "ES-C54-hay-que-form",
      "ES-C54-hay-que-pair",
      "ES-C54-hay-que-say",
      "ES-C54-hay-unique",
      "ES-C54-repaso-hay-never",
      "ES-C54-repaso-hay-small",
      "ES-C54-repaso-hay-three",
      "ES-C54-sintesis-lo-que-hay-ask",
      "ES-C54-sintesis-lo-que-hay-done",
      "ES-C54-sintesis-lo-que-hay-must",
      "ES-C54-sintesis-lo-que-hay-pick",
      "ES-C55-comieron-contrast",
      "ES-C55-comieron-say",
      "ES-C55-comieron-vivir",
      "ES-C55-fueron-say",
      "ES-C55-fueron-two",
      "ES-C55-fueron-why",
      "ES-C55-hablamos-dos-pair",
      "ES-C55-hablamos-dos-say",
      "ES-C55-hablamos-dos-which",
      "ES-C55-hablaron-accent",
      "ES-C55-hablaron-build",
      "ES-C55-hablaron-say",
      "ES-C55-la-bebo-drink",
      "ES-C55-la-bebo-test",
      "ES-C55-la-bebo-why",
      "ES-C55-repaso-preterito-both",
      "ES-C55-repaso-preterito-odd",
      "ES-C55-repaso-preterito-strong",
      "ES-C55-si-accent",
      "ES-C55-si-etymon",
      "ES-C55-si-futuro-say",
      "ES-C55-si-futuro-trap",
      "ES-C55-si-futuro-why",
      "ES-C55-si-say",
      "ES-C55-sintesis-condicion-habit",
      "ES-C55-sintesis-condicion-plan",
      "ES-C55-sintesis-condicion-two",
      "ES-C55-sintesis-preterito-completo-raw",
      "ES-C55-sintesis-preterito-completo-story",
      "ES-C55-sintesis-preterito-completo-we",
      "ES-C55-tuvieron-build",
      "ES-C55-tuvieron-say",
      "ES-C55-tuvieron-stress",
      "ES-C56-cion-gender",
      "ES-C56-cion-make",
      "ES-C56-cion-why",
      "ES-C56-dad-make",
      "ES-C56-dad-read",
      "ES-C56-dad-why",
      "ES-C56-hablara-build",
      "ES-C56-hablara-etymon",
      "ES-C56-hablara-why",
      "ES-C56-mente-fem",
      "ES-C56-mente-literal",
      "ES-C56-mente-make",
      "ES-C56-repaso-amigos-gender",
      "ES-C56-repaso-amigos-kinds",
      "ES-C56-repaso-amigos-three",
      "ES-C56-repaso-condiciones-bans",
      "ES-C56-repaso-condiciones-shift",
      "ES-C56-repaso-condiciones-si",
      "ES-C56-si-tuviera-implies",
      "ES-C56-si-tuviera-say",
      "ES-C56-si-tuviera-trap",
      "ES-C56-sintesis-condiciones-cost",
      "ES-C56-sintesis-condiciones-own",
      "ES-C56-sintesis-condiciones-ser",
      "ES-C56-sintesis-leer-count",
      "ES-C56-sintesis-leer-limit",
      "ES-C56-sintesis-leer-read",
      "ES-C56-sintesis-leer-shape",
      "ES-C56-tuviera-build",
      "ES-C56-tuviera-cost",
      "ES-C56-tuviera-ser",
      "ES-C57-dice-que-build",
      "ES-C57-dice-que-drop",
      "ES-C57-dice-que-echo",
      "ES-C57-dijo-letter",
      "ES-C57-dijo-plural",
      "ES-C57-dijo-que-shift",
      "ES-C57-dijo-que-why",
      "ES-C57-dijo-que-work",
      "ES-C57-dijo-say",
      "ES-C57-f-a-h-decode",
      "ES-C57-f-a-h-hablar",
      "ES-C57-f-a-h-limit",
      "ES-C57-f-a-h-why",
      "ES-C57-ll-claim",
      "ES-C57-ll-decode",
      "ES-C57-ll-shape",
      "ES-C57-ll-three",
      "ES-C57-pregunto-donde-accent",
      "ES-C57-pregunto-donde-join",
      "ES-C57-pregunto-donde-report",
      "ES-C57-pregunto-si-marks",
      "ES-C57-pregunto-si-report",
      "ES-C57-pregunto-si-same",
      "ES-C57-repaso-reportar-que",
      "ES-C57-repaso-reportar-si",
      "ES-C57-repaso-reportar-wh",
      "ES-C57-sintesis-descifrar-h",
      "ES-C57-sintesis-descifrar-limits",
      "ES-C57-sintesis-descifrar-ll",
      "ES-C57-sintesis-descifrar-mix",
      "ES-C57-sintesis-reportar-one",
      "ES-C57-sintesis-reportar-three",
      "ES-C57-sintesis-reportar-two",
      "ES-C58-pero-keeps",
      "ES-C58-pero-origin",
      "ES-C58-pero-say",
      "ES-C58-tambien-agree",
      "ES-C58-tambien-parts",
      "ES-C58-tambien-tan",
      "ES-C58-tampoco-agree",
      "ES-C58-tampoco-choose",
      "ES-C58-tampoco-parts",
      "ES-C59-aquel-ille",
      "ES-C59-aquel-say",
      "ES-C59-aquel-three",
      "ES-C59-ese-contrast",
      "ES-C59-ese-origin",
      "ES-C59-ese-say",
      "ES-C59-este-agree",
      "ES-C59-este-article",
      "ES-C59-este-say",
      "ES-C59-esto-eso-aquello-ask",
      "ES-C59-esto-eso-aquello-gender",
      "ES-C59-repaso-demostrativos-article",
      "ES-C59-repaso-demostrativos-noun",
      "ES-C59-repaso-demostrativos-side",
      "ES-C60-bastante-parts",
      "ES-C60-bastante-say",
      "ES-C60-bastante-scale",
      "ES-C60-mal-adj",
      "ES-C60-mal-pattern",
      "ES-C60-mal-say",
      "ES-C60-muy-alone",
      "ES-C60-muy-say",
      "ES-C60-muy-which",
      "ES-C60-repaso-grado-ante",
      "ES-C60-repaso-grado-scale",
      "ES-C60-repaso-grado-short",
      "ES-C61-al-must",
      "ES-C61-al-name",
      "ES-C61-al-say",
      "ES-C61-del-count",
      "ES-C61-del-la",
      "ES-C61-del-say",
      "ES-C61-ni-count",
      "ES-C61-ni-mean",
      "ES-C61-ni-say",
      "ES-C61-o-ask",
      "ES-C61-o-swap",
      "ES-C61-o-why",
      "ES-C61-quien-ask",
      "ES-C61-quien-latin",
      "ES-C61-quien-which",
      "ES-C61-repaso-juntores-count",
      "ES-C61-repaso-juntores-u",
      "ES-C61-repaso-juntores-who",
      "ES-C62-comiendo-count",
      "ES-C62-comiendo-er",
      "ES-C62-comiendo-ir",
      "ES-C62-estoy-hablando-future",
      "ES-C62-estoy-hablando-say",
      "ES-C62-estoy-hablando-which",
      "ES-C62-hablando-agree",
      "ES-C62-hablando-make",
      "ES-C62-hablando-mean",
      "ES-C62-repaso-gerundio-a",
      "ES-C62-repaso-gerundio-now",
      "ES-C62-repaso-gerundio-tomorrow",
      "ES-C62-veo-a-maria-person",
      "ES-C62-veo-a-maria-thing",
      "ES-C62-veo-a-maria-why",
      "ES-C63-comiamos-count",
      "ES-C63-comiamos-ser",
      "ES-C63-comiamos-we",
      "ES-C63-comisteis-done",
      "ES-C63-comisteis-er",
      "ES-C63-comisteis-strong",
      "ES-C63-hablabamos-accent",
      "ES-C63-hablabamos-they",
      "ES-C63-hablabamos-we",
      "ES-C63-hablasteis-make",
      "ES-C63-hablasteis-parts",
      "ES-C63-hablasteis-whole",
      "ES-C63-repaso-paradigmas-accent",
      "ES-C63-repaso-paradigmas-owed",
      "ES-C63-repaso-paradigmas-three",
      "ES-C64-conmigo-ask",
      "ES-C64-conmigo-third",
      "ES-C64-conmigo-twice",
      "ES-C64-maria-ven-mark",
      "ES-C64-maria-ven-noa",
      "ES-C64-maria-ven-say",
      "ES-C64-para-mi-accent",
      "ES-C64-para-mi-say",
      "ES-C64-para-mi-third",
      "ES-C64-que-grande-accent",
      "ES-C64-que-grande-good",
      "ES-C64-que-grande-say",
      "ES-C64-repaso-tonicos-comma",
      "ES-C64-repaso-tonicos-for",
      "ES-C64-repaso-tonicos-with",
      "ES-C65-ahi-alli-far",
      "ES-C65-ahi-alli-pair",
      "ES-C65-ahi-alli-yours",
      "ES-C65-ahora-hoy-inside",
      "ES-C65-ahora-hoy-now",
      "ES-C65-ahora-hoy-today",
      "ES-C65-repaso-parejas-bare",
      "ES-C65-repaso-parejas-dar",
      "ES-C65-repaso-parejas-hora",
      "ES-C65-unos-unas-bare",
      "ES-C65-unos-unas-fem",
      "ES-C65-unos-unas-say",
      "ES-C65-vi-di-accent",
      "ES-C65-vi-di-gave",
      "ES-C65-vi-di-saw",
      "ES-C65-vuestro-count",
      "ES-C65-vuestro-say",
      "ES-C65-vuestro-which",
      "ES-C66-hablo-dos-lecturas-both",
      "ES-C66-hablo-dos-lecturas-normal",
      "ES-C66-hablo-dos-lecturas-when",
      "ES-C66-la-terminacion-dice-quien-back",
      "ES-C66-la-terminacion-dice-quien-drop",
      "ES-C66-la-terminacion-dice-quien-rule",
      "ES-C66-mucha-agua-noun",
      "ES-C66-mucha-agua-test",
      "ES-C66-mucha-agua-verb",
      "ES-C66-nombres-propios-name",
      "ES-C66-nombres-propios-title",
      "ES-C66-nombres-propios-why",
      "ES-C66-repaso-lo-implicito-drop",
      "ES-C66-repaso-lo-implicito-prog",
      "ES-C66-repaso-lo-implicito-quant",
      "ES-C67-comer-es-bueno-long",
      "ES-C67-comer-es-bueno-say",
      "ES-C67-comer-es-bueno-trap",
      "ES-C67-hoy-como-en-casa-front",
      "ES-C67-hoy-como-en-casa-three",
      "ES-C67-hoy-como-en-casa-why",
      "ES-C67-primero-fem",
      "ES-C67-primero-first",
      "ES-C67-primero-second",
      "ES-C67-repaso-a1-apocope",
      "ES-C67-repaso-a1-inf",
      "ES-C67-repaso-a1-otro",
      "ES-C67-uno-otro-article",
      "ES-C67-uno-otro-latin",
      "ES-C67-uno-otro-split",
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
      "FA-C09-ab-request",
      "FA-C09-chay-request",
      "FA-C09-kelid-request",
      "FA-C09-nan-request",
      "FA-C10-baradar-meaning",
      "FA-C10-dokhtar-meaning",
      "FA-C10-madar-meaning",
      "FA-C10-pedar-meaning",
      "FA-C11-cheshm-meaning",
      "FA-C11-dast-meaning",
      "FA-C11-pa-meaning",
      "FA-C11-zaban-meaning",
      "FA-C12-dar-meaning",
      "FA-C12-del-meaning",
      "FA-C12-ketab-request",
      "FA-C12-nam-meaning",
      "FA-C13-aseman-meaning",
      "FA-C13-baran-meaning",
      "FA-C13-khorshid-meaning",
      "FA-C13-mah-meaning",
      "FA-C13-setare-meaning",
      "FA-C14-dust-meaning",
      "FA-C14-khahar-meaning",
      "FA-C14-mard-meaning",
      "FA-C14-pesar-meaning",
      "FA-C14-zan-meaning",
      "FA-C15-practice-vowels",
      "FA-W15-alef-direction",
      "FA-W15-be-dot",
      "FA-W15-he-final",
      "FA-W15-joining-break",
      "FA-W15-lam-vs-alef",
      "FA-W15-mim-tail",
      "FA-W15-sin-teeth",
      "FA-W15-te-nun-dots",
      "FA-W15-vav-join",
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
      "JA-C03-practice-ticks",
      "JA-W01-chi-beats",
      "JA-W01-e-beats",
      "JA-W01-ha-strokes",
      "JA-W01-hai-read-beats",
      "JA-W01-i-strokes",
      "JA-W01-ko-strokes",
      "JA-W01-konnichiwa-read-beats",
      "JA-W01-n-vowel",
      "JA-W01-ni-shared",
      "JA-W01-wa-particle",
      "JA-W03-a-strokes",
      "JA-W03-arigatou-read-ga",
      "JA-W03-dakuten-job",
      "JA-W03-ka-strokes",
      "JA-W03-ma-strokes",
      "JA-W03-ri-tap",
      "JA-W03-sa-za",
      "JA-W03-su-devoice",
      "JA-W03-to-join",
      "JA-W03-u-long",
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
      "RU-C15-practice-false",
      "RU-W07-a-kind",
      "RU-W07-f-borrowed",
      "RU-W07-g-sound",
      "RU-W07-kh-kind",
      "RU-W07-o-unstressed",
      "RU-W07-ts-tail",
      "RU-W07-y-short-base",
      "RU-W07-yu-pattern",
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
      "UR-C16-practice-vowels",
      "UR-W16-alef-cascade",
      "UR-W16-joining-gap",
      "UR-W16-kaf-lifts",
      "UR-W16-lam-bowl",
      "UR-W16-mim-head",
      "UR-W16-nun-dot",
      "UR-W16-sin-lifts",
      "UR-W16-ye-dots",
      "ZH-C01-hao-components",
      "ZH-C01-hao-fond-tone",
      "ZH-C01-ni-meaning",
      "ZH-C01-nihao-greeting",
      "ZH-C01-practice-greet",
      "ZH-C01-tone-sandhi-spoken",
      "ZH-C01-tones-count",
      "ZH-C03-er-tone",
      "ZH-C03-practice-lift",
      "ZH-C03-san-next",
      "ZH-C03-si-tone",
      "ZH-C03-wu-derive",
      "ZH-C03-yi-count",
      "ZH-C04-bu-no",
      "ZH-C04-kou-meaning",
      "ZH-C04-practice-double",
      "ZH-C04-ri-two",
      "ZH-C05-bushi-sandhi",
      "ZH-C05-practice-pair",
      "ZH-C05-shi-yes",
      "ZH-C06-practice-close",
      "ZH-C06-zaijian-literal",
      "ZH-W01-er-role",
      "ZH-W01-hao-build-halves",
      "ZH-W01-ni-build-halves",
      "ZH-W01-nu-strokes",
      "ZH-W01-ren-radical-meaning",
      "ZH-W01-ren-strokes",
      "ZH-W01-zi-meaning",
      "ZH-W03-er-longer",
      "ZH-W03-san-middle",
      "ZH-W03-si-last",
      "ZH-W03-wu-strokes",
      "ZH-W03-yi-strokes",
      "ZH-W04-bu-lifts",
      "ZH-W04-kou-last",
      "ZH-W04-ri-order",
      "ZH-W05-shi-top",
      "ZH-W06-jian-legs",
      "ZH-W06-zai-frame",
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
