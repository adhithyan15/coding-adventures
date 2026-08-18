# VLT06 — Vault Policy

## Overview

The pluggable **policy engine** layer of the Vault stack.
Authentication (VLT05) answers *who*; policy answers *what they
can do*. Hosts a `PolicyEngine` trait and ships
`SimpleRbacEngine` plus four composition decorators in v0.1.

Implementation lives at `code/packages/rust/vault-policy/`.

## Why pluggable

Both reference targets need policy expressed differently:

- **End-user password manager** wants simple roles
  (member / admin / owner) and per-vault sharing rules.
- **HashiCorp-Vault-class machine secrets** wants a path-based
  capability DSL like HCL, or Cedar / Rego for richer rules.

Both reduce to "given a context (principal, action, resource,
factors, time, metadata), decide allow/deny." VLT06 is the trait
host; engines plug in.

## Data model

```rust
pub struct PolicyContext {
    pub principal: String,
    pub action:    String,
    pub resource:  String,
    pub factors:   Vec<String>,         // factor `kind`s from VLT05
    pub time:      u64,                  // UNIX seconds
    pub metadata:  HashMap<String, String>,
}

pub enum Decision { Allow, Deny(Reason) }

pub struct Reason(pub &'static str);
```

`Reason` is intentionally `&'static str` — never derived from
input bytes. The engine never echoes attacker-controlled
content into logs.

## Trait API

```rust
pub trait PolicyEngine: Send + Sync {
    fn kind(&self) -> &'static str;
    fn decide(&self, ctx: &PolicyContext) -> Decision;
}
```

`decide` is a pure function over the context. Implementations
must NOT touch the network, filesystem, or wall clock —
`ctx.time` is the time of record.

## Built-in engines

- **`SimpleRbacEngine`** — principal → role → set of
  `(action, resource_pattern)`. `resource_pattern` is exact
  match or `"*"`. Distinct deny reasons for unknown-principal vs
  role-lacks-perm so the operator can debug without leaking
  secret content.
- **`AllOf(inner)`** — every inner engine must allow.
- **`AnyOf(inner)`** — any inner engine allowing wins.
- **`RequireFactor(inner, factor_kind)`** — additionally requires
  `factor_kind` to appear in `ctx.factors`. Useful for step-up
  auth: allow `read` with just a password but require WebAuthn
  for `rotate-kek`.
- **`TimeBound(inner, start, end)`** — only forwards the inner
  decision when `ctx.time ∈ [start, end]` (inclusive).

## Threat model & test coverage

| Threat                                                 | Defence                                                  | Test                                                                   |
|--------------------------------------------------------|----------------------------------------------------------|------------------------------------------------------------------------|
| Caller's principal bypasses RBAC                       | Distinct deny reason `UNKNOWN_PRINCIPAL`                 | `rbac_unknown_principal_denied_with_specific_reason`                   |
| Member tries an admin-only action                      | `ROLE_LACKS_PERMISSION`                                  | `rbac_member_cannot_delete`                                            |
| Wildcard grant accidentally over-applies               | Exact-resource grants don't match other resources        | `rbac_exact_resource_grant`                                            |
| Operator forgets to give role any perms                | `ROLE_LACKS_PERMISSION` (no implicit allow)              | `rbac_role_with_no_perms_denies`                                       |
| `AllOf` with empty inner list "vacuously true"         | Empty AllOf denies                                       | `all_of_empty_denies`                                                  |
| `AnyOf` with empty inner list "vacuously true"         | Empty AnyOf denies                                       | `any_of_empty_denies`                                                  |
| Step-up bypass                                         | `RequireFactor` denies if factor absent                  | `require_factor_denies_when_factor_absent`                             |
| Time-window bypass                                     | `TimeBound` denies outside `[start, end]`                | `time_bound_outside_window_denies`                                     |
| Inclusive endpoints (off-by-one)                       | Inclusive on both ends                                   | `time_bound_inclusive_endpoints`                                       |
| Composition: nested decorators                         | Trait composition works as expected                      | `nested_all_of_require_factor_time_bound`                              |
| Attacker-controlled bytes in deny messages             | All `Reason`s are `&'static str` from a fixed table      | `reasons_are_static_literals`                                          |

## Per-secret admission policy (D18 vault runtime)

VLT06's engine answers "may this principal perform this action on this path".
This section covers a narrower, mandatory check that sits *below* it, inside
`ChiefVaultRuntime`: the policy each secret carries about **who may request it**
and **by which delivery mode**. It is stated here because it is policy, but it
is not pluggable — it is an admission check the vault always runs.

### Why it cannot be expressed by the layers above

The D18D tool gate checks `required_tier` and `required_capabilities` when a
handler is *registered*, once, per host. It is a statement about a **tool**, not
about a **secret**. `vault.request_lease` either is or is not available to a
host; there is no way to say "this host may lease the weather key but not the
bank password". So without a per-secret check, any caller that clears the tool
gate can name any registered secret.

