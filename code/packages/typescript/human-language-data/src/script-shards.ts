import { mergeSectionedShards, type Shard } from "./shard.js";
import type { Letter, Mark, ScriptData } from "./types.js";

/**
 * The stable, filesystem-safe identity of one authored script entry.
 *
 * Glyph text is the entry's real identity, but using it directly as a filename
 * makes the shard set depend on filesystem Unicode normalisation. Code points
 * are unambiguous on every platform and remain readable in review:
 * `あ` becomes `U-3042`, while a multi-code-point grapheme becomes
 * `U-XXXX-U-YYYY`.
 */
export function scriptEntryId(glyph: unknown): string {
  if (typeof glyph !== "string" || glyph.length === 0) {
    throw new Error(`script shard entry has no non-empty glyph: ${JSON.stringify(glyph)}`);
  }
  return [...glyph]
    .map((character) => `U-${character.codePointAt(0)!.toString(16).toUpperCase()}`)
    .join("-");
}

const SCRIPT_SECTIONS = [
  { key: "letters", dir: "letters" },
  { key: "marks", dir: "marks" },
] as const;

const ENTRY_NAME = /^(letters|marks)\/(\d{4})-(U-[0-9A-F]+(?:-U-[0-9A-F]+)*)\.json$/;

function entryGlyph(kind: "letters" | "marks", value: unknown, name: string): string {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`script shard '${name}': must contain one JSON object`);
  }
  const glyph = kind === "letters"
    ? (value as Partial<Letter>).glyph
    : (value as Partial<Mark>).mark;
  if (typeof glyph !== "string" || glyph.length === 0) {
    throw new Error(
      `script shard '${name}': must carry one non-empty '${kind === "letters" ? "glyph" : "mark"}'`,
    );
  }
  return glyph;
}

/**
 * Reassemble a script inventory while enforcing the filename as a trustworthy
 * entry identity, not merely a sorting hint.
 *
 * `mergeSectionedShards` owns the general `_meta`, key-order and unknown-file
 * guards. These checks are specific to script inventories: one entry per file,
 * a code-point id that matches its glyph, one ordinal per section, and one
 * owner for each glyph across letters and marks.
 */
export function mergeScriptInventoryShards(shards: Shard[]): ScriptData {
  const ordinals = new Map<string, Set<string>>([
    ["letters", new Set<string>()],
    ["marks", new Set<string>()],
  ]);
  const glyphOwners = new Map<string, string>();

  for (const shard of shards) {
    if (shard.name === "_meta.json") continue;
    const match = ENTRY_NAME.exec(shard.name);
    if (match === null) {
      throw new Error(
        `script shard '${shard.name}': expected ` +
          `letters/NNNN-U-<CODEPOINT>.json or marks/NNNN-U-<CODEPOINT>.json`,
      );
    }
    const kind = match[1] as "letters" | "marks";
    const ordinal = match[2];
    const id = match[3];
    const seenOrdinals = ordinals.get(kind)!;
    if (seenOrdinals.has(ordinal)) {
      throw new Error(`script shard '${shard.name}': duplicate ${kind} ordinal '${ordinal}'`);
    }
    seenOrdinals.add(ordinal);

    const glyph = entryGlyph(kind, shard.value, shard.name);
    const expected = scriptEntryId(glyph);
    if (id !== expected) {
      throw new Error(
        `script shard '${shard.name}': filename id '${id}' does not match ` +
          `${JSON.stringify(glyph)} (${expected})`,
      );
    }
    const owner = glyphOwners.get(glyph);
    if (owner !== undefined) {
      throw new Error(
        `script shard '${shard.name}': glyph ${JSON.stringify(glyph)} is already owned by '${owner}'`,
      );
    }
    glyphOwners.set(glyph, shard.name);
  }

  return mergeSectionedShards(shards, SCRIPT_SECTIONS) as unknown as ScriptData;
}
