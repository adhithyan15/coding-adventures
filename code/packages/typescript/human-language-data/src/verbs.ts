// Core verb coverage — how much of the shared verb vocabulary each track actually teaches.
//
// WHY THIS NEEDED A TAXONOMY CHANGE BEFORE IT COULD EXIST.
//
// Before the core verb concepts landed, the corpus held 85 verb concept tags and **every
// one of them was namespaced**: `BN-VERB-BOLA`, `FR-VERB-ALLER`, `HI-VERB-BOLNA`. A
// namespaced id is by definition language-local, so "to speak" in Bengali and "to speak"
// in French were unrelated concepts. Zero verb tags were canonical and zero were taught by
// more than one language — the cross-language join that HL01 exists to provide contained
// not a single verb.
//
// That made "cover the most common verbs in every language" unmeasurable rather than
// merely unfinished: there was nothing shared to measure against, and each new track
// invented its own ids. Canonical `VERB-*` concepts fix that, and this module turns the
// result into a burn-down — "Tamil covers 4 of the core 40" — instead of an impression.
//
// A track keeps its namespaced ids for verbs the core does not name; those are real
// vocabulary and are counted as extras, not as noise.

import type { Taxonomy } from "./types.js";
import type { ParsedLesson } from "./parse.js";

/** Coverage for one track. */
export interface TrackVerbCoverage {
  language: string;
  /** Core verb concepts this track teaches. */
  covered: string[];
  /** Core verb concepts it does not yet teach — the authoring list, in taxonomy order. */
  missing: string[];
  /** Verbs it teaches that the core does not name, by namespaced id. */
  extras: string[];
  coveredPercent: number;
}

export interface VerbCoverageReport {
  /** Every canonical verb concept, in taxonomy order. */
  coreVerbs: string[];
  tracks: TrackVerbCoverage[];
  summary: {
    coreVerbCount: number;
    /** Tracks teaching no core verb at all. */
    tracksWithNoCoreVerb: number;
    /** Core verbs no track teaches yet. */
    universallyMissing: string[];
    /** Mean share of the core covered, across tracks. */
    meanCoveredPercent: number;
  };
}

/**
 * The canonical verb concepts, in taxonomy declaration order.
 *
 * Read from the taxonomy rather than hard-coded here: the taxonomy is the authored source,
 * and a second list in code is a second place for it to go stale. Grammatical verb
 * concepts (`VERB-INFINITIVE`, `VERB-PRESENT-HABITUAL`, `VERB-PAST`, …) share the `VERB`
 * family but describe how a verb behaves rather than which verb it is, so they are
 * excluded — a track does not "cover" the past tense the way it covers *to eat*.
 */
const GRAMMATICAL = new Set([
  "VERB-INFINITIVE",
  "VERB-PRESENT-HABITUAL",
  "VERB-NEGATE",
  "VERB-PAST",
  "VERB-FUTURE",
  "VERB-WANT",
]);

export function coreVerbConcepts(taxonomy: Taxonomy): string[] {
  return Object.entries(taxonomy.concepts)
    .filter(([id, concept]) => concept.family === "VERB" && !GRAMMATICAL.has(id))
    .map(([id]) => id);
}

/** Measure core verb coverage across every track that has lessons. */
export function verbCoverage(
  lessons: ParsedLesson[],
  taxonomy: Taxonomy,
): VerbCoverageReport {
  const coreVerbs = coreVerbConcepts(taxonomy);
  const core = new Set(coreVerbs);

  const taughtByTrack = new Map<string, Set<string>>();
  const extrasByTrack = new Map<string, Set<string>>();
  for (const lesson of lessons) {
    const concept = lesson.realization.concept;
    if (!concept) continue;
    const language = lesson.language;
    if (core.has(concept)) {
      let set = taughtByTrack.get(language);
      if (!set) taughtByTrack.set(language, (set = new Set()));
      set.add(concept);
    } else if (/(^|-)VERB-/.test(concept)) {
      // A namespaced verb the core does not name. Real vocabulary, not noise — a track
      // that teaches its own idiomatic verbs is doing the right thing.
      let set = extrasByTrack.get(language);
      if (!set) extrasByTrack.set(language, (set = new Set()));
      set.add(concept);
    }
  }

  const languages = new Set<string>();
  for (const lesson of lessons) languages.add(lesson.language);

  const tracks: TrackVerbCoverage[] = [...languages]
    .sort((a, b) => a.localeCompare(b))
    .map((language) => {
      const taught = taughtByTrack.get(language) ?? new Set<string>();
      const covered = coreVerbs.filter((verb) => taught.has(verb));
      return {
        language,
        covered,
        missing: coreVerbs.filter((verb) => !taught.has(verb)),
        extras: [...(extrasByTrack.get(language) ?? new Set<string>())].sort(),
        coveredPercent:
          coreVerbs.length === 0 ? 0 : Math.round((covered.length / coreVerbs.length) * 100),
      };
    });

  const everTaught = new Set<string>();
  for (const set of taughtByTrack.values()) for (const verb of set) everTaught.add(verb);

  return {
    coreVerbs,
    tracks,
    summary: {
      coreVerbCount: coreVerbs.length,
      tracksWithNoCoreVerb: tracks.filter((track) => track.covered.length === 0).length,
      universallyMissing: coreVerbs.filter((verb) => !everTaught.has(verb)),
      meanCoveredPercent:
        tracks.length === 0
          ? 0
          : Math.round(
              tracks.reduce((sum, track) => sum + track.coveredPercent, 0) / tracks.length,
            ),
    },
  };
}
