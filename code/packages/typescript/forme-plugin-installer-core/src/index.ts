import { Buffer } from "node:buffer";
import { randomBytes } from "node:crypto";
import { constants as fsConstants, type BigIntStats } from "node:fs";
import {
  lstat,
  mkdir,
  open,
  opendir,
  realpath,
  rename,
  rm,
} from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { tryParseCapability, type Capability } from "@coding-adventures/forme-capability";
import {
  computeManifestHash,
  parseManifest,
  resolveCapabilityTemplate,
  validateManifest,
  verifyManifest,
  type CapabilityEntry,
  type Manifest,
  type TemplateEnv,
} from "@coding-adventures/forme-manifest";
import {
  formatGrantsFile,
  formatTrustStore,
  parseTrustStore,
  type PluginGrantDecision,
  type PluginTrustStore,
} from "@coding-adventures/forme-plugin-host";

const PATH_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=@-]+$/u;
const WINDOWS_RESERVED_RE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/iu;
const PROTOTYPE_SEGMENTS = new Set(["__proto__", "constructor", "prototype"]);
const TEXT_DECODER = new TextDecoder("utf-8", { fatal: true });
const PREPARED_SNAPSHOTS = new WeakSet<object>();
const PREPARED_FILES = new WeakMap<object, readonly PluginPackageFile[]>();

export const PLUGIN_INSTALL_LIMITS = Object.freeze({
  maxFileCount: 4_096,
  maxDirectoryCount: 4_096,
  maxTreeEntryCount: 8_193,
  maxDepth: 256,
  maxGrantCount: 4_096,
  maxFileSizeBytes: 16 * 1024 * 1024,
  maxTotalSizeBytes: 128 * 1024 * 1024,
  maxPathCharacters: 2_048,
  maxDestinationNameBytes: 200,
});

export interface PluginPackageFile {
  readonly path: string;
  readonly bytes: Uint8Array;
}

export type PluginTrustTier = "verified-third-party" | "unverified-third-party";

export interface PluginInstallPlatform {
  readonly os: string;
  readonly arch: string;
}

export interface PreparePluginInstallOptions {
  readonly installRoot: string;
  readonly files: readonly PluginPackageFile[];
  readonly trustStore: PluginTrustStore;
  readonly reviewedGrants: readonly PluginGrantDecision[];
  readonly capabilityEnvironment: Omit<TemplateEnv, "pluginDir">;
  readonly platform?: PluginInstallPlatform;
}

export interface PreparedPluginInstall {
  readonly pluginName: string;
  readonly pluginVersion: string;
  readonly manifestHash: string;
  readonly trustTier: PluginTrustTier;
  readonly destinationName: string;
  readonly destinationPath: string;
  readonly grantedCapabilities: readonly Capability[];
  readonly files: readonly PluginPackageFile[];
  readonly fileCount: number;
  readonly totalSizeBytes: number;
}

