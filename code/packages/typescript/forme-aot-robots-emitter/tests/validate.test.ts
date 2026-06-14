/**
 * validate.test.ts — field-level validators.
 */

import { describe, it, expect } from "vitest";
import {
  validateCrawlDelay,
  validateDirectiveValue,
  validateHost,
  validateSitemapUrl,
} from "../src/index.js";

describe("validateDirectiveValue — accepts", () => {
  it("plain string", () => {
    expect(validateDirectiveValue("Googlebot", "ua")).toBe("Googlebot");
  });

  it("path with /", () => {
    expect(validateDirectiveValue("/admin", "p")).toBe("/admin");
  });

  it("wildcard *", () => {
    expect(validateDirectiveValue("*", "ua")).toBe("*");
  });

  it("path with $", () => {
    expect(validateDirectiveValue("/*.pdf$", "p")).toBe("/*.pdf$");
  });

  it("path with %-encoding", () => {
    expect(validateDirectiveValue("/path%20with%20spaces", "p")).toBe("/path%20with%20spaces");
  });

  it("path containing tab (TAB allowed per code comment)", () => {
    // TAB is a C0 char but it's commonly used as whitespace in
    // some user-agent strings; we permit it.
    expect(validateDirectiveValue("Foo\tBar", "ua")).toBe("Foo\tBar");
  });
});

