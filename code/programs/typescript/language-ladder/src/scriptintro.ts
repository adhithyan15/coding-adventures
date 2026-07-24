// ---------------------------------------------------------------------------
// scriptintro.ts — introducing a writing system the first time you meet it.
//
// The Learn session walks concepts in book order, each across the language
// chain. The chain crosses writing systems: Latin (Spanish…German), then the
// Arabic abjad, then the Devanagari and Dravidian abugidas. A learner shouldn't
// be dropped into an unfamiliar script with no warning — the book "introduces
// scripts as needed", and so should the app.
//
// This module answers one question, purely: for each non-Latin script the
// curriculum uses, WHICH concept is the first (in book order) to teach it? That
// concept's card is where the "new script" note belongs; every later appearance
// is old news. The note's content is pulled straight from the script data
// (`data/scripts/*.json` → `signature`), never invented — a script we have no
// data for simply gets no note.
//
// Everything here is pure and data-driven, so "is this the first Devanagari
// stop?" is unit-testable without a browser.
// ---------------------------------------------------------------------------

import type { ChainLanguage } from "./sequence.ts";
import type { ScriptData } from "./types.ts";

/**
 * Which writing system each chain language uses. The four Latin-script languages
 * map to "latin" — the base the learner already reads, so it is never
 * "introduced". The Dravidian trio (kannada/telugu/malayalam) map to their own
 * scripts even though we may not yet have `signature` data for them: the mapping
 * is the truth about the language; whether a NOTE shows is gated separately on
 * having data, so we never fabricate one.
 */
export const LANGUAGE_SCRIPT: Record<ChainLanguage, string> = {
  spanish: "latin",
  latin: "latin",
  french: "latin",
  german: "latin",
  arabic: "arabic",
  hindi: "devanagari",
  tamil: "tamil",
  kannada: "kannada",
  telugu: "telugu",
  malayalam: "malayalam",
};

/** The script id a language is written in; "latin" for anything off the chain. */
export function scriptOf(language: string): string {
  return LANGUAGE_SCRIPT[language as ChainLanguage] ?? "latin";
}

/** Index script data by its `script` id for O(1) lookup. */
export function scriptsById(scripts: ScriptData[]): Map<string, ScriptData> {
  const byId = new Map<string, ScriptData>();
  for (const s of scripts) byId.set(s.script, s);
  return byId;
}

/** The minimal lesson shape this module needs (concept tag + language). */
interface ConceptLesson {
  concept: string;
  language: string;
}

/**
 * For each non-Latin script the spine actually teaches AND we have data for, the
 * concept tag at which it is FIRST introduced (earliest in book order).
 *
 * `orderedConcepts` is the book-ordered spine; a lesson whose concept is not on
 * the spine (e.g. a writing lesson, concept "") is ignored. A script maps to the
 * earliest spine position among all lessons written in it. Only scripts present
 * in `available` are returned — the gate that keeps us from ever showing a note
 * for a script with no `signature` data.
 */
export function firstIntroductionByScript(
  orderedConcepts: string[],
  lessons: ConceptLesson[],
  available: Set<string>,
): Map<string, string> {
  const order = new Map<string, number>();
  orderedConcepts.forEach((concept, index) => {
    // First wins: a concept tag appears once in the spine, but guard anyway.
    if (!order.has(concept)) order.set(concept, index);
  });

  const earliest = new Map<string, number>(); // script id → smallest spine index
  for (const lesson of lessons) {
    const index = order.get(lesson.concept);
    if (index === undefined) continue; // concept not on the spine
    const script = scriptOf(lesson.language);
    if (script === "latin" || !available.has(script)) continue; // base / no data
    const prev = earliest.get(script);
    if (prev === undefined || index < prev) earliest.set(script, index);
  }

  const introAt = new Map<string, string>();
  for (const [script, index] of earliest) introAt.set(script, orderedConcepts[index]!);
  return introAt;
}

/** A "new script" note to show on a teaching step: the script + how to spot it. */
export interface ScriptIntro {
  /** Display name, e.g. "Devanagari". */
  name: string;
  /** Writing-system class, e.g. "abugida" / "abjad". */
  system: string;
  /** The recognition cue (the script's `signature`), or "" if none recorded. */
  signature: string;
}

/**
 * The intro to show for one teaching step, or null if none applies.
 *
 * Returns a note only when ALL hold: the step's language is non-Latin, we have
 * data for its script, and `concept` is exactly where that script is first
 * introduced. Everywhere else — a later concept, a Latin stop, a script with no
 * data — returns null, so the note fires once and only once, on real data.
 */
export function scriptIntroFor(
  concept: string,
  language: string,
  introAt: Map<string, string>,
  byId: Map<string, ScriptData>,
): ScriptIntro | null {
  const script = scriptOf(language);
  if (introAt.get(script) !== concept) return null;
  const data = byId.get(script);
  if (!data) return null; // gated already, but keep the type honest
  return { name: data.name, system: data.system, signature: data.signature ?? "" };
}
