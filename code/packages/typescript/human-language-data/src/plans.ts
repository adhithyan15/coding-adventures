// Pure queries over per-language shared-spine realization maps.

import type {
  CurriculumExtensionNode,
  CurriculumPathSegment,
  LanguageCurriculum,
} from "./types.js";

export type ExtensionRelation = "before" | "inline" | "after";

export interface AttachedExtension {
  relation: ExtensionRelation;
  extension: CurriculumExtensionNode;
}

/** The next prerequisite-safe local lesson for one language. */
export interface CurriculumFrontierStep {
  language: string;
  segmentId: string;
  spineNode: string;
  lessonId: string;
  extensions: AttachedExtension[];
}

/** Flatten a track path without losing its authored segment order. */
export function orderedCurriculumLessonIds(curriculum: LanguageCurriculum): string[] {
  return curriculum.path.flatMap((segment) => segment.lessons);
}

/** Every extension attached to a segment, preserving before/inline/after order. */
export function extensionsForSegment(
  curriculum: LanguageCurriculum,
  segment: CurriculumPathSegment,
): AttachedExtension[] {
  const byId = new Map(curriculum.extensions.map((extension) => [extension.id, extension]));
  const out: AttachedExtension[] = [];
  for (const relation of ["before", "inline", "after"] as const) {
    for (const id of segment[relation]) {
      const extension = byId.get(id);
      if (extension) out.push({ relation, extension });
    }
  }
  return out;
}

/**
 * Return the first unfinished lesson in the authored local path.
 *
 * Validation proves that every declared prerequisite occurs earlier in this
 * path. Choosing the first unfinished item therefore cannot jump over a local
 * grammar, vocabulary, register, or script dependency.
 */
export function nextCurriculumLesson(
  curriculum: LanguageCurriculum,
  completed: ReadonlySet<string>,
): CurriculumFrontierStep | undefined {
  for (const segment of curriculum.path) {
    for (const lessonId of segment.lessons) {
      if (completed.has(lessonId)) continue;
      return {
        language: curriculum.language,
        segmentId: segment.id,
        spineNode: segment.spine_node,
        lessonId,
        extensions: extensionsForSegment(curriculum, segment).filter(({ extension }) =>
          extension.lessons.includes(lessonId),
        ),
      };
    }
  }
  return undefined;
}

export interface MixedCurriculumFrontier {
  /** One safe next step per selected language, in caller-selected order. */
  steps: CurriculumFrontierStep[];
  /** Steps currently sharing a spine ability and therefore safe to compare. */
  bySpineNode: Map<string, CurriculumFrontierStep[]>;
}

/**
 * Compute independent per-language frontiers, then group only the abilities
 * that are ready on both sides. Progress never leaks from one language into
 * another language's prerequisite state.
 */
export function mixedCurriculumFrontier(
  curricula: readonly LanguageCurriculum[],
  selectedLanguages: readonly string[],
  completedByLanguage: ReadonlyMap<string, ReadonlySet<string>>,
): MixedCurriculumFrontier {
  const byLanguage = new Map(curricula.map((curriculum) => [curriculum.language, curriculum]));
  const steps: CurriculumFrontierStep[] = [];
  for (const language of selectedLanguages) {
    const curriculum = byLanguage.get(language);
    if (!curriculum) continue;
    const step = nextCurriculumLesson(
      curriculum,
      completedByLanguage.get(language) ?? new Set<string>(),
    );
    if (step) steps.push(step);
  }
  const bySpineNode = new Map<string, CurriculumFrontierStep[]>();
  for (const step of steps) {
    const group = bySpineNode.get(step.spineNode) ?? [];
    group.push(step);
    bySpineNode.set(step.spineNode, group);
  }
  return { steps, bySpineNode };
}
