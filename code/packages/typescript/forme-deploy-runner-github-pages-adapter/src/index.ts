import { Buffer } from "node:buffer";
import {
  createVerifiedContentReader,
  parseDeployManifest,
  validateOutputPath,
  type ContentStore,
  type DeployManifest,
} from "@coding-adventures/forme-deploy-runner-core";

export { createGitHubRestBoundary, type GitHubRestBoundaryOptions } from "./github-rest.js";

const OWNER_RE = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;
const ACCOUNT_RE = /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$/;
const REPOSITORY_RE = /^[A-Za-z0-9_.-]{1,100}$/;
const GIT_SHA_RE = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const SHA256_BASE64_RE = /^(?:[A-Za-z0-9+/]{43}=)$/;
const OWNERSHIP_PREFIX = ".forme/deployments/";
const MAX_OWNERSHIP_BYTES = 16 * 1024 * 1024;
const MAX_OWNERSHIP_MANIFESTS = 1_024;
const MAX_OWNED_FILES = 100_000;
const MAX_TARGET_ENTRIES = 100_000;
const MAX_TOTAL_OWNERSHIP_BYTES = 64 * 1024 * 1024;

export type GitHubPagesPublishCode =
  | "OWNERSHIP_INVALID"
  | "OWNERSHIP_CONFLICT"
  | "ABORTED"
  | "TARGET_FAILED"
  | "RETRY_EXHAUSTED"
  | "INDETERMINATE";

export class GitHubPagesPublishError extends Error {
  readonly code: GitHubPagesPublishCode;
  override readonly cause?: unknown;

  constructor(code: GitHubPagesPublishCode, detail: string, cause?: unknown) {
    super(`${code}: ${detail}`);
    this.name = "GitHubPagesPublishError";
    this.code = code;
    this.cause = cause;
  }
}

export class GitHubPagesBoundaryError extends Error {
  readonly code: string;
  readonly status?: number;
  readonly retryAfterMs?: number;

  constructor(code: string, detail: string, status?: number, retryAfterMs?: number) {
    super(`${code}: ${detail}`);
    this.name = "GitHubPagesBoundaryError";
    this.code = code;
    this.status = status;
    this.retryAfterMs = retryAfterMs;
  }
}

interface RepositoryInput {
  readonly owner: string;
  readonly repository: string;
  readonly signal?: AbortSignal;
}

export type GitHubPagesBoundaryCall =
  | { readonly method: "getRef"; readonly input: RepositoryInput & { readonly ref: string } }
  | { readonly method: "getCommit"; readonly input: RepositoryInput & { readonly sha: string } }
  | { readonly method: "listOwnershipManifests"; readonly input: RepositoryInput & { readonly commitSha: string } }
  | { readonly method: "getTargetTree"; readonly input: RepositoryInput & {
      readonly commitSha: string;
    } }
  | { readonly method: "createBlob"; readonly input: RepositoryInput & { readonly contentBase64: string } }
  | { readonly method: "createTree"; readonly input: RepositoryInput & {
      readonly baseTreeSha: string;
      readonly entries: readonly GitHubTreeEntry[];
    } }
  | { readonly method: "createCommit"; readonly input: RepositoryInput & {
      readonly message: string;
      readonly treeSha: string;
      readonly parents: readonly string[];
    } }
  | { readonly method: "updateRef"; readonly input: RepositoryInput & {
      readonly ref: string;
      readonly sha: string;
      readonly force: false;
    } };

export interface GitHubTreeEntry {
  readonly path: string;
  readonly mode: "100644";
  readonly type: "blob";
  readonly sha: string | null;
}

export interface GitHubTargetEntry {
  readonly type: "blob" | "tree";
  readonly mode: string;
  readonly sha: string;
}

export interface GitHubPagesBoundary {
  readonly getRef: (
    input: Extract<GitHubPagesBoundaryCall, { method: "getRef" }>["input"],
  ) => Promise<{ readonly sha: string }>;
  readonly getCommit: (
    input: Extract<GitHubPagesBoundaryCall, { method: "getCommit" }>["input"],
  ) => Promise<{ readonly treeSha: string }>;
  readonly listOwnershipManifests: (
    input: Extract<GitHubPagesBoundaryCall, { method: "listOwnershipManifests" }>["input"],
  ) => Promise<readonly { readonly path: string; readonly bytes: Uint8Array }[]>;
  readonly getTargetTree: (
    input: Extract<GitHubPagesBoundaryCall, { method: "getTargetTree" }>["input"],
  ) => Promise<ReadonlyMap<string, GitHubTargetEntry>>;
  readonly createBlob: (
    input: Extract<GitHubPagesBoundaryCall, { method: "createBlob" }>["input"],
  ) => Promise<{ readonly sha: string }>;
  readonly createTree: (
    input: Extract<GitHubPagesBoundaryCall, { method: "createTree" }>["input"],
  ) => Promise<{ readonly sha: string }>;
  readonly createCommit: (
    input: Extract<GitHubPagesBoundaryCall, { method: "createCommit" }>["input"],
  ) => Promise<{ readonly sha: string }>;
  readonly updateRef: (
    input: Extract<GitHubPagesBoundaryCall, { method: "updateRef" }>["input"],
  ) => Promise<void>;
}

