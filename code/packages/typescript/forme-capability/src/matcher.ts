/**
 * Capability matching — does a *declared* capability cover a *requested* one?
 *
 * Core rule, applied to parsed capabilities:
 *
 *   1. **Realms must match.**  No cross-realm coverage.
 *
 *   2. **Realm wildcard (`*:*`) is illegal.**  Capabilities must name a
 *      concrete realm; `*` at the realm position is a parse-level
 *      mistake we don't honour even if it slips through.
 *
 *   3. **Scope `*` covers any scope.**  `network:*` matches any
 *      `network:X[:Y]`.  `env:*` matches any `env:VAR`.  This is the
 *      strongest form of grant — install-time UX should warn loudly.
 *
 *   4. **Detail `*` covers any detail at the same scope.**
 *      `network:https:*` matches `network:https:foo.com`,
 *      `network:https:bar.com`, etc., but NOT `network:http:foo.com`
 *      (different scope).
 *
 *   5. **Network host hierarchy.**  In the `network` realm specifically,
 *      a declared host scope covers itself and its subdomains:
 *
 *      - `network:foo.com` covers `network:foo.com` and `network:x.foo.com`.
 *      - `network:*.foo.com` covers `network:x.foo.com` and
 *        `network:y.x.foo.com`, but NOT bare `network:foo.com`.
 *
 *      Both are needed in practice: `network:foo.com` is the common
 *      "talk to this site, ok if it redirects to a subdomain" case;
 *      `network:*.foo.com` is the strict "subdomains only" case.
 *
 *   6. **No host hierarchy outside `network`.**  Other realms use
 *      exact scope matching only — applying subdomain semantics to
 *      paths or env names would be confusing.  Realm-specific matching
 *      knowledge stays here, narrowly and deliberately.
 *
 * The function returns `false` (rather than throwing) when either
 * input is unparseable.  The orchestrator's enforcement layer wants
 * "is this allowed?" → bool; making it throw on bad input forces
 * extra try/catch noise at every check site.  Use `parseCapability`
 * directly when you want errors.
 */

import { tryParseCapability } from "./parser.js";
import type { Capability, ParsedCapability } from "./parser.js";

/**
 * Test whether a *declared* capability covers a *requested* one.
 *
 * Both arguments may be raw strings or pre-parsed; we re-parse strings
 * internally.  When either input is malformed, returns `false`.
 *
 * Asymmetric: `matchesCapability("network:*", "network:api.github.com")`
 * is `true`; the reverse is `false`.
 */
export function matchesCapability(
  declared: Capability | ParsedCapability,
  requested: Capability | ParsedCapability,
): boolean {
  const d = typeof declared  === "string" ? tryParseCapability(declared)  : declared;
  const r = typeof requested === "string" ? tryParseCapability(requested) : requested;
  if (d === null || r === null) return false;

  // Rule 1: realms must match.
  if (d.realm !== r.realm) return false;

  // Rule 3: scope wildcard covers any scope/detail in the same realm.
  if (d.scope === "*") return true;

  // Rule 5: network realm gets host-hierarchy semantics.
  if (d.realm === "network") {
    return matchesNetwork(d, r);
  }

  // Generic 2- or 3-segment matching for non-network realms.
  if (d.scope !== r.scope) return false;

  // Detail handling.
  if (d.detail === null) {
    // 2-segment declaration: requested must also be 2-segment.
    return r.detail === null;
  }
  if (d.detail === "*") {
    // 3-segment declaration with detail wildcard: any non-null detail.
    return r.detail !== null;
  }
  return d.detail === r.detail;
}

/**
 * Network-realm matching.  Both `d` and `r` are guaranteed to have
 * `realm === "network"`.  Implements the host-hierarchy rules described
 * in the module header.
 */
function matchesNetwork(d: ParsedCapability, r: ParsedCapability): boolean {
  // Two-segment vs three-segment cases (3-seg = scheme:host).
  if (d.detail === null && r.detail === null) {
    return matchesHost(d.scope, r.scope);
  }
  if (d.detail !== null && r.detail !== null) {
    // Both have schemes.  Schemes must match exactly OR declared scheme
    // can be `*`.  Then the host hierarchy rule applies to the detail.
    if (d.scope !== "*" && d.scope !== r.scope) return false;
    return matchesHost(d.detail, r.detail);
  }
  // Mismatched arity: a 2-segment declaration does NOT cover a
  // 3-segment request (the request is asking for scheme-restricted
  // access; the declaration didn't grant scheme awareness).  Inverse
  // also doesn't cover: 3-segment declaration is scheme-narrower than
  // a 2-segment request.
  return false;
}

/**
 * Test whether a declared host pattern covers a requested host.
 *
 * - Exact match: `foo.com` matches `foo.com`.
 * - Subdomain coverage: `foo.com` matches `x.foo.com` (and `y.x.foo.com`).
 * - Subdomain wildcard: `*.foo.com` matches `x.foo.com` but NOT `foo.com`.
 *
 * Hosts are compared case-insensitively because DNS is case-insensitive.
 */
function matchesHost(declaredHost: string, requestedHost: string): boolean {
  const dh = declaredHost.toLowerCase();
  const rh = requestedHost.toLowerCase();

  if (dh === rh) return true;

  if (dh.startsWith("*.")) {
    // Subdomain-only wildcard.  Must end with `.<rest>` AND have at
    // least one character before that suffix.
    const suffix = dh.slice(1); // ".foo.com"
    return rh.endsWith(suffix) && rh.length > suffix.length;
  }

  // Plain host: covers itself and any deeper subdomain.
  return rh.endsWith("." + dh);
}
