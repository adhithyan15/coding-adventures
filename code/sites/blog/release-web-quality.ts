/**
 * FM-B070 release evidence for the two live Forme web products.
 *
 * The generated sites are mounted at their real GitHub Pages prefixes on a
 * loopback-only server. Each target gets a fresh, bounded Chrome process so a
 * hung or contaminated audit cannot affect the next target. Raw Lighthouse
 * traces are intentionally discarded; only the small reviewed contract in
 * `web-quality.ts` reaches the retained summary.
 */
import { execFile } from "node:child_process";
import { createServer, type Server } from "node:http";
import { readFile, realpath, stat } from "node:fs/promises";
import { dirname, extname, isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { Launcher, launch } from "chrome-launcher";
import lighthouse from "lighthouse";
import {
  MAX_DIAGNOSTICS,
  atomicWriteQualitySummary,
  evaluateLighthouseResult,
  inspectStaticFallback,
  resolveStaticRequest,
  type LighthouseReport,
  type QualityTarget,
  type StaticFallbackReport,
  type StaticMount,
} from "./web-quality.js";

const execFileAsync = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const landingRoot = resolve(here, "../landing-page/dist");
const blogRoot = resolve(here, "dist/blog");
const summaryPath = resolve(here, "dist/.forme-web-quality.json");
const LIGHTHOUSE_VERSION = "13.5.0";
const AUDIT_TIMEOUT_MS = 90_000;
const BROWSER_VERSION_TIMEOUT_MS = 10_000;
const MAX_SERVED_BYTES = 2 * 1024 * 1024;
const MAX_ERROR_CHARACTERS = 1_024;

const targets: readonly QualityTarget[] = [
  {
    id: "landing",
    route: "/coding-adventures/",
    document: "../landing-page/dist/index.html",
    performanceMinimum: 0.95,
    accessibilityMinimum: 1,
    resources: { total: 640 * 1024, image: 512 * 1024, script: 0 },
    fallback: {
      requiredSelectors: ["main#main", "nav[aria-label]", "#forme ol.pipeline"],
      minimumTextCharacters: 1_000,
      minimumLinks: 12,
      minimumListItems: 8,
      maximumScripts: 0,
    },
  },
  {
    id: "blog-index",
    route: "/coding-adventures/blog/",
    document: "dist/blog/index.html",
    performanceMinimum: 0.95,
    accessibilityMinimum: 1,
    resources: { total: 160 * 1024, script: 0 },
    fallback: {
      requiredSelectors: ["header a[href]", "main", "#post-index ul"],
      minimumTextCharacters: 160,
      minimumLinks: 4,
      minimumListItems: 3,
      maximumScripts: 0,
    },
  },
  {
    id: "hello-forme",
    route: "/coding-adventures/blog/2026-05-15-hello-forme.html",
    document: "dist/blog/2026-05-15-hello-forme.html",
    performanceMinimum: 0.95,
    accessibilityMinimum: 1,
    resources: { total: 256 * 1024, image: 64 * 1024, script: 32 * 1024 },
    fallback: {
      requiredSelectors: ["header a[href]", "main", "#forme-pipeline-steps"],
      minimumTextCharacters: 500,
      minimumLinks: 2,
      minimumListItems: 13,
      maximumScripts: 1,
    },
  },
] as const;

interface TargetSummary {
  readonly id: string;
  readonly route: string;
  readonly fallback: StaticFallbackReport;
  readonly lighthouse: LighthouseReport | null;
  readonly diagnostics: readonly string[];
}

interface QualitySummary {
  readonly schemaVersion: 1;
  readonly lighthouseVersion: string;
  readonly chromeVersion: string;
  readonly targets: readonly TargetSummary[];
}

async function main(): Promise<void> {
  const chromePath = requireChromePath();
  const chromeVersion = await readChromeVersion(chromePath);
  const expectedVersion = process.env.FORME_EXPECTED_CHROME_VERSION;
  if (expectedVersion !== undefined && chromeVersion !== expectedVersion) {
    throw new Error(`Chrome version ${chromeVersion} does not match reviewed ${expectedVersion}`);
  }
  const installedLighthouseVersion = await readLighthouseVersion();
  if (installedLighthouseVersion !== LIGHTHOUSE_VERSION) {
    throw new Error(
      `Lighthouse version ${installedLighthouseVersion} does not match reviewed ${LIGHTHOUSE_VERSION}`,
    );
  }

  const mounts = await canonicalMounts();
  const server = createStaticServer(mounts);
  const port = await listen(server);
  const summaries: TargetSummary[] = [];
  try {
    for (const target of targets) {
      const html = await readFile(resolve(here, target.document), "utf8");
      const fallback = inspectStaticFallback(html, target);
      let browser: LighthouseReport | null = null;
      let browserDiagnostics: readonly string[] = [];
      try {
        const raw = await runLighthouse(
          `http://127.0.0.1:${port}${target.route}`,
          chromePath,
        );
        browser = evaluateLighthouseResult(target, raw);
        browserDiagnostics = browser.diagnostics;
      } catch (error) {
        browserDiagnostics = [`browser audit failed: ${boundedError(error)}`];
      }
      summaries.push({
        id: target.id,
        route: target.route,
        fallback,
        lighthouse: browser,
        diagnostics: boundDiagnostics([...fallback.diagnostics, ...browserDiagnostics]),
      });
    }
  } finally {
    await closeServer(server);
  }

  const summary: QualitySummary = {
    schemaVersion: 1,
    lighthouseVersion: installedLighthouseVersion,
    chromeVersion,
    targets: summaries,
  };
  await atomicWriteQualitySummary(summaryPath, here, `${JSON.stringify(summary, null, 2)}\n`);

  const failures = summaries.flatMap(target =>
    target.diagnostics.map(message => `${target.id}: ${message}`));
  if (failures.length > 0) {
    throw new Error(`Forme web-quality gate failed:\n${failures.join("\n")}`);
  }
  process.stdout.write(
    `web quality: ${summaries.map(target =>
      `${target.id} p=${target.lighthouse?.performance.toFixed(2)} a=${target.lighthouse?.accessibility.toFixed(2)}`)
      .join(", ")}\n`,
  );
}

async function runLighthouse(url: string, chromePath: string): Promise<unknown> {
  const chrome = await launch({
    chromePath,
    chromeFlags: [
      "--headless=new",
      "--no-sandbox",
      "--disable-dev-shm-usage",
      "--disable-gpu",
    ],
    logLevel: "silent",
  });
  try {
    const result = await withTimeout(
      lighthouse(url, {
        port: chrome.port,
        output: "json",
        logLevel: "error",
        onlyCategories: ["performance", "accessibility"],
        formFactor: "desktop",
        screenEmulation: {
          mobile: false,
          width: 1350,
          height: 940,
          deviceScaleFactor: 1,
          disabled: false,
        },
      }),
      AUDIT_TIMEOUT_MS,
      "Lighthouse audit timed out",
    );
    if (result?.lhr === undefined) throw new Error("Lighthouse returned no result");
    return result.lhr;
  } finally {
    chrome.kill();
  }
}

function createStaticServer(mounts: readonly StaticMount[]): Server {
  const canonicalRoots = mounts.map(mount => resolve(mount.root));
  return createServer(async (request, response) => {
    try {
      const path = resolveStaticRequest(request.method ?? "", request.url ?? "", mounts);
      const info = await stat(path);
      if (!info.isFile() || info.size > MAX_SERVED_BYTES) {
        response.writeHead(info.isFile() ? 413 : 404).end();
        return;
      }
      const canonicalPath = await realpath(path);
      if (!canonicalRoots.some(root => contained(root, canonicalPath))) {
        response.writeHead(403).end();
        return;
      }
      const bytes = await readFile(canonicalPath);
      response.writeHead(200, {
        "cache-control": "no-store",
        "content-length": String(bytes.byteLength),
        "content-type": contentType(canonicalPath),
        "x-content-type-options": "nosniff",
      });
      response.end(request.method === "HEAD" ? undefined : bytes);
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      response.writeHead(code === "ENOENT" ? 404 : 400).end();
    }
  });
}

async function canonicalMounts(): Promise<readonly StaticMount[]> {
  return [
    { routePrefix: "/coding-adventures/blog/", root: await realpath(blogRoot) },
    { routePrefix: "/coding-adventures/", root: await realpath(landingRoot) },
  ];
}

async function listen(server: Server): Promise<number> {
  await new Promise<void>((accept, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      accept();
    });
  });
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("quality server did not acquire a TCP port");
  }
  return address.port;
}

