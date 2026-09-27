import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { CEFR_LEVELS, type CefrLevel } from "../src/levels.js";

export type LevelGateAttainmentPins = Readonly<Record<string, CefrLevel | null>>;

const OWNER_NAME = /^[A-Za-z][A-Za-z0-9-]*\.json$/;

/** Read one exact level-gate verdict per track without following owner symlinks. */
export function loadLevelGateAttainmentPins(directory: string): LevelGateAttainmentPins {
  if (lstatSync(directory).isSymbolicLink()) {
    throw new Error(`level-gate attainment directory must not be a symbolic link: ${directory}`);
  }
  const entries = readdirSync(directory, { withFileTypes: true });
  const folded = new Set<string>();
  const pins: Record<string, CefrLevel | null> = {};

  entries.sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    if (!OWNER_NAME.test(entry.name)) {
      throw new Error(`unsafe level-gate attainment owner '${entry.name}'`);
    }
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`level-gate attainment owner must be a real file: ${entry.name}`);
    }
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (folded.has(key)) {
      throw new Error(`level-gate attainment owners collide when case-folded: ${entry.name}`);
    }
    folded.add(key);
  }

  for (const entry of entries) {
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (language !== key) {
      throw new Error(`unsafe level-gate attainment owner '${entry.name}': use lowercase`);
    }

    const parsed: unknown = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      Array.isArray(parsed) ||
      Object.keys(parsed).length !== 1 ||
      !Object.hasOwn(parsed, "attained")
    ) {
      throw new Error(`${entry.name}: expected exactly one 'attained' field`);
    }
    const attained = (parsed as { attained: unknown }).attained;
    if (attained !== null && !CEFR_LEVELS.includes(attained as CefrLevel)) {
      throw new Error(`${entry.name}: unsafe attained level ${JSON.stringify(attained)}`);
    }
    pins[language] = attained as CefrLevel | null;
  }

  if (Object.keys(pins).length === 0) {
    throw new Error("level-gate attainment owner set must not be empty");
  }
  return pins;
}
