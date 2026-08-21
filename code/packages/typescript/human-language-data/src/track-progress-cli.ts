import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
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
  renderTrackProgressCard,
  type GeneratedBookChapterRef,
} from "./track-progress.js";

export const TRACK_PROGRESS_DIR = "progress";

interface GeneratedBookManifest {
  version: number;
  chapters: GeneratedBookChapterRef[];
}

function loadGeneratedBookChapters(root: string): GeneratedBookChapterRef[] {
  const path = join(root, "core", "generated-book-hashes.json");
  const manifest = JSON.parse(readFileSync(path, "utf8")) as GeneratedBookManifest;
  if (manifest.version !== 1 || !Array.isArray(manifest.chapters)) {
    throw new Error("generated-book-hashes.json must declare version 1 and chapters[]");
  }
  return manifest.chapters;
}

function safeProgressOutput(root: string, relative: string): string {
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.startsWith(`${TRACK_PROGRESS_DIR}/`) ||
    !fromRoot.endsWith(".md")
  ) {
    throw new Error(`unsafe track progress output '${relative}'`);
  }
  return output;
}

/** Produce one deterministic progress card per language. */
export function generatedTrackProgressOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const tracks = buildTrackProgress(
    loadLanguageRegistry(root),
    loadLessons(root),
    loadLanguageCurricula(root),
    loadBookCorpus(root),
    loadGeneratedBookChapters(root),
  );
  return new Map(
    tracks.map((track) => {
      if (!/^[a-z0-9-]+$/.test(track.id)) {
        throw new Error(`unsafe language id '${track.id}' for progress card`);
      }
      const relative = `${TRACK_PROGRESS_DIR}/${track.id}.md`;
      safeProgressOutput(root, relative);
      return [relative, renderTrackProgressCard(track)];
    }),
  );
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

  const outputs = generatedTrackProgressOutputs(root);
  let mismatch = false;
  for (const [relative, expected] of outputs) {
    const output = safeProgressOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      process.stdout.write(`generated ${relative}\n`);
      continue;
    }
    const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
    if (actual !== expected) {
      process.stderr.write(`${relative}: generated track progress is missing or stale\n`);
      mismatch = true;
    }
  }

  if (mode === "--check") {
    const directory = resolve(root, TRACK_PROGRESS_DIR);
    const expectedNames = new Set([...outputs.keys()].map((path) => path.split("/").at(-1)!));
    const actualNames = existsSync(directory)
      ? readdirSync(directory).filter((name) => name.endsWith(".md"))
      : [];
    for (const name of actualNames) {
      if (!expectedNames.has(name)) {
        process.stderr.write(`${TRACK_PROGRESS_DIR}/${name}: unexpected stale progress card\n`);
        mismatch = true;
      }
    }
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runTrackProgress());
}
