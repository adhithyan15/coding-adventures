import { Buffer } from "node:buffer";
import {
  GitHubPagesBoundaryError,
  type GitHubPagesBoundary,
  type GitHubPagesBoundaryCall,
  type GitHubTreeEntry,
} from "./index.js";

const DEFAULT_API_BASE = "https://api.github.com";
const DEFAULT_API_VERSION = "2026-03-10";
const MAX_JSON_BYTES = 24 * 1024 * 1024;
const MAX_TREE_ENTRIES = 100_000;
const MAX_RESPONSE_CHUNKS = 100_000;
const MAX_OWNERSHIP_MANIFESTS = 1_024;
const MAX_OWNERSHIP_BYTES = 16 * 1024 * 1024;
const MAX_TOTAL_OWNERSHIP_BYTES = 64 * 1024 * 1024;

export interface GitHubRestBoundaryOptions {
  readonly token: string;
  readonly apiVersion?: string;
  readonly fetch?: typeof globalThis.fetch;
}

interface RequestOptions {
  readonly method?: "GET" | "POST" | "PATCH";
  readonly body?: unknown;
  readonly signal?: AbortSignal;
  readonly refUpdate?: boolean;
  readonly discardSuccessBody?: boolean;
}

interface TreeItem {
  readonly path: string;
  readonly mode: string;
  readonly type: "blob" | "tree";
  readonly sha: string;
}

type BoundaryInput<Method extends GitHubPagesBoundaryCall["method"]> = Extract<
  GitHubPagesBoundaryCall,
  { readonly method: Method }
>["input"];

