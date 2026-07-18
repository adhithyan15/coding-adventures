// Turn lesson frontmatter into realizations, and realizations into a dataset.
// Pure functions only — they take strings/objects in and return data out, with
// no filesystem access, so they're trivially unit-testable. The fs boundary
// lives in loader.ts.

import { splitFrontmatter, type Frontmatter } from "./frontmatter.js";
import { LANGUAGE_SCRIPT, CONTENT_TYPES } from "./constants.js";
import type {
  Concept,
  Dataset,
  Gender,
  Realization,
  Script,
  Taxonomy,
} from "./types.js";

/** A lesson after parsing: its raw frontmatter kept alongside the derived row. */
export interface ParsedLesson {
  language: string;
  script: Script;
  frontmatter: Frontmatter;
  realization: Realization;
}

function arrayify(value: Frontmatter[string] | undefined): string[] {
  if (Array.isArray(value)) return value;
  if (typeof value === "string" && value.trim() !== "") return [value];
  return [];
}

function str(value: Frontmatter[string] | undefined): string {
  return typeof value === "string" ? value : "";
}

/** Gender: prefer an explicit field, else sniff the gloss, else none. */
function deriveGender(fm: Frontmatter, gloss: string): Gender {
  const explicit = str(fm.gender).toLowerCase();
  if (explicit === "masc" || explicit === "masculine") return "masc";
  if (explicit === "fem" || explicit === "feminine") return "fem";
  if (explicit === "neut" || explicit === "neuter") return "neut";
  if (/\bmasc/i.test(gloss)) return "masc";
  if (/\bfem/i.test(gloss)) return "fem";
  if (/\bneut/i.test(gloss)) return "neut";
  return null;
}

/**
 * Parse one lesson file's text into a ParsedLesson. `language` is the track slug
 * (the lessons/ directory's parent). Missing fields are left empty/zero here and
 * flagged later by the validator, so parsing never throws on imperfect input.
 */
export function parseLesson(source: string, language: string): ParsedLesson {
  const { frontmatter } = splitFrontmatter(source);
  const fm = frontmatter ?? {};
  const script = LANGUAGE_SCRIPT[language] ?? "latin";
  const headword = str(fm.headword);
  const gloss = str(fm.gloss);
  const chapterRaw = str(fm.chapter);
  const romanization = str(fm.romanization) || (script === "latin" ? headword : "");

  const realization: Realization = {
    concept: str(fm.concept_tag),
    language,
    lessonId: str(fm.id),
    chapter: chapterRaw === "" ? NaN : Number(chapterRaw),
    type: str(fm.type) || "word",
    headword,
    gloss,
    romanization,
    script,
    gender: deriveGender(fm, gloss),
    sounds: arrayify(fm.sounds),
    roots: arrayify(fm.roots),
    etymologyHook: str(fm.etymology_hook),
  };
  return { language, script, frontmatter: fm, realization };
}

/**
 * Assemble parsed lessons + the taxonomy into the queryable Dataset. Only
 * content lessons (word/phrase) become concepts/realizations — practice and
 * review lessons are session labels, not concepts.
 */
export function buildDataset(taxonomy: Taxonomy, lessons: ParsedLesson[]): Dataset {
  const content = lessons.filter((l) => CONTENT_TYPES.has(l.realization.type));

  const byConcept = new Map<string, Realization[]>();
  const byLanguage: Record<string, Realization[]> = {};
  for (const { realization } of content) {
    if (realization.concept === "") continue;
    (byConcept.get(realization.concept) ?? setGet(byConcept, realization.concept)).push(
      realization,
    );
    (byLanguage[realization.language] ??= []).push(realization);
  }

  const concepts: Concept[] = [];
  for (const [id, realizations] of byConcept) {
    const canon = taxonomy.concepts[id];
    concepts.push({
      id,
      family: canon?.family ?? "(namespaced)",
      gloss: canon?.gloss ?? realizations[0]?.gloss ?? "",
      core: canon?.core ?? false,
      namespaced: canon === undefined,
      realizations,
    });
  }
  concepts.sort((a, b) => a.id.localeCompare(b.id));

  return {
    taxonomy,
    concepts,
    byLanguage,
    languages: Object.keys(byLanguage).sort(),
  };
}

// Small helper: get-or-create a bucket in a Map.
function setGet<K, V>(map: Map<K, V[]>, key: K): V[] {
  const fresh: V[] = [];
  map.set(key, fresh);
  return fresh;
}
