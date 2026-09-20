import { createHash } from "node:crypto";
import { Buffer } from "node:buffer";
import {
  constants,
  lstat,
  mkdir,
  mkdtemp,
  open,
  opendir,
  realpath,
  rename,
  rm,
  rmdir,
  stat,
} from "node:fs/promises";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import {
  createVerifiedContentReader,
  DEPLOY_LIMITS,
  parseDeployManifest,
  type ContentStore,
  type DeployManifest,
  type VerifiedContentReader,
} from "@coding-adventures/forme-deploy-runner-core";

export type FilesystemPublishCode =
  | "ROOT_UNSAFE"
  | "TARGET_UNSAFE"
  | "TARGET_BUSY"
  | "STAGING_FAILED"
  | "STAGING_UNSAFE"
  | "COMMIT_FAILED"
  | "ROLLBACK_FAILED"
  | "CLEANUP_FAILED"
  | "ABORTED"
  | "INVALID_STATE";

export class FilesystemPublishError extends Error {
  readonly code: FilesystemPublishCode;
  override readonly cause?: unknown;

  constructor(code: FilesystemPublishCode, detail: string, cause?: unknown) {
    super(`${code}: ${detail}`);
    this.name = "FilesystemPublishError";
    this.code = code;
    this.cause = cause;
  }
}

export interface FilesystemPublishOptions {
  readonly root: string;
  readonly manifest: unknown;
  readonly contentStore: ContentStore;
  readonly signal?: AbortSignal;
  /** Optional lower resource ceilings for constrained hosts and tests. */
  readonly scanLimits?: Partial<FilesystemScanLimits>;
}

export interface FilesystemScanLimits {
  readonly maxEntries: number;
  readonly maxDepth: number;
  readonly maxMetadataBytes: number;
}

export const FILESYSTEM_SCAN_LIMITS: FilesystemScanLimits = Object.freeze({
  maxEntries: DEPLOY_LIMITS.maxFileCount * 2,
  maxDepth: 256,
  maxMetadataBytes: 64 * 1024 * 1024,
});

export interface FilesystemPublishResult {
  readonly status: "published" | "unchanged";
  readonly fileCount: number;
  readonly totalSizeBytes: number;
}

export type FilesystemPublicationState =
  | "prepared"
  | "committed"
  | "finalized"
  | "rolled-back";

export interface FilesystemPublication {
  readonly state: FilesystemPublicationState;
  readonly changed: boolean;
  readonly fileCount: number;
  readonly totalSizeBytes: number;
  readonly commit: () => Promise<void>;
  readonly finalize: () => Promise<void>;
  readonly rollback: () => Promise<void>;
}

interface RootLocation {
  readonly root: string;
  readonly parent: string;
  readonly name: string;
  readonly parentIdentity: PathIdentity;
}

interface PathIdentity {
  readonly dev: bigint;
  readonly ino: bigint;
}

interface OwnedDirectory {
  readonly path: string;
  readonly identity: PathIdentity;
}

interface TreeSnapshot {
  readonly canonicalRoot: string;
  readonly rootIdentity: PathIdentity;
  readonly files: ReadonlyMap<string, FileSnapshot>;
  readonly directories: ReadonlySet<string>;
}

interface FileSnapshot extends PathIdentity {
  readonly size: bigint;
  readonly mtimeNs: bigint;
  readonly ctimeNs: bigint;
}

export async function publishFilesystemSite(
  options: FilesystemPublishOptions,
): Promise<FilesystemPublishResult> {
  const transaction = await prepareFilesystemPublication(options);
  try {
    await transaction.commit();
  } catch (primaryError) {
    try {
      await transaction.rollback();
    } catch (rollbackError) {
      attachSecondary(primaryError, rollbackError);
    }
    throw primaryError;
  }
  await transaction.finalize();
  return Object.freeze({
    status: transaction.changed ? "published" : "unchanged",
    fileCount: transaction.fileCount,
    totalSizeBytes: transaction.totalSizeBytes,
  });
}

