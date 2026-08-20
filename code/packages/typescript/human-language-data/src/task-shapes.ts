import { ASSESSMENT_SKILLS, type AssessmentSkill } from "./assessment.js";
import { CEFR_LEVELS, type CefrLevel } from "./levels.js";

export const TASK_INTERACTIONS = ["individual", "pair", "group"] as const;
export type TaskInteraction = (typeof TASK_INTERACTIONS)[number];

export const TASK_LENGTH_UNITS = ["words", "items", "seconds"] as const;
export type TaskLengthUnit = (typeof TASK_LENGTH_UNITS)[number];

export interface TaskShapeSource {
  id: string;
  title: string;
  url: string;
  published: string;
  accessed: string;
}

export interface TaskLength {
  unit: TaskLengthUnit;
  minimum: number;
  maximum: number;
  approximate: boolean;
}

/** A published duration may be exact or an exact bounded range. */
export interface TaskMinutesRange {
  minimum: number;
  maximum: number;
}
export type TaskMinutes = number | TaskMinutesRange;

export interface TaskShapeVariant {
  id: string;
  partIds: string[];
  note: string;
  sourceIds: string[];
}

export interface TaskPartShape {
  id: string;
  promptModes: string[];
  promptGenres: string[];
  responseModes: string[];
  items: number | null;
  stimulusLength: TaskLength | null;
  responseLength: TaskLength | null;
  interaction: TaskInteraction;
  replayCount: number | null;
  scoring: {
    maxRawPoints: number | null;
    criteria: string[];
  };
  aids: {
    allowed: string[];
    forbidden: string[];
    notPublished: string[];
  };
  notPublished: string[];
  sourceIds: string[];
}

export interface TaskShapeInventory {
  version: 1;
  language: string;
  level: CefrLevel;
  target: {
    name: string;
    basis: "external" | "project-defined";
  };
  sources: TaskShapeSource[];
  administration: {
    writtenMinutes: TaskMinutes;
    speakingMinutes: TaskMinutes;
    speakingGroupMaximum: number | null;
    speakingPreparationMinutes: number | null;
    deliveryModes: string[];
  };
  passRule: {
    maximumPoints: number;
    passPoints: number;
    requiresEverySectionAttempted: boolean;
    independentSkillThresholds: Record<AssessmentSkill, number | null>;
    note: string;
  };
  sections: Array<{
    skill: AssessmentSkill;
    minutes: TaskMinutes;
    parts: TaskPartShape[];
    /** Empty means one implicit form containing every part. */
    variants: TaskShapeVariant[];
  }>;
}

export interface TaskShapePresence {
  language: string;
  level: CefrLevel;
}

export interface TaskShapeBacklogItem {
  id: string;
  language: string;
  level: Exclude<CefrLevel, "pre-A1">;
  goal: string;
}

/**
 * Enumerate the finite research backlog without pretending an absent inventory
 * says anything good about a book. Ordering is level-first and round-robin by
 * track, so one language cannot consume the entire queue while peers stay dark.
 */
export function buildTaskShapeBacklog(
  languages: readonly string[],
  present: readonly TaskShapePresence[],
): TaskShapeBacklogItem[] {
  const have = new Set(present.map((entry) => `${entry.language}/${entry.level}`));
  const levels = CEFR_LEVELS.filter((entry): entry is Exclude<CefrLevel, "pre-A1"> => entry !== "pre-A1");
  return levels.flatMap((level) => languages.flatMap((language) => {
    if (have.has(`${language}/${level}`)) return [];
    return [{
      id: `task-shape/${language}/${level}`,
      language,
      level,
      goal:
        `inventory the sourced ${level} reading, listening, writing and speaking task shapes for ${language}; ` +
        `record unpublished speed and length measurements as unknown rather than inventing them`,
    }];
  }));
}

function object(value: unknown, where: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`task shapes: ${where} must be an object`);
  }
  return value as Record<string, unknown>;
}

function nonEmpty(value: unknown, where: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`task shapes: ${where} must be a non-empty string`);
  }
  return value;
}

function strings(value: unknown, where: string, allowEmpty = false): string[] {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string" || item.trim() === "")) {
    throw new Error(`task shapes: ${where} must be an array of non-empty strings`);
  }
  if (!allowEmpty && value.length === 0) throw new Error(`task shapes: ${where} must not be empty`);
  return [...value];
}

function positive(value: unknown, where: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    throw new Error(`task shapes: ${where} must be a positive finite number`);
  }
  return value;
}

