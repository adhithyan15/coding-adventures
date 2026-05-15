/**
 * The kernel-blessed capability realms (FM01 §5.2).
 *
 * Plugins MAY introduce new realms — those go under an `ext:` prefix
 * the host treats as opaque.  The realms here are the ones the kernel
 * itself enforces (storage, network, env, …) and are exported so that
 * (a) the manifest validator can warn about unknown realms in
 * first-party plugins, and (b) the install-time UX can offer
 * realm-specific descriptions.
 *
 * The `system:shell` capability is included but flagged as first-party
 * only — third-party plugins requesting it must be rewritten to avoid
 * shell execution.  See FM01 §4.8.5 for the full discipline.
 */

/** Closed list of kernel-blessed realm names. */
export const KERNEL_REALMS = Object.freeze([
  "storage",
  "network",
  "env",
  "filesystem",
  "system",
  "content",
  "editor",
  "telemetry",
  "plugin",
] as const);

export type KernelRealm = (typeof KERNEL_REALMS)[number];

/**
 * Capability strings that, by spec, only first-party stages should be
 * granted.  The host MAY refuse to grant these to third-party plugins
 * even with explicit user consent.  See FM01 §5.5.
 */
export const FIRST_PARTY_ONLY = Object.freeze([
  "system:shell",
  "system:time-nondeterministic",
] as const);

/**
 * Capability strings that always trigger an install-time warning even
 * when granted to first-party stages.  These cross trust boundaries
 * broadly enough that the user should see them explicitly.
 */
export const SENSITIVE = Object.freeze([
  "network:*",
  "env:*",
  "filesystem:user",
  "system:shell",
] as const);

/** Predicate: is this realm one the kernel knows about? */
export function isKernelRealm(realm: string): realm is KernelRealm {
  return (KERNEL_REALMS as readonly string[]).includes(realm);
}

/** Predicate: is this capability first-party-only by spec? */
export function isFirstPartyOnly(cap: string): boolean {
  return (FIRST_PARTY_ONLY as readonly string[]).includes(cap);
}

/** Predicate: should this capability surface a stark install-time warning? */
export function isSensitive(cap: string): boolean {
  return (SENSITIVE as readonly string[]).includes(cap);
}