export interface GitHubPagesPublishOptions {
  readonly manifest: unknown;
  readonly contentStore: ContentStore;
  readonly boundary: GitHubPagesBoundary;
  readonly owner: string;
  readonly repository: string;
  /** GitHub Git Data API ref without the leading `refs/`. */
  readonly ref: string;
  /** Stable ownership identifier used for the reserved target-side manifest. */
  readonly deploymentOwner: string;
  /** Portable branch prefix. Empty string publishes at the branch root. */
  readonly destination: string;
  /** Number of retries after the initial request/commit attempt. */
  readonly retryLimit?: number;
  readonly maxRetryDelayMs?: number;
  readonly signal?: AbortSignal;
  readonly sleep?: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
  readonly commitMessage?: string;
}

export interface GitHubPagesPublishResult {
  readonly status: "published" | "unchanged";
  readonly commitSha: string;
  readonly attempts: number;
  readonly fileCount: number;
  readonly totalSizeBytes: number;
}

interface ValidatedOptions {
  readonly manifest: DeployManifest;
  readonly contentStore: ContentStore;
  readonly boundary: GitHubPagesBoundary;
  readonly owner: string;
  readonly repository: string;
  readonly ref: string;
  readonly deploymentOwner: string;
  readonly destination: string;
  readonly retryLimit: number;
  readonly maxRetryDelayMs: number;
  readonly signal?: AbortSignal;
  readonly sleep: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
  readonly commitMessage: string;
  readonly desiredOwnershipUpperBytes: number;
}

interface OwnershipFile {
  readonly sha256: string;
  readonly gitBlobSha: string;
}

interface OwnershipManifest {
  readonly version: 1;
  readonly owner: string;
  readonly destination: string;
  readonly files: Readonly<Record<string, OwnershipFile>>;
}

interface BaseState {
  readonly commitSha: string;
  readonly treeSha: string;
  readonly owners: ReadonlyMap<string, OwnershipManifest>;
  readonly targetEntries: ReadonlyMap<string, GitHubTargetEntry>;
  readonly ownershipBytesByOwner: ReadonlyMap<string, number>;
  readonly totalOwnershipBytes: number;
}

