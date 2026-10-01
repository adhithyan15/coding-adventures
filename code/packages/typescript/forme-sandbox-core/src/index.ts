import { createHash, timingSafeEqual } from "node:crypto";
import { spawn, type ChildProcess } from "node:child_process";
import {
  constants,
  lstat,
  mkdir,
  open,
  readdir,
  realpath,
} from "node:fs/promises";
import { dirname, isAbsolute, join, normalize, relative, resolve, sep } from "node:path";
import { homedir } from "node:os";
import type { Readable, Writable } from "node:stream";
import {
  computeManifestHash,
  type RuntimeKind,
} from "@coding-adventures/forme-manifest";
import type {
  LaunchedPluginProcess,
  PluginLaunchRequest,
  PluginProcessExit,
  PluginProcessFactory,
} from "@coding-adventures/forme-plugin-host";

export type {
  VerifiedConfigSchemaSnapshot,
  VerifiedPluginSnapshot,
} from "@coding-adventures/forme-plugin-host";

export type SandboxLaunchErrorCode =
  | "SANDBOX_UNAVAILABLE"
  | "INVALID_LAUNCH_REQUEST"
  | "SNAPSHOT_IDENTITY_MISMATCH"
  | "ATTESTATION_MISMATCH"
  | "ATTESTATION_TIMEOUT";

export class SandboxLaunchError extends Error {
  readonly name = "SandboxLaunchError";
  constructor(
    readonly code: SandboxLaunchErrorCode,
    message: string,
    readonly details?: Readonly<Record<string, unknown>>,
  ) {
    super(message);
  }
}

/** The production host request type, re-exported under the sandbox vocabulary. */
export type SandboxLaunchRequest = PluginLaunchRequest;

/** A host process result narrowed to the only isolation value this package emits. */
export type SandboxedPluginProcess = LaunchedPluginProcess & { readonly isolation: "sandboxed" };

/** Compile-time integration contract with forme-plugin-host. */
export interface SandboxProcessFactory extends PluginProcessFactory {
  launch(request: PluginLaunchRequest): Promise<SandboxedPluginProcess>;
}

export interface StagedPluginSnapshot {
  readonly workingDirectory: string;
  readonly entryPath: string;
  readonly configSchemaPath: string | null;
  readonly manifestHash: string;
  readonly configSchemaHash: string | null;
  readonly entryHash: string;
}

export interface NativeSandboxPolicy {
  readonly platform: NodeJS.Platform;
  readonly provider: string;
  readonly launcherExecutable: string;
  readonly launcherPrefixArguments?: readonly string[];
  /** Host-to-supervisor protocol implemented by this native launcher. */
  readonly supervisorControl?: "windows-fd4";
}

export interface NativeLauncherOptions {
  readonly readinessTimeoutMs?: number;
  readonly runtimeExecutables?: Partial<Readonly<Record<Exclude<RuntimeKind, "binary">, string>>>;
  readonly runtimeRoots?: Partial<Readonly<Record<Exclude<RuntimeKind, "binary">, string>>>;
  readonly systemRoot?: string;
  /** Trusted-host diagnostic hook invoked after the supervisor is spawned. */
  readonly onLauncherSpawn?: (pid: number) => void;
}

interface ReadinessRecord {
  readonly protocol: number;
  readonly provider: string;
  readonly manifestHash: string;
  readonly configSchemaHash: string | null;
  readonly entryHash: string;
}

const DEFAULT_MEMORY_MB = 256;
const DEFAULT_WALL_CLOCK_MS = 30_000;
const DEFAULT_FILE_DESCRIPTORS = 128;
const MAX_READINESS_BYTES = 4 * 1024;

