/**
 * generate.test.ts — end-to-end generateDeployManifest.
 */

import { describe, it, expect } from "vitest";
import { generateDeployManifest } from "../src/index.js";

const MIN_BUNDLE = JSON.stringify({
  version: 1,
  routes: {
    "/": {
      route: "/",
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 100,
      sha256: "AAAA",
    },
  },
});

const BUNDLE_WITH_BASE = JSON.stringify({
  version: 1,
  baseUrl: "https://example.com",
  routes: {
    "/": {
      route: "/",
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 100,
      sha256: "AAAA",
    },
    "/about": {
      route: "/about",
      outputPath: "about/index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 200,
      sha256: "BBBB",
    },
  },
});

describe("generateDeployManifest — shape", () => {
  it("null config throws", () => {
    expect(() => generateDeployManifest(null as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("non-string pageBundle throws", () => {
    expect(() => generateDeployManifest({ pageBundle: 42 as unknown as string }))
      .toThrow(/pageBundle must be a string/);
  });
  it("invalid pageBundle JSON throws", () => {
    expect(() => generateDeployManifest({ pageBundle: "not json" }))
      .toThrow(/not valid JSON/);
  });
});

describe("generateDeployManifest — minimal", () => {
  it("page bundle only", () => {
    const out = generateDeployManifest({ pageBundle: MIN_BUNDLE });
    const m = JSON.parse(out);
    expect(m.version).toBe(1);
    expect(m.fileCount).toBe(1);
    expect(m.totalSizeBytes).toBe(100);
    expect(m.files["index.html"]).toMatchObject({
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 100,
      sha256: "AAAA",
      route: "/",
      source: "page-bundle",
    });
  });
  it("baseUrl propagated from page bundle", () => {
    const out = generateDeployManifest({ pageBundle: BUNDLE_WITH_BASE });
    const m = JSON.parse(out);
    expect(m.baseUrl).toBe("https://example.com");
  });
  it("trailing newline", () => {
    expect(generateDeployManifest({ pageBundle: MIN_BUNDLE }).endsWith("\n")).toBe(true);
  });
});

describe("generateDeployManifest — sitemap / robots / web app manifest", () => {
  it("sitemap entry synthesised", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      sitemapXml: "<?xml?><urlset/>",
    }));
    expect(m.files["sitemap.xml"]).toMatchObject({
      outputPath: "sitemap.xml",
      contentType: "application/xml",
      source: "sitemap",
    });
    expect(m.files["sitemap.xml"].sizeBytes).toBeGreaterThan(0);
  });
  it("robots entry synthesised", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      robotsTxt: "User-agent: *\nAllow: /\n",
    }));
    expect(m.files["robots.txt"]).toMatchObject({
      outputPath: "robots.txt",
      contentType: "text/plain; charset=utf-8",
      source: "robots",
    });
  });
  it("web app manifest synthesised", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      manifestJson: '{"name":"App"}',
    }));
    expect(m.files["manifest.webmanifest"]).toMatchObject({
      outputPath: "manifest.webmanifest",
      contentType: "application/manifest+json",
      source: "web-app-manifest",
    });
  });
  it("non-string sitemapXml throws", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE, sitemapXml: 42 as unknown as string,
    })).toThrow(/sitemapXml must be a string/);
  });
  it("non-string robotsTxt throws", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE, robotsTxt: 42 as unknown as string,
    })).toThrow(/robotsTxt must be a string/);
  });
  it("non-string manifestJson throws", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE, manifestJson: 42 as unknown as string,
    })).toThrow(/manifestJson must be a string/);
  });
});

describe("generateDeployManifest — extraFiles", () => {
  it("favicon binary added", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "favicon.ico", content: "base64data", contentType: "image/x-icon" }],
    }));
    expect(m.files["favicon.ico"]).toMatchObject({
      outputPath: "favicon.ico",
      contentType: "image/x-icon",
      source: "extra",
    });
  });
  it("multiple extras", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [
        { outputPath: "favicon.ico", content: "x", contentType: "image/x-icon" },
        { outputPath: ".well-known/security.txt", content: "Contact: x", contentType: "text/plain" },
      ],
    }));
    expect(m.files["favicon.ico"]).toBeDefined();
    expect(m.files[".well-known/security.txt"]).toBeDefined();
    expect(m.fileCount).toBe(3); // 1 page + 2 extras
  });
  it("non-array extraFiles throws", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: "nope" as unknown as never,
    })).toThrow(/extraFiles must be an array/);
  });
  it("null extra entry throws", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [null as unknown as never],
    })).toThrow(/extraFiles\[0\] must be a non-null object/);
  });
  it("path traversal in extraFiles rejected", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "../etc", content: "x", contentType: "text/plain" }],
    })).toThrow(/path traversal/);
  });
  it("absolute path in extraFiles rejected", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "/etc/passwd", content: "x", contentType: "text/plain" }],
    })).toThrow(/must be relative/);
  });
  it("backslash in extraFiles rejected", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "a\\b", content: "x", contentType: "text/plain" }],
    })).toThrow(/must not contain/);
  });
  it("lastmod preserved on extras", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "x.txt", content: "x", contentType: "text/plain", lastmod: "2026-05-19" }],
    }));
    expect(m.files["x.txt"].lastmod).toBe("2026-05-19");
  });
});

