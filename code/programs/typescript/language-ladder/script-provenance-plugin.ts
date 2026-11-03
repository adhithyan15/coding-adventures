import { readFileSync } from "node:fs";
import { dirname, isAbsolute, join, normalize, relative, resolve } from "node:path";
import type { Plugin } from "vite";

/**
 * Keep stroke-order provenance prose off the first paint.
 *
 * WHY THIS EXISTS
 * ---------------
 * Every script inventory under `data/scripts/*.json` carries, for each letter
 * whose pen path is source-verified, a `strokeOrderSource` record:
 *
 *     { citation: "Hanzi Writer Data 怪.json, ordered stroke paths …",
 *       url:      "https://raw.githubusercontent.com/…/%E6%80%AA.json",
 *       variation: "The cited PRC-order dataset supplies eight ordered …" }
 *
 * script-ductus's `scriptdata.ts` hands every inventory to the app's EAGER
 * path (the letter views). For a letter, that path needs only two facts from
 * the record: that it EXISTS (`core.ts` labels the letter "verified-ductus")
 * and its `url` (`verifiedLetterFont()` matches a cited letter to its font).
 * The citation and the `variation` caveat are shown in one place, the
 * handwriting card, which `main.ts` fills from the LAZY handwriting tools.
 *
 * For Chinese those tools read the record straight from `chinese.json`
 * (script-ductus `src/strokes/chinese.ts` imports the file itself), so the
 * eager copy's prose is pure weight. And it is a lot of weight: Chinese B1
 * took the inventory to 353 verified characters, and 410 kB of `chinese.json`'s
 * 789 kB is this prose. One JSON module cannot be split by the `script-data`
 * group's `maxSize`, so the whole file became one 700 kB eager chunk and the
 * 500 kB bundle gate failed. Raising the gate would only move the cliff: each
 * new character adds about 1 kB of prose nobody sees before opening a card.
 *
 * WHAT IT DOES
 * ------------
 * Two module identities for one file, chosen by the importer:
 *
 *   importer                     module id                                content
 *   ---------------------------  ---------------------------------------  ------------------------
 *   script-ductus scriptdata.ts  \0virtual:ladder-slim-script/chinese     strokeOrderSource -> {url}
 *   anyone else (strokes/…)      …/data/scripts/chinese.json              the file, untouched
 *
 * `isEagerScriptDataModuleId` (the `script-data` chunk group's test in
 * vite.config.ts) puts the slim copy in the eager batches and leaves the full
 * file out of them, so it rides with the lazy handwriting tools that import
 * it. Nothing a learner sees changes.
 *
 * WHY ONLY CHINESE
 * ----------------
 * A script can join `SLIM_SCRIPTS` only when its stroke module imports the
 * inventory file directly. Arabic's (`strokes/arabic-family.ts`) reads its
 * sources through scriptdata's `SCRIPTS`, so slimming arabic.json there would
 * blank the citations on its handwriting cards; the tests pin that rule.
 *
 * The plugin applies to `vite build` only. Tests use vitest.config.ts, which
 * does not load it, so every test reads the canonical files verbatim.
 */

/** Inventories whose eager copy is slimmed (see WHY ONLY CHINESE above). */
export const SLIM_SCRIPTS: readonly string[] = ["chinese"];

export const SLIM_MODULE_PREFIX = "virtual:ladder-slim-script/";
const RESOLVED_SLIM_PREFIX = `\0${SLIM_MODULE_PREFIX}`;

const SCRIPT_FILE = /[\\/]learning[\\/]human-languages[\\/]data[\\/]scripts[\\/]([a-z][a-z-]*)\.json$/;
const SCRIPTDATA = /[\\/]script-ductus[\\/]src[\\/]scriptdata\.ts$/;

