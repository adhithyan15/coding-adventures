import { CEFR_LEVELS, levelRank, type CefrLevel } from "./levels.js";

export const ASSESSMENT_SKILLS = ["reading", "listening", "writing", "speaking"] as const;
export type AssessmentSkill = (typeof ASSESSMENT_SKILLS)[number];

export interface WritingStagePolicy {
  id: string;
  firstRequiredAt: CefrLevel;
  description: string;
}

export interface AssessmentPolicy {
  version: number;
  maxLessonMinutes: number;
  levels: CefrLevel[];
  skills: AssessmentSkill[];
  writingStages: WritingStagePolicy[];
  passEvidence: {
    skillsPassIndependently: true;
    minimumFullMocksPerLevel: number;
    requiresTimedMocks: true;
    requiresRubric: true;
    requiresAnswerKey: true;
    requiresHumanValidation: true;
  };
}

/** One complete simulation and the checked-in evidence that a human validated it. */
export interface AssessmentFullMock {
  id: string;
  timed: boolean;
  rubric: string;
  answerKey: string;
  /**
   * Safe relative reference to the pilot/reviewer record for this exact mock.
   *
   * Optional while the corpus migrates. Absence is measured completion debt;
   * making it a parse error would hide that debt by making every existing
   * assessment contract unreadable at once.
   */
  humanValidation?: string;
}

export interface AssessmentContract {
  version: number;
  language: string;
  externalCapstones: Array<{
    id: string;
    target: {
      name: string;
      basis: "external";
      source: string;
      edition: string;
      accessed: string;
    };
    requiredAfterLevel: CefrLevel;
    cefrRelation: "not-mapped";
    skills: Record<AssessmentSkill, {
      taskInventory: string[];
      passThreshold: number;
    }>;
    additionalComponents: Record<string, {
      name: string;
      taskInventory: string[];
      passThreshold: number;
    }>;
    fullMocks: AssessmentFullMock[];
  }>;
  levels: Array<{
    level: CefrLevel;
    target: {
      name: string;
      basis: "external" | "project-defined";
      source: string;
    };
    skills: Record<AssessmentSkill, {
      taskInventory: string[];
      passThreshold: number;
    }>;
    additionalComponents: Record<string, {
      name: string;
      taskInventory: string[];
      passThreshold: number;
    }>;
    writingStages: string[];
    fullMocks: AssessmentFullMock[];
  }>;
}

function object(value: unknown, where: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`assessment: ${where} must be an object`);
  }
  return value as Record<string, unknown>;
}

function nonEmpty(value: unknown, where: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`assessment: ${where} must be a non-empty string`);
  }
  return value;
}

function level(value: unknown, where: string): CefrLevel {
  const found = CEFR_LEVELS.find((candidate) => candidate === value);
  if (!found) throw new Error(`assessment: ${where} must be one of ${CEFR_LEVELS.join(", ")}`);
  return found;
}

function exactStrings(value: unknown, expected: readonly string[], where: string): string[] {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`assessment: ${where} must be a string array`);
  }
  if (value.length !== expected.length || expected.some((item) => !value.includes(item))) {
    throw new Error(`assessment: ${where} must contain exactly ${expected.join(", ")}`);
  }
  return [...value] as string[];
}

function artifactReference(value: unknown, where: string): string {
  const reference = nonEmpty(value, where);
  const path = reference.split("#", 1)[0] ?? "";
  if (
    path === ""
    || path.startsWith("/")
    || path.startsWith("\\")
    || /^[A-Za-z]:/.test(path)
    || path.includes("\\")
    || path.split("/").includes("..")
    || /^[a-z][a-z0-9+.-]*:/i.test(path)
  ) {
    throw new Error(`assessment: ${where} must be a safe relative artifact reference`);
  }
  return reference;
}