export async function publishGitHubPagesSite(
  rawOptions: GitHubPagesPublishOptions,
): Promise<GitHubPagesPublishResult> {
  const options = validateOptions(rawOptions);
  const reader = createVerifiedContentReader(options.manifest, options.contentStore, {
    signal: options.signal,
  });
  throwIfAborted(options.signal);
  // Fail on missing or mutable content before any target access or Git object
  // creation. Individual reads are verified again immediately before upload.
  await reader.preflight();
  throwIfAborted(options.signal);

  let base = await readBaseState(options);
  validateOwnershipSet(base.owners, options);
  validateTargetShape(base.targetEntries, base.owners, options);
  validateProjectedOwnership(base, options, options.desiredOwnershipUpperBytes);

  const desiredFiles: Record<string, OwnershipFile> = Object.create(null) as Record<string, OwnershipFile>;
  const manifestPaths = Object.keys(options.manifest.files).sort(compareText);
  for (const outputPath of manifestPaths) {
    throwIfAborted(options.signal);
    const entry = options.manifest.files[outputPath];
    if (entry === undefined) continue;
    const bytes = await reader.read(outputPath);
    const blob = await callWithRetry(options, () => options.boundary.createBlob(withSignal(options, {
      owner: options.owner,
      repository: options.repository,
      contentBase64: Buffer.from(bytes).toString("base64"),
    })));
    validateGitSha(blob.sha, "created blob SHA");
    desiredFiles[outputPath] = Object.freeze({ sha256: entry.sha256, gitBlobSha: blob.sha });
  }
  const desired = freezeOwnership(options.deploymentOwner, options.destination, desiredFiles);
  const serializedOwnership = serializeOwnership(desired);
  const serializedOwnershipBytes = Buffer.byteLength(serializedOwnership, "utf8");
  if (serializedOwnershipBytes > MAX_OWNERSHIP_BYTES) {
    ownershipInvalid(`desired ownership manifest exceeds ${MAX_OWNERSHIP_BYTES} bytes`);
  }
  const ownershipBlob = await callWithRetry(options, () => options.boundary.createBlob(withSignal(options, {
    owner: options.owner,
    repository: options.repository,
    contentBase64: Buffer.from(serializedOwnership, "utf8").toString("base64"),
  })));
  validateGitSha(ownershipBlob.sha, "created ownership blob SHA");
  const current = base.owners.get(options.deploymentOwner);
  if (current !== undefined && ownershipMatchesDesired(current, desired)) {
    return result("unchanged", base.commitSha, 1, options.manifest);
  }

  for (let attempt = 1; attempt <= options.retryLimit + 1; attempt += 1) {
    if (attempt > 1) {
      base = await readBaseState(options);
      validateOwnershipSet(base.owners, options);
      validateTargetShape(base.targetEntries, base.owners, options);
      validateProjectedOwnership(base, options, serializedOwnershipBytes);
      const latest = base.owners.get(options.deploymentOwner);
      if (latest !== undefined && ownershipMatchesDesired(latest, desired)) {
        return result("unchanged", base.commitSha, attempt, options.manifest);
      }
    }
    throwIfAborted(options.signal);
    validateProjectedOwnership(base, options, serializedOwnershipBytes);
    const previous = base.owners.get(options.deploymentOwner);
    const entries = createTreeEntries(options, desired, previous, ownershipBlob.sha);
    const tree = await callWithRetry(options, () => options.boundary.createTree(withSignal(options, {
      owner: options.owner,
      repository: options.repository,
      baseTreeSha: base.treeSha,
      entries,
    })));
    validateGitSha(tree.sha, "created tree SHA");
    const commit = await callWithRetry(options, () => options.boundary.createCommit(withSignal(options, {
      owner: options.owner,
      repository: options.repository,
      message: options.commitMessage,
      treeSha: tree.sha,
      parents: [base.commitSha],
    })));
    validateGitSha(commit.sha, "created commit SHA");
    const candidate = await readCommitState(options, commit.sha);
    validateOwnershipSet(candidate.owners, options);
    validateTargetShape(candidate.targetEntries, candidate.owners, options);
    const candidateOwner = candidate.owners.get(options.deploymentOwner);
    if (candidateOwner === undefined || !ownershipMatchesDesired(candidateOwner, desired)) {
      throw new GitHubPagesPublishError("TARGET_FAILED", "candidate commit does not contain the exact desired ownership state");
    }
    for (const outputPath of Object.keys(previous?.files ?? {})) {
      if (desired.files[outputPath] !== undefined) continue;
      const stalePath = targetPath(options.destination, outputPath);
      if (candidate.targetEntries.has(stalePath)) {
        throw new GitHubPagesPublishError(
          "TARGET_FAILED",
          `candidate commit retained stale owned path ${JSON.stringify(stalePath)}`,
        );
      }
    }
    try {
      throwIfAborted(options.signal);
      await updateRefWithRetry(options, withSignal(options, {
        owner: options.owner,
        repository: options.repository,
        ref: options.ref,
        sha: commit.sha,
        force: false,
      }));
      return result("published", commit.sha, attempt, options.manifest);
    } catch (error) {
      if (!isRefConflict(error)) {
        const normalized = normalizeBoundaryError(error);
        if (!isAmbiguousUpdate(normalized)) throw normalized;
        try {
          const confirmation = await readBaseState(options);
          validateOwnershipSet(confirmation.owners, options);
          const confirmed = confirmation.owners.get(options.deploymentOwner);
          if (confirmed !== undefined && ownershipMatchesDesired(confirmed, desired)) {
            return result("published", confirmation.commitSha, attempt, options.manifest);
          }
        } catch (confirmationError) {
          throw new GitHubPagesPublishError(
            "INDETERMINATE",
            "the ref update had an ambiguous transport outcome and target confirmation failed",
            Object.freeze({ updateError: normalized, confirmationError }),
          );
        }
        throw new GitHubPagesPublishError(
          "INDETERMINATE",
          "the ref update had an ambiguous transport outcome and the candidate publication could not be confirmed",
          normalized,
        );
      }
      if (attempt > options.retryLimit) {
        throw new GitHubPagesPublishError(
          "RETRY_EXHAUSTED",
          `source ref advanced during all ${attempt} publication attempt${attempt === 1 ? "" : "s"}`,
          error,
        );
      }
    }
  }
  throw new GitHubPagesPublishError("RETRY_EXHAUSTED", "publication retry budget exhausted");
}

async function readBaseState(options: ValidatedOptions): Promise<BaseState> {
  const ref = await callWithRetry(options, () => options.boundary.getRef(withSignal(options, {
    owner: options.owner,
    repository: options.repository,
    ref: options.ref,
  })));
  validateGitSha(ref.sha, "source ref SHA");
  return await readCommitState(options, ref.sha);
}

