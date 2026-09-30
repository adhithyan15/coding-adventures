import { createHash } from "node:crypto";
import { createReadStream, fstatSync, type BigIntStats } from "node:fs";
import { constants, lstat, mkdir, open, opendir, realpath } from "node:fs/promises";
import type { FileHandle } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import {
  canonicalDeployManifest,
  contentDigestToStoreKey,
  createDeployPlan,
  createDryRunReport,
  DEPLOY_LIMITS,
  parseDeployManifest,
  serializeDeployReport,
  type ContentStore,
  type DeployManifest,
  type DeployReport,
  type DeployReportFile,
} from "@coding-adventures/forme-deploy-runner-core";
import {
  inspectFilesystemSite,
  publishFilesystemSite,
} from "@coding-adventures/forme-deploy-runner-fs-adapter";
import {
  bootstrapGitHubPagesOwnership,
  createGitHubRestBoundary,
  createGitHubRestReadBoundary,
  inspectGitHubPagesSite,
  publishGitHubPagesSite,
  type GitHubPagesBootstrapExpectation,
} from "@coding-adventures/forme-deploy-runner-github-pages-adapter";

const BUNDLE_MAGIC = Buffer.from("FORME-CONTENT-BUNDLE-V1\n", "ascii");
const MAX_CONFIG_BYTES = 1024 * 1024;
const MAX_INLINE_BYTES = 128 * 1024 * 1024;

interface DistTreeArtifact {
  readonly variant: { readonly kind: "dist-tree" };
  readonly files: Readonly<Record<string, Uint8Array>>;
}

export interface DeployInvocation {
  readonly cwd: string;
  readonly manifestPath: string;
  readonly content:
    | { readonly kind: "directory"; readonly path: string }
    | { readonly kind: "bundle"; readonly path: string }
    | { readonly kind: "inline"; readonly fd: number };
  readonly target: "fs" | "github-pages";
  readonly targetConfigPath: string;
  readonly bootstrapOwnershipPath?: string;
  readonly previousPath?: string;
  readonly dryRun: boolean;
  readonly retryLimit: number;
  readonly signal?: AbortSignal;
}

export async function materializeDeployInput(
  outputs: Readonly<Record<string, unknown>>,
  requestedDirectory: string,
  projectRoot: string,
): Promise<{ readonly manifestPath: string; readonly contentDirectory: string }> {
  const target = containedPath(projectRoot, requestedDirectory, "deploy input directory");
  const artifacts = distTreeArtifacts(outputs);
  if (artifacts.length === 0) throw new TypeError("--deploy-input requires at least one dist-tree output");
  const files = new Map<string, Uint8Array>();
  for (const [outputName, artifact] of artifacts) {
    for (const [path, value] of Object.entries(artifact.files)) {
      if (!(value instanceof Uint8Array)) {
        throw new TypeError(`deploy output ${JSON.stringify(outputName)} file ${JSON.stringify(path)} is not bytes`);
      }
      if (files.has(path)) throw new TypeError(`deploy outputs collide at ${JSON.stringify(path)}`);
      files.set(path, new Uint8Array(value));
    }
  }
  const parent = dirname(target);
  const ancestry = await ensureRealDirectory(rootPath(projectRoot), parent);
  const manifestFiles: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
  let totalSizeBytes = 0;
  const digestBytes = new Map<string, Uint8Array>();
  for (const [outputPath, bytes] of [...files.entries()].sort(([a], [b]) => compareText(a, b))) {
    const sha256 = createHash("sha256").update(bytes).digest("base64");
    totalSizeBytes += bytes.byteLength;
    manifestFiles[outputPath] = {
      outputPath,
      contentType: contentType(outputPath),
      sizeBytes: bytes.byteLength,
      sha256,
      source: "extra",
    };
    if (!digestBytes.has(sha256)) digestBytes.set(sha256, bytes);
  }
  const manifest = parseDeployManifest({
    version: 1,
    fileCount: files.size,
    totalSizeBytes,
    files: manifestFiles,
  });
  const inputFiles = new Map<string, Uint8Array>([[
    "deploy-manifest.json",
    Buffer.from(canonicalDeployManifest(manifest), "utf8"),
  ]]);
  for (const [sha256, bytes] of digestBytes) {
    inputFiles.set(`content/${contentDigestToStoreKey(sha256)}.bin`, bytes);
  }
  const packaged = manifestForBytes(inputFiles);
  const packagedContent = new Map<string, Uint8Array>();
  for (const [path, entry] of Object.entries(packaged.files)) {
    const bytes = inputFiles.get(path);
    if (bytes !== undefined && !packagedContent.has(entry.sha256)) packagedContent.set(entry.sha256, bytes);
  }
  const parentIdentity = ancestry.at(-1);
  if (parentIdentity === undefined) throw new TypeError("deploy input parent identity is unavailable");
  await revalidateDirectoryChain(ancestry);
  await publishFilesystemSite({
    root: target,
    manifest: packaged,
    contentStore: mapContentStore(packagedContent),
    expectedParentIdentity: {
      canonicalPath: await realpath(parentIdentity.path),
      dev: parentIdentity.dev,
      ino: parentIdentity.ino,
    },
  });
  await revalidateDirectoryChain(ancestry);
  return Object.freeze({
    manifestPath: join(target, "deploy-manifest.json"),
    contentDirectory: join(target, "content"),
  });
}

