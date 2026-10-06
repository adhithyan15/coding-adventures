import { Buffer } from "node:buffer";
import { constants as fsConstants } from "node:fs";
import { homedir } from "node:os";
import { createInterface } from "node:readline/promises";
import { stdin, stdout } from "node:process";
import {
  lstat,
  mkdir,
  open,
  opendir,
  realpath,
} from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { isSensitive, type Capability } from "@coding-adventures/forme-capability";
import {
  parseManifest,
  resolveCapabilityTemplate,
  validateManifest,
  type TemplateEnv,
} from "@coding-adventures/forme-manifest";
import {
  installPreparedPlugin,
  PLUGIN_INSTALL_LIMITS,
  preparePluginInstallSnapshot,
  type PluginInstallResult,
  type PluginPackageFile,
} from "@coding-adventures/forme-plugin-installer-core";
import { readTrustStore } from "@coding-adventures/forme-plugin-host";
import type { CancellationToken } from "@coding-adventures/forme-stage";

const TEXT_DECODER = new TextDecoder("utf-8", { fatal: true });

export interface CapabilityReview {
  readonly pluginName: string;
  readonly pluginVersion: string;
  readonly trustTier: "verified-third-party" | "unverified-third-party";
  readonly capability: Capability;
  readonly required: boolean;
  readonly reason: string;
  readonly sensitive: boolean;
}

export interface PluginInstallInvocation {
  readonly packagePath: string;
  readonly projectRoot: string;
  readonly storageRoot: string;
  readonly cacheDir: string | null;
  readonly trustStorePath?: string;
  readonly cancellation?: CancellationToken;
  readonly reviewCapability?: (review: CapabilityReview) => Promise<boolean>;
  readonly now?: () => Date;
  readonly verifyWindowsAcl?: (
    canonicalPath: string,
    scope: "install-root" | "existing-target-tree",
  ) => boolean | Promise<boolean>;
}

export type ProductPluginInstallResult = PluginInstallResult;

export async function executePluginInstall(
  invocation: PluginInstallInvocation,
): Promise<ProductPluginInstallResult> {
  invocation.cancellation?.throwIfCancelled();
  if (process.platform === "win32" && invocation.verifyWindowsAcl === undefined) {
    throw new Error("plugin installation on Windows requires the native install-root ACL verifier");
  }
  const projectRoot = await requireCanonicalDirectory(invocation.projectRoot, "project root");
  await requirePrivateProjectRoot(projectRoot);
  const packagePath = await requireCanonicalDirectory(invocation.packagePath, "plugin package");
  const installRootRequested = join(projectRoot, "forme-plugins");
  const files = await snapshotPluginDirectory(packagePath, invocation.cancellation);
  if (files.some(file => {
    const folded = file.path.toLowerCase();
    return folded === "grants.toml" || folded.startsWith("grants.toml/");
  })) {
    throw new Error("plugin packages must not supply the host-owned grants.toml file");
  }
  const manifestFile = files.find(file => file.path === "plugin.toml");
  if (!manifestFile) throw new Error("plugin package must contain plugin.toml at its root");

  const manifest = parseManifest(TEXT_DECODER.decode(manifestFile.bytes));
  validateManifest(manifest);
  const destinationName = `plugin-${Buffer.from(manifest.plugin.name, "utf8").toString("base64url")}`;
  const environment: TemplateEnv = {
    pluginDir: join(installRootRequested, destinationName),
    storageRoot: resolve(invocation.storageRoot),
    cacheDir: invocation.cacheDir === null ? null : resolve(invocation.cacheDir),
  };
  const trustStorePath = invocation.trustStorePath ?? join(homedir(), ".forme", "trust.toml");
  const trustStore = await readTrustStore(trustStorePath);
  const declaredCapabilities = [
    ...manifest.capabilities.required.map(entry => resolveCapability(entry, environment)),
    ...manifest.capabilities.optional.map(entry => resolveCapability(entry, environment)),
  ];
  const provisionalGrants = declaredCapabilities.map(capability => ({
    capability,
    grantedAt: "1970-01-01T00:00:00Z",
  }));
  const preflight = preparePluginInstallSnapshot({
    installRoot: installRootRequested,
    files,
    trustStore,
    reviewedGrants: provisionalGrants,
    capabilityEnvironment: { storageRoot: environment.storageRoot, cacheDir: environment.cacheDir },
  });
  const identity = {
    pluginName: preflight.pluginName,
    pluginVersion: preflight.pluginVersion,
    trustTier: preflight.trustTier,
  } as const;
  const review = invocation.reviewCapability ??
    (value => reviewCapabilityInteractively(value, invocation.cancellation?.signal));
  const grantedCapabilities: Capability[] = [];
  for (const [required, entries] of [
    [true, manifest.capabilities.required],
    [false, manifest.capabilities.optional],
  ] as const) {
    for (const entry of entries) {
      invocation.cancellation?.throwIfCancelled();
      const capability = resolveCapability(entry, environment);
      const granted = await review({
        ...identity,
        capability,
        required,
        reason: entry.reason,
        sensitive: isSensitive(capability),
      });
      if (!granted && required) {
        throw new Error(`required capability ${JSON.stringify(capability)} was denied`);
      }
      if (granted) grantedCapabilities.push(capability);
    }
  }

  invocation.cancellation?.throwIfCancelled();
  const reviewedAt = (invocation.now ?? (() => new Date()))().toISOString();
  const reviewedGrants = grantedCapabilities.map(capability => ({ capability, grantedAt: reviewedAt }));
  await mkdir(installRootRequested, { recursive: true, mode: 0o700 });
  const installRoot = await requireCanonicalDirectory(installRootRequested, "plugin install root");
  const prepared = preparePluginInstallSnapshot({
    installRoot,
    files,
    trustStore,
    reviewedGrants,
    capabilityEnvironment: { storageRoot: environment.storageRoot, cacheDir: environment.cacheDir },
  });
  return installPreparedPlugin({
    prepared,
    signal: invocation.cancellation?.signal,
    verifyWindowsAcl: invocation.verifyWindowsAcl,
  });
}

