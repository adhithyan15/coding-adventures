import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { lessonsUpToLevel } from "./levels.js";
import { defaultCurriculumRoot, loadEverything } from "./loader.js";
import { LINE_TERMINATORS, reportableFilename } from "./constants.js";

export const SPANISH_A1_MOCK_AUDIT = "spanish/mocks/a1/book-bounded-audit.json";

/**
 * The audit is per (track, level), and the level is the only thing that varies.
 *
 * It was written for A1 alone and hard-coded that everywhere, which was correct
 * while A1 was the only rung with mocks. pre-A1 now has a pair too, and the
 * question it answers -- "is every objective item answerable from what the book
 * has taught BY THIS RUNG?" -- is the same question with a different cut-off.
 *
 * `lessonsUpToLevel` already takes the level, so nothing about the measurement
 * needed to change; only the three places that spelled `a1` out loud did.
 */
export type MockAuditLevel = "pre-A1" | "A1" | "A2";

// Frozen, because `Readonly<>` is erased at runtime and this table is the
// single source of truth for every path this module builds.
const AUDIT_DIR: Readonly<Record<MockAuditLevel, string>> = Object.freeze({
  "pre-A1": "spanish/mocks/pre-a1",
  A1: "spanish/mocks/a1",
  A2: "spanish/mocks/a2",
});

export function spanishMockAuditPath(level: MockAuditLevel): string {
  // Through the guard, not a bare lookup: `runSpanishA1MockAudit` feeds this
  // straight into a `writeFileSync`, so an unchecked level here is a traversal
  // WRITE rather than a failed read. Unreachable from either CLI, both of which
  // validate `--level` first -- but `package.json` declares no `exports` map,
  // so a consumer can deep-import this module, which is the same
  // package-boundary argument that made the reporter's traversal real.
  return `${spanishMockDir(level)}/book-bounded-audit.json`;
}

/**
 * The directory a level's papers live in, by LOOKUP rather than interpolation.
 *
 * `AUDIT_DIR` is traversal-proof by construction: an unrecognised level yields
 * `undefined` and throws here, where a template string would have built a path
 * out of whatever it was handed. That is not hypothetical -- `mock-stem-coverage`
 * first wrote `spanish/mocks/${level.toLowerCase()}`, and because it is exported
 * from `index.ts`, a JavaScript caller passing `"../../../../../../etc"` read
 * outside the curriculum root entirely. TypeScript's union type does not
 * survive the package boundary; this check does.
 */
export function spanishMockDir(level: MockAuditLevel): string {
  const dir = Object.hasOwn(AUDIT_DIR, level) ? AUDIT_DIR[level] : undefined;
  if (dir === undefined) {
    // `reportableFilename`, not a bare strip: `stripControlCharacters` keeps
    // `\n` by design, and this is a ONE-LINE message, so a level carrying a
    // newline would forge a second log line. constants.ts documents the pair.
    throw new Error(`unknown mock level ${reportableFilename(String(level))}`);
  }
  return dir;
}

const citationFormCredits = [
  "llevar", "andar", "dar", "llover", "amigo", "sol",
  "vivir", "llamar", "llamarse", "año", "mes",
];
const numberWordCredits = [
  "cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete",
  "ocho", "nueve", "diez", "once", "doce", "trece", "catorce", "quince",
  "dieciséis", "diecisiete", "dieciocho", "diecinueve", "veinte", "veintiuno",
  "veintidós", "veintitrés", "veinticuatro", "veinticinco", "veintiséis",
  "veintisiete", "veintiocho", "veintinueve", "treinta", "cuarenta",
  "cincuenta", "sesenta", "setenta", "ochenta", "noventa", "cien",
];

const clean = (value: string): string =>
  value.toLowerCase().normalize("NFC").replace(/^\*+|\*+$/g, "").trim();

export type AnswerKeyRow = { paper: number; item: number; requires: string[] };

