/**
 * validate.ts — implements every numbered rule from FM02 §3.3.
 *
 * Aggregates every violation into a single `ManifestError` with
 * structured `errors[]` (matching FM03 §2.4's `ConfigError`
 * one-pass-many-errors pattern).  This lets a UI surface every
 * problem to the plugin author at once rather than playing whack-a-
 * mole through repeated validate/edit/validate cycles.
 *
 * The validator does NOT:
 *
 *   - check filesystem state (entry-file existence, runtime
 *     availability) — those rules require I/O and live in the
 *     plugin host (FM02 §3.3 rules 5, 6)
 *   - verify signatures — `verifyManifest` from signature.ts does
 *     that; calling it requires the entry bytes which the validator
 *     doesn't have
 *
 * Everything else from §3.3 lands here.  Tests in
 * tests/validate.test.ts cover every documented rejection reason.
 *
 * @module validate
 */

import { parseCapability, isFirstPartyOnly } from "@coding-adventures/forme-capability";
import { ManifestError, type ManifestErrorEntry } from "./errors.js";
import {
  RUNTIME_KINDS,
  SIGNATURE_ALGORITHMS,
  type Manifest,
  type CapabilityEntry,
  type StageContribution,
  type KindContribution,
} from "./manifest-types.js";

/** Hard ceiling for `manifestVersion`.  Initial = 1 (FM02 §3.2). */
export const SUPPORTED_MANIFEST_VERSIONS = Object.freeze([1] as const);

/** Hard ceiling for `plugin.apiVersion` (== FM01 KERNEL_API_VERSION). */
export const SUPPORTED_API_VERSIONS = Object.freeze([1] as const);

/** Regex matching the FM02 §3.3 rule 2 plugin-name format. */
export const PLUGIN_NAME_REGEX =
  /^(@[a-z0-9][a-z0-9-]*\/)?[a-z0-9][a-z0-9-]*$/;

/** Regex matching a semver MAJOR.MINOR.PATCH (with optional pre-release/build). */
export const SEMVER_REGEX =
  /^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?(\+[A-Za-z0-9.-]+)?$/;

/** Regex matching the FM02 §3.2 ext: kind name format. */
export const EXT_KIND_NAME_REGEX = /^ext:[a-z0-9][a-z0-9-]*$/;

/** Kernel kind names that plugins are NOT allowed to redefine. */
export const RESERVED_KIND_NAMES = Object.freeze(new Set([
  "ContentSource", "ContentNode", "Collection", "Asset", "Document",
  "RenderedPage", "PrintForme", "RequestHandler", "SearchIndex",
  "Feed", "DeployArtifact", "Stream", "Void",
]));

/** Hard ceiling for `[resources]` values.  Host MAY narrow these
 *  further in policy; these are absolute upper bounds. */
export const RESOURCE_CEILINGS = Object.freeze({
  maxMemoryMb:        16 * 1024,    // 16 GiB
  maxWallClockMs:     60 * 60_000,  // 1 hour
  maxFileDescriptors: 65_536,
  maxConcurrentRpcs:  4_096,
});

/**
 * Run every FM02 §3.3 validation rule against the manifest.
 *
 * On success: returns void.
 * On failure: throws a `ManifestError` whose `.errors[]` contains
 * every individual violation found.
 */
export function validateManifest(manifest: Manifest): void {
  const findings: ManifestErrorEntry[] = [];
  const add = (e: ManifestErrorEntry) => findings.push(e);

  validateTopLevel(manifest, add);
  validatePlugin(manifest, add);
  validateRuntime(manifest, add);
  validateCapabilities(manifest, add);
  validateContributes(manifest, add);
  validateResources(manifest, add);
  validateSignature(manifest, add);

  if (findings.length > 0) {
    throw new ManifestError({
      code: findings[0]!.code,
      message: "validateManifest: manifest failed FM02 §3.3 validation",
      errors: findings,
    });
  }
}

function validateTopLevel(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  if (!SUPPORTED_MANIFEST_VERSIONS.includes(m.manifestVersion as 1)) {
    add({
      code: "MANIFEST_VERSION_UNSUPPORTED",
      path: "manifestVersion",
      message: `manifestVersion ${m.manifestVersion} is not supported; ` +
               `supported: ${SUPPORTED_MANIFEST_VERSIONS.join(", ")}`,
    });
  }
}

