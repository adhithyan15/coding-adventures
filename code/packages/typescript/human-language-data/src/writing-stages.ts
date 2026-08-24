import type { AssessmentPolicy } from "./assessment.js";
import { CEFR_LEVELS, levelRank, lessonSpineNodes, type CefrLevel } from "./levels.js";
import type { ParsedLesson } from "./parse.js";
import type { CurriculumSpine, LanguageCurriculum } from "./types.js";

export type WritingStageDefectKind =
  | "unknown-stage"
  | "missing-writing-skill"
  | "empty-assessment"
  | "unmapped-level"
  | "unordered-evidence"
  | "missing-stage-prerequisite";

export interface WritingStageEvidence {
  language: string;
  lessonId: string;
  blockIndex: number;
  blockTitle: string;
  stage: string;
  level: CefrLevel | null;
  sequence: number | null;
}
export interface WritingStageDefect {
  language: string;
  lessonId: string;
  blockTitle: string;
  stage: string;
  kind: WritingStageDefectKind;
  detail: string;
}

export interface WritingStageLevelCoverage {
  level: CefrLevel;
  requiredStages: string[];
  evidencedStages: string[];
  missingStages: string[];
  complete: boolean;
}

export interface TrackWritingStageCoverage {
  language: string;
  evidence: WritingStageEvidence[];
  validEvidence: WritingStageEvidence[];
  defects: WritingStageDefect[];
  levels: WritingStageLevelCoverage[];
}

export interface WritingStageReport {
  stages: Array<{
    id: string;
    firstRequiredAt: CefrLevel;
    prerequisites: string[];
  }>;
  tracks: TrackWritingStageCoverage[];
  summary: {
    tracks: number;
    tracksWithAnyEvidence: number;
    tracksCompleteAtPreA1: number;
    evidenceBlocks: number;
    invalidEvidenceBlocks: number;
    missingTrackLevelStages: number;
  };
}

/**
 * Earlier stages at the same or a lower requirement level are prerequisites.
 *
 * This produces the intended fork in the current policy: connected composition
 * (first required at A2) and timed assessment production (first required at A1)
 * both depend on controlled composition, but A1 timed work cannot depend on an
 * A2 stage. At A2 both branches are required by the level matrix.
 */
export function writingStagePrerequisites(policy: AssessmentPolicy): Map<string, string[]> {
  return new Map(policy.writingStages.map((stage, index) => [
    stage.id,
    policy.writingStages
      .slice(0, index)
      .filter((candidate) => levelRank(candidate.firstRequiredAt) <= levelRank(stage.firstRequiredAt))
      .map((candidate) => candidate.id),
  ]));
}

function stringList(value: ParsedLesson["frontmatter"][string] | undefined): string[] {
  if (Array.isArray(value)) return value;
  return typeof value === "string" && value.trim() !== "" ? [value] : [];
}

function sequenceOf(lesson: ParsedLesson): number | null {
  const raw = lesson.frontmatter.sequence;
  const value = typeof raw === "string" ? Number(raw) : NaN;
  return Number.isFinite(value) ? value : null;
}

