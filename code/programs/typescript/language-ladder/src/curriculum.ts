// Browser-safe access to the structured HL04 curriculum. These JSON files are
// the source of truth shared with validators and future book generators.

import registryJson from "../../../../learning/human-languages/core/languages.json";
import {
  curriculumLoaders,
  spine,
} from "virtual:human-language-ledgers";
import {
  mixedCurriculumFrontier as buildMixedCurriculumFrontier,
  type MixedCurriculumFrontier,
} from "@coding-adventures/human-language-data/src/plans.ts";
import type { LanguageCurriculum } from "@coding-adventures/human-language-data/src/types.ts";

// The per-track plans are LAZY, and one virtual chunk per track.
//
// They used to be `{ eager: true }`, which welded all 22 tracks' plans into the
// module graph the browser must download and parse BEFORE first paint. Nothing
// on that path reads them: the first frame is the header plus "Loading the
// lessons needed for this view…" (see `refreshCorpus` in main.ts), and every
// consumer runs after that await. So half a megabyte of authored path,
// extension and spine ledgers was blocking a screen that shows none of it —
// and growing by a tranche a day, which is what finally pushed the eager
// budget in scripts/check-bundle.mjs over 500 kB.
//
// Lazy also means per-track: rolldown gives each build-time shard rollup its own
// chunk (see the `curriculum-<track>` group in vite.config.ts), so adding a
// Telugu chapter re-downloads Telugu's plan and nothing else, instead of
// invalidating one shared half-megabyte blob on every corpus commit.
//
// The virtual index contains one key per TRACK, not one per shard. Adding a
// path segment or spine realization therefore changes a lazy track module but
// never grows the eager lookup table (HL21 §4.4).
const CURRICULUM_LOADERS = curriculumLoaders as Record<
  string,
  () => Promise<LanguageCurriculum>
>;

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
export const SPINE_NODES = (spine as { nodes: SpineNode[] }).nodes;
export const SPINE_CONCEPTS: string[] = SPINE_NODES.flatMap((node) => node.concepts);

/**
 * Which tracks have an authored plan, read from the bounded virtual index.
 *
 * A key is the registry id itself. This lets the language picker and stored
 * selection stay synchronous while the plan bodies remain lazy.
 */
export const MAPPED_LANGUAGE_IDS: string[] = Object.keys(CURRICULUM_LOADERS)
  .sort((left, right) => LANGUAGE_ORDER.indexOf(left) - LANGUAGE_ORDER.indexOf(right));

/**
 * The loaded plans, in registry order. Empty until `loadCurriculumPlans()`
 * resolves; the array IDENTITY never changes, so a module that captured it at
 * import time sees the plans appear rather than holding a stale empty copy.
 */
export const LANGUAGE_CURRICULA: LanguageCurriculum[] = [];

const LANGUAGE_BY_ID = new Map(LANGUAGE_REGISTRY.map((language) => [language.id, language]));
const CURRICULUM_BY_LANGUAGE = new Map<string, LanguageCurriculum>();

let plansPromise: Promise<readonly LanguageCurriculum[]> | null = null;

/**
 * Fetch every track's authored plan, once.
 *
 * Idempotent and concurrency-safe: the promise is memoised, so the corpus
 * refresh, a mode switch and a language-selection change racing each other all
 * await the same fetch rather than pulling the plans three times.
 *
 * SUCCESS is memoised; failure is not. Caching a rejection would make one
 * dropped chunk permanent — every later refresh would re-await the same failed
 * promise and the app would sit in its error frame until the page was reloaded.
 * The eager import could not fail this way, so forgetting a failed attempt is
 * what keeps the lazy version's error behaviour honest: retrying retries.
 */
export function loadCurriculumPlans(): Promise<readonly LanguageCurriculum[]> {
  plansPromise ??= Promise.all(
    Object.values(CURRICULUM_LOADERS).map((load) => load()),
  ).then(
    (loaded) => {
      const ordered = [...loaded].sort(
        (left, right) => LANGUAGE_ORDER.indexOf(left.language) - LANGUAGE_ORDER.indexOf(right.language),
      );
      LANGUAGE_CURRICULA.length = 0;
      LANGUAGE_CURRICULA.push(...ordered);
      CURRICULUM_BY_LANGUAGE.clear();
      for (const curriculum of ordered) CURRICULUM_BY_LANGUAGE.set(curriculum.language, curriculum);
      return LANGUAGE_CURRICULA;
    },
    (error: unknown) => {
      plansPromise = null;
      throw error;
    },
  );
  return plansPromise;
}
const NODE_BY_CONCEPT = new Map(
  SPINE_NODES.flatMap((node) => node.concepts.map((concept) => [concept, node] as const)),
);
const NODE_BY_ID = new Map(SPINE_NODES.map((node) => [node.id, node]));

export function languageDefinition(id: string): LanguageDefinition | undefined {
  return LANGUAGE_BY_ID.get(id);
}

export function languageName(id: string): string {
  return languageDefinition(id)?.name ?? id;
}

export function spineNodeForConcept(concept: string): SpineNode | undefined {
  return NODE_BY_CONCEPT.get(concept);
}

export function spineNodeById(id: string): SpineNode | undefined {
  return NODE_BY_ID.get(id);
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
