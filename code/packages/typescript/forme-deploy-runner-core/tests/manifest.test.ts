import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  canonicalDeployManifest,
  contentDigestToStoreKey,
  DEPLOY_LIMITS,
  parseDeployManifest,
  validateOutputPath,
} from "../src/index.js";

const digest = (value: string): string =>
  createHash("sha256").update(value).digest("base64");

function manifest(files: Record<string, Record<string, unknown>>): unknown {
  return {
    version: 1,
    baseUrl: "https://example.test",
    fileCount: Object.keys(files).length,
    totalSizeBytes: Object.values(files).reduce(
      (sum, entry) => sum + Number(entry.sizeBytes),
      0,
    ),
    files,
  };
}

function entry(outputPath: string, body = outputPath): Record<string, unknown> {
  return {
    outputPath,
    contentType: "text/plain; charset=utf-8",
    sizeBytes: Buffer.byteLength(body),
    sha256: digest(body),
    source: "extra",
  };
}

describe("parseDeployManifest", () => {
  it("validates and canonicalizes a manifest in output-path order", () => {
    const parsed = parseDeployManifest(manifest({
      "z.txt": entry("z.txt"),
      "a.txt": { ...entry("a.txt"), lastmod: "2026-09-20T00:00:00Z" },
    }));

    expect(Object.keys(parsed.files)).toEqual(["a.txt", "z.txt"]);
    expect(canonicalDeployManifest(parsed)).toBe(`${JSON.stringify({
      version: 1,
      baseUrl: "https://example.test",
      fileCount: 2,
      totalSizeBytes: 10,
      files: {
        "a.txt": {
          outputPath: "a.txt",
          contentType: "text/plain; charset=utf-8",
          sizeBytes: 5,
          sha256: digest("a.txt"),
          source: "extra",
          lastmod: "2026-09-20T00:00:00Z",
        },
        "z.txt": entry("z.txt"),
      },
    }, null, 2)}\n`);
  });

  it("accepts validated page metadata", () => {
    const value = manifest({
      "index.html": {
        ...entry("index.html"),
        source: "page-bundle",
        route: "/",
        contentType: "text/html; charset=utf-8",
        lastmod: "2026-09-20T12:00:00Z",
      },
    });
    expect(parseDeployManifest(value).files["index.html"]?.route).toBe("/");
  });

  it("accepts JSON text and rejects malformed JSON", () => {
    expect(parseDeployManifest(JSON.stringify(manifest({
      "index.html": entry("index.html"),
    }))).fileCount).toBe(1);
    expect(() => parseDeployManifest("{"))
      .toThrow(/valid JSON/);
  });

  it("validates a maximally deep portable path in linear segment work", () => {
    const path = Array.from({ length: 1024 }, () => "a").join("/");
    expect(path).toHaveLength(2047);
    expect(parseDeployManifest(manifest({ [path]: entry(path) })).fileCount).toBe(1);
  });

  it("rejects manifest text above the parse budget before JSON decoding", () => {
    expect(() => parseDeployManifest(" ".repeat(DEPLOY_LIMITS.maxManifestCharacters + 1)))
      .toThrow(/manifest text.*limit/);
  });

  it.each([
    ["map key mismatch", manifest({ "a.txt": entry("b.txt") }), /must equal/],
    ["wrong file count", { ...manifest({ "a.txt": entry("a.txt") }), fileCount: 2 }, /fileCount/],
    ["wrong byte total", { ...manifest({ "a.txt": entry("a.txt") }), totalSizeBytes: 99 }, /totalSizeBytes/],
    ["bad digest", manifest({ "a.txt": { ...entry("a.txt"), sha256: "nope" } }), /SHA-256/],
    ["bad source", manifest({ "a.txt": { ...entry("a.txt"), source: "shell" } }), /source/],
    ["missing page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle" } }), /route is required/],
    ["route on extra", manifest({ "a.txt": { ...entry("a.txt"), route: "/a" } }), /allowed only/],
    ["empty content type", manifest({ "a.txt": { ...entry("a.txt"), contentType: "" } }), /contentType/],
    ["header-injecting content type", manifest({ "a.txt": { ...entry("a.txt"), contentType: "text/plain\r\nX-Injected: yes" } }), /contentType/],
    ["malformed content type", manifest({ "a.txt": { ...entry("a.txt"), contentType: "plain" } }), /contentType/],
    ["malformed content parameter", manifest({ "a.txt": { ...entry("a.txt"), contentType: "text/plain; nonsense" } }), /contentType/],
    ["control-bearing base URL", { ...manifest({ "a.txt": entry("a.txt") }), baseUrl: "https://example.test\nX" }, /baseUrl/],
    ["non-HTTP base URL", { ...manifest({ "a.txt": entry("a.txt") }), baseUrl: "file:///etc/passwd" }, /baseUrl/],
    ["credential-bearing base URL", { ...manifest({ "a.txt": entry("a.txt") }), baseUrl: "https://user:secret@example.test" }, /baseUrl/],
    ["query-bearing base URL", { ...manifest({ "a.txt": entry("a.txt") }), baseUrl: "https://example.test/site?token=x" }, /baseUrl/],
    ["non-absolute page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "relative" } }), /route/],
    ["control-bearing page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/ok\nX" } }), /route/],
    ["query-bearing page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/ok?draft=1" } }), /route/],
    ["fragment-bearing page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/ok#draft" } }), /route/],
    ["traversing page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/a/.." } }), /route/],
    ["dot-segment page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/a/./b" } }), /route/],
    ["empty-segment page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/a//b" } }), /route/],
    ["percent-encoded page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/%2e%2e" } }), /route/],
    ["whitespace page route", manifest({ "a.txt": { ...entry("a.txt"), source: "page-bundle", route: "/hello world" } }), /route/],
    ["invalid lastmod", manifest({ "a.txt": { ...entry("a.txt"), lastmod: "yesterday" } }), /lastmod/],
    ["normalized invalid lastmod", manifest({ "a.txt": { ...entry("a.txt"), lastmod: "2026-02-30" } }), /lastmod/],
    ["negative size", { ...manifest({ "a.txt": entry("a.txt") }), totalSizeBytes: 0, files: { "a.txt": { ...entry("a.txt"), sizeBytes: -1 } } }, /sizeBytes/],
    ["unexpected entry key", manifest({ "a.txt": { ...entry("a.txt"), surprise: true } }), /unexpected field/],
    ["prefix collision", manifest({ assets: entry("assets"), "assets/app.css": entry("assets/app.css") }), /prefix collision/],
    ["non-adjacent prefix collision", manifest({ a: entry("a"), "a-b": entry("a-b"), "a/c": entry("a/c") }), /prefix collision/],
    ["file-count limit", { version: 1, fileCount: DEPLOY_LIMITS.maxFileCount + 1, totalSizeBytes: 0, files: {} }, /fileCount.*limit/],
    ["total-size limit", { version: 1, fileCount: 0, totalSizeBytes: DEPLOY_LIMITS.maxTotalSizeBytes + 1, files: {} }, /totalSizeBytes.*limit/],
    ["per-file limit", { version: 1, fileCount: 1, totalSizeBytes: DEPLOY_LIMITS.maxFileSizeBytes + 1, files: { "huge.bin": { ...entry("huge.bin"), sizeBytes: DEPLOY_LIMITS.maxFileSizeBytes + 1 } } }, /sizeBytes.*limit/],
    ["digest size disagreement", { version: 1, fileCount: 2, totalSizeBytes: 3, files: { "a.txt": { ...entry("a.txt", "x"), sha256: digest("same") }, "b.txt": { ...entry("b.txt", "yy"), sha256: digest("same") } } }, /same SHA-256.*sizeBytes/],
  ])("rejects %s", (_name, value, pattern) => {
    expect(() => parseDeployManifest(value)).toThrow(pattern as RegExp);
  });
});

describe("validateOutputPath", () => {
  it.each([
    "index.html",
    "assets/app-123.css",
    ".well-known/security.txt",
  ])("accepts %s", value => expect(validateOutputPath(value)).toBe(value));

  it.each([
    "",
    "/etc/passwd",
    "\\\\server\\share",
    "C:/Windows/system.ini",
    "a\\b.txt",
    "a/../b",
    "a/./b",
    "a//b",
    "a:b",
    "a\u0000b",
    "CON",
    "dir/LPT1.txt",
    "__proto__/x",
  ])("rejects unsafe portable path %j", value => {
    expect(() => validateOutputPath(value)).toThrow();
  });
});

describe("contentDigestToStoreKey", () => {
  it("maps canonical base64 digests to one path-safe base64url segment", () => {
    const digestWithSeparators = Buffer.alloc(32, 255).toString("base64");
    const key = contentDigestToStoreKey(digestWithSeparators);
    expect(key).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(key).not.toContain("/");
    expect(key).not.toContain("+");
    expect(key).not.toContain("=");
  });

  it("rejects a non-canonical digest", () => {
    expect(() => contentDigestToStoreKey("../../outside")).toThrow(/SHA-256/);
  });
});
