/**
 * url.test.ts — isInternalSlug + assertResolvedUrl.
 */

import { describe, it, expect } from "vitest";
import { isInternalSlug, assertResolvedUrl } from "../src/index.js";

describe("isInternalSlug — accepts", () => {
  it("root-relative path", () => {
    expect(isInternalSlug("/about")).toBe(true);
  });

  it("multi-segment root-relative", () => {
    expect(isInternalSlug("/blog/2026/post")).toBe(true);
  });

  it("bare /", () => {
    expect(isInternalSlug("/")).toBe(true);
  });
});

describe("isInternalSlug — rejects", () => {
  it("absolute http", () => {
    expect(isInternalSlug("http://example.com/x")).toBe(false);
  });

  it("absolute https", () => {
    expect(isInternalSlug("https://example.com/x")).toBe(false);
  });

  it("protocol-relative //host", () => {
    expect(isInternalSlug("//example.com/x")).toBe(false);
  });

  it("bare relative", () => {
    expect(isInternalSlug("about")).toBe(false);
  });

  it("./about", () => {
    expect(isInternalSlug("./about")).toBe(false);
  });

  it("mailto:", () => {
    expect(isInternalSlug("mailto:x@y.com")).toBe(false);
  });

  it("javascript:", () => {
    expect(isInternalSlug("javascript:alert(1)")).toBe(false);
  });

  it("empty string", () => {
    expect(isInternalSlug("")).toBe(false);
  });

  it("non-string", () => {
    // @ts-expect-error — defensive runtime check
    expect(isInternalSlug(42)).toBe(false);
  });

  it("fragment only", () => {
    expect(isInternalSlug("#anchor")).toBe(false);
  });

  it("rejects /\\evil.com (browser may normalise \\ to / producing //evil.com)", () => {
    expect(isInternalSlug("/\\evil.com")).toBe(false);
  });
});

describe("assertResolvedUrl — accepts", () => {
  it("http://...", () => {
    expect(() => assertResolvedUrl("http://example.com/x")).not.toThrow();
  });

  it("https://...", () => {
    expect(() => assertResolvedUrl("https://example.com/x")).not.toThrow();
  });

  it("HTTP scheme case-insensitive", () => {
    expect(() => assertResolvedUrl("HTTPS://example.com")).not.toThrow();
    expect(() => assertResolvedUrl("Http://example.com")).not.toThrow();
  });

  it("https with port + query + fragment", () => {
    expect(() => assertResolvedUrl("https://example.com:8443/x?a=1#y")).not.toThrow();
  });

  it("root-relative path", () => {
    expect(() => assertResolvedUrl("/about")).not.toThrow();
  });

  it("bare /", () => {
    expect(() => assertResolvedUrl("/")).not.toThrow();
  });
});

describe("assertResolvedUrl — rejects", () => {
  it("javascript:", () => {
    expect(() => assertResolvedUrl("javascript:alert(1)")).toThrow(/unsafe URL/);
  });

  it("data:", () => {
    expect(() => assertResolvedUrl("data:text/html,<script>")).toThrow(/unsafe URL/);
  });

  it("file:", () => {
    expect(() => assertResolvedUrl("file:///etc/passwd")).toThrow(/unsafe URL/);
  });

  it("vbscript:", () => {
    expect(() => assertResolvedUrl("vbscript:msgbox(1)")).toThrow(/unsafe URL/);
  });

  it("protocol-relative //host", () => {
    expect(() => assertResolvedUrl("//evil.com")).toThrow(/unsafe URL/);
  });

  it("backslash variant /\\evil.com (browser \\-to-/ normalisation)", () => {
    expect(() => assertResolvedUrl("/\\evil.com")).toThrow(/unsafe URL/);
  });

  it("bare relative", () => {
    expect(() => assertResolvedUrl("about")).toThrow(/unsafe URL/);
  });

  it("mailto:", () => {
    expect(() => assertResolvedUrl("mailto:x@y.com")).toThrow(/unsafe URL/);
  });

  it("empty string mentions empty in message", () => {
    expect(() => assertResolvedUrl("")).toThrow(/empty string/);
  });

  it("null mentions null in message", () => {
    expect(() => assertResolvedUrl(null)).toThrow(/null/);
  });

  it("undefined mentions undefined in message", () => {
    expect(() => assertResolvedUrl(undefined)).toThrow(/undefined/);
  });

  it("number mentions number in message", () => {
    expect(() => assertResolvedUrl(42)).toThrow(/number/);
  });

  it("long URL truncates to ~200 chars + ellipsis in message", () => {
    const long = "javascript:" + "a".repeat(500);
    try {
      assertResolvedUrl(long);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      // Should contain truncated URL with `…`
      expect(msg).toMatch(/…/);
      // And the message itself shouldn't be enormous.
      expect(msg.length).toBeLessThan(400);
    }
  });
});
