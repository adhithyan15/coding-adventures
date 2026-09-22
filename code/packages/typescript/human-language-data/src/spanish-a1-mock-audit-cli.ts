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
  /**
   * Item counts the `## Prueba N` headings state for themselves.
   *
   * The file says how many items it has. Checking the parse against that is
   * the only guard here that does not depend on anticipating the shape of the
   * damage -- every silent drop, however it happened, is a shortfall.
   */
  readonly declared: ReadonlyMap<number, number>;
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
  const declared = new Map<number, number>();
  // The cell count of the FIRST scored row of each paper, which every later row
  // of that paper has to match. See the arity check below.
  const arityByPaper = new Map<number, number>();
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
    // EVERY level-2 heading resets the paper; only `## Prueba <digit>` sets it.
    //
    // The old `if (heading) paper = ...` could only ever SET, never clear, and
    // that is live in this corpus rather than hypothetical: `a2/mock-1` and
    // `a2/mock-2` write `## Pruebas 3 and 4` -- PLURAL, so `(\d)` cannot follow
    // the `s` -- above a section whose own text says it is "not read by the
    // audit". `paper` stayed 2 through it, so any numbered table added there
    // would be scored as listening. The A1 keys were safe only by luck: they
    // happen to spell `## Prueba 3` and `## Prueba 4`, which match and reset.
    //
    // Resetting to 0 rather than leaving it is the fail-CLOSED choice: an
    // unrecognised heading means "I do not know what section this is", and the
    // count check below then catches a Prueba whose rows went missing.
    if (/^##\s/.test(line)) {
      const heading = /^## Prueba (\d)/.exec(line);
      paper = heading ? Number(heading[1]) : 0;
      // The heading declares its own size -- `## Prueba 1 · Comprensión de
      // lectura (25 items)`. That is the invariant the guards were missing: a
      // count the FILE states, against a count the parse produced. Every silent
      // drop below shows up as a shortfall here, including the shapes the
      // `malformed` detector cannot see.
      const count = /^## Prueba \d[^(]*\((\d+) items?\)/.exec(line);
      if (heading && count) declared.set(Number(heading[1]), Number(count[1]));
    }
    // `([^|]*)` rather than `\s*([^|]+)\s*`. The old shape measured CUBIC --
    // 1.1s at a 2000-character line, 8.7s at 4000 -- which is strictly worse
    // than the two quadratic patterns CodeQL flagged in the sibling module, and
    // it is reachable by the same argument: `buildSpanishA1MockAudit` is
    // exported and takes a caller-supplied `root`. Fixing only the new copy and
    // leaving this one was the wrong call. `clean` already trims, so every row
    // in the corpus parses identically.
    const row = /^\|\s*(\d+)\s*\|.*\|([^|]*)\|$/.exec(line);
    if (row === null) {
      if (looksLikeDataRow(line)) malformed.push(line);
      continue;
    }
    if (paper !== 1 && paper !== 2) continue;
    // REJECT a row whose CELL COUNT disagrees with its table, rather than
    // taking the tail of a cell a stray pipe split.
    //
    // `([^|]*)` takes the LAST pipe-delimited run, so a pipe inside the
    // requirement cell silently truncates the list:
    //
    //     | 1 | a | casa \| ayuntamiento |   ->  ["ayuntamiento"]   `casa` gone
    //     | 1 | a | `a|b`, casa |            ->  ["b`", "casa"]
    //
    // A SHORTER requirement list is the fail-open direction: fewer things for
    // `taught` to miss, so the item is likelier to be credited as answerable
    // from the book.
    //
    // A first attempt compared the regex's capture against the last split
    // piece, which is WORTHLESS -- both take the last run, so they agree by
    // construction and the inline-code case sailed through. My own attack
    // matrix caught that, not review.
    //
    // The invariant that does work is the table's own: every row in a markdown
    // table has the same number of cells. A stray pipe gives the row one more;
    // an escaped `\|` gives it one fewer than the row pattern saw. Either way
    // it disagrees with the first row of its paper, and disagreement is all we
    // need -- we do not have to know which cell was meant.
    // TWO checks, because neither catches the other's case -- which I found by
    // running the matrix, after a single-check version passed one of them.
    //
    // ARITY catches a pipe that SPLITS a cell (`` `a|b` ``): the row gets one
    // more cell than the first row of its paper.
    //
    // ESCAPE catches a pipe that HIDES one (`casa \| ayuntamiento`): there the
    // split and the regex disagree in opposite directions, so the counts come
    // out equal and arity sees nothing -- while `([^|]*)` still stops at the
    // escaped pipe's `|` and drops `casa`. A requirement is a Spanish lexeme;
    // a pipe has no business inside one, escaped or not.
    if (/\\\|/.test(line)) {
      malformed.push(line);
      continue;
    }
    const arity = line.split("|").length;
    const expected = arityByPaper.get(paper);
    if (expected === undefined) {
      arityByPaper.set(paper, arity);
    } else if (arity !== expected) {
      malformed.push(line);
      continue;
    }
    const requires = (row[2] ?? "").split(",").map(clean).filter(Boolean);
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
  return { rows, unscored, malformed, declared };
}