Delivery mode makes this concrete. Direct delivery exists so that plaintext
never reaches the requesting agent — it is the mode for a bank password. Leased
delivery hands back a redeemable reference. If a secret configured for direct
delivery can be *leased* instead, the protection is not merely weakened, it is
inverted: the caller obtains exactly the material direct mode exists to
withhold, by asking differently.

### The record

Each registered secret carries:

| Field | Meaning |
|---|---|
| `privilege_tier` | minimum approval tier, 0–3 |
| `allowed_agents` | `Any`, or `Only(set)` of attested agent identities |
| `allowed_mode` | `Direct`, `Leased`, or `Both` |
| `rotated_at_ms` | when the secret was last changed |

### Rules

**P1 — mode admissibility.** A lease request against a secret whose
`allowed_mode` is `Direct` is refused; a direct request against a `Leased`
secret is refused. `Both` admits either.

**P2 — agent admissibility.** `Any` admits every caller. `Only(set)` admits a
caller whose **attested** identity is present in the set.

**P3 — absence of identity is denial, never a wildcard.** This is the rule most
easily got wrong, so it is stated separately. The requesting identity is an
`Option`. Under `Only(set)`, `None` is refused — it is not treated as "no
constraint to check". A comparison written the natural way against absent
identity fields succeeds vacuously, and a check that passes when it had nothing
to check is worse than no check, because it reads as enforcement.

The concrete hazard is present today: in the D18 tool stack only `agent_id` is
host-attested; `user_id` and `session_id` are unconditionally absent outside
tests. A rule granting on "user matches" would compare absent to absent and
admit everyone. Only fields the host actually attests may carry an admission
decision, and a binding must establish that its host attests a field before
relying on it.

**P4 — refuse before materializing.** Both checks run *before* the payload is
cloned out of storage. A refused request must never bring the secret into
memory, so that a later leak on the refusal path has nothing to leak.

**P5 — registration states policy explicitly.** There is no implicit default.
Registering a secret requires supplying its policy, so that a secret cannot
become permissive by omission. The safe-by-default direction is unavailable
here: a permissive default is silent, and a restrictive default would be
discovered only when a legitimate caller is refused.

**P6 — rotation revokes.** Re-registering a secret revokes every outstanding
lease over the previous value. A lease holds its own copy of the payload, taken
when it was issued, so overwriting the stored value alone would leave the old
one redeemable — and a secret rotated *because it was compromised* would keep
handing out the compromised value for the remaining lease lifetime. Tightening a
policy has the same shape: refusing new requests is worth little while an
already-minted reference sails past the new rule. Revocation is best-effort: a
lease already consumed or expired is simply gone. What must not survive is
anything still live.

**P7 — check the agent before the mode.** Both orders refuse the same requests.
Checking mode first tells a caller who is not permitted at all what the secret's
delivery mode is, which is a fact about the policy they have no access to. Order
the checks so the denial reveals less.

### Denials

Denial reasons are bounded and secret-free, per D18D section 7.1 V2, and carry
no free-form text. Whether a denial distinguishes "not permitted" from "no such
secret" is a deliberate choice, not an accident: the caller supplied the name,
and while per-secret policy is in force, conflating them costs debuggability
without buying confidentiality against a caller who can enumerate by other
means. A deployment that needs the two indistinguishable must collapse them
explicitly.

### What this does not do

It does not authorize the *consumer* of a direct delivery — that remains the
trusted adapter's decision, on the facts the D18D binding forwards. It does not
replace VLT06's engine, which still governs path-level operations. And it does
not make an unattested identity trustworthy; see P3.

Three limits worth stating plainly, because each is the kind of thing a reader
would otherwise assume works:

- **`privilege_tier` enforces nothing.** It is recorded and no component reads
  it. Enforcing it needs the caller's tier threaded down from the tool boundary,
  which no path does today. A field named like a control that gates nothing is
  a hazard, so it is called out here rather than left to be discovered.
- **The allow-list discriminates at host granularity.** On the real paths the
  attested identity is a host or package identity, so `allowed_agents` cannot
  separate two agents that share a host. Where that distinction matters, the
  agents must not share a host.
- **Denials are informative by construction.** A caller can tell "no such
  secret" from "wrong mode" from "not permitted", which reveals that a secret
  exists and something about its policy. That is the accepted trade — the caller
  supplied the name, and collapsing the cases costs debuggability. P7 limits the
  leak to callers who are at least admissible.

## Out of scope (future PRs)

- HCL policy DSL (HashiCorp Vault compatibility).
- Cedar (AWS).
- Rego / OPA bindings.
- ABAC / dynamic attribute fetch.
- Quorum decorator (e.g. "two admins must approve").
- Audit-trail integration — engine is pure today; an optional
  sink hook can be added at a higher layer.

## Citations

- HashiCorp Vault HCL ACL spec — model for path-capability DSL.
- AWS Cedar policy language.
- Open Policy Agent (Rego).
- VLT00-vault-roadmap.md — VLT06 layer purpose.
