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
  // That changed with Latin chapter 37, the eight core verbs (sum, habeō, eō, veniō,
  // dīcō, videō, sciō, dō). They attach to SPINE-SAY-WHAT-I-DO, which the shared spine
  // declares at stage A2, so the level is DERIVED rather than claimed — the eight are the
  // corpus's first A2 lessons anywhere:
  //
  //   A2  0 -> 8      pre-A1 657 (unchanged)   A1 307 (unchanged)
  //
  // `unmapped` and `mappedPercent` do not move: the eight are in a realization-path
  // segment (LA-PATH-025), so every one of them has a derivable level.
  it("pins where the corpus actually stands on the ladder", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);

    expect(summary.byLevel["pre-A1"]).toBe(657);
    expect(summary.byLevel.A1).toBe(307);
    expect(summary.byLevel.A2).toBe(8);
    expect(summary.byLevel.B1).toBe(0);
    expect(summary.byLevel.B2).toBe(0);
    expect(summary.byLevel.C1).toBe(0);
    expect(summary.byLevel.C2).toBe(0);

    // 170 lessons sit in no realization-path segment, all of them schema-v1. They are the
    // reason `mappedPercent` is not 100, and mapping them is migration work, not a gate.
    expect(summary.unmapped).toBe(170);
    expect(summary.mappedPercent).toBe(85);
  });

  it("shows exactly one track has reached A2, and five have not reached A1", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);
    // `reach` is the highest level a track has ANY lesson at, so this names the tracks
    // rather than counting them: "no track is at A2" has become "latin and only latin
    // is", and listing it keeps the assertion as tight as the original. Nothing has
    // reached B1 or beyond, which is still the honest ceiling for the whole corpus.
    expect(
      summary.tracks.filter((track) => track.reach === "A2").map((track) => track.language),
    ).toEqual(["latin"]);
    expect(
      summary.tracks.every((track) => track.reach === null || levelRank(track.reach) <= levelRank("A2")),
    ).toBe(true);
    expect(
      summary.tracks.filter((track) => track.reach === "pre-A1").map((track) => track.language),
    ).toEqual(["chinese", "japanese", "persian", "russian", "urdu"]);
  });

  it("can already build a ramp-to-A1 edition from the canonical corpus", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const ramp = lessonsUpToLevel(lessons, paths, spine, "A1");
    // The whole point: this is a FILTER over the one corpus, not a second corpus.
    expect(ramp).toHaveLength(964);
    expect(ramp.length).toBeLessThan(lessons.length);
  });
});