export async function executeDeploy(invocation: DeployInvocation): Promise<string> {
  if (!Number.isSafeInteger(invocation.retryLimit) || invocation.retryLimit < 0 || invocation.retryLimit > 10) {
    throw new TypeError("--retry must be an integer from 0 through 10");
  }
  const manifest = parseDeployManifest(parseJsonNoDuplicateKeys(
    await readBoundedUtf8(resolveInput(invocation.cwd, invocation.manifestPath), DEPLOY_LIMITS.maxManifestCharacters),
  ));
  const previous = invocation.previousPath === undefined
    ? undefined
    : parseDeployManifest(parseJsonNoDuplicateKeys(
      await readBoundedUtf8(resolveInput(invocation.cwd, invocation.previousPath), DEPLOY_LIMITS.maxManifestCharacters),
    ));
  const store = await openContentStore(invocation.content, invocation.cwd, manifest, invocation.signal);
  try {
    const plan = createDeployPlan(manifest, previous);
    const targetConfig = await readBoundedJson(resolveInput(invocation.cwd, invocation.targetConfigPath));
    if (invocation.target === "fs") {
      const config = filesystemTargetConfig(targetConfig, invocation.cwd);
      if (invocation.bootstrapOwnershipPath !== undefined) {
        throw new TypeError("--bootstrap-ownership is supported only for github-pages");
      }
      if (invocation.dryRun) {
        await inspectFilesystemSite({ root: config.root, manifest, contentStore: store, signal: invocation.signal });
        return serializeDeployReport(createDryRunReport(manifest, plan, "fs"));
      }
      await publishFilesystemSite({ root: config.root, manifest, contentStore: store, signal: invocation.signal });
      return serializeDeployReport(successReport(manifest, plan, "fs"));
    }

    const config = githubTargetConfig(targetConfig);
    const common = {
      manifest,
      contentStore: store,
      owner: config.owner,
      repository: config.repository,
      ref: config.ref,
      deploymentOwner: config.deploymentOwner,
      destination: config.destination,
      retryLimit: invocation.retryLimit,
      ...(invocation.signal === undefined ? {} : { signal: invocation.signal }),
    };
    const bootstrap = invocation.bootstrapOwnershipPath === undefined
      ? undefined
      : await readBoundedJson(resolveInput(invocation.cwd, invocation.bootstrapOwnershipPath)) as GitHubPagesBootstrapExpectation;
    if (invocation.dryRun) {
      const token = process.env.GITHUB_TOKEN;
      const boundary = createGitHubRestReadBoundary(token === undefined ? {} : { token });
      if (bootstrap !== undefined) assertBootstrapExpectation(bootstrap, config);
      await inspectGitHubPagesSite({
        ...common,
        boundary,
        ...(bootstrap === undefined ? {} : { bootstrapExpectation: bootstrap }),
      });
      return serializeDeployReport(createDryRunReport(manifest, plan, "github-pages"));
    }
    if (config.tokenEnv !== "GITHUB_TOKEN") throw new TypeError("github-pages tokenEnv must be exactly GITHUB_TOKEN");
    const token = process.env.GITHUB_TOKEN;
    if (token === undefined) throw new TypeError("GITHUB_TOKEN is required for github-pages publication");
    const boundary = createGitHubRestBoundary({ token });
    if (bootstrap !== undefined) {
      assertBootstrapExpectation(bootstrap, config);
      await bootstrapGitHubPagesOwnership({
        boundary,
        owner: config.owner,
        repository: config.repository,
        ref: config.ref,
        deploymentOwner: config.deploymentOwner,
        destination: config.destination,
        expectation: bootstrap,
        retryLimit: invocation.retryLimit,
        ...(invocation.signal === undefined ? {} : { signal: invocation.signal }),
      });
    }
    await publishGitHubPagesSite({ ...common, boundary });
    return serializeDeployReport(successReport(manifest, plan, "github-pages"));
  } finally {
    await closeContentStore(store);
  }
}

