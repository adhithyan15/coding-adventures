// A deliberately tiny YAML-frontmatter reader.
//
// Lesson frontmatter is a flat block of `key: value` lines between two `---`
// fences, where a value is a scalar or a simple `[a, b, c]` list. That's all the
// schema uses, so a full YAML parser (and a third-party dependency, which this
// repo forbids) would be overkill. This handles exactly that shape and nothing
// more — which is a feature, not a limitation: it can't silently mis-parse
// something clever because there's nothing clever to parse.

export type FrontmatterValue = string | string[];

export interface Frontmatter {
  [key: string]: FrontmatterValue;
}

/**
 * Split a document into its frontmatter block and body. Returns `null` for
 * frontmatter if the document doesn't open with a `---` fence.
 */
export function splitFrontmatter(source: string): {
  frontmatter: Frontmatter | null;
  body: string;
} {
  // The block must be the very first thing in the file (after an optional BOM).
  const text = source.replace(/^﻿/, "");
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(text);
  if (!match) return { frontmatter: null, body: text };
  return { frontmatter: parseBlock(match[1]), body: match[2] };
}

function parseBlock(block: string): Frontmatter {
  const out: Frontmatter = {};
  for (const raw of block.split(/\r?\n/)) {
    const line = raw.trimEnd();
    // Skip blanks and whole-line comments.
    if (line.trim() === "" || line.trimStart().startsWith("#")) continue;
    const colon = line.indexOf(":");
    if (colon === -1) continue; // not a key: value line — ignore
    const key = line.slice(0, colon).trim();
    if (key === "") continue;
    out[key] = parseValue(line.slice(colon + 1).trim());
  }
  return out;
}

function parseValue(value: string): FrontmatterValue {
  // A `[ ... ]` list — split on commas, trim, drop empties (so `[]` → `[]`).
  if (value.startsWith("[") && value.endsWith("]")) {
    return value
      .slice(1, -1)
      .split(",")
      .map((v) => unquote(v.trim()))
      .filter((v) => v !== "");
  }
  return unquote(value);
}

function unquote(value: string): string {
  if (value.length >= 2) {
    const first = value[0];
    const last = value[value.length - 1];
    if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
      return value.slice(1, -1);
    }
  }
  return value;
}