export async function stageVerifiedPlugin(
  request: SandboxLaunchRequest,
): Promise<StagedPluginSnapshot> {
  request = captureRequest(request);
  const directoryStat = await lstat(request.workingDirectory).catch(() => null);
  if (!directoryStat?.isDirectory() || directoryStat.isSymbolicLink()) {
    throw new SandboxLaunchError(
      "INVALID_LAUNCH_REQUEST",
      "sandbox working directory must be a real existing directory",
    );
  }
  if ((await readdir(request.workingDirectory)).length !== 0) {
    throw new SandboxLaunchError(
      "INVALID_LAUNCH_REQUEST",
      "sandbox working directory must be empty",
    );
  }

  const actualManifestHash = computeManifestHash(
    request.plugin.manifest,
    request.plugin.entryBytes,
  );
  if (actualManifestHash !== request.plugin.manifestHash) {
    throw new SandboxLaunchError(
      "SNAPSHOT_IDENTITY_MISMATCH",
      "manifest hash does not cover the requested entry snapshot",
    );
  }
  validateStage(request);

  const snapshotDirectory = join(request.workingDirectory, ".forme-snapshot");
  await mkdir(snapshotDirectory, { mode: 0o700 });
  const entryPath = join(snapshotDirectory, entryFileName(request.plugin.manifest.runtime.kind));
  await writeExclusive(entryPath, request.plugin.entryBytes, request.plugin.manifest.runtime.kind === "binary" ? 0o500 : 0o400);
  const entryHash = sha256(request.plugin.entryBytes);

  let configSchemaPath: string | null = null;
  let configSchemaHash: string | null = null;
  if (request.configSchema) {
    if (sha256(request.configSchema.bytes) !== request.configSchema.hash) {
      throw new SandboxLaunchError(
        "SNAPSHOT_IDENTITY_MISMATCH",
        "config schema hash does not cover the requested schema snapshot",
      );
    }
    validateRelativePath(request.workingDirectory, request.configSchema.relativePath);
    configSchemaPath = join(snapshotDirectory, "plugin-config-schema.json");
    await writeExclusive(configSchemaPath, request.configSchema.bytes, 0o400);
    configSchemaHash = request.configSchema.hash;
  }

  await verifyStagedBytes(entryPath, request.plugin.entryBytes);
  if (request.configSchema && configSchemaPath) {
    await verifyStagedBytes(configSchemaPath, request.configSchema.bytes);
  }
  return Object.freeze({
    workingDirectory: request.workingDirectory,
    entryPath,
    configSchemaPath,
    manifestHash: request.plugin.manifestHash,
    configSchemaHash,
    entryHash,
  });
}

