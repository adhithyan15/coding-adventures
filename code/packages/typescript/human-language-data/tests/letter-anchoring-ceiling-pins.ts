import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

export interface LetterAnchoringCeilingPin {
  readonly cold: number;
  readonly buildsToward: number;
  readonly unwritten: number;
}

export type LetterAnchoringCeilingPins = Readonly<Record<string, LetterAnchoringCeilingPin>>;

const OWNER_NAME = /^[A-Za-z][A-Za-z0-9-]*\.json$/;
const FIELDS = ["buildsToward", "cold", "unwritten"] as const;

/** Read one exact letter-anchoring ratchet per track without following owner symlinks. */
export function loadLetterAnchoringCeilingPins(directory: string): LetterAnchoringCeilingPins {
  if (lstatSync(directory).isSymbolicLink()) {
    throw new Error(`letter-anchoring ceiling directory must not be a symbolic link: ${directory}`);
  }
  const entries = readdirSync(directory, { withFileTypes: true });
  const folded = new Set<string>();
  const pins: Record<string, LetterAnchoringCeilingPin> = {};

  entries.sort((left, right) => (left.name < right.name ? -1 : left.name > right.name ? 1 : 0));
  for (const entry of entries) {
    if (!OWNER_NAME.test(entry.name)) {
      throw new Error(`unsafe letter-anchoring ceiling owner '${entry.name}'`);
    }
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`letter-anchoring ceiling owner must be a real file: ${entry.name}`);
    }
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (folded.has(key)) {
      throw new Error(`letter-anchoring ceiling owners collide when case-folded: ${entry.name}`);
    }
    folded.add(key);
  }

  for (const entry of entries) {
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (language !== key) {
      throw new Error(`unsafe letter-anchoring ceiling owner '${entry.name}': use lowercase`);
    }
    const parsed: unknown = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      throw new Error(`${entry.name}: expected one ceiling object`);
    }
    const keys = Object.keys(parsed).sort();
    if (keys.length !== FIELDS.length || keys.some((field, index) => field !== FIELDS[index])) {
      throw new Error(`${entry.name}: expected exactly cold, buildsToward, and unwritten`);
    }
    const candidate = parsed as Record<(typeof FIELDS)[number], unknown>;
    for (const field of FIELDS) {
      if (!Number.isSafeInteger(candidate[field]) || (candidate[field] as number) < 0) {
        throw new Error(`${entry.name}: ${field} must be a non-negative safe integer`);
      }
    }
    pins[language] = candidate as unknown as LetterAnchoringCeilingPin;
  }

  if (Object.keys(pins).length === 0) {
    throw new Error("letter-anchoring ceiling owner set must not be empty");
  }
  return pins;
}
