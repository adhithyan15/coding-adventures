# vault-policy (C)

A pluggable **authorization policy engine** (VLT06), in pure ISO C17. A faithful
port of the Rust [`vault-policy`](../../rust/vault-policy) crate.

## What it does

Authentication says *who*; this layer says *what they can do*. You build a
policy engine out of a role-based table plus composable decorators, then ask it
to decide a request described by a `VpContext`. Decisions **fail closed**: every
denial carries a static reason string from a fixed table (`vp_reason_*`), never
attacker-controlled bytes.

## Engines

| Kind                | Meaning                                              |
| ------------------- | ---------------------------------------------------- |
| RBAC (`vp_rbac_*`)  | role × (action, resource-pattern) table              |
| `vp_all_of`         | boolean AND of inner engines (all must allow)        |
| `vp_any_of`         | boolean OR of inner engines (any allow wins)         |
| `vp_require_factor` | wrap an engine, also require an auth factor present  |
| `vp_time_bound`     | wrap an engine, only allow within `[start, end]` secs |

## Ownership

An engine (`VpEngine *`) is an owned tree: the decorator constructors **take
ownership** of the inner engines handed to them, so you compose bottom-up and
release the whole tree with a single `vp_engine_free`. A constructor returns
`NULL` on allocation failure, having freed the inner engines you passed in. A
`VpContext` is a borrowed view of caller-owned strings and is not freed.

## Design notes

- **Tagged-union engine tree.** Rust's `Box<dyn PolicyEngine>` trait objects
  become a tagged `VpEngine` (RBAC leaf or decorator node); `vp_engine_decide`
  is a recursive interpreter over it. `Result`/`Decision` becomes a small
  `VpDecision { allow; reason }` value.
- **Static reasons.** Denial reasons are pointers into a fixed `vp_reason_*`
  table; compare returned reasons by identity or `strcmp`.
- **Robust by construction.** `require-factor` copies its factor kind (no
  dangling `&'static str` footgun), and every growable table guards against
  `size_t` overflow.

## Usage

```c
#include "vault_policy.h"

VpEngine *rbac = vp_rbac_new();
vp_rbac_assign_role(rbac, "alice", "admin");
vp_rbac_grant(rbac, "admin", "delete", "*");

/* Require WebAuthn for the admin's actions, only during a maintenance window. */
VpEngine *e = vp_time_bound(vp_require_factor(rbac, "webauthn-prf"), 1000, 2000);

const char *const factors[] = {"password", "webauthn-prf"};
VpContext ctx = {"alice", "delete", "vault/login/abc", factors, 2,
                 1500, NULL, NULL, 0};
VpDecision d = vp_engine_decide(e, &ctx);   /* d.allow == 1 */

vp_engine_free(e);
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