export function parseAssessmentPolicy(value: unknown): AssessmentPolicy {
  const raw = object(value, "policy");
  if (raw.version !== 1) throw new Error("assessment: policy version must be 1");
  if (raw.maxLessonMinutes !== 5) {
    throw new Error("assessment: maxLessonMinutes must be 5; a larger value breaks the gentle-ramp contract");
  }
  const levels = exactStrings(raw.levels, CEFR_LEVELS, "policy.levels") as CefrLevel[];
  const skills = exactStrings(raw.skills, ASSESSMENT_SKILLS, "policy.skills") as AssessmentSkill[];
  if (!Array.isArray(raw.writingStages) || raw.writingStages.length === 0) {
    throw new Error("assessment: policy.writingStages must be a non-empty array");
  }
  const stageIds = new Set<string>();
  const writingStages = raw.writingStages.map((entry, index) => {
    const stage = object(entry, `policy.writingStages[${index}]`);
    const id = nonEmpty(stage.id, `policy.writingStages[${index}].id`);
    if (stageIds.has(id)) throw new Error(`assessment: duplicate writing stage '${id}'`);
    stageIds.add(id);
    return {
      id,
      firstRequiredAt: level(stage.firstRequiredAt, `policy.writingStages[${index}].firstRequiredAt`),
      description: nonEmpty(stage.description, `policy.writingStages[${index}].description`),
    };
  });
  const evidence = object(raw.passEvidence, "policy.passEvidence");
  for (const key of [
    "skillsPassIndependently",
    "requiresTimedMocks",
    "requiresRubric",
    "requiresAnswerKey",
    "requiresHumanValidation",
  ]) {
    if (evidence[key] !== true) throw new Error(`assessment: policy.passEvidence.${key} must be true`);
  }
  if (!Number.isInteger(evidence.minimumFullMocksPerLevel) || (evidence.minimumFullMocksPerLevel as number) < 1) {
    throw new Error("assessment: policy.passEvidence.minimumFullMocksPerLevel must be a positive integer");
  }
  return {
    version: 1,
    maxLessonMinutes: 5,
    levels,
    skills,
    writingStages,
    passEvidence: evidence as unknown as AssessmentPolicy["passEvidence"],
  };
}

