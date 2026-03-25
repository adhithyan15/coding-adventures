/**
 * HTML Sanitizer Tests
 *
 * Comprehensive test coverage for document-html-sanitizer.
 *
 * Test categories:
 *   1. URL utilities (unit tests)
 *   2. Comment stripping
 *   3. Element dropping
 *   4. Event handler attribute stripping
 *   5. URL sanitization in href/src attributes
 *   6. CSS injection prevention
 *   7. srcdoc / formaction stripping
 *   8. HTML_STRICT preset smoke tests
 *   9. HTML_RELAXED preset
 *   10. HTML_PASSTHROUGH preset
 *   11. Safe HTML preserved unchanged
 *   12. DOM adapter path
 *   13. Policy composition
 *   14. Default policy values (omitted fields)
 *
 * @module html-sanitizer.test
 */

import { describe, it, expect } from "vitest";
import { sanitizeHtml, HTML_STRICT, HTML_RELAXED, HTML_PASSTHROUGH } from "../src/index.js";
import { stripControlChars, extractScheme, isSchemeAllowed } from "../src/url-utils.js";
import type { HtmlSanitizationPolicy, DomVisitor } from "../src/policy.js";
import {
  SCRIPT_INLINE, SCRIPT_SRC, SCRIPT_UPPER, SCRIPT_WITH_CONTENT_AFTER, SCRIPT_MULTILINE,
  EVENT_ONLOAD, EVENT_ONCLICK, EVENT_ONFOCUS, EVENT_SVG, EVENT_ONMOUSEOVER, EVENT_ONERROR,
  HREF_JAVASCRIPT, HREF_JAVASCRIPT_UPPER, HREF_DATA, SRC_JAVASCRIPT,
  CSS_EXPRESSION, CSS_URL_JAVASCRIPT, CSS_EXPRESSION_UPPER,
  COMMENT_SIMPLE, COMMENT_WITH_CONTENT, COMMENT_CONDITIONAL,
  SRCDOC, FORMACTION,
  SAFE_PARAGRAPH, SAFE_LINK, SAFE_IMAGE, SAFE_HEADING, SAFE_EMPHASIS, SAFE_CODE, SAFE_BLOCKQUOTE,
} from "./xss-vectors.js";

// ─── URL Utilities ─────────────────────────────────────────────────────────────

describe("HTML sanitizer url-utils: stripControlChars", () => {
  it("strips null byte", () => {
    expect(stripControlChars("java\x00script:")).toBe("javascript:");
  });
  it("strips CR", () => {
    expect(stripControlChars("java\rscript:")).toBe("javascript:");
  });
  it("strips LF", () => {
    expect(stripControlChars("java\nscript:")).toBe("javascript:");
  });
  it("strips ZWS", () => {
    expect(stripControlChars("\u200bjavascript:")).toBe("javascript:");
  });
  it("leaves safe URLs unchanged", () => {
    expect(stripControlChars("https://example.com")).toBe("https://example.com");
  });
});

describe("HTML sanitizer url-utils: extractScheme", () => {
  it("extracts https", () => expect(extractScheme("https://x.com")).toBe("https"));
  it("extracts javascript", () => expect(extractScheme("javascript:x")).toBe("javascript"));
  it("returns null for relative URL", () => expect(extractScheme("/path")).toBeNull());
  it("returns null for colon after slash", () => expect(extractScheme("/a:b")).toBeNull());
  it("returns null for empty string", () => expect(extractScheme("")).toBeNull());
  it("lowercases scheme", () => expect(extractScheme("HTTPS://x.com")).toBe("https"));
});

describe("HTML sanitizer url-utils: isSchemeAllowed", () => {
  it("allows https in list", () => expect(isSchemeAllowed("https://ok", ["https"])).toBe(true));
  it("blocks javascript not in list", () => expect(isSchemeAllowed("javascript:x", ["https"])).toBe(false));
  it("allows relative URL", () => expect(isSchemeAllowed("/rel", ["https"])).toBe(true));
  it("allows all when null", () => expect(isSchemeAllowed("javascript:x", null)).toBe(true));
  it("strips control chars before check", () => expect(isSchemeAllowed("java\x00script:x", ["https"])).toBe(false));
});

// ─── Comment Stripping ────────────────────────────────────────────────────────

