import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { Launcher } from "chrome-launcher";
import { describe, expect, it } from "vitest";
import {
  boundedError,
  ChromeRetirementError,
  chromeFlagsFor,
  createBoundedChromeLauncher,
  discardWarmupAudit,
  preflightQualityTargets,
  retireChrome,
  reportedChromeVersionForPlatform,
  readChromeStartupDiagnostic,
  readBoundedUtf8,
} from "./release-web-quality.js";
import { MAX_FALLBACK_HTML_BYTES, type QualityTarget } from "./web-quality.js";

describe("Forme release web-quality effects", () => {
  it("runs the warm-up audit exactly once and drops its result", async () => {
    let calls = 0;
    await expect(discardWarmupAudit(async () => {
      calls += 1;
      return { categories: { performance: { score: 0.1 } } };
    })).resolves.toBeUndefined();
    expect(calls).toBe(1);
  });

  it("does not turn a failed warm-up audit into a gate failure", async () => {
    let calls = 0;
    const lines: string[] = [];
    await expect(discardWarmupAudit(async () => {
      calls += 1;
      throw new Error("Chrome exited before exposing a reachable debugging port");
    }, line => lines.push(line))).resolves.toBeUndefined();
    expect(calls).toBe(1);
    expect(lines).toEqual([
      "warm-up audit dropped: Chrome exited before exposing a reachable debugging port\n",
    ]);
  });

  it("still fails when the warm-up browser could not be retired", async () => {
    const lines: string[] = [];
    await expect(discardWarmupAudit(async () => {
      throw new ChromeRetirementError(new Error("Chrome retirement timed out"));
    }, line => lines.push(line))).rejects.toThrow("Chrome retirement timed out");
    expect(lines).toEqual([]);
  });

  it("does not lose fatal retirement identity when startup diagnostics are truncated", async () => {
    const fatal = new ChromeRetirementError(new Error("cleanup denied"),
      `Chrome startup failed: ${"x".repeat(1_024)}; retirement failed: cleanup denied`);
    expect(boundedError(fatal)).not.toContain("retirement");
    const lines: string[] = [];
    await expect(discardWarmupAudit(async () => { throw fatal; }, line => lines.push(line)))
      .rejects.toBe(fatal);
    expect(lines).toEqual([]);
  });

  it("classifies a trapping thrown proxy without invoking its traps", async () => {
    let trapped = false;
    const hostile = new Proxy({}, {
      get() { trapped = true; throw new Error("trap ran"); },
      getPrototypeOf() { trapped = true; throw new Error("trap ran"); },
    });
    const lines: string[] = [];
    await expect(discardWarmupAudit(async () => { throw hostile; }, line => lines.push(line)))
      .resolves.toBeUndefined();
    expect(lines).toEqual(["warm-up audit dropped: uninspectable thrown value\n"]);
    expect(trapped).toBe(false);
  });

  it("propagates native temporary cleanup failures from a warm-up", async () => {
    const cause = new Error("EPERM: access denied");
    const lines: string[] = [];
    await expect(discardWarmupAudit(() => retireChrome({
      chromeProcess: undefined,
      kill() {},
      destroyTmp() { throw cause; },
    }), line => lines.push(line))).rejects.toMatchObject({
      name: "ChromeRetirementError", cause,
    });
    expect(lines).toEqual([]);
  });

  it("classifies native kill failures independently of their message", async () => {
    const child = spawn(process.execPath, ["-e", ""], { stdio: "ignore" });
    await once(child, "exit");
    const cause = new Error("operation denied");
    await expect(retireChrome({
      chromeProcess: child,
      kill() { throw cause; },
      destroyTmp() {},
    })).rejects.toMatchObject({ name: "ChromeRetirementError", cause });
  });

  it("preflights all bounded HTML before returning reusable fallback reports", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-quality-preflight-"));
    const target: QualityTarget = {
      id: "test", route: "/test", document: "index.html",
      performanceMinimum: 0.95, accessibilityMinimum: 1, resources: { total: 1_024 },
      fallback: { requiredSelectors: ["main"], minimumTextCharacters: 1,
        minimumLinks: 0, minimumListItems: 0, maximumScripts: 0 },
    };
    try {
      const document = resolve(root, "index.html");
      await writeFile(document, "<main>usable static content</main>");
      const reports = await preflightQualityTargets([{ target, document, root }]);
      expect(reports).toHaveLength(1);
      expect(reports[0]?.fallback.diagnostics).toEqual([]);
      const oversized = resolve(root, "oversized.html");
      await writeFile(oversized, "x".repeat(MAX_FALLBACK_HTML_BYTES + 1));
      await expect(preflightQualityTargets([
        { target, document, root },
        { target: { ...target, id: "oversized" }, document: oversized, root },
      ])).rejects.toThrow(/no larger than/);
      // The retained report came from the bounded preflight, not a later reread.
      expect(reports[0]?.fallback.diagnostics).toEqual([]);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  it("uses the pinned setup action's file-metadata version on Windows", () => {
    expect(reportedChromeVersionForPlatform("win32", "154.0.8037.92")).toBe("154.0.8037.92");
    expect(() => reportedChromeVersionForPlatform("win32", undefined)).toThrow(/reported/);
    expect(() => reportedChromeVersionForPlatform("win32", "154.0")).toThrow(/four-part/);
    expect(reportedChromeVersionForPlatform("linux", undefined)).toBeNull();
  });

  it("allows browser traffic to bypass the proxy only for the exact artifact origin", () => {
    const flags = chromeFlagsFor("http://127.0.0.1:4321/coding-adventures/blog/");
    expect(flags).toContain("--proxy-server=http://127.0.0.1:4321");
    expect(flags).toContain("--proxy-bypass-list=<-loopback>;127.0.0.1:4321");
    expect(flags).not.toContain("--proxy-bypass-list=127.0.0.1");
  });

  it("redacts a trapping thrown proxy without invoking it", () => {
    let trapped = false;
    const hostile = new Proxy({}, {
      get() {
        trapped = true;
        throw new Error("trap ran");
      },
      getPrototypeOf() {
        trapped = true;
        throw new Error("trap ran");
      },
    });
    expect(boundedError(hostile)).toBe("uninspectable thrown value");
    expect(trapped).toBe(false);
  });

  it("retains only bounded printable Chrome startup diagnostics", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-web-quality-chrome-test-"));
    try {
      await writeFile(
        resolve(root, "chrome-err.log"),
        `namespace sandbox unavailable\n${"x".repeat(32_000)}\n`,
        "utf8",
      );
      const diagnostic = await readChromeStartupDiagnostic(root);
      if (diagnostic === null) throw new Error("expected a Chrome startup diagnostic");
      expect(diagnostic).toContain("namespace sandbox unavailable");
      expect(diagnostic).not.toContain("\n");
      expect(diagnostic.length).toBeLessThanOrEqual(1_024);

      const outside = resolve(root, "outside.log");
      await writeFile(outside, "outside", "utf8");
      await rm(resolve(root, "chrome-err.log"));
      await symlink(outside, resolve(root, "chrome-err.log"));
      expect(await readChromeStartupDiagnostic(root)).toBeNull();
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  it("replaces chrome-launcher's whole-log dynamic-port polling", () => {
    const launcher = createBoundedChromeLauncher(
      "/absolute/chrome",
      "http://127.0.0.1:4321/coding-adventures/blog/",
    );
    expect(launcher.waitUntilReady).not.toBe(Launcher.prototype.waitUntilReady);
    const flags = (launcher as unknown as { readonly flags: readonly string[] }).flags;
    expect(flags).toContain("--disable-extensions");
    expect(flags).not.toContain("--disable-setuid-sandbox");
    expect(flags).not.toContain("--no-sandbox");
  });

  it("reads only bounded regular single-link files without following symlinks", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-web-quality-read-test-"));
    try {
      const generated = resolve(root, "generated");
      await mkdir(generated);
      const document = resolve(generated, "index.html");
      await writeFile(document, "safe html", "utf8");
      expect(await readBoundedUtf8(document, generated, 32)).toBe("safe html");

      const alias = resolve(generated, "alias.html");
      await symlink(document, alias);
      await expect(readBoundedUtf8(alias, generated, 32)).rejects.toThrow();
      await expect(readBoundedUtf8(document, generated, 4)).rejects.toThrow(/no larger/);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
