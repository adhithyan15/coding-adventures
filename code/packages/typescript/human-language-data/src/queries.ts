// The typed accessors HL01 promises. Thin, pure lookups over a built Dataset —
// these are what the Engram deck generator and the companion app call.

import type { Concept, Dataset, Realization } from "./types.js";

/** Every concept (canonical + namespaced), id-sorted. */
export function allConcepts(dataset: Dataset): Concept[] {
  return dataset.concepts;
}

/** Every concept a given language realizes. */
export function conceptsByLanguage(dataset: Dataset, language: string): Concept[] {
  return dataset.concepts.filter((c) =>
    c.realizations.some((r) => r.language === language),
  );
}

/**
 * Every language's realization of a concept — the cross-language join that
 * powers "the same word in Spanish, French, German." Empty if unknown.
 */
export function languagesForConcept(dataset: Dataset, concept: string): Realization[] {
  return dataset.concepts.find((c) => c.id === concept)?.realizations ?? [];
}

/** How many `core` concepts each language realizes — the parity dashboard. */
export function coverageByLanguage(
  dataset: Dataset,
): Record<string, { core: number; total: number }> {
  const coreIds = new Set(
    dataset.concepts.filter((c) => c.core).map((c) => c.id),
  );
  const out: Record<string, { core: number; total: number }> = {};
  for (const lang of dataset.languages) {
    const concepts = conceptsByLanguage(dataset, lang);
    out[lang] = {
      total: concepts.length,
      core: concepts.filter((c) => coreIds.has(c.id)).length,
    };
  }
  return out;
}