async function closeServer(server: Server): Promise<void> {
  server.closeAllConnections();
  if (!server.listening) return;
  await new Promise<void>((accept, reject) => {
    server.close(error => error === undefined ? accept() : reject(error));
  });
}

function requireChromePath(): string {
  const configured = process.env.CHROME_PATH;
  const discovered = configured === undefined ? Launcher.getInstallations()[0] : configured;
  if (discovered === undefined || !isAbsolute(discovered)) {
    throw new Error("CHROME_PATH must select an installed absolute Chrome binary");
  }
  return discovered;
}

async function readChromeVersion(chromePath: string): Promise<string> {
  const result = await execFileAsync(chromePath, ["--version"], {
    timeout: BROWSER_VERSION_TIMEOUT_MS,
    maxBuffer: 4_096,
    killSignal: "SIGKILL",
  });
  const match = /\b(\d+\.\d+\.\d+\.\d+)\b/.exec(result.stdout);
  if (match?.[1] === undefined) throw new Error("Chrome did not report a four-part version");
  return match[1];
}

async function readLighthouseVersion(): Promise<string> {
  const packageJson = JSON.parse(
    await readFile(resolve(here, "node_modules/lighthouse/package.json"), "utf8"),
  ) as { readonly version?: unknown };
  if (typeof packageJson.version !== "string") {
    throw new Error("installed Lighthouse package has no version");
  }
  return packageJson.version;
}

