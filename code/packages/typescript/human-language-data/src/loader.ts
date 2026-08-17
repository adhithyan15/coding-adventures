// The filesystem boundary. Everything impure lives here: it reads the curriculum
// directory off disk and hands strings to the pure parse/build/validate core.
// This is the only module that needs the `filesystem` capability.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  MODALITY_MANIFEST_PATH,
  type ModalityManifest,
  type ModalityManifestLesson,
} from "./modality-manifest.js";
import { buildDataset, parseLesson, type ParsedLesson } from "./parse.js";
import type { ExamInventory } from "./exam-inventory.js";
import type { LedgerLetter, LetterLedger } from "./letter-ledger.js";
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
    const track_ = JSON.parse(readFileSync(path, "utf8")) as TrackChapters;
    for (const chapter of track_.chapters ?? []) {
      // A chapter's `label` is interpolated RAW into `\label{...}` by the book
      // generator -- the one author-controlled field in that file with no guard,
      // while titles go through the LaTeX escaper, output paths through
      // `safeOutput`, activity ids through their own regex, and script commands
      // through `/^[A-Za-z@]+$/`.
      //
      // Left raw, a label of `ch:x}\immediate\write18{id}{` closes the brace and
      // emits a live control sequence into a generated .tex. Today's builds run
      // plain `latexmk -xelatex` with no `-shell-escape`, so `\write18` would be
      // refused -- but `\input` and `\openout` would not be, and "the compiler
      // flag saves us" is not a property this file can see or keep.
      //
      // The allowlist matches every label convention the corpus already uses:
      // `ch:greetings`, `ch:fa-alefbe`, `ch:persian-greetings`, `ch:zh-components`.
      if (typeof chapter.label !== "string" || !/^[A-Za-z0-9:_-]+$/.test(chapter.label)) {
        throw new Error(
          `${track_.language} chapter ${chapter.chapter}: label must match /^[A-Za-z0-9:_-]+$/, got '${chapter.label}'`,
        );
      }
    }
    out.push(track_);
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

/**
 * Is this a letter ledger rather than a script inventory?
 *
 * Case-insensitive on purpose. A file committed as `Tamil-Ledger.json` would
 * otherwise fail this test, be read as a script inventory, and collide with the
 * real `tamil.json` under the same key -- which is precisely the collision the
 * skip exists to prevent.
 */
function isLedgerFile(file: string): boolean {
  return file.toLowerCase().endsWith(LEDGER_SUFFIX);
}

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
    if (isLedgerFile(file)) continue;
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
    if (!isLedgerFile(file)) continue;
    const raw = JSON.parse(readFileSync(join(dir, file), "utf8")) as Partial<LetterLedger>;
    // Shape-checked at the boundary. `validateLetterLedger` cannot report a
    // malformed ledger, because it walks these two arrays before it checks
    // anything -- so a missing key would surface as an unhandled TypeError out
    // of `loadEverything`, not as an issue anyone could read.
    if (!Array.isArray(raw.letters) || !Array.isArray(raw.tracks)) {
      throw new Error(
        `${file}: a letter ledger needs 'letters' and 'tracks' arrays`,
      );
    }
    // And the rows, not just the two arrays. The validator reads `glyph`,
    // `unicodeName` and `unlocks` off every row before it checks anything, so a
    // row missing one of them fails as an unhandled TypeError out of
    // `loadEverything` rather than as an error anyone can read. Checking the top
    // level alone moved that failure down a level; it did not remove it.
    raw.letters.forEach((row, index) => {
      const letter = row as Partial<LedgerLetter> | null;
      if (
        !letter ||
        typeof letter !== "object" ||
        typeof letter.glyph !== "string" ||
        typeof letter.unicodeName !== "string" ||
        typeof letter.codePoint !== "string" ||
        !Array.isArray(letter.unlocks)
      ) {
        throw new Error(
          `${file}: letters[${index}] needs string 'glyph', 'codePoint' and ` +
          `'unicodeName', and an 'unlocks' array`,
        );
      }
    });
    out.push(raw as LetterLedger);
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