function parseMinutes(value: unknown, where: string): TaskMinutes {
  if (typeof value === "number") return positive(value, where);
  const raw = object(value, where);
  const minimum = positive(raw.minimum, `${where}.minimum`);
  const maximum = positive(raw.maximum, `${where}.maximum`);
  if (minimum > maximum) throw new Error(`task shapes: ${where}.minimum cannot exceed maximum`);
  return { minimum, maximum };
}

function minuteBounds(value: TaskMinutes): TaskMinutesRange {
  return typeof value === "number" ? { minimum: value, maximum: value } : value;
}

function sameMinutes(left: TaskMinutes, right: TaskMinutes): boolean {
  const a = minuteBounds(left);
  const b = minuteBounds(right);
  return a.minimum === b.minimum && a.maximum === b.maximum;
}

function nullableNonNegativeInteger(value: unknown, where: string): number | null {
  if (value === null) return null;
  if (!Number.isInteger(value) || (value as number) < 0) {
    throw new Error(`task shapes: ${where} must be a non-negative integer or null`);
  }
  return value as number;
}

function nullablePositiveInteger(value: unknown, where: string): number | null {
  if (value === null) return null;
  if (!Number.isInteger(value) || (value as number) < 1) {
    throw new Error(`task shapes: ${where} must be a positive integer or null`);
  }
  return value as number;
}

function parseLength(value: unknown, where: string): TaskLength | null {
  if (value === null) return null;
  const raw = object(value, where);
  const unit = TASK_LENGTH_UNITS.find((candidate) => candidate === raw.unit);
  if (!unit) throw new Error(`task shapes: ${where}.unit must be one of ${TASK_LENGTH_UNITS.join(", ")}`);
  const minimum = nullableNonNegativeInteger(raw.minimum, `${where}.minimum`);
  const maximum = nullableNonNegativeInteger(raw.maximum, `${where}.maximum`);
  if (minimum === null || maximum === null) throw new Error(`task shapes: ${where} bounds cannot be null`);
  if (minimum > maximum) throw new Error(`task shapes: ${where}.minimum cannot exceed maximum`);
  if (typeof raw.approximate !== "boolean") throw new Error(`task shapes: ${where}.approximate must be boolean`);
  return { unit, minimum, maximum, approximate: raw.approximate };
}

/**
 * Parse one awarding-body or project-defined task-shape inventory.
 *
 * Nullable measurements are deliberate. An awarding body not publishing audio
 * speed or stimulus length is evidence debt, not permission to invent a tidy
 * number. Every task therefore also carries `notPublished`, so null can never
 * be mistaken for "not applicable" or silently treated as zero.
 */