function captureRequest(request: SandboxLaunchRequest): SandboxLaunchRequest {
  const manifest = deepFreeze(structuredClone(request.plugin.manifest));
  const stage = deepFreeze(structuredClone(request.stage));
  const resources = request.resources === undefined
    ? undefined
    : deepFreeze(structuredClone(request.resources));
  const configSchema = request.configSchema === null ? null : Object.freeze({
    relativePath: request.configSchema.relativePath,
    bytes: Uint8Array.from(request.configSchema.bytes),
    hash: request.configSchema.hash,
  });
  return Object.freeze({
    plugin: Object.freeze({
      manifest,
      manifestHash: request.plugin.manifestHash,
      entryBytes: Uint8Array.from(request.plugin.entryBytes),
    }),
    stage,
    instanceId: request.instanceId,
    workingDirectory: request.workingDirectory,
    resources,
    configSchema,
  });
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === "object") {
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

export async function launchWithNativeHelper(
  request: SandboxLaunchRequest,
  policy: NativeSandboxPolicy,
  options: NativeLauncherOptions = {},
): Promise<SandboxedPluginProcess> {
  request = captureRequest(request);
  if (process.platform !== policy.platform) {
    throw new SandboxLaunchError(
      "SANDBOX_UNAVAILABLE",
      `${policy.provider} requires ${policy.platform}; current platform is ${process.platform}`,
    );
  }
  if (!/^[a-z0-9-]+-v[1-9][0-9]*$/.test(policy.provider)) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "sandbox provider must be versioned");
  }
  const staged = await stageVerifiedPlugin(request);
  const launcher = await trustedExecutable(policy.launcherExecutable, "native sandbox launcher");
  const runtime = request.plugin.manifest.runtime.kind === "binary"
    ? staged.entryPath
    : await trustedExecutable(
      options.runtimeExecutables?.[request.plugin.manifest.runtime.kind]
        ?? defaultRuntime(request.plugin.manifest.runtime.kind),
      `${request.plugin.manifest.runtime.kind} runtime`,
    );
  const runtimeRoot = request.plugin.manifest.runtime.kind === "binary"
    ? staged.workingDirectory
    : await trustedRuntimeRoot(
      options.runtimeRoots?.[request.plugin.manifest.runtime.kind]
        ?? defaultRuntimeRoot(runtime),
      runtime,
    );
  const limits = resourceLimits(request.resources);
  const argumentsList = [
    ...(policy.launcherPrefixArguments ?? []),
    `--provider=${policy.provider}`,
    `--manifest-hash=${staged.manifestHash}`,
    `--schema-hash=${staged.configSchemaHash ?? "-"}`,
    `--entry-hash=${staged.entryHash}`,
    `--memory-bytes=${limits.memoryBytes}`,
    `--cpu-ms=${limits.wallClockMs}`,
    `--wall-clock-ms=${limits.wallClockMs}`,
    `--fd-limit=${limits.fileDescriptors}`,
    `--working-directory=${staged.workingDirectory}`,
    `--runtime=${runtime}`,
    `--runtime-root=${runtimeRoot}`,
    `--entry=${staged.entryPath}`,
    `--schema=${staged.configSchemaPath ?? "-"}`,
    `--stage=${request.stage.id}`,
    `--instance=${request.instanceId}`,
  ];
  const child = spawn(launcher, argumentsList, {
    cwd: staged.workingDirectory,
    detached: process.platform !== "win32",
    env: minimalEnvironment(staged.workingDirectory, options.systemRoot),
    stdio: ["pipe", "pipe", "pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  const exited = processExit(child);
  const readiness = child.stdio[3];
  const control = child.stdio[4];
  if (!child.stdin || !child.stdout || !child.stderr || !readiness || !control) {
    signalProcessTree(child, "SIGKILL", control as Writable | null, policy.supervisorControl);
    await exited.catch(() => undefined);
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", "native launcher did not expose bounded protocol pipes");
  }
  // Cancellation may race normal supervisor exit. Consume late pipe errors so
  // EPIPE cannot become an unhandled host-process exception.
  (control as Writable).on("error", Function.prototype as (...argumentsList: unknown[]) => void);
  try {
    if (child.pid !== undefined) options.onLauncherSpawn?.(child.pid);
  } catch (error) {
    signalProcessTree(child, "SIGKILL", control as Writable, policy.supervisorControl);
    await exited.catch(() => undefined);
    throw error;
  }

  try {
    const record = await readReadiness(
      readiness as Readable,
      positiveInteger(options.readinessTimeoutMs ?? 5_000, "readinessTimeoutMs"),
    );
    assertAttestation(record, policy, staged);
  } catch (error) {
    const launcherDiagnostic = readLauncherDiagnostic(child.stderr);
    signalProcessTree(child, "SIGKILL", control as Writable, policy.supervisorControl);
    const [launcherExit, launcherStderr] = await Promise.all([
      exited.catch(() => null),
      launcherDiagnostic,
    ]);
    if (error instanceof SandboxLaunchError) {
      throw new SandboxLaunchError(error.code, error.message, {
        ...error.details,
        launcherExit,
        ...(launcherStderr.length > 0 ? { launcherStderr } : {}),
      });
    }
    throw error;
  }

  return Object.freeze({
    isolation: "sandboxed" as const,
    isolationProvider: policy.provider,
    launchedManifestHash: staged.manifestHash,
    launchedConfigSchemaHash: staged.configSchemaHash,
    stdin: child.stdin,
    stdout: child.stdout,
    stderr: child.stderr,
    exited,
    signal(signal: NodeJS.Signals) {
      signalProcessTree(child, signal, control as Writable, policy.supervisorControl);
    },
  });
}

async function readLauncherDiagnostic(stream: Readable): Promise<string> {
  const chunks: Buffer[] = [];
  let bytes = 0;
  for await (const value of stream) {
    const chunk = Buffer.isBuffer(value) ? value : Buffer.from(value as Uint8Array);
    if (bytes < MAX_READINESS_BYTES) {
      const remaining = MAX_READINESS_BYTES - bytes;
      chunks.push(chunk.subarray(0, remaining));
      bytes += Math.min(chunk.length, remaining);
    }
  }
  return Buffer.concat(chunks).toString("utf8").trim();
}

function signalProcessTree(
  child: ChildProcess,
  signal: NodeJS.Signals,
  control?: Writable | null,
  supervisorControl?: NativeSandboxPolicy["supervisorControl"],
): void {
  /* v8 ignore start -- exercised by the Windows native integration gate */
  if (process.platform === "win32" && supervisorControl === "windows-fd4" && control) {
    // The Windows supervisor owns the Job and ephemeral AppContainer profile.
    // Ask it to terminate the Job so cleanup is not bypassed by TerminateProcess.
    if (!control.destroyed && control.writable) {
      try {
        control.write(`${signal}\n`, error => {
          if (error) child.kill(signal);
        });
      } catch {
        child.kill(signal);
      }
    } else {
      child.kill(signal);
    }
    return;
  }
  /* v8 ignore stop */
  if (process.platform !== "win32" && child.pid !== undefined) {
    // SIGKILL cannot be forwarded by a supervisor. Ask the trusted launcher to
    // kill and reap its owned child instead, including a child that escaped the
    // original process group. Native launchers reserve SIGUSR2 for this path.
    if (signal === "SIGKILL") {
      try { process.kill(child.pid, "SIGUSR2"); } catch { child.kill("SIGKILL"); }
      return;
    }
    try {
      process.kill(-child.pid, signal);
    } catch {
      // A launcher that exited before group creation still gets a direct kill attempt.
    }
  }
  child.kill(signal);
}

export function createNativeSandboxFactory(
  policy: NativeSandboxPolicy,
  options: NativeLauncherOptions = {},
): SandboxProcessFactory {
  return Object.freeze({
    launch: (request: SandboxLaunchRequest) => launchWithNativeHelper(request, policy, options),
  });
}

function validateStage(request: SandboxLaunchRequest): void {
  const declared = request.plugin.manifest.contributes.stages.find(stage => stage.id === request.stage.id);
  if (!declared || JSON.stringify(declared) !== JSON.stringify(request.stage)) {
    throw new SandboxLaunchError(
      "INVALID_LAUNCH_REQUEST",
      "requested stage is not an exact contribution from the verified manifest",
    );
  }
  if (request.instanceId.length === 0 || request.instanceId.includes("\0")) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "instance id must be a non-empty string without NUL");
  }
}

