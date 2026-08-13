// The filesystem boundary. Everything impure lives here: it reads the curriculum
// directory off disk and hands strings to the pure parse/build/validate core.
// This is the only module that needs the `filesystem` capability.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  MODALITY_MANIFEST_PATH,
  type ModalityManifest,
  type ModalityManifestLesson,
} from "./modality-manifest.js";
import { buildDataset, parseLesson, type ParsedLesson } from "./parse.js";
import type { LetterLedger } from "./letter-ledger.js";
import type {
  BookChapter,
  BookCorpus,
  ChapterPolicy,
  CurriculumSpine,
  Dataset,
  LanguageCurriculum,
  LanguageRegistry,
  Script,
  ScriptData,
  Taxonomy,
  TrackChapters,
  GrammarSlotInventory,
  MetalanguageInventory,
  TrackGrammarCells,
} from "./types.js";

/** Default curriculum root: code/learning/human-languages, relative to this package. */
/**
 * Directory entries in a stable order.
 *
 * `readdirSync` returns whatever the filesystem hands back, which differs
 * between APFS and ext4 and shifts as files are added. That leaked once
 * already: the cross-track cousin join resolved ties by "whichever lesson the
 * corpus yielded first", so reversing the corpus changed the printed cousin for
 * 35 lessons. That module now carries a total order of its own — but every
 * other consumer inherits this iteration order, and one of the chapter reads
 * below was ALREADY sorting, so the convention existed and had simply not been
 * applied everywhere. Sorting here removes the class of problem rather than the
 * one instance that happened to be found.
 */
function sortedEntries(root: string) {
  return readdirSync(root, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
}

export function defaultCurriculumRoot(): string {
  const here = dirname(fileURLToPath(import.meta.url));
  // src/ -> human-language-data -> typescript -> packages -> code
  return join(here, "..", "..", "..", "..", "learning", "human-languages");
}

export function loadTaxonomy(root = defaultCurriculumRoot()): Taxonomy {
  const raw = JSON.parse(readFileSync(join(root, "concepts", "taxonomy.json"), "utf8"));
  return { version: raw.version ?? 1, concepts: raw.concepts ?? {} };
}

export function loadLanguageRegistry(root = defaultCurriculumRoot()): LanguageRegistry {
  return JSON.parse(
    readFileSync(join(root, "core", "languages.json"), "utf8"),
  ) as LanguageRegistry;
}

export function loadCurriculumSpine(root = defaultCurriculumRoot()): CurriculumSpine {
  return JSON.parse(
    readFileSync(join(root, "core", "spine.json"), "utf8"),
  ) as CurriculumSpine;
}

/** HL10 section 7.5: the metalanguage ramp -- the words for talking about language. */
export function loadMetalanguage(root = defaultCurriculumRoot()): MetalanguageInventory {
  return JSON.parse(
    readFileSync(join(root, "core", "metalanguage.json"), "utf8"),
  ) as MetalanguageInventory;
}

/** HL10 section 5.1: the universal grammar-slot inventory, generated and committed. */
export function loadGrammarSlots(root = defaultCurriculumRoot()): GrammarSlotInventory {
  return JSON.parse(
    readFileSync(join(root, "core", "grammar-slots.json"), "utf8"),
  ) as GrammarSlotInventory;
}

/** HL10 section 5: one track's filling of those slots, with its prerequisite ordering. */
export function loadTrackGrammarCells(
  language: string,
  root = defaultCurriculumRoot(),
): TrackGrammarCells {
  return JSON.parse(
    readFileSync(join(root, language, "grammar-cells.json"), "utf8"),
  ) as TrackGrammarCells;
}

/** Read each track's authored shared-spine realization map. */
export function loadLanguageCurricula(root = defaultCurriculumRoot()): LanguageCurriculum[] {
  const out: LanguageCurriculum[] = [];
  for (const track of sortedEntries(root)) {
    if (!track.isDirectory()) continue;
    const path = join(root, track.name, "curriculum.json");
    if (!existsSync(path)) continue;
    out.push(JSON.parse(readFileSync(path, "utf8")) as LanguageCurriculum);
  }
  return out.sort((left, right) => left.language.localeCompare(right.language));
}

/**
 * Read each track's authored chapter capability ledger (HL05).
 *
 * Tracks without a `chapters.json` are skipped rather than defaulted. That is
 * deliberate: an absent ledger means "not yet authored", which the gap report must
 * be able to distinguish from "authored and empty". Inventing a placeholder here
 * would erase exactly the debt the report exists to measure.
 */
export function loadTrackChapters(root = defaultCurriculumRoot()): TrackChapters[] {
  const out: TrackChapters[] = [];
  for (const track of sortedEntries(root)) {
    if (!track.isDirectory()) continue;
    const path = join(root, track.name, "chapters.json");
    if (!existsSync(path)) continue;
    out.push(JSON.parse(readFileSync(path, "utf8")) as TrackChapters);
  }
  return out.sort((left, right) => left.language.localeCompare(right.language));
}

/**
 * Read the tunable chapter policy. Unlike the ledgers above, this file is required:
 * a missing policy would silently disable the payoff and ramp rules, and a gate that
 * quietly stops running is worse than one that fails loudly.
 */
export function loadChapterPolicy(root = defaultCurriculumRoot()): ChapterPolicy {
  const policy = JSON.parse(
    readFileSync(join(root, "core", "chapter-policy.json"), "utf8"),
  ) as ChapterPolicy;

  // Validate the budgets, because the alternative is a gate that reads zero and looks
  // clean. `JSON.parse` turns `1e999` into `Infinity`, and `newTarget.size > Infinity`
  // is false for every lesson in the corpus — so a single typo publishes "0 violations",
  // which is indistinguishable in the report from "measured, found none". A string or an
  // object budget fails the same way. That silent-disable is the exact failure this
  // function's own docstring warns about, and it was unchecked.
  //
  // Required, not optional, for the atom budgets: `measureRamp` reads them with no
  // default, so a policy file missing them yields `undefined` and the same silent zero.
  const budgets: ReadonlyArray<readonly [keyof ChapterPolicy, boolean]> = [
    ["maxNewAtomsPerLesson", true],
    ["maxNewAtomsPerChapter", true],
    ["maxLinearisableTableColumns", false],
    ["maxNewGlyphsPerLesson", false],
    ["maxNewScriptSystemsPerLesson", false],
  ];
  for (const [key, required] of budgets) {
    const value = policy[key];
    if (value === undefined) {
      if (required) {
        throw new Error(`chapter-policy.json: ${key} is required and missing`);
      }
      continue;
    }
    if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
      throw new Error(
        `chapter-policy.json: ${key} must be a non-negative integer, got ${JSON.stringify(value)}`,
      );
    }
  }
  return policy;
}