export function preparePluginInstallSnapshot(
  options: PreparePluginInstallOptions,
): PreparedPluginInstall {
  if (!isRecord(options)) throw new TypeError("install options must be an object");
  const installRoot = absolutePath(options.installRoot, "installRoot");
  if (!Array.isArray(options.files)) throw new TypeError("files must be an array");
  if (options.files.length > PLUGIN_INSTALL_LIMITS.maxFileCount) {
    throw new TypeError(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxFileCount}-file limit`);
  }

  const packageFiles = copyAndValidateFiles(options.files);
  const byPath = new Map(packageFiles.map(file => [file.path, file]));
  if (packageFiles.some(file => {
    const folded = file.path.toLowerCase();
    return folded === "grants.toml" || folded.startsWith("grants.toml/");
  })) {
    throw new TypeError("plugin packages must not supply the host-owned grants.toml file");
  }
  const manifestFile = byPath.get("plugin.toml");
  if (!manifestFile) throw new TypeError("plugin package must contain plugin.toml at its root");

  let manifest: Manifest;
  try {
    manifest = parseManifest(TEXT_DECODER.decode(manifestFile.bytes));
    validateManifest(manifest);
  } catch (cause) {
    throw new TypeError("plugin.toml failed validation", { cause });
  }

  const destinationName = `plugin-${Buffer.from(manifest.plugin.name, "utf8").toString("base64url")}`;
  if (Buffer.byteLength(destinationName, "utf8") > PLUGIN_INSTALL_LIMITS.maxDestinationNameBytes) {
    throw new TypeError("plugin name produces an installation basename that exceeds the filesystem-safe limit");
  }
  const destinationPath = join(installRoot, destinationName);
  const selectedEntry = canonicalManifestReference(runtimeEntry(manifest, options.platform), "runtime entry");
  const entryFile = byPath.get(selectedEntry);
  if (!entryFile) throw new TypeError(`plugin package is missing runtime entry ${JSON.stringify(selectedEntry)}`);
  validateReferencedSchemas(manifest, byPath);

  if (manifest.signature && !verifyManifest(manifest, entryFile.bytes)) {
    throw new TypeError("plugin signature verification failed");
  }
  // Formatting performs the same strict, bounded validation used by the host.
  const trustStore = parseTrustStore(formatTrustStore(options.trustStore));
  const trusted = manifest.signature !== undefined && trustStore.trustedKeys.some(key =>
    key.algorithm === manifest.signature!.algorithm && key.publicKey === manifest.signature!.publicKey
  );
  const trustTier: PluginTrustTier = trusted && packageFiles.length === 2
    ? "verified-third-party"
    : "unverified-third-party";

  const manifestHash = computeManifestHash(manifest, entryFile.bytes);
  const environment: TemplateEnv = {
    ...validateCapabilityEnvironment(options.capabilityEnvironment),
    pluginDir: destinationPath,
  };
  const required = manifest.capabilities.required.map(entry => resolveCapability(entry, environment));
  const optional = manifest.capabilities.optional.map(entry => resolveCapability(entry, environment));
  const declared = new Set([...required, ...optional]);
  if (!Array.isArray(options.reviewedGrants)) throw new TypeError("reviewedGrants must be an array");
  if (options.reviewedGrants.length > PLUGIN_INSTALL_LIMITS.maxGrantCount) {
    throw new TypeError(`reviewedGrants exceeds the ${PLUGIN_INSTALL_LIMITS.maxGrantCount}-decision limit`);
  }
  const reviewed = options.reviewedGrants.map(decision => cloneGrant(decision));
  const seenGrants = new Set<string>();
  for (const decision of reviewed) {
    if (seenGrants.has(decision.capability)) {
      throw new TypeError(`reviewed grants contain duplicate capability ${JSON.stringify(decision.capability)}`);
    }
    seenGrants.add(decision.capability);
    if (!declared.has(decision.capability)) {
      throw new TypeError(`reviewed capability ${JSON.stringify(decision.capability)} is not declared by the plugin`);
    }
  }
  for (const capability of required) {
    if (!seenGrants.has(capability)) {
      throw new TypeError(`required capability ${JSON.stringify(capability)} was not granted`);
    }
  }
  const grantsBytes = new TextEncoder().encode(formatGrantsFile({ manifestHash, granted: reviewed }));
  const outputFiles = [...packageFiles, Object.freeze({ path: "grants.toml", bytes: grantsBytes })]
    .sort((a, b) => compareText(a.path, b.path));
  const totalSizeBytes = outputFiles.reduce((sum, file) => sum + file.bytes.byteLength, 0);

  const privateFiles = Object.freeze(outputFiles.map(file => Object.freeze({
    path: file.path,
    bytes: Uint8Array.from(file.bytes),
  })));
  const prepared: PreparedPluginInstall = Object.freeze({
    pluginName: manifest.plugin.name,
    pluginVersion: manifest.plugin.version,
    manifestHash,
    trustTier,
    destinationName,
    destinationPath,
    grantedCapabilities: Object.freeze(reviewed.map(decision => decision.capability)),
    files: Object.freeze(outputFiles.map(file => Object.freeze({
      path: file.path,
      bytes: Uint8Array.from(file.bytes),
    }))),
    fileCount: outputFiles.length,
    totalSizeBytes,
  });
  PREPARED_SNAPSHOTS.add(prepared);
  PREPARED_FILES.set(prepared, privateFiles);
  return prepared;
}

export type PluginInstallErrorCode =
  | "ROOT_UNSAFE"
  | "TARGET_UNSAFE"
  | "TARGET_BUSY"
  | "TARGET_EXISTS"
  | "STAGING_FAILED"
  | "COMMIT_FAILED"
  | "ROLLBACK_FAILED"
  | "CLEANUP_FAILED"
  | "ABORTED"
  | "INVALID_SNAPSHOT";

export class PluginInstallError extends Error {
  readonly code: PluginInstallErrorCode;
  override readonly cause?: unknown;

  constructor(code: PluginInstallErrorCode, detail: string, cause?: unknown) {
    super(`${code}: ${detail}`);
    this.name = "PluginInstallError";
    this.code = code;
    this.cause = cause;
  }
}

export interface InstallPreparedPluginOptions {
  readonly prepared: PreparedPluginInstall;
  readonly mode?: "replace" | "immutable";
  readonly signal?: AbortSignal;
}

export interface PluginInstallResult {
  readonly status: "installed" | "updated" | "unchanged";
  readonly pluginName: string;
  readonly pluginVersion: string;
  readonly manifestHash: string;
  readonly destinationPath: string;
  readonly trustTier: PluginTrustTier;
  readonly grantedCapabilities: readonly Capability[];
  readonly fileCount: number;
  readonly totalSizeBytes: number;
}

interface PathIdentity {
  readonly dev: bigint;
  readonly ino: bigint;
}

interface OwnedDirectory {
  readonly path: string;
  readonly identity: PathIdentity;
  readonly ownerUid: bigint;
}

interface ExistingTree extends OwnedDirectory {
  readonly matches: boolean;
}

export async function installPreparedPlugin(
  options: InstallPreparedPluginOptions,
): Promise<PluginInstallResult> {
  if (!isRecord(options) || !isRecord(options.prepared) || !PREPARED_SNAPSHOTS.has(options.prepared)) {
    throw new PluginInstallError("INVALID_SNAPSHOT", "prepared must come from preparePluginInstallSnapshot");
  }
  if (options.mode !== undefined && options.mode !== "replace" && options.mode !== "immutable") {
    throw new PluginInstallError("INVALID_SNAPSHOT", "mode must be 'replace' or 'immutable'");
  }
  const prepared = options.prepared;
  throwIfAborted(options.signal);
  const root = await requireSafeRoot(dirname(prepared.destinationPath), prepared.destinationPath);
  const targetPath = join(root.path, prepared.destinationName);
  const lockPath = join(root.path, `.${prepared.destinationName}.forme-lock`);
  const lock = await acquireLock(root, lockPath);
  let stage: OwnedDirectory | undefined;
  let backup: OwnedDirectory | undefined;
  let backupTarget: string | undefined;
  let oldTargetMoved = false;
  let primaryFailure: unknown;
  let status: PluginInstallResult["status"] = "installed";
  try {
    throwIfAborted(options.signal);
    const existing = await inspectExistingTree(targetPath, prepared, root, options.signal);
    if (existing?.matches) {
      await requireRootStable(root);
      await requireOwnedDirectory(existing, "TARGET_UNSAFE");
      status = "unchanged";
      return resultFor(prepared, status, targetPath);
    }
    if (existing && options.mode === "immutable") {
      throw new PluginInstallError("TARGET_EXISTS", "an installed plugin already exists and differs");
    }
    status = existing ? "updated" : "installed";
    stage = await createOwnedDirectory(root.path, `.${prepared.destinationName}.forme-stage-`);
    try {
      await materializePreparedSnapshot(stage, prepared, options.signal);
      const staged = await inspectTree(stage.path, prepared, stage, options.signal, "STAGING_FAILED");
      /* v8 ignore next -- only an out-of-band mutation can alter the private stage between write and verification */
      if (!staged.matches) throw new PluginInstallError("STAGING_FAILED", "staged files did not match the prepared snapshot");
      await requireOwnedDirectory(stage, "STAGING_FAILED");
    } catch (cause) {
      if (cause instanceof PluginInstallError) throw cause;
      throw new PluginInstallError("STAGING_FAILED", message(cause), cause);
    }
    throwIfAborted(options.signal);
    await requireRootStable(root);
    await requireOwnedDirectory(lock, "COMMIT_FAILED");

    if (existing) {
      await requireOwnedDirectory(existing, "TARGET_UNSAFE");
      backup = await createOwnedDirectory(root.path, `.${prepared.destinationName}.forme-backup-`);
      backupTarget = join(backup.path, "root");
      await rename(targetPath, backupTarget);
      oldTargetMoved = true;
    }
    try {
      throwIfAborted(options.signal);
      await requireRootStable(root);
      await requireOwnedDirectory(lock, "COMMIT_FAILED");
      await requireOwnedDirectory(stage, "COMMIT_FAILED");
      await rename(stage.path, targetPath);
      stage = undefined;
    } catch (cause) {
      const primary = cause instanceof PluginInstallError
        ? cause
        : new PluginInstallError("COMMIT_FAILED", message(cause), cause);
      if (backup && backupTarget) {
        try {
          await requireRootStable(root);
          await requireOwnedDirectory(backup, "ROLLBACK_FAILED");
          await rename(backupTarget, targetPath);
          oldTargetMoved = false;
          backupTarget = undefined;
        } catch (rollbackCause) {
          throw new PluginInstallError(
            "ROLLBACK_FAILED",
            `commit failed (${primary.message}) and the old install could not be restored: ${message(rollbackCause)}`,
            rollbackCause,
          );
        }
      }
      throw primary;
    }
    if (backup) {
      await removeOwnedDirectory(backup, "CLEANUP_FAILED");
      backup = undefined;
      backupTarget = undefined;
    }
    return resultFor(prepared, status, targetPath);
  } catch (cause) {
    primaryFailure = cause;
    throw cause;
  } finally {
    let cleanupError: unknown;
    if (stage) {
      try { await removeOwnedDirectory(stage, "CLEANUP_FAILED"); } catch (cause) { cleanupError = cause; }
    }
    if (backup && !oldTargetMoved) {
      try { await removeOwnedDirectory(backup, "CLEANUP_FAILED"); } catch (cause) { cleanupError ??= cause; }
    }
    try { await removeOwnedDirectory(lock, "CLEANUP_FAILED"); } catch (cause) { cleanupError ??= cause; }
    if (cleanupError !== undefined) {
      /* v8 ignore next -- requires an injected cleanup failure concurrent with another primary failure */
      if (primaryFailure !== undefined) attachSecondary(primaryFailure, cleanupError);
      else throw cleanupError;
    }
  }
}

function resultFor(
  prepared: PreparedPluginInstall,
  status: PluginInstallResult["status"],
  canonicalDestinationPath: string,
): PluginInstallResult {
  return Object.freeze({
    status,
    pluginName: prepared.pluginName,
    pluginVersion: prepared.pluginVersion,
    manifestHash: prepared.manifestHash,
    destinationPath: canonicalDestinationPath,
    trustTier: prepared.trustTier,
    grantedCapabilities: Object.freeze([...prepared.grantedCapabilities]),
    fileCount: prepared.fileCount,
    totalSizeBytes: prepared.totalSizeBytes,
  });
}

async function requireSafeRoot(rootPath: string, destinationPath: string): Promise<OwnedDirectory> {
  try {
    const lexicalRoot = resolve(rootPath);
    /* v8 ignore next -- prepared snapshots derive destinationPath directly from this root */
    if (dirname(resolve(destinationPath)) !== lexicalRoot) {
      throw new Error("destination path is not an immediate child of the install root");
    }
    const info = await lstat(lexicalRoot, { bigint: true });
    if (info.isSymbolicLink() || !info.isDirectory()) throw new Error("install root is not a real directory");
    const canonicalRoot = await realpath(lexicalRoot);
    if (canonicalRoot !== lexicalRoot) throw new Error("install root must use its canonical path");
    const canonicalInfo = await lstat(canonicalRoot, { bigint: true });
    /* v8 ignore next -- concurrent ancestor replacement race, guarded fail-closed */
    if (!sameIdentity(identity(info), identity(canonicalInfo))) throw new Error("install root changed during resolution");
    if (typeof process.getuid === "function" && canonicalInfo.uid !== BigInt(process.getuid())) {
      throw new Error("install root is not owned by the current host user");
    }
    return { path: canonicalRoot, identity: identity(canonicalInfo), ownerUid: canonicalInfo.uid };
  } catch (cause) {
    throw new PluginInstallError("ROOT_UNSAFE", message(cause), cause);
  }
}

async function requireRootStable(root: OwnedDirectory): Promise<void> {
  try {
    const info = await lstat(root.path, { bigint: true });
    /* v8 ignore next -- concurrent install-root replacement race, guarded fail-closed */
    if (!info.isDirectory() || info.isSymbolicLink() || !sameIdentity(root.identity, identity(info))) {
      throw new Error("install root identity changed during installation");
    }
    /* v8 ignore next -- concurrent canonical-path replacement race, guarded fail-closed */
    if (await realpath(root.path) !== root.path) throw new Error("install root canonical path changed");
  } catch (cause) {
    throw new PluginInstallError("ROOT_UNSAFE", message(cause), cause);
  }
}

async function acquireLock(root: OwnedDirectory, path: string): Promise<OwnedDirectory> {
  await requireRootStable(root);
  try {
    await mkdir(path, { mode: 0o700 });
    return await ownedDirectory(path, "TARGET_BUSY", root.ownerUid);
  } catch (cause) {
    if (isErrno(cause, "EEXIST")) throw new PluginInstallError("TARGET_BUSY", "another installer holds the plugin lock", cause);
    /* v8 ignore next -- only a concurrent lock-path replacement can make ownedDirectory fail here */
    if (cause instanceof PluginInstallError) throw cause;
    throw new PluginInstallError("TARGET_BUSY", message(cause), cause);
  }
}

async function createOwnedDirectory(parent: string, prefix: string): Promise<OwnedDirectory> {
  for (let attempt = 0; attempt < 16; attempt += 1) {
    const path = join(parent, `${prefix}${randomBytes(12).toString("hex")}`);
    try {
      await mkdir(path, { mode: 0o700 });
      return await ownedDirectory(path, "STAGING_FAILED");
    } catch (cause) {
      /* v8 ignore next -- a cryptographic random-name collision is operationally unreachable */
      if (isErrno(cause, "EEXIST")) continue;
      throw cause;
    }
  }
  throw new PluginInstallError("STAGING_FAILED", "could not reserve a unique transaction directory");
}

async function ownedDirectory(path: string, code: PluginInstallErrorCode, expectedOwnerUid?: bigint): Promise<OwnedDirectory> {
  const info = await lstat(path, { bigint: true });
  /* v8 ignore next -- mkdir just created this private path; only an out-of-band replacement can violate it */
  if (info.isSymbolicLink() || !info.isDirectory() ||
      (expectedOwnerUid !== undefined && info.uid !== expectedOwnerUid)) {
    throw new PluginInstallError(code, `${path} is not an owned directory`);
  }
  return { path, identity: identity(info), ownerUid: info.uid };
}

async function requireOwnedDirectory(directory: OwnedDirectory, code: PluginInstallErrorCode): Promise<void> {
  try {
    const info = await lstat(directory.path, { bigint: true });
    /* v8 ignore next -- concurrent replacement of a private transaction directory */
    if (!info.isDirectory() || info.isSymbolicLink() || !sameIdentity(directory.identity, identity(info))) {
      throw new Error(`${directory.path} changed identity`);
    }
  } catch (cause) {
    throw new PluginInstallError(code, message(cause), cause);
  }
}

async function materializePreparedSnapshot(
  stage: OwnedDirectory,
  prepared: PreparedPluginInstall,
  signal?: AbortSignal,
): Promise<void> {
  const madeDirectories = new Set<string>();
  for (const file of privateFilesFor(prepared)) {
    throwIfAborted(signal);
    const parent = dirname(file.path);
    if (parent !== "." && !madeDirectories.has(parent)) {
      await mkdir(join(stage.path, parent), { recursive: true, mode: 0o700 });
      for (let cursor = parent; cursor !== "."; cursor = dirname(cursor)) madeDirectories.add(cursor);
    }
    const path = join(stage.path, file.path);
    /* v8 ignore next -- fallback is only used on platforms without O_NOFOLLOW */
    const noFollow = "O_NOFOLLOW" in fsConstants ? fsConstants.O_NOFOLLOW : 0;
    const handle = await open(path, fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_EXCL | noFollow, 0o600);
    try {
      await handle.writeFile(file.bytes);
      await handle.sync();
      await handle.chmod(0o400);
    } finally {
      await handle.close();
    }
  }
}

async function inspectExistingTree(
  targetPath: string,
  prepared: PreparedPluginInstall,
  owner: OwnedDirectory,
  signal?: AbortSignal,
): Promise<ExistingTree | undefined> {
  let rootInfo;
  try {
    rootInfo = await lstat(targetPath, { bigint: true });
  } catch (cause) {
    if (isErrno(cause, "ENOENT")) return undefined;
    /* v8 ignore next -- requires an injected metadata failure after the cooperative lock is held */
    throw new PluginInstallError("TARGET_UNSAFE", message(cause), cause);
  }
  if (rootInfo.isSymbolicLink() || !rootInfo.isDirectory() ||
      rootInfo.uid !== owner.ownerUid || rootInfo.dev !== owner.identity.dev) {
    throw new PluginInstallError("TARGET_UNSAFE", "existing plugin target is not a real host-owned directory");
  }
  return inspectTree(targetPath, prepared, owner, signal, "TARGET_UNSAFE", identity(rootInfo));
}

async function inspectTree(
  rootPath: string,
  prepared: PreparedPluginInstall,
  owner: OwnedDirectory,
  signal: AbortSignal | undefined,
  code: PluginInstallErrorCode,
  knownIdentity?: PathIdentity,
): Promise<ExistingTree> {
  const expected = new Map(privateFilesFor(prepared).map(file => [file.path, file.bytes]));
  const seen = new Set<string>();
  const stack: Array<{ path: string; relative: string; depth: number }> = [{ path: rootPath, relative: "", depth: 0 }];
  let directoryCount = 0;
  let entryCount = 0;
  let matches = true;
  try {
    while (stack.length > 0) {
      throwIfAborted(signal);
      const current = stack.pop()!;
      if (current.depth > PLUGIN_INSTALL_LIMITS.maxDepth) throw new Error("installed tree exceeds the maximum depth");
      const directory = await opendir(current.path);
      for await (const entry of directory) {
        throwIfAborted(signal);
        entryCount += 1;
        if (entryCount > PLUGIN_INSTALL_LIMITS.maxTreeEntryCount) {
          throw new Error("installed tree exceeds the total entry limit");
        }
        const relativePath = current.relative ? `${current.relative}/${entry.name}` : entry.name;
        validatePackagePath(relativePath, "installed path");
        const fullPath = join(current.path, entry.name);
        const info = await lstat(fullPath, { bigint: true });
        if (info.uid !== owner.ownerUid || info.dev !== owner.identity.dev) {
          throw new Error(`installed path ${JSON.stringify(relativePath)} is not host-owned on the install filesystem`);
        }
        if (info.isSymbolicLink()) throw new Error(`installed path ${JSON.stringify(relativePath)} is a symbolic link`);
        if (info.isDirectory()) {
          directoryCount += 1;
          if (directoryCount > PLUGIN_INSTALL_LIMITS.maxDirectoryCount) {
            throw new Error("installed tree exceeds the directory limit");
          }
          stack.push({ path: fullPath, relative: relativePath, depth: current.depth + 1 });
          continue;
        }
        if (!info.isFile() || info.nlink !== 1n) {
          throw new Error(`installed path ${JSON.stringify(relativePath)} is not an exclusive regular file`);
        }
        if (seen.size >= PLUGIN_INSTALL_LIMITS.maxFileCount + 1) throw new Error("installed tree exceeds the file limit");
        seen.add(relativePath);
        const bytes = expected.get(relativePath);
        if (!bytes || info.size !== BigInt(bytes.byteLength)) {
          matches = false;
          continue;
        }
        if (!await regularFileEquals(fullPath, info, bytes)) matches = false;
      }
    }
    if (seen.size !== expected.size || [...expected.keys()].some(path => !seen.has(path))) matches = false;
    const finalRoot = await lstat(rootPath, { bigint: true });
    const rootIdentity = knownIdentity ?? identity(finalRoot);
    /* v8 ignore next -- concurrent target-root replacement during the bounded scan */
    if (!finalRoot.isDirectory() || finalRoot.isSymbolicLink() || finalRoot.uid !== owner.ownerUid ||
        finalRoot.dev !== owner.identity.dev || !sameIdentity(rootIdentity, identity(finalRoot))) {
      throw new Error("installed tree root changed during inspection");
    }
    return { path: rootPath, identity: rootIdentity, ownerUid: finalRoot.uid, matches };
  } catch (cause) {
    if (cause instanceof PluginInstallError && cause.code === "ABORTED") throw cause;
    throw new PluginInstallError(code, message(cause), cause);
  }
}

async function regularFileEquals(path: string, before: BigIntStats, expected: Uint8Array): Promise<boolean> {
  /* v8 ignore next -- fallback is only used on platforms without O_NOFOLLOW */
  const noFollow = "O_NOFOLLOW" in fsConstants ? fsConstants.O_NOFOLLOW : 0;
  const handle = await open(path, fsConstants.O_RDONLY | noFollow);
  try {
    const opened = await handle.stat({ bigint: true });
    /* v8 ignore next -- concurrent replacement between lstat and no-follow open */
    if (!opened.isFile() || opened.nlink !== 1n || !sameIdentity(identity(before), identity(opened))) {
      throw new Error(`${path} changed during open`);
    }
    const bytes = await handle.readFile();
    const after = await handle.stat({ bigint: true });
    /* v8 ignore next -- concurrent mutation during a single bounded file read */
    if (!sameIdentity(identity(opened), identity(after)) || opened.size !== after.size ||
        opened.mtimeNs !== after.mtimeNs || opened.ctimeNs !== after.ctimeNs) {
      throw new Error(`${path} changed during read`);
    }
    return bytes.equals(Buffer.from(expected));
  } finally {
    await handle.close();
  }
}

async function removeOwnedDirectory(directory: OwnedDirectory, code: PluginInstallErrorCode): Promise<void> {
  await requireOwnedDirectory(directory, code);
  try {
    await rm(directory.path, { recursive: true, force: false });
  } catch (cause) {
    throw new PluginInstallError(code, message(cause), cause);
  }
}

function identity(info: { readonly dev: bigint; readonly ino: bigint }): PathIdentity {
  return { dev: info.dev, ino: info.ino };
}

function sameIdentity(left: PathIdentity, right: PathIdentity): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function throwIfAborted(signal?: AbortSignal): void {
  if (signal?.aborted) throw new PluginInstallError("ABORTED", "plugin installation was cancelled", signal.reason);
}

function isErrno(error: unknown, code: string): boolean {
  return isRecord(error) && error.code === code;
}

function message(error: unknown): string {
  /* v8 ignore next -- Node filesystem APIs and all internal callers throw Error instances */
  return error instanceof Error ? error.message : String(error);
}

function privateFilesFor(prepared: PreparedPluginInstall): readonly PluginPackageFile[] {
  const files = PREPARED_FILES.get(prepared);
  /* v8 ignore next -- the WeakMap is populated atomically with the WeakSet membership */
  if (files === undefined) throw new PluginInstallError("INVALID_SNAPSHOT", "prepared snapshot bytes are unavailable");
  return files;
}

function attachSecondary(primary: unknown, secondary: unknown): void {
  /* v8 ignore next -- only filesystem cleanup failure injection reaches this diagnostic path */
  if (!(primary instanceof Error)) return;
  const record = primary as Error & { secondaryErrors?: unknown[] };
  record.secondaryErrors = [...(record.secondaryErrors ?? []), secondary];
}

function copyAndValidateFiles(files: readonly PluginPackageFile[]): PluginPackageFile[] {
  const result: PluginPackageFile[] = [];
  const exact = new Set<string>();
  const portable = new Map<string, string>();
  const directories = new Set<string>();
  let total = 0;
  for (const [index, value] of files.entries()) {
    if (!isRecord(value)) throw new TypeError(`files[${index}] must be an object`);
    const path = validatePackagePath(value.path, `files[${index}].path`);
    const segments = path.split("/");
    if (segments.length - 1 > PLUGIN_INSTALL_LIMITS.maxDepth) {
      throw new TypeError(`files[${index}].path exceeds the maximum directory depth`);
    }
    let prefix = "";
    for (const segment of segments.slice(0, -1)) {
      prefix = prefix ? `${prefix}/${segment}` : segment;
      directories.add(prefix);
      if (directories.size > PLUGIN_INSTALL_LIMITS.maxDirectoryCount) {
        throw new TypeError(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxDirectoryCount}-directory limit`);
      }
    }
    if (!(value.bytes instanceof Uint8Array)) throw new TypeError(`files[${index}].bytes must be a Uint8Array`);
    if (value.bytes.byteLength > PLUGIN_INSTALL_LIMITS.maxFileSizeBytes) {
      throw new TypeError(`files[${index}] exceeds the per-file byte limit`);
    }
    total += value.bytes.byteLength;
    if (total > PLUGIN_INSTALL_LIMITS.maxTotalSizeBytes) {
      throw new TypeError(`plugin package exceeds the total byte limit`);
    }
    if (exact.has(path)) throw new TypeError(`plugin package contains duplicate path ${JSON.stringify(path)}`);
    exact.add(path);
    const folded = path.toLowerCase();
    const conflicting = portable.get(folded);
    if (conflicting !== undefined) {
      throw new TypeError(`plugin package path ${JSON.stringify(path)} collides with ${JSON.stringify(conflicting)}`);
    }
    portable.set(folded, path);
    result.push(Object.freeze({ path, bytes: Uint8Array.from(value.bytes) }));
  }
  rejectPrefixCollisions([...exact].sort(compareText));
  return result;
}