/**
 * The exam inventory for one language and level.
 *
 * Validated on load rather than trusted, for the same reason `loadChapterPolicy`
 * validates its budgets: a malformed probe would make the gate read HIGHER, not
 * lower. `probe: []` — an empty array rather than `null` — asks for zero atoms,
 * every one of which is trivially present, so the point would score as covered
 * while demonstrating nothing. That is the one shape this file must refuse.
 */
const SAFE_INVENTORY_SEGMENT = /^[A-Za-z0-9]+$/;

export function loadExamInventory(
  language: string,
  level: string,
  root = defaultCurriculumRoot(),
): ExamInventory {
  // Both parameters are interpolated into a filename, and `join` NORMALISES
  // `..` inside the interpolated part rather than rejecting it — so
  // `level = "../../../../etc/shadow"` resolves to `/etc/shadow.json` and the
  // trailing `.json` is no protection at all (`.docker/config.json` holds
  // registry credentials). Today the only callers pass literals, which is
  // exactly when a guard is cheap; the moment this is wired to a `--level`
  // flag, as the A2 work will, it stops being cheap.
  if (!SAFE_INVENTORY_SEGMENT.test(language) || !SAFE_INVENTORY_SEGMENT.test(level)) {
    throw new Error(`exam inventory: refusing unsafe language/level '${language}'/'${level}'`);
  }
  // Lower-cased before the comparison as well as after: `language` is matched
  // against a literal, so `"SPANISH"` would otherwise build
  // `exam-inventory-SPANISH-a1.json` — which resolves on a case-insensitive
  // filesystem and ENOENTs on case-sensitive CI. A gate that depends on the
  // developer's filesystem is not a gate.
  const normalized = language.toLowerCase();
  const code = normalized === "spanish" ? "es" : normalized;
  const directory = resolve(root, "core");
  const file = resolve(directory, `exam-inventory-${code}-${level.toLowerCase()}.json`);
  // Belt and braces: the allowlist above already excludes separators, so this
  // can only fire if that regex is ever loosened. It is here so that loosening
  // it fails loudly rather than silently reopening the hole.
  if (dirname(file) !== directory) {
    throw new Error("exam inventory: resolved path escapes the curriculum root");
  }

  const parsed = JSON.parse(readFileSync(file, "utf8")) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !Array.isArray((parsed as ExamInventory).points) ||
    (parsed as ExamInventory).points.length === 0
  ) {
    // An unchecked cast would defer this to `TypeError: points is not iterable`
    // somewhere downstream, which reads like a crash rather than a refusal.
    throw new Error(`exam inventory: ${file} has no non-empty 'points' array`);
  }
  const inventory = parsed as ExamInventory;
  const seen = new Set<string>();
  for (const point of inventory.points) {
    if (seen.has(point.id)) throw new Error(`exam inventory: duplicate point id '${point.id}'`);
    seen.add(point.id);
    // `__proto__` and friends as a category would pollute the accumulator in
    // `measureExamCoverage`. That function is defended independently with a
    // null-prototype object; this refusal keeps a malformed file from reaching
    // it at all, and names the file rather than leaving a silent NaN behind.
    if (point.category === "__proto__" || point.category === "constructor" || point.category === "prototype") {
      throw new Error(`exam inventory: point '${point.id}' uses a reserved category name`);
    }
    // `covered` is `probe !== null && ...`. A MISSING key is `undefined`, which is
    // not `null`, so the point scores COVERED while demonstrating nothing -- and
    // an inventory with every probe deleted reports 100% and silently suppresses
    // its own work item. The empty-array case was already refused for exactly
    // this reason; `undefined` and a non-array are the same fault in a different
    // shape, so all three are refused together.
    if (!(point.probe === null || (Array.isArray(point.probe) && point.probe.length > 0))) {
      throw new Error(
        `exam inventory: point '${point.id}' has no usable probe; use null to mean "nothing in the corpus covers this"`,
      );
    }
  }
  return inventory;
}

