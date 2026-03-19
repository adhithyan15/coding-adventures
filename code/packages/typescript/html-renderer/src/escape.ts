/**
 * HTML Escaping — Preventing cross-site scripting (XSS) attacks.
 * ===============================================================
 *
 * When we embed user-provided data (like source code) into HTML, we
 * must *escape* special characters. Without escaping, a source string
 * like `<script>alert("hacked")</script>` would be interpreted as
 * an actual script tag by the browser.
 *
 * HTML has five characters that have special meaning:
 *
 * | Character | HTML Entity | Why it's special                    |
 * |-----------|-------------|-------------------------------------|
 * | &         | &amp;       | Starts an entity reference          |
 * | <         | &lt;        | Starts an HTML tag                  |
 * | >         | &gt;        | Ends an HTML tag                    |
 * | "         | &quot;      | Delimits attribute values            |
 * | '         | &#x27;      | Delimits attribute values (alt)      |
 *
 * By replacing these characters with their entity equivalents, we
 * ensure the browser displays them as literal text rather than
 * interpreting them as HTML markup.
 *
 * This is one of the most important security practices in web
 * development — every template engine and web framework does this
 * automatically. Since we're generating raw HTML strings, we need
 * to do it ourselves.
 */

/**
 * Escape a string for safe inclusion in HTML content.
 *
 * Replaces the five HTML-special characters with their entity
 * equivalents. The ampersand must be replaced first, because
 * if we did it last, we'd double-escape the & in &lt; etc.
 *
 * Examples:
 *   escapeHtml('<b>bold</b>')  →  '&lt;b&gt;bold&lt;/b&gt;'
 *   escapeHtml('x & y')        →  'x &amp; y'
 *   escapeHtml('a="hello"')    →  'a=&quot;hello&quot;'
 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;") // Must be first!
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;");
}