export function createGitHubRestBoundary(options: GitHubRestBoundaryOptions): GitHubPagesBoundary {
  const token = validateToken(options.token);
  const apiVersion = validateApiVersion(options.apiVersion ?? DEFAULT_API_VERSION);
  const fetchImplementation = options.fetch ?? globalThis.fetch;
  if (typeof fetchImplementation !== "function") throw new TypeError("fetch implementation is required");

  const request = async <T>(path: string, requestOptions: RequestOptions = {}): Promise<T> => {
    throwIfAborted(requestOptions.signal);
    let response: Response;
    try {
      response = await fetchImplementation(`${DEFAULT_API_BASE}${path}`, {
        method: requestOptions.method ?? "GET",
        headers: {
          Accept: "application/vnd.github+json",
          Authorization: `Bearer ${token}`,
          "X-GitHub-Api-Version": apiVersion,
          "User-Agent": "coding-adventures-forme-deploy-runner",
          ...(requestOptions.body === undefined ? {} : { "Content-Type": "application/json" }),
        },
        ...(requestOptions.body === undefined ? {} : { body: JSON.stringify(requestOptions.body) }),
        ...(requestOptions.signal === undefined ? {} : { signal: requestOptions.signal }),
        redirect: "error",
      });
    } catch (error) {
      if (requestOptions.signal?.aborted === true) throw error;
      throw new GitHubPagesBoundaryError("NETWORK_ERROR", "GitHub API request failed", 503);
    }
    if (requestOptions.refUpdate !== true) throwIfAborted(requestOptions.signal);
    try {
      if (!response.ok) {
        let detail = `GitHub API returned ${response.status}`;
        try {
          detail = await responseDetail(response, token, requestOptions.signal);
        } catch (error) {
          if (requestOptions.signal?.aborted === true) throw error;
          // Diagnostics are best effort. A malformed, oversized, fragmented,
          // or failed body must not erase the authoritative HTTP status,
          // especially for an ambiguous ref-update response.
        }
        const retryAfterMs = parseRetryAfter(response.headers.get("retry-after"));
        if (requestOptions.refUpdate === true && (
          response.status === 409
          || (response.status === 422 && isRefConflictDetail(detail))
        )) {
          throw new GitHubPagesBoundaryError("REF_CONFLICT", detail, response.status);
        }
        if (response.status === 403 && retryAfterMs !== undefined) {
          throw new GitHubPagesBoundaryError("RATE_LIMITED", detail, 429, retryAfterMs);
        }
        throw new GitHubPagesBoundaryError(statusCode(response.status), detail, response.status, retryAfterMs);
      }
      if (requestOptions.discardSuccessBody === true) {
        try { void response.body?.cancel().catch(() => undefined); } catch { /* Best-effort release only. */ }
        return undefined as T;
      }
      return await parseJsonResponse<T>(response, requestOptions.signal);
    } catch (error) {
      if (requestOptions.signal?.aborted === true || error instanceof GitHubPagesBoundaryError) throw error;
      throw new GitHubPagesBoundaryError("NETWORK_ERROR", "GitHub API response stream failed", 503);
    }
  };

  const getCommit: GitHubPagesBoundary["getCommit"] = async (input) => {
    const value = await request<Record<string, unknown>>(
      repoPath(input, `git/commits/${segment(input.sha)}`),
      { signal: input.signal },
    );
    const tree = object(value.tree, "commit.tree");
    return { treeSha: stringField(tree.sha, "commit.tree.sha") };
  };

  const getTree = async (
    input: { readonly owner: string; readonly repository: string; readonly signal?: AbortSignal },
    sha: string,
    recursive = false,
  ): Promise<readonly TreeItem[]> => {
    const value = await request<Record<string, unknown>>(
      repoPath(input, `git/trees/${segment(sha)}${recursive ? "?recursive=1" : ""}`),
      { signal: input.signal },
    );
    if (value.truncated !== false) {
      throw malformed("tree response must explicitly report truncated=false");
    }
    if (!Array.isArray(value.tree)) throw malformed("tree response must contain an array");
    if (value.tree.length > MAX_TREE_ENTRIES) throw malformed(`tree response exceeds ${MAX_TREE_ENTRIES} entries`);
    return value.tree.map((raw, index) => {
      const item = object(raw, `tree[${index}]`);
      const type = stringField(item.type, `tree[${index}].type`);
      if (type !== "blob" && type !== "tree") throw malformed(`tree[${index}].type is unsupported`);
      return Object.freeze({
        path: stringField(item.path, `tree[${index}].path`),
        mode: stringField(item.mode, `tree[${index}].mode`),
        type,
        sha: stringField(item.sha, `tree[${index}].sha`),
      });
    });
  };

  return Object.freeze({
    getRef: async (input: BoundaryInput<"getRef">) => {
      const value = await request<Record<string, unknown>>(
        repoPath(input, `git/ref/${segment(input.ref)}`),
        { signal: input.signal },
      );
      const objectValue = object(value.object, "ref.object");
      return { sha: stringField(objectValue.sha, "ref.object.sha") };
    },
    getCommit,
    listOwnershipManifests: async (input: BoundaryInput<"listOwnershipManifests">) => {
      const commit = await getCommit({
        owner: input.owner,
        repository: input.repository,
        sha: input.commitSha,
        ...(input.signal === undefined ? {} : { signal: input.signal }),
      });
      const root = await getTree(input, commit.treeSha);
      const forme = findTree(root, ".forme");
      if (forme === undefined) return [];
      const formeTree = await getTree(input, forme);
      const deployments = findTree(formeTree, "deployments");
      if (deployments === undefined) return [];
      const ownershipTree = await getTree(input, deployments);
      const blobs = ownershipTree
        .filter((item) => item.path.endsWith(".json"))
        .sort((left, right) => compareText(left.path, right.path));
      if (blobs.length > MAX_OWNERSHIP_MANIFESTS) {
        throw malformed(`ownership tree exceeds ${MAX_OWNERSHIP_MANIFESTS} manifests`);
      }
      for (let index = 0; index < blobs.length; index += 1) {
        const blob = blobs[index];
        if (blob === undefined) continue;
        if (index > 0 && blobs[index - 1]?.path === blob.path) {
          throw malformed(`ownership tree repeats ${JSON.stringify(blob.path)}`);
        }
        if (blob.type !== "blob" || blob.mode !== "100644") {
          throw malformed(`ownership manifest ${JSON.stringify(blob.path)} must be one regular 100644 blob`);
        }
      }
      const result: { readonly path: string; readonly bytes: Uint8Array }[] = [];
      let totalBytes = 0;
      for (const blob of blobs) {
        const value = await request<Record<string, unknown>>(
          repoPath(input, `git/blobs/${segment(blob.sha)}`),
          { signal: input.signal },
        );
        if (value.encoding !== "base64") throw malformed(`ownership blob ${JSON.stringify(blob.path)} is not base64 encoded`);
        const content = stringField(value.content, `ownership blob ${JSON.stringify(blob.path)} content`).replace(/[\r\n]/g, "");
        if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(content)) {
          throw malformed(`ownership blob ${JSON.stringify(blob.path)} contains invalid base64`);
        }
        const bytes = Uint8Array.from(Buffer.from(content, "base64"));
        if (bytes.byteLength > MAX_OWNERSHIP_BYTES) {
          throw malformed(`ownership blob ${JSON.stringify(blob.path)} exceeds ${MAX_OWNERSHIP_BYTES} bytes`);
        }
        totalBytes += bytes.byteLength;
        if (totalBytes > MAX_TOTAL_OWNERSHIP_BYTES) {
          throw malformed(`ownership blobs exceed ${MAX_TOTAL_OWNERSHIP_BYTES} aggregate bytes`);
        }
        result.push(Object.freeze({
          path: `.forme/deployments/${blob.path}`,
          bytes,
        }));
      }
      return Object.freeze(result);
    },
    getTargetTree: async (input: BoundaryInput<"getTargetTree">) => {
      const commit = await getCommit({
        owner: input.owner,
        repository: input.repository,
        sha: input.commitSha,
        ...(input.signal === undefined ? {} : { signal: input.signal }),
      });
      const tree = await getTree(input, commit.treeSha, true);
      const result = new Map<string, { readonly type: "blob" | "tree"; readonly mode: string; readonly sha: string }>();
      for (const item of tree) {
        if (result.has(item.path)) throw malformed(`target tree repeats ${JSON.stringify(item.path)}`);
        result.set(item.path, Object.freeze({ type: item.type, mode: item.mode, sha: item.sha }));
      }
      return result;
    },
    createBlob: async (input: BoundaryInput<"createBlob">) => {
      const value = await request<Record<string, unknown>>(repoPath(input, "git/blobs"), {
        method: "POST",
        body: { content: input.contentBase64, encoding: "base64" },
        signal: input.signal,
      });
      return { sha: stringField(value.sha, "created blob sha") };
    },
    createTree: async (input: BoundaryInput<"createTree">) => {
      const value = await request<Record<string, unknown>>(repoPath(input, "git/trees"), {
        method: "POST",
        body: {
          base_tree: input.baseTreeSha,
          tree: input.entries.map(toGitHubTreeEntry),
        },
        signal: input.signal,
      });
      return { sha: stringField(value.sha, "created tree sha") };
    },
    createCommit: async (input: BoundaryInput<"createCommit">) => {
      const value = await request<Record<string, unknown>>(repoPath(input, "git/commits"), {
        method: "POST",
        body: { message: input.message, tree: input.treeSha, parents: input.parents },
        signal: input.signal,
      });
      return { sha: stringField(value.sha, "created commit sha") };
    },
    updateRef: async (input: BoundaryInput<"updateRef">) => {
      await request<Record<string, unknown>>(repoPath(input, `git/refs/${segment(input.ref)}`), {
        method: "PATCH",
        body: { sha: input.sha, force: false },
        signal: input.signal,
        refUpdate: true,
        discardSuccessBody: true,
      });
    },
  });
}