function validatePlugin(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  const p = m.plugin;
  if (!p || !p.name) {
    add({ code: "REQUIRED_FIELD_MISSING", path: "plugin.name",
      message: "plugin.name is required" });
  } else if (!PLUGIN_NAME_REGEX.test(p.name)) {
    add({ code: "PLUGIN_NAME_INVALID", path: "plugin.name",
      message: `plugin.name "${p.name}" must match ${PLUGIN_NAME_REGEX} ` +
               `(lowercase, optional @scope/ prefix, no spaces or uppercase)` });
  }
  if (!p?.version) {
    add({ code: "REQUIRED_FIELD_MISSING", path: "plugin.version",
      message: "plugin.version is required" });
  } else if (!SEMVER_REGEX.test(p.version)) {
    add({ code: "PLUGIN_VERSION_INVALID", path: "plugin.version",
      message: `plugin.version "${p.version}" is not valid semver MAJOR.MINOR.PATCH` });
  }
  if (typeof p?.apiVersion !== "number" || p.apiVersion <= 0) {
    add({ code: "REQUIRED_FIELD_MISSING", path: "plugin.apiVersion",
      message: "plugin.apiVersion is required and must be a positive integer" });
  } else if (!SUPPORTED_API_VERSIONS.includes(p.apiVersion as 1)) {
    add({ code: "PLUGIN_API_VERSION_INVALID", path: "plugin.apiVersion",
      message: `plugin.apiVersion ${p.apiVersion} is not supported; ` +
               `supported: ${SUPPORTED_API_VERSIONS.join(", ")}` });
  }
}

function validateRuntime(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  const r = m.runtime;
  if (!r || !r.kind) {
    add({ code: "REQUIRED_FIELD_MISSING", path: "runtime.kind",
      message: "runtime.kind is required" });
    return;
  }
  if (!RUNTIME_KINDS.includes(r.kind)) {
    add({ code: "RUNTIME_KIND_INVALID", path: "runtime.kind",
      message: `runtime.kind "${r.kind}" must be one of ${RUNTIME_KINDS.join(", ")}` });
  }
  if (r.kind === "binary") {
    if (!r.platforms || Object.keys(r.platforms).length === 0) {
      add({ code: "RUNTIME_PLATFORMS_MISSING", path: "runtime.platforms",
        message: 'runtime.kind = "binary" requires a runtime.platforms map ' +
                 'with at least one entry (e.g. linux-x86_64 = "./bin/linux")' });
    }
  } else {
    if (!r.entry) {
      add({ code: "RUNTIME_ENTRY_MISSING", path: "runtime.entry",
        message: "runtime.entry is required for non-binary runtimes" });
    }
  }
}

function validateCapabilities(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  for (const [bucket, entries] of [
    ["required" as const, m.capabilities?.required ?? []],
    ["optional" as const, m.capabilities?.optional ?? []],
  ]) {
    entries.forEach((cap, i) => validateCapabilityEntry(cap, bucket, i, add));
  }
}

function validateCapabilityEntry(
  cap: CapabilityEntry,
  bucket: "required" | "optional",
  index: number,
  add: (e: ManifestErrorEntry) => void,
): void {
  const path = `capabilities.${bucket}[${index}]`;
  if (!cap.realm) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.realm`,
      message: "capability.realm is required" });
  }
  if (!cap.scope) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.scope`,
      message: "capability.scope is required" });
  }
  if (!cap.reason) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.reason`,
      message: "capability.reason is required " +
               "(shown to the user at install time)" });
  }

  // The capability string is built from `realm:scope[:detail]`.  We
  // run it through the kernel parser so format errors surface here
  // instead of at install time.  Detail may contain a literal
  // `$variable` template — strip the template before parsing so the
  // parser's whitespace-rejection rule doesn't trip on it.
  if (cap.realm && cap.scope) {
    const raw = cap.detail
      ? `${cap.realm}:${cap.scope}:${cap.detail}`
      : `${cap.realm}:${cap.scope}`;
    try {
      parseCapability(raw);
    } catch (err) {
      add({ code: "CAPABILITY_MALFORMED", path,
        message: `capability "${raw}" failed to parse: ${(err as Error).message}` });
      return;
    }

    // FM02 §3.3 rule 8: third-party plugins MUST NOT request
    // FIRST_PARTY_ONLY capabilities.  The validator enforces this
    // regardless of trust tier; first-party plugins skip the
    // validator entirely (they're loaded by direct import, not
    // through this path).  Strip template variables from the
    // capability before checking against the kernel's static set.
    const cleaned = raw.replace(/\$[A-Za-z]+/g, "");
    if (isFirstPartyOnly(cleaned) && bucket === "required") {
      add({
        code: "CAPABILITY_FIRST_PARTY_ONLY",
        path,
        message: `capability "${cleaned}" is reserved for first-party stages; ` +
                 `third-party plugins cannot require it`,
      });
    }
  }
}

function validateContributes(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  const stages = m.contributes?.stages ?? [];
  const kinds  = m.contributes?.kinds  ?? [];

  if (stages.length === 0 && kinds.length === 0) {
    add({ code: "REQUIRED_FIELD_MISSING", path: "contributes",
      message: "contributes must declare at least one stage or kind" });
  }

  const seen = new Set<string>();
  stages.forEach((s, i) => {
    const path = `contributes.stages[${i}]`;
    validateStage(s, path, add);
    if (s.id) {
      if (seen.has(s.id)) {
        add({ code: "STAGE_ID_DUPLICATE", path: `${path}.id`,
          message: `stage id "${s.id}" appears more than once` });
      }
      seen.add(s.id);
    }
  });

  kinds.forEach((k, i) => {
    const path = `contributes.kinds[${i}]`;
    validateKind(k, path, add);
  });
}

function validateStage(
  s: StageContribution,
  path: string,
  add: (e: ManifestErrorEntry) => void,
): void {
  if (!s.id) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.id`,
      message: "stage.id is required" });
  } else if (!/^[a-z0-9][a-z0-9-]*$/.test(s.id)) {
    add({ code: "STAGE_ID_INVALID", path: `${path}.id`,
      message: `stage.id "${s.id}" must be lowercase alphanumeric with hyphens` });
  }
  if (!s.consumes) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.consumes`,
      message: "stage.consumes is required" });
  } else {
    validateKindReference(s.consumes, `${path}.consumes`, add);
  }
  if (!s.produces) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.produces`,
      message: "stage.produces is required" });
  } else {
    validateKindReference(s.produces, `${path}.produces`, add);
  }
}

