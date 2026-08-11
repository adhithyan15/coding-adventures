/**
 * HL10 section 7.3 -- the info-dump gate.
 *
 * The owner's rule is one sentence: "will not info dump ever". This module is
 * what makes it a measurement instead of an aspiration.
 *
 * WHAT THE FIRST MEASUREMENT ACTUALLY FOUND
 *
 * The prose is not the problem. Scanning all 1,694 lessons for rule-statement
 * shapes -- "X is used for...", "X always takes...", "there are four kinds of
 * X" -- turns up **17 lessons**, seventeen. HL09 said the writing was "as well
 * built as anything commercial" and the corpus agrees with it.
 *
 * The info dump lives in TABLES, and specifically in one shape:
 *
 *   **70 lessons carry a paradigm-shaped table** -- a grid whose first column
 *   is a list of grammatical persons -- and **19 of those are full grids** of
 *   five or more rows. `FR-C05-parler`, `GE-C05-wohnen` and `ES-C17-practice`
 *   each present the complete six-person conjugation at once.
 *
 * That is the exact artifact HL10 section 5.3 forbids. Six new forms, one new
 * concept, no retrieval, and an implicit claim that the learner absorbs them by
 * staring. It is also the single most universal convention in language
 * publishing, which is why it needs a gate rather than a style note: nobody
 * writing one thinks they are doing anything unusual.
 *
 * WHY A PARADIGM TABLE AND NOT JUST A BIG TABLE
 *
 * 470 tables in the corpus have three or more data rows, and most are fine --
 * a vocabulary recap, a three-way regional comparison, a list of labelled
 * facts. Flagging all 470 would bury the 70 that matter and teach authors that
 * the gate cries wolf. The signal is not size; it is a first column that walks
 * a paradigm, because that is a table presenting N grammar cells where the
 * budget allows one.
 *
 * THE PERSON MAP IS A CENSUS, NOT A GRAMMAR
 *
 * `PERSON_LABELS` lists what the corpus's own tables actually put in that first
 * column, per track. It covers the six Latin-script tracks whose lessons show
 * person labels today; a track absent from the map is never flagged, which is
 * honest rather than silently clean. The same rule `continuity.ts` uses for its
 * article map: describe what is written, not what the language could write.
 *
 * REPORT-ONLY
 *
 * Nothing here throws. 70 lessons predate the rule, and per the HL05 precedent
 * a gate that fails on recorded debt teaches authors to route around it. Its
 * real value is as a review aid: a lesson that trips it is not automatically
 * wrong, but it is automatically read by a human before merge.
 */

import { hasOwn, stripControlCharacters as clean } from "./constants.js";
import type { ParsedLesson } from "./parse.js";

export type InfoDumpKind =
  | "rule-statement"
  | "partial-paradigm-table"
  | "full-paradigm-grid";

export interface InfoDumpFinding {
  lessonId: string;
  language: string;
  kind: InfoDumpKind;
  /** The matched sentence, or a description of the table. */
  detail: string;
}

export interface InfoDumpReport {
  findings: InfoDumpFinding[];
  budget: number;
  summary: {
    lessons: number;
    lessonsWithFindings: number;
    ruleStatements: number;
    paradigmTables: number;
    fullParadigmGrids: number;
    overBudget: number;
  };
}

/**
 * Prose shapes that assert a rule rather than showing an instance.
 *
 * Deliberately narrow. A bare "always" or "never" appears constantly in ordinary
 * teaching prose ("you will never need this form yet"), so each pattern requires
 * the verb that turns it into a claim about the language.
 */
const RULE_PATTERNS: { name: string; pattern: RegExp }[] = [
  { name: "is-used-for", pattern: /\b(?:is|are) used (?:for|to|when|with|in)\b/i },
  {
    name: "always-never",
    pattern: /\b(?:always|never) (?:takes|uses|has|is|comes|goes|means|ends|begins|appears)\b/i,
  },
  {
    name: "there-are-n-kinds",
    pattern:
      /\bthere are (?:\d+|two|three|four|five|six|seven|eight|nine|ten|twelve) (?:kinds|types|ways|forms|categories|classes|groups)\b/i,
  },
  { name: "the-rule-is", pattern: /\bthe rule (?:is|for|here)\b/i },
];

/**
 * What each track's own paradigm tables put in their first column.
 *
 * A census of the committed corpus, not a grammar of the language. Tracks whose
 * tables do not use person labels are absent and are never flagged.
 */