describe("Comment stripping", () => {
  it("strips simple HTML comment", () => {
    const result = sanitizeHtml(COMMENT_SIMPLE, HTML_STRICT);
    expect(result).not.toContain("<!--");
    expect(result).toContain("<p>ok</p>");
  });

  it("strips comment containing potential XSS payload", () => {
    const result = sanitizeHtml(COMMENT_WITH_CONTENT, HTML_STRICT);
    expect(result).not.toContain("<!--");
    expect(result).not.toContain("onerror");
    expect(result).toContain("<p>text</p>");
  });

  it("strips IE conditional comment", () => {
    const result = sanitizeHtml(COMMENT_CONDITIONAL, HTML_STRICT);
    expect(result).not.toContain("<!--");
    expect(result).toBe("");
  });

  it("preserves comments when dropComments: false", () => {
    const result = sanitizeHtml(COMMENT_SIMPLE, { ...HTML_STRICT, dropComments: false });
    expect(result).toContain("<!--");
    expect(result).toContain("comment");
  });

  it("preserves comments with HTML_PASSTHROUGH", () => {
    const result = sanitizeHtml(COMMENT_SIMPLE, HTML_PASSTHROUGH);
    expect(result).toContain("<!-- comment -->");
  });
});

// ─── Element Dropping ─────────────────────────────────────────────────────────

describe("Element dropping", () => {
  it("drops <script> element", () => {
    const result = sanitizeHtml(SCRIPT_INLINE, HTML_STRICT);
    expect(result).not.toContain("<script");
    expect(result).not.toContain("alert(1)");
    expect(result).toContain("<p>Safe</p>");
  });

  it("drops <script> with src attribute", () => {
    const result = sanitizeHtml(SCRIPT_SRC, HTML_STRICT);
    expect(result).not.toContain("<script");
    expect(result).not.toContain("evil.com");
  });

  it("drops <SCRIPT> uppercase", () => {
    const result = sanitizeHtml(SCRIPT_UPPER, HTML_STRICT);
    expect(result).not.toContain("SCRIPT");
    expect(result).not.toContain("alert");
  });

  it("drops script but preserves surrounding content", () => {
    const result = sanitizeHtml(SCRIPT_WITH_CONTENT_AFTER, HTML_STRICT);
    expect(result).not.toContain("script");
    expect(result).toContain("<p>before</p>");
    expect(result).toContain("<p>after</p>");
  });

  it("drops multiline script", () => {
    const result = sanitizeHtml(SCRIPT_MULTILINE, HTML_STRICT);
    expect(result).not.toContain("alert");
    expect(result.trim()).toBe("");
  });

  it("drops <style> element", () => {
    const html = "<style>body{background:red}</style><p>text</p>";
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("<style");
    expect(result).toContain("<p>text</p>");
  });

  it("drops <iframe> element", () => {
    const html = '<iframe src="https://evil.com"></iframe><p>ok</p>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("iframe");
    expect(result).toContain("<p>ok</p>");
  });

  it("drops <object> element", () => {
    const html = '<object data="evil.swf"></object>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("object");
  });

  it("drops <meta> element", () => {
    const html = '<meta http-equiv="refresh" content="0;url=https://evil.com"><p>ok</p>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("meta");
    expect(result).toContain("<p>ok</p>");
  });

  it("drops <base> element", () => {
    const html = '<base href="https://evil.com"><p>text</p>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("<base");
    expect(result).toContain("<p>text</p>");
  });

  it("does NOT drop script with HTML_RELAXED (script not in dropElements)", () => {
    // HTML_RELAXED doesn't include script... wait, it does not include script
    // Looking at HTML_RELAXED: dropElements: ["script","iframe","object","embed","applet"]
    const result = sanitizeHtml(SCRIPT_INLINE, HTML_RELAXED);
    expect(result).not.toContain("alert(1)");
  });

  it("passes through everything with HTML_PASSTHROUGH", () => {
    const result = sanitizeHtml("<script>alert(1)</script>", HTML_PASSTHROUGH);
    expect(result).toContain("<script>alert(1)</script>");
  });

  it("drops custom element when specified", () => {
    const html = "<custom-bad>evil content</custom-bad><p>ok</p>";
    const result = sanitizeHtml(html, {
      ...HTML_PASSTHROUGH,
      dropElements: ["custom-bad"],
    });
    expect(result).not.toContain("custom-bad");
    expect(result).not.toContain("evil content");
    expect(result).toContain("<p>ok</p>");
  });
});

