/**
 * Capability-gated API interfaces (FM01 §4.8).
 *
 * Every API in this file maps 1:1 to a capability or capability family
 * declared by a stage's manifest:
 *
 *   StorageApi    → storage:read, storage:write
 *   NetworkApi    → network:* / network:<host> / network:<scheme>:<host>
 *   EnvApi        → env:<VAR_NAME> or env:*
 *   FilesystemApi → filesystem:user
 *   ShellApi      → system:shell  (first-party only — see FM01 §4.8.5)
 *
 * The interfaces themselves are concrete-implementation-agnostic.  The
 * orchestrator (and individual sources/sinks) provides the actual
 * impls — `source-fs` implements `StorageApi` with disk reads, the
 * dev-server implements `watch` with `chokidar`, etc.  The kernel only
 * defines the *contract* and the *denied wrapper*.
 *
 * === Denied wrappers ===
 *
 * Every stage receives an instance of *every* API in its `StageContext`.
 * If the stage didn't declare the matching capability, the API it
 * receives is a "denied" wrapper whose every method throws
 * `CapabilityError`.  This is what makes the capability check
 * inescapable: a stage cannot smuggle access by simply having a
 * reference to the API.
 *
 * The `denied*` factories below build these wrappers.  Each takes the
 * exact capability string the call would require and embeds it in the
 * error so the stage author sees, at runtime, "you tried to call
 * storage.read; declare `storage:read` in your manifest."
 */

import { CapabilityError } from "@coding-adventures/forme-errors";

// ─── StorageApi ───────────────────────────────────────────────────────────

export interface StorageApi {
  read(path: string): Promise<Uint8Array>;
  write(path: string, bytes: Uint8Array): Promise<void>;
  exists(path: string): Promise<boolean>;
  list(path: string): AsyncIterable<StorageEntry>;
  watch(path: string): AsyncIterable<StorageChange>;
  remove(path: string): Promise<void>;
  stat(path: string): Promise<StorageStat>;
}

export interface StorageEntry {
  readonly path: string;
  readonly type: "file" | "dir" | "symlink";
}
export interface StorageChange {
  readonly path: string;
  readonly kind: "added" | "modified" | "removed";
}
export interface StorageStat {
  readonly size: number;
  readonly mtimeMs: number;
  readonly type: "file" | "dir" | "symlink";
}

/**
 * Build a StorageApi where every method throws `CapabilityError`.  The
 * orchestrator hands this to stages that lack a `storage:*` capability.
 *
 * The required capability is embedded in the error so stage authors
 * know exactly what to add to their manifest.  We default to
 * `storage:read` because read paths are the common reach; `write` and
 * the remove/watch family include `storage:write` in the message.
 */
export function deniedStorageApi(): StorageApi {
  return {
    read:    (path) => deny("storage:read",  `read(${JSON.stringify(path)})`),
    write:   (path) => deny("storage:write", `write(${JSON.stringify(path)})`),
    exists:  (path) => deny("storage:read",  `exists(${JSON.stringify(path)})`),
    list:    (path) => denyIterable("storage:read",  `list(${JSON.stringify(path)})`),
    watch:   (path) => denyIterable("storage:read",  `watch(${JSON.stringify(path)})`),
    remove:  (path) => deny("storage:write", `remove(${JSON.stringify(path)})`),
    stat:    (path) => deny("storage:read",  `stat(${JSON.stringify(path)})`),
  };
}

// ─── NetworkApi ───────────────────────────────────────────────────────────

export interface NetworkApi {
  fetch(input: string | Request, init?: RequestInit): Promise<Response>;
}

export function deniedNetworkApi(): NetworkApi {
  return {
    fetch: (input) => {
      const target = typeof input === "string" ? input : input.url;
      return deny("network:*", `fetch(${JSON.stringify(target)})`);
    },
  };
}

// ─── EnvApi ───────────────────────────────────────────────────────────────

export interface EnvApi {
  get(name: string): string | undefined;
  getOrThrow(name: string): string;
}

/**
 * Build an EnvApi that always denies.  `get` and `getOrThrow` are
 * synchronous (per FM01 §4.8.3) so they throw directly rather than
 * returning a rejected Promise.  The error names the specific env var
 * being requested so the manifest can be amended precisely
 * (`env:GITHUB_TOKEN` rather than the broader `env:*`).
 */
export function deniedEnvApi(): EnvApi {
  return {
    get: (name) => {
      throw capabilityError(`env:${name}`, `env.get(${JSON.stringify(name)})`);
    },
    getOrThrow: (name) => {
      throw capabilityError(`env:${name}`, `env.getOrThrow(${JSON.stringify(name)})`);
    },
  };
}

// ─── FilesystemApi ────────────────────────────────────────────────────────

export interface FilesystemApi {
  readAbsolute(path: string): Promise<Uint8Array>;
  writeAbsolute(path: string, bytes: Uint8Array): Promise<void>;
  homeDir(): string;
  tempDir(): string;
}

export function deniedFilesystemApi(): FilesystemApi {
  return {
    readAbsolute:  (path) => deny("filesystem:user", `readAbsolute(${JSON.stringify(path)})`),
    writeAbsolute: (path) => deny("filesystem:user", `writeAbsolute(${JSON.stringify(path)})`),
    homeDir: () => {
      throw capabilityError("filesystem:user", "homeDir()");
    },
    tempDir: () => {
      throw capabilityError("filesystem:user", "tempDir()");
    },
  };
}

// ─── ShellApi ─────────────────────────────────────────────────────────────

export interface ShellOptions {
  readonly cwd?: string;
  readonly env?: Record<string, string>;
  readonly timeoutMs?: number;
  readonly stdin?: Uint8Array;
}
export interface ShellResult {
  readonly exitCode: number;
  readonly stdout: Uint8Array;
  readonly stderr: Uint8Array;
}
export interface ShellApi {
  run(
    command: string,
    args: readonly string[],
    options?: ShellOptions,
  ): Promise<ShellResult>;
}

/**
 * Build a ShellApi that always denies.  `system:shell` is *never*
 * granted to third-party plugins (FM01 §4.8.5); the host's manifest
 * validator refuses third-party requests for it.  The denied wrapper
 * is what every stage receives by default, including first-party ones
 * that didn't declare the capability — which is the right safety
 * stance even for trusted code.
 */
export function deniedShellApi(): ShellApi {
  return {
    run: (command, args) => deny(
      "system:shell",
      `shell.run(${JSON.stringify(command)}, ${JSON.stringify([...args])})`,
    ),
  };
}

// ─── Internals ────────────────────────────────────────────────────────────

function deny<T>(capability: string, op: string): Promise<T> {
  return Promise.reject(capabilityError(capability, op));
}

/**
 * Build a denied AsyncIterable.  The first `next()` throws — there's
 * no way to lazily defer the denial without misleading the caller into
 * thinking the operation might succeed and run partial work.
 */
function denyIterable<T>(capability: string, op: string): AsyncIterable<T> {
  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      return {
        async next() {
          throw capabilityError(capability, op);
        },
      };
    },
  };
}

function capabilityError(capability: string, op: string): CapabilityError {
  return new CapabilityError({
    message: `Stage attempted ${op} without declaring ${capability}`,
    capability,
  });
}
