import { ASSESSMENT_SKILLS, type AssessmentSkill } from "./assessment.js";
import type { CefrLevel } from "./levels.js";
import type { TaskShapeInventory, TaskMinutes } from "./task-shapes.js";

export interface MockManifest {
  version: 1;
  language: string;
  level: CefrLevel;
  mockId: string;
  sections: Array<{
    skill: AssessmentSkill;
    minutes: number;
    variantId?: string;
    partIds: string[];
  }>;
}

function object(value: unknown, where: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`mock manifest: ${where} must be an object`);
  }
  return value as Record<string, unknown>;
}

function containsMinutes(published: TaskMinutes, minutes: number): boolean {
  if (typeof published === "number") return minutes === published;
  return minutes >= published.minimum && minutes <= published.maximum;
}

/** Prove that one timed paper covers one complete form of every published skill. */
export function parseMockManifest(
  value: unknown,
  language: string,
  level: CefrLevel,
  mockId: string,
  shape: TaskShapeInventory,
): MockManifest {
  if (shape.language !== language || shape.level !== level) {
    throw new Error("mock manifest: task-shape identity does not match the mock");
  }
  const raw = object(value, "root");
  if (raw.version !== 1) throw new Error("mock manifest: version must be 1");
  if (raw.language !== language) throw new Error("mock manifest: language does not match the contract");
  if (raw.level !== level) throw new Error("mock manifest: level does not match the contract");
  if (raw.mockId !== mockId) throw new Error("mock manifest: mockId does not match the contract");
  if (!Array.isArray(raw.sections) || raw.sections.length !== ASSESSMENT_SKILLS.length) {
    throw new Error("mock manifest: sections must cover exactly the four skills");
  }
  const seen = new Set<AssessmentSkill>();
  const sections = raw.sections.map((entry, index) => {
    const section = object(entry, `sections[${index}]`);
    const skill = ASSESSMENT_SKILLS.find((candidate) => candidate === section.skill);
    if (skill === undefined || seen.has(skill)) {
      throw new Error(`mock manifest: sections must cover exactly the four skills once; invalid '${String(section.skill)}'`);
    }
    seen.add(skill);
    const published = shape.sections.find((candidate) => candidate.skill === skill);
    if (published === undefined) throw new Error(`mock manifest: no task-shape section for ${skill}`);
    const minutes = section.minutes;
    if (typeof minutes !== "number" || !Number.isFinite(minutes) || minutes <= 0 || !containsMinutes(published.minutes, minutes)) {
      throw new Error(`mock manifest: ${skill}.minutes must match the published task shape`);
    }
    let expected: readonly string[];
    let variantId: string | undefined;
    if (published.variants.length === 0) {
      if (section.variantId !== undefined) throw new Error(`mock manifest: ${skill} has no variant`);
      expected = published.parts.map((part) => part.id);
    } else {
      variantId = section.variantId as string | undefined;
      const variant = published.variants.find((candidate) => candidate.id === variantId);
      if (variant === undefined) throw new Error(`mock manifest: ${skill} must select a published variant`);
      expected = variant.partIds;
    }
    if (!Array.isArray(section.partIds) || section.partIds.some((part) => typeof part !== "string" || part.trim() === "")) {
      throw new Error(`mock manifest: ${skill}.partIds must list part IDs`);
    }
    const partIds = section.partIds as string[];
    if (partIds.length !== expected.length || new Set(partIds).size !== partIds.length || expected.some((part) => !partIds.includes(part))) {
      throw new Error(`mock manifest: ${skill}.partIds must match exactly the selected task-shape part IDs`);
    }
    return { skill, minutes, ...(variantId === undefined ? {} : { variantId }), partIds: [...partIds] };
  });
  return { version: 1, language, level, mockId, sections };
}

/**
 * Bind each declared task part to a nonempty visible span in the document.
 * A standalone JSON list is otherwise a self-attestation that could describe
 * four skills while the paper contains only a title. Human review still owns
 * pedagogical quality; this check owns structural presence and exact identity.
 */
function validateMockDocument(
  manifest: MockManifest,
  markdown: string,
  role: "paper" | "rubric" | "answer key",
): void {
  const label = `mock ${role}`;
  const expected = new Set(manifest.sections.flatMap((section) => section.partIds));
  const seen = new Set<string>();
  const marker = /<!--\s*(\/?)mock-part:([^\s>]+)\s*-->/g;
  let openId: string | undefined;
  let bodyStart = 0;
  for (const match of markdown.matchAll(marker)) {
    const id = match[2]!;
    if (match[1] === "") {
      if (openId !== undefined || !expected.has(id) || seen.has(id)) {
        throw new Error(`${label}: unexpected, nested, or duplicate part '${id}'`);
      }
      openId = id;
      bodyStart = match.index! + match[0].length;
    } else {
      if (openId !== id) throw new Error(`${label}: unmatched end marker for '${id}'`);
      const body = markdown.slice(bodyStart, match.index);
      const visible = body
        .replace(/<!--[\s\S]*?-->/g, "")
        .replace(/<[^>]*>/g, "")
        .replace(/&(?:[A-Za-z][A-Za-z0-9]+|#\d+|#x[0-9a-f]+);/gi, "");
      if (!/[\p{L}\p{N}]/u.test(visible)) {
        throw new Error(`${label}: part '${id}' has no visible content`);
      }
      seen.add(id);
      openId = undefined;
    }
  }
  if (openId !== undefined) throw new Error(`${label}: part '${openId}' has no end marker`);
  if (seen.size !== expected.size) {
    throw new Error(`${label}: missing task-shape part marker(s): ${[...expected].filter((id) => !seen.has(id)).join(", ")}`);
  }
}

export function validateMockPaper(manifest: MockManifest, markdown: string): void {
  validateMockDocument(manifest, markdown, "paper");
}

export function validateMockRubric(manifest: MockManifest, markdown: string): void {
  validateMockDocument(manifest, markdown, "rubric");
}

export function validateMockAnswerKey(manifest: MockManifest, markdown: string): void {
  validateMockDocument(manifest, markdown, "answer key");
}
