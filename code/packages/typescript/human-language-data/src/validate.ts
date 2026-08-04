// The round-trip validator (HL01 §"The round-trip validator").
//
// This is the guard that keeps the lessons and the taxonomy from drifting apart.
// It runs in CI (via a test that loads the real curriculum). ERRORS fail the
// build; WARNINGS and INFO are reported but tolerated, because some things
// (romanization fields, script data) are still being authored track by track.

import {
  CONTENT_TYPES,
  EXEMPT_TYPES,
  hasOwn,
  MAX_ETYMOLOGY_HOOK,
  NAMESPACED_TAG,
} from "./constants.js";
import type { Issue, ScriptData, Taxonomy } from "./types.js";
import type { ParsedLesson } from "./parse.js";

export interface ValidateInput {
  taxonomy: Taxonomy;
  lessons: ParsedLesson[];
  /** Optional per-script character data, keyed by script name. */
  scripts?: Record<string, ScriptData>;
  /** Tracks that declare parity:complete — core coverage is enforced for these. */
  completeTracks?: Set<string>;
}

export function validate(input: ValidateInput): Issue[] {
  const { taxonomy, lessons, scripts = {}, completeTracks = new Set() } = input;
  const issues: Issue[] = [];
  const err = (code: string, message: string, lessonId?: string) =>
    issues.push({ level: "error", code, message, lessonId });
  const warn = (code: string, message: string, lessonId?: string) =>
    issues.push({ level: "warning", code, message, lessonId });
  const info = (code: string, message: string, lessonId?: string) =>
    issues.push({ level: "info", code, message, lessonId });

  // Per-(language) content-concept ledger, to catch duplicate realizations.
  const seen = new Map<string, string>(); // `${lang}|${concept}` -> lessonId

  for (const { realization: r } of lessons) {
    const id = r.lessonId || "(no id)";
    const isContent = CONTENT_TYPES.has(r.type);
    const isExempt = EXEMPT_TYPES.has(r.type);

    // (3) Required fields — every lesson.
    if (r.headword === "") err("missing-headword", `${id}: no headword`, id);
    if (r.gloss === "") err("missing-gloss", `${id}: no gloss`, id);
    if (Number.isNaN(r.chapter)) err("missing-chapter", `${id}: no/invalid chapter`, id);
    if (!isContent && !isExempt) {
      warn("unknown-type", `${id}: unrecognized type '${r.type}'`, id);
    }

    if (!isContent) continue; // only content lessons join the taxonomy

    // (1) The concept tag must resolve.
    if (r.concept === "") {
      err("missing-concept", `${id}: content lesson has no concept_tag`, id);
    } else if (!hasOwn(taxonomy.concepts, r.concept) && !NAMESPACED_TAG.test(r.concept)) {
      err(
        "unresolved-concept",
        `${id}: concept_tag '${r.concept}' is neither canonical nor namespaced`,
        id,
      );
    }

    // (2) One realization per (concept, language).
    if (r.concept !== "") {
      const key = `${r.language}|${r.concept}`;
      const prev = seen.get(key);
      if (prev) {
        err(
          "duplicate-realization",
          `${r.language}: concept '${r.concept}' realized twice (${prev} and ${id})`,
          id,
        );
      } else {
        seen.set(key, id);
      }
    }

    // (6) Field shapes.
    if (r.script !== "latin" && r.romanization === "") {
      warn("missing-romanization", `${id}: non-Latin lesson missing romanization`, id);
    }
    if (r.etymologyHook.length > MAX_ETYMOLOGY_HOOK) {
      warn(
        "long-etymology-hook",
        `${id}: etymology_hook is ${r.etymologyHook.length} chars (max ${MAX_ETYMOLOGY_HOOK})`,
        id,
      );
    }

    // (4) Script glyph references resolve (only where we have the script data).
    // A gap is a warning while a script is still being authored, and hardens to
    // an error once the script file declares itself `complete`.
    const sd = hasOwn(scripts, r.script) ? scripts[r.script] : undefined;
    if (sd) {
      const uncovered = uncoveredGlyphs(r.headword, sd);
      if (uncovered.length > 0) {
        const msg = `${id}: characters not yet in ${r.script}.json: ${uncovered.join(" ")}`;
        if (sd.complete) err("uncovered-glyphs", msg, id);
        else warn("uncovered-glyphs", msg, id);
      }
    }
  }

  // (5) Core-concept coverage — enforced only for parity-complete tracks.
  const languages = [...new Set(lessons.map((l) => l.language))];
  const coreConcepts = Object.entries(taxonomy.concepts)
    .filter(([, c]) => c.core)
    .map(([id]) => id);
  for (const lang of languages) {
    const realized = new Set(
      lessons
        .filter((l) => l.language === lang && CONTENT_TYPES.has(l.realization.type))
        .map((l) => l.realization.concept),
    );
    const missing = coreConcepts.filter((c) => !realized.has(c));
    if (missing.length === 0) continue;
    const msg = `${lang}: missing ${missing.length} core concept(s): ${missing.join(", ")}`;
    if (completeTracks.has(lang)) err("core-coverage", msg);
    else info("core-coverage", msg);
  }

  return issues;
}

/** Characters of a headword not represented anywhere in the script data. */
function uncoveredGlyphs(headword: string, sd: ScriptData): string[] {
  const covered = new Set<string>();
  const add = (s?: string) => {
    if (s) for (const ch of s) covered.add(ch);
  };
  for (const l of sd.letters) {
    add(l.glyph);
    add(l.forms?.isolated);
    add(l.forms?.initial);
    add(l.forms?.medial);
    add(l.forms?.final);
  }
  for (const m of sd.marks ?? []) add(m.mark);
  const skip = /[\s\p{P}‌‍]/u; // spaces, punctuation, ZWNJ/ZWJ
  const out: string[] = [];
  for (const ch of headword) {
    if (skip.test(ch)) continue;
    if (!covered.has(ch) && !out.includes(ch)) out.push(ch);
  }
  return out;
}

export function hasErrors(issues: Issue[]): boolean {
  return issues.some((i) => i.level === "error");
}

/** A one-line-per-level tally, handy for CLI/test output. */
export function summarize(issues: Issue[]): string {
  const n = (lvl: string) => issues.filter((i) => i.level === lvl).length;
  return `${n("error")} error(s), ${n("warning")} warning(s), ${n("info")} info`;
}
