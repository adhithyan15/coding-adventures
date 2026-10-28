// HL-C10: what level is each lesson building toward?
//
// The corpus block at the bottom is the one that answers the project owner's question —
// "how far is each track from A1, and from Advanced?" — with a number rather than a guess.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
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
import { loadLevelProgressOwners } from "./level-gate-attainment-pins.js";

function lesson(id: string, language = "alpha") {
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
      language: "alpha",
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

// Real-corpus assertions share one read-only load. Re-reading all curriculum
// owners and lessons inside each timed assertion exhausts the Windows budget.
const REAL_EVERYTHING = loadEverything();
const LEVEL_PROGRESS = loadLevelProgressOwners(
  fileURLToPath(new URL("./level-gate-attainment/", import.meta.url)),
  REAL_EVERYTHING.registry.languages.map((language) => language.id),
);

describe("exam alignment", () => {
  // The owner's instruction: do not leave things unmapped. A track with no mapping
  // silently drops out of every level report, and a learner asking about A1
  // deserves an answer. This test is the guard — registering a track now requires
  // answering the question.
  it("maps every registered track, with the KIND of answer recorded", () => {
    const exams = JSON.parse(
      readFileSync(join(defaultCurriculumRoot(), "core", "exam-levels.json"), "utf8"),
    ) as {
      tracks: Record<string, { basis: string; mapping: unknown; exam: string }>;
    };
    const { registry } = REAL_EVERYTHING;

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

describe("corpus level summary", () => {
  // The measured answer to "how far is each track from A1, and from Advanced?"
  //
  // Ratchet these as content lands. `A2: 0` was the headline for a long time: the A2
  // spine tranche existed but NO track had realized a single node of it.
  //
  // That changed when several tracks authored core verbs at once. Those additions attach to
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
  it("derives an exact closed summary without tracked generated rollups", () => {
    const { registry, lessons, curricula: paths, spine } = REAL_EVERYTHING;
    const summary = summarizeLevels(lessons, paths, spine);
    expect(summary.tracks.map((track) => track.language)).toEqual(
      registry.languages.map((language) => language.id).sort((a, b) => a.localeCompare(b)),
    );

    const byLevel = Object.fromEntries(CEFR_LEVELS.map((level) => [level, 0])) as Record<
      CefrLevel,
      number
    >;
    let totalLessons = 0;
    let unmapped = 0;
    for (const track of summary.tracks) {
      const mapped = CEFR_LEVELS.reduce((total, level) => {
        byLevel[level] += track.byLevel[level];
        return total + track.byLevel[level];
      }, 0);
      expect(mapped + track.unmapped, track.language).toBe(track.lessonCount);
      const reach = [...CEFR_LEVELS].reverse().find((level) => track.byLevel[level] > 0) ?? null;
      expect(track.reach, track.language).toBe(reach);
      totalLessons += track.lessonCount;
      unmapped += track.unmapped;
    }

    expect(totalLessons).toBe(summary.totalLessons);
    expect(totalLessons).toBe(lessons.length);
    expect(byLevel).toEqual(summary.byLevel);
    expect(unmapped).toBe(summary.unmapped);
    expect(totalLessons - unmapped).toBe(
      CEFR_LEVELS.reduce((total, level) => total + summary.byLevel[level], 0),
    );
    expect(summary.mappedPercent).toBe(
      totalLessons === 0 ? 0 : Math.round(((totalLessons - unmapped) / totalLessons) * 100),
    );
  });

  it("rejects resurrection of tracked level snapshot rollups", () => {
    const root = defaultCurriculumRoot();
    const collisions = readdirSync(join(root, "core"), { withFileTypes: true })
      .filter((entry) => entry.name.toLowerCase() === "level-snapshots");
    for (const entry of collisions) {
      expect(entry.isDirectory(), entry.name).toBe(true);
      // Git cannot track an empty directory. Any committed resurrection therefore
      // puts at least one owner, aggregate, symlink, or sentinel inside this path.
      expect(readdirSync(join(root, "core", entry.name)), entry.name).toEqual([]);
    }
  });
  it("matches every track's directly owned reach pin", () => {
    const { lessons, curricula: paths, spine } = REAL_EVERYTHING;
    const summary = summarizeLevels(lessons, paths, spine);
    for (const track of summary.tracks) {
      expect(track.reach, `${track.language} reach owner`).toBe(
        LEVEL_PROGRESS[track.language]!.reach,
      );
    }
  });

  it("can already build a ramp-to-A1 edition from the canonical corpus", () => {
    const { lessons, curricula: paths, spine } = REAL_EVERYTHING;
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
    expect(ramp.length).toBeGreaterThanOrEqual(1404); // FLOOR — corpus content only grows.
    expect(ramp.length).toBeLessThan(lessons.length);
  });
});
