import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  MAX_DIAGNOSTICS,
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

  it("accepts explicit scores and transfer sizes while retaining audit evidence", () => {
    const report = evaluateLighthouseResult(target, lighthouse({
      performance: 0.98,
      accessibility: 1,
      resources: { total: 120_000, image: 20_000, script: 4_000 },
    }));

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
    }));

    expect(report.diagnostics).toEqual([
      "accessibility score 0.99 < 1",
      "accessibility audit failed: button-name",
      "accessibility audit failed: color-contrast",
      "performance score 0.94 < 0.95",
      "resource image 80000 > 65536 bytes",
      "resource script 40000 > 32768 bytes",
      "resource total 300000 > 262144 bytes",
    ]);
    expect(() => evaluateLighthouseResult(target, { categories: {}, audits: {} }))
      .toThrow(/missing a finite performance score/);
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
    }));

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
  readonly resources: Readonly<Record<string, number>>;
  readonly failedAccessibilityAudits?: readonly string[];
}): unknown {
  const failed = new Set(options.failedAccessibilityAudits ?? []);
  return {
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
          items: Object.entries(options.resources).map(([resourceType, transferSize]) => ({
            resourceType,
            transferSize,
          })),
        },
      },
      ...Object.fromEntries([...failed].map(id => [id, { score: 0, scoreDisplayMode: "binary" }])),
    },
  };
}