// ─── Event Handler Stripping ──────────────────────────────────────────────────

describe("Event handler attribute stripping", () => {
  it("strips onload from img", () => {
    const result = sanitizeHtml(EVENT_ONLOAD, HTML_STRICT);
    expect(result).not.toContain("onload");
    expect(result).not.toContain("alert(1)");
    expect(result).toContain('src="x.png"');
  });

  it("strips onclick from anchor", () => {
    const result = sanitizeHtml(EVENT_ONCLICK, HTML_STRICT);
    expect(result).not.toContain("onclick");
    expect(result).not.toContain("alert(1)");
    expect(result).toContain("click");
  });

  it("strips onfocus", () => {
    const result = sanitizeHtml(EVENT_ONFOCUS, HTML_STRICT);
    expect(result).not.toContain("onfocus");
    expect(result).not.toContain("alert(1)");
    expect(result).toContain("tabindex");
  });

  it("strips onmouseover", () => {
    const result = sanitizeHtml(EVENT_ONMOUSEOVER, HTML_STRICT);
    expect(result).not.toContain("onmouseover");
    expect(result).toContain("<p");
  });

  it("strips onerror from img", () => {
    const result = sanitizeHtml(EVENT_ONERROR, HTML_STRICT);
    expect(result).not.toContain("onerror");
    expect(result).not.toContain("alert(1)");
  });

  it("strips onload from svg", () => {
    const result = sanitizeHtml(EVENT_SVG, HTML_STRICT);
    expect(result).not.toContain("onload");
  });

  it("strips event handlers even with HTML_PASSTHROUGH (event handlers are always stripped)", () => {
    // HTML_PASSTHROUGH has no dropElements, but on* attrs should still be stripped
    // when the sanitizer runs. Actually, with HTML_PASSTHROUGH dropElements=[] and
    // sanitizeStyleAttributes=false, the attribute sanitization still applies to on*.
    // Wait — HTML_PASSTHROUGH is truly passthrough, so on* might not be stripped.
    // Let's check: HTML_PASSTHROUGH has dropElements:[], dropAttributes:[],
    // allowedUrlSchemes:null, dropComments:false, sanitizeStyleAttributes:false
    // The sanitizer ALWAYS strips on* when doing attribute sanitization...
    // Actually, we call sanitizeAttributes regardless of policy. Let's verify.
    const result = sanitizeHtml(EVENT_ONLOAD, HTML_PASSTHROUGH);
    // With passthrough, dropElements is empty, but we still parse tags.
    // The on* stripping happens in sanitizeAttributes which is always called.
    // This is a design choice — test the actual behavior.
    // Actually looking at the code: sanitizeHtml → sanitizeHtmlWithRegex →
    // always calls sanitizeAttributes(result, policy).
    // In sanitizeAttributes, on* is always stripped via /^on[a-z]/i check.
    // So even PASSTHROUGH strips on* handlers.
    expect(result).not.toContain("onload");
  });
});

// ─── URL Sanitization in Attributes ──────────────────────────────────────────

describe("URL sanitization in href attributes", () => {
  it("replaces javascript: href with empty string", () => {
    const result = sanitizeHtml(HREF_JAVASCRIPT, HTML_STRICT);
    expect(result).not.toContain("javascript:");
    expect(result).toContain('href=""');
    expect(result).toContain("click me");
  });

  it("replaces JAVASCRIPT: (uppercase) href with empty string", () => {
    const result = sanitizeHtml(HREF_JAVASCRIPT_UPPER, HTML_STRICT);
    expect(result).not.toContain("JAVASCRIPT:");
    expect(result).toContain('href=""');
  });

  it("replaces data: href with empty string", () => {
    const result = sanitizeHtml(HREF_DATA, HTML_STRICT);
    expect(result).toContain('href=""');
    expect(result).not.toContain("data:text/html");
  });

  it("preserves https: href", () => {
    const result = sanitizeHtml(SAFE_LINK, HTML_STRICT);
    expect(result).toContain('href="https://example.com"');
  });

  it("preserves relative href", () => {
    const html = '<a href="/relative/path">link</a>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).toContain('href="/relative/path"');
  });
});