async function readCommitState(options: ValidatedOptions, commitSha: string): Promise<BaseState> {
  const [commit, rawOwners] = await Promise.all([
    callWithRetry(options, () => options.boundary.getCommit(withSignal(options, {
      owner: options.owner,
      repository: options.repository,
      sha: commitSha,
    }))),
    callWithRetry(options, () => options.boundary.listOwnershipManifests(withSignal(options, {
      owner: options.owner,
      repository: options.repository,
      commitSha,
    }))),
  ]);
  validateGitSha(commit.treeSha, "base tree SHA");
  if (!Array.isArray(rawOwners)) ownershipInvalid("ownership listing must be an array");
  if (rawOwners.length > MAX_OWNERSHIP_MANIFESTS) {
    ownershipInvalid(`ownership listing exceeds the ${MAX_OWNERSHIP_MANIFESTS}-manifest limit`);
  }
  const owners = new Map<string, OwnershipManifest>();
  const ownershipBytesByOwner = new Map<string, number>();
  let totalBytes = 0;
  for (const raw of rawOwners) {
    if (typeof raw !== "object" || raw === null || typeof raw.path !== "string") {
      ownershipInvalid("ownership listing contains an invalid entry");
    }
    if (!(raw.bytes instanceof Uint8Array)) ownershipInvalid(`${raw.path} did not return bytes`);
    if (raw.bytes.byteLength > MAX_OWNERSHIP_BYTES) ownershipInvalid(`${raw.path} exceeds the ownership-manifest byte limit`);
    totalBytes += raw.bytes.byteLength;
    if (totalBytes > MAX_TOTAL_OWNERSHIP_BYTES) {
      ownershipInvalid(`ownership listing exceeds the ${MAX_TOTAL_OWNERSHIP_BYTES}-byte aggregate limit`);
    }
    const owner = ownerFromManifestPath(raw.path);
    if (owners.has(owner)) ownershipInvalid(`duplicate ownership manifest for ${JSON.stringify(owner)}`);
    owners.set(owner, parseOwnership(raw.bytes, owner));
    ownershipBytesByOwner.set(owner, raw.bytes.byteLength);
  }
  const targetEntries = await callWithRetry(options, () => options.boundary.getTargetTree(withSignal(options, {
    owner: options.owner,
    repository: options.repository,
    commitSha,
  })));
  if (!(targetEntries instanceof Map)) ownershipInvalid("target tree must be a Map");
  if (targetEntries.size > MAX_TARGET_ENTRIES) ownershipInvalid(`target tree exceeds ${MAX_TARGET_ENTRIES} entries`);
  for (const [path, entry] of targetEntries) {
    if (typeof path !== "string" || path.length === 0 || path.length > 4096) ownershipInvalid("target tree has an invalid path");
    if (typeof entry !== "object" || entry === null || (entry.type !== "blob" && entry.type !== "tree")) {
      ownershipInvalid(`target tree entry ${JSON.stringify(path)} is invalid`);
    }
    if (typeof entry.mode !== "string" || !/^[0-7]{6}$/.test(entry.mode)) {
      ownershipInvalid(`target tree entry ${JSON.stringify(path)} has an invalid mode`);
    }
    validateGitSha(entry.sha, `target tree entry ${JSON.stringify(path)} SHA`);
  }
  for (const manifest of owners.values()) {
    for (const [outputPath, expected] of Object.entries(manifest.files)) {
      const path = targetPath(manifest.destination, outputPath);
      const actual = targetEntries.get(path);
      if (actual?.type !== "blob" || actual.mode !== "100644" || actual.sha !== expected.gitBlobSha) {
        ownershipInvalid(`${JSON.stringify(path)} does not match its recorded Git blob`);
      }
    }
  }
  return Object.freeze({
    commitSha,
    treeSha: commit.treeSha,
    owners,
    targetEntries,
    ownershipBytesByOwner,
    totalOwnershipBytes: totalBytes,
  });
}

function validateProjectedOwnership(base: BaseState, options: ValidatedOptions, desiredBytes: number): void {
  const previousBytes = base.ownershipBytesByOwner.get(options.deploymentOwner);
  const projectedCount = base.owners.size + (previousBytes === undefined ? 1 : 0);
  if (projectedCount > MAX_OWNERSHIP_MANIFESTS) {
    ownershipInvalid(`publication would exceed the ${MAX_OWNERSHIP_MANIFESTS}-manifest ownership limit`);
  }
  const projectedBytes = base.totalOwnershipBytes - (previousBytes ?? 0) + desiredBytes;
  if (projectedBytes > MAX_TOTAL_OWNERSHIP_BYTES) {
    ownershipInvalid(`publication would exceed the ${MAX_TOTAL_OWNERSHIP_BYTES}-byte aggregate ownership limit`);
  }
}

function validateTargetShape(
  targetEntries: ReadonlyMap<string, GitHubTargetEntry>,
  owners: ReadonlyMap<string, OwnershipManifest>,
  options: ValidatedOptions,
): void {
  const sortedPaths = [...targetEntries.keys()].sort(compareText);
  const previous = owners.get(options.deploymentOwner);
  for (const outputPath of Object.keys(options.manifest.files)) {
    const desired = targetPath(options.destination, outputPath);
    const segments = desired.split("/");
    let ancestor = "";
    for (const segment of segments.slice(0, -1)) {
      ancestor = ancestor === "" ? segment : `${ancestor}/${segment}`;
      if (targetEntries.get(ancestor)?.type === "blob") {
        throw new GitHubPagesPublishError(
          "OWNERSHIP_CONFLICT",
          `desired file ${JSON.stringify(desired)} is below existing target blob ${JSON.stringify(ancestor)}`,
        );
      }
    }
    const exact = targetEntries.get(desired);
    if (exact?.type === "blob" && previous?.files[outputPath] === undefined) {
      throw new GitHubPagesPublishError(
        "OWNERSHIP_CONFLICT",
        `desired file ${JSON.stringify(desired)} would overwrite an unowned target blob`,
      );
    }
    if (targetEntries.get(desired)?.type === "tree" || hasSortedPrefix(sortedPaths, `${desired}/`)) {
      throw new GitHubPagesPublishError(
        "OWNERSHIP_CONFLICT",
        `desired file ${JSON.stringify(desired)} would replace an existing target directory`,
      );
    }
  }
}

function hasSortedPrefix(sorted: readonly string[], prefix: string): boolean {
  let low = 0;
  let high = sorted.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if ((sorted[middle] ?? "") < prefix) low = middle + 1;
    else high = middle;
  }
  return sorted[low]?.startsWith(prefix) === true;
}

