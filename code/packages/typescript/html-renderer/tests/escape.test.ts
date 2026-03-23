/**
 * Tests for HTML escaping — verifying XSS prevention.
 * =====================================================
 *
 * These tests ensure that all five HTML-special characters are
 * properly escaped, and that normal text passes through unchanged.
 * This is critical security functionality — if escaping breaks,
 * user-supplied source code could be interpreted as HTML.
 */

import { describe, it, expect } from "vitest";
import { escapeHtml } from "../src/escape.js";

describe("escapeHtml", () => {
  // =========================================================================
  // Basic escaping
  // =========================================================================

  it("should escape ampersands", () => {
    expect(escapeHtml("a & b")).toBe("a &amp; b");
  });

  it("should escape less-than signs", () => {
    expect(escapeHtml("a < b")).toBe("a &lt; b");
  });

  it("should escape greater-than signs", () => {
    expect(escapeHtml("a > b")).toBe("a &gt; b");
  });

  it("should escape double quotes", () => {
    expect(escapeHtml('a "b" c')).toBe("a &quot;b&quot; c");
  });

  it("should escape single quotes", () => {
    expect(escapeHtml("a 'b' c")).toBe("a &#x27;b&#x27; c");
  });

  // =========================================================================
  // Combined escaping
  // =========================================================================

  it("should escape all special characters at once", () => {
    expect(escapeHtml('<script>alert("xss")</script>')).toBe(
      "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;"
    );
  });

  it("should handle ampersand-first ordering correctly", () => {
    // If we escaped & last, we'd double-escape: &lt; -> &amp;lt;
    // The & in &lt; must NOT be re-escaped
    expect(escapeHtml("&<")).toBe("&amp;&lt;");
  });

  // =========================================================================
  // Pass-through
  // =========================================================================

  it("should leave normal text unchanged", () => {
    expect(escapeHtml("hello world")).toBe("hello world");
  });

  it("should handle empty string", () => {
    expect(escapeHtml("")).toBe("");
  });

  it("should handle source code with various special characters", () => {
    const source = 'x = 1 + 2; // "add" & sum';
    const expected = 'x = 1 + 2; // &quot;add&quot; &amp; sum';
    expect(escapeHtml(source)).toBe(expected);
  });
});
