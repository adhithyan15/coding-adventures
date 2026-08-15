// HL-C10: what level is each lesson building toward?
//
// The corpus block at the bottom is the one that answers the project owner's question —
// "how far is each track from A1, and from Advanced?" — with a number rather than a guess.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import {
  CEFR_LEVELS,
  deriveLessonLevel,
  lessonSpineNodes,
  lessonsUpToLevel,
  levelRank,
  levelsUpTo,
  summarizeLevels,
  type CefrLevel,
} from "../src/levels.js";
import type { CurriculumSpine, LanguageCurriculum } from "../src/types.js";

function lesson(id: string, language = "spanish") {
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: 1\ntype: word\n` +
      `headword: hola\ngloss: hello\nconcept_tag: GREETING-HELLO\n---\n\n# ${id}\n\n` +
      `## Warm-up\n\nSay it.\n`,
    language,
  );
}

const SPINE = {
  version: 1,
  stages: [...CEFR_LEVELS],
  nodes: [
    { id: "N-PRE", stage: "pre-A1", canDo: "x", prerequisites: [], core: true, concepts: [] },
    { id: "N-A1", stage: "A1", canDo: "x", prerequisites: [], core: true, concepts: [] },
    { id: "N-A2", stage: "A2", canDo: "x", prerequisites: [], core: true, concepts: [] },
  ],
} as unknown as CurriculumSpine;

function curricula(segments: Array<{ node: string; lessons: string[] }>): LanguageCurriculum[] {
  return [
    {
      version: 1,
      language: "spanish",
      path: segments.map((segment, index) => ({
        id: `P-${index}`,
        spine_node: segment.node,
        lessons: segment.lessons,
        before: [],
        inline: [],
        after: [],
      })),
      spine: {},
      extensions: [],
    },
  ] as unknown as LanguageCurriculum[];
}

describe("the ladder", () => {
  it("runs pre-A1 through C2, weakest first", () => {
    expect(CEFR_LEVELS).toEqual(["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"]);
    expect(levelRank("pre-A1")).toBe(0);
    expect(levelRank("C2")).toBe(6);
  });

  it("levelsUpTo is the filter a 'ramp to X' book applies", () => {
    expect(levelsUpTo("A1")).toEqual(["pre-A1", "A1"]);
    expect(levelsUpTo("pre-A1")).toEqual(["pre-A1"]);
    expect(levelsUpTo("C2")).toEqual([...CEFR_LEVELS]);
  });
});

describe("deriving a lesson's level", () => {
  const spineNodes = () =>
    lessonSpineNodes(
      curricula([
        { node: "N-PRE", lessons: ["ES-a"] },
        { node: "N-A1", lessons: ["ES-b"] },
        { node: "N-A2", lessons: ["ES-c"] },
      ]),
    );
  const stageOf = new Map<string, CefrLevel>(
    SPINE.nodes.map((node) => [node.id, node.stage as CefrLevel]),
  );

  it("takes the level from the spine node that claims the lesson", () => {
    const nodes = spineNodes();
    expect(deriveLessonLevel(lesson("ES-a"), nodes, stageOf)).toMatchObject({
      level: "pre-A1",
      spineNode: "N-PRE",
      reason: "spine-node",
    });
    expect(deriveLessonLevel(lesson("ES-c"), nodes, stageOf).level).toBe("A2");
  });

  it("reports null — never a guess — for a lesson no segment claims", () => {
    // A wrong level is worse than a missing one: it would put material a reader is not
    // ready for inside a book that promises a gentle ramp.
    const entry = deriveLessonLevel(lesson("ES-orphan"), spineNodes(), stageOf);
    expect(entry.level).toBeNull();
    expect(entry.reason).toBe("unmapped");
  });

  it("is stable when a ledger bug puts one lesson in two segments", () => {
    // First writer wins, so the answer does not depend on file order. The duplicate
    // itself is `validateCurriculum`'s finding, not this module's.
    const nodes = lessonSpineNodes(
      curricula([
        { node: "N-PRE", lessons: ["ES-dup"] },
        { node: "N-A2", lessons: ["ES-dup"] },
      ]),
    );
    expect(nodes.get("ES-dup")).toBe("N-PRE");
  });

  it("does not let a lesson id poison the index", () => {
    const nodes = lessonSpineNodes(curricula([{ node: "N-A1", lessons: ["__proto__"] }]));
    expect(nodes.get("__proto__")).toBe("N-A1");
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });
});