export async function prepareFilesystemPublication(
  options: FilesystemPublishOptions,
): Promise<FilesystemPublication> {
  const manifest = parseDeployManifest(options.manifest);
  throwIfAborted(options.signal);
  const scanLimits = resolveScanLimits(options.scanLimits);
  const location = await resolveRoot(options.root);
  const lockPath = join(location.parent, `.${location.name}.forme-lock`);
  await requireUnlocked(lockPath);
  const reader = createVerifiedContentReader(manifest, options.contentStore, { signal: options.signal });
  const existing = await inspectTarget(location.root, options.signal, scanLimits);
  if (existing !== undefined && await treeMatchesManifest(existing, manifest, reader, options.signal, scanLimits)) {
    // A concurrent changed publication may have acquired the cooperative lock
    // while this read-only verification was running. Check again before
    // reporting the currently visible tree as stable and unchanged.
    await requireParentStable(location);
    await requireTreeRootStable(existing, "TARGET_UNSAFE");
    await requireUnlocked(lockPath);
    throwIfAborted(options.signal);
    return createUnchangedTransaction(manifest);
  }

  const lock = await acquireLock(location, lockPath);
  let stage: OwnedDirectory | undefined;
  try {
    await inspectTarget(location.root, options.signal, scanLimits);
    stage = await createOwnedDirectory(location, `.${location.name}.forme-stage-`, "STAGING_UNSAFE");
    await materializeStage(stage.path, manifest, reader, options.signal);
    await verifyStagedTree(stage, manifest, options.signal, scanLimits);
    throwIfAborted(options.signal);
    return createChangedTransaction(location, manifest, stage, lock, options.signal, scanLimits);
  } catch (primaryError) {
    if (stage !== undefined) {
      try { await removeOwnedTree(location, stage); } catch (cleanupError) { attachSecondary(primaryError, cleanupError); }
    }
    try { await releaseLock(location, lock); } catch (cleanupError) { attachSecondary(primaryError, cleanupError); }
    throw primaryError;
  }
}

function createUnchangedTransaction(manifest: DeployManifest): FilesystemPublication {
  let state: FilesystemPublicationState = "prepared";
  return Object.freeze({
    get state() { return state; },
    changed: false,
    fileCount: manifest.fileCount,
    totalSizeBytes: manifest.totalSizeBytes,
    commit: async () => {
      requireState(state, "prepared", "commit");
      state = "committed";
    },
    finalize: async () => {
      requireState(state, "committed", "finalize");
      state = "finalized";
    },
    rollback: async () => {
      if (state !== "prepared" && state !== "committed") invalidState("rollback", state);
      state = "rolled-back";
    },
  });
}

