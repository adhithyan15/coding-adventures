import { createHash } from "node:crypto";
import { parseDeployManifest } from "./manifest.js";
import type {
  ContentPreflightOptions,
  ContentPreflightResult,
  ContentStore,
  DeployFileEntry,
  DeployManifest,
  VerifiedContentReader,
} from "./types.js";

const typedArrayByteLengthGetter = Object.getOwnPropertyDescriptor(
  Object.getPrototypeOf(Uint8Array.prototype) as object,
  "byteLength",
)?.get;

if (typedArrayByteLengthGetter === undefined) {
  throw new Error("forme deploy core could not resolve the intrinsic typed-array byteLength getter");
}
const TYPED_ARRAY_BYTE_LENGTH = typedArrayByteLengthGetter as (this: Uint8Array) => number;

export type ContentPreflightCode =
  | "CONTENT_MISSING"
  | "CONTENT_READ_ERROR"
  | "CONTENT_SIZE_MISMATCH"
  | "CONTENT_HASH_MISMATCH"
  | "CONTENT_ABORTED";

export class ContentPreflightError extends Error {
  readonly code: ContentPreflightCode;
  readonly outputPath: string;
  readonly sha256: string;
  override readonly cause?: unknown;

  constructor(
    code: ContentPreflightCode,
    outputPath: string,
    sha256: string,
    detail: string,
    cause?: unknown,
  ) {
    super(`${code}: ${outputPath}: ${detail}`);
    this.name = "ContentPreflightError";
    this.code = code;
    this.outputPath = outputPath;
    this.sha256 = sha256;
    this.cause = cause;
  }
}

export async function preflightDeployContent(
  manifestInput: unknown,
  store: ContentStore,
  options: ContentPreflightOptions = {},
): Promise<ContentPreflightResult> {
  return await createVerifiedContentReader(manifestInput, store, options).preflight();
}

export function createVerifiedContentReader(
  manifestInput: unknown,
  store: ContentStore,
  options: ContentPreflightOptions = {},
): VerifiedContentReader {
  const manifest = parseDeployManifest(manifestInput);
  return Object.freeze({
    preflight: async () => await preflightManifest(manifest, store, options.signal),
    read: async (outputPath: string) => {
      const entry = manifest.files[outputPath];
      if (entry === undefined) throw new TypeError(`publication outputPath ${JSON.stringify(outputPath)} is not owned by the manifest`);
      return await fetchVerified(entry, outputPath, store, options.signal);
    },
  });
}

async function preflightManifest(
  manifest: DeployManifest,
  store: ContentStore,
  signal: AbortSignal | undefined,
): Promise<ContentPreflightResult> {
  const verifiedDigests = new Set<string>();
  for (const outputPath of Object.keys(manifest.files).sort(compareText)) {
    const entry = manifest.files[outputPath];
    if (entry === undefined) continue;
    throwIfAborted(signal, outputPath, entry.sha256);
    if (verifiedDigests.has(entry.sha256)) continue;
    await fetchVerified(entry, outputPath, store, signal);
    verifiedDigests.add(entry.sha256);
  }
  return Object.freeze({
    fileCount: manifest.fileCount,
    uniqueContentCount: verifiedDigests.size,
    totalSizeBytes: manifest.totalSizeBytes,
  });
}

async function fetchVerified(
  entry: DeployFileEntry,
  outputPath: string,
  store: ContentStore,
  signal: AbortSignal | undefined,
): Promise<Uint8Array> {
  throwIfAborted(signal, outputPath, entry.sha256);
  let available: boolean;
  try {
    available = await abortable(
      store.has(entry.sha256, signal),
      signal,
      outputPath,
      entry.sha256,
    );
  } catch (error) {
    if (error instanceof ContentPreflightError) throw error;
    throw new ContentPreflightError("CONTENT_READ_ERROR", outputPath, entry.sha256, message(error), error);
  }
  if (!available) {
    throw new ContentPreflightError("CONTENT_MISSING", outputPath, entry.sha256, "content store does not contain the required digest");
  }
  let content: Uint8Array;
  try {
    content = await abortable(
      store.get(entry.sha256, signal),
      signal,
      outputPath,
      entry.sha256,
    );
  } catch (error) {
    if (error instanceof ContentPreflightError) throw error;
    throw new ContentPreflightError("CONTENT_READ_ERROR", outputPath, entry.sha256, message(error), error);
  }
  if (!ArrayBuffer.isView(content) || !(content instanceof Uint8Array)) {
    throw new ContentPreflightError("CONTENT_READ_ERROR", outputPath, entry.sha256, "content store returned a non-Uint8Array value");
  }
  let byteLength: number;
  try {
    byteLength = TYPED_ARRAY_BYTE_LENGTH.call(content);
  } catch (error) {
    throw new ContentPreflightError("CONTENT_READ_ERROR", outputPath, entry.sha256, message(error), error);
  }
  if (byteLength !== entry.sizeBytes) {
    throw new ContentPreflightError(
      "CONTENT_SIZE_MISMATCH",
      outputPath,
      entry.sha256,
      `expected ${entry.sizeBytes} bytes, received ${byteLength}`,
    );
  }
  let snapshot: Uint8Array;
  let actual: string;
  try {
    snapshot = new Uint8Array(content);
    actual = createHash("sha256").update(snapshot).digest("base64");
  } catch (error) {
    throw new ContentPreflightError("CONTENT_READ_ERROR", outputPath, entry.sha256, message(error), error);
  }
  if (actual !== entry.sha256) {
    throw new ContentPreflightError(
      "CONTENT_HASH_MISMATCH",
      outputPath,
      entry.sha256,
      `expected ${entry.sha256}, received ${actual}`,
    );
  }
  return snapshot;
}

function throwIfAborted(signal: AbortSignal | undefined, outputPath: string, sha256: string): void {
  if (signal?.aborted) {
    throw new ContentPreflightError("CONTENT_ABORTED", outputPath, sha256, "content preflight was aborted", signal.reason);
  }
}

async function abortable<T>(
  operation: Promise<T>,
  signal: AbortSignal | undefined,
  outputPath: string,
  sha256: string,
): Promise<T> {
  if (signal === undefined) return await operation;
  throwIfAborted(signal, outputPath, sha256);
  return await new Promise<T>((resolve, reject) => {
    const abort = (): void => {
      signal.removeEventListener("abort", abort);
      reject(new ContentPreflightError("CONTENT_ABORTED", outputPath, sha256, "content preflight was aborted", signal.reason));
    };
    signal.addEventListener("abort", abort, { once: true });
    Promise.resolve(operation).then(
      value => {
        signal.removeEventListener("abort", abort);
        resolve(value);
      },
      error => {
        signal.removeEventListener("abort", abort);
        reject(error);
      },
    );
  });
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
