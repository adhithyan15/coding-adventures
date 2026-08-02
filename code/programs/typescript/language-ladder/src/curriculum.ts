// Browser-safe access to the structured HL04 curriculum. These JSON files are
// the source of truth shared with validators and future book generators.

import registryJson from "../../../../learning/human-languages/core/languages.json";
import spineJson from "../../../../learning/human-languages/core/spine.json";

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

const LANGUAGE_BY_ID = new Map(LANGUAGE_REGISTRY.map((language) => [language.id, language]));
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
