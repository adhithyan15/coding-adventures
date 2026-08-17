# Chief of Staff Channel Crypto (Elixir)

This package implements the portable D18F immutable encrypted-message profile
and D18Q receiver-bound channel-key grant profile for Elixir Chief of Staff
agents. It consumes both shared fixture corpora and reproduces the production
Rust `D18M` message and `D18G` grant bytes exactly.

The implementation uses repository-owned SHA-256, Ed25519, X25519,
HKDF-SHA256, and XChaCha20-Poly1305 primitives. It never falls back to D19
`ACTM`, plaintext, host cryptographic algorithms, or a JSON-only envelope.

```elixir
message =
  CodingAdventures.ChiefOfStaffChannelCrypto.message_create(
    fields,
    plaintext,
    signing_secret_key,
    channel_master_key
  )

durable_record =
  CodingAdventures.ChiefOfStaffChannelCrypto.message_serialize(message)

recovered =
  CodingAdventures.ChiefOfStaffChannelCrypto.message_verify(
    message,
    originator_public_key,
    channel_master_key
  )
```

Messages and creation fields are immutable Elixir structs containing immutable
binaries. Callers can inject UUID-v7 and monotonic-clock sources through
`message_create_with_sources/6` for deterministic tests and host-specific
clocks. `MonotonicUuidV7Generator` provides pure explicit state for ordered
same-millisecond identifiers.

Errors are `MessageProfileError` values with the stable D18F `code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## D18Q channel-key grants

```elixir
alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants

receiver = Grants.generate_receiver_key_pair()
signer = Grants.generate_originator_signing_key()
cmk = Grants.generate_channel_master_key()
fields = Grants.grant_fields(originator_id, receiver_id, channel_id, key_epoch)

grant =
  Grants.seal_channel_key(
    fields,
    cmk,
    Grants.receiver_public_key(receiver),
    signer
  )

durable_grant = Grants.grant_serialize(grant)
```

Opening enforces the normative identity, channel, signature, X25519, HKDF, and
AEAD validation order before returning a redacted `ChannelMasterKey`.

### Verifying a grant you cannot open

Opening needs the receiver's private key, because unwrapping performs an X25519
agreement. An originator holds no receiver secrets, so it cannot open the grants
it seals — yet D18T requires it to verify the originator signature on every
receiver's grant before a rotation candidate may be offered to key custody.
`verify_grant_signature/5` is that weaker, receiver-key-free check:

```elixir
:ok =
  Grants.verify_grant_signature(
    grant, expected_originator_id, expected_receiver_id, channel_id, originator_public_key
  )
```

It proves the originator signed this exact `{originator, receiver, channel,
epoch, ephemeral key, nonce, wrapped CMK}` tuple. It proves nothing about
whether the wrapped CMK decrypts — only unwrapping can establish that, so a
caller verifying grants it cannot open must not treat `:ok` as proof the
receiver will be able to use the grant. Both entry points share one verification
path, so they always agree on binding order and stable error codes.

`ReceiverEpochKeys` retains historic keys in explicit immutable state while
rejecting decreasing or conflicting grants, and `plan_rotation/6` returns a
receiver-sorted prospective plan without claiming D18P durable activation.

Secret-bearing structs redact key bytes. The package reports
`not_enforceable` secret erasure because immutable, garbage-collected BEAM
values cannot promise physical overwrite of every runtime copy. Destroy
helpers invalidate and zero the returned replacement value only; callers must
not interpret that as guaranteed erasure of the original immutable value.

## Development

```bash
mix deps.get
mix test --cover
mix compile --warnings-as-errors
```

The fixtures contain deterministic private keys, channel keys, and nonces for
tests only. Never use them in production.
