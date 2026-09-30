import {
  open,
  opendir,
  realpath,
  stat,
} from "node:fs/promises";
import { constants as fsConstants } from "node:fs";
import { createHash } from "node:crypto";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import {
  computeManifestHash,
  parseManifest,
  validateManifest,
  verifyManifest,
  type Manifest,
} from "@coding-adventures/forme-manifest";
import { PluginHostError } from "./errors.js";
import type { DiscoveredPlugin } from "./types.js";

const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_ENTRY_BYTES = 128 * 1024 * 1024;
const MAX_CONFIG_SCHEMA_BYTES = 1024 * 1024;
const MAX_DISCOVERY_ROOTS = 64;
const MAX_DISCOVERY_CANDIDATES = 1_024;
const MAX_DISCOVERY_ENTRIES = 4_096;
// The host retains one private snapshot and one defensive public copy, and may
// hold one launch copy. Keeping source snapshots below 128 MiB bounds the
// worst-case live entry-byte footprint below 384 MiB.
const MAX_DISCOVERED_SNAPSHOT_BYTES = 128 * 1024 * 1024;

export async function discoverPlugins(
  roots: readonly string[],
): Promise<ReadonlyMap<string, DiscoveredPlugin>> {
  const discovered = new Map<string, DiscoveredPlugin>();
  let snapshotBytes = 0;
  let examinedCandidates = 0;
  let examinedEntries = 0;
  if (roots.length > MAX_DISCOVERY_ROOTS) {
    throw new PluginHostError(
      "RESOURCE_LIMIT_EXCEEDED",
      `plugin discovery exceeds the ${MAX_DISCOVERY_ROOTS} root limit`,
    );
  }
  for (const root of roots) {
    const namesInRoot = new Set<string>();
    const candidateNames: string[] = [];
    let directory;
    try {
      directory = await opendir(root);
    } catch (error) {
      if (isErrno(error, "ENOENT")) continue;
      throw error;
    }
    for await (const entry of directory) {
      examinedEntries += 1;
      if (examinedEntries > MAX_DISCOVERY_ENTRIES) {
        throw new PluginHostError(
          "RESOURCE_LIMIT_EXCEEDED",
          `plugin discovery exceeds the ${MAX_DISCOVERY_ENTRIES} directory-entry limit`,
        );
      }
      if (entry.isDirectory()) {
        examinedCandidates += 1;
        if (examinedCandidates > MAX_DISCOVERY_CANDIDATES) {
          throw new PluginHostError(
            "RESOURCE_LIMIT_EXCEEDED",
            `plugin discovery exceeds the ${MAX_DISCOVERY_CANDIDATES} candidate limit`,
          );
        }
        candidateNames.push(entry.name);
      }
    }
    candidateNames.sort(compareCodePoints);
    for (const candidateName of candidateNames) {
      const candidate = await loadPlugin(
        join(root, candidateName),
        MAX_DISCOVERED_SNAPSHOT_BYTES - snapshotBytes,
      );
      if (namesInRoot.has(candidate.manifest.plugin.name)) {
        throw new PluginHostError(
          "MANIFEST_INVALID",
          `duplicate plugin name ${candidate.manifest.plugin.name} in discovery root`,
          { root },
        );
      }
      namesInRoot.add(candidate.manifest.plugin.name);
      if (!discovered.has(candidate.manifest.plugin.name)) {
        snapshotBytes += snapshotByteLength(candidate);
        discovered.set(candidate.manifest.plugin.name, candidate);
      }
    }
  }
  return new Map([...discovered.entries()].sort(([a], [b]) => compareCodePoints(a, b)));
}

async function loadPlugin(directory: string, remainingSnapshotBytes: number): Promise<DiscoveredPlugin> {
  const rootDirectory = await realpath(directory);
  const manifestPath = await resolveContainedFile(rootDirectory, "plugin.toml", "plugin manifest");
  let manifest: Manifest;
  try {
    const manifestBytes = await readBoundedRegularFile(
      manifestPath,
      MAX_MANIFEST_BYTES,
      "plugin.toml",
      rootDirectory,
    );
    manifest = parseManifest(manifestBytes.toString("utf8"));
    validateManifest(manifest);
  } catch (cause) {
    if (cause instanceof PluginHostError) throw cause;
    throw new PluginHostError("MANIFEST_INVALID", "plugin.toml failed validation", {
      manifestPath,
    }, { cause });
  }
  const entryRelative = runtimeEntry(manifest);
  const entryPath = await resolveContainedFile(rootDirectory, entryRelative, "runtime entry");
  if (remainingSnapshotBytes <= 0) {
    throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", "plugin discovery exhausted its snapshot budget");
  }
  const entryBytes = await readBoundedRegularFile(
    entryPath, Math.min(MAX_ENTRY_BYTES, remainingSnapshotBytes), "runtime entry", rootDirectory,
  );
  let retainedBytes = entryBytes.byteLength;
  const configSchemas: Record<string, {
    readonly relativePath: string;
    readonly bytes: Uint8Array;
    readonly hash: string;
  }> = {};
  const schemasByPath = new Map<string, {
    readonly bytes: Uint8Array;
    readonly hash: string;
  }>();
  for (const stage of manifest.contributes.stages) {
    if (!stage.configSchema) continue;
    if (manifest.signature) {
      throw new PluginHostError(
        "MANIFEST_INVALID",
        "signed plugins cannot reference external config schemas until package signatures bind auxiliary files",
        { plugin: manifest.plugin.name, stage: stage.id },
      );
    }
    const schemaPath = await resolveContainedFile(rootDirectory, stage.configSchema, "config schema");
    const existing = schemasByPath.get(schemaPath);
    if (existing) {
      configSchemas[stage.id] = Object.freeze({
        relativePath: stage.configSchema,
        bytes: existing.bytes,
        hash: existing.hash,
      });
      continue;
    }
    const remaining = remainingSnapshotBytes - retainedBytes;
    if (remaining <= 0) {
      throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", "plugin discovery exhausted its snapshot budget");
    }
    const bytes = await readBoundedRegularFile(
      schemaPath, Math.min(MAX_CONFIG_SCHEMA_BYTES, remaining), "config schema", rootDirectory,
    );
    retainedBytes += bytes.byteLength;
    const sharedSnapshot = Object.freeze({
      bytes,
      hash: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
    });
    schemasByPath.set(schemaPath, sharedSnapshot);
    configSchemas[stage.id] = Object.freeze({
      relativePath: stage.configSchema,
      bytes: sharedSnapshot.bytes,
      hash: sharedSnapshot.hash,
    });
  }
  if (manifest.signature && !verifyManifest(manifest, entryBytes)) {
    throw new PluginHostError("MANIFEST_INVALID", "plugin signature verification failed", {
      plugin: manifest.plugin.name,
    });
  }
  const immutableManifest = deepFreeze(structuredClone(manifest));
  return Object.freeze({
    rootDirectory,
    manifestPath,
    entryPath,
    entryBytes,
    configSchemas: Object.freeze(configSchemas),
    manifest: immutableManifest,
    manifestHash: computeManifestHash(immutableManifest, entryBytes),
  });
}

