// The filesystem boundary. Everything impure lives here: it reads the curriculum
// directory off disk and hands strings to the pure parse/build/validate core.
// This is the only module that needs the `filesystem` capability.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { buildDataset, parseLesson, type ParsedLesson } from "./parse.js";
import type { Dataset, Script, ScriptData, Taxonomy } from "./types.js";

/** Default curriculum root: code/learning/human-languages, relative to this package. */
export function defaultCurriculumRoot(): string {
  const here = dirname(fileURLToPath(import.meta.url));
  // src/ -> human-language-data -> typescript -> packages -> code
  return join(here, "..", "..", "..", "..", "learning", "human-languages");
}

export function loadTaxonomy(root = defaultCurriculumRoot()): Taxonomy {
  const raw = JSON.parse(readFileSync(join(root, "concepts", "taxonomy.json"), "utf8"));
  return { version: raw.version ?? 1, concepts: raw.concepts ?? {} };
}

/**
 * A track may declare its own script in `<track>/track.json` (`{ "script": "hebrew" }`),
 * so adding a new-script language needs no edit to the built-in map. Returns
 * `undefined` when there's no declaration, and the parser falls back to the map.
 */
export function trackScript(root: string, trackName: string): Script | undefined {
  const p = join(root, trackName, "track.json");
  if (!existsSync(p)) return undefined;
  try {
    const t = JSON.parse(readFileSync(p, "utf8"));
    return typeof t?.script === "string" ? t.script : undefined;
  } catch {
    return undefined; // a malformed track.json just falls back to the map
  }
}

/** Read every track's lessons/*.md into parsed lessons. */
export function loadLessons(root = defaultCurriculumRoot()): ParsedLesson[] {
  const out: ParsedLesson[] = [];
  for (const track of readdirSync(root, { withFileTypes: true })) {
    if (!track.isDirectory()) continue;
    const lessonsDir = join(root, track.name, "lessons");
    if (!existsSync(lessonsDir)) continue;
    const script = trackScript(root, track.name);
    for (const file of readdirSync(lessonsDir)) {
      if (!file.endsWith(".md")) continue;
      const source = readFileSync(join(lessonsDir, file), "utf8");
      out.push(parseLesson(source, track.name, script));
    }
  }
  return out;
}

/** Read data/scripts/*.json (may be empty while scripts are still being authored). */
export function loadScripts(root = defaultCurriculumRoot()): Record<string, ScriptData> {
  const dir = join(root, "data", "scripts");
  const out: Record<string, ScriptData> = Object.create(null);
  if (!existsSync(dir)) return out;
  for (const file of readdirSync(dir)) {
    if (!file.endsWith(".json")) continue;
    const sd = JSON.parse(readFileSync(join(dir, file), "utf8")) as ScriptData;
    out[sd.script] = sd;
  }
  return out;
}

/** Load and build everything from disk in one call. */
export function loadEverything(root = defaultCurriculumRoot()): {
  taxonomy: Taxonomy;
  lessons: ParsedLesson[];
  scripts: Record<string, ScriptData>;
  dataset: Dataset;
} {
  const taxonomy = loadTaxonomy(root);
  const lessons = loadLessons(root);
  const scripts = loadScripts(root);
  return { taxonomy, lessons, scripts, dataset: buildDataset(taxonomy, lessons) };
}
