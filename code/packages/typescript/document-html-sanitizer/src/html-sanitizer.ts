/**
 * Pattern-Based HTML String Sanitizer
 *
 * Sanitizes an HTML string using regex/string operations — no DOM required.
 * This approach trades some fidelity for portability: the same algorithm can
 * be ported to Go, Python, Rust, Elixir, and Lua without a browser runtime.
 *
 * === Algorithm Overview ===
 *
 * The sanitizer applies transforms in a specific order to avoid interference:
 *
 *   Step 1: Drop HTML comments (if dropComments: true)
 *           Comments first — they may contain HTML that would be processed
 *           by later steps if not removed early.
 *
 *   Step 2: Drop dangerous elements (if dropElements is non-empty)
 *           Elements are removed including all their inner content.
 *           This handles <script>alert(1)</script> → ""
 *
 *   Step 3: Strip dangerous attributes from remaining elements
 *           - All `on*` event handlers (onclick, onload, etc.)
 *           - srcdoc, formaction (always stripped)
 *           - Any additional names in dropAttributes
 *           - style attributes containing expression() or url() (if sanitizeStyleAttributes)
 *           - href/src attributes with disallowed URL schemes
 *
 * === Limitations of the Regex Approach ===
 *
 * Pattern-based sanitization has known limitations:
 *
 *   - Nested same-tag elements can confuse greedy regex matching
 *   - Malformed HTML (unclosed tags, mismatched quotes) may not be handled
 *   - Attribute values with complex escaping might evade detection
 *
 * For high-security contexts in a browser environment, supply a `domAdapter`
 * in the policy to use the DOM-based path (see dom-sanitizer.ts).
 *
 * === Design ===
 *
 * This module exports only `sanitizeHtml`. Internal helper functions are not
 * exported — callers should use the public API and policy presets only.
 *
 * Spec: TE02 — Document Sanitization §Stage 2
 *
 * @module html-sanitizer
 */

import type { HtmlSanitizationPolicy } from "./policy.js";
import { isSchemeAllowed } from "./url-utils.js";
import { sanitizeHtmlWithDom } from "./dom-sanitizer.js";

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Sanitize an HTML string by stripping dangerous elements and attributes.
 *
 * When `policy.domAdapter` is provided, delegates to the DOM-based sanitizer
 * for higher-fidelity results. Otherwise, uses regex/string operations.
 *
 * @param html    The HTML string to sanitize.
 * @param policy  The sanitization policy to apply.
 * @returns       A sanitized HTML string.
 *
 * @example
 * // Drop scripts and event handlers
 * sanitizeHtml("<p>Safe</p><script>alert(1)</script>", HTML_STRICT)
 * // → "<p>Safe</p>"
 *
 * sanitizeHtml('<img src="x.png" onload="alert(1)">', HTML_STRICT)
 * // → '<img src="x.png">'
 *
 * sanitizeHtml('<a href="javascript:alert(1)">click</a>', HTML_STRICT)
 * // → '<a href="">click</a>'
 */
export function sanitizeHtml(html: string, policy: HtmlSanitizationPolicy): string {
  // DOM path: higher fidelity when a DOM adapter is available
  if (policy.domAdapter) {
    return sanitizeHtmlWithDom(html, policy, policy.domAdapter);
  }

  // String/regex path: portable across all target languages
  return sanitizeHtmlWithRegex(html, policy);
}

// ─── Regex-Based Sanitizer ────────────────────────────────────────────────────

/**
 * Main regex-based sanitizer pipeline.
 *
 * Applies transforms in order: comments → elements → attributes.
 * Each step produces a new string; no intermediate state is shared.
 */
function sanitizeHtmlWithRegex(html: string, policy: HtmlSanitizationPolicy): string {
  let result = html;

  // Step 1: Drop HTML comments
  // Comments before elements — they may contain HTML that later steps
  // would process if the comment is not removed first.
  if (policy.dropComments ?? true) {
    result = dropComments(result);
  }

  // Step 2: Drop dangerous elements (and their content)
  const dropElems = policy.dropElements ?? [];
  if (dropElems.length > 0) {
    result = dropElements(result, dropElems);
  }

  // Step 3: Strip dangerous attributes from all remaining tags
  result = sanitizeAttributes(result, policy);

  return result;
}

// ─── Step 1: Comment Stripping ────────────────────────────────────────────────