describe("validateDirectiveValue — rejects (header injection)", () => {
  it("rejects LF (\\n) — line splitting attack", () => {
    expect(() => validateDirectiveValue("good\nDisallow: /evil", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects CR (\\r)", () => {
    expect(() => validateDirectiveValue("good\rbad", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects CR+LF (\\r\\n)", () => {
    expect(() => validateDirectiveValue("good\r\nbad", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects NUL (\\x00)", () => {
    expect(() => validateDirectiveValue("a\x00b", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects DEL (\\x7F)", () => {
    expect(() => validateDirectiveValue("a\x7Fb", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects ESC (\\x1B)", () => {
    expect(() => validateDirectiveValue("a\x1Bb", "ua"))
      .toThrow(/forbidden control character/);
  });

  it("rejects all other C0 controls", () => {
    for (let c = 0; c < 32; c++) {
      if (c === 9) continue;  // TAB allowed
      expect(() => validateDirectiveValue(`a${String.fromCharCode(c)}b`, "ua"))
        .toThrow(/forbidden control character/);
    }
  });

  it("rejects empty string", () => {
    expect(() => validateDirectiveValue("", "ua")).toThrow(/non-empty string/);
  });

  it("rejects non-string", () => {
    // @ts-expect-error
    expect(() => validateDirectiveValue(42, "ua")).toThrow(/non-empty string/);
  });

  it("rejects null", () => {
    // @ts-expect-error
    expect(() => validateDirectiveValue(null, "ua")).toThrow(/null/);
  });

  it("error message contains field name", () => {
    try {
      validateDirectiveValue("", "my-special-field");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("my-special-field");
    }
  });
});

describe("validateCrawlDelay — accepts", () => {
  it("zero", () => expect(validateCrawlDelay(0)).toBe(0));
  it("positive integer", () => expect(validateCrawlDelay(5)).toBe(5));
  it("large integer", () => expect(validateCrawlDelay(86400)).toBe(86400));
});

describe("validateCrawlDelay — rejects", () => {
  it("negative", () => expect(() => validateCrawlDelay(-1)).toThrow(/non-negative/));
  it("NaN", () => expect(() => validateCrawlDelay(NaN)).toThrow(/finite/));
  it("Infinity", () => expect(() => validateCrawlDelay(Infinity)).toThrow(/finite/));
  it("-Infinity", () => expect(() => validateCrawlDelay(-Infinity)).toThrow(/finite/));
  it("fractional", () => expect(() => validateCrawlDelay(1.5)).toThrow(/integer/));
  it("non-number", () => {
    // @ts-expect-error
    expect(() => validateCrawlDelay("5")).toThrow(/finite/);
  });
});

describe("validateSitemapUrl — accepts", () => {
  it("https://", () => {
    expect(validateSitemapUrl("https://example.com/sitemap.xml"))
      .toBe("https://example.com/sitemap.xml");
  });

  it("http://", () => {
    expect(validateSitemapUrl("http://example.com/sitemap.xml"))
      .toBe("http://example.com/sitemap.xml");
  });

  it("case-insensitive scheme", () => {
    expect(validateSitemapUrl("HTTPS://example.com/x")).toBe("HTTPS://example.com/x");
  });

  it("with port + query", () => {
    expect(validateSitemapUrl("https://example.com:8443/sitemap.xml?v=1"))
      .toBe("https://example.com:8443/sitemap.xml?v=1");
  });
});

describe("validateSitemapUrl — rejects", () => {
  it("root-relative (must be absolute)", () => {
    expect(() => validateSitemapUrl("/sitemap.xml")).toThrow(/http\(s\)/);
  });

  it("javascript:", () => {
    expect(() => validateSitemapUrl("javascript:alert(1)")).toThrow(/http\(s\)/);
  });

  it("file:", () => {
    expect(() => validateSitemapUrl("file:///etc")).toThrow(/http\(s\)/);
  });

  it("data:", () => {
    expect(() => validateSitemapUrl("data:text/html,x")).toThrow(/http\(s\)/);
  });

  it("protocol-relative", () => {
    expect(() => validateSitemapUrl("//example.com")).toThrow(/http\(s\)/);
  });

  it("empty string", () => {
    expect(() => validateSitemapUrl("")).toThrow(/non-empty/);
  });

  it("non-string", () => {
    // @ts-expect-error
    expect(() => validateSitemapUrl(42)).toThrow(/non-empty/);
  });

  it("LF injection in URL", () => {
    expect(() => validateSitemapUrl("https://good.com/\nDisallow: /evil"))
      .toThrow(/forbidden control character/);
  });

  it("long unsafe URL truncated in error", () => {
    const long = "ftp://" + "x".repeat(500);
    try {
      validateSitemapUrl(long);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toMatch(/…/);
      expect(msg.length).toBeLessThan(400);
    }
  });
});

describe("validateHost — accepts", () => {
  it("bare hostname", () => expect(validateHost("example.com")).toBe("example.com"));
  it("hostname with port", () => expect(validateHost("example.com:8080")).toBe("example.com:8080"));
  it("subdomain", () => expect(validateHost("www.example.co.uk")).toBe("www.example.co.uk"));
  it("IP address", () => expect(validateHost("192.168.1.1")).toBe("192.168.1.1"));
});

describe("validateHost — rejects", () => {
  it("URL with scheme", () => {
    expect(() => validateHost("https://example.com")).toThrow(/not a URL/);
  });

  it("URL without scheme but with path", () => {
    expect(() => validateHost("example.com/path")).toThrow(/no path/);
  });

  it("URL with query", () => {
    expect(() => validateHost("example.com?a=1")).toThrow(/no path/);
  });

  it("URL with fragment", () => {
    expect(() => validateHost("example.com#frag")).toThrow(/no path/);
  });

  it("contains space", () => {
    expect(() => validateHost("example .com")).toThrow(/no path.*spaces/);
  });

  it("LF injection", () => {
    expect(() => validateHost("good.com\nDisallow: /evil"))
      .toThrow(/forbidden control character/);
  });

  it("empty string", () => {
    expect(() => validateHost("")).toThrow(/non-empty/);
  });

  it("non-string", () => {
    // @ts-expect-error
    expect(() => validateHost(42)).toThrow(/non-empty/);
  });

  it("null", () => {
    // @ts-expect-error
    expect(() => validateHost(null)).toThrow(/null/);
  });
});
