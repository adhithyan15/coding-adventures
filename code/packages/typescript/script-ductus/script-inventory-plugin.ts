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
import {
  mergeScriptInventoryShards,
  readShards,
  shardDirectoryFor,
} from "@coding-adventures/human-language-data/src/shard.ts";

export const SCRIPT_INVENTORIES_ID = "virtual:script-ductus-inventories";
export const RESOLVED_SCRIPT_INVENTORIES_ID = `\0${SCRIPT_INVENTORIES_ID}`;

const INVENTORIES = [
  { id: "japanese", exportName: "japanese" },
  { id: "perso-arabic", exportName: "persoArabic" },
  { id: "tamil", exportName: "tamil" },
  { id: "urdu-nastaliq", exportName: "urduNastaliq" },
] as const;
const INVENTORY_IDS = new Set<string>(INVENTORIES.map(({ id }) => id));

export interface ScriptInventoryPluginOptions {
  readonly curriculumRoot: string;
}

type WatchFile = (path: string) => void;

/** Keep even fixed build-time paths inside the configured curriculum root. */
function safeInventoryPath(curriculumRoot: string, inventory: string): string {
  if (!INVENTORY_IDS.has(inventory)) {
    throw new Error(
      `script-ductus-inventories: unknown inventory ${JSON.stringify(inventory)}`,
    );
  }
  const path = join(curriculumRoot, "data", "scripts", `${inventory}.json`);
  const realRoot = realpathSync(curriculumRoot);
  const realParent = realpathSync(dirname(path));
  const inside = normalize(relative(realRoot, realParent)).replaceAll(
    "\\",
    "/",
  );
  if (inside === ".." || inside.startsWith("../") || isAbsolute(inside)) {
    throw new Error(
      `script-ductus-inventories: inventory parent escapes curriculum root: ${path}`,
    );
  }
  return path;
}

/** Resolve the single public virtual id; path-like suffixes are never accepted. */
export function resolveScriptInventoryId(id: string): string | null {
  return id === SCRIPT_INVENTORIES_ID ? RESOLVED_SCRIPT_INVENTORIES_ID : null;
}

/** Build one fixed browser module from the canonical shard directories. */
export function loadScriptInventoryModule(
  id: string,
  curriculumRoot: string,
  watch: WatchFile,
): string | null {
  if (id !== RESOLVED_SCRIPT_INVENTORIES_ID) return null;
  const exports: string[] = [];
  for (const inventory of INVENTORIES) {
    const monolithPath = safeInventoryPath(curriculumRoot, inventory.id);
    const shards = readShards(monolithPath);
    if (shards === null) {
      throw new Error(
        `script-ductus-inventories: ${shardDirectoryFor(monolithPath)} is missing`,
      );
    }
    // Validate the entire trust boundary before exposing even a watch path.
    // Enumerating first would follow a symlinked directory long enough to leak
    // target filenames into Vite's watcher despite the later read refusal.
    for (const shard of shards) watch(shard.path);
    const value = mergeScriptInventoryShards<
      { script: string } & Record<string, unknown>
    >(shards);
    if (value.script !== inventory.id) {
      throw new Error(
        `script-ductus-inventories: ${inventory.id} metadata claims ` +
          `${JSON.stringify(value.script)}`,
      );
    }
    // Keep authored keys as inert JSON data. Emitting an object literal directly
    // would give `__proto__` special setter semantics during module evaluation,
    // even though JSON.parse correctly treats it as an ordinary own property.
    const json = JSON.stringify(value);
    exports.push(
      `export const ${inventory.exportName} = JSON.parse(${JSON.stringify(json)});`,
    );
  }
  return `${exports.join("\n")}\n`;
}

/** Return the fixed module only for selected, in-root JSON shard changes. */
export function scriptInventoryModuleIdsForShardPath(
  changedPath: string,
  curriculumRoot: string,
): string[] {
  const inside = normalize(
    relative(resolve(curriculumRoot), resolve(changedPath)),
  ).replaceAll("\\", "/");
  if (inside === ".." || inside.startsWith("../") || isAbsolute(inside))
    return [];
  const match = /^data\/scripts\/([^/]+)\.d\/(?:[^/]+\/)?[^/]+\.json$/.exec(
    inside,
  );
  return match?.[1] !== undefined && INVENTORY_IDS.has(match[1])
    ? [RESOLVED_SCRIPT_INVENTORIES_ID]
    : [];
}

/** Vite/Vitest adapter around the pure fixed-id shard boundary above. */
export function scriptInventoryPlugin(
  options: ScriptInventoryPluginOptions,
): Plugin {
  return {
    name: "script-ductus-inventories",
    configureServer(server) {
      const scriptsRoot = dirname(
        safeInventoryPath(options.curriculumRoot, INVENTORIES[0].id),
      );
      server.watcher.add(scriptsRoot);
      server.watcher.on("all", (event, changedPath) => {
        const ids = scriptInventoryModuleIdsForShardPath(
          changedPath,
          options.curriculumRoot,
        );
        for (const id of ids) {
          const module = server.moduleGraph.getModuleById(id);
          if (module !== undefined) server.moduleGraph.invalidateModule(module);
        }
        if (ids.length > 0 && (event === "add" || event === "unlink")) {
          server.ws.send({ type: "full-reload", path: "*" });
        }
      });
    },
    resolveId(id) {
      return resolveScriptInventoryId(id);
    },
    load(id) {
      return loadScriptInventoryModule(id, options.curriculumRoot, (path) =>
        this.addWatchFile(path),
      );
    },
  };
}
