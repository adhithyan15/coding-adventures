import { describe, expect, it } from "vitest";
import {
  buildCompletionPlan,
  CERTIFIABLE_LEVELS,
  renderCompletionPlan,
  type CompletionPlanInput,
} from "../src/completion-plan.js";
import { LEVEL_VOCABULARY, type LevelGateReport } from "../src/level-gate.js";
import type { ScriptClosureReport, TrackClosure } from "../src/script-closure.js";
import type { CefrLevel } from "../src/levels.js";

// A two-track fixture is enough to pin every ordering rule, and it keeps these
// tests off the corpus — which is the point of `buildCompletionPlan` being pure
// over report data rather than reading the filesystem itself.
function gate(
  tracks: {
    language: string;
    inProgressAt: CefrLevel | null;
    vocabulary?: number;
    blockers?: LevelGateReport["tracks"][number]["blockers"];
  }[],
): LevelGateReport {
  return {
    vocabularyTargets: LEVEL_VOCABULARY,
    tracks: tracks.map((track) => ({
      language: track.language,
      touches: null,
      attained: null,
      inProgressAt: track.inProgressAt,
      blockers: track.blockers ?? [],
      vocabulary: track.vocabulary ?? 0,
    })),
    summary: {
      tracksOverstating: 0,
      tracksWithAnyLevel: 0,
      attainedByLevel: { "pre-A1": 0, A1: 0, A2: 0, B1: 0, B2: 0, C1: 0, C2: 0 },
    },
  };
}

function closure(tracks: Partial<TrackClosure>[]): ScriptClosureReport {
  return {
    tracks: tracks.map((track) => ({
      language: track.language ?? "x",
      script: track.script ?? "devanagari",
      lessonCount: 0,
      scriptLessons: 0,
      taughtGlyphs: 0,
      shownGlyphs: 0,
      neverTaughtGlyphs: track.neverTaughtGlyphs ?? 0,
      violations: track.violations ?? 0,
      exposureOnly: 0,
      exposureExemptedGlyphs: 0,
      headwordsWithoutRomanization: 0,
    })),
    violations: [],
    unknownScriptTracks: [],
    summary: {
      tracksWithScript: tracks.length,
      tracksTeachingNothing: 0,
      violations: 0,
      exposureOnly: 0,
      exposureExemptedGlyphs: 0,
      headwordsWithoutRomanization: 0,
      tracksWithUnknownScript: 0,
    },
  };
}

const vocabularyBlocker = (short: number) =>
  [{ criterion: "vocabulary" as const, detail: `teaches ${300 - short} against 300`, shortfall: short }];

function plan(input: Partial<CompletionPlanInput> & Pick<CompletionPlanInput, "levelGate">) {
  return buildCompletionPlan({
    scriptClosure: closure([]),
    inventories: [],
    ...input,
  });
}

