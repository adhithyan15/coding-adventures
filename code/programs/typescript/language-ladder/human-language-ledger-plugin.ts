import { realpathSync } from "node:fs";
import {
  dirname,
  isAbsolute,
  join,
  normalize,
  relative,
  resolve,
} from "node:path";
import type { Plugin } from "vite";
// This plugin is imported while Vite loads its config. Keep this source import
// relative so Vite bundles the shared validator and resolves its NodeNext
// `.js` specifiers to the adjacent `.ts` sources; package imports are
// externalized during config loading and would be handed straight to Node
// before the human-language-data package has been compiled.
import {
  listGeneratedBookHashOwnerLanguages,
  readGeneratedBookHashManifest,
} from "../../../packages/typescript/human-language-data/src/generated-hash-shards.ts";
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
export const CURRICULUM_MODULE_PREFIX =
  "virtual:human-language-ledger/curriculum/";
export const CHAPTER_MODULE_PREFIX = "virtual:human-language-ledger/chapters/";
export const BOOK_HASH_MODULE_PREFIX =
  "virtual:human-language-ledger/book-hashes/";

const TRACK_ID = /^[a-z][a-z0-9-]*$/;

export interface HumanLanguageLedgerPluginOptions {
  readonly curriculumRoot: string;
}

type WatchFile = (path: string) => void;

function safeLedgerPath(curriculumRoot: string, ...parts: string[]): string {
  const path = join(curriculumRoot, ...parts);
  const realRoot = realpathSync(curriculumRoot);
  const realParent = realpathSync(dirname(path));
  const inside = normalize(relative(realRoot, realParent)).replaceAll(
    "\\",
    "/",
  );
  if (inside === ".." || inside.startsWith("../") || isAbsolute(inside)) {
    throw new Error(
      `human-language-ledgers: ledger parent escapes curriculum root: ${path}`,
    );
  }
  return path;
}

function validatedRegistryTracks(registry: {
  languages: Array<{ id: string }>;
}): string[] {
  const tracks: string[] = [];
  const seen = new Set<string>();
  for (const entry of registry.languages) {
    if (!TRACK_ID.test(entry.id)) {
      throw new Error(
        `human-language-ledgers: unsafe registry track id ${JSON.stringify(entry.id)}`,
      );
    }
    if (seen.has(entry.id)) {
      throw new Error(
        `human-language-ledgers: duplicate registry track id ${JSON.stringify(entry.id)}`,
      );
    }
    seen.add(entry.id);
    tracks.push(entry.id);
  }
  return tracks;
}

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
    resolvedChildId(CHAPTER_MODULE_PREFIX, id) ??
    resolvedChildId(BOOK_HASH_MODULE_PREFIX, id)
  );
}

function trackFromResolvedId(prefix: string, id: string): string | null {
  const resolvedPrefix = `\0${prefix}`;
  if (!id.startsWith(resolvedPrefix)) return null;
  const track = id.slice(resolvedPrefix.length);
  return TRACK_ID.test(track) ? track : null;
}

function watchShards(
  curriculumRoot: string,
  monolithPath: string,
  watch: WatchFile,
): void {
  // `readShards` refuses a symlink at the shard directory and its children.
  // This separate parent check closes the remaining `track -> outside` gap.
  safeLedgerPath(curriculumRoot, relative(curriculumRoot, monolithPath));
  const shardDir = shardDirectoryFor(monolithPath);
  for (const name of listShardNames(monolithPath)) watch(join(shardDir, name));
}

function moduleWithDefault(value: unknown): string {
  return `export default ${JSON.stringify(value)};\n`;
}