function validateKindReference(
  kind: string,
  path: string,
  add: (e: ManifestErrorEntry) => void,
): void {
  if (kind.startsWith("ext:")) {
    if (!EXT_KIND_NAME_REGEX.test(kind)) {
      add({ code: "STAGE_KIND_NAME_INVALID", path,
        message: `ext kind name "${kind}" must match ${EXT_KIND_NAME_REGEX}` });
    }
  } else if (!RESERVED_KIND_NAMES.has(kind)) {
    add({ code: "STAGE_KIND_NAME_INVALID", path,
      message: `unknown kind "${kind}"; must be a kernel kind or "ext:<name>"` });
  }
}

function validateKind(
  k: KindContribution,
  path: string,
  add: (e: ManifestErrorEntry) => void,
): void {
  if (!k.name) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.name`,
      message: "kind.name is required" });
  } else if (!EXT_KIND_NAME_REGEX.test(k.name)) {
    add({ code: "KIND_NAME_INVALID", path: `${path}.name`,
      message: `kind.name "${k.name}" must begin with "ext:" (kernel kinds are reserved)` });
  }
  if (!k.version) {
    add({ code: "REQUIRED_FIELD_MISSING", path: `${path}.version`,
      message: "kind.version is required" });
  } else if (!/^\d+\.\d+$/.test(k.version)) {
    add({ code: "FIELD_TYPE_MISMATCH", path: `${path}.version`,
      message: `kind.version "${k.version}" must be MAJOR.MINOR` });
  }
  if (k.subtypeOf && !RESERVED_KIND_NAMES.has(k.subtypeOf) && !k.subtypeOf.startsWith("ext:")) {
    add({ code: "STAGE_KIND_NAME_INVALID", path: `${path}.subtypeOf`,
      message: `subtypeOf "${k.subtypeOf}" must be a kernel kind or ext:<name>` });
  }
}

function validateResources(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  if (!m.resources) return;
  for (const [field, ceiling] of Object.entries(RESOURCE_CEILINGS) as
       Array<[keyof typeof RESOURCE_CEILINGS, number]>) {
    const v = m.resources[field];
    if (v === undefined) continue;
    if (!Number.isInteger(v) || v <= 0) {
      add({ code: "RESOURCE_VALUE_INVALID", path: `resources.${field}`,
        message: `resources.${field} must be a positive integer; got ${v}` });
    } else if (v > ceiling) {
      add({ code: "RESOURCE_VALUE_INVALID", path: `resources.${field}`,
        message: `resources.${field} = ${v} exceeds the host ceiling of ${ceiling}` });
    }
  }
}

function validateSignature(m: Manifest, add: (e: ManifestErrorEntry) => void): void {
  if (!m.signature) return;
  const s = m.signature;
  if (!SIGNATURE_ALGORITHMS.includes(s.algorithm as "ed25519")) {
    add({ code: "SIGNATURE_ALGORITHM_INVALID", path: "signature.algorithm",
      message: `signature.algorithm "${s.algorithm}" must be one of ${SIGNATURE_ALGORITHMS.join(", ")}` });
  }
  if (!s.publicKey) {
    add({ code: "SIGNATURE_FIELD_MISSING", path: "signature.publicKey",
      message: "signature.publicKey is required" });
  }
  if (!s.signature) {
    add({ code: "SIGNATURE_FIELD_MISSING", path: "signature.signature",
      message: "signature.signature is required" });
  }
  if (!s.signedAt) {
    add({ code: "SIGNATURE_FIELD_MISSING", path: "signature.signedAt",
      message: "signature.signedAt is required (RFC 3339 UTC)" });
  } else if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/.test(s.signedAt)) {
    add({ code: "SIGNATURE_FIELD_MISSING", path: "signature.signedAt",
      message: `signature.signedAt "${s.signedAt}" is not RFC 3339` });
  }
}