/** Measure explicit writing-stage evidence for every registered track and level. */
export function measureWritingStages(
  policy: AssessmentPolicy,
  languages: readonly string[],
  lessons: readonly ParsedLesson[],
  curricula: readonly LanguageCurriculum[],
  spine: CurriculumSpine,
): WritingStageReport {
  const stageIds = new Set(policy.writingStages.map((stage) => stage.id));
  const prerequisites = writingStagePrerequisites(policy);
  const nodeOf = lessonSpineNodes([...curricula]);
  const stageOfNode = new Map<string, CefrLevel>();
  for (const node of spine.nodes) {
    const current = CEFR_LEVELS.find((candidate) => candidate === node.stage);
    if (current) stageOfNode.set(node.id, current);
  }
  const lessonLevel = (lesson: ParsedLesson): CefrLevel | null => {
    const node = nodeOf.get(lesson.realization.lessonId);
    return node ? (stageOfNode.get(node) ?? null) : null;
  };

  const tracks = languages.map((language): TrackWritingStageCoverage => {
    const trackLessons = lessons.filter((lesson) => lesson.language === language);
    const evidence: WritingStageEvidence[] = [];
    for (const lesson of trackLessons) {
      lesson.blocks.forEach((block, blockIndex) => {
        if (!block.writingStage) return;
        evidence.push({
          language,
          lessonId: lesson.realization.lessonId,
          blockIndex,
          blockTitle: block.title,
          stage: block.writingStage,
          level: lessonLevel(lesson),
          sequence: sequenceOf(lesson),
        });
      });
    }
    evidence.sort((left, right) =>
      (left.sequence ?? Number.MAX_SAFE_INTEGER) - (right.sequence ?? Number.MAX_SAFE_INTEGER) ||
      left.blockIndex - right.blockIndex ||
      left.lessonId.localeCompare(right.lessonId),
    );

    const lessonById = new Map(trackLessons.map((lesson) => [lesson.realization.lessonId, lesson]));
    const completed = new Set<string>();
    const validEvidence: WritingStageEvidence[] = [];
    const defects: WritingStageDefect[] = [];
    const defect = (item: WritingStageEvidence, kind: WritingStageDefectKind, detail: string): void => {
      defects.push({
        language,
        lessonId: item.lessonId,
        blockTitle: item.blockTitle,
        stage: item.stage,
        kind,
        detail,
      });
    };

    for (const item of evidence) {
      const lesson = lessonById.get(item.lessonId)!;
      const block = lesson.blocks[item.blockIndex]!;
      if (!stageIds.has(item.stage)) {
        defect(item, "unknown-stage", `stage '${item.stage}' is absent from assessment-policy.json`);
        continue;
      }
      if (!stringList(lesson.frontmatter.skills).includes("writing")) {
        defect(item, "missing-writing-skill", "lesson does not declare the writing skill");
        continue;
      }
      if ((block.knowledge?.assesses.length ?? 0) === 0) {
        defect(item, "empty-assessment", "evidence block assesses no declared knowledge");
        continue;
      }
      if (item.level === null) {
        defect(item, "unmapped-level", "lesson has no derived curriculum level");
        continue;
      }
      if (item.sequence === null) {
        defect(item, "unordered-evidence", "lesson has no finite sequence, so cumulative evidence order is unknown");
        continue;
      }
      const missing = (prerequisites.get(item.stage) ?? []).filter((stage) => !completed.has(stage));
      if (missing.length > 0) {
        defect(
          item,
          "missing-stage-prerequisite",
          `requires earlier evidence for ${missing.join(", ")}`,
        );
        continue;
      }
      completed.add(item.stage);
      validEvidence.push(item);
    }

    const levels = CEFR_LEVELS.map((level): WritingStageLevelCoverage => {
      const requiredStages = policy.writingStages
        .filter((stage) => levelRank(stage.firstRequiredAt) <= levelRank(level))
        .map((stage) => stage.id);
      const evidencedStages = [...new Set(validEvidence
        .filter((item) => item.level !== null && levelRank(item.level) <= levelRank(level))
        .map((item) => item.stage))];
      const missingStages = requiredStages.filter((stage) => !evidencedStages.includes(stage));
      return { level, requiredStages, evidencedStages, missingStages, complete: missingStages.length === 0 };
    });
    return { language, evidence, validEvidence, defects, levels };
  }).sort((left, right) => left.language.localeCompare(right.language));

  return {
    stages: policy.writingStages.map((stage) => ({
      id: stage.id,
      firstRequiredAt: stage.firstRequiredAt,
      prerequisites: [...(prerequisites.get(stage.id) ?? [])],
    })),
    tracks,
    summary: {
      tracks: tracks.length,
      tracksWithAnyEvidence: tracks.filter((track) => track.evidence.length > 0).length,
      tracksCompleteAtPreA1: tracks.filter((track) => track.levels[0]?.complete).length,
      evidenceBlocks: tracks.reduce((sum, track) => sum + track.evidence.length, 0),
      invalidEvidenceBlocks: tracks.reduce((sum, track) => sum + track.defects.length, 0),
      missingTrackLevelStages: tracks.reduce(
        (sum, track) => sum + track.levels.reduce((trackSum, level) => trackSum + level.missingStages.length, 0),
        0,
      ),
    },
  };
}