export function parseAssessmentContract(
  value: unknown,
  expectedLanguage: string,
  policy: AssessmentPolicy,
): AssessmentContract {
  const raw = object(value, `${expectedLanguage}/assessment.json`);
  if (raw.version !== 1) throw new Error(`assessment: ${expectedLanguage} contract version must be 1`);
  if (raw.language !== expectedLanguage) {
    throw new Error(`assessment: ${expectedLanguage} contract declares language '${String(raw.language)}'`);
  }
  if (!Array.isArray(raw.levels)) throw new Error(`assessment: ${expectedLanguage}.levels must be an array`);
  const seen = new Set<CefrLevel>();
  const levels = raw.levels.map((entry, index) => {
    const item = object(entry, `${expectedLanguage}.levels[${index}]`);
    const current = level(item.level, `${expectedLanguage}.levels[${index}].level`);
    if (seen.has(current)) throw new Error(`assessment: ${expectedLanguage} duplicates level ${current}`);
    seen.add(current);
    const target = object(item.target, `${expectedLanguage}.${current}.target`);
    if (target.basis !== "external" && target.basis !== "project-defined") {
      throw new Error(`assessment: ${expectedLanguage}.${current}.target.basis must be external or project-defined`);
    }
    const basis: "external" | "project-defined" = target.basis;
    const skills = object(item.skills, `${expectedLanguage}.${current}.skills`);
    const parsedSkills = Object.fromEntries(ASSESSMENT_SKILLS.map((skill) => {
      const skillRaw = object(skills[skill], `${expectedLanguage}.${current}.skills.${skill}`);
      if (!Array.isArray(skillRaw.taskInventory) || skillRaw.taskInventory.some((task) => typeof task !== "string")) {
        throw new Error(`assessment: ${expectedLanguage}.${current}.${skill}.taskInventory must be a string array`);
      }
      // `artifactReference`, not a bare string check. These are paths that a
      // checker joins to the track directory and stats, and the CEFR half of the
      // contract was the only half not validating their shape — the capstone half
      // below has always done so. A rule spelled in one of two places is a rule
      // whose absence in the other nobody notices (see loader.ts's TRACK_ID note).
      const taskInventory = skillRaw.taskInventory.map((reference, referenceIndex) =>
        artifactReference(
          reference,
          `${expectedLanguage}.${current}.skills.${skill}.taskInventory[${referenceIndex}]`,
        )
      );
      if (typeof skillRaw.passThreshold !== "number" || skillRaw.passThreshold <= 0 || skillRaw.passThreshold > 1) {
        throw new Error(`assessment: ${expectedLanguage}.${current}.${skill}.passThreshold must be in (0, 1]`);
      }
      return [skill, { taskInventory, passThreshold: skillRaw.passThreshold }];
    })) as AssessmentContract["levels"][number]["skills"];
    const additionalRaw = item.additionalComponents === undefined
      ? {}
      : object(item.additionalComponents, `${expectedLanguage}.${current}.additionalComponents`);
    const additionalComponents = Object.fromEntries(Object.entries(additionalRaw).map(([id, component]) => {
      if (!/^[a-z][a-z0-9-]*$/.test(id)) {
        throw new Error(
          `assessment: ${expectedLanguage}.${current}.additionalComponents key '${id}' must be a lowercase slug`,
        );
      }
      if (ASSESSMENT_SKILLS.includes(id as AssessmentSkill)) {
        throw new Error(
          `assessment: ${expectedLanguage}.${current}.additionalComponents '${id}' duplicates a universal skill`,
        );
      }
      const componentRaw = object(
        component,
        `${expectedLanguage}.${current}.additionalComponents.${id}`,
      );
      if (
        !Array.isArray(componentRaw.taskInventory)
        || componentRaw.taskInventory.length === 0
        || componentRaw.taskInventory.some((task) => typeof task !== "string" || task.trim() === "")
      ) {
        throw new Error(
          `assessment: ${expectedLanguage}.${current}.additionalComponents.${id}.taskInventory must be a non-empty string array`,
        );
      }
      if (
        typeof componentRaw.passThreshold !== "number"
        || componentRaw.passThreshold <= 0
        || componentRaw.passThreshold > 1
      ) {
        throw new Error(
          `assessment: ${expectedLanguage}.${current}.additionalComponents.${id}.passThreshold must be in (0, 1]`,
        );
      }
      return [id, {
        name: nonEmpty(
          componentRaw.name,
          `${expectedLanguage}.${current}.additionalComponents.${id}.name`,
        ),
        taskInventory: (componentRaw.taskInventory as string[]).map((reference, referenceIndex) =>
          artifactReference(
            reference,
            `${expectedLanguage}.${current}.additionalComponents.${id}.taskInventory[${referenceIndex}]`,
          )
        ),
        passThreshold: componentRaw.passThreshold,
      }];
    }));
    if (!Array.isArray(item.writingStages) || item.writingStages.some((stage) => typeof stage !== "string")) {
      throw new Error(`assessment: ${expectedLanguage}.${current}.writingStages must be a string array`);
    }
    const requiredStages = policy.writingStages
      .filter((stage) => levelRank(stage.firstRequiredAt) <= levelRank(current))
      .map((stage) => stage.id);
    for (const required of requiredStages) {
      if (!item.writingStages.includes(required)) {
        throw new Error(`assessment: ${expectedLanguage}.${current} is missing writing stage '${required}'`);
      }
    }
    if (!Array.isArray(item.fullMocks) || item.fullMocks.length < policy.passEvidence.minimumFullMocksPerLevel) {
      throw new Error(
        `assessment: ${expectedLanguage}.${current} needs at least ${policy.passEvidence.minimumFullMocksPerLevel} full mocks`,
      );
    }
    const fullMocks = item.fullMocks.map((mock, mockIndex) => {
      const mockRaw = object(mock, `${expectedLanguage}.${current}.fullMocks[${mockIndex}]`);
      if (mockRaw.timed !== true) throw new Error(`assessment: ${expectedLanguage}.${current} mock must be timed`);
      const humanValidation = mockRaw.humanValidation === undefined
        ? undefined
        : artifactReference(
            mockRaw.humanValidation,
            `${expectedLanguage}.${current}.fullMocks[${mockIndex}].humanValidation`,
          );
      return {
        id: nonEmpty(mockRaw.id, `${expectedLanguage}.${current}.mock.id`),
        timed: true,
        rubric: artifactReference(mockRaw.rubric, `${expectedLanguage}.${current}.mock.rubric`),
        answerKey: artifactReference(mockRaw.answerKey, `${expectedLanguage}.${current}.mock.answerKey`),
        ...(humanValidation === undefined ? {} : { humanValidation }),
      };
    });
    return {
      level: current,
      target: {
        name: nonEmpty(target.name, `${expectedLanguage}.${current}.target.name`),
        basis,
        source: nonEmpty(target.source, `${expectedLanguage}.${current}.target.source`),
      },
      skills: parsedSkills,
      additionalComponents,
      writingStages: [...item.writingStages] as string[],
      fullMocks,
    };
  });
  for (const required of policy.levels) {
    if (!seen.has(required)) throw new Error(`assessment: ${expectedLanguage} is missing level ${required}`);
  }

  const capstonesRaw = raw.externalCapstones === undefined ? [] : raw.externalCapstones;
  if (!Array.isArray(capstonesRaw)) {
    throw new Error(`assessment: ${expectedLanguage}.externalCapstones must be an array`);
  }
  const capstoneIds = new Set<string>();
  const externalCapstones = capstonesRaw.map((entry, index) => {
    const where = `${expectedLanguage}.externalCapstones[${index}]`;
    const item = object(entry, where);
    const id = nonEmpty(item.id, `${where}.id`);
    if (!/^[a-z][a-z0-9-]*$/.test(id)) {
      throw new Error(`assessment: ${where}.id must be a lowercase slug`);
    }
    if (capstoneIds.has(id)) throw new Error(`assessment: ${expectedLanguage} duplicates capstone '${id}'`);
    capstoneIds.add(id);

    const target = object(item.target, `${where}.target`);
    if (target.basis !== "external") {
      throw new Error(`assessment: ${where}.target.basis must be external`);
    }
    if (item.cefrRelation !== "not-mapped") {
      throw new Error(`assessment: ${where}.cefrRelation must be 'not-mapped'`);
    }

    const skills = object(item.skills, `${where}.skills`);
    const parsedSkills = Object.fromEntries(ASSESSMENT_SKILLS.map((skill) => {
      const skillRaw = object(skills[skill], `${where}.skills.${skill}`);
      if (!Array.isArray(skillRaw.taskInventory) || skillRaw.taskInventory.length === 0) {
        throw new Error(`assessment: ${where}.skills.${skill}.taskInventory must be a non-empty array`);
      }
      const taskInventory = skillRaw.taskInventory.map((reference, referenceIndex) =>
        artifactReference(reference, `${where}.skills.${skill}.taskInventory[${referenceIndex}]`)
      );
      if (typeof skillRaw.passThreshold !== "number" || skillRaw.passThreshold <= 0 || skillRaw.passThreshold > 1) {
        throw new Error(`assessment: ${where}.skills.${skill}.passThreshold must be in (0, 1]`);
      }
      return [skill, { taskInventory, passThreshold: skillRaw.passThreshold }];
    })) as AssessmentContract["externalCapstones"][number]["skills"];

    const additionalRaw = item.additionalComponents === undefined
      ? {}
      : object(item.additionalComponents, `${where}.additionalComponents`);
    const additionalComponents = Object.fromEntries(Object.entries(additionalRaw).map(([componentId, component]) => {
      if (!/^[a-z][a-z0-9-]*$/.test(componentId)) {
        throw new Error(`assessment: ${where}.additionalComponents key '${componentId}' must be a lowercase slug`);
      }
      if (ASSESSMENT_SKILLS.includes(componentId as AssessmentSkill)) {
        throw new Error(`assessment: ${where}.additionalComponents '${componentId}' duplicates a universal skill`);
      }
      const componentRaw = object(component, `${where}.additionalComponents.${componentId}`);
      if (!Array.isArray(componentRaw.taskInventory) || componentRaw.taskInventory.length === 0) {
        throw new Error(
          `assessment: ${where}.additionalComponents.${componentId}.taskInventory must be a non-empty array`,
        );
      }
      const taskInventory = componentRaw.taskInventory.map((reference, referenceIndex) =>
        artifactReference(
          reference,
          `${where}.additionalComponents.${componentId}.taskInventory[${referenceIndex}]`,
        )
      );
      if (
        typeof componentRaw.passThreshold !== "number"
        || componentRaw.passThreshold <= 0
        || componentRaw.passThreshold > 1
      ) {
        throw new Error(`assessment: ${where}.additionalComponents.${componentId}.passThreshold must be in (0, 1]`);
      }
      return [componentId, {
        name: nonEmpty(componentRaw.name, `${where}.additionalComponents.${componentId}.name`),
        taskInventory,
        passThreshold: componentRaw.passThreshold,
      }];
    }));

    if (!Array.isArray(item.fullMocks) || item.fullMocks.length < policy.passEvidence.minimumFullMocksPerLevel) {
      throw new Error(
        `assessment: ${where} needs at least ${policy.passEvidence.minimumFullMocksPerLevel} full mocks`,
      );
    }
    const fullMocks = item.fullMocks.map((mock, mockIndex) => {
      const mockRaw = object(mock, `${where}.fullMocks[${mockIndex}]`);
      if (mockRaw.timed !== true) throw new Error(`assessment: ${where} mock must be timed`);
      const humanValidation = mockRaw.humanValidation === undefined
        ? undefined
        : artifactReference(mockRaw.humanValidation, `${where}.fullMocks[${mockIndex}].humanValidation`);
      return {
        id: nonEmpty(mockRaw.id, `${where}.fullMocks[${mockIndex}].id`),
        timed: true,
        rubric: artifactReference(mockRaw.rubric, `${where}.fullMocks[${mockIndex}].rubric`),
        answerKey: artifactReference(mockRaw.answerKey, `${where}.fullMocks[${mockIndex}].answerKey`),
        ...(humanValidation === undefined ? {} : { humanValidation }),
      };
    });

    return {
      id,
      target: {
        name: nonEmpty(target.name, `${where}.target.name`),
        basis: "external" as const,
        source: nonEmpty(target.source, `${where}.target.source`),
        edition: nonEmpty(target.edition, `${where}.target.edition`),
        accessed: nonEmpty(target.accessed, `${where}.target.accessed`),
      },
      requiredAfterLevel: level(item.requiredAfterLevel, `${where}.requiredAfterLevel`),
      cefrRelation: "not-mapped" as const,
      skills: parsedSkills,
      additionalComponents,
      fullMocks,
    };
  });

  return { version: 1, language: expectedLanguage, externalCapstones, levels };
}