/**
 * Remove all HTML comments from the string.
 *
 * Pattern: <!-- followed by anything (including newlines) followed by -->
 *
 * HTML comments can contain:
 *   - IE conditional comments: <!--[if IE]><script>alert(1)</script><![endif]-->
 *   - Content injection that survives in some partial-DOM parsers
 *   - Test harness bypass (<!-- <img src=x onerror=alert(1)> -->)
 *
 * The regex is non-greedy (?:[\s\S]*?) to avoid matching from the first
 * <!-- to the last --> when multiple comments are present.
 *
 * Note: this does NOT handle nested comments (HTML doesn't support them)
 * or comments inside attribute values (unusual but possible in malformed HTML).
 */
function dropComments(html: string): string {
  let result = "";
  let index = 0;

  while (index < html.length) {
    const start = html.indexOf("<!--", index);
    if (start === -1) {
      return result + html.slice(index);
    }

    result += html.slice(index, start);
    const end = findCommentEnd(html, start + 4);
    if (end === -1) {
      return result + html.slice(start);
    }
    index = end;
  }

  return result;
}

// ─── Step 2: Element Dropping ─────────────────────────────────────────────────

/**
 * Remove specified HTML elements and their complete content.
 *
 * For each element name in the drop list, we build a regex that matches
 * the opening tag, all inner content, and the closing tag. Content inside
 * dropped elements is also removed — this prevents attack patterns like:
 *
 *   <script>alert(1)</script>
 *   <script src="https://evil.com/xss.js"></script>
 *   <SCRIPT>alert(1)</SCRIPT>  (uppercase — handled by case-insensitive flag)
 *
 * Limitation: This regex approach works for well-formed HTML but may fail
 * for deeply nested same-tag elements (e.g. nested <div> inside <div>).
 * For the elements in the default drop list (script, iframe, style, etc.),
 * nesting is uncommon and the regex works reliably.
 *
 * The pattern:
 *   <tagName          opening tag name (case-insensitive)
 *   (?:\s[^>]*)?      optional attributes
 *   >                 end of opening tag
 *   [\s\S]*?          inner content (non-greedy, includes newlines)
 *   </tagName\s*>     closing tag
 *   |                 OR
 *   <tagName          self-closing or empty element (e.g. <script src="…" />)
 *   (?:\s[^>]*)?
 *   />
 *   |                 OR
 *   <tagName          opening tag with no closing tag (malformed)
 *   (?:\s[^>]*)?>
 */
function dropElements(html: string, elementNames: readonly string[]): string {
  let result = html;
  for (const tagName of elementNames) {
    // Escape the tag name for use in regex (it's from a controlled list, but
    // defensive escaping is a good habit)
    const escaped = escapeForRegex(tagName);

    // Pattern for elements with paired opening and closing tags
    // The [\s\S]*? is non-greedy to handle multiple elements of the same type
    const pairedPattern = new RegExp(
      `<${escaped}(?:\\s[^>]*)?>(?:[\\s\\S]*?)<\\/${escaped}\\s*>`,
      "gi",
    );

    // Pattern for self-closing tags (e.g. <script />) — rare but valid
    const selfClosingPattern = new RegExp(
      `<${escaped}(?:\\s[^>]*)?/>`,
      "gi",
    );

    // Pattern for lone opening tags with no closing tag (malformed HTML)
    // This is a last resort for malformed content
    const loneOpenPattern = new RegExp(
      `<${escaped}(?:\\s[^>]*)?>`,
      "gi",
    );

    result = result.replace(pairedPattern, "");
    result = result.replace(selfClosingPattern, "");
    result = result.replace(loneOpenPattern, "");
  }
  return result;
}

/**
 * Escape special regex characters in a string.
 * Used to safely include tag names and attribute names in regex patterns.
 */
function escapeForRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ─── Step 3: Attribute Sanitization ──────────────────────────────────────────

/**
 * Resolve which attributes to drop, based on the policy.
 *
 * Always drops:
 *   - All `on*` event handlers (onclick, onload, onerror, etc.)
 *   - srcdoc (iframe inline HTML injection)
 *   - formaction (form submission URL override)
 *
 * Also drops anything in policy.dropAttributes.
 */
function buildDropAttributeSet(policy: HtmlSanitizationPolicy): Set<string> {
  const always = new Set(["srcdoc", "formaction"]);
  for (const attr of policy.dropAttributes ?? []) {
    always.add(attr.toLowerCase());
  }
  return always;
}

/**
 * Walk all HTML tags in the string and strip dangerous attributes from each.
 *
 * This function finds opening tags using a regex, then processes each tag's
 * attributes individually. The original tag is replaced with a sanitized version.
 *
 * Tag pattern breakdown:
 *   <                  opening angle bracket
 *   ([a-zA-Z][^\s/>]*) tag name (captured)
 *   ((?:\s[^>]*)?)     all attributes as one capture group (optional)
 *   (/?>)              end of tag: either "/>" (self-closing) or ">"
 *
 * We specifically do NOT modify closing tags (</div>) — they have no attributes.
 */
