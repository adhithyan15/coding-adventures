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