/**
 * Does this line CLAIM to be one of the table's data rows?
 *
 * By SHAPE rather than by a prefix, because the first version -- a prefix test,
 * `/^\|\s*\d+\s*\|/` -- required the pipe at index 0 and a bare ASCII digit
 * immediately after it, which round three showed is blind to every edit but the
 * one it was written for. All of these dropped their row into no bucket at all
 * while the gate reported success:
 *
 *     | 2 | a | perro |     one trailing space   <- the only one it caught
 *    " | 2 | a | perro |"   one leading space, legal in GFM
 *     | **2** | a | perro | a bolded item number, already house style in
 *                           `pre-a1/mock-1-answer-key.md`
 *     | 2a | a | perro |    an item label with a suffix
 *
 * So: up to three leading spaces (GFM's own allowance), a pipe, and at least
 * three cells, of which the FIRST contains a digit. The digit is what separates
 * a data row from the furniture -- `| # | Clave | Requiere |` and `|---|---|---|`
 * carry none, and flagging those would make the gate refuse every real key.
 *
 * It is deliberately generous, and that is safe because it only ever runs on
 * the six keys the audit opens, which flag zero lines between them. Measured
 * across the wider corpus for scale: a line somewhere in 100 of the 8670
 * markdown files under `human-languages/` would trip it, mostly numbered
 * two-column tables that the arity check above already excludes from a
 * stricter draft's 177.
 *
 * This is the cheap half. The count check in `assertAnswerKeyParse` is the half
 * that does not depend on guessing which mutations a human might make.
 */
function looksLikeDataRow(line: string): boolean {
  if (!/^ {0,3}\|/.test(line)) return false;
  // FIVE, not four. A data row of this table has THREE columns, so splitting on
  // the pipe yields five pieces -- the two empty edges plus the three cells.
  // Four is what a TWO-column numbered table yields (`| 1 | ***onru*** |`), and
  // those exist all over the corpus. A test pins it, because the first draft of
  // this used four and the test is what caught it.
  const cells = line.split("|");
  return cells.length >= 5 && /\d/.test(cells[1] ?? "");
}

/**
 * Refuse a parse that lost something, and say what.
 *
 * Exported and taking a PARSE rather than a path, for the reason
 * `parseAnswerKeyRows` is: the only way into this logic used to be a
 * twenty-minute run over the real corpus, which is why three rounds of review
 * found its holes by reading instead of by a failing test.
 *
 * A GATE THAT READ NOTHING MUST NOT REPORT SUCCESS -- and "nothing" has six
 * shapes, not one. The first version of this checked `rows.length === 0`,
 * which fires only when EVERY row is lost; one trailing space drops a single
 * row and sails past it, and a single dropped row is an item this gate never
 * scores, which downstream is indistinguishable from an item that passed.
 */
