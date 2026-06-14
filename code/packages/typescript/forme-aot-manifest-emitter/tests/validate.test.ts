/**
 * validate.test.ts — field-level validators.
 */

import { describe, it, expect } from "vitest";
import {
  validateColor,
  validateDisplay,
  validateManifestUrl,
} from "../src/index.js";

describe("validateManifestUrl — accepts", () => {
  it("https://", () => {
    expect(validateManifestUrl("https://example.com/icon.png", "f"))
      .toBe("https://example.com/icon.png");
  });
  it("http://", () => {
    expect(validateManifestUrl("http://example.com/icon.png", "f"))
      .toBe("http://example.com/icon.png");
  });
  it("case-insensitive scheme", () => {
    expect(validateManifestUrl("HTTPS://example.com", "f")).toBe("HTTPS://example.com");
  });
  it("root-relative path", () => {
    expect(validateManifestUrl("/icon.png", "f")).toBe("/icon.png");
  });
  it("bare /", () => {
    expect(validateManifestUrl("/", "f")).toBe("/");
  });
  it("multi-segment path", () => {
    expect(validateManifestUrl("/icons/192.png", "f")).toBe("/icons/192.png");
  });
});

describe("validateManifestUrl — rejects", () => {
  it("javascript:", () => {
    expect(() => validateManifestUrl("javascript:alert(1)", "f")).toThrow(/http\(s\)/);
  });
  it("data:", () => {
    expect(() => validateManifestUrl("data:text/html,x", "f")).toThrow(/http\(s\)/);
  });
  it("file:", () => {
    expect(() => validateManifestUrl("file:///etc", "f")).toThrow(/http\(s\)/);
  });
  it("protocol-relative //host", () => {
    expect(() => validateManifestUrl("//evil.com", "f")).toThrow(/http\(s\)/);
  });
  it("/\\host (backslash variant)", () => {
    expect(() => validateManifestUrl("/\\evil.com", "f")).toThrow(/http\(s\)/);
  });
  it("bare relative", () => {
    expect(() => validateManifestUrl("icon.png", "f")).toThrow(/http\(s\)/);
  });
  it("empty string", () => {
    expect(() => validateManifestUrl("", "f")).toThrow(/non-empty/);
  });
  it("non-string", () => {
    expect(() => validateManifestUrl(42, "f")).toThrow(/non-empty/);
  });
  it("null", () => {
    expect(() => validateManifestUrl(null, "f")).toThrow(/null/);
  });
  it("error includes field name", () => {
    try {
      validateManifestUrl("", "my-field");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("my-field");
    }
  });
  it("long unsafe URL truncated", () => {
    const long = "ftp://" + "x".repeat(500);
    try {
      validateManifestUrl(long, "f");
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toMatch(/…/);
      expect(msg.length).toBeLessThan(400);
    }
  });
});

describe("validateDisplay — accepts allowlist", () => {
  const allowed = ["fullscreen", "standalone", "minimal-ui", "browser"];
  for (const v of allowed) {
    it(`'${v}' accepted`, () => {
      expect(validateDisplay(v)).toBe(v);
    });
  }

  it("case-insensitive: 'Standalone' → 'standalone'", () => {
    expect(validateDisplay("Standalone")).toBe("standalone");
  });
  it("case-insensitive: 'FULLSCREEN' → 'fullscreen'", () => {
    expect(validateDisplay("FULLSCREEN")).toBe("fullscreen");
  });
});

describe("validateDisplay — rejects", () => {
  it("'tab' (not in allowlist)", () => {
    expect(() => validateDisplay("tab"))
      .toThrow(/one of \[fullscreen, standalone, minimal-ui, browser\]/);
  });
  it("'window-controls-overlay' (newer spec extension)", () => {
    expect(() => validateDisplay("window-controls-overlay")).toThrow(/one of/);
  });
  it("empty string", () => {
    expect(() => validateDisplay("")).toThrow(/one of/);
  });
  it("non-string", () => {
    // @ts-expect-error
    expect(() => validateDisplay(42)).toThrow(/string/);
  });
  it("null", () => {
    // @ts-expect-error
    expect(() => validateDisplay(null)).toThrow(/string/);
  });
  it("error contains bad value", () => {
    try {
      validateDisplay("tab");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain('"tab"');
    }
  });
});

describe("validateColor — accepts hex", () => {
  it("3-digit", () => expect(validateColor("#abc", "c")).toBe("#abc"));
  it("4-digit (with alpha)", () => expect(validateColor("#abcd", "c")).toBe("#abcd"));
  it("6-digit", () => expect(validateColor("#aabbcc", "c")).toBe("#aabbcc"));
  it("8-digit (with alpha)", () => expect(validateColor("#aabbccdd", "c")).toBe("#aabbccdd"));
  it("uppercase hex", () => expect(validateColor("#AABBCC", "c")).toBe("#AABBCC"));
  it("mixed case hex", () => expect(validateColor("#aAbBcC", "c")).toBe("#aAbBcC"));
});

describe("validateColor — rejects", () => {
  it("missing #", () => {
    expect(() => validateColor("aabbcc", "c")).toThrow(/hex colour/);
  });
  it("named colour 'red'", () => {
    expect(() => validateColor("red", "c")).toThrow(/hex colour/);
  });
  it("rgb()", () => {
    expect(() => validateColor("rgb(255, 0, 0)", "c")).toThrow(/hex colour/);
  });
  it("rgba()", () => {
    expect(() => validateColor("rgba(255, 0, 0, 0.5)", "c")).toThrow(/hex colour/);
  });
  it("hsl()", () => {
    expect(() => validateColor("hsl(0, 100%, 50%)", "c")).toThrow(/hex colour/);
  });
  it("'#xyz' (non-hex chars)", () => {
    expect(() => validateColor("#xyz", "c")).toThrow(/hex colour/);
  });
  it("5-digit hex (invalid length)", () => {
    expect(() => validateColor("#abcde", "c")).toThrow(/hex colour/);
  });
  it("7-digit hex (invalid length)", () => {
    expect(() => validateColor("#abcdefg", "c")).toThrow(/hex colour/);
  });
  it("empty string", () => {
    expect(() => validateColor("", "c")).toThrow(/hex colour/);
  });
  it("non-string", () => {
    // @ts-expect-error
    expect(() => validateColor(0xff0000, "c")).toThrow(/string/);
  });
  it("error includes field name", () => {
    try {
      validateColor("red", "my-theme-field");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("my-theme-field");
    }
  });
});