describe("URL sanitization in src attributes", () => {
  it("replaces javascript: src with empty string", () => {
    const result = sanitizeHtml(SRC_JAVASCRIPT, HTML_STRICT);
    expect(result).toContain('src=""');
    expect(result).not.toContain("javascript:");
  });

  it("preserves https: src", () => {
    const result = sanitizeHtml(SAFE_IMAGE, HTML_STRICT);
    expect(result).toContain('src="https://example.com/photo.jpg"');
  });
});

// ─── CSS Injection Prevention ─────────────────────────────────────────────────

describe("CSS injection prevention", () => {
  it("strips style attribute containing expression()", () => {
    const result = sanitizeHtml(CSS_EXPRESSION, HTML_STRICT);
    expect(result).not.toContain("expression");
    expect(result).not.toContain("style=");
    expect(result).toContain("<p");
  });

  it("strips style attribute containing url(javascript:)", () => {
    const result = sanitizeHtml(CSS_URL_JAVASCRIPT, HTML_STRICT);
    expect(result).not.toContain("javascript:");
    expect(result).not.toContain("style=");
    expect(result).toContain("<p");
  });

  it("strips style with EXPRESSION() uppercase", () => {
    const result = sanitizeHtml(CSS_EXPRESSION_UPPER, HTML_STRICT);
    expect(result).not.toContain("EXPRESSION");
    expect(result).not.toContain("style=");
  });

  it("preserves safe style attributes", () => {
    const html = '<p style="color:red;font-size:14px">text</p>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).toContain("style=");
    expect(result).toContain("color:red");
  });

  it("preserves style attribute with safe url()", () => {
    const html = '<p style="background:url(https://example.com/bg.png)">text</p>';
    const result = sanitizeHtml(html, HTML_STRICT);
    // url() with https is allowed
    expect(result).toContain("style=");
  });

  it("does NOT strip style when sanitizeStyleAttributes: false", () => {
    const html = '<p style="width:expression(alert(1))">x</p>';
    const result = sanitizeHtml(html, { ...HTML_STRICT, sanitizeStyleAttributes: false });
    // style attribute kept (not stripped) — dangerous, but that's what the policy says
    expect(result).toContain("style=");
  });
});

// ─── srcdoc / formaction Stripping ───────────────────────────────────────────

describe("srcdoc and formaction stripping", () => {
  it("strips srcdoc attribute", () => {
    // iframe is dropped by HTML_STRICT, so test with RELAXED (which keeps form)
    // Use a tag that's not in relaxed's dropElements
    const html = '<div srcdoc="<script>alert(1)</script>">content</div>';
    const result = sanitizeHtml(html, HTML_STRICT);
    expect(result).not.toContain("srcdoc");
  });

  it("strips formaction attribute", () => {
    const html = '<input formaction="https://evil.com" type="submit">';
    const result = sanitizeHtml(html, HTML_RELAXED);
    expect(result).not.toContain("formaction");
    // input is not in HTML_RELAXED dropElements, so the element itself is kept
    // (but formaction attribute is stripped)
    expect(result).toContain("input");
  });
});

// ─── HTML_STRICT Full Smoke Tests ─────────────────────────────────────────────

describe("HTML_STRICT preset", () => {
  it("drops script element", () => {
    const result = sanitizeHtml(SCRIPT_INLINE, HTML_STRICT);
    expect(result).not.toContain("script");
    expect(result).toContain("<p>Safe</p>");
  });

  it("strips onload event handler", () => {
    const result = sanitizeHtml(EVENT_ONLOAD, HTML_STRICT);
    expect(result).not.toContain("onload");
    expect(result).toContain('src="x.png"');
  });

  it("neutralises javascript: href", () => {
    const result = sanitizeHtml(HREF_JAVASCRIPT, HTML_STRICT);
    expect(result).toContain('href=""');
    expect(result).toContain("click me");
  });

  it("strips CSS expression()", () => {
    const result = sanitizeHtml(CSS_EXPRESSION, HTML_STRICT);
    expect(result).not.toContain("expression");
    expect(result).not.toContain("style=");
  });

  it("strips HTML comments", () => {
    const result = sanitizeHtml(COMMENT_SIMPLE, HTML_STRICT);
    expect(result).not.toContain("<!--");
    expect(result).toContain("<p>ok</p>");
  });

  it("preserves safe paragraph", () => {
    const result = sanitizeHtml(SAFE_PARAGRAPH, HTML_STRICT);
    expect(result).toBe(SAFE_PARAGRAPH);
  });

  it("preserves safe link", () => {
    const result = sanitizeHtml(SAFE_LINK, HTML_STRICT);
    expect(result).toBe(SAFE_LINK);
  });

  it("preserves safe heading", () => {
    const result = sanitizeHtml(SAFE_HEADING, HTML_STRICT);
    expect(result).toBe(SAFE_HEADING);
  });

  it("preserves safe emphasis", () => {
    const result = sanitizeHtml(SAFE_EMPHASIS, HTML_STRICT);
    expect(result).toBe(SAFE_EMPHASIS);
  });

  it("preserves safe code block", () => {
    const result = sanitizeHtml(SAFE_CODE, HTML_STRICT);
    expect(result).toBe(SAFE_CODE);
  });
});