function withTimeout<T>(promise: Promise<T>, milliseconds: number, message: string): Promise<T> {
  return new Promise<T>((accept, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), milliseconds);
    timer.unref();
    promise.then(
      value => {
        clearTimeout(timer);
        accept(value);
      },
      error => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

function contentType(path: string): string {
  switch (extname(path).toLowerCase()) {
    case ".css": return "text/css; charset=utf-8";
    case ".html": return "text/html; charset=utf-8";
    case ".jpg":
    case ".jpeg": return "image/jpeg";
    case ".js": return "text/javascript; charset=utf-8";
    case ".json": return "application/json; charset=utf-8";
    case ".svg": return "image/svg+xml; charset=utf-8";
    case ".xml": return "application/xml; charset=utf-8";
    default: return "application/octet-stream";
  }
}

function contained(root: string, candidate: string): boolean {
  const suffix = relative(root, candidate);
  return suffix === "" || (!isAbsolute(suffix) && suffix !== ".." && !suffix.startsWith(`..${sep}`));
}

function boundedError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  return raw.replace(/[\r\n\t]+/g, " ").slice(0, MAX_ERROR_CHARACTERS);
}

function boundDiagnostics(diagnostics: readonly string[]): readonly string[] {
  if (diagnostics.length <= MAX_DIAGNOSTICS) return diagnostics;
  return [
    ...diagnostics.slice(0, MAX_DIAGNOSTICS - 1),
    `diagnostics truncated after ${MAX_DIAGNOSTICS - 1} entries`,
  ];
}

const invokedPath = process.argv[1] === undefined ? "" : resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch(error => {
    process.stderr.write(`${error instanceof Error ? error.stack ?? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