const stripQuery = (id: string): string => id.split(/[?#]/, 1)[0];

/** The script id of a monolithic inventory path (`chinese` for …/data/scripts/chinese.json), else null. */
export function scriptOfInventoryPath(id: string): string | null {
  return SCRIPT_FILE.exec(stripQuery(id))?.[1] ?? null;
}

/**
 * The `script-data` chunk group's test: the eager inventory batches.
 *
 *   - a slim copy (`\0virtual:ladder-slim-script/<script>`): eager, in the batches;
 *   - script-ductus's shard-native inventories module: eager, as before;
 *   - a monolithic inventory file: in the batches UNLESS it is slimmed, because
 *     then only the lazy stroke modules import the file and its prose must stay
 *     with them.
 */
export function isEagerScriptDataModuleId(id: string): boolean {
  if (id.includes(SLIM_MODULE_PREFIX)) return true;
  // The group's original test, unchanged: anything under data/scripts/ and the
  // shard-native inventories module …
  if (!/learning[\\/]human-languages[\\/]data[\\/]scripts[\\/]|virtual:script-ductus-inventories/.test(id)) {
    return false;
  }
  // … minus the full files of the slimmed inventories.
  const script = scriptOfInventoryPath(id);
  return script === null || !SLIM_SCRIPTS.includes(script);
}

/**
 * Replace every `strokeOrderSource` record, at any depth, with `{ url }`.
 *
 * Depth matters: sources hang off letters, independent vowels, final
 * consonants, digits and marks alike. Nothing else in the tree is touched, and
 * a record without a string `url` keeps an empty object, so "has a source"
 * stays true.
 */
export function slimProvenance(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(slimProvenance);
  if (value === null || typeof value !== "object") return value;
  const out: Record<string, unknown> = {};
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    let next: unknown;
    if (key === "strokeOrderSource" && child !== null && typeof child === "object") {
      const url = (child as { url?: unknown }).url;
      next = typeof url === "string" ? { url } : {};
    } else {
      next = slimProvenance(child);
    }
    // defineProperty, not `out[key] =`: a data key named "__proto__" stays an
    // own property instead of silently re-parenting the copy.
    Object.defineProperty(out, key, { value: next, enumerable: true, writable: true, configurable: true });
  }
  return out;
}

/** Keep a resolved path inside the curriculum root; refuse anything that escapes it. */
function insideRoot(root: string, file: string): boolean {
  const rel = normalize(relative(root, file)).replaceAll("\\", "/");
  return rel !== ".." && !rel.startsWith("../") && !isAbsolute(rel);
}

export interface ScriptProvenancePluginOptions {
  readonly curriculumRoot: string;
}

export function scriptProvenancePlugin(options: ScriptProvenancePluginOptions): Plugin {
  const root = resolve(options.curriculumRoot);
  return {
    name: "language-ladder:script-provenance",
    enforce: "pre",
    apply: "build",
    resolveId(source, importer) {
      if (importer === undefined || !SCRIPTDATA.test(stripQuery(importer))) return null;
      if (!source.startsWith(".")) return null;
      const file = resolve(dirname(stripQuery(importer)), source);
      const script = scriptOfInventoryPath(file);
      if (script === null || !SLIM_SCRIPTS.includes(script) || !insideRoot(root, file)) return null;
      return `${RESOLVED_SLIM_PREFIX}${script}`;
    },
    load(id) {
      if (!id.startsWith(RESOLVED_SLIM_PREFIX)) return null;
      const script = id.slice(RESOLVED_SLIM_PREFIX.length);
      // Only a listed id is ever produced by resolveId; refuse anything else
      // rather than building a path from it.
      if (!SLIM_SCRIPTS.includes(script)) {
        throw new Error(`language-ladder:script-provenance: unknown script ${JSON.stringify(script)}`);
      }
      const file = join(root, "data", "scripts", `${script}.json`);
      this.addWatchFile(file);
      const slim = slimProvenance(JSON.parse(readFileSync(file, "utf8")));
      return `export default ${JSON.stringify(slim)};\n`;
    },
  };
}
