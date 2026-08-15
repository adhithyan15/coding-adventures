# Chief of Staff Channel Crypto (Ruby)

This gem implements the portable D18F immutable encrypted-message profile and
the D18Q receiver-bound channel-key grant profile for Ruby Chief of Staff
agents. It consumes both shared fixture corpora and reproduces the production
Rust `D18M` message and `D18G` grant bytes exactly.

The implementation uses repository-owned SHA-256, Ed25519, X25519,
HKDF-SHA256, and XChaCha20-Poly1305 primitives. It never falls back to D19
`ACTM`, plaintext, a host crypto API, or a JSON-only envelope.

```ruby
message = CodingAdventures::ChiefOfStaffChannelCrypto.message_create(
  fields, plaintext, signing_secret_key, channel_master_key
)
durable_record = CodingAdventures::ChiefOfStaffChannelCrypto.message_serialize(message)
recovered = CodingAdventures::ChiefOfStaffChannelCrypto.message_verify(
  message, originator_public_key, channel_master_key
)
```

Public byte inputs are copied into frozen binary strings. `D18Message` and
`PortableKeyGrant` are frozen and return fresh copies from byte accessors.
Callers can inject UUID-v7, monotonic-clock, and secure-random sources for
deterministic tests and host-specific boundaries.

Errors are `MessageProfileError` values with the stable D18F `code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## D18Q channel-key grants

```ruby
receiver = CodingAdventures::ChiefOfStaffChannelCrypto::ReceiverKeyPair.generate
signer = CodingAdventures::ChiefOfStaffChannelCrypto::OriginatorSigningKey.generate
cmk = CodingAdventures::ChiefOfStaffChannelCrypto::ChannelMasterKey.generate
fields = CodingAdventures::ChiefOfStaffChannelCrypto::KeyGrantFields.new(
  originator_id, receiver_id, channel_id, key_epoch
)
grant = CodingAdventures::ChiefOfStaffChannelCrypto.seal_channel_key(
  fields, cmk, receiver.public_key, signer
)
durable_grant = CodingAdventures::ChiefOfStaffChannelCrypto.grant_serialize(grant)
```

Opening enforces the normative identity, channel, signature, X25519, HKDF, and
AEAD validation order before returning a managed `ChannelMasterKey`.
`ReceiverEpochKeys` retains historic keys while rejecting decreasing or
conflicting grants, and `plan_rotation` returns a complete receiver-sorted
prospective plan without claiming D18P durable activation. Secret containers
support controlled overwrite and report Ruby's honest `best_effort` erasure
capability; the runtime cannot guarantee erasure of every copied string buffer.

## Development

```bash
bundle install
bundle exec rake test
gem build coding_adventures_chief_of_staff_channel_crypto.gemspec
```

The fixtures contain deterministic private keys, channel keys, and nonces for
tests only. Never use them in production.
