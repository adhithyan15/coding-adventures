/**
 * FM-B070 release evidence for the two live Forme web products.
 *
 * The generated sites are mounted at their real GitHub Pages prefixes on a
 * loopback-only server. Each target gets a fresh, bounded Chrome process so a
 * hung or contaminated audit cannot affect the next target. Raw Lighthouse
 * traces are intentionally discarded; only the small reviewed contract in
 * `web-quality.ts` reaches the retained summary.
 */
import { execFile, type ChildProcess } from "node:child_process";
import { constants as fsConstants } from "node:fs";
import { createServer, type Server } from "node:http";
import { lstat, open, readFile, realpath, stat } from "node:fs/promises";
import { dirname, extname, isAbsolute, relative, resolve, sep } from "node:path";
import { createConnection } from "node:net";
import { fileURLToPath } from "node:url";
import { promisify, types } from "node:util";
import { Launcher } from "chrome-launcher";
import lighthouse from "lighthouse";
import {
  MAX_DIAGNOSTICS,
  MAX_FALLBACK_HTML_BYTES,
  LIGHTHOUSE_SAMPLE_COUNT,
  QUALITY_CONTENT_SECURITY_POLICY,
  aggregateLighthouseReports,
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
const BROWSER_RETIREMENT_TIMEOUT_MS = 10_000;
const CHROME_CONNECTION_POLL_MS = 250;
const CHROME_CONNECTION_RETRIES = 120;
const MAX_CHROME_STARTUP_LOG_BYTES = 8 * 1024;
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
  readonly performanceSamples: readonly number[];
  readonly diagnostics: readonly string[];
}

interface QualitySummary {
  readonly schemaVersion: 2;
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
      const html = await readBoundedUtf8(
        resolve(here, target.document),
        target.id === "landing" ? landingRoot : blogRoot,
        MAX_FALLBACK_HTML_BYTES,
      );
      const fallback = inspectStaticFallback(html, target);
      let browser: LighthouseReport | null = null;
      let performanceSamples: readonly number[] = [];
      let browserDiagnostics: readonly string[] = [];
      try {
        const url = `http://127.0.0.1:${port}${target.route}`;
        const reports: LighthouseReport[] = [];
        const sampleTarget = { ...target, performanceMinimum: 0 };
        for (let sample = 0; sample < LIGHTHOUSE_SAMPLE_COUNT; sample += 1) {
          const raw = await runLighthouse(url, chromePath);
          reports.push(evaluateLighthouseResult(sampleTarget, raw, url));
        }
        const evidence = aggregateLighthouseReports(target, reports);
        browser = evidence.aggregate;
        performanceSamples = evidence.performanceSamples;
        browserDiagnostics = browser.diagnostics;
      } catch (error) {
        browserDiagnostics = [`browser audit failed: ${boundedError(error)}`];
      }
      summaries.push({
        id: target.id,
        route: target.route,
        fallback,
        lighthouse: browser,
        performanceSamples,
        diagnostics: boundDiagnostics([...fallback.diagnostics, ...browserDiagnostics]),
      });
    }
  } finally {
    await closeServer(server);
  }

  const summary: QualitySummary = {
    schemaVersion: 2,
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
      `${target.id} p=${target.lighthouse?.performance.toFixed(2)}` +
      ` samples=${target.performanceSamples.map(score => score.toFixed(2)).join("/")}` +
      ` a=${target.lighthouse?.accessibility.toFixed(2)}`)
      .join(", ")}\n`,
  );
}

async function runLighthouse(url: string, chromePath: string): Promise<unknown> {
  const chrome = createBoundedChromeLauncher(chromePath, url);
  try {
    await chrome.launch();
    if (chrome.port === undefined || chrome.chromeProcess === undefined) {
      throw new Error("Chrome launcher returned no owned process or debugging port");
    }
  } catch (error) {
    const startupDiagnostic = await readChromeStartupDiagnostic(chrome.userDataDir);
    try {
      await retireChrome(chrome);
    } catch (retirementError) {
      throw new Error(
        `Chrome startup failed: ${boundedError(error)}; retirement failed: ${boundedError(retirementError)}` +
        (startupDiagnostic === null ? "" : `; Chrome stderr: ${startupDiagnostic}`),
      );
    }
    if (startupDiagnostic !== null) {
      throw new Error(`${boundedError(error)}; Chrome stderr: ${startupDiagnostic}`);
    }
    throw error;
  }
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
    await retireChrome(chrome);
  }
}

export function createBoundedChromeLauncher(chromePath: string, url: string): Launcher {
  return new BoundedChromeLauncher({
    chromePath,
    connectionPollInterval: CHROME_CONNECTION_POLL_MS,
    maxConnectionRetries: CHROME_CONNECTION_RETRIES,
    chromeFlags: [...Launcher.defaultFlags(), ...chromeFlagsFor(url)],
    handleSIGINT: false,
    ignoreDefaultFlags: true,
    logLevel: "silent",
  });
}

class BoundedChromeLauncher extends Launcher {
  override async waitUntilReady(): Promise<void> {
    for (let retry = 0; retry <= CHROME_CONNECTION_RETRIES; retry += 1) {
      if (this.port === undefined || this.port === 0) {
        const startupLog = await readChromeStartupLogPrefix(this.userDataDir);
        const match = startupLog?.match(/DevTools listening on ws:\/\/127\.0\.0\.1:(\d+)\//);
        const discoveredPort = match?.[1] === undefined ? 0 : Number(match[1]);
        if (Number.isInteger(discoveredPort) && discoveredPort > 0 && discoveredPort <= 65_535) {
          this.port = discoveredPort;
        }
      }
      if (this.port !== undefined && this.port > 0 && await debuggerPortReady(this.port)) return;
      if (
        this.chromeProcess !== undefined &&
        (this.chromeProcess.exitCode !== null || this.chromeProcess.signalCode !== null)
      ) {
        break;
      }
      if (retry < CHROME_CONNECTION_RETRIES) await delay(CHROME_CONNECTION_POLL_MS);
    }
    throw new Error("Chrome exited before exposing a reachable debugging port");
  }
}

function debuggerPortReady(port: number): Promise<boolean> {
  return new Promise<boolean>(accept => {
    const socket = createConnection({ host: "127.0.0.1", port });
    let settled = false;
    const finish = (ready: boolean): void => {
      if (settled) return;
      settled = true;
      socket.destroy();
      accept(ready);
    };
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
    socket.setTimeout(CHROME_CONNECTION_POLL_MS, () => finish(false));
  });
}

function delay(milliseconds: number): Promise<void> {
  return new Promise<void>(accept => setTimeout(accept, milliseconds));
}

export function chromeFlagsFor(url: string): string[] {
  const origin = new URL(url);
  if (origin.protocol !== "http:" || origin.hostname !== "127.0.0.1" || origin.port === "") {
    throw new Error("Chrome quality target must use an ephemeral IPv4 loopback origin");
  }
  return [
    "--headless=new",
    "--disable-dev-shm-usage",
    "--disable-gpu",
    `--proxy-server=http://127.0.0.1:${origin.port}`,
    `--proxy-bypass-list=<-loopback>;127.0.0.1:${origin.port}`,
  ];
}