function entryFileName(kind: RuntimeKind): string {
  if (kind === "node" || kind === "deno" || kind === "bun") return "plugin-entry.mjs";
  if (kind === "python") return "plugin-entry.py";
  return process.platform === "win32" ? "plugin-entry.exe" : "plugin-entry";
}

function validateRelativePath(root: string, child: string): void {
  if (child.length === 0 || child.includes("\0") || isAbsolute(child)) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "schema path must be a non-empty relative path");
  }
  const normalized = normalize(child);
  if (normalized === ".." || normalized.startsWith(`..${sep}`)) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "schema path escapes the working directory");
  }
  const result = resolve(root, normalized);
  const remainder = relative(resolve(root), result);
  if (remainder === ".." || remainder.startsWith(`..${sep}`) || isAbsolute(remainder)) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "schema path escapes the working directory");
  }
}

async function writeExclusive(path: string, bytes: Uint8Array, mode: number): Promise<void> {
  const flags = constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL
    | (constants.O_NOFOLLOW ?? 0);
  let handle;
  try {
    handle = await open(path, flags, mode);
  } catch (error) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", `cannot exclusively stage ${path}`, {
      cause: String(error),
    });
  }
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function verifyStagedBytes(path: string, expected: Uint8Array): Promise<void> {
  const flags = constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0);
  let handle;
  try {
    handle = await open(path, flags);
  } catch (error) {
    throw new SandboxLaunchError("SNAPSHOT_IDENTITY_MISMATCH", `cannot re-open staged snapshot: ${path}`, {
      cause: String(error),
    });
  }
  try {
    const stat = await handle.stat();
    if (!stat.isFile() || stat.size !== expected.byteLength) {
      throw new SandboxLaunchError("SNAPSHOT_IDENTITY_MISMATCH", `staged snapshot changed: ${path}`);
    }
    const actual = new Uint8Array(await handle.readFile());
    if (actual.byteLength !== expected.byteLength
        || !timingSafeEqual(Buffer.from(actual), Buffer.from(expected))) {
      throw new SandboxLaunchError("SNAPSHOT_IDENTITY_MISMATCH", `staged snapshot changed: ${path}`);
    }
  } finally {
    await handle.close();
  }
}