/**
 * What a parse of an answer key found, INCLUDING what it could not use.
 *
 * `rows` alone was the wrong return type, and the reason is the whole subject
 * of this module. Every way this parse can go wrong is silent: a line
 * terminator the split misses, a heading whose numbering changes, a table that
 * grew a trailing space. Each one removes a row, and a removed row is an item
 * the gate never scores -- which reads downstream as an item that is fine.
 * Anything the parse REJECTED has to come back with it, or the caller cannot
 * tell "nothing was wrong" from "I could not see it".
 */
export interface AnswerKeyParse {
  /** Rows under Prueba 1 or 2 that carry at least one requirement. */
  readonly rows: readonly AnswerKeyRow[];
  /** Items whose requirement column parsed to nothing. Never scored -- see below. */
  readonly unscored: readonly { paper: number; item: number }[];
  /** Lines that open like a numbered table row but that the row pattern rejected. */
  readonly malformed: readonly string[];
}

/**
 * The answer key's scored rows, from its TEXT.
 *
 * Split from `parseAnswerKey` so the parse can be tested the way
 * `parseMockPaper` is -- on strings, without a whole synthetic curriculum
 * behind it. All three of this gate's silent drops (a line terminator the split
 * misses, an empty last column, a row the pattern rejects) were found by
 * READING, in two rounds of security review, because the only way into the
 * parser was a 20-minute run over the real corpus.
 *
 * It returns rather than throws: "" is a legitimate input to a parser and is
 * not a legitimate answer key, and that judgement belongs to `parseAnswerKey`,
 * which knows which file it opened.
 */
export function parseAnswerKeyRows(text: string): AnswerKeyParse {
  const rows: AnswerKeyRow[] = [];
  const unscored: { paper: number; item: number }[] = [];
  const malformed: string[] = [];
  let paper = 0;
  // `LINE_TERMINATORS`, not `/\r?\n/`, and this is the FAIL-OPEN copy of that
  // omission. `.` cannot match a lone CR or U+2028, so a key using either
  // collapsed to one line, matched no rows, and this gate reported
  // `objectiveFailed: 0` with `reading: 0` and `listening: 0` -- a clean bill
  // of health for items it never read, which `--write` would then persist.
  // `mock-stem-coverage.ts` had the same omission in the direction that only
  // adds noise; the hardening went to the harmless copy first because the two
  // were written out separately instead of shared.
  for (const line of text.split(LINE_TERMINATORS)) {
    const heading = /^## Prueba (\d)/.exec(line);
    if (heading) paper = Number(heading[1]);
    // `([^|]*)` rather than `\s*([^|]+)\s*`. The old shape measured CUBIC --
    // 1.1s at a 2000-character line, 8.7s at 4000 -- which is strictly worse
    // than the two quadratic patterns CodeQL flagged in the sibling module, and
    // it is reachable by the same argument: `buildSpanishA1MockAudit` is
    // exported and takes a caller-supplied `root`. Fixing only the new copy and
    // leaving this one was the wrong call. `clean` already trims, so every row
    // in the corpus parses identically.
    const row = /^\|\s*(\d+)\s*\|.*\|([^|]*)\|$/.exec(line);
    if (row === null) {
      // A line that OPENS like a numbered data row and then fails the full
      // pattern is the silent drop, caught. One trailing space after the
      // closing pipe is enough to do it, and the result -- an item nobody
      // scored -- is indistinguishable downstream from an item that passed.
      // Deliberately not restricted to Prueba 1 and 2: a malformed row
      // anywhere means the table shape moved, and the paper it landed under is
      // itself decided by a heading pattern that may be what broke.
      if (/^\|\s*\d+\s*\|/.test(line)) malformed.push(line);
      continue;
    }
    if (paper !== 1 && paper !== 2) continue;
    const requires = row[2].split(",").map(clean).filter(Boolean);
    if (requires.length === 0) {
      // NOT A PASS, AND NOT A FAILURE EITHER. This row is unusable, and both
      // ways of scoring it are wrong in a way that hides something:
      //
      //   requires: [""]  -- what `+` -> `*` produced before the filter. `taught`
      //                      never contains "", so the item FAILS on a
      //                      requirement nobody wrote, inflating
      //                      `objectiveFailed` and putting "" in
      //                      `missingObjectiveLexemes`.
      //   requires: []    -- what the filter alone produced. `[].every(...)` is
      //                      `true`, so the item PASSES unconditionally and
      //                      counts toward `reading`/`listening`. Round two of
      //                      security review caught this: the fix for the
      //                      phantom failure had landed on a phantom pass,
      //                      which is the FAIL-OPEN direction.
      //
      // So it is neither. It is reported, and `parseAnswerKey` refuses the file.
      unscored.push({ paper, item: Number(row[1]) });
      continue;
    }
    rows.push({ paper, item: Number(row[1]), requires });
  }
  return { rows, unscored, malformed };
}