function createTreeEntries(
  options: ValidatedOptions,
  desired: OwnershipManifest,
  previous: OwnershipManifest | undefined,
  ownershipBlobSha: string,
): readonly GitHubTreeEntry[] {
  const entries = new Map<string, GitHubTreeEntry>();
  for (const [outputPath, file] of Object.entries(desired.files)) {
    const path = targetPath(options.destination, outputPath);
    entries.set(path, Object.freeze({ path, mode: "100644", type: "blob", sha: file.gitBlobSha }));
  }
  if (previous !== undefined) {
    for (const outputPath of Object.keys(previous.files)) {
      if (desired.files[outputPath] !== undefined) continue;
      const path = targetPath(options.destination, outputPath);
      entries.set(path, Object.freeze({ path, mode: "100644", type: "blob", sha: null }));
    }
  }
  const ownershipPath = `${OWNERSHIP_PREFIX}${options.deploymentOwner}.json`;
  entries.set(ownershipPath, Object.freeze({
    path: ownershipPath,
    mode: "100644",
    type: "blob",
    sha: ownershipBlobSha,
  }));
  return Object.freeze([...entries.values()].sort((left, right) => compareText(left.path, right.path)));
}

function validateOwnershipSet(
  owners: ReadonlyMap<string, OwnershipManifest>,
  options: ValidatedOptions,
): void {
  const storedPaths: { readonly path: string; readonly owner: string }[] = [];
  for (const [storedOwner, manifest] of owners) {
    for (const outputPath of Object.keys(manifest.files)) {
      storedPaths.push({ path: targetPath(manifest.destination, outputPath), owner: storedOwner });
    }
  }
  const storedCollision = firstCollision(storedPaths);
  if (storedCollision !== undefined) {
    ownershipInvalid(
      `${JSON.stringify(storedCollision.left.path)} owned by ${JSON.stringify(storedCollision.left.owner)} overlaps ${JSON.stringify(storedCollision.right.path)} owned by ${JSON.stringify(storedCollision.right.owner)}`,
    );
  }
  const own = owners.get(options.deploymentOwner);
  if (own !== undefined && own.destination !== options.destination) {
    ownershipInvalid(
      `deployment owner ${JSON.stringify(options.deploymentOwner)} is bound to destination ${JSON.stringify(own.destination)}, not ${JSON.stringify(options.destination)}`,
    );
  }
  const desiredOwner = options.deploymentOwner;
  const combined = storedPaths.filter(({ owner }) => owner !== desiredOwner);
  combined.push(...Object.keys(options.manifest.files).map((path) => ({
    path: targetPath(options.destination, path),
    owner: desiredOwner,
  })));
  const collision = firstCollision(combined);
  if (collision !== undefined) {
    throw new GitHubPagesPublishError(
      "OWNERSHIP_CONFLICT",
      `${JSON.stringify(collision.left.path)} owned by ${JSON.stringify(collision.left.owner)} overlaps ${JSON.stringify(collision.right.path)} owned by ${JSON.stringify(collision.right.owner)}`,
    );
  }
}

function parseOwnership(bytes: Uint8Array, expectedOwner: string): OwnershipManifest {
  let value: unknown;
  try {
    value = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown;
  } catch (error) {
    ownershipInvalid(`manifest for ${JSON.stringify(expectedOwner)} is not valid UTF-8 JSON`, error);
  }
  const root = record(value, `ownership manifest ${JSON.stringify(expectedOwner)}`);
  exactKeys(root, ["version", "owner", "destination", "files"], "ownership manifest");
  if (root.version !== 1) ownershipInvalid("ownership manifest version must be 1");
  const owner = validateDeploymentOwner(root.owner);
  if (owner !== expectedOwner) ownershipInvalid(`ownership manifest owner ${JSON.stringify(owner)} does not match its path`);
  const destination = validateDestination(root.destination);
  const rawFiles = record(root.files, "ownership manifest files");
  const outputPaths = Object.keys(rawFiles).sort(compareText);
  if (outputPaths.length > MAX_OWNED_FILES) {
    ownershipInvalid(`ownership manifest exceeds the ${MAX_OWNED_FILES}-file limit`);
  }
  for (const outputPath of outputPaths) {
    try {
      validateOutputPath(outputPath, "ownership output path");
      rejectReservedTarget(destination, outputPath);
    } catch (error) {
      ownershipInvalid(`ownership output path ${JSON.stringify(outputPath.slice(0, 256))} is invalid`, error);
    }
  }
  const pathCollision = firstCollision(outputPaths.map((path) => ({ path, owner })));
  if (pathCollision !== undefined) {
    ownershipInvalid(`ownership paths ${JSON.stringify(pathCollision.left.path)} and ${JSON.stringify(pathCollision.right.path)} collide`);
  }
  const files: Record<string, OwnershipFile> = Object.create(null) as Record<string, OwnershipFile>;
  for (const outputPath of outputPaths) {
    const rawFile = record(rawFiles[outputPath], `ownership file ${JSON.stringify(outputPath)}`);
    exactKeys(rawFile, ["sha256", "gitBlobSha"], "ownership file");
    if (typeof rawFile.sha256 !== "string" || !SHA256_BASE64_RE.test(rawFile.sha256)) {
      ownershipInvalid(`ownership file ${JSON.stringify(outputPath)} has an invalid SHA-256`);
    }
    if (typeof rawFile.gitBlobSha !== "string" || !GIT_SHA_RE.test(rawFile.gitBlobSha)) {
      ownershipInvalid(`ownership file ${JSON.stringify(outputPath)} has an invalid Git blob SHA`);
    }
    files[outputPath] = Object.freeze({ sha256: rawFile.sha256, gitBlobSha: rawFile.gitBlobSha });
  }
  return freezeOwnership(owner, destination, files);
}