function snapshotByteLength(plugin: DiscoveredPlugin): number {
  const uniqueSchemas = new Set(Object.values(plugin.configSchemas).map(schema => schema.bytes));
  return plugin.entryBytes.byteLength
    + [...uniqueSchemas].reduce((total, bytes) => total + bytes.byteLength, 0);
}

export async function readBoundedRegularFile(
  path: string,
  maxBytes: number,
  label: string,
  rootDirectory?: string,
): Promise<Buffer> {
  const noFollow = "O_NOFOLLOW" in fsConstants ? fsConstants.O_NOFOLLOW : 0;
  const nonBlock = "O_NONBLOCK" in fsConstants ? fsConstants.O_NONBLOCK : 0;
  const handle = await open(path, fsConstants.O_RDONLY | noFollow | nonBlock);
  try {
    const opened = await handle.stat();
    if (rootDirectory) {
      const currentRoot = await realpath(rootDirectory);
      if (currentRoot !== rootDirectory) {
        throw new PluginHostError("ENTRY_OUTSIDE_PLUGIN", `${label} plugin root changed during open`, { path });
      }
      const currentPath = await realpath(path);
      const rel = relative(currentRoot, currentPath);
      if (rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
        throw new PluginHostError("ENTRY_OUTSIDE_PLUGIN", `${label} escaped the plugin root during open`, { path });
      }
      const named = await stat(currentPath);
      if (named.dev !== opened.dev || named.ino !== opened.ino) {
        throw new PluginHostError("ENTRY_OUTSIDE_PLUGIN", `${label} changed during open`, { path });
      }
    }
    if (!opened.isFile() || opened.size > maxBytes) {
      throw new PluginHostError("MANIFEST_INVALID", `${label} must be a bounded regular file`, {
        path,
      });
    }
    const bytes = Buffer.allocUnsafe(Math.min(opened.size, maxBytes) + 1);
    let offset = 0;
    while (offset < bytes.byteLength) {
      const { bytesRead } = await handle.read(bytes, offset, bytes.byteLength - offset, offset);
      if (bytesRead === 0) break;
      offset += bytesRead;
    }
    if (offset > maxBytes || offset > opened.size) {
      throw new PluginHostError("MANIFEST_INVALID", `${label} changed or exceeded its byte limit`, { path });
    }
    return bytes.subarray(0, offset);
  } finally {
    await handle.close();
  }
}

function runtimeEntry(manifest: Manifest): string {
  if (manifest.runtime.kind !== "binary") return manifest.runtime.entry;
  const platform = `${platformName(process.platform)}-${architectureName(process.arch)}`;
  const entry = manifest.runtime.platforms?.[platform];
  if (!entry) {
    throw new PluginHostError("MANIFEST_INVALID", `binary plugin has no entry for ${platform}`);
  }
  return entry;
}

function platformName(value: NodeJS.Platform): string {
  return value === "win32" ? "windows" : value;
}

function architectureName(value: string): string {
  if (value === "x64") return "x86_64";
  if (value === "arm64") return "aarch64";
  return value;
}

export async function resolveContainedFile(
  rootDirectory: string,
  relativePath: string,
  label: string,
): Promise<string> {
  if (isAbsolute(relativePath)) {
    throw new PluginHostError("ENTRY_OUTSIDE_PLUGIN", `${label} must be relative`);
  }
  const lexical = resolve(rootDirectory, relativePath);
  const canonical = await realpath(lexical);
  const rel = relative(rootDirectory, canonical);
  if (rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
    throw new PluginHostError("ENTRY_OUTSIDE_PLUGIN", `${label} escapes the plugin root`, {
      rootDirectory,
      relativePath,
      canonical,
    });
  }
  return canonical;
}

function isErrno(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null
    && (error as { code?: unknown }).code === code;
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function compareCodePoints(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

/** @internal Test-only access to platform normalization helpers. */
export const __testing = { platformName, architectureName, isErrno, compareCodePoints };
