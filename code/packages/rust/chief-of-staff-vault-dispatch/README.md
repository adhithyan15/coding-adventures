# chief-of-staff-vault-dispatch

The D18D vault tools, implemented over the D18 vault runtime and registered
through the host's checked registration path.

## Where this sits in the stack

Three packages already existed and did not touch each other:

```
  chief-of-staff-tool-api        declares  vault.request_lease
                                           vault.request_direct
                                           ... and dispatches to handlers

  chief-of-staff-vault-runtime   owns      the secret material, the lease
                                           manager, and the trusted
                                           direct-delivery boundary

  chief-of-staff-host-runtime    enforces  which tools a host may register,
                                           the privilege ceiling, and the
                                           declared capabilities
```

The two vault tools were catalogued, but nothing implemented them — an agent
calling `vault.request_lease` got `ToolNotFound`. The vault was reachable in
principle and unreachable in practice. This crate is the missing edge.

## The two tools

| Tool | Capability | Returns |
|---|---|---|
| `vault.request_lease` | `vault:lease` | `{ vault_ref, expires_at_ms }` |
| `vault.request_direct` | `vault:direct` | `null` |

Both are `Tier2`.

`request_lease` hands back a **bearer capability**, not a secret. A `VaultRef`
is an opaque string that only the trusted host broker can redeem, and redeeming
it consumes it — a copied reference is worth nothing once the trusted handler
has used it.

`request_direct` hands back **nothing**. This is the design worth noticing. The
obvious alternative — return the secret so the caller can forward it — turns the
caller into a secret-handling component. Instead the vault runtime moves the
payload into a trusted delivery adapter and returns unit, so there is no return
path a secret could travel along. The return type has no room for one.

`consumer_agent_id` names a consumer the caller *asserts* is authorized. This
crate does not authorize it; the trusted adapter is the component entitled to
accept or refuse. Treating a caller-supplied name as evidence would let any
caller redirect a secret by renaming its destination.

For the adapter to refuse for a *reason*, it has to know what it is being asked,
so `deliver` receives a `VaultDirectRequest` carrying the requesting agent, the
session, and the secret name alongside the destination. Given only
`(consumer, payload)` the strongest rule an adapter can express is a global
destination allowlist — under which a caller cleared to send one secret to a
consumer is equally cleared to send *every* secret to it, because nothing in the
chain can tell the two requests apart.

**What this does not establish.** Forwarding those fields makes a decision
possible; it does not make one. There is still no per-secret policy — the
`privilege_tier` / `allowed_agents` / `allowed_mode` metadata is unimplemented,
and the tier and capability checks are per-*tool* and run once at registration.
So any caller that clears the tool gate can request any registered secret in
either mode, including leasing one meant to be direct-delivery only. And
`requesting_agent_id` is only as trustworthy as whatever populated it: if a host
lets a caller assert its own identity, an adapter authorizing on that field is
authorizing on the attacker's own claim. See D18D §7.2.

## Usage

```rust
use std::sync::Arc;

use chief_of_staff_vault_dispatch::VaultToolBridge;
use chief_of_staff_vault_runtime::ChiefVaultRuntime;

let vault = Arc::new(ChiefVaultRuntime::new());
// ... register secrets, construct your trusted delivery adapter ...

let bridge = VaultToolBridge::new(vault, delivery);

// At a real host boundary — this path applies the tool-allowed, tier, and
// capability checks, and a failure here should stop host startup.
bridge.register_into_host(&mut orchestrator_runtime)?;

// For local dispatch and tests — schema validation and dispatch, but no host
// profile to check against.
bridge.register_all(&mut tool_runtime)?;
```

## Invariants

The normative statement is D18D section 7.1; the short version:

- **V1 — no secret return channel.** Nothing derived from secret bytes reaches
  the handler output or the error.

  For `request_direct`'s `output` field specifically this is structural rather
  than a matter of handler discipline: the declared `output_schema` is JSON
  `null` and the tool runtime validates handler output against it after the
  handler returns. There is a test that registers a deliberately leaking handler
  under the real definition and asserts the runtime stops it.

  It is worth being exact about how far that reaches, because a guarantee
  believed to be wider than it is, is worse than none:

  | Field | Enforced by |
  |---|---|
  | `output` | the runtime, against `output_schema` |
  | `artifact_refs` | the handler — copied through unchecked |
  | `memory_refs` | the handler — copied through unchecked |
  | `events` | the handler — assembled *before* validation, published even on rejection |

  Both handlers here return those three empty, and a test pins it.
- **V2 — bounded, secret-free errors.** Every error message is a `&'static str`
  from the `errors` module. Nothing is interpolated.
- **V3 — handlers validate their own arguments.** The registry validates against
  `input_schema` first, but a `ToolHandler` is a trait object anything can call
  directly, so the checks are repeated here. No `unwrap`, no indexing, no
  arithmetic on caller-controlled values.
- **V4 — registration is the tier gate.** `register_into_host` goes through the
  host runtime's checked `register_handler`, and returns the failure rather than
  swallowing it. An unregistered vault tool is indistinguishable from a denied
  one from the agent's side, which is a miserable thing to debug at first use.

## Testing

```
cargo test -p chief-of-staff-vault-dispatch
```

23 integration tests plus a doctest. The suite has been mutation-checked: the
leak detector was confirmed to fail when a validation message was made to
interpolate the caller's secret name, and the structural V1 test counts handler
invocations so that it cannot pass on a schema-invalid argument that never
reached the handler at all.

The leak tests run through `invoke_with_events` and assert over the whole
`ToolExecutionTrace`, not the `ToolResult`. That distinction matters:
`ToolResult` has no `events` field, so a leak test built on it silently skips
the one channel the runtime does not validate and does publish even when it
rejects the call.