function validatePackagePath(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) throw new TypeError(`${field} must be a non-empty string`);
  if (value.length > PLUGIN_INSTALL_LIMITS.maxPathCharacters) throw new TypeError(`${field} is too long`);
  if (value !== value.normalize("NFC")) throw new TypeError(`${field} must use NFC Unicode normalization`);
  if (value.startsWith("/") || value.startsWith("\\") || /^[A-Za-z]:/u.test(value) || value.includes("\\")) {
    throw new TypeError(`${field} must be a portable relative path using '/' separators`);
  }
  for (const segment of value.split("/")) {
    if (segment.length === 0 || segment === "." || segment === "..") {
      throw new TypeError(`${field} must not contain empty, '.' or '..' segments`);
    }
    if (!PATH_SEGMENT_RE.test(segment)) throw new TypeError(`${field} contains a disallowed or control character`);
    if (Buffer.byteLength(segment, "utf8") > 255) throw new TypeError(`${field} contains a segment longer than 255 bytes`);
    if (segment.endsWith(".") || segment.endsWith(" ")) throw new TypeError(`${field} contains an unsafe suffix`);
    if (WINDOWS_RESERVED_RE.test(segment)) throw new TypeError(`${field} contains a Windows reserved device name`);
    if (PROTOTYPE_SEGMENTS.has(segment)) throw new TypeError(`${field} contains a prototype-pollution sink name`);
  }
  return value;
}

