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

  it("measures every live track and pins Marwadi's gentle pre-A1 proof", () => {
    const { registry, lessons, curricula, spine: realSpine } = loadEverything();
    const report = measureWritingStages(
      loadAssessmentPolicy(),
      registry.languages.map((track) => track.id),
      lessons,
      curricula,
      realSpine,
    );
    expect(report.summary).toEqual({
      tracks: 23,
      tracksWithAnyEvidence: 1,
      tracksCompleteAtPreA1: 1,
      evidenceBlocks: 4,
      invalidEvidenceBlocks: 0,
      missingTrackLevelStages: 1007,
    });
    const marwadi = report.tracks.find((track) => track.language === "marwadi")!;
    expect(marwadi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
    expect(marwadi.validEvidence.map((entry) => entry.stage)).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(report.tracks.filter((track) => track.levels[0]?.complete)).toHaveLength(1);
  }, 30_000);
});
