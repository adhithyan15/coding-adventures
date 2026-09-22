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
 * The answer key's scored rows, from its TEXT.
 *
 * Split from `parseAnswerKey` so the parse can be tested the way
 * `parseMockPaper` is -- on strings, without a whole synthetic curriculum
 * behind it. Both this gate's silent-drop bugs (a line terminator the split
 * misses, an empty last column) were found by reading, not by a failing test,
 * because the only way in was a 20-minute run over the real corpus.
 */
export function parseAnswerKeyRows(text: string): AnswerKeyRow[] {
  const rows: AnswerKeyRow[] = [];
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
    if (row && (paper === 1 || paper === 2)) {
      rows.push({
        paper,
        item: Number(row[1]),
        // `.filter(Boolean)`, because `+` -> `*` is NOT inert here. The sibling
        // module says the widening is harmless, and in the reporter it is: an
        // entry of "" matches no form and prints nothing. This copy feeds the
        // GATE. A row whose last column is empty (`| 7 | b | |`) did not match
        // under `([^|]+)` and now matches with a capture of "", which `taught`
        // never contains -- so the item would fail on a requirement nobody
        // wrote, inflating `objectiveFailed` and putting "" in
        // `missingObjectiveLexemes`. Fail-closed rather than fail-open, and no
        // such row exists in any of the 24 keys, but the steering number of the
        // whole programme is not the place to carry a phantom. A trailing comma
        // in a hand-written row (`casa, perro,`) reaches the same filter.
        requires: row[2].split(",").map(clean).filter(Boolean),
      });
    }
  }
  return rows;
}

function parseAnswerKey(path: string): AnswerKeyRow[] {
  const rows = parseAnswerKeyRows(readFileSync(path, "utf8"));
  // A GATE THAT READ NOTHING MUST NOT REPORT SUCCESS.
  //
  // Every failure mode in the parser is silent by construction: a line
  // terminator the split misses, a heading whose numbering changes, a table
  // rewritten with a different pipe shape. Each leaves `rows` empty, and an
  // empty `rows` makes every downstream number zero -- `objectiveFailed: 0`,
  // which reads as a clean bill of health and is the exact opposite of what
  // happened. `--write` would then persist it.
  //
  // The check lives on the FILE side rather than inside `parseAnswerKeyRows`
  // because "" is a legitimate input to a parser and is not a legitimate
  // answer key, and it is unconditional rather than gated on having seen a
  // `## Prueba` heading: a heading-gated version goes quiet in the very case
  // where the heading pattern is what stopped matching. Both keys the audit
  // opens have Prueba 1 and Prueba 2 sections full of rows, so zero rows
  // always means the parse broke.
  if (rows.length === 0) {
    throw new Error(`${reportableFilename(path)}: parsed no answer-key rows`);
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