function createChangedTransaction(
  location: RootLocation,
  manifest: DeployManifest,
  initialStage: OwnedDirectory,
  lock: OwnedDirectory,
  signal: AbortSignal | undefined,
  scanLimits: FilesystemScanLimits,
): FilesystemPublication {
  let state: FilesystemPublicationState = "prepared";
  let stage: OwnedDirectory | undefined = initialStage;
  let backupContainer: OwnedDirectory | undefined;
  let backupRootPath: string | undefined;
  let hadOldRoot = false;
  let rollbackPhase: "not-started" | "new-displaced" | "old-restored" = "not-started";
  let displacedContainer: OwnedDirectory | undefined;
  let displacedRootPath: string | undefined;

  const release = async (): Promise<void> => {
    await releaseLock(location, lock);
  };

  return Object.freeze({
    get state() { return state; },
    changed: true,
    fileCount: manifest.fileCount,
    totalSizeBytes: manifest.totalSizeBytes,
    commit: async () => {
      requireState(state, "prepared", "commit");
      throwIfAborted(signal);
      await requireParentStable(location);
      await requireOwnedDirectory(location, lock, "COMMIT_FAILED");
      const target = await inspectTarget(location.root, signal, scanLimits);
      if (stage === undefined) invalidState("commit", state);
      await verifyStagedTree(stage, manifest, signal, scanLimits);

      if (target !== undefined) {
        backupContainer = await createOwnedDirectory(location, `.${location.name}.forme-backup-`, "COMMIT_FAILED");
        backupRootPath = join(backupContainer.path, "root");
        try {
          // The destination is absent beneath a private container. This is
          // portable to hosts that cannot rename a directory over an existing
          // empty directory, without exposing an unreserved sibling path.
          await requireTreeRootStable(target, "COMMIT_FAILED");
          await requireOwnedDirectory(location, lock, "COMMIT_FAILED");
          await requireOwnedDirectory(location, backupContainer, "COMMIT_FAILED");
          await rename(location.root, backupRootPath);
          hadOldRoot = true;
        } catch (error) {
          const primaryError = new FilesystemPublishError(
            "COMMIT_FAILED",
            `could not move the old root to its reserved backup: ${message(error)}`,
            error,
          );
          try { await removeOwnedTree(location, backupContainer); } catch (cleanupError) { attachSecondary(primaryError, cleanupError); }
          backupContainer = undefined;
          backupRootPath = undefined;
          throw primaryError;
        }
      }

      try {
        throwIfAborted(signal);
        await requireOwnedDirectory(location, lock, "COMMIT_FAILED");
        await requireOwnedDirectory(location, stage, "STAGING_UNSAFE");
        await rename(stage.path, location.root);
        stage = undefined;
        state = "committed";
      } catch (error) {
        const primaryError = error instanceof FilesystemPublishError
          ? error
          : new FilesystemPublishError("COMMIT_FAILED", message(error), error);
        if (hadOldRoot && backupRootPath !== undefined) {
          try {
            await requireParentStable(location);
            if (backupContainer === undefined) throw new Error("backup container identity is missing");
            await requireOwnedDirectory(location, lock, "ROLLBACK_FAILED");
            await requireOwnedDirectory(location, backupContainer, "ROLLBACK_FAILED");
            await rename(backupRootPath, location.root);
            hadOldRoot = false;
            backupRootPath = undefined;
            try { await removeOwnedTree(location, backupContainer); } catch (cleanupError) { attachSecondary(primaryError, cleanupError); }
            backupContainer = undefined;
          } catch (rollbackError) {
            attachSecondary(primaryError, new FilesystemPublishError(
              "ROLLBACK_FAILED",
              `the old root could not be restored after publication failed: ${message(rollbackError)}`,
              rollbackError,
            ));
          }
        }
        throw primaryError;
      }
    },
    finalize: async () => {
      requireState(state, "committed", "finalize");
      let primaryError: FilesystemPublishError | undefined;
      let backupDeletionStarted = false;
      try {
        await requireOwnedDirectory(location, lock, "CLEANUP_FAILED");
        if (backupContainer !== undefined) {
          const backupExists = await ownedDirectoryExists(location, backupContainer, "CLEANUP_FAILED");
          if (backupExists) {
            backupDeletionStarted = true;
            await rm(backupContainer.path, { recursive: true, force: true, maxRetries: 2 });
          }
          backupContainer = undefined;
          backupRootPath = undefined;
        }
      } catch (error) {
        primaryError = new FilesystemPublishError("CLEANUP_FAILED", message(error), error);
      }
      if (primaryError !== undefined && !backupDeletionStarted && backupContainer !== undefined) {
        // Validation failed before irreversible backup deletion began. Retain
        // both rollback authority and the exclusive lock so repair can safely
        // retry finalize or choose rollback.
        throw primaryError;
      }
      if (backupDeletionStarted) {
        // Recursive deletion may be partial even when it throws. From this
        // point the backup is never again a valid rollback source.
        backupContainer = undefined;
        backupRootPath = undefined;
      }
      try { await release(); } catch (error) {
        const cleanupError = new FilesystemPublishError("CLEANUP_FAILED", message(error), error);
        if (primaryError === undefined) primaryError = cleanupError;
        else attachSecondary(primaryError, cleanupError);
      }
      // Once the retained backup is gone, finalize is irreversible even if
      // lock cleanup reports residue. If the backup still exists, preserve the
      // committed state so a repaired parent can retry or rollback safely.
      if (backupContainer === undefined) state = "finalized";
      if (primaryError !== undefined) throw primaryError;
    },
    rollback: async () => {
      if (state !== "prepared" && state !== "committed") invalidState("rollback", state);
      if (state === "prepared") {
        let primaryError: FilesystemPublishError | undefined;
        try {
          // A failed commit can leave the old root in its reserved backup when
          // even the immediate restoration rename failed. A later rollback
          // must preserve and retry that recovery path rather than treating
          // the transaction as an ordinary pre-commit stage.
          if (hadOldRoot && backupRootPath !== undefined) {
            await requireParentStable(location);
            if (await pathExists(location.root)) {
              throw new FilesystemPublishError("ROLLBACK_FAILED", "cannot restore the old root because the publication root is occupied");
            }
            if (backupContainer === undefined) throw new Error("backup container identity is missing");
            await requireOwnedDirectory(location, lock, "ROLLBACK_FAILED");
            await requireOwnedDirectory(location, backupContainer, "ROLLBACK_FAILED");
            await rename(backupRootPath, location.root);
            hadOldRoot = false;
            backupRootPath = undefined;
          }
        } catch (error) {
          primaryError = error instanceof FilesystemPublishError && error.code === "ROLLBACK_FAILED"
            ? error
            : new FilesystemPublishError("ROLLBACK_FAILED", message(error), error);
        }
        if (!hadOldRoot) {
          if (backupContainer !== undefined) {
            try { await removeOwnedTree(location, backupContainer); backupContainer = undefined; }
            catch (error) { primaryError = combineRollbackFailure(primaryError, error); }
          }
          if (stage !== undefined) {
            try { await removeOwnedTree(location, stage); stage = undefined; }
            catch (error) { primaryError = combineRollbackFailure(primaryError, error); }
          }
          try { await release(); }
          catch (error) { primaryError = combineRollbackFailure(primaryError, error); }
          if (primaryError === undefined) state = "rolled-back";
        }
        if (primaryError !== undefined) throw primaryError;
        return;
      }

      try {
        await requireParentStable(location);
        await requireOwnedDirectory(location, lock, "ROLLBACK_FAILED");
        if (rollbackPhase === "not-started") {
          const published = await inspectTargetRequired(location.root, scanLimits);
          await requireTreeRootStable(published, "ROLLBACK_FAILED");
          if (displacedContainer === undefined) {
            displacedContainer = await createOwnedDirectory(location, `.${location.name}.forme-rollback-`, "ROLLBACK_FAILED");
            displacedRootPath = join(displacedContainer.path, "root");
          }
          if (displacedRootPath === undefined) {
            throw new FilesystemPublishError("ROLLBACK_FAILED", "rollback destination identity is missing");
          }
          await requireOwnedDirectory(location, lock, "ROLLBACK_FAILED");
          await requireOwnedDirectory(location, displacedContainer, "ROLLBACK_FAILED");
          await rename(location.root, displacedRootPath);
          rollbackPhase = "new-displaced";
        }
        if (rollbackPhase === "new-displaced") {
          if (hadOldRoot) {
            if (backupContainer === undefined || backupRootPath === undefined) {
              throw new FilesystemPublishError("ROLLBACK_FAILED", "the retained old-root backup is unavailable");
            }
            await requireOwnedDirectory(location, lock, "ROLLBACK_FAILED");
            await requireOwnedDirectory(location, backupContainer, "ROLLBACK_FAILED");
            await rename(backupRootPath, location.root);
            hadOldRoot = false;
            backupRootPath = undefined;
          }
          rollbackPhase = "old-restored";
        }
        if (backupContainer !== undefined) {
          await removeOwnedTree(location, backupContainer);
          backupContainer = undefined;
        }
        if (displacedContainer !== undefined) {
          await removeOwnedTree(location, displacedContainer);
          displacedContainer = undefined;
          displacedRootPath = undefined;
        }
        await release();
        state = "rolled-back";
      } catch (error) {
        throw new FilesystemPublishError("ROLLBACK_FAILED", message(error), error);
      }
    },
  });
}