async function openContentStore(
  selection: DeployInvocation["content"],
  cwd: string,
  manifest: DeployManifest,
  signal?: AbortSignal,
): Promise<ContentStore> {
  if (selection.kind === "directory") return await directoryStore(resolveInput(cwd, selection.path), manifest);
  if (selection.kind === "bundle") return await bundleStore(resolveInput(cwd, selection.path), manifest);
  return await inlineStore(selection.fd, manifest, signal);
}

async function directoryStore(root: string, manifest: DeployManifest): Promise<ContentStore> {
  const rootStat = await lstat(root, { bigint: true });
  if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) throw new TypeError("content directory must be a real directory");
  const values = new Map<string, { readonly path: string; readonly length: number }>();
  const digestLengths = new Map<string, number>();
  for (const entry of Object.values(manifest.files)) {
    if (!digestLengths.has(entry.sha256)) digestLengths.set(entry.sha256, entry.sizeBytes);
  }
  for (const [digest, length] of [...digestLengths.entries()].sort(([a], [b]) => compareText(a, b))) {
    values.set(digest, Object.freeze({
      path: join(root, `${contentDigestToStoreKey(digest)}.bin`),
      length,
    }));
  }
  const expectedNames = new Set([...values.values()].map(({ path }) => basename(path)));
  const directory = await opendir(root);
  let actualCount = 0;
  for await (const entry of directory) {
    actualCount += 1;
    if (actualCount > values.size || !expectedNames.delete(entry.name)) {
      throw new TypeError("content directory entries must exactly match the manifest digest set");
    }
  }
  if (expectedNames.size !== 0) {
    throw new TypeError("content directory entries must exactly match the manifest digest set");
  }
  return Object.freeze({
    has: async (digest: string) => values.has(digest),
    get: async (digest: string) => {
      const entry = values.get(digest);
      if (entry === undefined) throw new Error("content digest is not present in the manifest-bound directory store");
      const beforeOpen = await lstat(entry.path, { bigint: true });
      if (!beforeOpen.isFile() || beforeOpen.isSymbolicLink() || beforeOpen.nlink !== 1n) {
        throw new TypeError("content entry must be one unlinked regular file");
      }
      const handle = await open(entry.path, constants.O_RDONLY | noFollow());
      try {
        const stat = await handle.stat({ bigint: true });
        if (!stat.isFile() || stat.nlink !== 1n) throw new TypeError("content entry must be one unlinked regular file");
        if (stat.dev !== beforeOpen.dev || stat.ino !== beforeOpen.ino) {
          throw new TypeError("content entry changed while it was opened");
        }
        if (stat.size !== BigInt(entry.length)) throw new TypeError("content entry size does not match the manifest");
        const bytes = await readUpTo(handle, entry.length);
        const after = await handle.stat({ bigint: true });
        if (after.dev !== stat.dev || after.ino !== stat.ino || after.size !== stat.size || after.mtimeNs !== stat.mtimeNs || after.ctimeNs !== stat.ctimeNs) {
          throw new TypeError("content entry changed while it was read");
        }
        return bytes;
      } finally { await handle.close(); }
    },
    hashes: async function* () { yield* [...values.keys()].sort(compareText); },
  });
}

