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
  generatedLevelSnapshotOutputsFromSummary,
  summarizeLevelTracks,
} from "../src/level-snapshot-cli.js";
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
  it("pins the exact corpus through independently mergeable language shards", () => {
    const { lessons, curricula: paths, spine } = loadEverything();
    const summary = summarizeLevels(lessons, paths, spine);
    const snapshots = generatedLevelSnapshotOutputsFromSummary(summary);
    const snappedTracks = [...snapshots.entries()].map(([relative, expected]) => {
      expect(readFileSync(join(defaultCurriculumRoot(), relative), "utf8"), relative).toBe(expected);
      return JSON.parse(expected);
    });

    expect(summarizeLevelTracks(snappedTracks)).toEqual({
      totalLessons: summary.totalLessons,
      byLevel: summary.byLevel,
      unmapped: summary.unmapped,
      mappedPercent: summary.mappedPercent,
    });
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
      // HL-C230: chinese leaves this list. Its numerals chapter realizes
      // SPINE-COUNT-ONE-TO-FIVE, an A1 node, so its `reach` is now A1 -- while the
      // track still holds twenty-five lessons and can say hello and count to five. The
      // metric is honest about what it measures and misleading about what it implies,
      // which is worth writing down here rather than celebrating in a changelog.
    ).toEqual(["japanese", "marwadi"]);
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