async function resolveRoot(input: string): Promise<RootLocation> {
  if (typeof input !== "string" || input.length === 0 || input.includes("\0")) {
    throw new FilesystemPublishError("ROOT_UNSAFE", "root must be a non-empty path without NUL");
  }
  const lexicalRoot = resolve(input);
  const name = basename(lexicalRoot);
  const lexicalParent = dirname(lexicalRoot);
  if (lexicalRoot === lexicalParent || name === "." || name === "..") {
    throw new FilesystemPublishError("ROOT_UNSAFE", "filesystem root must not be a filesystem volume root");
  }
  let parent: string;
  let parentIdentity: PathIdentity;
  try {
    const parentInfo = await lstat(lexicalParent);
    if (!parentInfo.isDirectory()) throw new Error("parent is not a directory");
    parent = await realpath(lexicalParent);
    parentIdentity = await directoryIdentity(parent, "ROOT_UNSAFE");
  } catch (error) {
    throw new FilesystemPublishError("ROOT_UNSAFE", `root parent must be an existing directory: ${message(error)}`, error);
  }
  return { root: join(parent, name), parent, name, parentIdentity };
}

async function requireParentStable(location: RootLocation): Promise<void> {
  const canonical = await realpath(location.parent);
  const identity = await directoryIdentity(location.parent, "ROOT_UNSAFE");
  if (canonical !== location.parent || !sameIdentity(identity, location.parentIdentity)) {
    throw new FilesystemPublishError("ROOT_UNSAFE", "the canonical root parent changed during publication");
  }
}

async function inspectTarget(
  root: string,
  signal?: AbortSignal,
  scanLimits: FilesystemScanLimits = FILESYSTEM_SCAN_LIMITS,
): Promise<TreeSnapshot | undefined> {
  throwIfAborted(signal);
  let rootInfo: Awaited<ReturnType<typeof lstat>>;
  try {
    rootInfo = await lstat(root);
  } catch (error) {
    if (isNotFound(error)) return undefined;
    throw error;
  }
  if (rootInfo.isSymbolicLink() || !rootInfo.isDirectory()) {
    throw new FilesystemPublishError("ROOT_UNSAFE", "configured root must be a real directory or absent");
  }
  try {
    return await scanTree(root, "TARGET_UNSAFE", signal, scanLimits);
  } catch (error) {
    if (isNotFound(error)) {
      throw new FilesystemPublishError("TARGET_UNSAFE", "target tree changed while it was scanned", error);
    }
    throw error;
  }
}

async function inspectTargetRequired(
  root: string,
  scanLimits: FilesystemScanLimits = FILESYSTEM_SCAN_LIMITS,
): Promise<TreeSnapshot> {
  const target = await inspectTarget(root, undefined, scanLimits);
  if (target === undefined) throw new FilesystemPublishError("ROLLBACK_FAILED", "published root disappeared before rollback");
  return target;
}