interface ClosableContentStore extends ContentStore { close?: () => Promise<void> }

async function bundleStore(path: string, manifest: DeployManifest): Promise<ClosableContentStore> {
  const beforeOpen = await lstat(path, { bigint: true });
  if (!beforeOpen.isFile() || beforeOpen.isSymbolicLink() || beforeOpen.nlink !== 1n) {
    throw new TypeError("content bundle must be one unlinked regular file");
  }
  const handle = await open(path, constants.O_RDONLY | noFollow());
  try {
    const identity = await handle.stat({ bigint: true });
    if (!identity.isFile() || identity.nlink !== 1n) throw new TypeError("content bundle must be one unlinked regular file");
    if (identity.dev !== beforeOpen.dev || identity.ino !== beforeOpen.ino) {
      throw new TypeError("content bundle changed while it was opened");
    }
    let offset = 0;
    const magic = await readExact(handle, offset, BUNDLE_MAGIC.byteLength);
    if (!magic.equals(BUNDLE_MAGIC)) throw new TypeError("content bundle has invalid magic");
    offset += BUNDLE_MAGIC.byteLength;
    const countBytes = await readExact(handle, offset, 4);
    offset += 4;
    const count = countBytes.readUInt32BE(0);
    const required = bundleDigests(manifest);
    if (count !== required.length) throw new TypeError("content bundle record count does not match the manifest");
    const entries = new Map<string, { readonly offset: number; readonly length: number }>();
    let previous: Buffer | undefined;
    for (let index = 0; index < count; index += 1) {
      const header = await readExact(handle, offset, 40);
      offset += 40;
      const rawDigest = header.subarray(0, 32);
      const digest = rawDigest.toString("base64");
      const lengthBig = header.readBigUInt64BE(32);
      if (lengthBig > BigInt(DEPLOY_LIMITS.maxFileSizeBytes)) throw new TypeError("content bundle record exceeds the file-size limit");
      const length = Number(lengthBig);
      if (previous !== undefined && Buffer.compare(previous, rawDigest) >= 0) throw new TypeError("content bundle digests must be unique and sorted");
      if (digest !== required[index]) throw new TypeError("content bundle digest set does not match the manifest");
      entries.set(digest, Object.freeze({ offset, length }));
      previous = Buffer.from(rawDigest);
      offset += length;
      if (!Number.isSafeInteger(offset) || offset > Number(identity.size)) throw new TypeError("content bundle is truncated or oversized");
    }
    if (BigInt(offset) !== identity.size) throw new TypeError("content bundle has trailing or truncated bytes");
    return Object.freeze({
      has: async (digest: string) => entries.has(digest),
      get: async (digest: string) => {
        const entry = entries.get(digest);
        if (entry === undefined) throw new Error("content digest is not present in the bundle");
        const current = await handle.stat({ bigint: true });
        if (current.dev !== identity.dev || current.ino !== identity.ino || current.size !== identity.size) {
          throw new TypeError("content bundle identity changed during deployment");
        }
        return new Uint8Array(await readExact(handle, entry.offset, entry.length));
      },
      hashes: async function* () { yield* required; },
      close: async () => { await handle.close(); },
    });
  } catch (error) {
    await handle.close().catch(() => undefined);
    throw error;
  }
}