describe("generateDeployManifest — duplicate detection", () => {
  it("duplicate path in page bundle throws", () => {
    const dupBundle = JSON.stringify({
      version: 1,
      routes: {
        "/a": { route: "/a", outputPath: "x.html", contentType: "t", sizeBytes: 0, sha256: "z" },
        "/b": { route: "/b", outputPath: "x.html", contentType: "t", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => generateDeployManifest({ pageBundle: dupBundle }))
      .toThrow(/pageBundle has duplicate outputPath/);
  });
  it("sitemap collides with page-bundle route at sitemap.xml", () => {
    const collidingBundle = JSON.stringify({
      version: 1,
      routes: {
        "/sitemap.xml": { route: "/sitemap.xml", outputPath: "sitemap.xml", contentType: "x", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => generateDeployManifest({ pageBundle: collidingBundle, sitemapXml: "<?xml?>" }))
      .toThrow(/sitemap output path "sitemap.xml" collides/);
  });
  it("robots collides with page-bundle entry", () => {
    const collidingBundle = JSON.stringify({
      version: 1,
      routes: {
        "/robots.txt": { route: "/robots.txt", outputPath: "robots.txt", contentType: "x", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => generateDeployManifest({ pageBundle: collidingBundle, robotsTxt: "x" }))
      .toThrow(/robots output path "robots.txt" collides/);
  });
  it("web-app-manifest collides", () => {
    const collidingBundle = JSON.stringify({
      version: 1,
      routes: {
        "/m": { route: "/m", outputPath: "manifest.webmanifest", contentType: "x", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => generateDeployManifest({ pageBundle: collidingBundle, manifestJson: "{}" }))
      .toThrow(/web-app-manifest output path "manifest.webmanifest" collides/);
  });
  it("extra collides with page-bundle entry", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      extraFiles: [{ outputPath: "index.html", content: "x", contentType: "text/html" }],
    })).toThrow(/extraFiles\[0\]\.outputPath "index.html" duplicates/);
  });
  it("extra collides with sitemap entry", () => {
    expect(() => generateDeployManifest({
      pageBundle: MIN_BUNDLE,
      sitemapXml: "<?xml?>",
      extraFiles: [{ outputPath: "sitemap.xml", content: "x", contentType: "application/xml" }],
    })).toThrow(/extraFiles\[0\]\.outputPath "sitemap.xml" duplicates/);
  });
});

describe("generateDeployManifest — output format", () => {
  it("files sorted by outputPath", () => {
    const bundle = JSON.stringify({
      version: 1,
      routes: {
        "/z": { route: "/z", outputPath: "z/index.html", contentType: "t", sizeBytes: 1, sha256: "z" },
        "/a": { route: "/a", outputPath: "a/index.html", contentType: "t", sizeBytes: 1, sha256: "a" },
      },
    });
    const out = generateDeployManifest({ pageBundle: bundle });
    const aIdx = out.indexOf("a/index.html");
    const zIdx = out.indexOf("z/index.html");
    expect(aIdx).toBeLessThan(zIdx);
  });
  it("totalSizeBytes is sum across all files", () => {
    const m = JSON.parse(generateDeployManifest({
      pageBundle: BUNDLE_WITH_BASE,
      sitemapXml: "abc", // 3 bytes
    }));
    expect(m.totalSizeBytes).toBe(100 + 200 + 3);
  });
  it("entry key order: outputPath → contentType → sizeBytes → sha256 → source → route → lastmod", () => {
    const out = generateDeployManifest({
      pageBundle: JSON.stringify({
        version: 1,
        routes: {
          "/": { route: "/", outputPath: "index.html", contentType: "t", sizeBytes: 0, sha256: "z", lastmod: "2026-05-19" },
        },
      }),
    });
    const order = ["outputPath", "contentType", "sizeBytes", "sha256", "source", "route", "lastmod"];
    let lastIdx = -1;
    for (const key of order) {
      const idx = out.indexOf(`"${key}"`);
      expect(idx).toBeGreaterThan(lastIdx);
      lastIdx = idx;
    }
  });
});

describe("generateDeployManifest — determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = { pageBundle: BUNDLE_WITH_BASE, sitemapXml: "<?xml?>" };
    expect(generateDeployManifest(cfg)).toBe(generateDeployManifest(cfg));
  });
  it("does not mutate extraFiles", () => {
    const extras = [{ outputPath: "x.txt", content: "x", contentType: "text/plain" }];
    const before = JSON.stringify(extras);
    generateDeployManifest({ pageBundle: MIN_BUNDLE, extraFiles: extras });
    expect(JSON.stringify(extras)).toBe(before);
  });
});

describe("generateDeployManifest — full real-world example", () => {
  it("page bundle + sitemap + robots + manifest + favicon", () => {
    const out = generateDeployManifest({
      pageBundle: BUNDLE_WITH_BASE,
      sitemapXml: "<?xml version=\"1.0\"?><urlset/>",
      robotsTxt: "User-agent: *\nAllow: /\n",
      manifestJson: '{"name":"Site"}',
      extraFiles: [
        { outputPath: "favicon.ico", content: "BinaryFaviconBase64", contentType: "image/x-icon" },
        { outputPath: ".well-known/security.txt", content: "Contact: mailto:x@example.com\n", contentType: "text/plain" },
      ],
    });
    const m = JSON.parse(out);
    expect(m.version).toBe(1);
    expect(m.baseUrl).toBe("https://example.com");
    expect(m.fileCount).toBe(7); // 2 pages + sitemap + robots + manifest + 2 extras
    expect(Object.keys(m.files).sort()).toEqual([
      ".well-known/security.txt",
      "about/index.html",
      "favicon.ico",
      "index.html",
      "manifest.webmanifest",
      "robots.txt",
      "sitemap.xml",
    ]);
    expect(m.files["index.html"].source).toBe("page-bundle");
    expect(m.files["sitemap.xml"].source).toBe("sitemap");
    expect(m.files["robots.txt"].source).toBe("robots");
    expect(m.files["manifest.webmanifest"].source).toBe("web-app-manifest");
    expect(m.files["favicon.ico"].source).toBe("extra");
  });
});
