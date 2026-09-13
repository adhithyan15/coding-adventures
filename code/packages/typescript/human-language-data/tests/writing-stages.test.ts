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
  it("never loses a track that proves the pre-A1 writing ladder", () => {
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

    // 15 -> 19: german, italian, persian and portuguese each proved observe-trace
    // and guided-copy and stopped there. Each gained a delayed copy with the
    // model covered and a dictation from the sound, on the word or letter it
    // already had -- the stages measure what the hand is asked to do, not how
    // much language is on the page.
    expect(report.summary.tracksCompleteAtPreA1).toBeGreaterThanOrEqual(19);

    // The four that remain have NO stage evidence at all, not a partial ladder:
    // between them they hold 223 script lessons and not one writing-stage
    // directive, which is a different and larger piece of work than this was.
    // A track may leave this list; none may join it.
    for (const language of incomplete) {
      expect(["bengali", "kannada", "sanskrit", "telugu"]).toContain(language);
    }
  }, 30_000);
});