async function inlineStore(fd: number, manifest: DeployManifest, signal?: AbortSignal): Promise<ContentStore> {
  if (!Number.isSafeInteger(fd) || fd < 3 || fd > 1024) throw new TypeError("--content-inline-fd must be an integer from 3 through 1024");
  const text = await readStreamBounded(fd, MAX_INLINE_BYTES, signal);
  const value = parseJsonNoDuplicateKeys(new TextDecoder("utf-8", { fatal: true }).decode(text));
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError("inline content must be a JSON object");
  const raw = value as Record<string, unknown>;
  const required = uniqueDigests(manifest);
  const actual = Object.keys(raw).sort(compareText);
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    throw new TypeError("inline content digest set must exactly match the manifest");
  }
  const values = new Map<string, Uint8Array>();
  for (const digest of required) {
    const encoded = raw[digest];
    if (typeof encoded !== "string" || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) {
      throw new TypeError("inline content values must be canonical base64");
    }
    const bytes = Uint8Array.from(Buffer.from(encoded, "base64"));
    if (Buffer.from(bytes).toString("base64") !== encoded) throw new TypeError("inline content values must be canonical base64");
    values.set(digest, bytes);
  }
  return Object.freeze({
    has: async (digest: string) => values.has(digest),
    get: async (digest: string) => {
      const bytes = values.get(digest);
      if (bytes === undefined) throw new Error("inline content digest is missing");
      return bytes.slice();
    },
    hashes: async function* () { yield* required; },
  });
}

async function closeContentStore(store: ContentStore): Promise<void> {
  const close = (store as ClosableContentStore).close;
  if (close !== undefined) await close();
}

function filesystemTargetConfig(value: unknown, cwd: string): { readonly root: string } {
  const config = exactRecord(value, ["root"], "filesystem target config");
  if (typeof config.root !== "string" || config.root.length === 0) throw new TypeError("filesystem target root is required");
  return Object.freeze({ root: resolveInput(cwd, config.root) });
}

interface GitHubTargetConfig {
  readonly owner: string;
  readonly repository: string;
  readonly ref: string;
  readonly deploymentOwner: string;
  readonly destination: string;
  readonly tokenEnv: "GITHUB_TOKEN";
}

function githubTargetConfig(value: unknown): GitHubTargetConfig {
  const config = exactRecord(value, ["owner", "repository", "ref", "deploymentOwner", "destination", "tokenEnv"], "github-pages target config");
  for (const key of ["owner", "repository", "ref", "deploymentOwner", "destination", "tokenEnv"] as const) {
    if (typeof config[key] !== "string") throw new TypeError(`github-pages ${key} must be a string`);
  }
  if (config.tokenEnv !== "GITHUB_TOKEN") throw new TypeError("github-pages tokenEnv must be exactly GITHUB_TOKEN");
  return Object.freeze(config as unknown as GitHubTargetConfig);
}

function assertBootstrapExpectation(
  expectation: GitHubPagesBootstrapExpectation,
  config: GitHubTargetConfig,
): void {
  for (const key of ["owner", "repository", "ref", "deploymentOwner", "destination"] as const) {
    if (expectation[key] !== config[key]) throw new TypeError(`bootstrap expectation ${key} does not match target config`);
  }
}

function successReport(manifest: DeployManifest, plan: ReturnType<typeof createDeployPlan>, target: string): DeployReport {
  const dry = createDryRunReport(manifest, plan, target);
  const files: Record<string, DeployReportFile> = Object.fromEntries(plan.entries.map(entry => [entry.outputPath, Object.freeze({
    action: entry.action,
    bytesWritten: entry.action === "create" || entry.action === "update" ? entry.current?.sizeBytes ?? 0 : 0,
    elapsedMs: 0,
  })]));
  const totalBytesWritten = Object.values(files).reduce((total, file) => total + file.bytesWritten, 0);
  return Object.freeze({
    ...dry,
    files: Object.freeze(files),
    summary: Object.freeze({ ...dry.summary, totalBytesWritten }),
  });
}