/** Read a braced LaTeX command argument without truncating nested formatting commands. */
export function chapterTitleFromTex(tex: string, fallback: string): string {
  const command = /\\chapter(?:\s*\[[^\]]*\])?\s*\{/.exec(tex);
  if (!command) return fallback;
  const openingBrace = command.index + command[0].lastIndexOf("{");
  let depth = 1;
  for (let index = openingBrace + 1; index < tex.length; index += 1) {
    const character = tex[index];
    const escaped = index > 0 && tex[index - 1] === "\\";
    if (!escaped && character === "{") depth += 1;
    if (!escaped && character === "}") {
      depth -= 1;
      if (depth === 0) return tex.slice(openingBrace + 1, index);
    }
  }
  return fallback;
}

/**
 * Load the existing authored LaTeX books losslessly. The short Markdown lessons
 * remain the smallest teaching units; chapters are the narrative and sequencing
 * layer around them. Keeping both in the data package prevents an app or future
 * book generator from silently forgetting either source.
 */
export function loadBookCorpus(root = defaultCurriculumRoot()): BookCorpus {
  const books: BookCorpus["books"] = [];
  for (const track of sortedEntries(root)) {
    if (!track.isDirectory()) continue;
    const bookDir = join(root, track.name, "book");
    const entrypoint = join(bookDir, "book.tex");
    const chaptersDir = join(bookDir, "chapters");
    if (!existsSync(entrypoint) || !existsSync(chaptersDir)) continue;

    const chapters: BookChapter[] = [];
    for (const file of readdirSync(chaptersDir).sort()) {
      const match = /^ch(\d+)-(.+)\.tex$/.exec(file);
      if (!match) continue;
      const tex = readFileSync(join(chaptersDir, file), "utf8");
      const title = chapterTitleFromTex(tex, match[2]);
      chapters.push({
        language: track.name,
        chapter: Number(match[1]),
        slug: match[2],
        title,
        source: `${track.name}/book/chapters/${file}`,
        tex,
      });
    }
    books.push({
      language: track.name,
      entrypoint: `${track.name}/book/book.tex`,
      chapters,
    });
  }
  return { books: books.sort((a, b) => a.language.localeCompare(b.language)) };
}

/**
 * A track may declare its own script in `<track>/track.json` (`{ "script": "hebrew" }`),
 * so adding a new-script language needs no edit to the built-in map. Returns
 * `undefined` when there's no declaration, and the parser falls back to the map.
 */
export function trackScript(root: string, trackName: string): Script | undefined {
  const p = join(root, trackName, "track.json");
  if (!existsSync(p)) return undefined;
  try {
    const t = JSON.parse(readFileSync(p, "utf8"));
    return typeof t?.script === "string" ? t.script : undefined;
  } catch {
    return undefined; // a malformed track.json just falls back to the map
  }
}

/** Read every track's lessons/*.md into parsed lessons. */
export function loadLessons(root = defaultCurriculumRoot()): ParsedLesson[] {
  const out: ParsedLesson[] = [];
  for (const track of sortedEntries(root)) {
    if (!track.isDirectory()) continue;
    const lessonsDir = join(root, track.name, "lessons");
    if (!existsSync(lessonsDir)) continue;
    const script = trackScript(root, track.name);
    for (const file of readdirSync(lessonsDir).sort()) {
      if (!file.endsWith(".md")) continue;
      const source = readFileSync(join(lessonsDir, file), "utf8");
      out.push(parseLesson(source, track.name, script));
    }
  }
  return out;
}

