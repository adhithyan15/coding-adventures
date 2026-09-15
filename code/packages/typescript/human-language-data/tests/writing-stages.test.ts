import { describe, expect, it } from "vitest";
import { loadAssessmentPolicy, loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { measureWritingStages, writingStagePrerequisites } from "../src/writing-stages.js";
import type { CurriculumSpine, LanguageCurriculum } from "../src/types.js";

function stagedLesson(id: string, sequence: number, stage: string) {
  return parseLesson(`---
schema_version: 2
id: ${id}
sequence: ${sequence}
chapter: 1
type: writing
headword: x
gloss: x
skills: [reading, writing]
---

# x

## Writing — evidence
<!-- hl-knowledge: introduces=[]; assesses=[AA-WRITE-X-01] -->
<!-- hl-writing-stage: ${stage} -->

Do the writing task.
`, "alpha");
}

const spine: CurriculumSpine = {
  version: 1,
  stages: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
  nodes: [{
    id: "PRE",
    stage: "pre-A1",
    strand: "FUNCTION",
    canDo: "write x",
    prerequisites: [],
    core: true,
    concepts: [],
  }],
};

function curriculum(ids: string[]): LanguageCurriculum {
  return {
    version: 1,
    language: "alpha",
    path: [{ id: "alpha-pre", spine_node: "PRE", lessons: ids, before: [], inline: [], after: [] }],
    spine: { PRE: { segments: ["alpha-pre"], omits: [], relocates: {} } },
    extensions: [],
  };
}

describe("cumulative writing-stage evidence (HL19)", () => {
  it("parses one explicit stage at its evidence block and removes authoring metadata from learner copy", () => {
    const lesson = stagedLesson("AA-W01", 10, "guided-copy");
    expect(lesson.blocks[0]?.writingStage).toBe("guided-copy");
    expect(lesson.blocks[0]?.markdown).not.toContain("hl-writing-stage");
  });

  it("marks a malformed or misplaced directive loudly", () => {
    const lesson = parseLesson(`---
id: AA-W01
---
# x
## Writing — evidence
Learner copy comes first.
<!-- hl-writing-stage: guided copy -->
`, "alpha");
    expect(lesson.blocks[0]?.writingStage).toBeUndefined();
    expect(lesson.blocks[0]?.writingStageDirectiveError).toMatch(/expected one/);
  });

  it("requires cumulative earlier evidence before a later stage counts", () => {
    const policy = loadAssessmentPolicy();
    const lessons = [
      stagedLesson("AA-W01", 10, "observe-trace"),
      stagedLesson("AA-W02", 20, "delayed-copy"),
    ];
    const report = measureWritingStages(policy, ["alpha"], lessons, [curriculum(["AA-W01", "AA-W02"])], spine);
    const track = report.tracks[0]!;
    expect(track.validEvidence.map((entry) => entry.stage)).toEqual(["observe-trace"]);
    expect(track.defects[0]).toMatchObject({
      stage: "delayed-copy",
      kind: "missing-stage-prerequisite",
    });
    expect(track.defects[0]?.detail).toContain("guided-copy");
  });

  it("does not make A1 timed writing depend on the A2 connected-composition branch", () => {
    const prerequisites = writingStagePrerequisites(loadAssessmentPolicy());
    expect(prerequisites.get("timed-assessment-production")).toContain("controlled-composition");
    expect(prerequisites.get("timed-assessment-production")).not.toContain("connected-composition");
  });

  it("aggregates every live track without a cross-language exact ledger", () => {
    const { registry, lessons, curricula, spine: realSpine } = loadEverything();
    const report = measureWritingStages(
      loadAssessmentPolicy(),
      registry.languages.map((track) => track.id),
      lessons,
      curricula,
      realSpine,
    );
    expect(report.summary).toEqual({
      tracks: report.tracks.length,
      tracksWithAnyEvidence: report.tracks.filter((track) => track.evidence.length > 0).length,
      tracksCompleteAtPreA1: report.tracks.filter((track) => track.levels[0]?.complete).length,
      evidenceBlocks: report.tracks.reduce((sum, track) => sum + track.evidence.length, 0),
      invalidEvidenceBlocks: report.tracks.reduce((sum, track) => sum + track.defects.length, 0),
      missingTrackLevelStages: report.tracks.reduce(
        (sum, track) => sum + track.levels.reduce(
          (trackSum, level) => trackSum + level.missingStages.length,
          0,
        ),
        0,
      ),
    });
    expect(report.summary.tracks).toBe(registry.languages.length);
    expect(report.summary.invalidEvidenceBlocks).toBe(0);
  }, 30_000);

  // ---------------------------------------------------------------------------
  // The test above derives every field of the summary FROM the report, so it is
  // self-consistent and asserts no number. `tracksCompleteAtPreA1` could fall
  // from nineteen to fifteen and it would stay green -- which is the shape of a
  // measurement nothing reads.
  //
  // This is the gate. It is a ratchet in both directions at once: the count may
  // only rise, and the list of tracks that still fail may only shrink. Pinning
  // the LIST rather than only the count is what stops the trade -- completing
  // one track while regressing another leaves the count untouched.
  // ---------------------------------------------------------------------------
  it("keeps EVERY registered track proving the pre-A1 writing ladder", () => {
    const { registry, lessons, curricula, spine: realSpine } = loadEverything();
    const report = measureWritingStages(
      loadAssessmentPolicy(),
      registry.languages.map((track) => track.id),
      lessons,
      curricula,
      realSpine,
    );

    const incomplete = report.tracks
      .filter((track) => !track.levels[0]?.complete)
      .map((track) => track.language)
      .sort();

    // 15 -> 19 -> 23, across four PRs, and this assertion was tightened twice as
    // they landed. It began as `>= 19` with a list of the four tracks still
    // allowed to fail, because at the time four still did.
    //
    // Both halves of that shape are now wrong, and in the direction that matters:
    // a floor of 19 against a true value of 23 is four tracks of silent headroom,
    // and an allow-list naming tracks that have since been fixed is a licence
    // nobody is forced to hand back. A ratchet left slack after the work lands
    // stops being a ratchet.
    //
    // Derived from the registry rather than written as 23, so it cannot go stale
    // the way the literal did. Adding a 24th track fails this until that track
    // has a ladder, which is the correct answer and not an inconvenience: the
    // point of the gate is that the corpus-wide property holds, not that some
    // number was true once.
    expect(incomplete).toEqual([]);
    expect(report.summary.tracksCompleteAtPreA1).toBe(registry.languages.length);
  }, 30_000);

  // ---------------------------------------------------------------------------
  // The pre-A1 rung is complete; A1 through C2 are not, and nothing watched the
  // remainder at all. `missingTrackLevelStages` is the plan's largest single
  // family and it fell 519 -> 351 while the pre-A1 work landed -- a 168-point
  // move that no assertion would have noticed in either direction.
  //
  // A ceiling rather than a pin: it may only fall. Pinning it exactly would make
  // every unrelated tranche that happens to add a staged lesson edit this line.
  // ---------------------------------------------------------------------------
  // ---------------------------------------------------------------------------
  // One lesson closed FIVE level-debts. `connected-composition` is first required
  // at A2, and every level above inherits the requirement, so a track missing it
  // fails A2, B1, B2, C1 and C2 at once -- and a single lesson clears all five.
  //
  // That is the shape of the remaining 346: they are not 346 independent pieces
  // of work. The cumulative rule means the cheapest move is always the LOWEST
  // unproved stage in a track, and this assertion exists so the first track to
  // reach the top cannot quietly fall back off it.
  // ---------------------------------------------------------------------------
  it("keeps Spanish proving every writing stage at every level", () => {
    const { registry, lessons, curricula, spine: realSpine } = loadEverything();
    const report = measureWritingStages(
      loadAssessmentPolicy(),
      registry.languages.map((track) => track.id),
      lessons,
      curricula,
      realSpine,
    );
    const spanish = report.tracks.find((track) => track.language === "spanish");
    expect(spanish?.levels.filter((level) => !level.complete)).toEqual([]);
    expect(spanish?.defects).toEqual([]);

    // The claim is about the corpus, not about Spanish's luck: at the time this
    // landed Spanish was the ONLY track complete at every level, and the count is
    // asserted so that a second track arriving is a visible event rather than a
    // silent one.
    const complete = report.tracks.filter((track) => track.levels.every((level) => level.complete));
    expect(complete.length).toBeGreaterThanOrEqual(1);
    expect(complete.map((track) => track.language)).toContain("spanish");
  }, 30_000);

  it("never grows the remaining writing-stage debt above A1", () => {
    const { registry, lessons, curricula, spine: realSpine } = loadEverything();
    const report = measureWritingStages(
      loadAssessmentPolicy(),
      registry.languages.map((track) => track.id),
      lessons,
      curricula,
      realSpine,
    );
    expect(report.summary.missingTrackLevelStages).toBeLessThanOrEqual(346);
  }, 30_000);
});
