# Chief of Staff Channel Crypto (Ruby)

This gem implements the portable D18F immutable encrypted-message profile for
Ruby Chief of Staff agents. It consumes the shared fixture corpus in
`code/fixtures/chief-of-staff-message/v1` and produces the same authenticated
header, `D18M` version 1 record, canonical JSON, ciphertext, tag, and Ed25519
signature as the production Rust implementation.

The implementation uses repository-owned SHA-256, Ed25519, and
XChaCha20-Poly1305 primitives. It never falls back to D19 `ACTM`, plaintext, a
host crypto API, or a JSON-only envelope.

```ruby
message = CodingAdventures::ChiefOfStaffChannelCrypto.message_create(
  fields, plaintext, signing_secret_key, channel_master_key
)
durable_record = CodingAdventures::ChiefOfStaffChannelCrypto.message_serialize(message)
recovered = CodingAdventures::ChiefOfStaffChannelCrypto.message_verify(
  message, originator_public_key, channel_master_key
)
```

All byte inputs are copied into frozen binary strings. `D18Message` is frozen
and returns fresh mutable copies from byte accessors. Callers can inject UUID-v7
and monotonic-clock sources through `message_create_with_sources` for
deterministic tests and host-specific clocks.

Errors are `MessageProfileError` values with the stable D18F `code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## Development

```bash
bundle install
bundle exec rake test
gem build coding_adventures_chief_of_staff_channel_crypto.gemspec
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