function parseAnswerKey(path: string): readonly AnswerKeyRow[] {
  const { rows, unscored, malformed } = parseAnswerKeyRows(readFileSync(path, "utf8"));
  const name = reportableFilename(path);
  // A GATE THAT READ NOTHING MUST NOT REPORT SUCCESS -- and "nothing" has four
  // shapes, not one.
  //
  // The first version of this guard checked only `rows.length === 0`, which is
  // reached only when EVERY row is lost. One trailing space after a closing
  // pipe drops a single row and sails past it, and a single dropped row is an
  // item this gate never scores -- which is what a passing item looks like from
  // the outside. An all-or-nothing check on an all-or-nothing failure mode is
  // the one case that was never the risk.
  //
  // Each `throw` below is unconditional rather than gated on having seen a
  // `## Prueba` heading: a heading-gated check goes quiet in exactly the case
  // where the heading pattern is what stopped matching.
  if (malformed.length > 0) {
    throw new Error(`${name}: ${malformed.length} table row(s) the row pattern rejected`);
  }
  if (unscored.length > 0) {
    const items = unscored.map(({ paper, item }) => `${paper}.${item}`).join(", ");
    throw new Error(`${name}: item(s) ${items} state no requirements`);
  }
  if (rows.length === 0) {
    throw new Error(`${name}: parsed no answer-key rows`);
  }
  // The comment on the previous guard asserted this -- "both keys the audit
  // opens have Prueba 1 and Prueba 2 sections full of rows" -- without the code
  // checking it. A claim in a comment that the code does not enforce is how
  // this module's header got three drafts wrong; here it is as a check.
  for (const paper of [1, 2]) {
    if (!rows.some((row) => row.paper === paper)) {
      throw new Error(`${name}: parsed no Prueba ${paper} rows`);
    }
  }
  return rows;
}

/**
 * The taught set this gate measures against, and the ONLY copy of it.
 *
 * Exported rather than left local because a second consumer now exists --
 * `mock-stem-coverage.ts`, which reports the words a paper's STEMS and OPTIONS
 * use -- and a reporter that rebuilds the taught set from its own reading of
 * the corpus is measuring a different thing from the gate it reports on.
 * `root-slug-splits.ts` records the same lesson under "one notion of a slug":
 * a guard reading different bytes from the thing it guards is not a guard.
 *
 * HEADWORDS ONLY, deliberately, and that is a live argument rather than an
 * oversight -- see HL-C422. A lexeme taught inside another lesson's body reads
 * as untaught here, which is why `glossed-not-taught.ts` exists to queue those
 * for review instead of silently crediting them.
 */
export function spanishTaughtForms(
  root = defaultCurriculumRoot(),
  level: MockAuditLevel = "A1",
): { taught: Set<string>; lessonCount: number } {
  const everything = loadEverything(root);
  const lessons = lessonsUpToLevel(
    everything.lessons.filter((lesson) => lesson.language === "spanish"),
    everything.curricula.filter((path) => path.language === "spanish"),
    everything.spine,
    level,
  );
  const taught = new Set<string>();
  for (const lesson of lessons) {
    const headword = clean(lesson.realization.headword ?? "");
    for (const rawPart of headword.split(/[/,] |\s+y\s+|\s*[—–]\s*/)) {
      const part = clean(rawPart);
      if (!part) continue;
      taught.add(part);
      taught.add(part.replace(/^(?:el|la|los|las|un|una)\s+/, ""));
      for (const token of part.split(/\s+/)) taught.add(token);
    }
  }
  for (const credit of citationFormCredits) taught.add(credit);
  for (const credit of numberWordCredits) taught.add(credit);
  for (let number = 0; number <= 100; number += 1) taught.add(String(number));
  return { taught, lessonCount: lessons.length };
}