function rejectPrefixCollisions(paths: readonly string[]): void {
  for (let index = 1; index < paths.length; index += 1) {
    const previous = paths[index - 1]!;
    const current = paths[index]!;
    if (current.startsWith(`${previous}/`)) {
      throw new TypeError(`plugin package path prefix collision between ${JSON.stringify(previous)} and ${JSON.stringify(current)}`);
    }
  }
}

function runtimeEntry(manifest: Manifest, platform?: PluginInstallPlatform): string {
  if (manifest.runtime.kind !== "binary") return manifest.runtime.entry;
  const os = platformName(platform?.os ?? process.platform);
  const arch = architectureName(platform?.arch ?? process.arch);
  const key = `${os}-${arch}`;
  const entry = manifest.runtime.platforms?.[key];
  if (!entry) throw new TypeError(`binary plugin has no entry for ${key}`);
  return entry;
}

function platformName(value: string): string {
  return value === "win32" ? "windows" : value;
}

function architectureName(value: string): string {
  if (value === "x64") return "x86_64";
  if (value === "arm64") return "aarch64";
  return value;
}

function validateReferencedSchemas(manifest: Manifest, files: ReadonlyMap<string, PluginPackageFile>): void {
  const schemaPaths = [
    ...manifest.contributes.stages.map(stage => stage.configSchema),
    ...manifest.contributes.kinds.map(kind => kind.schema),
  ].filter((path): path is string => path !== undefined);
  for (const path of schemaPaths) {
    const canonicalPath = canonicalManifestReference(path, "schema path");
    if (!files.has(canonicalPath)) throw new TypeError(`plugin package is missing schema ${JSON.stringify(path)}`);
  }
  if (manifest.signature && schemaPaths.length > 0) {
    throw new TypeError("signed plugins cannot reference schemas until package signatures bind auxiliary files");
  }
}