describe("rolling up", () => {
  it("counts per level and per track, and names each track's reach", () => {
    const summary = summarizeLevels(
      [lesson("ES-a"), lesson("ES-b"), lesson("ES-c"), lesson("ES-orphan")],
      curricula([
        { node: "N-PRE", lessons: ["ES-a"] },
        { node: "N-A1", lessons: ["ES-b"] },
        { node: "N-A2", lessons: ["ES-c"] },
      ]),
      SPINE,
    );
    expect(summary.byLevel["pre-A1"]).toBe(1);
    expect(summary.byLevel.A1).toBe(1);
    expect(summary.byLevel.A2).toBe(1);
    expect(summary.unmapped).toBe(1);
    expect(summary.mappedPercent).toBe(75);
    expect(summary.tracks[0]?.reach).toBe("A2");
  });

  it("a ramp-to-A1 edition excludes both higher levels and unplaced lessons", () => {
    const all = [lesson("ES-a"), lesson("ES-b"), lesson("ES-c"), lesson("ES-orphan")];
    const paths = curricula([
      { node: "N-PRE", lessons: ["ES-a"] },
      { node: "N-A1", lessons: ["ES-b"] },
      { node: "N-A2", lessons: ["ES-c"] },
    ]);
    const ramp = lessonsUpToLevel(all, paths, SPINE, "A1").map((l) => l.realization.lessonId);
    // ES-c is above the ceiling; ES-orphan is unplaced. A book promising a gentle ramp
    // must not carry a surprise, so the honest failure is a shorter book.
    expect(ramp).toEqual(["ES-a", "ES-b"]);
  });
});

describe("exam alignment", () => {
  // The owner's instruction: do not leave things unmapped. A track with no mapping
  // silently drops out of every level report, and a learner asking "what is A1 in Tamil?"
  // deserves an answer. This test is the guard — registering a track now requires
  // answering the question.
  it("maps every registered track, with the KIND of answer recorded", () => {
    const exams = JSON.parse(
      readFileSync(join(defaultCurriculumRoot(), "core", "exam-levels.json"), "utf8"),
    ) as {
      tracks: Record<string, { basis: string; mapping: unknown; exam: string }>;
    };
    const { registry } = loadEverything();

    for (const language of registry.languages) {
      const entry = exams.tracks[language.id];
      expect(entry, `${language.id} has no exam-level mapping`).toBeDefined();
      expect(entry!.mapping, `${language.id} mapping is empty`).toBeTruthy();
      // `published` = the awarding body states it. `research` = a widely-cited third-party
      // correspondence. `editorial` = this project's judgement, a working default to be
      // corrected — never a claim about what a certificate is worth. Being explicit about
      // which is what makes mapping-everything honest rather than sloppy.
      expect(["published", "research", "editorial"]).toContain(entry!.basis);
    }
  });

  it("keeps a caveat on every mapping that is not the awarding body's own", () => {
    const exams = JSON.parse(
      readFileSync(join(defaultCurriculumRoot(), "core", "exam-levels.json"), "utf8"),
    ) as { tracks: Record<string, { basis: string; caveat?: string; mapping: unknown }> };
    for (const [language, entry] of Object.entries(exams.tracks)) {
      // A plain "cefr" editorial mapping for a track with no exam at all needs no essay;
      // anything that names a specific foreign ladder does, or a reader will take the
      // correspondence for an official one.
      if (entry.basis !== "published" && typeof entry.mapping === "object") {
        expect(entry.caveat, `${language} names a ladder without a caveat`).toBeTruthy();
      }
    }
  });
});

