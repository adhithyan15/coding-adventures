# Chief of Staff Vault Runtime

This crate is the trusted host-side broker for short-lived secret leases and
direct delivery to approved consumers.
`request_lease` returns the canonical agent-facing receipt:

```text
{ vault_ref, expires_at_ms }
```

`vault_ref` is an opaque bearer capability. Agents may pass it only to an
approved host operation that explicitly accepts a Vault reference; they cannot
resolve it into secret bytes. The trusted host atomically consumes the reference,
uses the zeroizing payload within that operation, and makes the reference unusable
after consumption, release, revocation, or expiry.

The receipt never contains plaintext, ciphertext, or a decryption key. Its `Debug`
implementation also redacts the bearer reference so routine diagnostics do not
turn logs into an alternate secret channel.

`request_direct` transfers an owned, zeroizing payload to a replaceable
`VaultDirectDelivery` implementation. That trusted adapter can route bytes over
an authenticated browser, agent, or host channel, while the requesting agent
receives only success or a bounded secret-free error. Missing secrets, invalid
consumer identifiers, unavailable consumers, and rejected deliveries all fail
closed without returning secret material.

## Per-secret admission policy

Every registered secret carries a `SecretPolicy` (VLT06). It is required at
registration, not defaulted: a permissive default leaves a secret unguarded
silently, and a restrictive one surfaces only when a legitimate caller is
refused. `SecretPolicy::unrestricted()` is the named, greppable way to say a
secret really is open.

| Field | Effect |
|---|---|
| `allowed_mode` | `Direct`, `Leased`, or `Both` — a direct-only secret cannot be leased |
| `allowed_agents` | `Any`, or `Only(set)` of attested agent identities |
| `privilege_tier` | recorded only; **nothing reads it yet** |
| `rotated_at_ms` | when the secret last changed |

Mode matters more than it looks. Direct delivery exists so plaintext never
reaches the requesting agent — it is the mode for a bank password. Letting a
direct-only secret be *leased* would not weaken that, it would invert it: the
caller gets exactly the material direct mode withholds, by asking differently.

**An absent identity is refused under `Only`, never treated as unconstrained.**
Written the natural way, the comparison succeeds vacuously when both sides are
absent — and in this stack only `agent_id` is host-attested, while `user_id` and
`session_id` are always `None` outside tests, so a rule over either would have
admitted everyone while reading as enforcement.

## Rotation revokes

`register_secret` revokes every outstanding lease over the previous value. A
lease holds its own copy of the payload from issue time, so overwriting the
stored value alone would leave the old one redeemable — and you rotate precisely
because the old value is compromised.

The runtime keeps a per-secret index of issued lease ids to make that possible.
It is bounded (1024 per secret, failing closed) and pruned on redemption,
revocation, and expiry, because an unbounded second table over the
agent-reachable path would reintroduce the exhaustion the lease layer was
hardened against.

The expiry sweep tests whether a lease is still *usable*, not whether it is
still present: `LeaseManager::lookup` returns `Ok` for expired and revoked
leases on purpose, so a presence check reclaims nothing — and a cap that never
reclaims is worse than no cap, since 1024 one-millisecond leases would refuse
the secret permanently.