function serializeOwnership(manifest: OwnershipManifest): string {
  const entries = Object.keys(manifest.files).sort(compareText).map((outputPath) => [
    outputPath,
    manifest.files[outputPath]!,
  ] as const);
  const header = `{"version":1,"owner":${JSON.stringify(manifest.owner)},"destination":${JSON.stringify(manifest.destination)},"files":{`;
  const body = entries.map(([path, file]) => (
    `${JSON.stringify(path)}:{"sha256":${JSON.stringify(file.sha256)},"gitBlobSha":${JSON.stringify(file.gitBlobSha)}}`
  )).join(",");
  return `${header}${body}}}\n`;
}

function ownershipSerializedByteLength(
  owner: string,
  destination: string,
  entries: readonly (readonly [string, OwnershipFile])[],
): number {
  let bytes = Buffer.byteLength(
    `{"version":1,"owner":${JSON.stringify(owner)},"destination":${JSON.stringify(destination)},"files":{}}\n`,
    "utf8",
  );
  for (const [index, [path, file]] of entries.entries()) {
    bytes += Buffer.byteLength(
      `${index === 0 ? "" : ","}${JSON.stringify(path)}:{"sha256":${JSON.stringify(file.sha256)},"gitBlobSha":${JSON.stringify(file.gitBlobSha)}}`,
      "utf8",
    );
    if (bytes > MAX_OWNERSHIP_BYTES) return bytes;
  }
  return bytes;
}

function freezeOwnership(
  owner: string,
  destination: string,
  files: Record<string, OwnershipFile>,
): OwnershipManifest {
  return Object.freeze({ version: 1, owner, destination, files: Object.freeze(files) });
}

function ownershipMatchesDesired(left: OwnershipManifest, right: OwnershipManifest): boolean {
  if (left.destination !== right.destination) return false;
  const paths = Object.keys(right.files);
  if (paths.length !== Object.keys(left.files).length) return false;
  return paths.every((path) => {
    const a = left.files[path];
    const b = right.files[path];
    return a?.sha256 === b?.sha256 && a.gitBlobSha === b.gitBlobSha;
  });
}

function validateOptions(options: GitHubPagesPublishOptions): ValidatedOptions {
  const manifest = parseDeployManifest(options.manifest);
  const owner = requiredString(options.owner, "owner");
  if (!ACCOUNT_RE.test(owner)) throw new TypeError("owner must be a portable GitHub account name");
  const repository = requiredString(options.repository, "repository");
  if (!REPOSITORY_RE.test(repository) || repository === "." || repository === "..") {
    throw new TypeError("repository must be a GitHub repository name without slashes");
  }
  const ref = validateRef(options.ref);
  const deploymentOwner = validateDeploymentOwner(options.deploymentOwner);
  const destination = validateDestination(options.destination);
  for (const outputPath of Object.keys(manifest.files)) {
    validateOutputPath(targetPath(destination, outputPath), "combined target path");
    rejectReservedTarget(destination, outputPath);
  }
  const retryLimit = nonNegativeSafeInteger(options.retryLimit ?? 3, "retryLimit", 10);
  const maxRetryDelayMs = nonNegativeSafeInteger(options.maxRetryDelayMs ?? 30_000, "maxRetryDelayMs", 300_000);
  if (typeof options.boundary !== "object" || options.boundary === null) throw new TypeError("boundary is required");
  if (typeof options.contentStore !== "object" || options.contentStore === null) throw new TypeError("contentStore is required");
  if (options.sleep !== undefined && typeof options.sleep !== "function") throw new TypeError("sleep must be a function");
  const commitMessage = options.commitMessage ?? `forme deploy: ${deploymentOwner}`;
  if (typeof commitMessage !== "string" || commitMessage.length < 1 || commitMessage.length > 256 || /[\0\r\n]/.test(commitMessage)) {
    throw new TypeError("commitMessage must be 1-256 characters without NUL or line breaks");
  }
  const desiredOwnershipUpperBytes = ownershipSerializedByteLength(
    deploymentOwner,
    destination,
    Object.keys(manifest.files).sort(compareText).map((outputPath) => [outputPath, {
      sha256: manifest.files[outputPath]?.sha256 ?? "",
      gitBlobSha: "0".repeat(64),
    }] as const),
  );
  if (desiredOwnershipUpperBytes > MAX_OWNERSHIP_BYTES) {
    ownershipInvalid(`desired ownership manifest can exceed ${MAX_OWNERSHIP_BYTES} bytes`);
  }
  return Object.freeze({
    manifest,
    contentStore: options.contentStore,
    boundary: options.boundary,
    owner,
    repository,
    ref,
    deploymentOwner,
    destination,
    retryLimit,
    maxRetryDelayMs,
    ...(options.signal === undefined ? {} : { signal: options.signal }),
    sleep: options.sleep ?? abortableSleep,
    commitMessage,
    desiredOwnershipUpperBytes,
  });
}

