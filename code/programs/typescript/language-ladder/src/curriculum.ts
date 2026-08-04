// Browser-safe access to the structured HL04 curriculum. These JSON files are
// the source of truth shared with validators and future book generators.

import registryJson from "../../../../learning/human-languages/core/languages.json";
import spineJson from "../../../../learning/human-languages/core/spine.json";
import {
  mixedCurriculumFrontier as buildMixedCurriculumFrontier,
  type MixedCurriculumFrontier,
} from "@coding-adventures/human-language-data/src/plans.ts";
import type { LanguageCurriculum } from "@coding-adventures/human-language-data/src/types.ts";

const CURRICULUM_MODULES = import.meta.glob(
  "../../../../learning/human-languages/*/curriculum.json",
  { eager: true, import: "default" },
) as Record<string, LanguageCurriculum>;

export interface LanguageDefinition {
  id: string;
  name: string;
  family: string;
  script: string;
  status: string;
  bridges: string[];
}
export interface SpineNode {
  id: string;
  stage: string;
  canDo: string;
  prerequisites: string[];
  core: boolean;
  concepts: string[];
}

export const LANGUAGE_REGISTRY = registryJson.languages as LanguageDefinition[];
export const LANGUAGE_ORDER: string[] = LANGUAGE_REGISTRY
  .filter((language) => language.status === "active")
  .map((language) => language.id);
export const SPINE_NODES = spineJson.nodes as SpineNode[];
export const SPINE_CONCEPTS: string[] = SPINE_NODES.flatMap((node) => node.concepts);
export const LANGUAGE_CURRICULA: LanguageCurriculum[] = Object.values(CURRICULUM_MODULES)
  .sort((left, right) => LANGUAGE_ORDER.indexOf(left.language) - LANGUAGE_ORDER.indexOf(right.language));

const LANGUAGE_BY_ID = new Map(LANGUAGE_REGISTRY.map((language) => [language.id, language]));
const CURRICULUM_BY_LANGUAGE = new Map(
  LANGUAGE_CURRICULA.map((curriculum) => [curriculum.language, curriculum]),
);
const NODE_BY_CONCEPT = new Map(
  SPINE_NODES.flatMap((node) => node.concepts.map((concept) => [concept, node] as const)),
);

export function languageDefinition(id: string): LanguageDefinition | undefined {
  return LANGUAGE_BY_ID.get(id);
}

export function languageName(id: string): string {
  return languageDefinition(id)?.name ?? id;
}

export function spineNodeForConcept(concept: string): SpineNode | undefined {
  return NODE_BY_CONCEPT.get(concept);
}

export function curriculumForLanguage(language: string): LanguageCurriculum | undefined {
  return CURRICULUM_BY_LANGUAGE.get(language);
}

/** Lesson ids admitted to Learn mode by the selected tracks' authored maps. */
export function mappedLessonIds(languages: readonly string[]): Set<string> {
  const out = new Set<string>();
  for (const language of languages) {
    for (const segment of curriculumForLanguage(language)?.path ?? []) {
      for (const lessonId of segment.lessons) out.add(lessonId);
    }
  }
  return out;
}

/** Browser-safe access to the pure, per-language frontier planner. */
export function mixedCurriculumFrontier(
  selectedLanguages: readonly string[],
  completedByLanguage: ReadonlyMap<string, ReadonlySet<string>>,
): MixedCurriculumFrontier {
  return buildMixedCurriculumFrontier(
    LANGUAGE_CURRICULA,
    selectedLanguages,
    completedByLanguage,
  );
}