function sha256(bytes: Uint8Array): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

async function trustedExecutable(path: string | undefined, label: string): Promise<string> {
  if (!path || !isAbsolute(path)) {
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", `${label} must be configured as an absolute path`);
  }
  let canonical: string | null = null;
  try {
    canonical = await realpath(path);
  } catch {
    // Converted to the package's fail-closed public error below.
  }
  if (!canonical || !(await lstat(canonical)).isFile()) {
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", `${label} is unavailable at ${path}`);
  }
  return canonical;
}

async function trustedRuntimeRoot(path: string | undefined, runtime: string): Promise<string> {
  if (!path || !isAbsolute(path)) {
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", "runtime distribution root must be an absolute path");
  }
  const canonical = await realpath(path).catch(() => null);
  if (!canonical || !(await lstat(canonical)).isDirectory()) {
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", `runtime distribution root is unavailable at ${path}`);
  }
  const remainder = relative(canonical, runtime);
  if (!remainder || remainder === ".." || remainder.startsWith(`..${sep}`) || isAbsolute(remainder)) {
    throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", "trusted runtime is outside its distribution root");
  }
  return canonical;
}

function defaultRuntimeRoot(runtime: string): string | undefined {
  const executableDirectory = dirname(runtime);
  const candidate = process.platform === "win32" || dirname(runtime).split(sep).at(-1) !== "bin"
    ? executableDirectory
    : dirname(executableDirectory);
  return resolve(candidate) === resolve(homedir()) ? undefined : candidate;
}

function defaultRuntime(kind: Exclude<RuntimeKind, "binary">): string | undefined {
  return kind === "node" ? process.execPath : undefined;
}

function resourceLimits(resources: PluginLaunchRequest["resources"]): {
  memoryBytes: number;
  wallClockMs: number;
  fileDescriptors: number;
} {
  const memoryMb = positiveInteger(resources?.maxMemoryMb ?? DEFAULT_MEMORY_MB, "maxMemoryMb");
  const wallClockMs = positiveInteger(resources?.maxWallClockMs ?? DEFAULT_WALL_CLOCK_MS, "maxWallClockMs");
  const fileDescriptors = positiveInteger(
    resources?.maxFileDescriptors ?? DEFAULT_FILE_DESCRIPTORS,
    "maxFileDescriptors",
  );
  const memoryBytes = memoryMb * 1024 * 1024;
  if (!Number.isSafeInteger(memoryBytes)) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", "maxMemoryMb is not safely representable");
  }
  return { memoryBytes, wallClockMs, fileDescriptors };
}

function positiveInteger(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new SandboxLaunchError("INVALID_LAUNCH_REQUEST", `${label} must be a positive safe integer`);
  }
  return value;
}

function minimalEnvironment(workingDirectory: string, configuredSystemRoot?: string): NodeJS.ProcessEnv {
  const environment: NodeJS.ProcessEnv = {
    HOME: workingDirectory,
    TMPDIR: workingDirectory,
    TMP: workingDirectory,
    TEMP: workingDirectory,
  };
  if (process.platform === "win32") {
    const systemRoot = configuredSystemRoot ?? process.env.SystemRoot;
    if (!systemRoot || !isAbsolute(systemRoot)) {
      throw new SandboxLaunchError("SANDBOX_UNAVAILABLE", "Windows SystemRoot is unavailable");
    }
    environment.SystemRoot = systemRoot;
    environment.WINDIR = systemRoot;
  }
  return environment;
}

