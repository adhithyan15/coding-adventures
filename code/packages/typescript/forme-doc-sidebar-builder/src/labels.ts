/**
 * labels.ts — humanise directory / file names into display labels.
 *
 * Authors write directory names in `kebab-case` or `snake_case`
 * because file systems and URLs hate spaces.  Sidebars want
 * `Title Case` because humans hate hyphens.  This module bridges
 * the two with a deterministic, locale-independent transform.
 *
 * Rules:
 *   1. Replace `-` and `_` with single spaces.
 *   2. Collapse runs of whitespace.
 *   3. Title-case each word (first letter uppercased; rest lowered
 *      where possible — but acronyms like `API` / `SDK` / `URL`
 *      stay all-caps).
 *   4. Strip leading / trailing whitespace.
 *
 * @module labels
 */

/**
 * Acronyms / initialisms that should stay all-caps in display
 * labels.  Lowercased keys so the lookup matches the
 * normalisation step.
 *
 * Built as `Object.create(null)` so a directory literally named
 * `__proto__` doesn't read the inherited prototype accessor
 * during lookup — falls through to the default capitalisation
 * instead, same as any other unknown word.
 */
const ACRONYMS: Record<string, string> = Object.assign(Object.create(null), {
  "api": "API",
  "sdk": "SDK",
  "url": "URL",
  "uri": "URI",
  "ui": "UI",
  "ux": "UX",
  "id": "ID",
  "ip": "IP",
  "dns": "DNS",
  "http": "HTTP",
  "https": "HTTPS",
  "tcp": "TCP",
  "udp": "UDP",
  "tls": "TLS",
  "ssl": "SSL",
  "ssh": "SSH",
  "json": "JSON",
  "yaml": "YAML",
  "xml": "XML",
  "html": "HTML",
  "css": "CSS",
  "sql": "SQL",
  "ai": "AI",
  "ml": "ML",
  "vm": "VM",
  "os": "OS",
  "cpu": "CPU",
  "gpu": "GPU",
  "ssd": "SSD",
  "hdd": "HDD",
  "ram": "RAM",
  "cdn": "CDN",
  "cli": "CLI",
  "gui": "GUI",
  "ide": "IDE",
  "tdd": "TDD",
  "bdd": "BDD",
  "ci": "CI",
  "cd": "CD",
  "qa": "QA",
  "io": "I/O",
  "faq": "FAQ",
});

/**
 * Humanise a slug-style name (`getting-started`, `api_reference`)
 * into a display label (`Getting Started`, `API Reference`).
 *
 * @param slug - Directory or file name (without extension).
 * @returns The humanised label.  Empty input returns the empty
 *          string.
 */
export function humanise(slug: string): string {
  // Step 1: replace separators with spaces, then trim + collapse
  // runs of whitespace.
  const spaced = slug.replace(/[-_]+/g, " ").replace(/\s+/g, " ").trim();
  if (spaced === "") return "";
  // Step 2: title-case each word, honouring the acronym table.
  return spaced
    .split(" ")
    .map((word) => {
      const lower = word.toLowerCase();
      const acro = ACRONYMS[lower];
      if (acro !== undefined) return acro;
      // Default: first letter upper, rest lower.  Locale-independent
      // toUpperCase / toLowerCase per the same reasoning as the slug
      // module in heading-anchors — sidebar labels must be stable
      // across machines.
      return lower.charAt(0).toUpperCase() + lower.slice(1);
    })
    .join(" ");
}