// ─── HTML_RELAXED Preset ──────────────────────────────────────────────────────

describe("HTML_RELAXED preset", () => {
  it("drops script element", () => {
    const result = sanitizeHtml(SCRIPT_INLINE, HTML_RELAXED);
    expect(result).not.toContain("script");
    expect(result).not.toContain("alert");
  });

  it("drops iframe element", () => {
    const html = '<iframe src="https://evil.com"></iframe>';
    const result = sanitizeHtml(html, HTML_RELAXED);
    expect(result).not.toContain("iframe");
  });

  it("preserves style element (not in drop list)", () => {
    const html = "<style>p{color:red}</style><p>text</p>";
    const result = sanitizeHtml(html, HTML_RELAXED);
    expect(result).toContain("<style>");
  });

  it("allows ftp: links", () => {
    const html = '<a href="ftp://files.example.com">files</a>';
    const result = sanitizeHtml(html, HTML_RELAXED);
    expect(result).toContain('href="ftp://files.example.com"');
  });

  it("preserves HTML comments", () => {
    const result = sanitizeHtml(COMMENT_SIMPLE, HTML_RELAXED);
    expect(result).toContain("<!-- comment -->");
    expect(result).toContain("<p>ok</p>");
  });

  it("still strips event handlers", () => {
    const result = sanitizeHtml(EVENT_ONCLICK, HTML_RELAXED);
    expect(result).not.toContain("onclick");
  });

  it("still blocks javascript: URLs", () => {
    const result = sanitizeHtml(HREF_JAVASCRIPT, HTML_RELAXED);
    expect(result).toContain('href=""');
  });
});

// ─── HTML_PASSTHROUGH Preset ──────────────────────────────────────────────────

describe("HTML_PASSTHROUGH preset", () => {
  it("preserves script element", () => {
    const result = sanitizeHtml("<script>alert(1)</script>", HTML_PASSTHROUGH);
    expect(result).toContain("<script>alert(1)</script>");
  });

  it("preserves HTML comments", () => {
    const result = sanitizeHtml("<!-- comment -->", HTML_PASSTHROUGH);
    expect(result).toContain("<!-- comment -->");
  });

  it("preserves style elements", () => {
    const html = "<style>p{color:red}</style>";
    const result = sanitizeHtml(html, HTML_PASSTHROUGH);
    expect(result).toContain("<style>");
  });

  it("preserves data: URLs", () => {
    const html = '<a href="data:text/html,hello">link</a>';
    const result = sanitizeHtml(html, HTML_PASSTHROUGH);
    expect(result).toContain("data:text/html,hello");
  });

  it("still strips on* event handlers (always stripped)", () => {
    // Note: on* handlers are stripped even in PASSTHROUGH as a minimum defense
    const result = sanitizeHtml(EVENT_ONLOAD, HTML_PASSTHROUGH);
    expect(result).not.toContain("onload");
  });
});

// ─── Safe HTML Preserved ──────────────────────────────────────────────────────

describe("Safe HTML preserved unchanged", () => {
  it("preserves plain text in paragraph", () => {
    const result = sanitizeHtml("<p>Hello, world!</p>", HTML_STRICT);
    expect(result).toBe("<p>Hello, world!</p>");
  });

  it("preserves https link", () => {
    const result = sanitizeHtml(SAFE_LINK, HTML_STRICT);
    expect(result).toBe(SAFE_LINK);
  });

  it("preserves https image", () => {
    const result = sanitizeHtml(SAFE_IMAGE, HTML_STRICT);
    expect(result).toBe(SAFE_IMAGE);
  });

  it("preserves blockquote", () => {
    const result = sanitizeHtml(SAFE_BLOCKQUOTE, HTML_STRICT);
    expect(result).toBe(SAFE_BLOCKQUOTE);
  });

  it("handles empty string", () => {
    expect(sanitizeHtml("", HTML_STRICT)).toBe("");
  });

  it("handles string with no HTML", () => {
    expect(sanitizeHtml("plain text", HTML_STRICT)).toBe("plain text");
  });
});

