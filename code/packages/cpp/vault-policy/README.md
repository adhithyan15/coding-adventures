# vault-policy (C++)

A pluggable **authorization policy engine** (VLT06), **header-only** in pure ISO
C++17 (namespace `ca::vault_policy`). A faithful port of the Rust
[`vault-policy`](../../rust/vault-policy) crate.

## What it does

Authentication says *who*; this layer says *what they can do*. You build a
policy engine out of a role-based table plus composable decorators, then ask it
to `decide` a request described by a `PolicyContext`. Decisions **fail closed**:
every denial carries a static `Reason` from a fixed table, never
attacker-controlled bytes.

## Engines

- `SimpleRbacEngine` — role × (action, resource-pattern) table (exact or `"*"`).
- `AllOf` / `AnyOf` — boolean AND / OR of inner engines.
- `RequireFactor` — require an auth factor present, then defer to the inner.
- `TimeBound` — only allow within `[start, end]` UNIX seconds.

## Design notes

- **Virtual dispatch + `unique_ptr` ownership.** Rust's `Box<dyn PolicyEngine>`
  trait objects become an abstract `PolicyEngine` base with concrete subclasses;
  decorators own their inner engines via `std::unique_ptr`.
- **`std::optional` / value types.** Rust's `enum Decision { Allow, Deny(Reason) }`
  becomes a `Decision` value with `reason()` returning `std::optional<Reason>`;
  `Reason` wraps a static literal from the fixed `Reason::*` table.
- **Header-only.** `#include "vault_policy.hpp"` and go.

## Usage

```cpp
#include "vault_policy.hpp"
using namespace ca::vault_policy;

auto rbac = std::make_unique<SimpleRbacEngine>();
rbac->assign_role("alice", "admin");
rbac->grant("admin", "delete", "*");

// Require WebAuthn, only during a maintenance window.
TimeBound engine(
    std::make_unique<RequireFactor>(std::move(rbac), "webauthn-prf"), 1000, 2000);

PolicyContext ctx;
ctx.principal = "alice";
ctx.action = "delete";
ctx.resource = "vault/login/abc";
ctx.factors = {"password", "webauthn-prf"};
ctx.time = 1500;

Decision d = engine.decide(ctx);   // d.is_allow() == true
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
