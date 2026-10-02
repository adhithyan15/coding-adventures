import { constants, type Stats } from "node:fs";
import {
  lstat,
  mkdir,
  open,
  opendir,
  realpath,
  unlink,
} from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { basename, dirname, isAbsolute, relative, resolve, sep } from "node:path";
import type { StageContext, StorageApi, StorageStat } from "@coding-adventures/forme-stage";

type ProductCapabilityApis = Partial<Pick<
  StageContext,
  "storage" | "network" | "env" | "filesystem"
>>;

/** Trusted product backends; the plugin host still authorizes every call. */
export function createProductCapabilityApis(
  storageRoot: string,
  reservedRoots: readonly string[] = [],
): ProductCapabilityApis {
  return Object.freeze({
    storage: createProjectStorage(storageRoot, reservedRoots),
    network: Object.freeze({ fetch: (input: string | Request, init?: RequestInit) => fetch(input, init) }),
    env: Object.freeze({
      get: async (name: string) => process.env[name],
      getOrThrow: async (name: string) => {
        const value = process.env[name];
        if (value === undefined) throw new Error(`environment variable ${JSON.stringify(name)} is not set`);
        return value;
      },
    }),
    filesystem: Object.freeze({
      readAbsolute: async (path: string) => readComplete(await openAbsoluteFile(path, reservedRoots)),
      readAbsoluteBounded: async (path: string, maxBytes: number) =>
        readBounded(await openAbsoluteFile(path, reservedRoots), maxBytes, path),
      writeAbsolute: async (path: string, bytes: Uint8Array) => writeAbsolute(path, bytes, reservedRoots),
      homeDir: async () => homedir(),
      tempDir: async () => tmpdir(),
    }),
  });
}

export function createProjectStorage(rootPath: string, reservedRoots: readonly string[] = []): StorageApi {
  const root = resolve(rootPath);
  const reserved = reservedRoots.map(path => resolve(path));
  const storage: StorageApi = {
    async read(path) {
      return readComplete(await openStorageFile(root, path, reserved));
    },
    async readBounded(path, maxBytes) {
      return readBounded(await openStorageFile(root, path, reserved), maxBytes, path);
    },
    async write(path, bytes) {
      const target = await writableStoragePath(root, path, reserved);
      const flags = constants.O_WRONLY | constants.O_CREAT
        | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0);
      const handle = await open(target, flags, 0o600);
      try {
        const opened = await handle.stat();
        if (!opened.isFile() || opened.nlink !== 1) {
          throw new TypeError("storage write target must be a singly linked regular file");
        }
        await handle.truncate(0);
        await handle.writeFile(bytes);
        await handle.sync();
      } finally {
        await handle.close();
      }
    },
    async exists(path) {
      try {
        await containedExistingPath(root, path, false, reserved);
        return true;
      } catch (error) {
        if (isErrno(error, "ENOENT")) return false;
        throw error;
      }
    },
    async *list(path) {
      const directoryPath = await containedExistingPath(root, path, true, reserved);
      const directory = await opendir(directoryPath);
      try {
        for await (const entry of directory) {
          const child = resolve(directoryPath, entry.name);
          if (await isReservedPath(child, reserved)) continue;
          const info = await lstat(child);
          yield Object.freeze({
            path: storageJoin(path, entry.name),
            type: storageType(info),
          });
        }
      } finally {
        await directory.close().catch(() => undefined);
      }
    },
    watch(path) {
      validateStoragePath(path, true);
      assertNotLexicallyReserved(resolve(root, path === "." ? "" : path), reserved);
      return {
        async *[Symbol.asyncIterator]() {
          throw new Error("storage watch is not available in the v1 product adapter");
        },
      };
    },
    async remove(path) {
      const target = await containedExistingPath(root, path, false, reserved);
      const info = await lstat(target);
      if (!info.isFile() || info.isSymbolicLink() || info.nlink !== 1) {
        throw new TypeError("storage remove target must be a regular file");
      }
      await unlink(target);
    },
    async stat(path) {
      const target = await lexicalStoragePath(root, path, true);
      assertNotLexicallyReserved(target, reserved);
      const lexicalInfo = await lstat(target);
      if (lexicalInfo.isSymbolicLink()) {
        const canonicalRoot = await canonicalStorageRoot(root);
        const canonicalParent = await realpath(dirname(target));
        assertContained(canonicalRoot, canonicalParent);
        await assertNotCanonicallyReserved(target, reserved);
        return storageStat(lexicalInfo);
      }
      const info = await lstat(await containedExistingPath(root, path, false, reserved));
      return storageStat(info);
    },
  };
  return Object.freeze(storage);
}

type FileHandle = Awaited<ReturnType<typeof open>>;
type FileInfo = Stats;