export function parseTaskShapeInventory(value: unknown, expectedLanguage: string): TaskShapeInventory {
  const raw = object(value, `${expectedLanguage}/task-shapes`);
  if (raw.version !== 1) throw new Error(`task shapes: ${expectedLanguage} version must be 1`);
  if (raw.language !== expectedLanguage) {
    throw new Error(`task shapes: ${expectedLanguage} inventory declares language '${String(raw.language)}'`);
  }
  const level = CEFR_LEVELS.find((candidate) => candidate === raw.level);
  if (!level || level === "pre-A1") throw new Error(`task shapes: ${expectedLanguage}.level must be A1 through C2`);

  const target = object(raw.target, `${expectedLanguage}.target`);
  if (target.basis !== "external" && target.basis !== "project-defined") {
    throw new Error(`task shapes: ${expectedLanguage}.target.basis must be external or project-defined`);
  }

  if (!Array.isArray(raw.sources) || raw.sources.length === 0) {
    throw new Error(`task shapes: ${expectedLanguage}.sources must be a non-empty array`);
  }
  const sourceIds = new Set<string>();
  const sources = raw.sources.map((entry, index) => {
    const source = object(entry, `${expectedLanguage}.sources[${index}]`);
    const id = nonEmpty(source.id, `${expectedLanguage}.sources[${index}].id`);
    if (sourceIds.has(id)) throw new Error(`task shapes: ${expectedLanguage} duplicates source '${id}'`);
    sourceIds.add(id);
    const url = nonEmpty(source.url, `${expectedLanguage}.sources[${index}].url`);
    if (!url.startsWith("https://")) throw new Error(`task shapes: source '${id}' must use an https URL`);
    return {
      id,
      title: nonEmpty(source.title, `${expectedLanguage}.sources[${index}].title`),
      url,
      published: nonEmpty(source.published, `${expectedLanguage}.sources[${index}].published`),
      accessed: nonEmpty(source.accessed, `${expectedLanguage}.sources[${index}].accessed`),
    };
  });

  const administration = object(raw.administration, `${expectedLanguage}.administration`);
  const writtenMinutes = parseMinutes(administration.writtenMinutes, `${expectedLanguage}.administration.writtenMinutes`);
  const speakingMinutes = parseMinutes(administration.speakingMinutes, `${expectedLanguage}.administration.speakingMinutes`);
  const speakingGroupMaximum = nullablePositiveInteger(
    administration.speakingGroupMaximum,
    `${expectedLanguage}.administration.speakingGroupMaximum`,
  );
  const speakingPreparationMinutes = nullableNonNegativeInteger(
    administration.speakingPreparationMinutes,
    `${expectedLanguage}.administration.speakingPreparationMinutes`,
  );

  const passRule = object(raw.passRule, `${expectedLanguage}.passRule`);
  const maximumPoints = positive(passRule.maximumPoints, `${expectedLanguage}.passRule.maximumPoints`);
  const passPoints = positive(passRule.passPoints, `${expectedLanguage}.passRule.passPoints`);
  if (passPoints > maximumPoints) throw new Error(`task shapes: passPoints cannot exceed maximumPoints`);
  if (typeof passRule.requiresEverySectionAttempted !== "boolean") {
    throw new Error(`task shapes: requiresEverySectionAttempted must be boolean`);
  }
  const thresholds = object(passRule.independentSkillThresholds, `${expectedLanguage}.independentSkillThresholds`);
  if (
    Object.keys(thresholds).length !== ASSESSMENT_SKILLS.length ||
    Object.keys(thresholds).some((key) => !ASSESSMENT_SKILLS.includes(key as AssessmentSkill))
  ) {
    throw new Error(`task shapes: independentSkillThresholds must contain exactly ${ASSESSMENT_SKILLS.join(", ")}`);
  }
  const independentSkillThresholds = Object.fromEntries(ASSESSMENT_SKILLS.map((skill) => {
    const threshold = thresholds[skill];
    if (threshold !== null && (typeof threshold !== "number" || threshold <= 0 || threshold > 1)) {
      throw new Error(`task shapes: ${skill} threshold must be in (0, 1] or null`);
    }
    return [skill, threshold];
  })) as Record<AssessmentSkill, number | null>;

  if (!Array.isArray(raw.sections)) throw new Error(`task shapes: ${expectedLanguage}.sections must be an array`);
  const seenSkills = new Set<AssessmentSkill>();
  const seenParts = new Set<string>();
  const seenVariants = new Set<string>();
  const sections = raw.sections.map((entry, sectionIndex) => {
    const section = object(entry, `${expectedLanguage}.sections[${sectionIndex}]`);
    const skill = ASSESSMENT_SKILLS.find((candidate) => candidate === section.skill);
    if (!skill) throw new Error(`task shapes: unknown skill '${String(section.skill)}'`);
    if (seenSkills.has(skill)) throw new Error(`task shapes: duplicate ${skill} section`);
    seenSkills.add(skill);
    if (!Array.isArray(section.parts) || section.parts.length === 0) {
      throw new Error(`task shapes: ${skill}.parts must be a non-empty array`);
    }
    const sectionPartIds = new Set<string>();
    const parts = section.parts.map((entryPart, partIndex) => {
      const part = object(entryPart, `${skill}.parts[${partIndex}]`);
      const id = nonEmpty(part.id, `${skill}.parts[${partIndex}].id`);
      if (seenParts.has(id)) throw new Error(`task shapes: duplicate part id '${id}'`);
      seenParts.add(id);
      sectionPartIds.add(id);
      const interaction = TASK_INTERACTIONS.find((candidate) => candidate === part.interaction);
      if (!interaction) throw new Error(`task shapes: ${id}.interaction must be ${TASK_INTERACTIONS.join(", ")}`);
      const scoring = object(part.scoring, `${id}.scoring`);
      const aids = object(part.aids, `${id}.aids`);
      const partSources = strings(part.sourceIds, `${id}.sourceIds`);
      for (const sourceId of partSources) {
        if (!sourceIds.has(sourceId)) throw new Error(`task shapes: ${id} cites unknown source '${sourceId}'`);
      }
      const notPublished = strings(part.notPublished, `${id}.notPublished`, true);
      const stimulusLength = parseLength(part.stimulusLength, `${id}.stimulusLength`);
      const responseLength = parseLength(part.responseLength, `${id}.responseLength`);
      if ((stimulusLength === null || part.replayCount === null) && notPublished.length === 0) {
        throw new Error(`task shapes: ${id} has null measurements but no notPublished explanation`);
      }
      return {
        id,
        promptModes: strings(part.promptModes, `${id}.promptModes`),
        promptGenres: strings(part.promptGenres, `${id}.promptGenres`),
        responseModes: strings(part.responseModes, `${id}.responseModes`),
        items: nullablePositiveInteger(part.items, `${id}.items`),
        stimulusLength,
        responseLength,
        interaction,
        replayCount: nullableNonNegativeInteger(part.replayCount, `${id}.replayCount`),
        scoring: {
          maxRawPoints: nullableNonNegativeInteger(scoring.maxRawPoints, `${id}.scoring.maxRawPoints`),
          criteria: strings(scoring.criteria, `${id}.scoring.criteria`),
        },
        aids: {
          allowed: strings(aids.allowed, `${id}.aids.allowed`, true),
          forbidden: strings(aids.forbidden, `${id}.aids.forbidden`, true),
          notPublished: strings(aids.notPublished, `${id}.aids.notPublished`, true),
        },
        notPublished,
        sourceIds: partSources,
      };
    });

    const variants = section.variants === undefined
      ? []
      : (() => {
          if (!Array.isArray(section.variants)) {
            throw new Error(`task shapes: ${skill}.variants must be an array`);
          }
          const usedParts = new Set<string>();
          const parsed = section.variants.map((entryVariant, variantIndex) => {
            const variant = object(entryVariant, `${skill}.variants[${variantIndex}]`);
            const id = nonEmpty(variant.id, `${skill}.variants[${variantIndex}].id`);
            if (seenVariants.has(id)) throw new Error(`task shapes: duplicate variant id '${id}'`);
            seenVariants.add(id);
            const partIds = strings(variant.partIds, `${id}.partIds`);
            if (new Set(partIds).size !== partIds.length) {
              throw new Error(`task shapes: ${id}.partIds must not contain duplicates`);
            }
            for (const partId of partIds) {
              if (!sectionPartIds.has(partId)) throw new Error(`task shapes: ${id} cites unknown part '${partId}'`);
              usedParts.add(partId);
            }
            const variantSources = strings(variant.sourceIds, `${id}.sourceIds`);
            for (const sourceId of variantSources) {
              if (!sourceIds.has(sourceId)) throw new Error(`task shapes: ${id} cites unknown source '${sourceId}'`);
            }
            return {
              id,
              partIds,
              note: nonEmpty(variant.note, `${id}.note`),
              sourceIds: variantSources,
            };
          });
          if (parsed.length > 0) {
            const uncovered = [...sectionPartIds].filter((partId) => !usedParts.has(partId));
            if (uncovered.length > 0) {
              throw new Error(`task shapes: ${skill} variant sets omit part(s): ${uncovered.join(", ")}`);
            }
          }
          return parsed;
        })();
    return { skill, minutes: parseMinutes(section.minutes, `${skill}.minutes`), parts, variants };
  });
  for (const skill of ASSESSMENT_SKILLS) {
    if (!seenSkills.has(skill)) throw new Error(`task shapes: missing ${skill} section`);
  }
  const writtenSectionMinutes = sections
    .filter((section) => section.skill !== "speaking")
    .reduce(
      (sum, section) => {
        const bounds = minuteBounds(section.minutes);
        return { minimum: sum.minimum + bounds.minimum, maximum: sum.maximum + bounds.maximum };
      },
      { minimum: 0, maximum: 0 },
    );
  const speakingSectionMinutes = sections.find((section) => section.skill === "speaking")?.minutes;
  if (!sameMinutes(writtenSectionMinutes, writtenMinutes)) {
    throw new Error(`task shapes: written section minutes do not add up to administration.writtenMinutes`);
  }
  if (speakingSectionMinutes === undefined || !sameMinutes(speakingSectionMinutes, speakingMinutes)) {
    throw new Error(`task shapes: speaking minutes do not match administration.speakingMinutes`);
  }

  return {
    version: 1,
    language: expectedLanguage,
    level,
    target: {
      name: nonEmpty(target.name, `${expectedLanguage}.target.name`),
      basis: target.basis,
    },
    sources,
    administration: {
      writtenMinutes,
      speakingMinutes,
      speakingGroupMaximum,
      speakingPreparationMinutes,
      deliveryModes: strings(administration.deliveryModes, `${expectedLanguage}.administration.deliveryModes`),
    },
    passRule: {
      maximumPoints,
      passPoints,
      requiresEverySectionAttempted: passRule.requiresEverySectionAttempted,
      independentSkillThresholds,
      note: nonEmpty(passRule.note, `${expectedLanguage}.passRule.note`),
    },
    sections,
  };
}