async function scanTree(
  root: string,
  code: "TARGET_UNSAFE" | "STAGING_UNSAFE",
  signal?: AbortSignal,
  scanLimits: FilesystemScanLimits = FILESYSTEM_SCAN_LIMITS,
): Promise<TreeSnapshot> {
  throwIfAborted(signal);
  const canonicalRoot = await realpath(root);
  const rootIdentity = await directoryIdentity(root, code);
  const files = new Map<string, FileSnapshot>();
  const directories = new Set<string>();
  let entryCount = 0;
  let metadataBytes = 0;
  const visit = async (directory: string, prefix: string, depth: number): Promise<void> => {
    throwIfAborted(signal);
    const entries = await opendir(directory);
    for await (const entry of entries) {
      throwIfAborted(signal);
      entryCount += 1;
      if (entryCount > scanLimits.maxEntries) {
        throw new FilesystemPublishError(code, `tree exceeds the ${scanLimits.maxEntries}-entry scan limit`);
      }
      const absolute = join(directory, entry.name);
      const portable = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
      metadataBytes += Buffer.byteLength(portable, "utf8");
      if (metadataBytes > scanLimits.maxMetadataBytes) {
        throw new FilesystemPublishError(code, `tree exceeds the ${scanLimits.maxMetadataBytes}-byte scan-metadata limit`);
      }
      const info = await lstat(absolute, { bigint: true });
      if (info.isSymbolicLink()) throw new FilesystemPublishError(code, `linked path component ${JSON.stringify(portable)} is forbidden`);
      if (info.isDirectory()) {
        if (depth >= scanLimits.maxDepth) {
          throw new FilesystemPublishError(code, `tree exceeds the ${scanLimits.maxDepth}-level scan-depth limit`);
        }
        directories.add(portable);
        await requireContainedRealPath(canonicalRoot, absolute, code);
        await visit(absolute, portable, depth + 1);
      } else if (info.isFile()) {
        if (info.nlink !== 1n) throw new FilesystemPublishError(code, `multiply linked file ${JSON.stringify(portable)} is forbidden`);
        files.set(portable, {
          dev: info.dev,
          ino: info.ino,
          size: info.size,
          mtimeNs: info.mtimeNs,
          ctimeNs: info.ctimeNs,
        });
      } else {
        throw new FilesystemPublishError(code, `non-regular target entry ${JSON.stringify(portable)} is forbidden`);
      }
    }
  };
  await visit(root, "", 0);
  await requireIdentity(root, rootIdentity, code);
  return { canonicalRoot, rootIdentity, files, directories };
}

async function treeMatchesManifest(
  snapshot: TreeSnapshot,
  manifest: DeployManifest,
  reader: VerifiedContentReader,
  signal: AbortSignal | undefined,
  scanLimits: FilesystemScanLimits,
): Promise<boolean> {
  throwIfAborted(signal);
  const paths = Object.keys(manifest.files).sort(compareText);
  if (snapshot.files.size !== paths.length) return false;
  const expectedDirectories = manifestDirectories(paths);
  if (snapshot.directories.size !== expectedDirectories.size ||
      [...expectedDirectories].some(path => !snapshot.directories.has(path))) return false;
  for (const path of paths) {
    throwIfAborted(signal);
    const entry = manifest.files[path];
    if (entry === undefined || snapshot.files.get(path)?.size !== BigInt(entry.sizeBytes)) return false;
  }
  for (const path of paths) {
    throwIfAborted(signal);
    const entry = manifest.files[path];
    if (entry === undefined) return false;
    await reader.read(path);
    const target = await readContainedFile(
      snapshot.canonicalRoot,
      path,
      "TARGET_UNSAFE",
      snapshot.files.get(path),
      signal,
    );
    if (createHash("sha256").update(target).digest("base64") !== entry.sha256) return false;
  }
  const current = await scanTree(snapshot.canonicalRoot, "TARGET_UNSAFE", signal, scanLimits);
  return sameTreeSnapshot(snapshot, current);
}

async function materializeStage(
  stageRoot: string,
  manifest: DeployManifest,
  reader: VerifiedContentReader,
  signal: AbortSignal | undefined,
): Promise<void> {
  const canonicalStage = await realpath(stageRoot);
  try {
    for (const path of Object.keys(manifest.files).sort(compareText)) {
      throwIfAborted(signal);
      const content = await reader.read(path);
      const segments = path.split("/");
      let parent = stageRoot;
      for (const segment of segments.slice(0, -1)) {
        parent = join(parent, segment);
        try { await mkdir(parent, { mode: 0o700 }); } catch (error) {
          if (!isAlreadyExists(error)) throw error;
        }
        const info = await lstat(parent);
        if (!info.isDirectory() || info.isSymbolicLink()) {
          throw new FilesystemPublishError("STAGING_UNSAFE", `staging component ${JSON.stringify(segment)} is not a real directory`);
        }
        await requireContainedRealPath(canonicalStage, parent, "STAGING_UNSAFE");
      }
      throwIfAborted(signal);
      const output = join(parent, segments.at(-1)!);
      const handle = await open(output, constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY | noFollowFlag(), 0o600);
      try {
        await handle.writeFile(content);
        await handle.sync();
      } finally {
        await handle.close();
      }
    }
  } catch (error) {
    if (error instanceof FilesystemPublishError || hasStableContentCode(error)) throw error;
    throw new FilesystemPublishError("STAGING_FAILED", message(error), error);
  }
}

