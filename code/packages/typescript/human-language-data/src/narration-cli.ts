// The narration export's filesystem shell — `--write` and `--check`.
//
// This file is deliberately a near-twin of `book-cli.ts`, and the resemblance is the
// point. The generated `.tex` chapters solved exactly this problem once already: a
// derived artefact, committed to the repository, that must never quietly disagree
// with the lessons it came from. Rather than invent a second discipline for
// narration, this reuses the first one, joint for joint:
//
//   `narrationOutputs()`   builds a path -> content map with no I/O at all, so the
//                          whole export is testable without touching a disk.
//   `safeOutput()`         fail-closes on any path that escapes the curriculum root
//                          or does not end in the extension we intend to write.
//   `--check`              compares byte for byte and exits 1 on any difference.
//   the hash manifest      records `fnv1a64` of each chapter's lesson AST, so drift
//                          is visible as a one-line diff even in a 500-file export.
//
// ---------------------------------------------------------------------------
// Why the manifest matters more here than for the book
// ---------------------------------------------------------------------------
//
// A stale `.tex` file produces a book that looks wrong to anyone who opens it. A
// stale narration file produces a *voice assistant confidently teaching a lesson that
// no longer exists* — to someone who is driving and cannot check. The manifest turns
// "someone forgot to re-run the exporter" from an invisible failure into a failing
// build.
//
// ---------------------------------------------------------------------------
// Two files per chapter
// ---------------------------------------------------------------------------
//
//   <language>/narration/ch<NN>.txt    the continuous script, for "read me this"
//   <language>/narration/ch<NN>.json   the structured segments, for a voice agent
//                                      that must pause, listen, and score
//
// The `.txt` is not stored inside the `.json`. It is entirely derivable from the
// segments — `renderChapterNarrationText` does exactly that — and storing it twice
// would double the committed bytes to no benefit while creating a second thing that
// can go stale.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import {
  dirname,
  join,
  normalize,
  relative as pathRelative,
  resolve,
} from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { pathToFileURL } from "node:url";
import { fnv1a64 } from "./hash.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadLanguageRegistry,
  loadLessons,
  loadTrackChapters,
} from "./loader.js";
import {
  narrationChapters,
  renderChapterNarrationText,
  type ChapterNarration,
} from "./narration.js";
import { narrationCacheFor, type NarrationCache } from "./narration-cache.js";
import type { ParsedLesson } from "./parse.js";
import { DEFAULT_LINEARISABLE_TABLE_COLUMNS } from "./speech.js";
import {
  GENERATED_NARRATION_HASH_DIR,
  assertGeneratedHashOwnerNames,
  generatedNarrationHashOwnerContents,
  prepareGeneratedHashOwnerWrite,
  type GeneratedNarrationHashManifest,
} from "./generated-hash-shards.js";

/** One independently mergeable narration manifest per language. */
export const NARRATION_HASH_MANIFEST_DIR = GENERATED_NARRATION_HASH_DIR;

/**
 * Resolve a curriculum-root-relative output path, refusing anything that escapes.
 *
 * Lifted from `book-cli.ts` with the extension check widened to the two we write.
 * The check is on the *normalized, root-relative* form, so `../`, absolute paths, and
 * symlink-flavoured tricks all collapse into something that either starts with `..`
 * or is empty — both rejected. Language slugs come from directory names on disk, so
 * this is defence in depth rather than the only guard, which is exactly how it should
 * be for a function that writes files.
 */
export function safeOutput(root: string, relative: string): string {
  assertRelativeManifestPath(
    relative,
    `unsafe generated narration output '${relative}'`,
  );
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll(
    "\\",
    "/",
  );
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !(fromRoot.endsWith(".txt") || fromRoot.endsWith(".json"))
  ) {
    throw new Error(`unsafe generated narration output '${relative}'`);
  }
  return output;
}

/** `ch07` — zero-padded so a directory listing sorts the way a reader reads. */
function chapterSlug(chapter: number): string {
  const whole = Math.trunc(chapter);
  return `ch${String(whole).padStart(2, "0")}`;
}

/**
 * Read `maxLinearisableTableColumns` out of the chapter policy.
 *
 * The policy file is authored JSON, so the value is validated rather than trusted: a
 * non-integer or negative width would silently reshape every lesson's modality. An
 * absent value falls back to the lineariser's measured default, which keeps older
 * checkouts of the policy file working.
 */
export function policyTableWidth(root: string): number {
  const policy = loadChapterPolicy(root) as {
    maxLinearisableTableColumns?: unknown;
  };
  const value = policy.maxLinearisableTableColumns;
  if (value === undefined) return DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < 0 ||
    value > 16
  ) {
    throw new Error(
      "chapter-policy.json: maxLinearisableTableColumns must be an integer from 0 through 16",
    );
  }
  return value;
}

/**
 * Chapter titles from the HL05 ledgers, keyed `"<language>/<chapter>"`.
 *
 * A track that has not authored a ledger is not defaulted to an invented title —
 * `narrateChapter` says "Chapter 7", which is honest, and the missing ledger stays
 * visible as debt in the gap report rather than being papered over here.
 */
