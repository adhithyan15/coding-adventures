import { lstatSync } from "node:fs";
import {
  normalize,
  relative as pathRelative,
  resolve,
} from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { listGeneratedBookHashManifests } from "./generated-hash-shards.js";
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
  renderTrackProgressTable,
  type GeneratedBookChapterRef,
  type TrackProgress,
} from "./track-progress.js";

export const TRACK_PROGRESS_DIR = "progress";

export function loadGeneratedBookChapterRefs(
  root: string,
  registeredLanguages: ReadonlySet<string>,
): GeneratedBookChapterRef[] {
  return listGeneratedBookHashManifests(root).flatMap(
    ({ language, manifest }) => {
      if (!registeredLanguages.has(language)) {
        throw new Error(
          `generated book hash owner '${language}' is not a registered language`,
        );
      }
      return manifest.chapters;
    },
  );
}

function safeProgressOutput(root: string, relative: string): string {
  assertRelativeManifestPath(
    relative,
    `unsafe track progress output '${relative}'`,
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
    !fromRoot.startsWith(`${TRACK_PROGRESS_DIR}/`) ||
    !fromRoot.endsWith(".md")
  ) {
    throw new Error(`unsafe track progress output '${relative}'`);
  }
  return output;
}

/** Derive the registry-ordered progress rows shared by every presentation. */
function generatedTrackProgress(
  root = defaultCurriculumRoot(),
): TrackProgress[] {
  const registry = loadLanguageRegistry(root);
  return buildTrackProgress(
    registry,
    loadLessons(root),
    loadLanguageCurricula(root),
    loadBookCorpus(root),
    loadGeneratedBookChapterRefs(
      root,
      new Set(registry.languages.map((language) => language.id)),
    ),
  );
}

function progressOutputs(
  tracks: TrackProgress[],
  root: string,
): Map<string, string> {
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

/** Produce one deterministic in-memory progress card per language. */
export function generatedTrackProgressOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  return progressOutputs(generatedTrackProgress(root), root);
}

/** Render the same rows as one on-demand Markdown report. */
export function generatedTrackProgressReport(
  root = defaultCurriculumRoot(),
): string {
  return `${renderTrackProgressTable(generatedTrackProgress(root))}\n`;
}

/** lstat-based resurrection guard: false for files, directories, and dangling links. */
export function progressDirectoryIsAbsent(root: string): boolean {
  try {
    lstatSync(resolve(root, TRACK_PROGRESS_DIR));
    return false;
  } catch (error) {
    if (
      typeof error === "object" &&
      error !== null &&
      "code" in error &&
      error.code === "ENOENT"
    ) {
      return true;
    }
    throw error;
  }
}

export function runTrackProgress(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--report") {
    process.stderr.write("usage: track-progress-cli (--check | --report)\n");
    return 2;
  }

  const tracks = generatedTrackProgress(root);
  if (mode === "--report") {
    process.stdout.write(`${renderTrackProgressTable(tracks)}\n`);
    return 0;
  }

  // Building the map validates every registry-owned output identity and keeps
  // the exact per-track projection exercised without persisting it. The whole
  // old directory is forbidden categorically: lstat sees regular files,
  // directories, real links, and dangling links, so none can resurrect the
  // high-churn generated cards behind an existsSync false negative.
  progressOutputs(tracks, root);
  if (!progressDirectoryIsAbsent(root)) {
    process.stderr.write(
      `${TRACK_PROGRESS_DIR}: tracked progress projections must remain absent; run npm run report:progress for the on-demand table\n`,
    );
    return 1;
  }
  return 0;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  process.exit(runTrackProgress());
}
