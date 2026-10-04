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
import {
  measureGlyphGaps,
  type ScriptInventoryEvidenceModule,
} from "./script-inventories/helpers.js";
import { assertCorpusGlyphGapQueue } from "./script-inventory-queue.js";
import type { IntegrationTrackEvidenceModule } from "./integration-track-evidence/helpers.js";

const scriptInventoryModules = import.meta.glob<ScriptInventoryEvidenceModule>(
  "./script-inventories/**/*.evidence.ts",
  { eager: true },
);

const integrationTrackEvidenceModules = import.meta.glob<IntegrationTrackEvidenceModule>(
  "./integration-track-evidence/*.evidence.ts",
  { eager: true },
);

const {
  taxonomy,
  registry,
  spine,
  curricula,
  books,
  lessons,
  scripts,
  soundTags,
  dataset,
} = loadEverything();

const curriculumGapReport = buildCurriculumGapReport({
  registry,
  lessons,
  books,
});

describe("real curriculum", () => {
  it("registers a sound-tag vocabulary for every language track", () => {
    expect(Object.keys(soundTags.tracks).sort()).toEqual(
      registry.languages.map((language) => language.id).sort(),
    );
    // Spanish already teaches esdrújulas. Naming this legitimate extension in
    // the registry keeps the next lesson from having to masquerade as the
    // broader `accent-acute` tag merely to satisfy the closed vocabulary.
    expect(soundTags.tracks.spanish).toContain("stress-antepenultimate");
  });

  it("has zero validation errors", () => {
    const issues = validate({ taxonomy, lessons, scripts, soundTags });
    const errors = issues.filter((i) => i.level === "error");
    // Surface any error messages so a failure is self-explaining.
    expect(errors.map((e) => e.message)).toEqual([]);
    expect(hasErrors(issues)).toBe(false);
  });

  it("keeps script inventories source-backed and closure-pinned", () => {
    const gaps = measureGlyphGaps({ taxonomy, lessons, scripts });
    const context = { taxonomy, lessons, scripts, ...gaps };
    for (const [, module] of Object.entries(scriptInventoryModules).sort(
      ([left], [right]) => left.localeCompare(right),
    )) {
      module.scriptInventoryEvidence.assert(context);
    }
    assertCorpusGlyphGapQueue(context);
  });

  it("loaded every track (17+ and growing)", () => {
    expect(dataset.languages.length).toBeGreaterThanOrEqual(20);
    for (const t of [
      "spanish",
      "telugu",
      "arabic",
      "russian",
      "persian",
      "urdu",
    ]) {
      expect(dataset.languages).toContain(t);
    }
  });

  it("has a valid shared spine covering every registered language", () => {
    const issues = validateCurriculum({
      registry,
      spine,
      curricula,
      taxonomy,
      lessons,
      books,
    });
    expect(
      issues
        .filter((issue) => issue.level === "error")
        .map((issue) => issue.message),
    ).toEqual([]);
    expect(registry.languages.map((language) => language.id)).toEqual(
      dataset.languages.sort((a, b) => {
        const order = new Map(
          registry.languages.map((language, index) => [language.id, index]),
        );
        return order.get(a)! - order.get(b)!;
      }),
    );
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
      // HL-C417 adds the fifth: the A2 tranche above was four GRAMMAR nodes and
      // nothing else, so an A2 READING chapter had no rung to stand on and 44
      // Spanish vocabulary chapters -- whose wordlists came off a DELE A2 paper --
      // were parked on the A1 `SPINE-READ-SIGNS-AND-NOTICES` instead. That put A2
      // words inside the A1 cut-off, which is the defect, not the bookkeeping.
      "SPINE-READ-PRACTICAL-TEXTS",
      "SPINE-SAY-WHAT-I-DO",
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
    expect(curricula.map((curriculum) => curriculum.language).sort()).toEqual(
      registry.languages.map((language) => language.id).sort(),
    );
    expect(
      curricula.flatMap((curriculum) => curriculum.path).length,
    ).toBeGreaterThan(300);
    expect(
      curricula.flatMap((curriculum) =>
        curriculum.path.flatMap((segment) => segment.lessons),
      ).length,
    ).toBeGreaterThan(800);
    expect(
      curricula.every((curriculum) =>
        spine.nodes.every((node) => curriculum.spine[node.id] !== undefined),
      ),
    ).toBe(true);

    const spanish = curricula.find(
      (curriculum) => curriculum.language === "spanish",
    )!;
    expect(spanish.spine["SPINE-MEET-GREET"]?.segments.length).toBeGreaterThan(
      1,
    );
    expect(
      spanish.spine["SPINE-TAKE-LEAVE"]?.relocates["GREETING-GOODNIGHT"],
    ).toBe("SPINE-TIME-OF-DAY");

    for (const language of ["persian", "urdu"]) {
      const curriculum = curricula.find((item) => item.language === language)!;
      expect(
        curriculum.extensions.some(
          (extension) => extension.category === "script",
        ),
      ).toBe(true);
    }
  });

  it("preserves every existing LaTeX book and maps each chapter to short lessons", () => {
    expect(books.books.length).toBeGreaterThanOrEqual(20);
    expect(
      books.books.reduce((sum, book) => sum + book.chapters.length, 0),
    ).toBeGreaterThanOrEqual(100);
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
    expect(
      books.books.find((book) => book.language === "spanish")?.chapters.length,
    ).toBeGreaterThanOrEqual(305); // spanish pre-A1 survival tranche: +15 lessons, +3 chapters (chapters 303-305) // +4: HL-C98 // +5: HL-C99 splits the four mind-verbs into a chapter each, plus review and synthesis // +3: HL-C88 slice 8 // +1: HL-C88 slice 9 (falsos amigos) // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // spanish pre-A1 round 3: +35 lessons, +7 chapters (chapters 296-302) // FLOOR — content only grows; exact pins serialize parallel tranches
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
      // 15 -> 20: HL-C350, the numeral tranche. Persian taught NO numeral at all in 71
      // lessons -- no age, no price, no telephone number, no time. Chapter 16 teaches
      // one to five, 17 six to ten with the Iranian s-to-h rule behind haft, 18 the
      // teens as a rule rather than nine words, 19 the ten Persian digits, and 20 the
      // ordinals, which cost one ending.
      // 20 -> 21: the reading rung. Three lessons built from the nine letters the
      // script ladder has taught and the ten digits -- which is why the chapter
      // ends on figures rather than prose: a price and a platform number are the
      // first Persian a reader can take off a page without knowing the letters.
      // 21 -> 26: HL-C443, one lesson per letter the reader had read in words and
      // never written -- eighteen letters in chapters 22-26, each from its word.
      // 26 -> 78: the pre-A1 vocabulary tranche, fifty-two chapters of five
      // words in three runs, each run closed by two reviews.
      // 78 -> 135: the A1 tranche, fifty-seven chapters of five words in six
      // runs (time, this and that, places, can, want, why, things, describing
      // words and verbs), each run closed by two reviews.
      // 135 -> 139: chapters 136-139 realize the last four A2 spine nodes (negation and questions, the past, the future, practical texts).
      // 139 -> 178: the first A2 vocabulary tranche, chapters 140-178: 195
      // headwords, fifty of them verbs, in five runs each closed by two
      // reviews.
    ).toEqual(Array.from({ length: 178 }, (_, i) => i + 1));
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
      // 16 -> 18: HL-C240, the second and third script chapters, and the first Urdu
      // ones that interleave rather than batch. Chapter 16 taught seven letters in
      // seven consecutive lessons; 17 and 18 each spend ten lessons alternating one
      // letter with glossed vocabulary and review, so a word is met by ear two or
      // more lessons before the glyph that spells it arrives. Both stay inside the
      // 12-atom chapter budget (eleven each) and both file under SPINE-RESPOND-BASIC.
      // 18 -> 22: the negation-and-joining tranche. Chapter 19 redeems the deferral
      // chapter 1 made in as many words and files under SPINE-NEGATE-AND-ASK, emptying
      // that node's omission ledger; 20 is the joiners under SPINE-RESPOND-BASIC; 21
      // completes the k- question family and empties SPINE-SAY-WHY; 22 is the repair
      // kit under SPINE-POLITE-REQUEST-REPAIR. Atom counts are 3 / 4 / 5 / 3, all well
      // inside the 12-atom chapter budget, and every headword was chosen from the
      // fifteen letters the script ladder has actually taught.
      // 22 -> 27: the oblique tranche. Chapter 23 teaches the direct/oblique split as
      // a RULE before any postposition that needs it -- the inventory called it "the
      // invisible prerequisite under four of the five points" in this column -- and
      // then spends three lessons on کو, میں and پر. 24 gives the demonstratives their
      // oblique shapes and closes the ک- / یہ- / وہ- place frame, emptying
      // SPINE-ASK-LOCATION's omission ledger; 25 is the plural and the oblique plural;
      // 26 is possession for the remaining persons plus اپنا, the tag and the vocative;
      // 27 is the track's first place names, emptying SPINE-NAME-EVERYDAY-THINGS's.
      // Atom counts are 5 / 4 / 4 / 4 / 4, all inside the 12-atom chapter budget, and
      // every headword is spellable with the fifteen letters the ladder has taught.
    
      // 27 -> 32: HL-C350, the numeral tranche. Urdu taught NO numeral at all in
      // 134 lessons -- ek and do both returned zero as words, the two raw matches
      // for do being inside dost -- so no age, no price and no telephone number
      // was reachable. Chapters 28-29 teach one to ten, 30 twenty and a hundred
      // with an explicit account of why the teens are NOT here, 31 the ten Urdu
      // digits, and 32 the ordinals.
    ).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
      // 32 -> 33: the reading rung. Three lessons -- six words, six lines and a
      // 37-word paragraph -- all built from the fifteen letters the script ladder
      // has actually taught, which is why the passage says nothing about tea,
      // shoes or goodbyes: چائے, جوتا and خدا حافظ each contain a letter this
      // track has not reached.
      // 33 -> 37: HL-C443, one lesson per letter the reader had read in words and
      // never written -- sixteen letters in chapters 34-37, each from its word,
      // which also reaches the خ and د of خدا حافظ.
      26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
      // 37 -> 87: the pre-A1 vocabulary tranche, fifty chapters of five words
      // (the body, family, thirty-seven verbs, food, home, school, nature,
      // animals, describing words, time, place, the town, clothes and the
      // numbers from eleven to ninety), closed by four reviews.
      ...Array.from({ length: 50 }, (_, i) => 38 + i),
      // 87 -> 142: the A1 tranche, fifty-four chapters of five words (this and
      // that, time, places, can and want, things, describing words and verbs),
      // with chapter 99 writing ز and ط from روز and طرف.
      // 142 -> 143: the first A2 writing rung, a model-free connected message.
      // 143 -> 146: the A2 spine chapters (the past, the future, reading
      // practical texts).
      // 146 -> 179: the first A2 vocabulary tranche, 165 headwords, twenty-five
      // of them verbs.
      // 179 -> 211: the second A2 vocabulary tranche, 160 headwords.
      // 211 -> 238: the third A2 vocabulary tranche, 135 headwords.
      // 238 -> 263: the last A2 vocabulary tranche, 125 headwords; Urdu attains
      // A2.
      ...Array.from({ length: 176 }, (_, i) => 88 + i)]);
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
      // 15 -> 22: the joining and repair tranche. 16 is coordination (и, или,
      // ни ... ни, тоже); 17 is the two buts and the closing punctuation marks;
      // 18 is что and the two frames it unlocks; 19 is почему/потому что and
      // когда, closing on the comma rule; 20 is the verb that takes another
      // verb; 21 is the repair kit the track had none of; 22 is the question
      // family and который. All seven are generated, all seven are Cyrillic,
      // and none of them needed a letter the track had not already taught.
      // 22 -> 26: HL-C350, the numeral tranche. Russian held exactly ONE numeral --
      // odin, and it arrived as half of the odin ... drugoy joining pattern rather
      // than as a number -- so the track could ask skolko and understand no answer.
      // Chapters 23-24 teach one to ten, 25 twenty and a hundred with the
      // centum/satem split the hundred names, and 26 the ordinals said and written.
      // 26 -> 27: the reading rung. Russian is the first track whose passage was
      // bounded by vocabulary rather than by the alphabet -- it had already
      // taught every letter it needed, which is what twenty-six chapters of
      // script work buys.
      // 27 -> 28: HL-C443, the small я. The word for I is written small inside a
      // sentence, and only the capital Я had a lesson of its own.
      // 28 -> 81: the pre-A1 vocabulary tranche, fifty-two chapters of five
      // words (260 headwords, twenty-three verbs), every one written in letters
      // the track already reads -- no э, щ or ъ yet -- and chapter 81, which
      // writes ё from ребёнок, the one letter those words read but no lesson wrote.
      // 81 -> 135: the A1 vocabulary tranche, chapters 82-92 and 94-135 (265
      // headwords, twenty-five verbs), and chapter 93, which writes э from этот
      // and щ from площадь -- the two letters these words bring -- before any
      // review prints them.
      // 135 -> 139: the A2 spine chapters -- saying no, the past, the future
      // and reading practical texts.
      // 139 -> 172: the first A2 vocabulary tranche, 165 headwords, thirty-five
      // of them verbs.
      // 172 -> 203: the second A2 vocabulary tranche, 155 headwords.
      // 203 -> 231: the third A2 vocabulary tranche, 140 headwords.
      // 231 -> 257: the last A2 vocabulary tranche, 130 headwords; Russian
      // attains A2.
    ).toEqual(Array.from({ length: 257 }, (_, index) => index + 1));
    expect(
      books.books.every((book) =>
        book.chapters.every((chapter) => chapter.tex.length > 100),
      ),
    ).toBe(true);
  });

  it("produces a machine-readable migration gap baseline", () => {
    const report = curriculumGapReport;
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
    const activities = lessons.flatMap((lesson) =>
      compileLessonActivities(lesson.blocks),
    );
    const ids = activities.map((activity) => activity.id);
    expect(activities.length).toBeGreaterThan(0);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.every((id) => id.length > 0 && id.trim() === id)).toBe(true);
  });

  it("keeps track-specific integration evidence independently owned", () => {
    const seen = new Set<string>();
    for (const [, module] of Object.entries(integrationTrackEvidenceModules).sort(
      ([left], [right]) => left.localeCompare(right),
    )) {
      expect(seen.has(module.integrationTrackEvidence.id)).toBe(false);
      seen.add(module.integrationTrackEvidence.id);
      module.integrationTrackEvidence.assert({
        taxonomy, registry, spine, curricula, books, lessons, scripts, soundTags, dataset,
        curriculumGapReport,
      });
    }
    expect(seen.size).toBeGreaterThan(0);
  });

  it("GREETING-HELLO joins every track (the normalization payoff)", () => {
    // Every track realizes 'hello', so the join size tracks the track count.
    const langs = languagesForConcept(dataset, "GREETING-HELLO").map(
      (r) => r.language,
    );
    expect(new Set(langs).size).toBe(dataset.languages.length);
  });

  it("the self-introduction concepts join many languages", () => {
    expect(
      languagesForConcept(dataset, "INTRO-MY-NAME-IS").length,
    ).toBeGreaterThanOrEqual(8);
    expect(
      languagesForConcept(dataset, "INTRO-WHATS-YOUR-NAME").length,
    ).toBeGreaterThanOrEqual(8);
  });

  it("every concept id is canonical or namespaced", () => {
    const NS = /^[A-Z]{2}-[A-Z0-9-]+$/;
    for (const c of dataset.concepts) {
      const ok = c.id in taxonomy.concepts || NS.test(c.id);
      expect(ok, `bad concept id: ${c.id}`).toBe(true);
    }
  });
});