function distTreeArtifacts(outputs: Readonly<Record<string, unknown>>): readonly (readonly [string, DistTreeArtifact])[] {
  const result: Array<readonly [string, DistTreeArtifact]> = [];
  for (const name of Object.keys(outputs).sort(compareText)) {
    const values = Array.isArray(outputs[name]) ? outputs[name] : [outputs[name]];
    for (const [index, value] of values.entries()) {
      if (typeof value !== "object" || value === null) continue;
      const candidate = value as Partial<DistTreeArtifact>;
      if (candidate.variant?.kind !== "dist-tree" || typeof candidate.files !== "object" || candidate.files === null) continue;
      result.push([`${name}[${index}]`, candidate as DistTreeArtifact]);
    }
  }
  return Object.freeze(result);
}

function containedPath(root: string, value: string, field: string): string {
  const target = isAbsolute(value) ? resolve(value) : resolve(root, value);
  const rel = relative(resolve(root), target);
  if (rel === "" || rel === "." || rel === ".." || rel.startsWith("../") || rel.startsWith("..\\") || isAbsolute(rel)) {
    throw new TypeError(`${field} must be a child of the project root`);
  }
  return target;
}

function rootPath(root: string): string {
  return resolve(root);
}

interface DirectoryIdentity {
  readonly path: string;
  readonly dev: bigint;
  readonly ino: bigint;
}

async function ensureRealDirectory(root: string, target: string): Promise<readonly DirectoryIdentity[]> {
  const relativeTarget = relative(root, target);
  if (relativeTarget === ".." || relativeTarget.startsWith("../") || relativeTarget.startsWith("..\\") || isAbsolute(relativeTarget)) {
    throw new TypeError("deploy input parent must remain beneath the project root");
  }
  const segments = relativeTarget === "" ? [] : relativeTarget.split(/[\\/]/);
  let cursor = root;
  const identities: DirectoryIdentity[] = [];
  for (const segment of ["", ...segments]) {
    if (segment !== "") cursor = join(cursor, segment);
    let stat: BigIntStats;
    try {
      stat = await lstat(cursor, { bigint: true });
    } catch (error) {
      if (!isErrno(error, "ENOENT") || segment === "") throw error;
      await mkdir(cursor).catch((mkdirError: unknown) => {
        if (!isErrno(mkdirError, "EEXIST")) throw mkdirError;
      });
      stat = await lstat(cursor, { bigint: true });
    }
    if (!stat.isDirectory() || stat.isSymbolicLink()) {
      throw new TypeError("deploy input parent components must be real directories");
    }
    identities.push(Object.freeze({ path: cursor, dev: stat.dev, ino: stat.ino }));
  }
  return Object.freeze(identities);
}

async function revalidateDirectoryChain(identities: readonly DirectoryIdentity[]): Promise<void> {
  for (const identity of identities) {
    const stat = await lstat(identity.path, { bigint: true });
    if (!stat.isDirectory() || stat.isSymbolicLink() || stat.dev !== identity.dev || stat.ino !== identity.ino) {
      throw new TypeError("deploy input parent identity changed during publication");
    }
  }
}

function manifestForBytes(files: ReadonlyMap<string, Uint8Array>): DeployManifest {
  const entries: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
  let totalSizeBytes = 0;
  for (const [path, bytes] of [...files.entries()].sort(([a], [b]) => compareText(a, b))) {
    totalSizeBytes += bytes.byteLength;
    entries[path] = {
      outputPath: path,
      contentType: contentType(path),
      sizeBytes: bytes.byteLength,
      sha256: createHash("sha256").update(bytes).digest("base64"),
      source: "extra",
    };
  }
  return parseDeployManifest({ version: 1, fileCount: files.size, totalSizeBytes, files: entries });
}

