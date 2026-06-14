/**
 * Capability strings — parsing.
 *
 * A capability is a colon-separated string with two or three segments:
 *
 *     <realm>:<scope>[:<detail>]
 *
 * Realm names the *kind* of permission ("storage", "network", "env",
 * "system", …).  Scope narrows the realm ("read", "write", a hostname,
 * a wildcard `*`).  Detail further narrows a 3-segment capability —
 * canonical example: `network:<scheme>:<host>` (FM01 §4.8.2).
 *
 * The format intentionally bottoms out at three segments.  More than
 * that and capability strings become unreadable; less than two and the
 * meaning is ambiguous (`network` alone could mean "any network use"
 * or "the network namespace prefix").  Two or three is the sweet spot.
 *
 * === Wildcards ===
 *
 * Two wildcard forms appear in practice:
 *
 *   1. **Segment wildcard.**  A whole segment is the literal string `*`.
 *      Example: `network:*` (any network host), `env:*` (any env var).
 *      `parseCapability` reports `wildcard: true` when this happens.
 *
 *   2. **Host wildcard.**  A scope value contains a leading `*.`, e.g.
 *      `network:*.google.com` matches subdomains of `google.com` only.
 *      We don't flag this in `wildcard` — that flag is reserved for
 *      pure-segment wildcards.  Host-wildcard semantics live in the
 *      matcher (`matcher.ts`) where the network realm gets special
 *      treatment.
 *
 * === Validation ===
 *
 * We accept any non-empty UTF-8 sequence in each segment except the
 * colon itself (which is the segment separator) and the empty string.
 * Stricter realm-specific validation (e.g. RFC-952 hostnames for
 * `network:`) is deliberately deferred — the kernel parses; the
 * realm-specific stages decide what's a meaningful value.  We *do*
 * reject leading/trailing whitespace and embedded newlines, both of
 * which are almost certainly bugs (a copy-paste artefact rather than
 * an intentional capability name).
 *
 * Throws `RangeError` on malformed input rather than returning a
 * sentinel.  Callers that want to test-and-handle should use
 * `tryParseCapability` instead.
 */

/**
 * Capability string.  Any colon-separated string of two or three
 * non-empty segments.  Type alias rather than a branded type because
 * capabilities flow through pipeline configuration and plugin
 * manifests as plain strings; branding adds friction without buying
 * meaningful type safety here.
 */
export type Capability = string;

/** Parsed view of a capability string. */
export interface ParsedCapability {
  /** First segment.  E.g. `"network"`, `"storage"`, `"env"`. */
  readonly realm: string;
  /** Second segment.  E.g. `"read"`, `"api.github.com"`, `"*"`. */
  readonly scope: string;
  /** Third segment when present, else null.  E.g. for `network:https:host`. */
  readonly detail: string | null;
  /**
   * `true` if any segment is the literal `"*"`.  Host wildcards like
   * `*.google.com` do NOT set this flag — see module header.
   */
  readonly wildcard: boolean;
  /** Echo of the original input string, for diagnostics. */
  readonly raw: string;
}

/**
 * Parse a capability string.  Throws `RangeError` if the input is
 * malformed (wrong segment count, empty segment, whitespace).
 */
export function parseCapability(cap: Capability): ParsedCapability {
  const result = tryParseCapability(cap);
  if (result === null) {
    throw new RangeError(
      `parseCapability: malformed capability ${JSON.stringify(cap)}; ` +
      `expected "<realm>:<scope>" or "<realm>:<scope>:<detail>" with ` +
      `non-empty segments and no whitespace`,
    );
  }
  return result;
}

/**
 * Same as `parseCapability` but returns `null` on malformed input
 * instead of throwing.  Use when validation failure is an expected
 * outcome (e.g. iterating user-supplied capabilities to surface all
 * errors at once rather than the first).
 */
export function tryParseCapability(cap: Capability): ParsedCapability | null {
  if (typeof cap !== "string" || cap.length === 0) return null;
  // Reject any whitespace, control char, or trailing colon — these are
  // almost always typos, never intentional.
  if (/[\s\x00-\x1f]/.test(cap)) return null;

  const parts = cap.split(":");
  if (parts.length < 2 || parts.length > 3) return null;
  for (const segment of parts) {
    if (segment.length === 0) return null;
  }

  const realm  = parts[0]!;
  const scope  = parts[1]!;
  const detail = parts.length === 3 ? parts[2]! : null;

  const wildcard = realm === "*" || scope === "*" || detail === "*";

  return { realm, scope, detail, wildcard, raw: cap };
}