/**
 * Every external exam inventory that exists on disk, by its own declared fields.
 *
 * Deliberately NOT derived from the filename. `loadExamInventory` maps
 * `spanish` to the code `es`, so the file is `exam-inventory-es-a1.json` while
 * the track is `spanish` — and a queue keyed on the filename would therefore
 * report Spanish's A1 inventory as missing and queue somebody to write it again.
 * The file states `language` and `level` itself; that is the answer.
 *
 * A malformed file is SKIPPED rather than thrown on. This function answers
 * "which targets are written down", and one unparseable file should not stop the
 * whole plan from being computed — `loadExamInventory` is still the strict door
 * that anything actually measuring coverage has to come through.
 */
export function listExamInventories(root = defaultCurriculumRoot()): { language: string; level: string }[] {
  const directory = resolve(root, "core");
  if (!existsSync(directory)) return [];
  const found: { language: string; level: string }[] = [];
  for (const file of readdirSync(directory).sort()) {
    if (!file.startsWith("exam-inventory-") || !file.endsWith(".json")) continue;
    try {
      const parsed = JSON.parse(readFileSync(resolve(directory, file), "utf8")) as Partial<ExamInventory>;
      if (typeof parsed.language === "string" && typeof parsed.level === "string") {
        found.push({ language: parsed.language, level: parsed.level });
      }
    } catch {
      // Skipped on purpose — see the note above.
    }
  }
  return found;
}

/**
 * Everything the glyph gate needs: each book's preamble, its generated files, and
 * the coverage of every vendored font its preamble names.
 *
 * Fonts are read from `_fonts/` and their cmaps decoded here rather than in
 * `glyph-coverage.ts`, so that module stays pure over data and can be tested
 * without a font on disk.
 */
export function loadBookFonts(root = defaultCurriculumRoot()): {
  language: string;
  preamble: string;
  files: { path: string; text: string }[];
  scriptFonts: Record<string, Set<number>>;
}[] {
  const out: ReturnType<typeof loadBookFonts> = [];
  const fontCache = new Map<string, Set<number>>();
  for (const track of sortedEntries(root)) {
    if (!track.isDirectory()) continue;
    const bookDir = resolve(root, track.name, "book");
    const preamblePath = resolve(bookDir, "preamble.tex");
    if (!existsSync(preamblePath)) continue;
    const preamble = readFileSync(preamblePath, "utf8");

    const files: { path: string; text: string }[] = [];
    const walk = (dir: string): void => {
      for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
        const full = resolve(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else if (entry.name.endsWith(".tex") && entry.name !== "preamble.tex") {
          files.push({ path: `${track.name}/book/${full.slice(bookDir.length + 1)}`, text: readFileSync(full, "utf8") });
        }
      }
    };
    walk(bookDir);

    // `Object.create(null)`: a font named `constructor` or `__proto__` in a
    // preamble would otherwise resolve through the prototype chain to a truthy
    // value, sail past the `if (!cover)` unmeasured guard, and throw on
    // `cover.has`. The write side only accepts `.ttf`/`.otf`; the READ side had
    // no such filter, and that asymmetry was the bug.
    const scriptFonts: Record<string, Set<number>> = Object.create(null);
    for (const [, file] of preamble.matchAll(/\\newfontfamily\\\w+\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/g)) {
      const name = file.trim();
      if (!name.endsWith(".ttf") && !name.endsWith(".otf")) continue;
      let cover = fontCache.get(name);
      if (!cover) {
        const path = resolve(root, "_fonts", name.split("/").pop()!);
        // A font that cannot be resolved is left OUT of the map, which
        // `measureGlyphCoverage` treats as unmeasured rather than as clean.
        if (!existsSync(path)) continue;
        cover = readFontCoverage(readFileSync(path));
        // An empty cmap means the font did not parse. Leaving it OUT of the map
        // is genuinely unmeasured; putting it in would report every character in
        // that script as missing. The doc comment above used to promise an
        // assertion here that did not exist.
        if (cover.size === 0) continue;
        fontCache.set(name, cover);
      }
      scriptFonts[name] = cover;
    }
    out.push({ language: track.name, preamble, files, scriptFonts });
  }
  return out;
}

/**
 * Codepoints a TrueType/OpenType font covers, from its `cmap` table.
 *
 * Only formats 4 and 12 are decoded; between them they carry every mapping in
 * the vendored Noto faces. A font whose cmap uses neither yields an EMPTY set,
 * which would report every character missing -- so `loadBookFonts` drops an
 * empty result from the map, leaving that font genuinely unmeasured.
 */