function mapContentStore(values: ReadonlyMap<string, Uint8Array>): ContentStore {
  return Object.freeze({
    has: async (digest: string) => values.has(digest),
    get: async (digest: string) => {
      const value = values.get(digest);
      if (value === undefined) throw new Error("packaged content digest is missing");
      return value.slice();
    },
    hashes: async function* () { yield* [...values.keys()].sort(compareText); },
  });
}

function resolveInput(cwd: string, value: string): string {
  return isAbsolute(value) ? resolve(value) : resolve(cwd, value);
}

async function readBoundedJson(path: string): Promise<unknown> {
  return parseJsonNoDuplicateKeys(await readBoundedUtf8(path, MAX_CONFIG_BYTES));
}

async function readBoundedUtf8(path: string, maximum: number): Promise<string> {
  const beforeOpen = await lstat(path, { bigint: true });
  if (!beforeOpen.isFile() || beforeOpen.isSymbolicLink() || beforeOpen.nlink !== 1n || beforeOpen.size > BigInt(maximum)) {
    throw new TypeError(`${path} must be one bounded unlinked regular file`);
  }
  const handle = await open(path, constants.O_RDONLY | noFollow());
  try {
    const stat = await handle.stat({ bigint: true });
    if (!stat.isFile() || stat.nlink !== 1n || stat.size > BigInt(maximum)) {
      throw new TypeError(`${path} must be one bounded unlinked regular file`);
    }
    if (stat.dev !== beforeOpen.dev || stat.ino !== beforeOpen.ino) {
      throw new TypeError(`${path} changed while it was opened`);
    }
    const bytes = await readUpTo(handle, Number(stat.size));
    const after = await handle.stat({ bigint: true });
    if (after.dev !== stat.dev || after.ino !== stat.ino || after.size !== stat.size || after.mtimeNs !== stat.mtimeNs || after.ctimeNs !== stat.ctimeNs) {
      throw new TypeError(`${path} changed while it was read`);
    }
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } finally {
    await handle.close();
  }
}

async function readExact(handle: FileHandle, position: number, length: number): Promise<Buffer> {
  const buffer = Buffer.alloc(length);
  let offset = 0;
  while (offset < length) {
    const { bytesRead } = await handle.read(buffer, offset, length - offset, position + offset);
    if (bytesRead === 0) throw new TypeError("content bundle is truncated");
    offset += bytesRead;
  }
  return buffer;
}

async function readStreamBounded(fd: number, maximum: number, signal?: AbortSignal): Promise<Uint8Array> {
  if (signal?.aborted === true) throw new DOMException("deployment aborted", "AbortError");
  const stat = fstatSync(fd, { bigint: true });
  if (!stat.isFile()) throw new TypeError("inline content descriptor must be a regular file");
  const chunks: Buffer[] = [];
  let total = 0;
  const stream = createReadStream("", {
    fd,
    autoClose: false,
  });
  return await new Promise<Uint8Array>((resolveRead, rejectRead) => {
    let settled = false;
    const finish = (error?: unknown): void => {
      if (settled) return;
      settled = true;
      signal?.removeEventListener("abort", onAbort);
      if (error === undefined) resolveRead(new Uint8Array(Buffer.concat(chunks, total)));
      else rejectRead(error);
    };
    const onAbort = (): void => {
      stream.destroy();
      finish(new DOMException("deployment aborted", "AbortError"));
    };
    stream.on("data", (chunk: string | Buffer) => {
      if (settled) return;
      const bytes = Buffer.from(chunk);
      total += bytes.byteLength;
      if (total > maximum) {
        stream.destroy();
        finish(new TypeError(`inline content exceeds ${maximum} bytes`));
        return;
      }
      chunks.push(bytes);
    });
    stream.once("end", () => finish());
    stream.once("error", error => finish(error));
    signal?.addEventListener("abort", onAbort, { once: true });
    if (signal?.aborted === true) onAbort();
  });
}