function canonicalManifestReference(path: string, field: string): string {
  const canonical = path.startsWith("./") ? path.slice(2) : path;
  return validatePackagePath(canonical, field);
}

function resolveCapability(entry: CapabilityEntry, environment: TemplateEnv): Capability {
  const template = entry.detail
    ? `${entry.realm}:${entry.scope}:${entry.detail}`
    : `${entry.realm}:${entry.scope}`;
  const resolved = resolveCapabilityTemplate(template, environment);
  if (!tryParseCapability(resolved)) throw new TypeError(`manifest declares invalid capability ${JSON.stringify(resolved)}`);
  return resolved;
}

function validateCapabilityEnvironment(value: unknown): Omit<TemplateEnv, "pluginDir"> {
  if (!isRecord(value)) throw new TypeError("capabilityEnvironment must be an object");
  const storageRoot = absolutePath(value.storageRoot, "capabilityEnvironment.storageRoot");
  const cacheDir = value.cacheDir === null
    ? null
    : absolutePath(value.cacheDir, "capabilityEnvironment.cacheDir");
  return { storageRoot, cacheDir };
}

function cloneGrant(value: unknown): PluginGrantDecision {
  if (!isRecord(value)) throw new TypeError("reviewed grant must be an object");
  if (typeof value.capability !== "string" || !tryParseCapability(value.capability)) {
    throw new TypeError("reviewed grant capability is invalid");
  }
  if (typeof value.grantedAt !== "string") throw new TypeError("reviewed grant grantedAt must be a string");
  if (value.note !== undefined && typeof value.note !== "string") throw new TypeError("reviewed grant note must be a string");
  return Object.freeze({
    capability: value.capability,
    grantedAt: value.grantedAt,
    ...(value.note === undefined ? {} : { note: value.note }),
  });
}

function absolutePath(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0 || !isAbsolute(value) || value.includes("\0")) {
    throw new TypeError(`${field} must be a non-empty absolute path`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function compareText(left: string, right: string): number {
  /* v8 ignore next -- duplicate package paths are rejected before sorting */
  return left < right ? -1 : left > right ? 1 : 0;
}