async function callWithRetry<T>(
  options: ValidatedOptions,
  operation: () => Promise<T>,
): Promise<T> {
  for (let attempt = 0; ; attempt += 1) {
    throwIfAborted(options.signal);
    try {
      const value = await raceAbort(operation(), options.signal);
      throwIfAborted(options.signal);
      return value;
    } catch (error) {
      if (options.signal?.aborted === true) throw aborted();
      if (isRefConflict(error)) throw error;
      if (!isTransient(error) || attempt >= options.retryLimit) throw normalizeBoundaryError(error);
      const requested = error instanceof GitHubPagesBoundaryError ? error.retryAfterMs : undefined;
      const delay = Math.min(options.maxRetryDelayMs, requested ?? Math.min(1000 * (2 ** attempt), 30_000));
      await raceAbort(options.sleep(delay, options.signal), options.signal);
    }
  }
}

async function updateRefWithRetry(
  options: ValidatedOptions,
  input: Extract<GitHubPagesBoundaryCall, { method: "updateRef" }>["input"],
): Promise<void> {
  for (let attempt = 0; ; attempt += 1) {
    try {
      await raceRefUpdate(options.boundary.updateRef(input), options.signal);
      return;
    } catch (error) {
      if (options.signal?.aborted === true) throw error;
      if (!(error instanceof GitHubPagesBoundaryError) || error.status !== 429 || attempt >= options.retryLimit) {
        throw error;
      }
      const delay = Math.min(
        options.maxRetryDelayMs,
        error.retryAfterMs ?? Math.min(1000 * (2 ** attempt), 30_000),
      );
      await raceAbort(options.sleep(delay, options.signal), options.signal);
    }
  }
}

function normalizeBoundaryError(error: unknown): Error {
  if (error instanceof GitHubPagesBoundaryError || error instanceof GitHubPagesPublishError) return error;
  return new GitHubPagesPublishError("TARGET_FAILED", message(error), error);
}

function isTransient(error: unknown): boolean {
  if (!(error instanceof GitHubPagesBoundaryError)) return false;
  return error.status === 429 || error.status === 502 || error.status === 503 || error.status === 504;
}

function isRefConflict(error: unknown): boolean {
  return error instanceof GitHubPagesBoundaryError && error.code === "REF_CONFLICT"
    && (error.status === 409 || error.status === 422);
}

function isAmbiguousUpdate(error: unknown): boolean {
  return error instanceof GitHubPagesBoundaryError && (
    error.code === "NETWORK_ERROR"
    || (error.status !== undefined && error.status >= 500 && error.status <= 599)
  );
}

