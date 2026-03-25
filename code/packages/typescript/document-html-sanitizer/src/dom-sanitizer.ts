/**
 * DOM-Based HTML Sanitizer
 *
 * Higher-fidelity sanitizer that uses a real HTML parser via the DomAdapter
 * interface. This path is used when `policy.domAdapter` is provided.
 *
 * DOM-based sanitization is more accurate than regex-based sanitization
 * because a real parser handles:
 *   - Malformed HTML (unclosed tags, mismatched nesting)
 *   - HTML entities in attribute values
 *   - Character encoding tricks
 *   - Edge cases in the HTML5 parsing algorithm
 *
 * === Architecture ===
 *
 * The DOM sanitizer is decoupled from any specific DOM implementation via
 * the `HtmlSanitizerDomAdapter` interface. The caller supplies:
 *
 *   adapter.parse(html)        — parse HTML string to a DOM
 *   adapter.walk(dom, visitor) — walk the DOM calling the visitor
 *   adapter.serialize(dom)     — serialize the mutated DOM back to string
 *
 * The sanitizer implements the `DomVisitor` interface, applying the same
 * policy logic as the regex-based sanitizer but with DOM-level precision.
 *
 * Spec: TE02 §Stage 2
 *
 * @module dom-sanitizer
 */

import type { HtmlSanitizationPolicy, HtmlSanitizerDomAdapter, DomVisitor } from "./policy.js";
import { isSchemeAllowed } from "./url-utils.js";

/**
 * Sanitize HTML using a DOM adapter for higher-fidelity parsing.
 *
 * This function is called by sanitizeHtml() when policy.domAdapter is set.
 * It implements the full policy by creating a DomVisitor and threading
 * it through the adapter's walk.
 *
 * @param html     The HTML string to sanitize.
 * @param policy   The sanitization policy.
 * @param adapter  The DOM adapter for parsing/walking/serializing.
 * @returns        Sanitized HTML string.
 */
export function sanitizeHtmlWithDom(
  html: string,
  policy: HtmlSanitizationPolicy,
  adapter: HtmlSanitizerDomAdapter,
): string {
  // Parse into DOM
  const dom = adapter.parse(html);

  // Walk with our visitor
  const visitor = buildDomVisitor(policy);
  adapter.walk(dom, visitor);

  // Serialize back to string
  return adapter.serialize(dom);
}

/**
 * Build a DomVisitor that applies the given policy to DOM nodes.
 *
 * The visitor is stateless — it reads the policy and returns decisions.
 * The adapter is responsible for actually mutating (or rebuilding) the DOM
 * based on the visitor's return values.
 *
 * === Visitor contract ===
 *
 *   element():
 *     - Returns `false` to drop the element (and all its children)
 *     - Returns a Map of sanitized attributes to keep the element
 *
 *   comment():
 *     - Returns `false` to drop the comment
 *     - Returns the comment text string to keep the comment
 */
function buildDomVisitor(policy: HtmlSanitizationPolicy): DomVisitor {
  const dropElements = new Set((policy.dropElements ?? []).map(e => e.toLowerCase()));
  const dropAttrSet = new Set([
    "srcdoc",
    "formaction",
    ...(policy.dropAttributes ?? []).map(a => a.toLowerCase()),
  ]);
  const dropComments = policy.dropComments ?? true;

  return {
    /**
     * Process an element node.
     *
     * @param tagName     The element's tag name (lowercase)
     * @param attributes  All attributes as a Map<name, value>
     * @returns           false to drop the element, or a Map of safe attributes to keep
     */
    element(tagName: string, attributes: Map<string, string>): false | Map<string, string> {
      // Check if this element should be dropped entirely
      if (dropElements.has(tagName.toLowerCase())) {
        return false;
      }

      // Sanitize attributes
      const safeAttrs = new Map<string, string>();

      for (const [name, value] of attributes) {
        const nameLower = name.toLowerCase();

        // Drop all on* event handlers
        if (/^on[a-z]/i.test(nameLower)) continue;

        // Drop attributes in the explicit drop set
        if (dropAttrSet.has(nameLower)) continue;

        // Sanitize style attributes
        if (nameLower === "style" && (policy.sanitizeStyleAttributes ?? true)) {
          if (isDangerousStyle(value)) continue;
        }

        // Sanitize URL attributes
        if (nameLower === "href" || nameLower === "src") {
          const schemes = policy.allowedUrlSchemes ?? ["http", "https", "mailto", "ftp"];
          if (!isSchemeAllowed(value, schemes)) {
            safeAttrs.set(name, "");
            continue;
          }
        }

        // Attribute is safe — keep it
        safeAttrs.set(name, value);
      }

      return safeAttrs;
    },

    /**
     * Process a comment node.
     *
     * @param value  The comment text (without <!-- and -->)
     * @returns      false to drop, or the comment text to keep
     */
    comment(value: string): false | string {
      if (dropComments) return false;
      return value;
    },
  };
}

/**
 * Detect dangerous CSS patterns in a style attribute value.
 *
 * Duplicated from html-sanitizer.ts to keep dom-sanitizer.ts self-contained
 * (both are in the same package, but we avoid a circular import by keeping
 * the function local).
 *
 * @returns true if the style value is dangerous and should be dropped
 */
function isDangerousStyle(styleValue: string): boolean {
  if (/expression\s*\(/i.test(styleValue)) return true;

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
