import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { lessonsUpToLevel } from "./levels.js";
import { defaultCurriculumRoot, loadEverything } from "./loader.js";

export const SPANISH_A1_MOCK_AUDIT = "spanish/mocks/a1/book-bounded-audit.json";

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

type Item = { paper: number; item: number; requires: string[] };

function parseAnswerKey(path: string): Item[] {
  const rows: Item[] = [];
  let paper = 0;
  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    const heading = /^## Prueba (\d)/.exec(line);
    if (heading) paper = Number(heading[1]);
    const row = /^\|\s*(\d+)\s*\|.*\|\s*([^|]+)\s*\|$/.exec(line);
    if (row && (paper === 1 || paper === 2)) {
      rows.push({
        paper,
        item: Number(row[1]),
        requires: row[2].split(",").map(clean),
      });
    }
  }
  return rows;
}

export function buildSpanishA1MockAudit(root = defaultCurriculumRoot()) {
  const everything = loadEverything(root);
  const lessons = lessonsUpToLevel(
    everything.lessons.filter((lesson) => lesson.language === "spanish"),
    everything.curricula.filter((path) => path.language === "spanish"),
    everything.spine,
    "A1",
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

  const answerKeys = [1, 2].map((mock) => ({
    mock,
    rows: parseAnswerKey(resolve(root, `spanish/mocks/a1/mock-${mock}-answer-key.md`)),
  }));
  const mocks = answerKeys.map(({ mock, rows }) => {
    const failed = rows
      .map((row) => ({
        paper: row.paper,
        item: row.item,
        missing: row.requires.filter((entry) => !entry.startsWith("!") && !taught.has(entry)),
      }))
      .filter((row) => row.missing.length > 0);
    const passes = (row: Item) => row.requires.every((entry) => entry.startsWith("!") || taught.has(entry));
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
    level: "A1",
    policy: {
      description: "Credit explicit A1 headwords, their article-free and token forms, documented citation aliases, and Spanish numerals 0-100.",
      citationFormCredits,
      numberWordCredits,
      numericCredits: "0-100",
    },
    lessonCount: lessons.length,
    taughtForms: taught.size,
    objectiveFailed: mocks.reduce((sum, mock) => sum + mock.objectiveFailed, 0),
    mocks,
    missingObjectiveLexemes: [...missingFrequency.keys()].sort(),
    missingFrequency: [...missingFrequency]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([lexeme, count]) => ({ lexeme, count })),
  };
}

export function serializeSpanishA1MockAudit(root = defaultCurriculumRoot()): string {
  return `${JSON.stringify(buildSpanishA1MockAudit(root), null, 2)}\n`;
}

export function runSpanishA1MockAudit(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--write" && mode !== "--check" && mode !== "--report") {
    process.stderr.write("usage: spanish-a1-mock-audit-cli (--write | --check | --report)\n");
    return 2;
  }
  const current = serializeSpanishA1MockAudit(root);
  if (mode === "--report") {
    process.stdout.write(current);
    return 0;
  }
  const output = resolve(root, SPANISH_A1_MOCK_AUDIT);
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
