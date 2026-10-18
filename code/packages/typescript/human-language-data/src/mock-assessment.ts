import { readFileSync } from "node:fs";
import { join } from "node:path";

import type { CefrLevel } from "./levels.js";
import { auditTrackArtifacts } from "./assessment-artifacts.js";
import { loadAssessmentContracts, loadAssessmentPolicy, loadTaskShapeInventory, listTaskShapeInventories, defaultCurriculumRoot } from "./loader.js";
import { parseMockManifest, validateMockAnswerKey, validateMockPaper, validateMockRubric } from "./mock-manifest.js";
import { readLedgerFile } from "./shard.js";

export interface MockAssessmentCoverage {
  language: string;
  level: CefrLevel;
  requiredMocks: number;
  completeMocks: number;
  missingMocks: string[];
  /** An absent inventory is a prerequisite, not evidence of zero mock debt. */
  blockedBy: string[];
}

/**
 * Required artifact-gate companion: every declared, present manifest is
 * validated even when no completion-plan command runs. Missing declared files
 * remain the artifact gate's own diagnostic; undeclared legacy manifests are
 * measured by the plan, not made into parser errors during migration.
 */
export function validateDeclaredMockManifests(root = defaultCurriculumRoot()): void {
  for (const { language, contract } of loadAssessmentContracts(root)) {
    if (!contract.levels.some((level) => level.fullMocks.some((mock) => mock.manifest !== undefined))) continue;
    const present = new Set(auditTrackArtifacts(root, language, contract).present);
    for (const level of contract.levels) {
      for (const mock of level.fullMocks) {
        if (mock.manifest === undefined || !present.has(mock.manifest)) continue;
        const shape = loadTaskShapeInventory(language, level.level, root);
        const parsed = parseMockManifest(
          readLedgerFile(join(root, language, mock.manifest)), language, level.level, mock.id, shape,
        );
        const paperPath = mock.paper?.split("#", 1)[0];
        if (paperPath !== undefined && present.has(paperPath)) {
          validateMockPaper(parsed, readFileSync(join(root, language, paperPath), "utf8"));
        }
        const rubricPath = mock.rubric.split("#", 1)[0]!;
        if (present.has(rubricPath)) {
          validateMockRubric(parsed, readFileSync(join(root, language, rubricPath), "utf8"));
        }
        const answerKeyPath = mock.answerKey.split("#", 1)[0]!;
        if (present.has(answerKeyPath)) {
          validateMockAnswerKey(parsed, readFileSync(join(root, language, answerKeyPath), "utf8"));
        }
      }
    }
  }
}

/** Measure full-mock evidence without treating legacy timed declarations as complete papers. */
export function listMockAssessmentStatuses(root = defaultCurriculumRoot()): MockAssessmentCoverage[] {
  const requiredMocks = loadAssessmentPolicy(root).passEvidence.minimumFullMocksPerLevel;
  const haveShape = new Set(listTaskShapeInventories(root).map(({ language, level }) => `${language}/${level}`));
  return loadAssessmentContracts(root).flatMap(({ language, contract }) => {
    // This call checks canonical containment and regular-file identity for every
    // declared artifact before a manifest is opened. A symlinked parent cannot
    // silently let evidence from another track satisfy this status.
    const present = new Set(auditTrackArtifacts(root, language, contract).present);
    return contract.levels.map((level) => {
      const shapeId = `${language}/${level.level}`;
      if (!haveShape.has(shapeId)) {
        return {
          language, level: level.level, requiredMocks,
          completeMocks: 0, missingMocks: level.fullMocks.slice(0, requiredMocks).map((mock) => mock.id),
          blockedBy: [`task-shape/${shapeId}`],
        };
      }
      const shape = loadTaskShapeInventory(language, level.level, root);
      const incompleteMocks = level.fullMocks.filter((mock) => {
        const evidence = [mock.paper, mock.rubric, mock.answerKey, mock.manifest];
        const paths = evidence.map((reference) => reference?.split("#", 1)[0]);
        // Neither a paper nor a rubric nor a key nor a structure manifest can
        // stand in for another of those independent pieces of evidence.
        if (paths.some((path) => path === undefined || !present.has(path))) return true;
        if (new Set(paths).size !== paths.length) return true;
        const manifestPath = paths[3]!;
        const parsed = parseMockManifest(
          readLedgerFile(join(root, language, manifestPath)), language, level.level, mock.id, shape,
        );
        validateMockPaper(parsed, readFileSync(join(root, language, paths[0]!), "utf8"));
        validateMockRubric(parsed, readFileSync(join(root, language, paths[1]!), "utf8"));
        validateMockAnswerKey(parsed, readFileSync(join(root, language, paths[2]!), "utf8"));
        return false;
      }).map((mock) => mock.id);
      const completeMocks = level.fullMocks.length - incompleteMocks.length;
      return {
        language, level: level.level, requiredMocks,
        completeMocks,
        missingMocks: incompleteMocks.slice(0, Math.max(0, requiredMocks - completeMocks)),
        blockedBy: [],
      };
    });
  });
}
