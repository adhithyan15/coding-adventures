import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  MAX_DIAGNOSTICS,
  QUALITY_CONTENT_SECURITY_POLICY,
  atomicWriteQualitySummary,
  evaluateLighthouseResult,
  inspectStaticFallback,
  resolveStaticRequest,
  type QualityTarget,
} from "./web-quality.js";

const target: QualityTarget = {
  id: "enhanced-article",
  route: "/coding-adventures/blog/hello.html",
  document: "blog/hello.html",
  performanceMinimum: 0.95,
  accessibilityMinimum: 1,
  resources: {
    total: 256 * 1024,
    image: 64 * 1024,
    script: 32 * 1024,
  },
  fallback: {
    requiredSelectors: ["main", "nav", "#steps ol"],
    minimumTextCharacters: 40,
    minimumLinks: 1,
    minimumListItems: 3,
    maximumScripts: 1,
  },
};
const expectedUrl = "http://127.0.0.1:4321/coding-adventures/blog/hello.html";

describe("Forme release web-quality gate", () => {
  it("proves useful static content without executing the admitted script", () => {
    const result = inspectStaticFallback(`<!doctype html>
      <html><body>
        <nav><a href="/coding-adventures/blog/">Blog home</a></nav>
        <main id="steps"><h1>Hello, Forme</h1><ol>
          <li>source stage remains readable</li>
          <li>render stage remains readable</li>
          <li>emit stage remains readable</li>
        </ol></main>
        <script type="module" src="/coding-adventures/blog/assets/steps.js"></script>
      </body></html>`, target);

    expect(result).toEqual({
      textCharacters: 110,
      links: 1,
      listItems: 3,
      scripts: 1,
      diagnostics: [],
    });
  });

  it("reports every bounded static fallback regression deterministically", () => {
    const result = inspectStaticFallback(
      "<main><p>short</p></main><script></script><script></script>",
      target,
    );

    expect(result.diagnostics).toEqual([
      "fallback missing selector #steps ol",
      "fallback missing selector nav",
      "fallback text 5 < 40 characters",
      "fallback links 0 < 1",
      "fallback list items 0 < 3",
      "fallback scripts 2 > 1",
    ]);
  });

  it("never fetches stylesheets or frames during the static fallback pass", async () => {
    let requests = 0;
    const server = createServer((_request, response) => {
      requests += 1;
      response.writeHead(200, { "content-type": "text/plain" }).end("unexpected");
    });
    await new Promise<void>((accept, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", accept);
    });
    try {
      const address = server.address();
      if (address === null || typeof address === "string") throw new Error("missing test port");
      const resource = `http://127.0.0.1:${address.port}/resource`;
      inspectStaticFallback(
        `<main id="steps"><nav><a href="/">home</a></nav><ol><li>a</li><li>b</li><li>c</li></ol></main>` +
        `<link rel="stylesheet" href="${resource}.css"><iframe src="${resource}.html"></iframe>`,
        { ...target, fallback: { ...target.fallback, minimumTextCharacters: 0 } },
      );
      await new Promise(resolve => setTimeout(resolve, 50));
      expect(requests).toBe(0);
    } finally {
      await new Promise<void>(accept => server.close(() => accept()));
    }
  });

  it("accepts explicit scores and transfer sizes while retaining audit evidence", () => {
    const report = evaluateLighthouseResult(target, lighthouse({
      performance: 0.98,
      accessibility: 1,
      resources: { total: 120_000, image: 20_000, script: 4_000 },
    }), expectedUrl);

    expect(report).toEqual({
      performance: 0.98,
      accessibility: 1,
      resources: { image: 20_000, script: 4_000, total: 120_000 },
      diagnostics: [],
    });
  });

  it("fails closed on scores, resource budgets, malformed evidence, and audit failures", () => {
    const report = evaluateLighthouseResult(target, lighthouse({
      performance: 0.94,
      accessibility: 0.99,
      resources: { total: 300_000, image: 80_000, script: 40_000 },
      failedAccessibilityAudits: ["color-contrast", "button-name"],
    }), expectedUrl);

    expect(report.diagnostics).toEqual([
      "accessibility score 0.99 < 1",
      "accessibility audit failed: button-name",
      "accessibility audit failed: color-contrast",
      "performance score 0.94 < 0.95",
      "resource image 80000 > 65536 bytes",
      "resource script 40000 > 32768 bytes",
      "resource total 300000 > 262144 bytes",
    ]);
    expect(() => evaluateLighthouseResult(target, { categories: {}, audits: {} }, expectedUrl))
      .toThrow(/missing a finite performance score/);
    expect(() => evaluateLighthouseResult(target, {
      requestedUrl: expectedUrl,
      finalUrl: expectedUrl,
      categories: {
        performance: { score: 1 },
        accessibility: { score: 1 },
      },
      audits: {
        "resource-summary": {
          details: {
            items: [
              { resourceType: "total", transferSize: 0 },
              { resourceType: "image", transferSize: 0 },
              { resourceType: "script", transferSize: 0 },
            ],
          },
        },
      },
    }, expectedUrl)).toThrow(/bounded auditRefs/);

    const nearBoundary = evaluateLighthouseResult(target, lighthouse({
      performance: 0.949,
      accessibility: 1,
      resources: { total: 0, image: 0, script: 0 },
    }), expectedUrl);
    expect(nearBoundary.diagnostics).toContain("performance score 0.949 < 0.95");

    expect(() => evaluateLighthouseResult(target, lighthouse({
      performance: 1,
      accessibility: 1,
      resources: [["total", 0], ["image", 70_000], ["image", 0], ["script", 0]],
    }), expectedUrl)).toThrow(/duplicate resource type image/);
    expect(() => evaluateLighthouseResult(target, lighthouse({
      performance: 1,
      accessibility: 1,
      resources: [["total", 0], ["image", 65_536.1], ["script", 0]],
    }), expectedUrl)).toThrow(/exact safe-integer transfer size/);

    const navigated = lighthouse({
      performance: 1,
      accessibility: 1,
      resources: { total: 0, image: 0, script: 0 },
    });
    navigated.finalUrl = "https://example.com/borrowed-scores";
    expect(() => evaluateLighthouseResult(target, navigated, expectedUrl))
      .toThrow(/URL identity/);
  });

  it("rejects proxy and accessor-backed browser evidence before invoking it", () => {
    let proxyWasRead = false;
    const proxy = new Proxy({}, {
      get() {
        proxyWasRead = true;
        throw new Error("untrusted proxy trap ran");
      },
    });
    expect(() => evaluateLighthouseResult(target, proxy, expectedUrl)).toThrow(/proxies/);
    expect(proxyWasRead).toBe(false);

    const accessorBacked = Object.defineProperty({}, "categories", {
      enumerable: true,
      get() {
        throw new Error("untrusted getter ran");
      },
    });
    expect(() => evaluateLighthouseResult(target, accessorBacked, expectedUrl)).toThrow(/accessors/);
  });

  it("keeps audited pages offline except for their own generated resources", () => {
    expect(QUALITY_CONTENT_SECURITY_POLICY).toContain("default-src 'none'");
    expect(QUALITY_CONTENT_SECURITY_POLICY).toContain("connect-src 'none'");
    expect(QUALITY_CONTENT_SECURITY_POLICY).toContain("script-src 'self'");
  });

  it("caps hostile diagnostic volume", () => {
    const failedAccessibilityAudits = Array.from(
      { length: MAX_DIAGNOSTICS + 10 },
      (_value, index) => `audit-${String(index).padStart(2, "0")}`,
    );
    const report = evaluateLighthouseResult(target, lighthouse({
      performance: 1,
      accessibility: 0,
      resources: { total: 0, image: 0, script: 0 },
      failedAccessibilityAudits,
    }), expectedUrl);

    expect(report.diagnostics).toHaveLength(MAX_DIAGNOSTICS);
    expect(report.diagnostics.at(-1)).toBe("diagnostics truncated after 19 entries");
  });

  it("maps only contained GET requests into generated mounts", () => {
    const mounts = [
      { routePrefix: "/coding-adventures/blog/", root: "/tmp/generated/blog" },
      { routePrefix: "/coding-adventures/", root: "/tmp/generated/landing" },
    ] as const;

    expect(resolveStaticRequest("GET", "/coding-adventures/blog/hello.html", mounts))
      .toBe("/tmp/generated/blog/hello.html");
    expect(resolveStaticRequest("HEAD", "/coding-adventures/", mounts))
      .toBe("/tmp/generated/landing/index.html");
    expect(() => resolveStaticRequest("POST", "/coding-adventures/", mounts))
      .toThrow(/GET and HEAD/);
    expect(() => resolveStaticRequest("GET", "/coding-adventures/%2e%2e/secret", mounts))
      .toThrow(/contained/);
    expect(() => resolveStaticRequest("GET", "/unowned/index.html", mounts))
      .toThrow(/configured mount/);
  });

  it("atomically replaces a hostile report symlink without following it", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-web-quality-report-test-"));
    try {
      const victim = resolve(root, "victim.json");
      const output = resolve(root, "dist/report.json");
      await writeFile(victim, "untouched\n", "utf8");
      await symlink(victim, output).catch(async error => {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        await mkdir(resolve(root, "dist"));
        await symlink(victim, output);
      });

      await atomicWriteQualitySummary(output, root, "safe\n");

      expect(await readFile(victim, "utf8")).toBe("untouched\n");
      expect(await readFile(output, "utf8")).toBe("safe\n");
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});

function lighthouse(options: {
  readonly performance: number;
  readonly accessibility: number;
  readonly resources: Readonly<Record<string, number>> | readonly (readonly [string, number])[];
  readonly failedAccessibilityAudits?: readonly string[];
}): Record<string, unknown> {
  const failed = new Set(options.failedAccessibilityAudits ?? []);
  const resources = Array.isArray(options.resources)
    ? options.resources
    : Object.entries(options.resources);
  return {
    requestedUrl: expectedUrl,
    finalUrl: expectedUrl,
    categories: {
      performance: { score: options.performance },
      accessibility: {
        score: options.accessibility,
        auditRefs: [...failed].map(id => ({ id, weight: 1 })),
      },
    },
    audits: {
      "resource-summary": {
        details: {
          items: resources.map(([resourceType, transferSize]) => ({
            resourceType,
            transferSize,
          })),
        },
      },
      ...Object.fromEntries([...failed].map(id => [id, { score: 0, scoreDisplayMode: "binary" }])),
    },
  };
}
