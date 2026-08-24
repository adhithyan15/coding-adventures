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
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
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
import { DEFAULT_LINEARISABLE_TABLE_COLUMNS } from "./speech.js";

/** One independently mergeable narration manifest per language. */
export const NARRATION_HASH_MANIFEST_DIR = "core/generated-narration-hashes";

function manifestPath(language: string): string {
  if (!/^[a-z0-9-]+$/.test(language)) {
    throw new Error(`unsafe generated narration manifest language '${language}'`);
  }
  return `${NARRATION_HASH_MANIFEST_DIR}/${language}.json`;
}

interface GeneratedNarrationManifest {
  version: 1;
  algorithm: "fnv1a64";
  /** The width the export was generated at; a change here changes every file. */
  maxLinearisableTableColumns: number;
  chapters: Array<{
    language: string;
    chapter: number;
    /** Fingerprint of the chapter's lesson ASTs — the drift detector. */
    sourceHash: string;
    lessonIds: string[];
    voiceLessons: number;
    drivablePrefix: number;
    text: string;
    json: string;
    /** Fingerprint of what we generated, so a hand-edited export is caught too. */
    textHash: string;
    jsonHash: string;
  }>;
  findings: Array<{ code: string; lessonId: string; language: string; message: string }>;
}

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
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
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
  const policy = loadChapterPolicy(root) as { maxLinearisableTableColumns?: unknown };
  const value = policy.maxLinearisableTableColumns;
  if (value === undefined) return DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 16) {
    throw new Error(
      "chapter-policy.json: maxLinearisableTableColumns must be an integer from 0 through 16",
    );
  }
  return value;
}

/**
 * Build every narration file as an in-memory path -> content map.
 *
 * Pure enough to test: it reads the curriculum once through the loader and then does
 * no I/O of its own. `runNarrationGeneration` is the only thing that writes.
 */
export function narrationOutputs(root = defaultCurriculumRoot()): Map<string, string> {
  const maxLinearisableTableColumns = policyTableWidth(root);
  const lessons = loadLessons(root);

  // Chapter titles come from the HL05 ledgers when a track has authored one. A track
  // that has not is not defaulted to an invented title — `narrateChapter` says
  // "Chapter 7", which is honest, and the missing ledger stays visible as debt in the
  // gap report rather than being papered over here.
  const titles = new Map<string, string>();
  for (const track of loadTrackChapters(root)) {
    for (const chapter of track.chapters) {
      titles.set(`${track.language}/${chapter.chapter}`, chapter.title);
    }
  }
  const names = new Map<string, string>();
  for (const language of loadLanguageRegistry(root).languages) {
    names.set(language.id, language.name ?? language.id);
  }

  const chapters = narrationChapters(lessons, { maxLinearisableTableColumns, titles });
  const outputs = new Map<string, string>();
  const manifests = new Map<string, GeneratedNarrationManifest>();

  for (const chapter of chapters) {
    const slug = chapterSlug(chapter.chapter);
    const textPath = `${chapter.language}/narration/${slug}.txt`;
    const jsonPath = `${chapter.language}/narration/${slug}.json`;
    safeOutput(root, textPath);
    safeOutput(root, jsonPath);

    const text = renderChapterNarrationText(chapter, names.get(chapter.language));
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
      voiceLessons: chapter.lessons.filter((lesson) => lesson.modality === "voice").length,
      drivablePrefix: chapter.drivablePrefix,
      text: textPath,
      json: jsonPath,
      textHash: fnv1a64(text),
      jsonHash: fnv1a64(json),
    });
    manifest.findings.push(...chapter.findings);
  }

  for (const [language, manifest] of [...manifests].sort(([left], [right]) => left.localeCompare(right))) {
    manifest.chapters.sort((left, right) => left.chapter - right.chapter);
    manifest.findings.sort(
      (left, right) => left.lessonId.localeCompare(right.lessonId) || left.code.localeCompare(right.code),
    );
    outputs.set(manifestPath(language), `${JSON.stringify(manifest, null, 2)}\n`);
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
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    return 2;
  }
  let mismatch = false;
  for (const [relative, expected] of outputs) {
    const output = relative.startsWith(`${NARRATION_HASH_MANIFEST_DIR}/`)
      ? join(root, relative)
      : safeOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      continue;
    }
    const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
    if (actual !== expected) {
      process.stderr.write(`${relative}: generated narration is missing or stale\n`);
      mismatch = true;
    }
  }
  if (mode === "--write") {
    process.stdout.write(`generated ${outputs.size} narration file(s)\n`);
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runNarrationGeneration());
}