async function openStorageFile(
  root: string,
  path: string,
  reservedRoots: readonly string[],
): Promise<FileHandle> {
  const target = await containedExistingPath(root, path, false, reservedRoots);
  return openAbsoluteFile(target, reservedRoots);
}

async function openAbsoluteFile(path: string, reservedRoots: readonly string[]): Promise<FileHandle> {
  if (!isAbsolute(path) || path.includes("\0")) throw new TypeError("filesystem path must be absolute without NUL");
  assertNotLexicallyReserved(path, reservedRoots);
  await assertNotCanonicallyReserved(path, reservedRoots);
  const handle = await open(
    path,
    constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0),
  );
  const info = await handle.stat();
  if (!info.isFile() || info.nlink !== 1) {
    await handle.close();
    throw new TypeError("read target must be a regular file");
  }
  return handle;
}

async function readComplete(handle: FileHandle): Promise<Uint8Array> {
  try {
    const before = await handle.stat();
    const bytes = await handle.readFile();
    const after = await handle.stat();
    if (!sameIdentity(before, after) || bytes.byteLength !== before.size || after.size !== before.size) {
      throw new Error("file changed during read");
    }
    return bytes;
  } finally {
    await handle.close();
  }
}

async function readBounded(handle: FileHandle, maxBytes: number, path: string): Promise<Uint8Array> {
  try {
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 0) throw new RangeError("maxBytes must be a non-negative safe integer");
    const before = await handle.stat();
    if (before.size > maxBytes) throw new RangeError(`file exceeds bounded read limit: ${path}`);
    const bytes = Buffer.allocUnsafe(Math.min(before.size, maxBytes) + 1);
    let offset = 0;
    while (offset < bytes.byteLength) {
      const { bytesRead } = await handle.read(bytes, offset, bytes.byteLength - offset, offset);
      if (bytesRead === 0) break;
      offset += bytesRead;
    }
    const after = await handle.stat();
    if (offset > maxBytes || offset !== before.size || !sameIdentity(before, after) || after.size !== before.size) {
      throw new Error(`file changed or exceeds bounded read limit: ${path}`);
    }
    return bytes.subarray(0, offset);
  } finally {
    await handle.close();
  }
}

