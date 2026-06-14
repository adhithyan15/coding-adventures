/**
 * frontmatter.ts — hand-rolled YAML-style frontmatter splitter.
 *
 * v0 supports only the simplest possible shape:
 *
 *     ---
 *     <key>: <value>
 *     <key>: <value>
 *     ---
 *     <markdown body...>
 *
 * Rules:
 *   - The block must begin at byte 0 (no leading whitespace, no BOM
 *     handling here — callers strip a BOM upstream if needed).
 *   - Opening fence is exactly `---` then a newline.
 *   - Closing fence is exactly `---` then a newline OR end-of-file.
 *   - Each interior line is `<key>: <value>` — key is trimmed, value
 *     is everything after the first colon, then trimmed.
 *   - Values are *always strings*.  Numbers, booleans, dates stay as
 *     their string representation; consumers can parse if they want.
 *   - Blank lines inside the block are ignored.
 *   - A line missing a colon inside the block makes the WHOLE block
 *     invalid — we fall back to "no frontmatter" and treat the original
 *     bytes as body.  This matches Jekyll's behaviour: malformed
 *     frontmatter is preserved verbatim rather than silently dropped.
 *   - If no closing fence is found, the same fallback applies.
 *
 * The parser is intentionally tiny.  Anything fancier (quoted strings,
 * arrays, nested maps, multi-line scalars) is js-yaml's job — and we
 * are explicitly NOT pulling js-yaml in for v0 to keep the parse stage
 * dependency-free.  When richer frontmatter is needed, a sibling
 * stage (e.g. `forme-parse-markdown-yaml`) will be the right place.
 *
 * @module frontmatter
 */

/**
 * Result of splitting a Markdown source string into frontmatter +
 * body.  When there's no frontmatter (or it was malformed), `data` is
 * an empty object and `body` is the original `source` unchanged.
 */
export interface FrontmatterSplit {
  /** Parsed key→string map.  Empty when no valid frontmatter present. */
  readonly data: Record<string, string>;
  /** The Markdown body with any valid frontmatter block stripped. */
  readonly body: string;
}

const FENCE = "---";

/**
 * Split a source string into `{ data, body }`.  Pure, total, no
 * exceptions thrown — malformed input degrades to "no frontmatter
 * present" so the caller can always continue.
 *
 * @example
 * ```ts
 * splitFrontmatter("---\ntitle: hi\n---\nhello") ===
 *   { data: { title: "hi" }, body: "hello" };
 *
 * splitFrontmatter("hello") ===
 *   { data: {}, body: "hello" };
 *
 * splitFrontmatter("---\nbroken line\n---\nbody") ===
 *   { data: {}, body: "---\nbroken line\n---\nbody" };  // malformed → fall back
 * ```
 */
export function splitFrontmatter(source: string): FrontmatterSplit {
  // Fast path: no opening fence at byte 0.  Includes the trailing
  // newline check so `---something` (no break) isn't mistaken for a
  // fence.
  if (!source.startsWith(FENCE + "\n") && source !== FENCE) {
    // Allow CRLF too — Windows line endings are common in user content.
    if (!source.startsWith(FENCE + "\r\n")) {
      return { data: {}, body: source };
    }
  }

  // Normalise CRLF inside this function only — we hand the body back
  // unchanged later, so we work off a parallel array of lines.
  const lines = source.split(/\r?\n/);
  // lines[0] must be the fence.  We checked above; reassert here for
  // clarity (and because `source === FENCE` also passes the first
  // check but has no closing fence ahead).
  if (lines[0] !== FENCE) {
    return { data: {}, body: source };
  }

  // Walk forward to find the closing fence.
  let closeIdx = -1;
  for (let i = 1; i < lines.length; i++) {
    if (lines[i] === FENCE) { closeIdx = i; break; }
  }
  if (closeIdx === -1) {
    // No closing fence → treat as no frontmatter.
    return { data: {}, body: source };
  }

  // Parse interior lines.  Bail on any line that lacks a colon.
  const data: Record<string, string> = {};
  for (let i = 1; i < closeIdx; i++) {
    const line = lines[i]!;
    if (line.trim() === "") continue;  // blank interior line OK
    const colonIdx = line.indexOf(":");
    if (colonIdx === -1) {
      // Malformed: whole frontmatter block invalidated.
      return { data: {}, body: source };
    }
    const key = line.slice(0, colonIdx).trim();
    const value = line.slice(colonIdx + 1).trim();
    if (key === "") {
      return { data: {}, body: source };
    }
    data[key] = value;
  }

  // Compose the body: everything after the closing fence line.  Note
  // the closing fence consumed its own newline when we split — joining
  // back with "\n" reproduces LF endings even if the source was CRLF.
  // That's intentional: parser output is normalised LF.
  const bodyLines = lines.slice(closeIdx + 1);
  const body = bodyLines.join("\n");
  return { data, body };
}