function validateRef(value: unknown): string {
  const ref = requiredString(value, "ref");
  if (!ref.startsWith("heads/") || ref.length > 255 || /[\0-\x20~^:?*[\\]/.test(ref) || ref.includes("..") || ref.endsWith(".") || ref.endsWith("/")) {
    throw new TypeError("ref must be a safe branch ref beginning with 'heads/'");
  }
  for (const segment of ref.split("/")) {
    if (segment.length === 0 || segment === "." || segment === ".." || segment.startsWith(".")) {
      throw new TypeError("ref contains an unsafe path segment");
    }
  }
  return ref;
}

function validateDeploymentOwner(value: unknown): string {
  const owner = requiredString(value, "deploymentOwner");
  if (!OWNER_RE.test(owner)) throw new TypeError("deploymentOwner must be 1-63 lowercase letters, digits, or interior hyphens");
  return owner;
}

function validateDestination(value: unknown): string {
  if (value === "") return "";
  const destination = validateOutputPath(value, "destination");
  if (destination === ".forme" || destination.startsWith(".forme/")) {
    throw new TypeError("destination must not use Forme's reserved .forme namespace");
  }
  return destination;
}

function rejectReservedTarget(destination: string, outputPath: string): void {
  const path = targetPath(destination, outputPath);
  if (path === ".forme" || path.startsWith(".forme/")) {
    throw new TypeError("manifest output paths must not use Forme's reserved .forme namespace");
  }
}

function ownerFromManifestPath(path: string): string {
  if (!path.startsWith(OWNERSHIP_PREFIX) || !path.endsWith(".json")) {
    ownershipInvalid(`unexpected ownership manifest path ${JSON.stringify(path)}`);
  }
  const owner = path.slice(OWNERSHIP_PREFIX.length, -5);
  try {
    return validateDeploymentOwner(owner);
  } catch (error) {
    ownershipInvalid(`unsafe ownership manifest path ${JSON.stringify(path)}`, error);
  }
}

function targetPath(destination: string, outputPath: string): string {
  return destination === "" ? outputPath : `${destination}/${outputPath}`;
}

function firstCollision(
  paths: readonly { readonly path: string; readonly owner: string }[],
): { readonly left: { readonly path: string; readonly owner: string }; readonly right: { readonly path: string; readonly owner: string } } | undefined {
  const byPath = new Map<string, { readonly path: string; readonly owner: string }>();
  for (const candidate of [...paths].sort((left, right) => compareText(left.path, right.path))) {
    const duplicate = byPath.get(candidate.path);
    if (duplicate !== undefined) return { left: duplicate, right: candidate };
    byPath.set(candidate.path, candidate);
  }
  for (const candidate of byPath.values()) {
    let slash = candidate.path.indexOf("/");
    while (slash !== -1) {
      const ancestor = byPath.get(candidate.path.slice(0, slash));
      if (ancestor !== undefined) return { left: ancestor, right: candidate };
      slash = candidate.path.indexOf("/", slash + 1);
    }
  }
  return undefined;
}

function validateGitSha(value: unknown, field: string): asserts value is string {
  if (typeof value !== "string" || !GIT_SHA_RE.test(value)) {
    throw new GitHubPagesPublishError("TARGET_FAILED", `${field} is invalid`);
  }
}

function result(
  status: GitHubPagesPublishResult["status"],
  commitSha: string,
  attempts: number,
  manifest: DeployManifest,
): GitHubPagesPublishResult {
  return Object.freeze({
    status,
    commitSha,
    attempts,
    fileCount: manifest.fileCount,
    totalSizeBytes: manifest.totalSizeBytes,
  });
}

function withSignal<T extends object>(options: ValidatedOptions, input: T): T & { readonly signal?: AbortSignal } {
  return options.signal === undefined ? input : { ...input, signal: options.signal };
}

async function raceAbort<T>(promise: Promise<T>, signal?: AbortSignal): Promise<T> {
  if (signal === undefined) return promise;
  // The operation may synchronously abort the signal and return an already
  // rejected promise. Attach an observer before the early-abort branch so the
  // losing operation can never become an unhandled rejection.
  void promise.catch(() => undefined);
  if (signal.aborted) throw aborted();
  return await new Promise<T>((resolve, reject) => {
    const onAbort = () => reject(aborted());
    signal.addEventListener("abort", onAbort, { once: true });
    promise.then(
      (value) => { signal.removeEventListener("abort", onAbort); resolve(value); },
      (error) => { signal.removeEventListener("abort", onAbort); reject(error); },
    );
  });
}

async function raceRefUpdate(promise: Promise<void>, signal?: AbortSignal): Promise<void> {
  if (signal === undefined) return await promise;
  const outcome = promise.then(
    () => ({ kind: "fulfilled" as const }),
    (error: unknown) => ({ kind: "rejected" as const, error }),
  );
  let resolveAbort: ((value: { readonly kind: "aborted" }) => void) | undefined;
  const abortOutcome = new Promise<{ readonly kind: "aborted" }>((resolve) => { resolveAbort = resolve; });
  let abortQueued = false;
  const onAbort = () => {
    if (abortQueued) return;
    abortQueued = true;
    // Give an update that fulfilled in the same turn two promise-reaction
    // checkpoints to win before classifying a still-pending outcome.
    queueMicrotask(() => queueMicrotask(() => resolveAbort?.({ kind: "aborted" })));
  };
  signal.addEventListener("abort", onAbort, { once: true });
  if (signal.aborted) onAbort();
  const winner = await Promise.race([outcome, abortOutcome]);
  signal.removeEventListener("abort", onAbort);
  if (winner.kind === "fulfilled") return;
  if (winner.kind === "rejected" && !signal.aborted) throw winner.error;
  throw new GitHubPagesPublishError(
    "INDETERMINATE",
    "publication was cancelled after the ref update began and its outcome is unknown",
    winner.kind === "rejected" ? winner.error : undefined,
  );
}

async function abortableSleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
  if (milliseconds <= 0) return;
  await new Promise<void>((resolve, reject) => {
    if (signal?.aborted === true) { reject(aborted()); return; }
    const timer = setTimeout(() => { signal?.removeEventListener("abort", onAbort); resolve(); }, milliseconds);
    const onAbort = () => { clearTimeout(timer); reject(aborted()); };
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

function throwIfAborted(signal?: AbortSignal): void {
  if (signal?.aborted === true) throw aborted();
}

function aborted(): GitHubPagesPublishError {
  return new GitHubPagesPublishError("ABORTED", "publication was cancelled");
}

function ownershipInvalid(detail: string, cause?: unknown): never {
  throw new GitHubPagesPublishError("OWNERSHIP_INVALID", detail, cause);
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) ownershipInvalid(`${field} must be an object`);
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, allowed: readonly string[], field: string): void {
  const expected = [...allowed].sort(compareText);
  const actual = Object.keys(value).sort(compareText);
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    ownershipInvalid(`${field} must contain exactly ${expected.join(", ")}`);
  }
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) throw new TypeError(`${field} must be a non-empty string`);
  return value;
}

function nonNegativeSafeInteger(value: unknown, field: string, maximum: number): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0 || (value as number) > maximum) {
    throw new TypeError(`${field} must be a safe integer between 0 and ${maximum}`);
  }
  return value as number;
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
