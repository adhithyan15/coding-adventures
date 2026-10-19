import { join } from "node:path";

import type { CefrLevel } from "./levels.js";
import { auditTrackArtifacts } from "./assessment-artifacts.js";
import { loadAssessmentContracts, loadAssessmentPolicy, loadTaskShapeInventory, listTaskShapeInventories, defaultCurriculumRoot } from "./loader.js";
import { parseMockManifest, validateMockAnswerKey, validateMockPaper, validateMockRubric } from "./mock-manifest.js";
import { readMockEvidence } from "./mock-evidence-read.js";
import { isSharded, parseLedgerText, shardDirectoryFor } from "./shard.js";

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
    const requested = new Set<string>();
    for (const level of contract.levels) {
      for (const mock of level.fullMocks) {
        if (mock.manifest === undefined || !present.has(mock.manifest)) continue;
        for (const reference of [mock.manifest, mock.paper, mock.rubric, mock.answerKey]) {
          const path = reference?.split("#", 1)[0];
          if (path !== undefined && present.has(path)) requested.add(path);
        }
      }
    }
    const evidence = readMockEvidence(root, language, [...requested]);
    for (const level of contract.levels) {
      for (const mock of level.fullMocks) {
        if (mock.manifest === undefined || !present.has(mock.manifest)) continue;
        const shape = loadTaskShapeInventory(language, level.level, root);
        const parsed = parseMockManifest(
          readMockManifest(evidence, root, language, mock.manifest), language, level.level, mock.id, shape,
        );
        const paperPath = mock.paper?.split("#", 1)[0];
        if (paperPath !== undefined && present.has(paperPath)) {
          validateMockPaper(parsed, requiredEvidence(evidence, paperPath));
        }
        const rubricPath = mock.rubric.split("#", 1)[0]!;
        if (present.has(rubricPath)) {
          validateMockRubric(parsed, requiredEvidence(evidence, rubricPath));
        }
        const answerKeyPath = mock.answerKey.split("#", 1)[0]!;
        if (present.has(answerKeyPath)) {
          validateMockAnswerKey(parsed, requiredEvidence(evidence, answerKeyPath));
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
    const requested = new Set<string>();
    for (const level of contract.levels) {
      if (!haveShape.has(`${language}/${level.level}`)) continue;
      for (const mock of level.fullMocks) {
        const paths = [mock.paper, mock.rubric, mock.answerKey, mock.manifest]
          .map((reference) => reference?.split("#", 1)[0]);
        if (paths.some((path) => path === undefined || !present.has(path))) continue;
        if (new Set(paths).size !== paths.length) continue;
        for (const path of paths) requested.add(path!);
      }
    }
    const evidence = readMockEvidence(root, language, [...requested]);
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
        const references = [mock.paper, mock.rubric, mock.answerKey, mock.manifest];
        const paths = references.map((reference) => reference?.split("#", 1)[0]);
        // Neither a paper nor a rubric nor a key nor a structure manifest can
        // stand in for another of those independent pieces of evidence.
        if (paths.some((path) => path === undefined || !present.has(path))) return true;
        if (new Set(paths).size !== paths.length) return true;
        const manifestPath = paths[3]!;
        const parsed = parseMockManifest(
          readMockManifest(evidence, root, language, manifestPath), language, level.level, mock.id, shape,
        );
        validateMockPaper(parsed, requiredEvidence(evidence, paths[0]!));
        validateMockRubric(parsed, requiredEvidence(evidence, paths[1]!));
        validateMockAnswerKey(parsed, requiredEvidence(evidence, paths[2]!));
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

function requiredEvidence(evidence: Map<string, string>, path: string): string {
  const text = evidence.get(path);
  if (text === undefined) throw new Error(`mock evidence: anchored read omitted '${path}'`);
  return text;
}

function readMockManifest(
  evidence: Map<string, string>, root: string, language: string, path: string,
): unknown {
  const fullPath = join(root, language, path);
  if (isSharded(fullPath)) {
    throw new Error(`'${fullPath}': is sharded into '${shardDirectoryFor(fullPath)}'`);
  }
  return parseLedgerText(requiredEvidence(evidence, path), fullPath);
}