export function assertAnswerKeyParse(parse: AnswerKeyParse, name: string): void {
  const { rows, unscored, malformed, declared } = parse;
  if (malformed.length > 0) {
    // The rejected line is NAMED rather than counted. A message that says only
    // "1 table row rejected" tells a maintainer that something is wrong and
    // nothing about where, in a file of 160 lines. Through
    // `reportableFilename`, because this line is file content: it strips C0/C1
    // and quotes, so a row carrying a newline cannot forge a second log line.
    const first = reportableFilename(malformed[0] ?? "");
    throw new Error(
      `${name}: ${malformed.length} table row(s) the row pattern rejected, first ${first}`,
    );
  }
  if (unscored.length > 0) {
    const items = unscored.map(({ paper, item }) => `${paper}.${item}`).join(", ");
    throw new Error(`${name}: item(s) ${items} state no requirements`);
  }
  if (rows.length === 0) {
    throw new Error(`${name}: parsed no answer-key rows`);
  }
  for (const paper of [1, 2]) {
    const items = rows.filter((row) => row.paper === paper).map((row) => row.item);
    if (items.length === 0) {
      throw new Error(`${name}: parsed no Prueba ${paper} rows`);
    }
    // THE TWO GUARDS THAT DO NOT DEPEND ON GUESSING THE DAMAGE.
    //
    // Everything above catches a shape somebody anticipated, and round three
    // showed how thin that is: one leading space, a bolded item number, an item
    // label with a suffix, or a `\v` joining two rows all dropped a row into no
    // bucket while every guard reported success. On the real A1 key, joining
    // rows 3 and 4 scored item 3 against item 4's requirements and nothing
    // objected.
    //
    // These two do not care HOW a row went missing. The heading states the
    // count and the numbers run consecutively, so a parse that lost a row is
    // short or has a hole, whatever removed it. Measured against the real A1
    // key: all ten mutations round three raised are caught, and all six real
    // keys pass untouched.
    const sorted = [...items].sort((left, right) => left - right);
    // SPAN, not adjacency. `findIndex((item, index) => index > 0 && ...)` never
    // examines index 0, so a paper whose FIRST row was lost read as perfectly
    // contiguous -- `[2,3,...,25]` has no jump in it. Comparing the span to the
    // length has no such blind spot, and it catches duplicates too.
    const span = (sorted[sorted.length - 1] ?? 0) - (sorted[0] ?? 0) + 1;
    if (span !== sorted.length) {
      const gap = sorted.findIndex((item, index) => index > 0 && item !== (sorted[index - 1] ?? 0) + 1);
      throw new Error(
        gap > 0
          ? `${name}: Prueba ${paper} item numbers jump from ${sorted[gap - 1]} to ${sorted[gap]}`
          : `${name}: Prueba ${paper} has ${sorted.length} rows spanning ${sorted[0]}-${sorted[sorted.length - 1]}`,
      );
    }
    // UNCONDITIONAL. This was `count !== undefined && ...`, which means the one
    // guard here that does NOT depend on anticipating the shape of the damage
    // switched itself off whenever the heading stopped saying `(25 items)` --
    // and said nothing when it did.
    //
    // Both edits that disable it are ordinary editorial changes, not attacks.
    // `## Prueba 3 · Expresión e interacción escritas` in the SAME FILE already
    // carries no count, so normalising the headings is a plausible tidy-up; and
    // `items` -> `ítems` is the correct Spanish spelling in a file that already
    // writes `Comprensión` and `auditiva`. Either one, plus the loss of the
    // paper's first row by any mechanism, produced a clean bill of health for
    // 24 of 25 scored items. The two holes lined up exactly.
    //
    // A guard that switches itself off when the file it guards changes is not a
    // guard. All six real keys declare a count for both scored papers today.
    const count = declared.get(paper);
    if (count === undefined) {
      throw new Error(`${name}: Prueba ${paper} heading declares no item count`);
    }
    if (items.length !== count) {
      throw new Error(`${name}: Prueba ${paper} declares ${count} items, parsed ${items.length}`);
    }
  }
}

function parseAnswerKey(path: string): readonly AnswerKeyRow[] {
  const parse = parseAnswerKeyRows(readFileSync(path, "utf8"));
  assertAnswerKeyParse(parse, reportableFilename(path));
  return parse.rows;
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
