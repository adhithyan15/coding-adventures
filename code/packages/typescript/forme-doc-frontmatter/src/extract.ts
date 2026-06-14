/**
 * extract.ts — main `extractFrontmatter` entry.
 *
 * Detection rules (mirror Hugo / Jekyll / Zola / VuePress
 * conventions):
 *
 *   - YAML: first line is exactly `---`; matching closing
 *     `---` somewhere later in the document.  Content between
 *     them is YAML.
 *   - TOML: first line is exactly `+++`; matching closing
 *     `+++`.  Content between is TOML.
 *   - JSON-style `{` first character is NOT supported (Hugo
 *     supports it but we don't — it conflicts with markdown
 *     that legitimately starts with `{`).
 *
 * If no frontmatter is detected, returns the input verbatim
 * with `frontmatter: null` and `format: "none"`.
 *
 * If frontmatter delimiters are detected but the block is
 * malformed (unclosed delimiter, unparseable inner content),
 * throws `TypeError`.
 *
 * @module extract
 */

import type { FrontmatterResult } from "./types.js";
import { parseToml } from "./toml.js";
import { parseYaml } from "./yaml.js";

export function extractFrontmatter(source: string): FrontmatterResult {
  if (typeof source !== "string") {
    throw new TypeError(
      `forme-doc-frontmatter: source must be a string; got ${typeof source}`,
    );
  }

  // We accept BOM at the start (some Windows editors prepend it)
  // but don't preserve it — strip and re-check.
  let normalized = source;
  if (normalized.charCodeAt(0) === 0xFEFF) {
    normalized = normalized.slice(1);
  }

  const yamlFm = tryExtract(normalized, "---", "yaml", parseYaml);
  if (yamlFm !== null) return yamlFm;
  const tomlFm = tryExtract(normalized, "+++", "toml", parseToml);
  if (tomlFm !== null) return tomlFm;

  return { body: source, frontmatter: null, format: "none" };
}

function tryExtract(
  source: string,
  delim: string,
  format: "yaml" | "toml",
  parse: (s: string) => Record<string, unknown>,
): FrontmatterResult | null {
  // First non-trailing-whitespace line must be exactly the delim.
  // We accept exactly `<delim>\n` or `<delim>\r\n` at the start.
  if (!source.startsWith(`${delim}\n`) && !source.startsWith(`${delim}\r\n`)) {
    return null;
  }
  const afterOpenLen = source.startsWith(`${delim}\r\n`) ? delim.length + 2 : delim.length + 1;
  const rest = source.slice(afterOpenLen);

  // Find the matching closing `\n<delim>\n` (or `\r\n<delim>\r\n` /
  // `<delim>\n` at the end with no trailing newline).
  // We search line-by-line to ensure the close-delim is on its own
  // line (no leading/trailing whitespace).
  const lines = rest.split(/\r?\n/);
  let closeIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i] === delim) {
      closeIdx = i;
      break;
    }
  }
  if (closeIdx === -1) {
    throw new TypeError(
      `forme-doc-frontmatter: ${format.toUpperCase()} frontmatter starts with "${delim}" but has no matching closing "${delim}"`,
    );
  }
  const inner = lines.slice(0, closeIdx).join("\n");
  const afterCloseLines = lines.slice(closeIdx + 1);
  // Re-join the body with `\n` (canonical line ending).  We
  // DON'T strip a leading newline after the close delimiter —
  // callers `.trimStart()` if they want.
  //
  // NOTE: this **normalises CRLF line endings inside the body
  // to LF**.  We split on `/\r?\n/` above; the `\r` characters
  // are lost.  If downstream tooling does exact-byte hashing
  // of the body (rare), this layer is the source of the
  // normalisation.  The tradeoff is intentional — every
  // markdown engine wants LF anyway, and emitting a
  // canonical-line-ending body lets the hashing happen
  // downstream where it's well-defined.
  const body = afterCloseLines.join("\n");

  const frontmatter = parse(inner);
  return { body, frontmatter, format };
}