describe("completion plan", () => {
  it("moves every track once before any track moves twice", () => {
    // Three tracks, each with two items. A flat sort by family would emit all
    // three script items and only then the vocabulary ones; the rotation must
    // emit one item per language first. This is the regression the round-robin
    // exists for.
    const built = plan({
      levelGate: gate([
        { language: "alpha", inProgressAt: "pre-A1", blockers: vocabularyBlocker(100) },
        { language: "beta", inProgressAt: "pre-A1", blockers: vocabularyBlocker(200) },
        { language: "gamma", inProgressAt: "pre-A1", blockers: vocabularyBlocker(150) },
      ]),
      scriptClosure: closure([
        { language: "alpha", neverTaughtGlyphs: 10 },
        { language: "beta", neverTaughtGlyphs: 10 },
        { language: "gamma", neverTaughtGlyphs: 10 },
      ]),
    });
    expect(built.head.slice(0, 3).map((item) => item.language)).toEqual(["beta", "gamma", "alpha"]);
    expect(built.head.slice(0, 3).every((item) => item.kind === "script-closure")).toBe(true);
    expect(built.head.slice(3, 6).map((item) => item.language)).toEqual(["beta", "gamma", "alpha"]);
  });

  it("orders tracks furthest-behind first", () => {
    const built = plan({
      levelGate: gate([
        { language: "ahead", inProgressAt: "pre-A1", blockers: vocabularyBlocker(20) },
        { language: "behind", inProgressAt: "pre-A1", blockers: vocabularyBlocker(280) },
      ]),
    });
    expect(built.head.slice(0, 2).map((item) => item.language)).toEqual(["behind", "ahead"]);
  });

  it("puts a lower rung ahead of a higher one, in any track", () => {
    const built = plan({
      levelGate: gate([
        { language: "climbing", inProgressAt: "A2", blockers: vocabularyBlocker(900) },
        { language: "floor", inProgressAt: "pre-A1", blockers: vocabularyBlocker(10) },
      ]),
    });
    expect(built.head[0]?.language).toBe("floor");
  });

  it("demotes an inventory for a rung the track has not reached", () => {
    // pre-A1 is not certifiable, so this track's inventory item is for A1 —
    // lookahead. It must sort behind the track's own floor work.
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "pre-A1", blockers: vocabularyBlocker(250) }]),
    });
    expect(built.head.map((item) => item.kind)).toEqual(["vocabulary", "exam-inventory"]);
  });

  it("promotes the inventory to first once the track stands on that rung", () => {
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "A1", blockers: vocabularyBlocker(250) }]),
    });
    expect(built.head[0]?.kind).toBe("exam-inventory");
    expect(built.head[0]?.id).toBe("exam-inventory/alpha/A1");
  });

  it("rolls past an inventory that already exists to the next unwritten one", () => {
    // Spanish is the live case: it HAS an A1 inventory, so asking it for A1
    // again would be busywork. The queue must name the next rung with no target
    // written down, which is what keeps the climb always aimed at something
    // external.
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "A1" }]),
      inventories: [{ language: "alpha", level: "A1" }],
    });
    expect(built.head.map((item) => item.id)).toEqual(["exam-inventory/alpha/A2"]);
  });

  it("asks for no inventory once every level up to the ceiling is written", () => {
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "A1" }]),
      inventories: CERTIFIABLE_LEVELS.map((level) => ({ language: "alpha", level })),
    });
    expect(built.head.filter((item) => item.kind === "exam-inventory")).toHaveLength(0);
  });

  it("counts a finished track as done and queues nothing for it", () => {
    const built = plan({ levelGate: gate([{ language: "alpha", inProgressAt: null }]) });
    expect(built.summary.tracksDone).toBe(1);
    expect(built.head).toHaveLength(0);
  });

  it("measures script work in glyphs to teach, not in lessons that break", () => {
    // Deleting the offending lessons would drop `violations` to zero without
    // teaching anybody a letter. The outstanding count must be the glyphs.
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "pre-A1" }]),
      scriptClosure: closure([{ language: "alpha", neverTaughtGlyphs: 24, violations: 300 }]),
    });
    const item = built.head.find((entry) => entry.kind === "script-closure");
    expect(item?.outstanding).toBe(24);
    expect(item?.tranches).toBe(3);
  });

  it("respects the ceiling when projecting and when queueing", () => {
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "B2" }]),
      ceiling: "A2",
    });
    expect(built.summary.tracksDone).toBe(1);
    expect(built.ceiling).toBe("A2");
  });

  it("reports a non-projectable family as null rather than as zero", () => {
    // "Not projectable" and "nothing left to do" are opposite facts. A zero here
    // would read as the second.
    const built = plan({ levelGate: gate([{ language: "alpha", inProgressAt: "pre-A1" }]) });
    const reinforcement = built.projection.find((entry) => entry.kind === "reinforcement");
    expect(reinforcement?.items).toBeNull();
  });

  it("projects vocabulary against the ceiling's cumulative target", () => {
    const built = plan({
      levelGate: gate([{ language: "alpha", inProgressAt: "pre-A1", vocabulary: 100 }]),
      ceiling: "A1",
    });
    const vocabulary = built.projection.find((entry) => entry.kind === "vocabulary");
    // 600 at A1, holding 100, at 35 a tranche.
    expect(vocabulary?.items).toBe(Math.ceil(500 / 35));
  });

  it("excludes pre-A1 from the certifiable levels", () => {
    expect(CERTIFIABLE_LEVELS).not.toContain("pre-A1");
    expect(CERTIFIABLE_LEVELS).toHaveLength(6);
  });

  it("renders a head and a projection", () => {
    const text = renderCompletionPlan(
      plan({ levelGate: gate([{ language: "alpha", inProgressAt: "pre-A1", blockers: vocabularyBlocker(250) }]) }),
    ).join("\n");
    expect(text).toContain("Completion plan (HL15)");
    expect(text).toContain("alpha");
    expect(text).toContain("Projection to the ceiling");
  });
});
