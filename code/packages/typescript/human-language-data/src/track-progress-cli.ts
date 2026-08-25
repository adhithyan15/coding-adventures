import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { readLedgerFile } from "./shard.js";
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
  const directory = join(root, "core", "generated-book-hashes");
  const chapters: GeneratedBookChapterRef[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })
    .filter((candidate) => candidate.isFile() && candidate.name.endsWith(".json"))
    .sort((left, right) => left.name.localeCompare(right.name))) {
    const language = entry.name.slice(0, -".json".length);
    // `readLedgerFile`, not a bare parse. `core/generated-book-hashes/` is a
    // plain per-language directory rather than an HL21 `X.d/`, so there is no
    // monolith here to go stale — but the symlink refusal, the dangerous-key
    // rejection and the parse-error scrubbing apply to any repo JSON, and the
    // `.sort()` above already treats these files exactly as shards are treated.
    const manifest = readLedgerFile<GeneratedBookManifest>(join(directory, entry.name));
    if (manifest.version !== 1 || !Array.isArray(manifest.chapters)) {
      throw new Error(`${entry.name} must declare version 1 and chapters[]`);
    }
    if (manifest.chapters.some((chapter) => chapter.language !== language)) {
      throw new Error(`${entry.name} may contain only ${language} chapters`);
    }
    chapters.push(...manifest.chapters);
  }
  return chapters;
}

function safeProgressOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe track progress output '${relative}'`);
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