function readFontCoverage(buffer: Buffer): Set<number> {
  const out = new Set<number>();
  // Every offset below is read FROM the buffer and then used to index it, so each
  // one is bounds-checked before use. Node's readUInt*BE throws rather than
  // reading out of bounds, so the risk is availability, not disclosure -- but an
  // unhandled RangeError takes the whole report down, and a font is exactly the
  // kind of file somebody drops in without looking.
  if (buffer.length < 12) return out;
  const numTables = buffer.readUInt16BE(4);
  let cmapOffset = 0;
  for (let i = 0; i < numTables; i += 1) {
    const record = 12 + i * 16;
    if (record + 16 > buffer.length) break;
    if (buffer.toString("ascii", record, record + 4) === "cmap") cmapOffset = buffer.readUInt32BE(record + 8);
  }
  if (cmapOffset === 0 || cmapOffset + 4 > buffer.length) return out;
  const numSub = buffer.readUInt16BE(cmapOffset + 2);
  // A budget on WORK DONE, not on set size. Bounding by `out.size` looks
  // equivalent and is not: format 4 caps at 65,536 codepoints, so a font with
  // thousands of overlapping full-range segments never grows the set past that
  // ceiling and loops for 5.4 seconds anyway. Counting additions instead bounds
  // both branches. 0x110000 is the whole of Unicode; anything past it is a font
  // asking to map more codepoints than exist.
  const BUDGET = 0x110000;
  let work = 0;
  for (let i = 0; i < numSub; i += 1) {
    const sub = cmapOffset + 4 + i * 8;
    if (sub + 8 > buffer.length) break;
    const table = cmapOffset + buffer.readUInt32BE(sub + 4);
    if (table + 4 > buffer.length) continue;
    const format = buffer.readUInt16BE(table);
    if (format === 4) {
      if (table + 16 > buffer.length) continue;
      const segX2 = buffer.readUInt16BE(table + 6);
      const ends = table + 14;
      const starts = ends + segX2 + 2;
      if (starts + segX2 > buffer.length) continue;
      for (let s = 0; s < segX2; s += 2) {
        const end = buffer.readUInt16BE(ends + s);
        const start = buffer.readUInt16BE(starts + s);
        // `start > end` is malformed; without the guard a crafted font can spend
        // 45 seconds on segments that contribute nothing.
        if (start === 0xffff || start > end) continue;
        for (let c = start; c <= end; c += 1) out.add(c);
        work += end - start + 1;
        if (work >= BUDGET) return out;
      }
    } else if (format === 12) {
      if (table + 16 > buffer.length) continue;
      const groups = buffer.readUInt32BE(table + 12);
      for (let g = 0; g < groups; g += 1) {
        const rec = table + 16 + g * 12;
        if (rec + 12 > buffer.length) break;
        const start = buffer.readUInt32BE(rec);
        // UNCLAMPED, this was a hard OOM in a 68-byte file: one group record
        // claiming [0, 0xFFFFFFFF] drives 4.29 BILLION Set.add calls, 548MB RSS
        // and a V8 fatal abort under a constrained heap. Clamping to the real
        // Unicode ceiling costs nothing and removes the whole class.
        const end = Math.min(buffer.readUInt32BE(rec + 4), 0x10ffff);
        if (start > end || start > 0x10ffff) continue;
        for (let c = start; c <= end; c += 1) out.add(c);
        work += end - start + 1;
        if (work >= BUDGET) return out;
      }
    }
  }
  return out;
}

/** The characters verified present in the books' main font. */
export function loadMainFontCharset(root = defaultCurriculumRoot()): Set<string> {
  const parsed = JSON.parse(readFileSync(resolve(root, "core", "main-font-charset.json"), "utf8")) as {
    characters: { char: string }[];
  };
  if (!Array.isArray(parsed.characters) || parsed.characters.length === 0) {
    throw new Error("main-font-charset.json has no characters; refusing to report a clean glyph gate");
  }
  return new Set(parsed.characters.map((entry) => entry.char));
}