export async function snapshotPluginDirectory(
  requestedRoot: string,
  cancellation?: CancellationToken,
): Promise<readonly PluginPackageFile[]> {
  const root = await requireCanonicalDirectory(requestedRoot, "plugin package");
  const files: PluginPackageFile[] = [];
  let directoryCount = 1;
  let treeEntryCount = 1;
  let totalBytes = 0;

  async function visit(directory: string, prefix: string, depth: number): Promise<void> {
    if (depth > PLUGIN_INSTALL_LIMITS.maxDepth) {
      throw new Error(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxDepth}-directory-depth limit`);
    }
    const handle = await opendir(directory);
    try {
      for await (const entry of handle) {
        cancellation?.throwIfCancelled();
        treeEntryCount += 1;
        if (treeEntryCount > PLUGIN_INSTALL_LIMITS.maxTreeEntryCount) {
          throw new Error(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxTreeEntryCount}-entry limit`);
        }
        const absolute = join(directory, entry.name);
        const packagePath = prefix.length === 0 ? entry.name : `${prefix}/${entry.name}`;
        const before = await lstat(absolute, { bigint: true });
        if (before.isSymbolicLink()) throw new Error(`plugin package contains symbolic link ${JSON.stringify(packagePath)}`);
        const canonicalBefore = await realpath(absolute);
        if (!isContained(root, canonicalBefore) || canonicalBefore !== absolute) {
          throw new Error(`plugin package entry ${JSON.stringify(packagePath)} escapes its canonical root`);
        }
        if (before.isDirectory()) {
          directoryCount += 1;
          if (directoryCount > PLUGIN_INSTALL_LIMITS.maxDirectoryCount) {
            throw new Error(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxDirectoryCount}-directory limit`);
          }
          await visit(absolute, packagePath, depth + 1);
          continue;
        }
        if (!before.isFile() || before.nlink !== 1n) {
          throw new Error(`plugin package entry ${JSON.stringify(packagePath)} must be a singly-linked regular file`);
        }
        if (files.length >= PLUGIN_INSTALL_LIMITS.maxFileCount) {
          throw new Error(`plugin package exceeds the ${PLUGIN_INSTALL_LIMITS.maxFileCount}-file limit`);
        }
        if (before.size > BigInt(PLUGIN_INSTALL_LIMITS.maxFileSizeBytes)) {
          throw new Error(`plugin package file ${JSON.stringify(packagePath)} exceeds the per-file byte limit`);
        }
        const noFollow = "O_NOFOLLOW" in fsConstants ? fsConstants.O_NOFOLLOW : 0;
        const file = await open(absolute, fsConstants.O_RDONLY | noFollow);
        try {
          const opened = await file.stat({ bigint: true });
          if (!sameFile(before, opened) || !opened.isFile() || opened.nlink !== 1n) {
            throw new Error(`plugin package file ${JSON.stringify(packagePath)} changed while opening`);
          }
          const bytes = Buffer.allocUnsafe(PLUGIN_INSTALL_LIMITS.maxFileSizeBytes + 1);
          let offset = 0;
          while (offset < bytes.length) {
            cancellation?.throwIfCancelled();
            const result = await file.read(bytes, offset, bytes.length - offset, offset);
            if (result.bytesRead === 0) break;
            offset += result.bytesRead;
          }
          if (offset > PLUGIN_INSTALL_LIMITS.maxFileSizeBytes) {
            throw new Error(`plugin package file ${JSON.stringify(packagePath)} exceeds the per-file byte limit`);
          }
          const after = await file.stat({ bigint: true });
          const canonicalAfter = await realpath(absolute);
          if (!sameFile(opened, after) || BigInt(offset) !== after.size ||
              canonicalAfter !== canonicalBefore || !isContained(root, canonicalAfter)) {
            throw new Error(`plugin package file ${JSON.stringify(packagePath)} changed while reading`);
          }
          totalBytes += offset;
          if (totalBytes > PLUGIN_INSTALL_LIMITS.maxTotalSizeBytes) {
            throw new Error("plugin package exceeds the aggregate byte limit");
          }
          files.push({ path: packagePath, bytes: Uint8Array.from(bytes.subarray(0, offset)) });
        } finally {
          await file.close();
        }
      }
    } finally {
      await handle.close().catch(() => undefined);
    }
  }

  await visit(root, "", 0);
  return Object.freeze(files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0));
}

function isContained(root: string, candidate: string): boolean {
  const path = relative(root, candidate);
  return path === "" || (path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path));
}

async function requireCanonicalDirectory(requested: string, label: string): Promise<string> {
  if (typeof requested !== "string" || requested.length === 0) throw new Error(`${label} must be a path`);
  const absolute = resolve(requested);
  const stats = await lstat(absolute);
  if (!stats.isDirectory() || stats.isSymbolicLink()) throw new Error(`${label} must be a real directory`);
  const canonical = await realpath(absolute);
  return canonical;
}

async function requirePrivateProjectRoot(path: string): Promise<void> {
  if (process.platform === "win32") return;
  const stats = await lstat(path, { bigint: true });
  const uid = process.getuid?.();
  if (uid === undefined || stats.uid !== BigInt(uid) || (stats.mode & 0o022n) !== 0n) {
    throw new Error("project root must be owned by the current user and not writable by group or others");
  }
}

function sameFile(
  left: { readonly dev: bigint; readonly ino: bigint; readonly size: bigint; readonly mtimeNs: bigint },
  right: { readonly dev: bigint; readonly ino: bigint; readonly size: bigint; readonly mtimeNs: bigint },
): boolean {
  return left.dev === right.dev && left.ino === right.ino &&
    left.size === right.size && left.mtimeNs === right.mtimeNs;
}

function resolveCapability(entry: {
  readonly realm: string;
  readonly scope: string;
  readonly detail?: string;
}, environment: TemplateEnv): Capability {
  const template = entry.detail
    ? `${entry.realm}:${entry.scope}:${entry.detail}`
    : `${entry.realm}:${entry.scope}`;
  return resolveCapabilityTemplate(template, environment) as Capability;
}

export function escapeTerminalText(value: string): string {
  let output = "";
  for (const character of value) {
    const codePoint = character.codePointAt(0)!;
    const unsafe = codePoint < 0x20 || (codePoint >= 0x7f && codePoint <= 0x9f) ||
      codePoint === 0x061c || codePoint === 0x200e || codePoint === 0x200f ||
      codePoint === 0x2028 || codePoint === 0x2029 ||
      (codePoint >= 0x202a && codePoint <= 0x202e) ||
      (codePoint >= 0x2066 && codePoint <= 0x2069);
    output += unsafe ? `\\u{${codePoint.toString(16).padStart(4, "0")}}` : character;
  }
  return output;
}

async function reviewCapabilityInteractively(
  review: CapabilityReview,
  signal?: AbortSignal,
): Promise<boolean> {
  if (!stdin.isTTY || !stdout.isTTY) {
    throw new Error("capability review requires an interactive terminal");
  }
  const marker = review.sensitive ? " SENSITIVE" : "";
  const requirement = review.required ? "required" : "optional";
  const prompt = `${escapeTerminalText(review.pluginName)}@${escapeTerminalText(review.pluginVersion)} (${review.trustTier})\n` +
    `${escapeTerminalText(review.capability)} (${requirement}${marker})\n` +
    `  ${escapeTerminalText(review.reason)}\nGrant? ${review.required ? "[y/n]" : "[y/N]"} `;
  const readline = createInterface({ input: stdin, output: stdout });
  try {
    while (true) {
      const answer = (await readline.question(prompt, { signal })).trim().toLowerCase();
      if (answer === "y" || answer === "yes") return true;
      if (answer === "n" || answer === "no" || (!review.required && answer === "")) return false;
    }
  } finally {
    readline.close();
  }
}