async function verifyStagedTree(
  stage: OwnedDirectory,
  manifest: DeployManifest,
  signal?: AbortSignal,
  scanLimits: FilesystemScanLimits = FILESYSTEM_SCAN_LIMITS,
): Promise<void> {
  await requireOwnedDirectoryPath(stage, "STAGING_UNSAFE");
  const snapshot = await scanTree(stage.path, "STAGING_UNSAFE", signal, scanLimits);
  const paths = Object.keys(manifest.files).sort(compareText);
  const expectedDirectories = manifestDirectories(paths);
  if (snapshot.files.size !== paths.length || snapshot.directories.size !== expectedDirectories.size ||
      [...expectedDirectories].some(path => !snapshot.directories.has(path))) {
    throw new FilesystemPublishError("STAGING_UNSAFE", "staging tree does not exactly match the manifest file set");
  }
  for (const path of paths) {
    throwIfAborted(signal);
    const entry = manifest.files[path];
    if (entry === undefined || snapshot.files.get(path)?.size !== BigInt(entry.sizeBytes)) {
      throw new FilesystemPublishError("STAGING_UNSAFE", `staged file ${JSON.stringify(path)} has the wrong size`);
    }
    const content = await readContainedFile(
      snapshot.canonicalRoot,
      path,
      "STAGING_UNSAFE",
      snapshot.files.get(path),
      signal,
    );
    if (createHash("sha256").update(content).digest("base64") !== entry.sha256) {
      throw new FilesystemPublishError("STAGING_UNSAFE", `staged file ${JSON.stringify(path)} has the wrong digest`);
    }
  }
  await requireOwnedDirectoryPath(stage, "STAGING_UNSAFE");
}

async function readContainedFile(
  canonicalRoot: string,
  portablePath: string,
  code: "TARGET_UNSAFE" | "STAGING_UNSAFE",
  expected?: FileSnapshot,
  signal?: AbortSignal,
): Promise<Uint8Array> {
  const absolute = join(canonicalRoot, ...portablePath.split("/"));
  await requireContainedRealPath(canonicalRoot, dirname(absolute), code);
  const before = await lstat(absolute, { bigint: true });
  if (!before.isFile() || before.isSymbolicLink() || before.nlink !== 1n) {
    throw new FilesystemPublishError(code, `unsafe file ${JSON.stringify(portablePath)}`);
  }
  if (expected !== undefined && !sameFileSnapshot(expected, before)) {
    throw new FilesystemPublishError(code, `file ${JSON.stringify(portablePath)} changed before validation`);
  }
  const handle = await open(absolute, constants.O_RDONLY | noFollowFlag());
  try {
    const opened = await handle.stat({ bigint: true });
    if (!opened.isFile() || opened.nlink !== 1n || opened.dev !== before.dev || opened.ino !== before.ino) {
      throw new FilesystemPublishError(code, `file ${JSON.stringify(portablePath)} changed during validation`);
    }
    const content = new Uint8Array(await handle.readFile());
    throwIfAborted(signal);
    const after = await handle.stat({ bigint: true });
    if (!sameFileSnapshot(opened, after)) {
      throw new FilesystemPublishError(code, `file ${JSON.stringify(portablePath)} changed while it was read`);
    }
    return content;
  } finally {
    await handle.close();
  }
}

async function requireContainedRealPath(
  canonicalRoot: string,
  candidate: string,
  code: "TARGET_UNSAFE" | "STAGING_UNSAFE",
): Promise<void> {
  const canonical = await realpath(candidate);
  const suffix = relative(canonicalRoot, canonical);
  if (suffix === ".." || suffix.startsWith(`..${sep}`) || resolve(canonicalRoot, suffix) !== canonical) {
    throw new FilesystemPublishError(code, `${JSON.stringify(candidate)} escapes its publication root`);
  }
}

async function createOwnedDirectory(
  location: RootLocation,
  prefix: string,
  code: "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED",
): Promise<OwnedDirectory> {
  await requireParentStable(location);
  const path = await mkdtemp(join(location.parent, prefix));
  const owned = { path, identity: await directoryIdentity(path, code) };
  await requireOwnedDirectory(location, owned, code);
  return owned;
}

async function requireDirectChild(
  parent: string,
  child: string,
  code: "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED" | "CLEANUP_FAILED",
): Promise<void> {
  const canonical = await realpath(child);
  if (dirname(canonical) !== parent || canonical !== child) {
    throw new FilesystemPublishError(code, "transaction directory is not a canonical child of the target parent");
  }
}