function loaderAssignments(
  name: "curriculumLoaders" | "chapterLoaders" | "bookHashLoaders",
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
 * Discover the bounded set of per-language book owners and refuse owners that
 * are not in the canonical registry.
 *
 * Iterating only registry ids makes a plausible `ghost.d/` invisible forever:
 * no virtual child imports it, so neither its malformed contents nor its
 * unowned language can fail the build. The shared discovery boundary validates
 * top-level names/types/symlinks and each owner's strict chapter shards. This
 * without reading any chapter bytes. Each virtual child validates its own
 * strict chapter shards when Vite builds it. This cross-check closes the
 * remaining semantic gap without putting chapter keys in the emitted index:
 * the returned table is still one key per registered track.
 */
export function validatedBookHashTracks(
  curriculumRoot: string,
  registryTracks: readonly string[],
): string[] {
  const registered = new Set(registryTracks);
  const discovered = listGeneratedBookHashOwnerLanguages(curriculumRoot);
  for (const language of discovered) {
    if (!registered.has(language)) {
      throw new Error(
        `human-language-ledgers: generated book hash owner ${JSON.stringify(`${language}.d`)} is not registered`,
      );
    }
  }
  const ownerLanguages = new Set(discovered);
  for (const track of registryTracks) {
    if (!ownerLanguages.has(track)) {
      throw new Error(
        `human-language-ledgers: registered language ${JSON.stringify(track)} has no generated book hash owner`,
      );
    }
  }
  return [...registryTracks];
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
    const registryPath = safeLedgerPath(
      curriculumRoot,
      "core",
      "languages.json",
    );
    watch(registryPath);
    const registry = readLedgerFile<{ languages: Array<{ id: string }> }>(
      registryPath,
    );
    const registryTracks = validatedRegistryTracks(registry);
    const tracks = registryTracks.filter((track) => {
      const curriculumPath = safeLedgerPath(
        curriculumRoot,
        track,
        "curriculum.json",
      );
      const chaptersPath = safeLedgerPath(
        curriculumRoot,
        track,
        "chapters.json",
      );
      return isSharded(curriculumPath) && isSharded(chaptersPath);
    });
    const bookHashTracks = validatedBookHashTracks(
      curriculumRoot,
      registryTracks,
    );
    const spinePath = safeLedgerPath(curriculumRoot, "core", "spine.json");
    watchShards(curriculumRoot, spinePath, watch);
    const spineShards = readShards(spinePath);
    if (spineShards === null) return null;
    const spine = mergeMetaAndList(spineShards, "nodes");

    return [
      `export const spine = ${JSON.stringify(spine)};`,
      "export const curriculumLoaders = Object.create(null);",
      loaderAssignments("curriculumLoaders", CURRICULUM_MODULE_PREFIX, tracks),
      "export const chapterLoaders = Object.create(null);",
      loaderAssignments("chapterLoaders", CHAPTER_MODULE_PREFIX, tracks),
      "export const bookHashLoaders = Object.create(null);",
      loaderAssignments(
        "bookHashLoaders",
        BOOK_HASH_MODULE_PREFIX,
        bookHashTracks,
      ),
      "",
    ].join("\n");
  }

  const curriculumTrack = trackFromResolvedId(CURRICULUM_MODULE_PREFIX, id);
  if (curriculumTrack !== null) {
    const path = safeLedgerPath(
      curriculumRoot,
      curriculumTrack,
      "curriculum.json",
    );
    if (!isSharded(path)) return null;
    watchShards(curriculumRoot, path, watch);
    const shards = readShards(path);
    if (shards === null) return null;
    return moduleWithDefault(mergeSectionedShards(shards, CURRICULUM_SECTIONS));
  }

  const chapterTrack = trackFromResolvedId(CHAPTER_MODULE_PREFIX, id);
  if (chapterTrack !== null) {
    const path = safeLedgerPath(curriculumRoot, chapterTrack, "chapters.json");
    if (!isSharded(path)) return null;
    watchShards(curriculumRoot, path, watch);
    const shards = readShards(path);
    if (shards === null) return null;
    return moduleWithDefault(mergeMetaAndList(shards, "chapters"));
  }

  const bookHashTrack = trackFromResolvedId(BOOK_HASH_MODULE_PREFIX, id);
  if (bookHashTrack !== null) {
    const registryPath = safeLedgerPath(
      curriculumRoot,
      "core",
      "languages.json",
    );
    watch(registryPath);
    const registry = readLedgerFile<{ languages: Array<{ id: string }> }>(
      registryPath,
    );
    if (!validatedRegistryTracks(registry).includes(bookHashTrack)) {
      throw new Error(
        `human-language-ledgers: book hash track ${JSON.stringify(bookHashTrack)} is not registered`,
      );
    }
    const path = safeLedgerPath(
      curriculumRoot,
      "core",
      "generated-book-hashes",
      `${bookHashTrack}.json`,
    );
    if (!isSharded(path)) return null;
    const loaded = readGeneratedBookHashManifest(path);
    for (const sourcePath of loaded.sourcePaths) watch(sourcePath);
    return moduleWithDefault(loaded.manifest);
  }

  return null;
}

/** Virtual modules affected by an authored shard path, excluding path escapes. */
export function ledgerModuleIdsForShardPath(
  changedPath: string,
  curriculumRoot: string,
): string[] {
  const inside = normalize(
    relative(resolve(curriculumRoot), resolve(changedPath)),
  ).replaceAll("\\", "/");
  if (inside === ".." || inside.startsWith("../") || isAbsolute(inside))
    return [];
  if (/^core\/generated-book-hashes\/[^/]+$/.test(inside)) {
    // A direct child may be a newly resurrected monolith, an unregistered
    // owner directory, or a malformed entry. None belongs to a per-language
    // child yet, but the strict name-only index discovery must see it.
    return [RESOLVED_LEDGER_INDEX_ID];
  }
  if (!inside.includes(".d/") || !inside.endsWith(".json")) return [];

  const ids = [RESOLVED_LEDGER_INDEX_ID];
  const curriculum = /^([^/]+)\/curriculum\.d\//.exec(inside)?.[1];
  if (curriculum !== undefined && TRACK_ID.test(curriculum)) {
    ids.push(`\0${CURRICULUM_MODULE_PREFIX}${curriculum}`);
  }
  const chapters = /^([^/]+)\/chapters\.d\//.exec(inside)?.[1];
  if (chapters !== undefined && TRACK_ID.test(chapters)) {
    ids.push(`\0${CHAPTER_MODULE_PREFIX}${chapters}`);
  }
  const bookHashes = /^core\/generated-book-hashes\/([^/]+)\.d\//.exec(
    inside,
  )?.[1];
  if (bookHashes !== undefined && TRACK_ID.test(bookHashes)) {
    ids.push(`\0${BOOK_HASH_MODULE_PREFIX}${bookHashes}`);
  }
  return ids;
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
      server.watcher.on("all", (event, changedPath) => {
        const ids = ledgerModuleIdsForShardPath(
          changedPath,
          options.curriculumRoot,
        );
        if (ids.length === 0) return;
        for (const id of ids) {
          const module = server.moduleGraph.getModuleById(id);
          if (module != null) server.moduleGraph.invalidateModule(module);
        }
        // A newly added shard has no prior `addWatchFile` association, and an
        // unlinked one has just lost it. Vite therefore has no hot-update
        // module set for either event; explicitly refresh the browser.
        if (event === "add" || event === "unlink") {
          server.ws.send({ type: "full-reload", path: "*" });
        }
      });
    },
    resolveId(id) {
      return resolveHumanLanguageLedgerId(id);
    },
    load(id) {
      return loadHumanLanguageLedgerModule(id, options.curriculumRoot, (path) =>
        this.addWatchFile(path),
      );
    },
  };
}
