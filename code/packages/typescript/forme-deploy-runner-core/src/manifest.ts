import type { DeployFileEntry, DeployManifest, DeploySource } from "./types.js";

const PATH_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=@-]+$/;
const ROUTE_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=:@-]+$/;
const WINDOWS_RESERVED_RE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;
const PROTOTYPE_SEGMENTS = new Set(["__proto__", "constructor", "prototype"]);
const SOURCES = new Set<DeploySource>([
  "page-bundle",
  "sitemap",
  "robots",
  "web-app-manifest",
  "extra",
]);
const ROOT_FIELDS = new Set(["version", "baseUrl", "fileCount", "totalSizeBytes", "files"]);
const ENTRY_FIELDS = new Set([
  "outputPath",
  "contentType",
  "sizeBytes",
  "sha256",
  "source",
  "route",
  "lastmod",
]);

export const DEPLOY_LIMITS = Object.freeze({
  maxManifestCharacters: 16 * 1024 * 1024,
  maxFileCount: 100_000,
  maxFileSizeBytes: 100 * 1024 * 1024,
  maxTotalSizeBytes: 1024 * 1024 * 1024,
});

export function parseDeployManifest(input: string | unknown): DeployManifest {
  let value: unknown = input;
  if (typeof input === "string") {
    if (input.length > DEPLOY_LIMITS.maxManifestCharacters) {
      throw new TypeError(`manifest text exceeds the ${DEPLOY_LIMITS.maxManifestCharacters}-character limit`);
    }
    try {
      value = JSON.parse(input) as unknown;
    } catch (error) {
      throw new TypeError(`forme deploy manifest must be valid JSON: ${message(error)}`);
    }
  }
  const root = record(value, "manifest");
  rejectUnexpected(root, ROOT_FIELDS, "manifest");
  if (root.version !== 1) {
    throw new TypeError(`manifest.version must be 1; got ${JSON.stringify(root.version)}`);
  }
  const baseUrl = validateBaseUrl(root.baseUrl);
  const fileCount = nonNegativeSafeInteger(root.fileCount, "manifest.fileCount");
  const totalSizeBytes = nonNegativeSafeInteger(root.totalSizeBytes, "manifest.totalSizeBytes");
  if (fileCount > DEPLOY_LIMITS.maxFileCount) {
    throw new TypeError(`manifest.fileCount exceeds the ${DEPLOY_LIMITS.maxFileCount}-file limit`);
  }
  if (totalSizeBytes > DEPLOY_LIMITS.maxTotalSizeBytes) {
    throw new TypeError(`manifest.totalSizeBytes exceeds the ${DEPLOY_LIMITS.maxTotalSizeBytes}-byte limit`);
  }
  const rawFiles = record(root.files, "manifest.files");
  const paths: string[] = [];
  for (const key in rawFiles) {
    if (!Object.hasOwn(rawFiles, key)) continue;
    paths.push(key);
    if (paths.length > DEPLOY_LIMITS.maxFileCount) {
      throw new TypeError(`manifest.files exceeds the ${DEPLOY_LIMITS.maxFileCount}-file limit`);
    }
  }
  paths.sort(compareText);
  if (paths.length !== fileCount) {
    throw new TypeError(`manifest.fileCount is ${fileCount}, but manifest.files has ${paths.length} entries`);
  }

  const files: Record<string, DeployFileEntry> = Object.create(null) as Record<string, DeployFileEntry>;
  const digestSizes = new Map<string, number>();
  let measuredTotal = 0;
  for (const key of paths) {
    validateOutputPath(key, `manifest.files key ${JSON.stringify(key)}`);
    const rawEntry = record(rawFiles[key], `manifest.files[${JSON.stringify(key)}]`);
    rejectUnexpected(rawEntry, ENTRY_FIELDS, `manifest.files[${JSON.stringify(key)}]`);
    const outputPath = validateOutputPath(rawEntry.outputPath, `manifest.files[${JSON.stringify(key)}].outputPath`);
    if (outputPath !== key) {
      throw new TypeError(`manifest.files key ${JSON.stringify(key)} must equal entry.outputPath ${JSON.stringify(outputPath)}`);
    }
    const contentType = validateContentType(rawEntry.contentType, `manifest.files[${JSON.stringify(key)}].contentType`);
    const sizeBytes = nonNegativeSafeInteger(rawEntry.sizeBytes, `manifest.files[${JSON.stringify(key)}].sizeBytes`);
    if (sizeBytes > DEPLOY_LIMITS.maxFileSizeBytes) {
      throw new TypeError(`manifest.files[${JSON.stringify(key)}].sizeBytes exceeds the ${DEPLOY_LIMITS.maxFileSizeBytes}-byte per-file limit`);
    }
    const sha256 = validateSha256(rawEntry.sha256, `manifest.files[${JSON.stringify(key)}].sha256`);
    const knownSize = digestSizes.get(sha256);
    if (knownSize !== undefined && knownSize !== sizeBytes) {
      throw new TypeError(`manifest entries with the same SHA-256 must declare the same sizeBytes; digest ${sha256} has both ${knownSize} and ${sizeBytes}`);
    }
    digestSizes.set(sha256, sizeBytes);
    const source = validateSource(rawEntry.source, `manifest.files[${JSON.stringify(key)}].source`);
    const route = validateRoute(rawEntry.route, `manifest.files[${JSON.stringify(key)}].route`);
    const lastmod = validateLastmod(rawEntry.lastmod, `manifest.files[${JSON.stringify(key)}].lastmod`);
    if (source === "page-bundle" && route === undefined) {
      throw new TypeError(`manifest.files[${JSON.stringify(key)}].route is required for page-bundle entries`);
    }
    if (source !== "page-bundle" && route !== undefined) {
      throw new TypeError(`manifest.files[${JSON.stringify(key)}].route is allowed only for page-bundle entries`);
    }
    const entry: DeployFileEntry = {
      outputPath,
      contentType,
      sizeBytes,
      sha256,
      source,
      ...(route === undefined ? {} : { route }),
      ...(lastmod === undefined ? {} : { lastmod }),
    };
    files[key] = Object.freeze(entry);
    measuredTotal += sizeBytes;
    if (!Number.isSafeInteger(measuredTotal)) {
      throw new TypeError("manifest total file size exceeds JavaScript's safe integer range");
    }
  }
  rejectPrefixCollisions(paths);
  if (measuredTotal !== totalSizeBytes) {
    throw new TypeError(`manifest.totalSizeBytes is ${totalSizeBytes}, but entries total ${measuredTotal}`);
  }

  return Object.freeze({
    version: 1,
    ...(baseUrl === undefined ? {} : { baseUrl }),
    fileCount,
    totalSizeBytes,
    files: Object.freeze(files),
  });
}

