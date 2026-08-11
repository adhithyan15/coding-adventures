/**
 * HL10 section 7.5 -- the metalanguage ramp.
 *
 * THE HIDDEN PREREQUISITE
 *
 * Every language textbook assumes the reader already knows grammar
 * *vocabulary*. "The first-person singular present indicative of a regular -ar
 * verb" spends six technical terms on one form, and a beginner who never
 * studied grammar understands none of them. The book is gentle about Spanish
 * and brutal about English, and nobody notices because the author has known
 * those words since school.
 *
 * So the metalanguage is a taught ramp in its own right: one new term per
 * lesson, each introduced only AFTER the learner can already do the thing it
 * names. `verb` arrives once *soy*, *estoy* and one present form are in use;
 * `mood` waits for block D of the subjunctive arc, twenty-four lessons in.
 *
 * WHY THIS AND NOT A PROSE LINT
 *
 * HL10 section 7.4 also asks for a banned-word lint -- no *simply*, *just*,
 * *obviously*. That was measured first, and it is nearly a no-op: a naive
 * denylist flags 535 of 1,694 lessons, narrowing to genuinely dismissive senses
 * drops it to 23, and reading those, most are still innocent ("*Desde luego*
 * means 'of course'" is teaching the phrase, not talking down to the reader).
 * The corpus's prose is already kind.
 *
 * The metalanguage is where the real gap is. Measured before authoring:
 * `infinitive` appears in 87 lessons, `participle` in 67, `dative` in 53,
 * `accusative` in 19, `declension` in 14, `nominative` in 12 -- and nothing
 * anywhere introduces any of them.
 *
 * `plainAlternative` IS THE POINT
 *
 * A rule that only forbids is a rule authors route around. Every term carries
 * the phrase a lesson must use *instead* until the term is earned -- "a doing
 * word" for verb, "the plain form, the one in a dictionary" for infinitive,
 * "whether you are asserting or wanting" for mood. The gate can therefore tell
 * an author what to write, not merely what not to.
 *
 * REPORT-ONLY, and matching only whole words in teaching prose. Directive
 * comments, frontmatter and table rows are excluded, because a gate that fires
 * on metadata teaches authors to distrust it.
 */

import { hasOwn, stripControlCharacters as clean } from "./constants.js";
import type { ParsedLesson } from "./parse.js";
import type { MetalanguageInventory, MetalanguageTerm } from "./types.js";

export interface MetalanguageUse {
  lessonId: string;
  language: string;
  termId: string;
  term: string;
  /** What the lesson should say instead, until the term is introduced. */
  plainAlternative: string;
  /** True when no earlier lesson declared it. */
  beforeIntroduction: boolean;
  /** False for ordinary English a reader already owns. */
  technical: boolean;
}

export interface MetalanguageReport {
  uses: MetalanguageUse[];
  summary: {
    terms: number;
    termsUsed: number;
    lessonsUsingTerms: number;
    usesBeforeIntroduction: number;
    /**
     * The actionable subset. The total says how pervasive the assumption is;
     * this says what to fix first, and stops `word` in 1,555 lessons from
     * burying `dative` in 53.
     */
    technicalUsesBeforeIntroduction: number;
    technicalLessons: number;
    /** term -> how many lessons use it before it is introduced. */
    worstTerms: { term: string; lessons: number }[];
  };
}

function frontmatterString(lesson: ParsedLesson, key: string): string {
  const raw = (lesson.frontmatter as Record<string, unknown>)[key];
  return typeof raw === "string" ? raw : "";
}

function lessonId(lesson: ParsedLesson): string {
  return frontmatterString(lesson, "id") || "<unidentified lesson>";
}