export function chapterTitles(root: string): Map<string, string> {
  const titles = new Map<string, string>();
  for (const track of loadTrackChapters(root)) {
    for (const chapter of track.chapters) {
      titles.set(`${track.language}/${chapter.chapter}`, chapter.title);
    }
  }
  return titles;
}

/** What the export narrates with: the policy's table width and the authored titles. */
export interface CorpusNarrationOptions {
  maxLinearisableTableColumns: number;
  titles: ReadonlyMap<string, string>;
}

/**
 * `narrationChapters(lessons, options)`, answered track by track from `cache`.
 *
 * Without a cache — every CLI process, every fixture root — `corpusNarration`
 * never comes here; it calls `narrationChapters` once, as it always did. With
 * one, the corpus is cut into tracks, each track is looked up under its lesson
 * key (see `src/narration-cache.ts`), and only the tracks that miss are
 * narrated. A track the loader reported no key for is narrated uncached.
 *
 * Why cutting by track returns exactly what one whole-corpus call returns:
 *
 *   - `narrationChapters` groups by `"<language>/<chapter>"`, so no chapter ever
 *     mixes two tracks, and each track's chapters see the same lessons, in the
 *     same relative order, either way.
 *   - It sorts by language with `localeCompare`, then by chapter number. Sorting
 *     the tracks by that same `localeCompare` and concatenating each track's
 *     (already chapter-sorted) result is that same order...
 *   - ...provided no two distinct track ids compare EQUAL, which would let the
 *     whole-corpus sort interleave their chapters by number, and no id holds the
 *     `/` that separates it from the chapter in the grouping key. No real id
 *     does either, but the claim is checked rather than assumed: if it ever
 *     fails, the cache is simply not used for this call.
 *
 *     tracks:   french (2 ch)   spanish (3 ch)
 *     whole:    french/1 french/2 spanish/1 spanish/2 spanish/3
 *     by track: [french/1 french/2] ++ [spanish/1 spanish/2 spanish/3]   (same)
 *
 * Not exported, deliberately: `lessonKeys` must be the keys the loader reported
 * for THESE lessons, and the only caller that can promise that is
 * `corpusNarration`, which loaded them a moment ago. `tests/narration-cache.test.ts`
 * pins the equality through `corpusNarration` on a multi-track fixture, and the
 * `--check` gate compares every byte the cached chapters produce with files
 * generated without the cache.
 */
function narrateCorpusByTrack(
  lessons: readonly ParsedLesson[],
  options: CorpusNarrationOptions,
  cache: NarrationCache,
  lessonKeys: ReadonlyMap<string, string>,
): ChapterNarration[] {
  const tracks = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    const own = tracks.get(lesson.language);
    if (own) own.push(lesson);
    else tracks.set(lesson.language, [lesson]);
  }
  const order = [...tracks.keys()].sort((left, right) => left.localeCompare(right));
  // The two assumptions above, checked: a `/` in an id would let the whole-corpus
  // key `"<language>/<chapter>"` split somewhere else, and an equal pair would let
  // its sort interleave two tracks. Loader ids can do neither; any other input that
  // does is narrated the plain way.
  const separable = order.every(
    (language, index) =>
      !language.includes("/") &&
      (index === 0 || order[index - 1]!.localeCompare(language) !== 0),
  );
  if (!separable) return narrationChapters(lessons, options);
  const chapters: ChapterNarration[] = [];
  for (const language of order) {
    const own = tracks.get(language)!;
    const lessonKey = lessonKeys.get(language);
    if (lessonKey === undefined) {
      chapters.push(...narrationChapters(own, options));
      continue;
    }
    const key = cache.key({ language, ...options, lessonKey });
    let narrated = cache.read(language, key);
    if (narrated === undefined) {
      narrated = narrationChapters(own, options);
      cache.write(language, key, narrated);
    }
    chapters.push(...narrated);
  }
  return chapters;
}

/**
 * The whole curriculum, narrated exactly as the export narrates it.
 *
 * The one place the export's inputs are gathered: the policy width (validated
 * FIRST, so a bad policy fails before the corpus is read), every lesson, and the
 * authored titles. `narrationOutputs` renders this; `tests/narration.test.ts`
 * inspects it. Inside the test suite, for the real curriculum root only, the
 * chapters come through the narration cache, so the suite narrates the corpus
 * once per run instead of once per file that asks.
 */
export function corpusNarration(root = defaultCurriculumRoot()): ChapterNarration[] {
  const maxLinearisableTableColumns = policyTableWidth(root);
  const cache = narrationCacheFor(root);
  const lessonKeys = new Map<string, string>();
  const lessons = loadLessons(
    root,
    cache === undefined ? undefined : (language, key) => lessonKeys.set(language, key),
  );
  const titles = chapterTitles(root);
  const options = { maxLinearisableTableColumns, titles };
  return cache === undefined
    ? narrationChapters(lessons, options)
    : narrateCorpusByTrack(lessons, options, cache, lessonKeys);
}