async function readUpTo(handle: FileHandle, expected: number): Promise<Uint8Array> {
  const buffer = Buffer.alloc(expected + 1);
  let offset = 0;
  while (offset < buffer.byteLength) {
    const { bytesRead } = await handle.read(buffer, offset, buffer.byteLength - offset, offset);
    if (bytesRead === 0) break;
    offset += bytesRead;
  }
  if (offset !== expected) throw new TypeError("content entry length changed while it was read");
  return new Uint8Array(buffer.subarray(0, expected));
}

function exactRecord(value: unknown, keys: readonly string[], field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${field} must be an object`);
  const record = value as Record<string, unknown>;
  const actual = Object.keys(record).sort(compareText);
  const expected = [...keys].sort(compareText);
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new TypeError(`${field} must contain exactly ${expected.join(", ")}`);
  }
  return record;
}

function parseJsonNoDuplicateKeys(text: string): unknown {
  let index = 0;
  const whitespace = (): void => { while (/\s/.test(text[index] ?? "")) index += 1; };
  const stringToken = (): string => {
    if (text[index] !== '"') throw new TypeError("JSON string expected");
    const start = index;
    index += 1;
    for (;;) {
      const character = text[index];
      if (character === undefined) throw new TypeError("unterminated JSON string");
      if (character === '"') {
        index += 1;
        return JSON.parse(text.slice(start, index)) as string;
      }
      if (character === "\\") {
        index += 2;
      } else {
        if (character < " ") throw new TypeError("JSON string contains a control character");
        index += 1;
      }
    }
  };
  const value = (): void => {
    whitespace();
    const character = text[index];
    if (character === "{") {
      index += 1;
      whitespace();
      const keys = new Set<string>();
      if (text[index] === "}") { index += 1; return; }
      for (;;) {
        whitespace();
        const key = stringToken();
        if (keys.has(key)) throw new TypeError(`JSON object contains duplicate key ${JSON.stringify(key)}`);
        keys.add(key);
        whitespace();
        if (text[index] !== ":") throw new TypeError("JSON object key must be followed by ':'");
        index += 1;
        value();
        whitespace();
        if (text[index] === "}") { index += 1; return; }
        if (text[index] !== ",") throw new TypeError("JSON object entries must be comma-separated");
        index += 1;
      }
    }
    if (character === "[") {
      index += 1;
      whitespace();
      if (text[index] === "]") { index += 1; return; }
      for (;;) {
        value();
        whitespace();
        if (text[index] === "]") { index += 1; return; }
        if (text[index] !== ",") throw new TypeError("JSON array entries must be comma-separated");
        index += 1;
      }
    }
    if (character === '"') { stringToken(); return; }
    const start = index;
    while (index < text.length && !/[\s,}\]]/.test(text[index] ?? "")) index += 1;
    if (start === index) throw new TypeError("JSON value expected");
    JSON.parse(text.slice(start, index)) as unknown;
  };
  value();
  whitespace();
  if (index !== text.length) throw new TypeError("JSON has trailing data");
  return JSON.parse(text) as unknown;
}

function uniqueDigests(manifest: DeployManifest): string[] {
  return [...new Set(Object.values(manifest.files).map(entry => entry.sha256))].sort(compareText);
}

function bundleDigests(manifest: DeployManifest): string[] {
  return uniqueDigests(manifest).sort((left, right) => Buffer.compare(
    Buffer.from(left, "base64"),
    Buffer.from(right, "base64"),
  ));
}

function contentType(path: string): string {
  if (path.endsWith(".html")) return "text/html; charset=utf-8";
  if (path.endsWith(".css")) return "text/css; charset=utf-8";
  if (path.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (path.endsWith(".json")) return "application/json; charset=utf-8";
  if (path.endsWith(".svg")) return "image/svg+xml";
  if (path.endsWith(".png")) return "image/png";
  return "application/octet-stream";
}

function noFollow(): number {
  return typeof constants.O_NOFOLLOW === "number" ? constants.O_NOFOLLOW : 0;
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function isErrno(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && "code" in error
    && (error as { readonly code?: unknown }).code === code;
}