export function buildSpanishA1MockAudit(
  root = defaultCurriculumRoot(),
  level: MockAuditLevel = "A1",
) {
  const { taught, lessonCount } = spanishTaughtForms(root, level);

  const answerKeys = [1, 2].map((mock) => ({
    mock,
    rows: parseAnswerKey(resolve(root, `${spanishMockDir(level)}/mock-${mock}-answer-key.md`)),
  }));
  const mocks = answerKeys.map(({ mock, rows }) => {
    const failed = rows
      .map((row) => ({
        paper: row.paper,
        item: row.item,
        missing: row.requires.filter((entry) => !entry.startsWith("!") && !taught.has(entry)),
      }))
      .filter((row) => row.missing.length > 0);
    const passes = (row: AnswerKeyRow) => row.requires.every((entry) => entry.startsWith("!") || taught.has(entry));
    return {
      mock,
      reading: rows.filter((row) => row.paper === 1 && passes(row)).length,
      listening: rows.filter((row) => row.paper === 2 && passes(row)).length,
      objectiveFailed: failed.length,
      failed,
    };
  });
  const failedRows = answerKeys.flatMap(({ rows }) => rows).filter((row) => !row.requires.every(
    (entry) => entry.startsWith("!") || taught.has(entry),
  ));
  const missingFrequency = new Map<string, number>();
  for (const row of failedRows) {
    for (const entry of row.requires.filter((value) => !value.startsWith("!") && !taught.has(value))) {
      missingFrequency.set(entry, (missingFrequency.get(entry) ?? 0) + 1);
    }
  }
  return {
    version: 1,
    language: "spanish",
    level,
    policy: {
      description: `Credit explicit ${level} headwords, their article-free and token forms, documented citation aliases, and Spanish numerals 0-100.`,
      citationFormCredits,
      numberWordCredits,
      numericCredits: "0-100",
    },
    lessonCount,
    taughtForms: taught.size,
    objectiveFailed: mocks.reduce((sum, mock) => sum + mock.objectiveFailed, 0),
    mocks,
    missingObjectiveLexemes: [...missingFrequency.keys()].sort(),
    missingFrequency: [...missingFrequency]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([lexeme, count]) => ({ lexeme, count })),
  };
}

export function serializeSpanishA1MockAudit(
  root = defaultCurriculumRoot(),
  level: MockAuditLevel = "A1",
): string {
  return `${JSON.stringify(buildSpanishA1MockAudit(root, level), null, 2)}\n`;
}

export function runSpanishA1MockAudit(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.find((arg) => arg.startsWith("--") && arg !== "--level");
  const levelArg = args.includes("--level") ? args[args.indexOf("--level") + 1] : "A1";
  const level: MockAuditLevel | undefined =
    levelArg === "pre-A1" || levelArg === "A1" || levelArg === "A2" ? levelArg : undefined;
  if (
    (mode !== "--write" && mode !== "--check" && mode !== "--report") ||
    level === undefined
  ) {
    process.stderr.write(
      "usage: spanish-a1-mock-audit-cli (--write | --check | --report) [--level pre-A1|A1|A2]\n",
    );
    return 2;
  }
  const current = serializeSpanishA1MockAudit(root, level);
  if (mode === "--report") {
    process.stdout.write(current);
    return 0;
  }
  const output = resolve(root, spanishMockAuditPath(level));
  if (mode === "--write") {
    writeFileSync(output, current, "utf8");
    return 0;
  }
  if (readFileSync(output, "utf8") !== current) {
    process.stderr.write(`${SPANISH_A1_MOCK_AUDIT}: generated audit is stale\n`);
    return 1;
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runSpanishA1MockAudit());
}
