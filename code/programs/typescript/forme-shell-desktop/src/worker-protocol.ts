import { createHash } from "node:crypto";
import { isAbsolute } from "node:path";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import type { AuthoringProject } from "@coding-adventures/forme-authoring-core";
import {
  canonicalDeployManifest,
  parseDeployManifest,
  type DeployManifest,
} from "@coding-adventures/forme-deploy-runner-core";
import type { DeployArtifact } from "@coding-adventures/forme-types";

export const MAX_WORKER_MESSAGE_BYTES = 8 * 1024 * 1024;
const MAX_REVISION_SCALARS = 256;
const MAX_NATIVE_PATH_SCALARS = 4_096;
const MAX_ARTIFACT_FILES = 10_000;
const MAX_ARTIFACT_FILE_BYTES = 64 * 1024 * 1024;
const MAX_ARTIFACT_BYTES = 512 * 1024 * 1024;

export interface WorkerBuildRequest {
  readonly schemaVersion: 1;
  readonly project: AuthoringProject;
  readonly revision: string;
  readonly output: string;
}

export interface WorkerFileRecord {
  readonly path: string;
  readonly size: number;
  readonly sha256: string;
}

export interface WorkerBuildSuccess {
  readonly schemaVersion: 1;
  readonly revision: string;
  readonly buildId: string;
  readonly manifestSha256: string;
  readonly manifest: DeployManifest;
  readonly files: readonly WorkerFileRecord[];
}

export interface WorkerBuildFailure {
  readonly schemaVersion: 1;
  readonly error: { readonly code: "BUILD_FAILED" };
}

/** Decode the worker's single closed build request. */
export function decodeWorkerRequest(bytes: Uint8Array): WorkerBuildRequest {
  if (bytes.length === 0 || bytes.length > MAX_WORKER_MESSAGE_BYTES) invalidRequest();
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch {
    invalidRequest();
  }
  const record = exactRecord(parsed, ["schemaVersion", "project", "revision", "output"]);
  if (record.schemaVersion !== 1) invalidRequest();
  const revision = boundedSafeString(record.revision, MAX_REVISION_SCALARS);
  const output = boundedSafeString(record.output, MAX_NATIVE_PATH_SCALARS);
  if (revision.length === 0 || output.length === 0 || !isAbsolute(output)) invalidRequest();

  let project: AuthoringProject;
  try {
    project = validateAuthoringProject(record.project);
  } catch {
    invalidRequest();
  }
  return Object.freeze({ schemaVersion: 1, project, revision, output });
}

/** Encode only bounded metadata; artifact bytes remain in the native workspace. */
export function encodeWorkerSuccess(revisionInput: string, artifact: DeployArtifact): Uint8Array {
  const revision = boundedSafeString(revisionInput, MAX_REVISION_SCALARS);
  if (revision.length === 0 || artifact.variant.kind !== "dist-tree") invalidResponse();
  const paths = Object.keys(artifact.files).sort();
  if (paths.length > MAX_ARTIFACT_FILES) invalidResponse();
  let aggregateBytes = 0;
  const portableIdentities = new Set<string>();
  const manifestFiles: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
  const files = paths.map((path): WorkerFileRecord => {
    const portableIdentity = path.toLowerCase();
    if (!portableArtifactPath(path)
      || [...portableIdentities].some((existing) => portableIdentity === existing
        || portableIdentity.startsWith(`${existing}/`)
        || existing.startsWith(`${portableIdentity}/`))) {
      throw new TypeError("the worker artifact path is invalid");
    }
    portableIdentities.add(portableIdentity);
    const bytes = artifact.files[path];
    if (!(bytes instanceof Uint8Array) || bytes.length > MAX_ARTIFACT_FILE_BYTES) invalidResponse();
    aggregateBytes += bytes.length;
    if (aggregateBytes > MAX_ARTIFACT_BYTES) invalidResponse();
    const digest = createHash("sha256").update(bytes).digest();
    manifestFiles[path] = Object.freeze({
      outputPath: path,
      contentType: contentType(path),
      sizeBytes: bytes.length,
      sha256: digest.toString("base64"),
      source: "extra",
    });
    return Object.freeze({
      path,
      size: bytes.length,
      sha256: digest.toString("hex"),
    });
  });
  const buildId = boundedSafeString(artifact.manifest.buildId, MAX_REVISION_SCALARS);
  if (buildId.length === 0) invalidResponse();
  const manifest = parseDeployManifest({
    version: 1,
    fileCount: files.length,
    totalSizeBytes: aggregateBytes,
    files: manifestFiles,
  });
  const manifestSha256 = createHash("sha256")
    .update(canonicalDeployManifest(manifest), "utf8")
    .digest("base64");
  const response: WorkerBuildSuccess = Object.freeze({
    schemaVersion: 1,
    revision,
    buildId,
    manifestSha256,
    manifest,
    files: Object.freeze(files),
  });
  const encoded = new TextEncoder().encode(JSON.stringify(response));
  if (encoded.length > MAX_WORKER_MESSAGE_BYTES) invalidResponse();
  return encoded;
}

/** Redact every product exception to the worker protocol's sole known failure. */
export function encodeWorkerFailure(): Uint8Array {
  const failure: WorkerBuildFailure = Object.freeze({
    schemaVersion: 1,
    error: Object.freeze({ code: "BUILD_FAILED" }),
  });
  return new TextEncoder().encode(JSON.stringify(failure));
}

function exactRecord(value: unknown, keys: readonly string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) invalidRequest();
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) invalidRequest();
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    invalidRequest();
  }
  return value as Record<string, unknown>;
}

function boundedSafeString(value: unknown, maximum: number): string {
  if (typeof value !== "string" || [...value].length > maximum || /[\u0000-\u001f\u007f]/u.test(value)) {
    invalidRequest();
  }
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) invalidRequest();
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      invalidRequest();
    }
  }
  return value;
}

function portableArtifactPath(value: string): boolean {
  if (value.length === 0
    || value.length > 4_096
    || value.startsWith("/")
    || value.includes("\\")
    || /[\u0000-\u001f\u007f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u.test(value)) {
    return false;
  }
  return value.split("/").every(portableArtifactComponent);
}

function portableArtifactComponent(component: string): boolean {
  if (component.length === 0
    || component.length > 255
    || component === "."
    || component === ".."
    || component.endsWith(".")
    || component.endsWith(" ")
    || component.includes(":")) {
    return false;
  }
  const stem = (component.split(".", 1)[0] ?? component).toLowerCase();
  return !/^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/u.test(stem);
}

function contentType(path: string): string {
  if (path.endsWith(".html")) return "text/html; charset=utf-8";
  if (path.endsWith(".css")) return "text/css; charset=utf-8";
  if (path.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (path.endsWith(".json")) return "application/json; charset=utf-8";
  if (path.endsWith(".svg")) return "image/svg+xml";
  if (path.endsWith(".png")) return "image/png";
  if (path.endsWith(".jpg") || path.endsWith(".jpeg")) return "image/jpeg";
  return "application/octet-stream";
}

function invalidRequest(): never {
  throw new TypeError("the worker request is invalid");
}

function invalidResponse(): never {
  throw new TypeError("the worker response is invalid");
}