async function writeAbsolute(
  path: string,
  bytes: Uint8Array,
  reservedRoots: readonly string[],
): Promise<void> {
  if (!isAbsolute(path) || path.includes("\0")) throw new TypeError("filesystem path must be absolute without NUL");
  assertNotLexicallyReserved(path, reservedRoots);
  await assertNotCanonicallyReserved(path, reservedRoots, true);
  const current = await lstat(path).catch(error => {
    if (isErrno(error, "ENOENT")) return null;
    throw error;
  });
  if (current !== null && (!current.isFile() || current.isSymbolicLink() || current.nlink !== 1)) {
    throw new TypeError("filesystem write target must be a regular file");
  }
  const handle = await open(
    path,
    constants.O_WRONLY | constants.O_CREAT
      | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0),
    0o600,
  );
  try {
    const opened = await handle.stat();
    if (!opened.isFile() || opened.nlink !== 1) {
      throw new TypeError("filesystem write target must be a singly linked regular file");
    }
    await handle.truncate(0);
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function containedExistingPath(
  root: string,
  path: string,
  directory: boolean,
  reservedRoots: readonly string[],
): Promise<string> {
  const canonicalRoot = await canonicalStorageRoot(root);
  const lexical = await lexicalStoragePath(canonicalRoot, path, true);
  assertNotLexicallyReserved(lexical, reservedRoots);
  const info = await lstat(lexical);
  if (info.isSymbolicLink()) throw new TypeError("storage paths must not traverse symbolic links");
  const canonical = await realpath(lexical);
  assertContained(canonicalRoot, canonical);
  await assertNotCanonicallyReserved(canonical, reservedRoots);
  if (directory && !info.isDirectory()) throw new TypeError("storage list target must be a directory");
  return canonical;
}

async function writableStoragePath(
  root: string,
  path: string,
  reservedRoots: readonly string[],
): Promise<string> {
  validateStoragePath(path, false);
  assertNotLexicallyReserved(resolve(root, ...path.split("/")), reservedRoots);
  await mkdir(root, { recursive: true, mode: 0o700 });
  const canonicalRoot = await canonicalStorageRoot(root);
  await assertNotCanonicallyReserved(canonicalRoot, reservedRoots);
  const segments = path.split("/");
  let parent = canonicalRoot;
  for (const segment of segments.slice(0, -1)) {
    const child = resolve(parent, segment);
    await mkdir(child, { mode: 0o700 }).catch(error => {
      if (!isErrno(error, "EEXIST")) throw error;
    });
    const info = await lstat(child);
    if (!info.isDirectory() || info.isSymbolicLink()) {
      throw new TypeError("storage parent must be a real directory");
    }
    const canonical = await realpath(child);
    assertContained(canonicalRoot, canonical);
    parent = canonical;
  }
  const target = resolve(parent, segments.at(-1)!);
  assertContained(canonicalRoot, target);
  assertNotLexicallyReserved(target, reservedRoots);
  await assertNotCanonicallyReserved(target, reservedRoots, true);
  const existing = await lstat(target).catch(error => {
    if (isErrno(error, "ENOENT")) return null;
    throw error;
  });
  if (existing !== null && (!existing.isFile() || existing.isSymbolicLink() || existing.nlink !== 1)) {
    throw new TypeError("storage write target must be a regular file");
  }
  return target;
}

async function canonicalStorageRoot(root: string): Promise<string> {
  const info = await lstat(root);
  if (!info.isDirectory() || info.isSymbolicLink()) throw new TypeError("storage root must be a real directory");
  return realpath(root);
}

async function lexicalStoragePath(root: string, path: string, allowRoot: boolean): Promise<string> {
  validateStoragePath(path, allowRoot);
  const target = path === "." ? root : resolve(root, ...path.split("/"));
  assertContained(root, target);
  return target;
}

function validateStoragePath(path: string, allowRoot: boolean): void {
  if (typeof path !== "string" || path.includes("\0") || path.includes("\\") || isAbsolute(path)) {
    throw new TypeError("storage path must be a portable relative path");
  }
  if (allowRoot && path === ".") return;
  const segments = path.split("/");
  if (segments.length === 0 || segments.some(segment => segment === "" || segment === "." || segment === "..")) {
    throw new TypeError("storage path contains an unsafe segment");
  }
}

function assertContained(root: string, target: string): void {
  const remainder = relative(root, target);
  if (remainder === ".." || remainder.startsWith(`..${sep}`) || isAbsolute(remainder)) {
    throw new TypeError("storage path escapes its configured root");
  }
}

function assertNotLexicallyReserved(target: string, reservedRoots: readonly string[]): void {
  if (reservedRoots.some(reserved => pathWithin(target, reserved))) {
    throw new TypeError("path enters a host-reserved Forme root");
  }
}

/**
 * The plugin host owns its installation and cache trees, even when user
 * content is configured at the project root. Checking both spellings matters:
 * the lexical check catches a not-yet-created target, while the canonical
 * check catches aliases through case folding or a symlinked parent.
 */
async function assertNotCanonicallyReserved(
  target: string,
  reservedRoots: readonly string[],
  allowMissingTarget = false,
): Promise<void> {
  const canonicalTarget = await realpath(target).catch(async error => {
    if (!allowMissingTarget || !isErrno(error, "ENOENT")) throw error;
    const canonicalParent = await realpath(dirname(target));
    return resolve(canonicalParent, basename(target));
  });
  for (const reserved of reservedRoots) {
    const canonicalReserved = await realpath(reserved).catch(error => {
      if (isErrno(error, "ENOENT")) return reserved;
      throw error;
    });
    if (pathWithin(canonicalTarget, canonicalReserved)) {
      throw new TypeError("path enters a host-reserved Forme root");
    }
  }
}

async function isReservedPath(target: string, reservedRoots: readonly string[]): Promise<boolean> {
  try {
    assertNotLexicallyReserved(target, reservedRoots);
    await assertNotCanonicallyReserved(target, reservedRoots);
    return false;
  } catch (error) {
    if (error instanceof TypeError && /host-reserved/.test(error.message)) return true;
    throw error;
  }
}

function pathWithin(target: string, boundary: string): boolean {
  const normalizedTarget = process.platform === "win32" ? resolve(target).toLowerCase() : resolve(target);
  const normalizedBoundary = process.platform === "win32" ? resolve(boundary).toLowerCase() : resolve(boundary);
  const remainder = relative(normalizedBoundary, normalizedTarget);
  return remainder === "" || (remainder !== ".." && !remainder.startsWith(`..${sep}`) && !isAbsolute(remainder));
}

function storageJoin(parent: string, child: string): string {
  return parent === "." ? child : `${parent}/${child}`;
}

function storageType(info: FileInfo): "file" | "dir" | "symlink" {
  if (info.isSymbolicLink()) return "symlink";
  if (info.isFile()) return "file";
  if (info.isDirectory()) return "dir";
  throw new TypeError("storage entry must be a file, directory, or symbolic link");
}

function storageStat(info: FileInfo): StorageStat {
  return Object.freeze({ size: info.size, mtimeMs: info.mtimeMs, type: storageType(info) });
}

function sameIdentity(left: FileInfo, right: FileInfo): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function isErrno(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && (error as { code?: unknown }).code === code;
}