/**
 * Read the generated modality manifest (HL08 / HL-C44).
 *
 * The consumer-side counterpart of `modality-cli --write`. An app, a book builder, or
 * the future driving edition reads this instead of importing the derivation and
 * re-parsing 1,096 Markdown files — which is the point of emitting it at all.
 *
 * Required, not optional, and deliberately so. A missing manifest throws rather than
 * returning an empty one: "no modality data" and "no lesson needs eyes" are opposite
 * facts, and a loader that quietly returns the second when it means the first would
 * hand a driver the handwriting drills. CI's `--check` guarantees the file is present
 * and current, so the throw is unreachable in a healthy checkout.
 */
export function loadModalityManifest(root = defaultCurriculumRoot()): ModalityManifest {
  return JSON.parse(readFileSync(join(root, MODALITY_MANIFEST_PATH), "utf8")) as ModalityManifest;
}

/**
 * Index a manifest's lessons by id.
 *
 * A `Map`, never a plain object, and this is the one place in the package where that
 * choice is load-bearing rather than stylistic. The keys come straight out of parsed
 * JSON, so `obj[lesson.id] = lesson` with an id of `__proto__` writes the object's
 * prototype instead of a property — every later lookup then inherits attacker-chosen
 * fields, and `manifest["anything"]` starts answering with a modality nobody authored.
 * `Map` keys are plain data with no prototype chain behind them, so the same input is
 * simply a key named `__proto__`.
 *
 * Providing this here means no consumer has to rediscover that.
 */
export function modalityManifestById(
  manifest: ModalityManifest,
): Map<string, ModalityManifestLesson> {
  const index = new Map<string, ModalityManifestLesson>();
  for (const lesson of manifest.lessons) index.set(lesson.id, lesson);
  return index;
}

/** Suffix marking a letter ledger, which lives beside the script inventories. */
const LEDGER_SUFFIX = "-ledger.json";

/** Read data/scripts/*.json (may be empty while scripts are still being authored). */
export function loadScripts(root = defaultCurriculumRoot()): Record<string, ScriptData> {
  const dir = join(root, "data", "scripts");
  const out: Record<string, ScriptData> = Object.create(null);
  if (!existsSync(dir)) return out;
  for (const file of readdirSync(dir).sort()) {
    if (!file.endsWith(".json")) continue;
    // A letter ledger sits in this directory and carries the SAME `script` key
    // as the inventory it orders, so reading both into one map would have one
    // silently overwrite the other. Which one won would depend on filename
    // sort order, which is not a thing to depend on.
    if (file.endsWith(LEDGER_SUFFIX)) continue;
    const sd = JSON.parse(readFileSync(join(dir, file), "utf8")) as ScriptData;
    out[sd.script] = sd;
  }
  return out;
}

/**
 * Read data/scripts/*-ledger.json — the order a reader meets each script's
 * letters (HL11 section 4).
 *
 * Scripts without a ledger are SKIPPED, not defaulted to an empty one, for the
 * same reason `loadTrackChapters` skips tracks without a capability ledger:
 * "not yet authored" and "authored and empty" are different kinds of debt, and
 * collapsing the first into the second erases what the gap report exists to
 * measure.
 */
export function loadLetterLedgers(root = defaultCurriculumRoot()): LetterLedger[] {
  const dir = join(root, "data", "scripts");
  if (!existsSync(dir)) return [];
  const out: LetterLedger[] = [];
  for (const file of readdirSync(dir).sort()) {
    if (!file.endsWith(LEDGER_SUFFIX)) continue;
    out.push(JSON.parse(readFileSync(join(dir, file), "utf8")) as LetterLedger);
  }
  return out;
}

/** Load and build everything from disk in one call. */
export function loadEverything(root = defaultCurriculumRoot()): {
  taxonomy: Taxonomy;
  registry: LanguageRegistry;
  spine: CurriculumSpine;
  curricula: LanguageCurriculum[];
  books: BookCorpus;
  lessons: ParsedLesson[];
  scripts: Record<string, ScriptData>;
  letterLedgers: LetterLedger[];
  dataset: Dataset;
} {
  const taxonomy = loadTaxonomy(root);
  const registry = loadLanguageRegistry(root);
  const spine = loadCurriculumSpine(root);
  const curricula = loadLanguageCurricula(root);
  const books = loadBookCorpus(root);
  const lessons = loadLessons(root);
  const scripts = loadScripts(root);
  const letterLedgers = loadLetterLedgers(root);
  return {
    taxonomy,
    registry,
    spine,
    curricula,
    books,
    lessons,
    scripts,
    letterLedgers,
    dataset: buildDataset(taxonomy, lessons),
  };
}