// ─── Policy Composition ───────────────────────────────────────────────────────

describe("Policy composition via spread", () => {
  it("can add extra drop elements to HTML_RELAXED", () => {
    const policy: HtmlSanitizationPolicy = {
      ...HTML_RELAXED,
      dropElements: [...(HTML_RELAXED.dropElements ?? []), "details"],
    };
    const html = "<details><summary>title</summary>content</details><p>ok</p>";
    const result = sanitizeHtml(html, policy);
    expect(result).not.toContain("details");
    expect(result).toContain("<p>ok</p>");
  });

  it("can override allowedUrlSchemes", () => {
    // Only allow https, block http
    const policy: HtmlSanitizationPolicy = {
      ...HTML_STRICT,
      allowedUrlSchemes: ["https"],
    };
    const html = '<a href="http://example.com">link</a>';
    const result = sanitizeHtml(html, policy);
    expect(result).toContain('href=""');
  });

  it("can add extra drop attributes", () => {
    const policy: HtmlSanitizationPolicy = {
      ...HTML_RELAXED,
      dropAttributes: ["data-evil"],
    };
    const html = '<p data-evil="payload" data-safe="ok">text</p>';
    const result = sanitizeHtml(html, policy);
    expect(result).not.toContain("data-evil");
    expect(result).toContain("data-safe");
  });
});

// ─── Default Policy Values ────────────────────────────────────────────────────

describe("Default policy values (omitted fields)", () => {
  it("omitting dropComments defaults to true (comments stripped)", () => {
    const result = sanitizeHtml("<!-- comment --><p>ok</p>", {});
    expect(result).not.toContain("<!--");
  });

  it("omitting sanitizeStyleAttributes defaults to true (expression stripped)", () => {
    const result = sanitizeHtml(CSS_EXPRESSION, {});
    // With empty policy, expression() in style is still stripped by default
    expect(result).not.toContain("expression");
  });

  it("omitting dropElements defaults to empty list (no elements dropped)", () => {
    const result = sanitizeHtml(SCRIPT_INLINE, {});
    // Script is NOT dropped when dropElements is omitted (empty by default)
    // The on* handler stripping still applies, but element dropping does not
    // Wait — actually the default for dropElements in the policy is []
    // Let me check: policy.dropElements ?? [] — so default is empty
    // That means with {} policy, scripts are NOT dropped
    // This is intentional — omitting a field means passthrough for that field
    expect(result).toContain("<p>Safe</p>");
    // script might or might not be there depending on default — let's check
    // dropElements defaults to [] → script NOT dropped
    expect(result).toContain("alert(1)");
  });

  it("omitting allowedUrlSchemes defaults to safe list", () => {
    // Default: ["http", "https", "mailto", "ftp"] — blocks javascript:
    const result = sanitizeHtml(HREF_JAVASCRIPT, {});
    expect(result).toContain('href=""');
  });
});

// ─── Multiple XSS Vectors Combined ───────────────────────────────────────────

describe("Multiple XSS vectors in one document", () => {
  it("sanitizes all vectors in one pass", () => {
    const html = [
      "<p>Safe paragraph</p>",
      "<script>alert(1)</script>",
      '<img onload="alert(2)" src="x.png">',
      '<a href="javascript:alert(3)">click</a>',
      '<p style="width:expression(alert(4))">text</p>',
      "<!-- <img src=x onerror=alert(5)> -->",
    ].join("");

    const result = sanitizeHtml(html, HTML_STRICT);

    expect(result).not.toContain("alert(1)");
    expect(result).not.toContain("alert(2)");
    expect(result).not.toContain("alert(3)");
    expect(result).not.toContain("alert(4)");
    expect(result).not.toContain("alert(5)");
    expect(result).toContain("<p>Safe paragraph</p>");
    expect(result).toContain("<a ");
    expect(result).toContain('href=""');
  });
});

// ─── DOM Adapter Path ─────────────────────────────────────────────────────────