export function validateOutputPath(value: unknown, field = "outputPath"): string {
  const path = nonEmptyString(value, field);
  if (path.length > 2048) throw new TypeError(`${field} must not exceed 2048 characters`);
  if (path.startsWith("/") || path.startsWith("\\") || /^[A-Za-z]:/.test(path)) {
    throw new TypeError(`${field} must be a portable relative path`);
  }
  if (path.includes("\\")) throw new TypeError(`${field} must use '/' separators only`);
  const segments = path.split("/");
  for (const segment of segments) {
    if (segment.length === 0) throw new TypeError(`${field} must not contain empty path segments`);
    if (segment === "." || segment === "..") throw new TypeError(`${field} must not contain '.' or '..' segments`);
    if (!PATH_SEGMENT_RE.test(segment)) throw new TypeError(`${field} contains a disallowed or control character`);
    if (Buffer.byteLength(segment, "utf8") > 255) throw new TypeError(`${field} contains a segment longer than 255 bytes`);
    if (segment.endsWith(".") || segment.endsWith(" ")) throw new TypeError(`${field} contains a segment with an unsafe suffix`);
    if (WINDOWS_RESERVED_RE.test(segment)) throw new TypeError(`${field} contains a Windows reserved device name`);
    if (PROTOTYPE_SEGMENTS.has(segment)) throw new TypeError(`${field} contains a prototype-pollution sink name`);
  }
  return path;
}

export function canonicalDeployManifest(manifest: DeployManifest): string {
  const files: Record<string, DeployFileEntry> = Object.create(null) as Record<string, DeployFileEntry>;
  for (const path of Object.keys(manifest.files).sort(compareText)) {
    const entry = manifest.files[path];
    if (entry === undefined) continue;
    files[path] = {
      outputPath: entry.outputPath,
      contentType: entry.contentType,
      sizeBytes: entry.sizeBytes,
      sha256: entry.sha256,
      source: entry.source,
      ...(entry.route === undefined ? {} : { route: entry.route }),
      ...(entry.lastmod === undefined ? {} : { lastmod: entry.lastmod }),
    };
  }
  return `${JSON.stringify({
    version: 1,
    ...(manifest.baseUrl === undefined ? {} : { baseUrl: manifest.baseUrl }),
    fileCount: manifest.fileCount,
    totalSizeBytes: manifest.totalSizeBytes,
    files,
  }, null, 2)}\n`;
}

export function contentDigestToStoreKey(value: unknown): string {
  const digest = validateSha256(value, "content digest");
  return Buffer.from(digest, "base64").toString("base64url");
}

function rejectPrefixCollisions(paths: readonly string[]): void {
  interface TrieNode {
    readonly children: Map<string, TrieNode>;
    terminal?: string;
  }
  const root: TrieNode = { children: new Map() };
  for (const path of paths) {
    let node = root;
    for (const segment of path.split("/")) {
      if (node.terminal !== undefined) {
        throw new TypeError(`manifest output-path prefix collision between ${JSON.stringify(node.terminal)} and ${JSON.stringify(path)}`);
      }
      let child = node.children.get(segment);
      if (child === undefined) {
        child = { children: new Map() };
        node.children.set(segment, child);
      }
      node = child;
    }
    node.terminal = path;
  }
}

