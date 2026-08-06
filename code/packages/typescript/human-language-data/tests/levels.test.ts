// HL-C10: what level is each lesson building toward?
//
// The corpus block at the bottom is the one that answers the project owner's question —
// "how far is each track from A1, and from Advanced?" — with a number rather than a guess.

import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";
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

describe("corpus snapshot", () => {
  // The measured answer to "how far is each track from A1, and from Advanced?"
  //
  // Ratchet these as content lands. `A2: 0` is the headline: the A2 spine tranche exists
  // but NO track has realized a single node of it, so the whole corpus sits at or below
  // A1 and "Advanced" does not exist anywhere yet.
  it("pins where the corpus actually stands on the ladder", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);

    expect(summary.byLevel["pre-A1"]).toBe(657);
    expect(summary.byLevel.A1).toBe(307);
    expect(summary.byLevel.A2).toBe(0);
    expect(summary.byLevel.B1).toBe(0);
    expect(summary.byLevel.B2).toBe(0);
    expect(summary.byLevel.C1).toBe(0);
    expect(summary.byLevel.C2).toBe(0);

    // 170 lessons sit in no realization-path segment, all of them schema-v1. They are the
    // reason `mappedPercent` is not 100, and mapping them is migration work, not a gate.
    expect(summary.unmapped).toBe(170);
    expect(summary.mappedPercent).toBe(85);
  });

  it("shows no track has reached A2, and five have not reached A1", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);
    expect(summary.tracks.every((track) => track.reach !== "A2")).toBe(true);
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