function sequenceOf(lesson: ParsedLesson): number {
  const raw = (lesson.frontmatter as Record<string, unknown>).sequence;
  if (raw === undefined || raw === null || String(raw).trim() === "") {
    return Number.POSITIVE_INFINITY;
  }
  const value = typeof raw === "number" ? raw : Number(raw);
  return Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

/**
 * Teaching prose only.
 *
 * A monotonic scan rather than `replace(/<!--[\s\S]*?-->/g, "")`, which is
 * quadratic in the count of `<!--` tokens when any is unterminated -- see the
 * same note in info-dump.ts, where 500 KB of comment starts took 13 seconds.
 * Table rows go too: a person-labelled grid is the info-dump gate's business,
 * and a header cell reading "person" is not an author explaining the word.
 */
function teachingProse(lesson: ParsedLesson): string {
  const source = lesson.body;
  let out = "";
  let index = 0;
  for (;;) {
    const start = source.indexOf("<!--", index);
    if (start === -1) break;
    const end = source.indexOf("-->", start + 4);
    if (end === -1) break;
    out += source.slice(index, start);
    index = end + 3;
  }
  out += source.slice(index);
  return out
    .split("\n")
    .filter((line) => !line.trim().startsWith("|"))
    .join("\n");
}

/** Metalanguage terms a lesson declares it introduces. */
function declaredIntroductions(lesson: ParsedLesson): string[] {
  const raw = (lesson.frontmatter as Record<string, unknown>).introduces_metalanguage;
  if (Array.isArray(raw)) {
    return raw.filter((v): v is string => typeof v === "string" && v !== "");
  }
  return typeof raw === "string" && raw.trim() !== "" ? [raw.trim()] : [];
}

/**
 * A whole-word, case-insensitive matcher for one term.
 *
 * Built per term from a literal, so there is no author-supplied pattern and no
 * ReDoS surface. Multi-word terms ("direct object") allow any run of whitespace
 * so a line break inside the phrase still matches. The optional trailing `s`
 * catches the plural without matching an unrelated longer word, because `\b`
 * closes it.
 */
function termPattern(term: string): RegExp {
  const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s+");
  return new RegExp(`\\b${escaped}s?\\b`, "i");
}

/**
 * Usable terms only.
 *
 * `termsUsedIn` and `measureMetalanguage` are exported from index.ts, so a
 * downstream consumer can hand this module an inventory it did not generate. A
 * null element or a non-string `term` would throw an uncaught TypeError; the
 * security review confirmed a hostile STRING cannot inject a pattern, which
 * leaves only crash-on-malformed-data, and that is cheaper to guard than to
 * diagnose.
 */
function usableTerms(inventory: MetalanguageInventory): MetalanguageTerm[] {
  if (!Array.isArray(inventory.terms)) return [];
  return inventory.terms.filter(
    (term): term is MetalanguageTerm =>
      term !== null &&
      typeof term === "object" &&
      typeof term.term === "string" &&
      term.term.trim() !== "" &&
      typeof term.id === "string",
  );
}

/** Which of the declared terms appear in a lesson's teaching prose. */
export function termsUsedIn(lesson: ParsedLesson, inventory: MetalanguageInventory): MetalanguageTerm[] {
  const prose = teachingProse(lesson);
  return usableTerms(inventory).filter((term) => termPattern(term.term).test(prose));
}

/** Measure metalanguage use against the declared introductions. */
export function measureMetalanguage(
  lessons: ParsedLesson[],
  inventory: MetalanguageInventory,
): MetalanguageReport {
  const terms = usableTerms(inventory);
  const byId: Record<string, MetalanguageTerm> = Object.create(null);
  for (const term of terms) byId[term.id] = term;

  const ordered = [...lessons].sort((a, b) => {
    const left = sequenceOf(a);
    const right = sequenceOf(b);
    if (left < right) return -1;
    if (left > right) return 1;
    return lessonId(a).localeCompare(lessonId(b));
  });

  const introducedAt = new Map<string, number>();
  const uses: MetalanguageUse[] = [];
  const lessonsWithUses = new Set<string>();

  ordered.forEach((lesson, index) => {
    // Introductions land first: a lesson may introduce a term and then use it.
    for (const id of declaredIntroductions(lesson)) {
      if (hasOwn(byId, id) && !introducedAt.has(id)) introducedAt.set(id, index);
    }
    const id = lessonId(lesson);
    for (const term of termsUsedIn(lesson, inventory)) {
      const at = introducedAt.get(term.id);
      lessonsWithUses.add(id);
      uses.push({
        lessonId: id,
        language: lesson.language,
        termId: term.id,
        term: term.term,
        plainAlternative: term.plainAlternative,
        beforeIntroduction: at === undefined || at > index,
        technical: term.technical !== false,
      });
    }
  });

  const early = uses.filter((u) => u.beforeIntroduction);
  // Ranked over TECHNICAL terms only. Including `word` (1,555 lessons) and
  // `sound` (641) would make the list identical for every corpus and useless
  // for every author.
  const earlyTechnical = early.filter((u) => u.technical);
  const perTerm = new Map<string, Set<string>>();
  for (const use of earlyTechnical) {
    const bucket = perTerm.get(use.term) ?? new Set<string>();
    bucket.add(use.lessonId);
    perTerm.set(use.term, bucket);
  }
  const worstTerms = [...perTerm.entries()]
    .map(([term, lessonIds]) => ({ term, lessons: lessonIds.size }))
    .sort((a, b) => b.lessons - a.lessons || a.term.localeCompare(b.term))
    .slice(0, 6);

  return {
    uses,
    summary: {
      terms: terms.length,
      termsUsed: new Set(uses.map((u) => u.termId)).size,
      lessonsUsingTerms: lessonsWithUses.size,
      usesBeforeIntroduction: early.length,
      technicalUsesBeforeIntroduction: earlyTechnical.length,
      technicalLessons: new Set(earlyTechnical.map((u) => u.lessonId)).size,
      worstTerms,
    },
  };
}

/** Human-readable lines for the gap report. */
export function renderMetalanguage(report: MetalanguageReport): string[] {
  const s = report.summary;
  const lines = [
    `metalanguage: ${s.termsUsed} of ${s.terms} grammar terms appear in ${s.lessonsUsingTerms} lessons; ` +
      `${s.usesBeforeIntroduction} uses come before introduction, of which ` +
      `${s.technicalUsesBeforeIntroduction} are technical terms across ${s.technicalLessons} lessons`,
  ];
  if (s.worstTerms.length > 0) {
    lines.push(
      `  most-used-before-introduction: ${s.worstTerms
        .map((t) => `${clean(t.term)} (${t.lessons} lessons)`)
        .join(", ")}`,
    );
  }
  return lines;
}