function toGitHubTreeEntry(entry: GitHubTreeEntry): Record<string, unknown> {
  return { path: entry.path, mode: entry.mode, type: entry.type, sha: entry.sha };
}

function repoPath(
  input: { readonly owner: string; readonly repository: string },
  suffix: string,
): string {
  return `/repos/${segment(input.owner)}/${segment(input.repository)}/${suffix}`;
}

function segment(value: string): string {
  return encodeURIComponent(value);
}

function findTree(entries: readonly TreeItem[], name: string): string | undefined {
  const matches = entries.filter((item) => item.path === name);
  if (matches.length === 0) return undefined;
  if (matches.length !== 1 || matches[0]?.type !== "tree") {
    throw malformed(`${JSON.stringify(name)} is not one unique tree entry`);
  }
  return matches[0].sha;
}

async function parseJsonResponse<T>(response: Response, signal?: AbortSignal): Promise<T> {
  const contentLength = response.headers.get("content-length");
  if (contentLength !== null && Number(contentLength) > MAX_JSON_BYTES) {
    throw malformed(`GitHub response exceeds ${MAX_JSON_BYTES} bytes`);
  }
  const text = await readResponse(response, MAX_JSON_BYTES, false, signal);
  if (text.length === 0) return Object.freeze({}) as T;
  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new GitHubPagesBoundaryError("MALFORMED_RESPONSE", `GitHub returned invalid JSON: ${message(error)}`);
  }
}

