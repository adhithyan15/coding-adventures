import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot,
  loadBookCorpus,
  loadLanguageCurricula,
  loadLanguageRegistry,
  loadLessons,
} from "./loader.js";
import {
  buildTrackProgress,
  renderTrackProgressTable,
  type GeneratedBookChapterRef,
} from "./track-progress.js";

export const TRACK_PROGRESS_START = "<!-- BEGIN GENERATED TRACK PROGRESS -->";
export const TRACK_PROGRESS_END = "<!-- END GENERATED TRACK PROGRESS -->";

interface GeneratedBookManifest {
  version: number;
  chapters: GeneratedBookChapterRef[];
}

/** Replace exactly one marked section while preserving the file's newline style. */
export function replaceTrackProgressSection(source: string, table: string): string {
  const start = source.indexOf(TRACK_PROGRESS_START);
  const end = source.indexOf(TRACK_PROGRESS_END);
  if (
    start < 0 ||
    end < 0 ||
    end <= start ||
    source.lastIndexOf(TRACK_PROGRESS_START) !== start ||
    source.lastIndexOf(TRACK_PROGRESS_END) !== end
  ) {
    throw new Error("README.md must contain exactly one ordered track-progress marker pair");
  }
  const newline = source.includes("\r\n") ? "\r\n" : "\n";
  const rendered = table.replaceAll("\n", newline);
  return `${source.slice(0, start + TRACK_PROGRESS_START.length)}${newline}${rendered}${newline}${source.slice(end)}`;
}

function loadGeneratedBookChapters(root: string): GeneratedBookChapterRef[] {
  const path = join(root, "core", "generated-book-hashes.json");
  const manifest = JSON.parse(readFileSync(path, "utf8")) as GeneratedBookManifest;
  if (manifest.version !== 1 || !Array.isArray(manifest.chapters)) {
    throw new Error("generated-book-hashes.json must declare version 1 and chapters[]");
  }
  return manifest.chapters;
}

/** Produce the complete expected README without mutating the filesystem. */
export function generatedTrackProgressReadme(root = defaultCurriculumRoot()): string {
  const readmePath = join(root, "README.md");
  const source = readFileSync(readmePath, "utf8");
  const tracks = buildTrackProgress(
    loadLanguageRegistry(root),
    loadLessons(root),
    loadLanguageCurricula(root),
    loadBookCorpus(root),
    loadGeneratedBookChapters(root),
  );
  return replaceTrackProgressSection(source, renderTrackProgressTable(tracks));
}

export function runTrackProgress(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: track-progress-cli (--check | --write)\n");
    return 2;
  }
  const output = join(root, "README.md");
  const expected = generatedTrackProgressReadme(root);
  if (mode === "--write") {
    writeFileSync(output, expected, "utf8");
    process.stdout.write("generated README.md track progress\n");
    return 0;
  }
  const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
  if (actual !== expected) {
    process.stderr.write("README.md: generated track progress is stale\n");
    return 1;
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runTrackProgress());
}