async function acquireLock(location: RootLocation, lockPath: string): Promise<OwnedDirectory> {
  await requireParentStable(location);
  try {
    await mkdir(lockPath, { mode: 0o700 });
  } catch (error) {
    if (isAlreadyExists(error)) throw new FilesystemPublishError("TARGET_BUSY", "another filesystem publication owns the target lock", error);
    throw new FilesystemPublishError("STAGING_FAILED", `could not acquire target lock: ${message(error)}`, error);
  }
  const lock = { path: lockPath, identity: await directoryIdentity(lockPath, "STAGING_UNSAFE") };
  await requireOwnedDirectory(location, lock, "STAGING_UNSAFE");
  return lock;
}

async function requireUnlocked(lockPath: string): Promise<void> {
  try {
    await lstat(lockPath);
    throw new FilesystemPublishError("TARGET_BUSY", "another filesystem publication owns the target lock");
  } catch (error) {
    if (isNotFound(error)) return;
    throw error;
  }
}

async function releaseLock(location: RootLocation, lock: OwnedDirectory): Promise<void> {
  await requireParentStable(location);
  if (!await ownedDirectoryExists(location, lock, "CLEANUP_FAILED")) return;
  try {
    await rmdir(lock.path);
  } catch (error) {
    if (!isNotFound(error)) throw new FilesystemPublishError("CLEANUP_FAILED", `could not release target lock: ${message(error)}`, error);
  }
}

async function removeOwnedTree(location: RootLocation, owned: OwnedDirectory): Promise<void> {
  await requireParentStable(location);
  if (!await ownedDirectoryExists(location, owned, "CLEANUP_FAILED")) return;
  await rm(owned.path, { recursive: true, force: true, maxRetries: 2 });
}

async function ownedDirectoryExists(
  location: RootLocation,
  owned: OwnedDirectory,
  code: "CLEANUP_FAILED",
): Promise<boolean> {
  if (dirname(owned.path) !== location.parent) {
    throw new FilesystemPublishError(code, "owned transaction path is no longer a lexical child of its parent");
  }
  try {
    await lstat(owned.path);
  } catch (error) {
    if (isNotFound(error)) return false;
    throw error;
  }
  await requireDirectChild(location.parent, owned.path, code);
  await requireOwnedDirectoryPath(owned, code);
  return true;
}

async function requireOwnedDirectory(
  location: RootLocation,
  owned: OwnedDirectory,
  code: "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED" | "CLEANUP_FAILED",
): Promise<void> {
  await requireParentStable(location);
  await requireDirectChild(location.parent, owned.path, code);
  await requireOwnedDirectoryPath(owned, code);
}

async function requireOwnedDirectoryPath(
  owned: OwnedDirectory,
  code: "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED" | "CLEANUP_FAILED",
): Promise<void> {
  await requireIdentity(owned.path, owned.identity, code);
}

async function requireTreeRootStable(
  snapshot: TreeSnapshot,
  code: "TARGET_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED",
): Promise<void> {
  const canonical = await realpath(snapshot.canonicalRoot);
  if (canonical !== snapshot.canonicalRoot) {
    throw new FilesystemPublishError(code, "publication root changed identity before rename");
  }
  await requireIdentity(snapshot.canonicalRoot, snapshot.rootIdentity, code);
}

async function directoryIdentity(
  path: string,
  code: "ROOT_UNSAFE" | "TARGET_UNSAFE" | "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED" | "CLEANUP_FAILED",
): Promise<PathIdentity> {
  const info = await lstat(path, { bigint: true });
  if (info.isSymbolicLink() || !info.isDirectory()) {
    throw new FilesystemPublishError(code, `${JSON.stringify(path)} is not an owned real directory`);
  }
  return { dev: info.dev, ino: info.ino };
}

async function requireIdentity(
  path: string,
  expected: PathIdentity,
  code: "TARGET_UNSAFE" | "STAGING_UNSAFE" | "COMMIT_FAILED" | "ROLLBACK_FAILED" | "CLEANUP_FAILED",
): Promise<void> {
  const actual = await directoryIdentity(path, code);
  if (!sameIdentity(actual, expected)) {
    throw new FilesystemPublishError(code, `filesystem identity changed for ${JSON.stringify(path)}`);
  }
}

function sameIdentity(left: PathIdentity, right: PathIdentity): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function sameFileSnapshot(
  left: FileSnapshot | { readonly dev: bigint; readonly ino: bigint; readonly size: bigint; readonly mtimeNs: bigint; readonly ctimeNs: bigint },
  right: FileSnapshot | { readonly dev: bigint; readonly ino: bigint; readonly size: bigint; readonly mtimeNs: bigint; readonly ctimeNs: bigint },
): boolean {
  return sameIdentity(left, right) && left.size === right.size &&
    left.mtimeNs === right.mtimeNs && left.ctimeNs === right.ctimeNs;
}

