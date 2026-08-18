# Changelog

## Unreleased

- **Breaking**: `register_secret` now takes a `SecretPolicy`, and
  `request_lease` takes a `VaultLeaseRequest` carrying the requesting agent.
  Implements VLT06's per-secret admission policy: each secret declares
  `allowed_agents` and `allowed_mode`, and both request paths apply them before
  the payload leaves storage. Previously any caller that cleared the D18D tool
  gate — which is per-tool and runs once at registration — could name any
  registered secret in either mode, including leasing one configured for direct
  delivery only, which hands over exactly the material direct mode withholds.
- **Security**: rotating or re-registering a secret now revokes every
  outstanding lease over the previous value. A lease holds its own copy of the
  payload taken at issue time, so overwriting the stored value alone left the
  old one redeemable for the remaining lifetime — meaning a secret rotated
  *because it was compromised* kept serving the compromised value. Tightening a
  policy had the same shape.
- **Security**: `request_lease` holds the secrets lock from the admission
  decision through to indexing the new lease. Releasing it earlier let a
  concurrent rotation drain the index in between, leaving a live capability over
  pre-rotation bytes that nothing could revoke. Pinned by a threaded test — the
  barrier-synchronised version of that test did *not* reproduce the race, so the
  test is a sustained hammer instead.
- **Security**: the lease index is bounded per secret
  (`MAX_TRACKED_LEASES_PER_SECRET`, 1024) and pruned on redemption, revocation,
  and — via a sweep at issue time — expiry. The sweep tests lease *usability*,
  not presence: `lookup` deliberately returns `Ok` for expired and revoked
  leases, so a presence test reclaimed nothing and turned the cap into a
  permanent wedge (1024 one-millisecond leases refused a secret forever).
  Unbounded, the index was a second table over the same agent-driven path as
  the lease table, reintroducing the exhaustion that layer had been hardened
  against. Reaching the bound fails closed: an unrevocable capability is worse
  than a refused request.
- `SecretPolicy::privilege_tier` is recorded and read by nothing. The doc says
  so outright rather than implying another layer interprets it.
- The payload now lives in a private `custody` module reachable only through an
  accessor that requires proof of admission, so "refuse before materializing"
  cannot be reordered from anywhere else in the crate.

- **Breaking**: `VaultDirectDelivery::deliver` and
  `ChiefVaultRuntime::request_direct` now take a `VaultDirectRequest` carrying
  the requesting agent, the session, and the secret name alongside the
  destination, instead of a bare `consumer_agent_id`. An adapter told only the
  destination cannot authorize anything: the strongest rule it can express is a
  global destination allowlist, under which a caller cleared to send one secret
  to a consumer is equally cleared to send every secret to it, because nothing
  in the chain can tell the two requests apart. That is a confused deputy — the
  adapter holds the authority but not the facts. Naming the requester and the
  secret does not authorize anything by itself; it is the precondition for an
  adapter that wants to.

- Add a replaceable trusted direct-delivery boundary that transfers zeroizing
  secret payloads to approved consumers while exposing only success or bounded,
  secret-free failures to callers.
- Define the opaque `{ vault_ref, expires_at_ms }` receipt as the canonical
  agent-facing lease contract and redact the bearer reference from `Debug` output.