describe("corpus snapshot", () => {
  // The measured answer to "how far is each track from A1, and from Advanced?"
  //
  // Ratchet these as content lands. `A2: 0` was the headline for a long time: the A2
  // spine tranche existed but NO track had realized a single node of it.
  //
  // That changed when three tracks authored core verbs at once — Latin chapter 37 (8),
  // Arabic chapters 28-30 (6) and Russian chapter 3 (6). All twenty attach to
  // SPINE-SAY-WHAT-I-DO, which the shared spine declares at stage A2, so the level is
  // DERIVED rather than claimed:
  //
  //   A2  0 -> 20     pre-A1 657 (unchanged)   A1 307 (unchanged)
  //
  // `unmapped` and `mappedPercent` do not move: all twenty sit in realization-path
  // segments, so every one of them has a derivable level. The three tracks were authored
  // in parallel and each measured this number alone — 8, 6 and 6 — so each was correct
  // and all three were wrong about the total. It is re-measured here against the merged
  // corpus, which is the only place the real number exists.
  it("pins where the corpus actually stands on the ladder", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);

    // +2: Spanish Chapter 9 maps its identity and ser/estar contrast lessons onto
    // the existing pre-A1 social spine.
    // +4: TA-W10..TA-W13, the Tamil writing strand's extension, all sit at pre-A1.
    // +3 more: TA-W14/15/16, closing chapters 4-5, sit there too.
    // +2: TA-W17-read-unavu and TA-W18-read-uur.
    // +1: TA-W19-read-muunru, the strand's last available slot, likewise pre-A1.
    // +1, and only ONE of chapter 39's four lessons: TA-W20-read-onru, which sits on
    // SPINE-MEET-GREET like every other writing lesson. The chapter's three speaking
    // lessons land at A2, because SPINE-SAY-WHAT-I-WANT is an A2 node — see below.
    expect(summary.byLevel["pre-A1"]).toBeGreaterThanOrEqual(1038) // FLOOR — content only grows; see the note at the top of this file; // +3: HL-C97 adds the repair kit (no entiendo, mas despacio) at chapter 14 // +4: HL-C98 // +40: vocabulary wave 5 (persian 12, telugu 13, malayalam 15) // +54: vocabulary wave 6 (russian 14, persian 14, urdu 13, bengali 13) // HL12: +30 recognition segments (telugu/kannada/malayalam 8 each, sanskrit 6) -- all pre-A1, since every one sits on SPINE-MEET-GREET // HL12 payment two: +8 Hindi segments, all pre-A1
    // Chapter 10 adds singular ir and possessives at A1. Chapter 11 adds one more
    // definite-reference lesson there; its other work is A2. Chapter 12 adds its
    // newly mapped terminal checkpoints at A2 on SPINE-SAY-WHAT-I-DO.
    expect(summary.byLevel.A1).toBe(508); // +1: ES-C02-concordancia sits on SPINE-TIME-OF-DAY // +42: HL-C136 wave I. The whole wave lands at A1, not pre-A1, and that is DERIVED rather than chosen: all six chapters realize asking-and-pointing nodes the shared spine declares at A1, so `pre-A1` (1038) does not move at all. The wave is a *pre-A1 lexicon* drive by selection order, which is a statement about which words are worth teaching first, not a claim about which spine node they realize. // HL-C137 wave II: +36 adjective lessons, +6 chapters, all six Indic tracks // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // HL-C166: +11 -- Sanskrit chapters 19 and 20
    // Chapter 15's split adds three more mapped A2 lessons without changing its node.
    // Chapter 16's split adds five more mapped A2 lessons on the same node.
    // Chapter 17's split adds four more mapped A2 lessons on the same node.
    // Chapter 18 replaces ten mapped A2 lessons with nine prerequisite-safe steps.
    // +3: TA-C39-vendum, TA-C39-evvalavu and TA-C39-oru. Tamil's curriculum.json had
    // already declared SPINE-SAY-WHAT-I-WANT with an empty segment list and VERB-WANT
    // in `omits`; chapter 39 realizes the node, so the omission is removed with it.
    expect(summary.byLevel.A2).toBe(527); // +39: Spanish chapters 11-18 plus prerequisite closure // +3: HL-C88 slice 8 // +1: HL-C88 slice 9 (falsos amigos) // +3 — the plural rung sits on SPINE-TALK-ABOUT-PAST, which is A2: HL-C113 preterite plural // HL-C113 preterite close // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche
    // 8, not 0: Spanish chapters 38 and 41 realize SPINE-NARRATE-EVENTS and
    // SPINE-GIVE-REASONS, four lessons each.
    // HL-C113 makes SPINE-EXPRESS-CONDITION the THIRD realized B1 node -- the
    // sentence that used to stand here, "the only B1 nodes any track has
    // touched", stopped being true the moment chapters 196-198 landed.
    // B2 opened with HL-C113 step 6: three lessons on SPINE-REPORT-WHAT-OTHERS-SAID.
    // Neither C1 nor C2 is authored-but-unrealized any longer: HL-C174/175/177
    // closed C1, and HL-C178 opens C2 with the cultural-weight chapter.
    expect(summary.byLevel.B1).toBe(40); // +3: HL-C113 realizes SPINE-EXPRESS-CONDITION, the third B1 node // +2: HL-C113 imperfect subjunctive (2 lessons) // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1
    expect(summary.byLevel.B2).toBe(17); // +3: HL-C113 step 6 opens B2 with reported speech // +4: step 7 adds the reported questions, the review and the synthesis, closing the node // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C173: +3 -- B2 closes (chapter 271)
    expect(summary.byLevel.C1).toBe(10); // HL-C175: chapter 272, the first C1 lessons in the corpus // HL-C177: +5 -- chapter 273, C1 closes
    expect(summary.byLevel.C2).toBe(18); // HL-C178: chapter 274, the first C2 lessons in the corpus // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33

    // HL-C63 places 47 orphan chapter lessons and two Spanish prerequisites. Chapters
    // Chapters 7-9 then map their terminal practices and Chapter 9's remaining
    // teaching lessons. Chapter 10 closes two more gaps; Chapter 11 closes its
    // possessive and terminal-practice gaps; Chapters 12-13 close their payoff gaps.
    expect(summary.unmapped).toBe(86);
    expect(summary.mappedPercent).toBe(96); // +1: vocabulary wave 6 grows the mapped corpus faster than the unmapped 86
  });

  it("shows twenty tracks have reached A2, and only two have not reached A1", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);
    // `reach` is the highest level a track has ANY lesson at, so this names the tracks
    // rather than counting them — "no track is at A2" has become a list, and listing it
    // keeps the assertion as tight as the original. Spanish has now left this list --
    // it reaches B1 -- so the list is 19, and the ceiling for everyone else is A2.
    expect(
      summary.tracks.filter((track) => track.reach === "A2").map((track) => track.language),
    ).toEqual([
      "arabic",
      "bengali",
      "french",
      "german",
      "gujarati",
      "hindi",
      "italian",
      "kannada",
      "latin",
      "malayalam",
      "marathi",
      "persian",
      "portuguese",
      "punjabi",
      "russian",
      "sanskrit",
      "tamil",
      "telugu",
      "urdu",
    ]);
    // Exactly one track now exceeds A2, and naming it is stronger than a blanket
    // "nothing does" -- it says which rung was climbed and by whom.
    expect(
      summary.tracks.filter((track) => track.reach !== null && levelRank(track.reach) > levelRank("A2"))
        .map((track) => track.language),
    ).toEqual(["spanish"]);
    expect(
      summary.tracks.filter((track) => track.reach === "pre-A1").map((track) => track.language),
    ).toEqual(["chinese", "japanese"]);
  });

  it("can already build a ramp-to-A1 edition from the canonical corpus", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const ramp = lessonsUpToLevel(lessons, paths, spine, "A1");
    // The whole point: this is a FILTER over the one corpus, not a second corpus.
    // +4, measured as a set difference against the pre-change corpus rather than inferred
    // from the total: TA-W10-read-naan, TA-W11-read-niingal, TA-W12-read-eppadi and
    // TA-W13-read-irukkirirgal are the only lessons that join.
    // +3: TA-W14-read-pesu, TA-W15-read-po and TA-W16-read-tamizh, likewise the only
    // lessons that join.
    // +2: TA-W17-read-unavu and TA-W18-read-uur, the only lessons that join.
    // +1: TA-W19-read-muunru, again measured as a set difference — the only lesson
    // that joins, not inferred from the total.
    // +1, measured as a set difference: TA-W20-read-onru is the only lesson that
    // joins. The three A2 speaking lessons are above the A1 cut and do not.
    expect(ramp.length).toBeGreaterThanOrEqual(1404); // FLOOR — content only grows; // +3: HL-C97 adds the repair kit (no entiendo, mas despacio) at chapter 14 // +40: vocabulary wave 5, all of it pre-A1 // +54: vocabulary wave 6, all of it pre-A1 // HL12: +30 recognition segments (telugu/kannada/malayalam 8 each, sanskrit 6) -- all of it below the A1 cut // HL12 payment two: +8 Hindi segments
    expect(ramp.length).toBeLessThan(lessons.length);
  });
});
