/**
 * Manifest types — one TypeScript interface per [section] in
 * plugin.toml (FM02 §3.2).
 *
 * Every field is `readonly` end-to-end.  Optional fields are typed as
 * `T | undefined` (vs `?:`) when their absence is semantically
 * significant — `version: 1` is required, `description` is optional
 * and may be omitted entirely.
 *
 * The shape mirrors plugin.toml's structure 1:1.  The parser
 * (parse-toml.ts) produces these shapes; the validator (validate.ts)
 * checks them against FM02 §3.3 rules; the canonical encoder
 * (canonical.ts) serialises them back to TOML for signing.
 *
 * ═══ Why TOML structure, not flat ═══════════════════════════════════
 *
 * A flat manifest with dotted keys (e.g. `plugin_name`) would be
 * easier to handle as TypeScript but harder to write as TOML and
 * harder to evolve (adding a new section means dotting more keys
 * forever).  TOML's section structure groups related fields and
 * scales well as the manifest grows.  Cost: this file has more
 * interfaces.  Benefit: every field's location is obvious to
 * authors writing plugin.toml.
 *
 * @module manifest-types
 */

/** Top-level manifest. */
export interface Manifest {
  /** Required; currently only `1` is supported. */
  readonly manifestVersion: number;
  /** Required. */
  readonly plugin: PluginIdentity;
  /** Required. */
  readonly runtime: RuntimeSpec;
  /** Optional — defaults to `{ required: [], optional: [] }` on missing. */
  readonly capabilities: CapabilityBlock;
  /** Required.  At least one of `stages` / `kinds` must be non-empty. */
  readonly contributes: ContributesBlock;
  /** Optional; absent → host defaults. */
  readonly resources?: ResourceLimits;
  /** Optional; absent → unsigned manifest. */
  readonly signature?: SignatureBlock;
}

/** `[plugin]` section. */
export interface PluginIdentity {
  /** Required.  Must match FM02 §3.3 rule 2 regex. */
  readonly name: string;
  /** Required.  Semver. */
  readonly version: string;
  /** Required.  FM01 KERNEL_API_VERSION the plugin targets. */
  readonly apiVersion: number;
  /** Optional human-readable description. */
  readonly description?: string;
  /** Optional SPDX-style license identifier. */
  readonly license?: string;
  /** Optional list of "Name <email>" strings. */
  readonly authors?: readonly string[];
  /** Optional URL. */
  readonly homepage?: string;
  /** Optional URL. */
  readonly repository?: string;
}

/** `[runtime]` section. */
export interface RuntimeSpec {
  /** Required.  One of the recognised values. */
  readonly kind: RuntimeKind;
  /** Required for non-`binary` runtimes; for `binary`, ignored if
   *  `platforms` is supplied. */
  readonly entry: string;
  /** For `binary` runtime: per-platform entry paths.
   *  Keys are platform strings of the form `<os>-<arch>` (e.g.
   *  `linux-x86_64`, `darwin-aarch64`, `windows-x86_64`). */
  readonly platforms?: Readonly<Record<string, string>>;
}

/** Recognised runtime kinds (FM02 §3.2). */
export const RUNTIME_KINDS = ["node", "deno", "bun", "python", "binary"] as const;
export type RuntimeKind = (typeof RUNTIME_KINDS)[number];

/** `[capabilities]` block — both required and optional sub-arrays. */
export interface CapabilityBlock {
  /** Required capabilities — without these the plugin can't function. */
  readonly required: readonly CapabilityEntry[];
  /** Optional capabilities — missing means reduced functionality. */
  readonly optional: readonly CapabilityEntry[];
}

/** One row of `[[capabilities.required]]` or `[[capabilities.optional]]`. */
export interface CapabilityEntry {
  /** Required.  E.g. `"filesystem"`, `"network"`, `"env"`. */
  readonly realm: string;
  /** Required.  E.g. `"read"`, `"write"`, `"api.github.com"`. */
  readonly scope: string;
  /** Optional third segment.  E.g. `"$storageRoot"`. */
  readonly detail?: string;
  /** Required.  Human-readable rationale shown to the user at install. */
  readonly reason: string;
}

/** `[contributes]` block. */
export interface ContributesBlock {
  /** Stages the plugin provides.  May be empty (kind-only contributions). */
  readonly stages: readonly StageContribution[];
  /** Kinds the plugin registers.  May be empty (stage-only contributions). */
  readonly kinds: readonly KindContribution[];
}

/** One row of `[[contributes.stages]]`. */
export interface StageContribution {
  /** Required.  Local identifier; qualified to `<plugin.name>/<id>` at
   *  load time. */
  readonly id: string;
  /** Required.  Kind name; either kernel kind or `ext:<name>`. */
  readonly consumes: string;
  /** Required.  Kind name; either kernel kind or `ext:<name>`. */
  readonly produces: string;
  /** Optional.  Path to a JSON Schema for the stage's `config` object. */
  readonly configSchema?: string;
}

/** One row of `[[contributes.kinds]]`. */
export interface KindContribution {
  /** Required.  MUST start with `ext:` — kernel kinds are reserved. */
  readonly name: string;
  /** Required.  Semver of the kind's shape. */
  readonly version: string;
  /** Optional.  Path to JSON Schema for the kind payload. */
  readonly schema?: string;
  /** Optional.  Kind this one subtypes (e.g. `"ContentNode"`). */
  readonly subtypeOf?: string;
}

/** `[resources]` section. */
export interface ResourceLimits {
  readonly maxMemoryMb?: number;
  readonly maxWallClockMs?: number;
  readonly maxFileDescriptors?: number;
  readonly maxConcurrentRpcs?: number;
}

/** `[signature]` section. */
export interface SignatureBlock {
  /** Required.  Currently only `"ed25519"` is recognised. */
  readonly algorithm: string;
  /** Required.  Base64-encoded SPKI public key. */
  readonly publicKey: string;
  /** Required.  Base64-encoded signature over the manifest hash. */
  readonly signature: string;
  /** Required.  RFC 3339 timestamp for when the signature was made. */
  readonly signedAt: string;
}

/** Recognised signature algorithms. */
export const SIGNATURE_ALGORITHMS = ["ed25519"] as const;
export type SignatureAlgorithm = (typeof SIGNATURE_ALGORITHMS)[number];
