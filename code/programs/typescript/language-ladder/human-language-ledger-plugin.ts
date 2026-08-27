import { join } from "node:path";
import type { Plugin } from "vite";
import {
  CURRICULUM_SECTIONS,
  isSharded,
  listShardNames,
  mergeMetaAndList,
  mergeSectionedShards,
  readLedgerFile,
  readShards,
  shardDirectoryFor,
} from "@coding-adventures/human-language-data/src/shard.ts";

export const LEDGER_INDEX_ID = "virtual:human-language-ledgers";
export const RESOLVED_LEDGER_INDEX_ID = `\0${LEDGER_INDEX_ID}`;
export const CURRICULUM_MODULE_PREFIX = "virtual:human-language-ledger/curriculum/";
export const CHAPTER_MODULE_PREFIX = "virtual:human-language-ledger/chapters/";

const TRACK_ID = /^[a-z][a-z0-9-]*$/;

export interface HumanLanguageLedgerPluginOptions {
  readonly curriculumRoot: string;
}

type WatchFile = (path: string) => void;

function resolvedChildId(prefix: string, id: string): string | null {
  if (!id.startsWith(prefix)) return null;
  const track = id.slice(prefix.length);
  return TRACK_ID.test(track) ? `\0${id}` : null;
}

/** Resolve a public virtual id, refusing path-like track ids. */
export function resolveHumanLanguageLedgerId(id: string): string | null {
  if (id === LEDGER_INDEX_ID) return RESOLVED_LEDGER_INDEX_ID;
  return (
    resolvedChildId(CURRICULUM_MODULE_PREFIX, id) ??
    resolvedChildId(CHAPTER_MODULE_PREFIX, id)
  );
}

function trackFromResolvedId(prefix: string, id: string): string | null {
  const resolvedPrefix = `\0${prefix}`;
  if (!id.startsWith(resolvedPrefix)) return null;
  const track = id.slice(resolvedPrefix.length);
  return TRACK_ID.test(track) ? track : null;
}

function watchShards(monolithPath: string, watch: WatchFile): void {
  const shardDir = shardDirectoryFor(monolithPath);
  for (const name of listShardNames(monolithPath)) watch(join(shardDir, name));
}

function moduleWithDefault(value: unknown): string {
  return `export default ${JSON.stringify(value)};\n`;
}

function loaderAssignments(
  name: "curriculumLoaders" | "chapterLoaders",
  prefix: string,
  tracks: readonly string[],
): string {
  return tracks
    .map(
      (track) =>
        `${name}[${JSON.stringify(track)}] = () => import(${JSON.stringify(`${prefix}${track}`)})` +
        `.then((module) => module.default);`,
    )
    .join("\n");
}

/**
 * Produce one browser module from the canonical shard directories.
 *
 * This function is exported so the boundary can be tested without starting a
 * Vite server. The Vite plugin below is deliberately only an adapter around it.
 */
export function loadHumanLanguageLedgerModule(
  id: string,
  curriculumRoot: string,
  watch: WatchFile,
): string | null {
  if (id === RESOLVED_LEDGER_INDEX_ID) {
    const registryPath = join(curriculumRoot, "core", "languages.json");
    watch(registryPath);
    const registry = readLedgerFile<{ languages: Array<{ id: string }> }>(registryPath);
    const tracks = registry.languages
      .map((track) => track.id)
      .filter((track) => {
        const curriculumPath = join(curriculumRoot, track, "curriculum.json");
        const chaptersPath = join(curriculumRoot, track, "chapters.json");
        return isSharded(curriculumPath) && isSharded(chaptersPath);
      });
    const spinePath = join(curriculumRoot, "core", "spine.json");
    watchShards(spinePath, watch);
    const spineShards = readShards(spinePath);
    if (spineShards === null) return null;
    const spine = mergeMetaAndList(spineShards, "nodes");

    return [
      `export const spine = ${JSON.stringify(spine)};`,
      "export const curriculumLoaders = Object.create(null);",
      loaderAssignments("curriculumLoaders", CURRICULUM_MODULE_PREFIX, tracks),
      "export const chapterLoaders = Object.create(null);",
      loaderAssignments("chapterLoaders", CHAPTER_MODULE_PREFIX, tracks),
      "",
    ].join("\n");
  }

  const curriculumTrack = trackFromResolvedId(CURRICULUM_MODULE_PREFIX, id);
  if (curriculumTrack !== null) {
    const path = join(curriculumRoot, curriculumTrack, "curriculum.json");
    if (!isSharded(path)) return null;
    watchShards(path, watch);
    const shards = readShards(path);
    if (shards === null) return null;
    return moduleWithDefault(mergeSectionedShards(shards, CURRICULUM_SECTIONS));
  }

  const chapterTrack = trackFromResolvedId(CHAPTER_MODULE_PREFIX, id);
  if (chapterTrack !== null) {
    const path = join(curriculumRoot, chapterTrack, "chapters.json");
    if (!isSharded(path)) return null;
    watchShards(path, watch);
    const shards = readShards(path);
    if (shards === null) return null;
    return moduleWithDefault(mergeMetaAndList(shards, "chapters"));
  }

  return null;
}

/** Build-time filesystem boundary for the browser-facing authored ledgers. */
export function humanLanguageLedgerPlugin(
  options: HumanLanguageLedgerPluginOptions,
): Plugin {
  return {
    name: "human-language-ledgers",
    configureServer(server) {
      // Individual files are registered by `load`, which covers edits. Watch
      // the external curriculum root as well so a newly added or deleted shard
      // invalidates the corresponding virtual module instead of requiring a
      // dev-server restart.
      server.watcher.add(options.curriculumRoot);
      server.watcher.on("all", (_event, changedPath) => {
        const normalized = changedPath.replaceAll("\\", "/");
        if (!normalized.includes(".d/") || !normalized.endsWith(".json")) return;

        const invalidate = (id: string) => {
          const module = server.moduleGraph.getModuleById(id);
          if (module !== undefined) server.moduleGraph.invalidateModule(module);
        };
        invalidate(RESOLVED_LEDGER_INDEX_ID);

        const curriculum = /\/([^/]+)\/curriculum\.d\//.exec(normalized)?.[1];
        if (curriculum !== undefined) invalidate(`\0${CURRICULUM_MODULE_PREFIX}${curriculum}`);
        const chapters = /\/([^/]+)\/chapters\.d\//.exec(normalized)?.[1];
        if (chapters !== undefined) invalidate(`\0${CHAPTER_MODULE_PREFIX}${chapters}`);
      });
    },
    resolveId(id) {
      return resolveHumanLanguageLedgerId(id);
    },
    load(id) {
      return loadHumanLanguageLedgerModule(
        id,
        options.curriculumRoot,
        (path) => this.addWatchFile(path),
      );
    },
  };
}