function sameTreeSnapshot(left: TreeSnapshot, right: TreeSnapshot): boolean {
  if (!sameIdentity(left.rootIdentity, right.rootIdentity) ||
      left.files.size !== right.files.size ||
      left.directories.size !== right.directories.size) return false;
  for (const [path, file] of left.files) {
    const current = right.files.get(path);
    if (current === undefined || !sameFileSnapshot(file, current)) return false;
  }
  return [...left.directories].every(path => right.directories.has(path));
}

function throwIfAborted(signal: AbortSignal | undefined): void {
  if (signal?.aborted) {
    throw new FilesystemPublishError("ABORTED", "filesystem publication was aborted", signal.reason);
  }
}

function resolveScanLimits(input: Partial<FilesystemScanLimits> | undefined): FilesystemScanLimits {
  const maxEntries = input?.maxEntries ?? FILESYSTEM_SCAN_LIMITS.maxEntries;
  const maxDepth = input?.maxDepth ?? FILESYSTEM_SCAN_LIMITS.maxDepth;
  const maxMetadataBytes = input?.maxMetadataBytes ?? FILESYSTEM_SCAN_LIMITS.maxMetadataBytes;
  if (!Number.isSafeInteger(maxEntries) || maxEntries < 1 || maxEntries > FILESYSTEM_SCAN_LIMITS.maxEntries) {
    throw new TypeError(`scanLimits.maxEntries must be an integer from 1 through ${FILESYSTEM_SCAN_LIMITS.maxEntries}`);
  }
  if (!Number.isSafeInteger(maxDepth) || maxDepth < 1 || maxDepth > FILESYSTEM_SCAN_LIMITS.maxDepth) {
    throw new TypeError(`scanLimits.maxDepth must be an integer from 1 through ${FILESYSTEM_SCAN_LIMITS.maxDepth}`);
  }
  if (!Number.isSafeInteger(maxMetadataBytes) || maxMetadataBytes < 1 ||
      maxMetadataBytes > FILESYSTEM_SCAN_LIMITS.maxMetadataBytes) {
    throw new TypeError(
      `scanLimits.maxMetadataBytes must be an integer from 1 through ${FILESYSTEM_SCAN_LIMITS.maxMetadataBytes}`,
    );
  }
  return Object.freeze({ maxEntries, maxDepth, maxMetadataBytes });
}

function manifestDirectories(paths: readonly string[]): ReadonlySet<string> {
  const directories = new Set<string>();
  for (const path of paths) {
    const segments = path.split("/");
    for (let length = 1; length < segments.length; length += 1) {
      directories.add(segments.slice(0, length).join("/"));
    }
  }
  return directories;
}

function attachSecondary(primary: unknown, secondary: unknown): void {
  if ((typeof primary !== "object" && typeof primary !== "function") || primary === null) return;
  const target = primary as { secondaryErrors?: unknown[] };
  try {
    if (Array.isArray(target.secondaryErrors)) {
      target.secondaryErrors.push(secondary);
    } else {
      Object.defineProperty(target, "secondaryErrors", {
        value: [secondary],
        enumerable: true,
        configurable: true,
      });
    }
  } catch {
    // The primary error remains authoritative even if a frozen foreign error
    // cannot carry structured cleanup diagnostics.
  }
}

function combineRollbackFailure(
  primary: FilesystemPublishError | undefined,
  error: unknown,
): FilesystemPublishError {
  const secondary = error instanceof FilesystemPublishError && error.code === "ROLLBACK_FAILED"
    ? error
    : new FilesystemPublishError("ROLLBACK_FAILED", message(error), error);
  if (primary === undefined) return secondary;
  attachSecondary(primary, secondary);
  return primary;
}

function requireState(actual: FilesystemPublicationState, expected: FilesystemPublicationState, operation: string): void {
  if (actual !== expected) invalidState(operation, actual);
}

function invalidState(operation: string, state: FilesystemPublicationState): never {
  throw new FilesystemPublishError("INVALID_STATE", `cannot ${operation} a ${state} filesystem publication`);
}

function noFollowFlag(): number {
  return typeof constants.O_NOFOLLOW === "number" ? constants.O_NOFOLLOW : 0;
}

function hasStableContentCode(error: unknown): boolean {
  return typeof error === "object" && error !== null &&
    "code" in error && typeof (error as { code?: unknown }).code === "string" &&
    (error as { code: string }).code.startsWith("CONTENT_");
}

function isNotFound(error: unknown): boolean {
  return nodeCode(error) === "ENOENT";
}

function isAlreadyExists(error: unknown): boolean {
  return nodeCode(error) === "EEXIST";
}

function nodeCode(error: unknown): string | undefined {
  return typeof error === "object" && error !== null && "code" in error
    ? String((error as { code?: unknown }).code)
    : undefined;
}

function pathExists(path: string): Promise<boolean> {
  return stat(path).then(() => true, error => isNotFound(error) ? false : Promise.reject(error));
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