async function responseDetail(response: Response, token: string, signal?: AbortSignal): Promise<string> {
  const text = await readResponse(response, 4096, true, signal);
  try {
    const value = JSON.parse(text) as unknown;
    if (typeof value === "object" && value !== null && "message" in value && typeof value.message === "string") {
      return `GitHub API returned ${response.status}: ${sanitizeDiagnostic(value.message, token).slice(0, 512)}`;
    }
  } catch {
    // Fall through to a status-only diagnostic. Response bodies may be HTML.
  }
  return `GitHub API returned ${response.status}`;
}

function sanitizeDiagnostic(value: string, token: string): string {
  return value
    .split(token).join("[REDACTED]")
    .replace(/[\u0000-\u001f\u007f-\u009f]/g, " ");
}

async function readResponse(response: Response, limit: number, truncate: boolean, signal?: AbortSignal): Promise<string> {
  if (response.body === null) return "";
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  let count = 0;
  try {
    for (;;) {
      throwIfAborted(signal);
      const part = await reader.read();
      throwIfAborted(signal);
      if (part.done) break;
      count += 1;
      if (count > MAX_RESPONSE_CHUNKS) {
        cancelReader(reader);
        throw malformed("GitHub response contains too many stream chunks");
      }
      if (part.value.byteLength === 0) continue;
      const remaining = limit - total;
      if (part.value.byteLength > remaining) {
        if (truncate && remaining > 0) chunks.push(part.value.slice(0, remaining));
        cancelReader(reader);
        if (!truncate) throw malformed(`GitHub response exceeds ${limit} bytes`);
        total = limit;
        break;
      }
      chunks.push(part.value);
      total += part.value.byteLength;
    }
  } finally {
    if (signal?.aborted === true) cancelReader(reader);
    reader.releaseLock();
  }
  return Buffer.concat(chunks.map((chunk) => Buffer.from(chunk)), total).toString("utf8");
}

function throwIfAborted(signal?: AbortSignal): void {
  if (signal?.aborted === true) throw new DOMException("request was aborted", "AbortError");
}

function cancelReader(reader: ReadableStreamDefaultReader<Uint8Array>): void {
  try { void reader.cancel().catch(() => undefined); } catch { /* Best-effort cleanup only. */ }
}

function parseRetryAfter(value: string | null): number | undefined {
  if (value === null) return undefined;
  if (/^[0-9]+$/.test(value)) {
    const seconds = Number(value);
    return Number.isSafeInteger(seconds) && seconds <= Math.floor(Number.MAX_SAFE_INTEGER / 1000)
      ? seconds * 1000
      : undefined;
  }
  const at = Date.parse(value);
  if (!Number.isFinite(at)) return undefined;
  return Math.max(0, at - Date.now());
}

function statusCode(status: number): string {
  if (status === 401) return "UNAUTHORIZED";
  if (status === 403) return "FORBIDDEN";
  if (status === 404) return "NOT_FOUND";
  if (status === 409) return "CONFLICT";
  if (status === 422) return "UNPROCESSABLE";
  if (status === 429) return "RATE_LIMITED";
  if (status === 502 || status === 503 || status === 504) return "SERVICE_UNAVAILABLE";
  return "HTTP_ERROR";
}

function isRefConflictDetail(detail: string): boolean {
  return /not a fast[- ]forward|reference update failed|ref(?:erence)? (?:has )?(?:changed|advanced)/i.test(detail);
}

function validateToken(value: unknown): string {
  if (typeof value !== "string" || value.length < 1 || value.length > 4096 || /[\0\r\n]/.test(value)) {
    throw new TypeError("token must be a non-empty credential without control line breaks");
  }
  return value;
}

function validateApiVersion(value: unknown): string {
  if (typeof value !== "string" || !/^20[0-9]{2}-[0-9]{2}-[0-9]{2}$/.test(value)) {
    throw new TypeError("apiVersion must use YYYY-MM-DD format");
  }
  return value;
}

function object(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed(`${field} must be an object`);
  return value as Record<string, unknown>;
}

function stringField(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_JSON_BYTES || /\0/.test(value)) {
    throw malformed(`${field} must be a bounded non-empty string`);
  }
  return value;
}

function malformed(detail: string): GitHubPagesBoundaryError {
  return new GitHubPagesBoundaryError("MALFORMED_RESPONSE", detail);
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