async function retireChrome(chrome: Launcher): Promise<void> {
  const child = chrome.chromeProcess;
  if (child === undefined) {
    chrome.destroyTmp();
    return;
  }
  if (child.exitCode !== null || child.signalCode !== null) {
    chrome.kill();
    chrome.destroyTmp();
    return;
  }

  const exited = processExit(child);
  chrome.kill();
  try {
    await withTimeout(exited, BROWSER_RETIREMENT_TIMEOUT_MS, "Chrome retirement timed out");
  } finally {
    chrome.destroyTmp();
  }
  if (child.exitCode === null && child.signalCode === null) {
    throw new Error("Chrome retirement completed without a process exit status");
  }
}

function processExit(child: ChildProcess): Promise<void> {
  return new Promise<void>((accept, reject) => {
    const onExit = (): void => {
      child.off("error", onError);
      accept();
    };
    const onError = (error: Error): void => {
      child.off("exit", onExit);
      reject(error);
    };
    child.once("exit", onExit);
    child.once("error", onError);
    if (child.exitCode !== null || child.signalCode !== null) onExit();
  });
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
        "content-security-policy": QUALITY_CONTENT_SECURITY_POLICY,
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
  const reportedVersion = reportedChromeVersionForPlatform(
    process.platform,
    process.env.FORME_INSTALLED_CHROME_VERSION,
  );
  if (reportedVersion !== null) return reportedVersion;
  const result = await execFileAsync(chromePath, ["--version"], {
    timeout: BROWSER_VERSION_TIMEOUT_MS,
    maxBuffer: 4_096,
    killSignal: "SIGKILL",
  });
  const match = /\b(\d+\.\d+\.\d+\.\d+)\b/.exec(result.stdout);
  if (match?.[1] === undefined) throw new Error("Chrome did not report a four-part version");
  return match[1];
}

