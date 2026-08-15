# Chief of Staff Channel Crypto (Python)

This package implements the portable D18F immutable encrypted-message profile
and D18Q channel-key grant profile for Python Chief of Staff agents. It
consumes the shared fixture corpora in `code/fixtures/chief-of-staff-message/v1`
and `code/fixtures/chief-of-staff-channel-key-grant/v1`, producing the same
authenticated `D18M` messages and sealed `D18G` grants as the production Rust
implementation.

The implementation uses repository-owned SHA-256, HKDF, X25519, Ed25519, UUID,
and XChaCha20-Poly1305 primitives. It never falls back to D19 `ACTM`, plaintext,
a host crypto API, or a JSON-only envelope.

```python
from coding_adventures_chief_of_staff_channel_crypto import (
    message_create,
    message_serialize,
    message_verify,
)

message = message_create(fields, plaintext, signing_key, channel_master_key)
durable_record = message_serialize(message)
recovered = message_verify(message, public_key, channel_master_key)
```

`D18Message` and its creation-field values are frozen, slotted dataclasses.
Mutable byte inputs are copied to immutable `bytes`, so neither caller-owned
buffers nor field access can mutate a constructed message. Deterministic tests
and hosts can inject UUID-v7 and monotonic-clock sources through
`message_create_with_sources`.

Errors are `MessageProfileError` values with the stable D18F `code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## Channel-key grants

D18Q grants wrap one channel master key for one receiver using repository-owned
X25519, HKDF-SHA256, XChaCha20-Poly1305, and Ed25519 primitives. The API keeps
secret material behind managed objects, validates identities before signature,
key-agreement, and authentication work, and installs grants atomically into
monotonic receiver epoch state.

```python
from coding_adventures_chief_of_staff_channel_crypto import (
    ChannelMasterKey,
    KeyGrantFields,
    ReceiverEpochKeys,
    seal_channel_key,
)

cmk = ChannelMasterKey.generate()
grant = seal_channel_key(fields, cmk, receiver_public_key, signing_key)
receiver_epochs = ReceiverEpochKeys(
    fields.originator_id,
    fields.receiver_id,
    fields.channel_id,
    receiver_key_pair,
    signing_key.public_key,
)
receiver_epochs.install_grant(grant)
epoch_key = receiver_epochs.key(fields.key_epoch)
```

`plan_rotation` produces a pure, one-shot prospective grant plan and sorts
receivers by identity. Revoked receivers receive no new grant. Errors are
`KeyGrantProfileError` values with the stable D18Q code taxonomy. Because
Python and the repository primitives create immutable byte strings and
big-integer intermediates, `secret_erasure_capability()` honestly reports
`"not_enforceable"`; managed mutable buffers are still overwritten on
`destroy()`, but this is not a physical-erasure guarantee.

## Development

```bash
uv venv
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 \
  -e ../hmac -e ../hkdf -e ../x25519 -e ../uuid -e ../ed25519 \
  -e ../chacha20-poly1305 -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