function validateSha256(value: unknown, field: string): string {
  const digest = nonEmptyString(value, field);
  if (!/^[A-Za-z0-9+/]{43}=$/.test(digest)) {
    throw new TypeError(`${field} must be a canonical base64 SHA-256 digest`);
  }
  const decoded = Buffer.from(digest, "base64");
  if (decoded.byteLength !== 32 || decoded.toString("base64") !== digest) {
    throw new TypeError(`${field} must be a canonical base64 SHA-256 digest`);
  }
  return digest;
}

function validateSource(value: unknown, field: string): DeploySource {
  if (typeof value !== "string" || !SOURCES.has(value as DeploySource)) {
    throw new TypeError(`${field} must be a recognized deploy source`);
  }
  return value as DeploySource;
}

function validateBaseUrl(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  const field = "manifest.baseUrl";
  const baseUrl = boundedSafeString(value, field, 2048);
  let parsed: URL;
  try {
    parsed = new URL(baseUrl);
  } catch {
    throw new TypeError(`${field} must be an absolute HTTP(S) URL`);
  }
  if (
    (parsed.protocol !== "https:" && parsed.protocol !== "http:") ||
    parsed.username !== "" || parsed.password !== "" ||
    parsed.search !== "" || parsed.hash !== ""
  ) {
    throw new TypeError(`${field} must be an absolute HTTP(S) URL without credentials, query, or fragment`);
  }
  return baseUrl;
}

function validateContentType(value: unknown, field: string): string {
  const contentType = boundedSafeString(value, field, 255);
  if (!/^[A-Za-z0-9!#$%&'*+.^_`|~-]+\/[A-Za-z0-9!#$%&'*+.^_`|~-]+(?:\s*;\s*[A-Za-z0-9!#$%&'*+.^_`|~-]+=[A-Za-z0-9!#$%&'*+.^_`|~-]+)*$/.test(contentType)) {
    throw new TypeError(`${field} must be a valid MIME media type without control characters`);
  }
  return contentType;
}

function validateRoute(value: unknown, field: string): string | undefined {
  if (value === undefined) return undefined;
  const route = boundedSafeString(value, field, 2048);
  if (!route.startsWith("/") || route.startsWith("//") || route.includes("\\")) {
    throw new TypeError(`${field} must be a root-relative URL path`);
  }
  if (route === "/") return route;
  for (const segment of route.slice(1).split("/")) {
    if (segment.length === 0 || segment === "." || segment === "..") {
      throw new TypeError(`${field} must not contain empty, '.' or '..' path segments`);
    }
    if (!ROUTE_SEGMENT_RE.test(segment)) {
      throw new TypeError(`${field} contains a disallowed URL-path character`);
    }
  }
  return route;
}

function validateLastmod(value: unknown, field: string): string | undefined {
  if (value === undefined) return undefined;
  const lastmod = boundedSafeString(value, field, 64);
  const match = /^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}):(\d{2})(?:\.\d{1,9})?(?:Z|[+-](\d{2}):(\d{2})))?$/.exec(lastmod);
  if (match === null || !validCalendarAndClock(match) || !Number.isFinite(Date.parse(lastmod))) {
    throw new TypeError(`${field} must be an ISO 8601 date or RFC 3339 timestamp`);
  }
  return lastmod;
}

function validCalendarAndClock(match: RegExpExecArray): boolean {
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  if (month < 1 || month > 12) return false;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (day < 1 || day > (days[month - 1] ?? 0)) return false;
  if (match[4] === undefined) return true;
  return Number(match[4]) <= 23 && Number(match[5]) <= 59 && Number(match[6]) <= 59 &&
    Number(match[7] ?? 0) <= 23 && Number(match[8] ?? 0) <= 59;
}

function boundedSafeString(value: unknown, field: string, maxLength: number): string {
  const text = nonEmptyString(value, field);
  if (text.length > maxLength) throw new TypeError(`${field} exceeds the ${maxLength}-character limit`);
  if (/[\u0000-\u001F\u007F]/.test(text)) throw new TypeError(`${field} must not contain control characters`);
  return text;
}

function nonNegativeSafeInteger(value: unknown, field: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new TypeError(`${field} must be a non-negative safe integer`);
  }
  return value as number;
}

function nonEmptyString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${field} must be a non-empty string`);
  }
  return value;
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${field} must be an object`);
  }
  return value as Record<string, unknown>;
}

function rejectUnexpected(value: Record<string, unknown>, allowed: ReadonlySet<string>, field: string): void {
  for (const key in value) {
    if (!Object.hasOwn(value, key)) continue;
    if (!allowed.has(key)) throw new TypeError(`${field} has unexpected field ${JSON.stringify(key)}`);
  }
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