function processExit(child: ChildProcess): Promise<PluginProcessExit> {
  return new Promise((resolveExit, reject) => {
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      // A launcher exit must not strand descendants in its fresh process group.
      /* v8 ignore start -- exercised by the Linux and macOS native integration gates */
      if (process.platform !== "win32" && child.pid !== undefined) {
        try { process.kill(-child.pid, "SIGKILL"); } catch { /* The group is already empty. */ }
      }
      /* v8 ignore stop */
      resolveExit({ code, signal });
    });
  });
}

async function readReadiness(stream: Readable, timeoutMs: number): Promise<ReadinessRecord> {
  return new Promise((resolveRecord, reject) => {
    let bytes = Buffer.alloc(0);
    const timer = setTimeout(() => finish(new SandboxLaunchError(
      "ATTESTATION_TIMEOUT",
      "native sandbox launcher did not attest before the readiness deadline",
    )), timeoutMs);
    timer.unref?.();

    const cleanup = (): void => {
      clearTimeout(timer);
      stream.off("data", onData);
      stream.off("end", onEnd);
      stream.off("close", onEnd);
      stream.off("error", onError);
    };
    const finish = (error: Error | null, record?: ReadinessRecord): void => {
      cleanup();
      if (error) reject(error);
      else resolveRecord(record!);
    };
    const onData = (chunk: Buffer): void => {
      bytes = Buffer.concat([bytes, chunk]);
      if (bytes.byteLength > MAX_READINESS_BYTES) {
        finish(new SandboxLaunchError("ATTESTATION_MISMATCH", "native launcher readiness record is too large"));
        return;
      }
      const newline = bytes.indexOf(0x0a);
      if (newline === -1) return;
      if (newline !== bytes.byteLength - 1) {
        finish(new SandboxLaunchError("ATTESTATION_MISMATCH", "native launcher emitted trailing readiness data"));
        return;
      }
      try {
        finish(null, parseReadiness(bytes.subarray(0, newline).toString("utf8")));
      } catch (error) {
        finish(error instanceof Error ? error : new Error(String(error)));
      }
    };
    const onEnd = (): void => finish(new SandboxLaunchError(
      "ATTESTATION_MISMATCH",
      "native launcher closed readiness pipe without an attestation",
    ));
    const onError = (error: Error): void => finish(error);
    stream.on("data", onData);
    stream.once("end", onEnd);
    stream.once("close", onEnd);
    stream.once("error", onError);
  });
}

function parseReadiness(text: string): ReadinessRecord {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new SandboxLaunchError("ATTESTATION_MISMATCH", "native launcher readiness is not JSON");
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new SandboxLaunchError("ATTESTATION_MISMATCH", "native launcher readiness must be an object");
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  const expected = ["configSchemaHash", "entryHash", "manifestHash", "protocol", "provider"];
  if (JSON.stringify(keys) !== JSON.stringify(expected)
      || record.protocol !== 1
      || typeof record.provider !== "string"
      || typeof record.manifestHash !== "string"
      || (record.configSchemaHash !== null && typeof record.configSchemaHash !== "string")
      || typeof record.entryHash !== "string") {
    throw new SandboxLaunchError("ATTESTATION_MISMATCH", "native launcher readiness shape is invalid");
  }
  return record as unknown as ReadinessRecord;
}

function assertAttestation(
  record: ReadinessRecord,
  policy: NativeSandboxPolicy,
  staged: StagedPluginSnapshot,
): void {
  if (record.provider !== policy.provider
      || record.manifestHash !== staged.manifestHash
      || record.configSchemaHash !== staged.configSchemaHash
      || record.entryHash !== staged.entryHash) {
    throw new SandboxLaunchError(
      "ATTESTATION_MISMATCH",
      "native launcher did not attest the requested policy and exact staged snapshots",
    );
  }
}