function sanitizeAttributes(html: string, policy: HtmlSanitizationPolicy): string {
  const dropAttrSet = buildDropAttributeSet(policy);
  let result = "";
  let index = 0;

  while (index < html.length) {
    const tagStart = html.indexOf("<", index);
    if (tagStart === -1) {
      return result + html.slice(index);
    }

    result += html.slice(index, tagStart);

    const parsedTag = parseOpeningTag(html, tagStart);
    if (!parsedTag) {
      result += "<";
      index = tagStart + 1;
      continue;
    }

    const { tagName, attrString, closing, endIndex } = parsedTag;
    if (!attrString || attrString.trim() === "") {
      result += `<${tagName}${closing}`;
      index = endIndex;
      continue;
    }

    const sanitizedAttrs = sanitizeAttributeString(attrString, dropAttrSet, policy);
    const attrPart = sanitizedAttrs ? " " + sanitizedAttrs : "";
    result += `<${tagName}${attrPart}${closing}`;
    index = endIndex;
  }

  return result;
}

function isHtmlWhitespace(ch: string | undefined): boolean {
  return ch === " " || ch === "\t" || ch === "\n" || ch === "\r" || ch === "\f";
}

function isAsciiLetter(ch: string | undefined): boolean {
  return ch !== undefined && (
    (ch >= "A" && ch <= "Z")
    || (ch >= "a" && ch <= "z")
  );
}

function isTagNameChar(ch: string | undefined): boolean {
  return ch !== undefined && (
    isAsciiLetter(ch)
    || (ch >= "0" && ch <= "9")
    || ch === "-"
    || ch === "_"
    || ch === ":"
  );
}

function findCommentEnd(html: string, startIndex: number): number {
  for (let index = startIndex; index < html.length - 2; index++) {
    if (html[index] !== "-" || html[index + 1] !== "-") {
      continue;
    }
    if (html[index + 2] === ">") {
      return index + 3;
    }
    if (html[index + 2] === "!" && html[index + 3] === ">") {
      return index + 4;
    }
  }
  return -1;
}

function findTagEnd(html: string, startIndex: number): number {
  let quote: string | null = null;

  for (let index = startIndex; index < html.length; index++) {
    const ch = html[index];
    if (quote) {
      if (ch === quote) {
        quote = null;
      }
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === ">") {
      return index;
    }
  }

  return -1;
}

function parseOpeningTag(
  html: string,
  startIndex: number,
): { tagName: string; attrString: string; closing: ">" | "/>"; endIndex: number } | null {
  if (html[startIndex] !== "<" || html[startIndex + 1] === "/" || !isAsciiLetter(html[startIndex + 1])) {
    return null;
  }

  let index = startIndex + 1;
  while (index < html.length && isTagNameChar(html[index])) {
    index++;
  }

  const tagName = html.slice(startIndex + 1, index);
  const tagEnd = findTagEnd(html, index);
  if (tagEnd === -1) {
    return null;
  }

  let attrString = html.slice(index, tagEnd);
  let trimmedEnd = attrString.length;
  while (trimmedEnd > 0 && isHtmlWhitespace(attrString[trimmedEnd - 1])) {
    trimmedEnd--;
  }

  let closing: ">" | "/>" = ">";
  if (trimmedEnd > 0 && attrString[trimmedEnd - 1] === "/") {
    closing = "/>";
    attrString = attrString.slice(0, trimmedEnd - 1);
  }

  return {
    tagName,
    attrString,
    closing,
    endIndex: tagEnd + 1,
  };
}

/**
 * Sanitize a raw attribute string from an HTML tag.
 *
 * This function parses key="value" pairs from the raw attribute string and
 * filters out dangerous ones.
 *
 * Attribute value parsing handles:
 *   - Double-quoted: attr="value"
 *   - Single-quoted: attr='value'
 *   - Unquoted: attr=value (stops at whitespace or >)
 *   - Boolean (no value): attr
 *
 * Returns the sanitized attribute string (space-separated key=value pairs),
 * or empty string if all attributes were stripped.
 */