export const PERSON_LABELS: Record<string, string[]> = {
  spanish: ["yo", "tú", "tu", "él", "ella", "usted", "nosotros", "vosotros", "ellos", "ellas", "ustedes"],
  italian: ["io", "tu", "lui", "lei", "noi", "voi", "loro"],
  french: ["je", "j'", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles"],
  german: ["ich", "du", "er", "sie", "es", "wir", "ihr"],
  portuguese: ["eu", "tu", "você", "voce", "ele", "ela", "nós", "nos", "vocês", "voces", "eles", "elas"],
  latin: ["ego", "tu", "is", "ea", "nos", "vos", "ei", "eae"],
};

/** How many person rows make a table a "full" grid rather than a partial one. */
export const FULL_GRID_ROWS = 5;

function lessonId(lesson: ParsedLesson): string {
  const raw = (lesson.frontmatter as Record<string, unknown>).id;
  return typeof raw === "string" ? raw : "<unidentified lesson>";
}

/**
 * Lesson body with directive comments removed, so a gate never reads metadata.
 *
 * A monotonic indexOf scan, NOT `replace(/<!--[\s\S]*?-->/g, "")`. The lazy
 * quantifier looks like the safe construct and is not: with `/g`, the engine
 * retries at every `<!--`, and when there is no closing `-->` each start
 * expands one character at a time to EOF before failing. That is O(n squared)
 * in the COUNT of `<!--` tokens, and no `-->` is ever needed to trigger it --
 * 500 KB of repeated `<!--` measured at 13 seconds, a 4 MB lesson at roughly
 * fifteen minutes of pinned CPU. This scan is linear and makes the unterminated
 * case explicit rather than catastrophic.
 */
function teachingProse(lesson: ParsedLesson): string {
  const source = lesson.body;
  let out = "";
  let index = 0;
  for (;;) {
    const start = source.indexOf("<!--", index);
    if (start === -1) break;
    const end = source.indexOf("-->", start + 4);
    // Unterminated: keep the remainder verbatim rather than swallowing the file.
    if (end === -1) break;
    out += source.slice(index, start);
    index = end + 3;
  }
  return out + source.slice(index);
}

function normalizeCell(cell: string): string {
  return cell
    .replace(/\*+/g, "")
    .replace(/`/g, "")
    .trim()
    .toLowerCase();
}

/** Contiguous runs of Markdown table data rows, delimiter rows excluded. */
function tableRuns(prose: string): string[][] {
  const runs: string[][] = [];
  let current: string[] = [];
  for (const line of `${prose}\n`.split("\n")) {
    const trimmed = line.trim();
    const isRow = trimmed.startsWith("|");
    const isDelimiter =
      isRow && trimmed.replace(/\|/g, "").trim().replace(/[-: ]/g, "") === "";
    if (isRow && !isDelimiter) {
      current.push(trimmed);
      continue;
    }
    if (!isRow && current.length > 0) {
      runs.push(current);
      current = [];
    }
  }
  if (current.length > 0) runs.push(current);
  return runs;
}

/** How many of a table's first-column cells name a grammatical person. */
export function personRowCount(rows: string[], language: string): number {
  // hasOwn, not a bare index. `language` is a DIRECTORY NAME -- loader.ts passes
  // `track.name` from readdirSync straight through -- so a track directory called
  // `constructor` or `toString` would resolve to an inherited member, pass an
  // `undefined` check, and throw on `.includes`. constants.ts exports hasOwn for
  // exactly this, and parse.ts already guards its language lookup the same way.
  const labels = hasOwn(PERSON_LABELS, language) ? PERSON_LABELS[language] : undefined;
  if (labels === undefined) return 0;
  let count = 0;
  for (const row of rows) {
    const first = normalizeCell(row.replace(/^\|/, "").split("|")[0] ?? "");
    if (first === "") continue;
    // A cell may be "él / ella / usted" or "yo (I)", so match on the leading token.
    const leading = first.split(/[\s/(,]/)[0] ?? "";
    if (labels.includes(leading) || labels.includes(first)) count += 1;
  }
  return count;
}

/** Findings for one lesson. */
export function lessonInfoDump(lesson: ParsedLesson): InfoDumpFinding[] {
  const id = lessonId(lesson);
  const language = lesson.language;
  const prose = teachingProse(lesson);
  const out: InfoDumpFinding[] = [];

  for (const line of prose.split("\n")) {
    if (line.trim().startsWith("|")) continue; // tables are judged as tables
    for (const { name, pattern } of RULE_PATTERNS) {
      if (pattern.test(line)) {
        out.push({
          lessonId: id,
          language,
          kind: "rule-statement",
          detail: `${name}: ${line.trim().slice(0, 120)}`,
        });
        break; // one finding per line, not one per pattern
      }
    }
  }

  for (const rows of tableRuns(prose)) {
    if (rows.length < 3) continue;
    const persons = personRowCount(rows, language);
    if (persons < 3) continue;
    out.push({
      lessonId: id,
      language,
      kind: persons >= FULL_GRID_ROWS ? "full-paradigm-grid" : "partial-paradigm-table",
      detail: `${rows.length}-row table with ${persons} person rows`,
    });
  }

  return out;
}

/** Measure the corpus. */
export function measureInfoDump(lessons: ParsedLesson[], budget: number): InfoDumpReport {
  const findings = lessons.flatMap((lesson) => lessonInfoDump(lesson));
  const byLesson = new Map<string, number>();
  for (const finding of findings) {
    byLesson.set(finding.lessonId, (byLesson.get(finding.lessonId) ?? 0) + 1);
  }
  let overBudget = 0;
  for (const count of byLesson.values()) if (count > budget) overBudget += 1;

  return {
    findings,
    budget,
    summary: {
      lessons: lessons.length,
      lessonsWithFindings: byLesson.size,
      ruleStatements: findings.filter((f) => f.kind === "rule-statement").length,
      paradigmTables: findings.filter((f) => f.kind !== "rule-statement").length,
      fullParadigmGrids: findings.filter((f) => f.kind === "full-paradigm-grid").length,
      overBudget,
    },
  };
}

/** Human-readable lines for the gap report. */
export function renderInfoDump(report: InfoDumpReport): string[] {
  const s = report.summary;
  const lines = [
    `info dump (report-only): ${s.lessonsWithFindings} of ${s.lessons} lessons -- ` +
      `${s.ruleStatements} rule statements, ${s.paradigmTables} paradigm tables ` +
      `(${s.fullParadigmGrids} full grids)`,
  ];
  const grids = report.findings.filter((f) => f.kind === "full-paradigm-grid").slice(0, 3);
  if (grids.length > 0) {
    lines.push(
      `  complete paradigms presented at once: ${grids.map((g) => `${clean(g.lessonId)} (${clean(g.detail)})`).join(", ")}`,
    );
  }
  return lines;
}
