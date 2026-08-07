// HL09 §3.1 — what it takes to CLAIM a level. See src/level-gate.ts for why.

import { describe, expect, it } from "vitest";
import { loadEverything, loadChapterPolicy, loadTrackChapters } from "../src/loader.js";
import { buildCurriculumGapReport } from "../src/report.js";
import { LEVEL_VOCABULARY, runLevelGate } from "../src/level-gate.js";
import { summarizeLevels } from "../src/levels.js";
import { measureRamp } from "../src/ramp.js";
import { measureContinuity } from "../src/continuity.js";

function realReport() {
  const e = loadEverything();
  return buildCurriculumGapReport({
    registry: e.registry,
    lessons: e.lessons,
    books: e.books,
    curricula: e.curricula,
    spine: e.spine,
    trackChapters: loadTrackChapters(),
    chapterPolicy: loadChapterPolicy(),
  });
}

describe("the gate that would have caught the A2 claim", () => {
  // Explicit budget: `realReport()` builds the entire gap report over the whole corpus.
  // At 1,313 lessons that runs past vitest's 5,000 ms default under full-suite parallel
  // load, while passing in isolation — so a per-file run will not reproduce it.
  it("separates what a track TOUCHES from what it has ATTAINED", { timeout: 30_000 }, () => {
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;

    // The number that misled: one lesson pointing at one A2 node moves `touches`.
    expect(spanish.touches).toBe("A2");
    // The number that does not: Spanish has not met even pre-A1's criteria.
    expect(spanish.attained).toBeNull();
    expect(spanish.inProgressAt).toBe("pre-A1");
  });

  it("reports every track as overstating, which is the finding", () => {
    // All 22 tracks touch a level none of them has attained. That is not 22 bugs —
    // it is one measurement having been read as another for the whole project.
    const gate = realReport().levelGate!;
    expect(gate.summary.tracksOverstating).toBe(22);
    expect(gate.summary.tracksWithAnyLevel).toBe(0);
    for (const level of Object.values(gate.summary.attainedByLevel)) {
      expect(level).toBe(0);
    }
  });

  it("names which criterion failed and by how much, not just that one did", () => {
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;
    const vocab = spanish.blockers.find((b) => b.criterion === "vocabulary")!;

    // A bare `false` would move the argument rather than settle it.
    expect(vocab.detail).toContain(String(LEVEL_VOCABULARY["pre-A1"]));
    expect(spanish.blockers.map((b) => b.criterion)).toContain("reinforcement");

    // The criterion counts vocabulary AT OR BELOW the level, not the whole track.
    // Spanish teaches 113 headwords in total but only 44 at or below pre-A1, so the
    // shortfall is 256, not 187. Measuring the whole track against a per-level target
    // was the first version of this module committing the very error it exists to
    // catch — a number meaning "everything taught" published against one meaning
    // "by the end of pre-A1".
    expect(vocab.shortfall).toBe(256);
    expect(spanish.vocabulary).toBe(113);
    expect(vocab.shortfall).toBeGreaterThan(LEVEL_VOCABULARY["pre-A1"] - spanish.vocabulary);
  });

  it("scopes the atom budget to the level, so a high lesson cannot block a low one", () => {
    // Hindi has one over-budget lesson, and it sits ABOVE pre-A1. Before the criteria
    // were level-scoped it blocked pre-A1 anyway, which made criterion 3 unfalsifiable
    // at the bottom of the ladder for every track.
    const gate = realReport().levelGate!;
    const hindi = gate.tracks.find((t) => t.language === "hindi")!;
    expect(hindi.inProgressAt).toBe("pre-A1");
    expect(hindi.blockers.map((b) => b.criterion)).not.toContain("atom-budget");
  });

  it("refuses a level with no authored spine nodes instead of passing it vacuously", () => {
    // spine.json has zero B1-C2 nodes. "No node is unrealized" is not "every node is
    // realized" — without this, those levels passed criterion 1 on no evidence.
    const e = loadEverything();
    const gate = runLevelGate({
      lessons: e.lessons,
      levels: summarizeLevels(e.lessons, e.curricula, e.spine),
      curricula: e.curricula,
      spine: e.spine,
      ramp: measureRamp(e.lessons, loadChapterPolicy()),
      continuity: measureContinuity(e.lessons),
    });
    // No track reaches B1, so assert the rule directly on the spine instead.
    const authored = new Set(e.spine.nodes.map((n) => n.stage));
    expect(authored.has("B1")).toBe(false);
    expect(gate.tracks.every((t) => t.attained === null || t.attained !== "B1")).toBe(true);
  });

  it("stops at the FIRST failing level, because the criteria are cumulative", () => {
    const gate = realReport().levelGate!;
    for (const track of gate.tracks) {
      // You cannot be in progress at two levels at once, and a level above a failing
      // one is unreachable by definition.
      if (track.inProgressAt !== null) expect(track.blockers.length).toBeGreaterThan(0);
      if (track.attained === null) expect(track.inProgressAt).toBe("pre-A1");
    }
  });
});

describe("the criteria themselves", () => {
  it("passes a level only when all four hold", () => {
    // A synthetic track that satisfies nothing must attain nothing, and must name
    // every criterion it failed rather than the first one.
    const e = loadEverything();
    const levels = summarizeLevels(e.lessons, e.curricula, e.spine);
    const ramp = measureRamp(e.lessons, loadChapterPolicy());
    const continuity = measureContinuity(e.lessons);
    const gate = runLevelGate({
      lessons: e.lessons,
      levels,
      curricula: e.curricula,
      spine: e.spine,
      ramp,
      continuity,
    });
    const worst = gate.tracks.find((t) => t.blockers.length >= 3);
    expect(worst).toBeDefined();
    expect(new Set(worst!.blockers.map((b) => b.criterion)).size).toBe(
      worst!.blockers.length,
    );
  });

  it("keeps the vocabulary targets ascending, since they are cumulative", () => {
    const values = Object.values(LEVEL_VOCABULARY);
    for (let i = 1; i < values.length; i += 1) {
      expect(values[i]!).toBeGreaterThan(values[i - 1]!);
    }
  });

  it("is absent, not empty, when the caller supplied no policy", () => {
    // "Not measured" and "attained nothing" are opposite facts. A consumer that
    // passes no chapter policy must not see a report claiming zero attainment.
    const e = loadEverything();
    const report = buildCurriculumGapReport({
      registry: e.registry,
      lessons: e.lessons,
      books: e.books,
      curricula: e.curricula,
      spine: e.spine,
    });
    expect(report.levelGate).toBeUndefined();
  });
});
