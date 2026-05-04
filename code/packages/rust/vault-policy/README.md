# `coding_adventures_vault_policy` — VLT06

Pluggable **policy engine** for the Vault stack. Authentication
(VLT05) says *who*; this crate decides *what they can do*.

## What's included (v0.1)

- **`PolicyEngine`** trait + `PolicyContext` + `Decision`.
- **`SimpleRbacEngine`** — role × `(action, resource_pattern)`
  table with `*` wildcard. Fits a Bitwarden-class app.
- Composition decorators: **`AllOf`**, **`AnyOf`**,
  **`RequireFactor`**, **`TimeBound`**.

## Quick example

```rust
use coding_adventures_vault_policy::{
    AllOf, Decision, PolicyContext, PolicyEngine,
    RequireFactor, SimpleRbacEngine, TimeBound,
};
use std::collections::HashMap;

let mut rbac = SimpleRbacEngine::new();
rbac.assign_role("alice", "admin");
rbac.grant("admin", "delete", "*");

// Compose: admin role + WebAuthn-PRF factor + within window.
let with_factor = Box::new(RequireFactor::new(Box::new(rbac), "webauthn-prf"));
let with_time = Box::new(TimeBound::new(with_factor, 1_700_000_000, 1_900_000_000));
let policy = AllOf::new(vec![with_time]);

let ctx = PolicyContext {
    principal: "alice".into(),
    action:    "delete".into(),
    resource:  "vault/login/abc".into(),
    factors:   vec!["password".into(), "webauthn-prf".into()],
    time:      1_800_000_000,
    metadata:  HashMap::new(),
};
assert_eq!(policy.decide(&ctx), Decision::Allow);
```

## Decisions are inert

`Decision::Deny(Reason)` carries a `&'static str` chosen from a
fixed table — the engine never quotes attacker-controlled bytes
back to the caller, so a malicious principal name in a deny
message cannot inject content into logs.

## Where it fits

```text
                ┌──────────────────────────────────────┐
                │  application                         │
                └──────────────┬───────────────────────┘
                               │
                ┌──────────────▼──────────────────────┐
                │  vault-auth (VLT05) → AuthAssertion │
                │  (kind, mode, key_contribution)     │
                └──────────────┬──────────────────────┘
                               │  factors: ["password", "totp"]
                ┌──────────────▼──────────────────────┐
                │  vault-policy (VLT06)            ◄  │  THIS CRATE
                │  decide(ctx) -> Allow | Deny(r)    │
                └──────────────┬──────────────────────┘
                               │  Allow → proceed
                               ▼
                ┌──────────────────────────────────────┐
                │  vault-key-custody (VLT03) /         │
                │  storage-core / VLT01 / engines …    │
                └──────────────────────────────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT06-vault-policy.md`](../../../specs/VLT06-vault-policy.md).