function sanitizeAttributeString(
  attrString: string,
  dropAttrSet: Set<string>,
  policy: HtmlSanitizationPolicy,
): string {
  // Parse attribute key-value pairs from the raw string
  const attrs = parseAttributes(attrString);
  const kept: string[] = [];

  for (const [name, value, raw] of attrs) {
    const nameLower = name.toLowerCase();

    // Drop all on* event handlers (onclick, onload, onerror, onmouseover, etc.)
    // This pattern catches any attribute starting with "on" followed by letters.
    if (/^on[a-z]/i.test(nameLower)) {
      continue; // dropped
    }

    // Drop attributes in the explicit drop set (srcdoc, formaction, + user list)
    if (dropAttrSet.has(nameLower)) {
      continue; // dropped
    }

    // Sanitize style attributes (strip expression() and url() attacks)
    if (nameLower === "style" && (policy.sanitizeStyleAttributes ?? true)) {
      if (isDangerousStyle(value)) {
        continue; // drop the entire style attribute
      }
    }

    // Sanitize URL attributes (href and src).
    // When allowedUrlSchemes is explicitly null, all schemes are allowed.
    // When it is undefined (omitted), use the default safe list.
    // Use !== undefined to distinguish null (allow all) from undefined (use default).
    if (nameLower === "href" || nameLower === "src") {
      const schemes = policy.allowedUrlSchemes !== undefined
        ? policy.allowedUrlSchemes
        : ["http", "https", "mailto", "ftp"];
      if (!isSchemeAllowed(value, schemes)) {
        // Replace the attribute value with "" to make the link/image inert
        kept.push(`${name}=""`);
        continue;
      }
    }

    // Attribute passes all checks — keep the original raw representation
    kept.push(raw);
  }

  return kept.join(" ");
}

// ─── Attribute Parsing ────────────────────────────────────────────────────────

/**
 * Parse an HTML attribute string into (name, value, rawString) tuples.
 *
 * Returns an array of triples:
 *   [0] name  — attribute name (original casing preserved)
 *   [1] value — decoded attribute value
 *   [2] raw   — original raw text as it appeared in the string (for round-trip)
 *
 * This parser handles the four attribute syntax forms:
 *
 *   1. name="value"   (double-quoted)
 *   2. name='value'   (single-quoted)
 *   3. name=value     (unquoted, value is everything up to whitespace or >)
 *   4. name           (boolean attribute, no value)
 *
 * The regex captures each form:
 *
 *   ([a-zA-Z:_][^\s=/>]*)   attribute name
 *   (?:                      optional = and value
 *     \s*=\s*                equals sign, optional whitespace
 *     (?:
 *       "([^"]*)"            double-quoted value
 *       | '([^']*)'          single-quoted value
 *       | ([^\s>]+)          unquoted value
 *     )
 *   )?
 */
function parseAttributes(attrString: string): Array<[string, string, string]> {
  const result: Array<[string, string, string]> = [];

  // Match attribute patterns
  const ATTR_PATTERN = /([a-zA-Z:_][^\s=/>]*)\s*(?:=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]*)))?/g;

  let match: RegExpExecArray | null;
  while ((match = ATTR_PATTERN.exec(attrString)) !== null) {
    const name = match[1];
    if (!name) continue;

    // Value: prefer double-quoted [2], then single-quoted [3], then unquoted [4]
    const value = match[2] ?? match[3] ?? match[4] ?? "";

    // Raw: reconstruct the original representation for round-trip fidelity
    const raw = match[0].trim();
    if (raw) {
      result.push([name, value, raw]);
    }
  }

  return result;
}

// ─── CSS Injection Detection ──────────────────────────────────────────────────

/**
 * Detect dangerous CSS patterns in a style attribute value.
 *
 * Two dangerous patterns are checked:
 *
 *   expression(…)  — IE CSS expression execution. `expression(alert(1))`
 *                    in a style attribute executes JavaScript in old IE.
 *                    Example: style="width:expression(alert(1))"
 *
 *   url(…)         — Can be used to load attacker resources or exfiltrate
 *                    data. We only flag url() with non-http/https arguments.
 *                    Example: style="background:url(javascript:alert(1))"
 *
 * When a dangerous pattern is found, the entire style attribute is dropped
 * (not just the dangerous value). CSS parsing is complex enough that partial
 * fixes create a false sense of security. Better to lose styling than to
 * allow injection.
 *
 * @returns true if the style value is dangerous and should be dropped
 */
function isDangerousStyle(styleValue: string): boolean {
  // expression() — CSS execution (case-insensitive: Expression(), EXPRESSION())
  if (/expression\s*\(/i.test(styleValue)) {
    return true;
  }

  // url() with potentially dangerous content
  // Check for url( followed by content that is NOT http:// or https://
  // Specifically flag: javascript:, data:, vbscript:
  const urlPattern = /url\s*\(\s*(['"]?)(.+?)\1\s*\)/gi;
  let urlMatch: RegExpExecArray | null;
  while ((urlMatch = urlPattern.exec(styleValue)) !== null) {
    const urlValue = (urlMatch[2] ?? "").trim();
    if (!isSchemeAllowed(urlValue, ["http", "https"])) {
      return true;
    }
  }

  return false;
}