export function reportedChromeVersionForPlatform(
  platform: NodeJS.Platform,
  reportedVersion: string | undefined,
): string | null {
  if (platform !== "win32") return null;
  if (reportedVersion === undefined) {
    throw new Error("the pinned Chrome setup action reported no Windows version");
  }
  if (!/^\d+\.\d+\.\d+\.\d+$/.test(reportedVersion)) {
    throw new Error("the pinned Chrome setup action did not report a four-part Windows version");
  }
  return reportedVersion;
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

export function boundedError(error: unknown): string {
  let raw = "uninspectable thrown value";
  try {
    if (
      (typeof error === "object" && error !== null) ||
      typeof error === "function"
    ) {
      if (types.isProxy(error)) return raw;
      if (error instanceof Error) {
        const descriptor = Object.getOwnPropertyDescriptor(error, "message");
        if (descriptor !== undefined && "value" in descriptor && typeof descriptor.value === "string") {
          raw = descriptor.value;
        }
      }
    } else {
      raw = String(error);
    }
  } catch {
    return raw;
  }
  return raw.replace(/[^\x20-\x7E]+/g, "?").slice(0, MAX_ERROR_CHARACTERS);
}

export async function readChromeStartupDiagnostic(
  userDataDir: string | undefined,
): Promise<string | null> {
  const raw = await readChromeStartupLogPrefix(userDataDir);
  if (raw === null) return null;
  const diagnostic = boundedError(raw).trim();
  return diagnostic === "" ? null : diagnostic;
}

async function readChromeStartupLogPrefix(
  userDataDir: string | undefined,
): Promise<string | null> {
  if (userDataDir === undefined || !isAbsolute(userDataDir)) return null;
  try {
    return await readBoundedPrefixUtf8(
      resolve(userDataDir, "chrome-err.log"),
      userDataDir,
      MAX_CHROME_STARTUP_LOG_BYTES,
    );
  } catch {
    return null;
  }
}

async function readBoundedPrefixUtf8(
  path: string,
  allowedRoot: string,
  maximumBytes: number,
): Promise<string> {
  const canonicalRoot = await realpath(allowedRoot);
  const lexicalRoot = resolve(allowedRoot);
  const candidate = resolve(path);
  if (!contained(lexicalRoot, candidate)) throw new Error("startup log escaped its root");
  const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
  const handle = await open(
    candidate,
    fsConstants.O_RDONLY | fsConstants.O_NONBLOCK | noFollow,
  );
  try {
    const before = await handle.stat();
    const pathInfo = await lstat(candidate);
    const canonicalPath = await realpath(candidate);
    if (
      !contained(canonicalRoot, canonicalPath) ||
      !before.isFile() ||
      !pathInfo.isFile() ||
      before.nlink !== 1 ||
      pathInfo.nlink !== 1 ||
      before.dev !== pathInfo.dev ||
      before.ino !== pathInfo.ino
    ) {
      throw new Error("startup log must be a regular single-link file");
    }
    const bytes = Buffer.alloc(maximumBytes);
    let offset = 0;
    while (offset < bytes.byteLength) {
      const result = await handle.read(bytes, offset, bytes.byteLength - offset, offset);
      if (result.bytesRead === 0) break;
      offset += result.bytesRead;
    }
    const after = await handle.stat();
    if (
      !after.isFile() ||
      after.nlink !== 1 ||
      after.dev !== before.dev ||
      after.ino !== before.ino
    ) {
      throw new Error("startup log changed identity while being read");
    }
    return bytes.subarray(0, offset).toString("utf8");
  } finally {
    await handle.close();
  }
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
    process.stderr.write(`web quality failed: ${boundedError(error)}\n`);
    process.exitCode = 1;
  });
}

export async function readBoundedUtf8(
  path: string,
  allowedRoot: string,
  maximumBytes: number,
): Promise<string> {
  const canonicalRoot = await realpath(allowedRoot);
  const lexicalRoot = resolve(allowedRoot);
  const candidate = resolve(path);
  if (!contained(lexicalRoot, candidate)) {
    throw new Error("fallback HTML escaped its generated root");
  }
  const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
  const handle = await open(
    candidate,
    fsConstants.O_RDONLY | fsConstants.O_NONBLOCK | noFollow,
  );
  try {
    const before = await handle.stat();
    const pathInfo = await lstat(candidate);
    const canonicalPath = await realpath(candidate);
    if (
      !contained(canonicalRoot, canonicalPath) ||
      !before.isFile() ||
      !pathInfo.isFile() ||
      before.nlink !== 1 ||
      pathInfo.nlink !== 1 ||
      before.dev !== pathInfo.dev ||
      before.ino !== pathInfo.ino ||
      before.size > maximumBytes
    ) {
      throw new Error(`fallback HTML must be a file no larger than ${maximumBytes} bytes`);
    }
    const bytes = Buffer.alloc(maximumBytes + 1);
    let offset = 0;
    while (offset < bytes.byteLength) {
      const result = await handle.read(bytes, offset, bytes.byteLength - offset, null);
      if (result.bytesRead === 0) break;
      offset += result.bytesRead;
    }
    const after = await handle.stat();
    if (
      offset > maximumBytes ||
      !after.isFile() ||
      after.nlink !== 1 ||
      after.dev !== before.dev ||
      after.ino !== before.ino ||
      after.size > maximumBytes
    ) {
      throw new Error(`fallback HTML must be a stable file no larger than ${maximumBytes} bytes`);
    }
    return bytes.subarray(0, offset).toString("utf8");
  } finally {
    await handle.close();
  }
}
