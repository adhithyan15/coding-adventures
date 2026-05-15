/**
 * @coding-adventures/forme-capability
 *
 * Forme kernel capability layer.  Three things:
 *
 *   - **Capability strings.**  Plain `<realm>:<scope>[:<detail>]`
 *     colon-separated strings.  No branding — they flow through plugin
 *     manifests as plain text.
 *
 *   - **Parser.**  `parseCapability` / `tryParseCapability` validate
 *     the format and surface the realm/scope/detail/wildcard split as
 *     a `ParsedCapability` view.
 *
 *   - **Matcher.**  `matchesCapability(declared, requested)` is the
 *     enforcement-time question every gated API asks.  Implements
 *     scope wildcards, detail wildcards, and the network-realm
 *     host-hierarchy semantics from FM01 §4.8.2.
 *
 * Plus a small kernel-realm catalogue (`KERNEL_REALMS`) and predicates
 * for install-time UX (`isFirstPartyOnly`, `isSensitive`).
 *
 * See FM01 §5 for the full design.
 */

export {
  parseCapability,
  tryParseCapability,
} from "./parser.js";
export type { Capability, ParsedCapability } from "./parser.js";

export { matchesCapability } from "./matcher.js";

export {
  KERNEL_REALMS,
  FIRST_PARTY_ONLY,
  SENSITIVE,
  isKernelRealm,
  isFirstPartyOnly,
  isSensitive,
} from "./realms.js";
export type { KernelRealm } from "./realms.js";