/**
 * Build every narration file as an in-memory path -> content map.
 *
 * Pure enough to test: it reads the curriculum once through the loader and then does
 * no I/O of its own. `runNarrationGeneration` is the only thing that writes.
 */
export function narrationOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const maxLinearisableTableColumns = policyTableWidth(root);
  const chapters = corpusNarration(root);
  const names = new Map<string, string>();
  for (const language of loadLanguageRegistry(root).languages) {
    names.set(language.id, language.name ?? language.id);
  }

  const outputs = new Map<string, string>();
  const manifests = new Map<string, GeneratedNarrationHashManifest>();

  for (const chapter of chapters) {
    const slug = chapterSlug(chapter.chapter);
    const textPath = `${chapter.language}/narration/${slug}.txt`;
    const jsonPath = `${chapter.language}/narration/${slug}.json`;
    safeOutput(root, textPath);
    safeOutput(root, jsonPath);

    const text = renderChapterNarrationText(
      chapter,
      names.get(chapter.language),
    );
    const json = `${JSON.stringify(serializeChapter(chapter), null, 2)}\n`;
    outputs.set(textPath, text);
    outputs.set(jsonPath, json);
    let manifest = manifests.get(chapter.language);
    if (!manifest) {
      manifest = {
        version: 1,
        algorithm: "fnv1a64",
        maxLinearisableTableColumns,
        chapters: [],
        findings: [],
      };
      manifests.set(chapter.language, manifest);
    }
    manifest.chapters.push({
      language: chapter.language,
      chapter: chapter.chapter,
      sourceHash: chapter.sourceHash,
      lessonIds: chapter.lessonIds,
      voiceLessons: chapter.lessons.filter(
        (lesson) => lesson.modality === "voice",
      ).length,
      drivablePrefix: chapter.drivablePrefix,
      text: textPath,
      json: jsonPath,
      textHash: fnv1a64(text),
      jsonHash: fnv1a64(json),
    });
    manifest.findings.push(...chapter.findings);
  }

  for (const [language, manifest] of [...manifests].sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    manifest.chapters.sort((left, right) => left.chapter - right.chapter);
    manifest.findings.sort(
      (left, right) =>
        left.lessonId.localeCompare(right.lessonId) ||
        left.code.localeCompare(right.code),
    );
    for (const [relative, content] of generatedNarrationHashOwnerContents(
      language,
      manifest,
    )) {
      outputs.set(relative, content);
    }
  }
  return outputs;
}

/**
 * The JSON shape a voice agent consumes.
 *
 * Written out field by field rather than `JSON.stringify(chapter)` so that adding a
 * field to an internal interface cannot silently change a committed export — and so
 * that the file is a documented contract rather than an accident of the type layout.
 */
function serializeChapter(chapter: ChapterNarration): unknown {
  return {
    version: 1,
    language: chapter.language,
    chapter: chapter.chapter,
    title: chapter.title,
    drivablePrefix: chapter.drivablePrefix,
    sourceHash: chapter.sourceHash,
    lessons: chapter.lessons.map((lesson) => ({
      id: lesson.lessonId,
      sequence: lesson.sequence,
      title: lesson.title,
      headword: lesson.headword,
      romanization: lesson.romanization,
      gloss: lesson.gloss,
      script: lesson.script,
      modality: lesson.modality,
      derivedModality: lesson.derivedModality,
      modalityReasons: lesson.modalityReasons,
      sourceHash: lesson.sourceHash,
      notice: lesson.notice,
      blocks: lesson.blocks,
    })),
    findings: chapter.findings,
  };
}

export function runNarrationGeneration(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: narration-cli (--check | --write)\n");
    return 2;
  }
  let outputs: Map<string, string>;
  try {
    outputs = narrationOutputs(root);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    return 2;
  }
  let mismatch = false;
  if (mode === "--check") {
    try {
      assertGeneratedHashOwnerNames(
        root,
        NARRATION_HASH_MANIFEST_DIR,
        outputs.keys(),
      );
    } catch (error) {
      process.stderr.write(
        `${error instanceof Error ? error.message : String(error)}\n`,
      );
      return 1;
    }
  }
  for (const [relative, expected] of outputs) {
    const hashOwner = relative.startsWith(`${NARRATION_HASH_MANIFEST_DIR}/`);
    const output = hashOwner
      ? mode === "--write"
        ? prepareGeneratedHashOwnerWrite(
            root,
            NARRATION_HASH_MANIFEST_DIR,
            relative,
          )
        : join(root, relative)
      : safeOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      continue;
    }
    const actual = existsSync(output)
      ? readFileSync(output, "utf8")
      : undefined;
    if (actual !== expected) {
      process.stderr.write(
        `${relative}: generated narration is missing or stale\n`,
      );
      mismatch = true;
    }
  }
  if (mode === "--write") {
    process.stdout.write(`generated ${outputs.size} narration file(s)\n`);
  }
  return mismatch ? 1 : 0;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  process.exitCode = runNarrationGeneration();
}