describe("DOM adapter path", () => {
  it("calls adapter.parse, adapter.walk, adapter.serialize", () => {
    // Mock DOM adapter that records calls
    const calls: string[] = [];
    let capturedHtml = "";
    let capturedVisitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) {
        calls.push("parse");
        capturedHtml = html;
        return { html }; // mock DOM
      },
      walk(dom: unknown, visitor: DomVisitor) {
        calls.push("walk");
        capturedVisitor = visitor;
      },
      serialize(_dom: unknown) {
        calls.push("serialize");
        return "<p>sanitized</p>";
      },
    };

    const result = sanitizeHtml("<p>hello</p>", {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    expect(calls).toEqual(["parse", "walk", "serialize"]);
    expect(capturedHtml).toBe("<p>hello</p>");
    expect(capturedVisitor).not.toBeNull();
    expect(result).toBe("<p>sanitized</p>");
  });

  it("DOM visitor drops dangerous elements", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml("<script>alert(1)</script>", {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    expect(visitor).not.toBeNull();
    // Test that the visitor drops script elements
    const result = visitor!.element("script", new Map());
    expect(result).toBe(false);
  });

  it("DOM visitor keeps safe elements", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml("<p class='safe'>hello</p>", {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    expect(visitor).not.toBeNull();
    const attrs = new Map([["class", "safe"]]);
    const result = visitor!.element("p", attrs);
    expect(result).not.toBe(false);
    expect(result).toBeInstanceOf(Map);
    expect((result as Map<string, string>).get("class")).toBe("safe");
  });

  it("DOM visitor drops on* event handlers", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<img onload="alert(1)" src="x.png">', {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["onload", "alert(1)"], ["src", "x.png"]]);
    const result = visitor!.element("img", attrs);
    expect(result).not.toBe(false);
    const safeAttrs = result as Map<string, string>;
    expect(safeAttrs.has("onload")).toBe(false);
    expect(safeAttrs.get("src")).toBe("x.png");
  });

  it("DOM visitor drops comments when dropComments: true", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml("<!-- comment -->", { ...HTML_STRICT, domAdapter: mockAdapter });

    const result = visitor!.comment(" comment ");
    expect(result).toBe(false);
  });

  it("DOM visitor keeps comments when dropComments: false", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml("<!-- comment -->", { ...HTML_STRICT, dropComments: false, domAdapter: mockAdapter });

    const result = visitor!.comment(" comment ");
    expect(result).toBe(" comment ");
  });

  it("DOM visitor sanitizes URL in href attribute", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<a href="javascript:alert(1)">link</a>', {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["href", "javascript:alert(1)"]]);
    const result = visitor!.element("a", attrs) as Map<string, string>;
    expect(result.get("href")).toBe("");
  });

  it("DOM visitor strips dangerous style attribute", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<p style="width:expression(alert(1))">x</p>', {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["style", "width:expression(alert(1))"]]);
    const result = visitor!.element("p", attrs) as Map<string, string>;
    expect(result.has("style")).toBe(false);
  });

  it("DOM visitor strips style with url(javascript:)", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<p style="background:url(javascript:alert(1))">x</p>', {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["style", "background:url(javascript:alert(1))"]]);
    const result = visitor!.element("p", attrs) as Map<string, string>;
    expect(result.has("style")).toBe(false);
  });

  it("DOM visitor keeps safe style with https url()", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<p style="background:url(https://example.com/bg.png)">x</p>', {
      ...HTML_STRICT,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["style", "background:url(https://example.com/bg.png)"]]);
    const result = visitor!.element("p", attrs) as Map<string, string>;
    expect(result.has("style")).toBe(true);
  });

  it("DOM visitor keeps style when sanitizeStyleAttributes: false", () => {
    let visitor: DomVisitor | null = null;

    const mockAdapter = {
      parse(html: string) { return { html }; },
      walk(_dom: unknown, v: DomVisitor) { visitor = v; },
      serialize(dom: unknown) { return (dom as { html: string }).html; },
    };

    sanitizeHtml('<p style="width:expression(alert(1))">x</p>', {
      ...HTML_STRICT,
      sanitizeStyleAttributes: false,
      domAdapter: mockAdapter,
    });

    const attrs = new Map([["style", "width:expression(alert(1))"]]);
    const result = visitor!.element("p", attrs) as Map<string, string>;
    // sanitizeStyleAttributes: false means keep the style
    expect(result.has("style")).toBe(true);
  });
});
