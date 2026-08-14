# Chief of Staff Channel Crypto (Python)

This package implements the portable D18F immutable encrypted-message profile
for Python Chief of Staff agents. It consumes the shared fixture corpus in
`code/fixtures/chief-of-staff-message/v1` and produces the same authenticated
header, `D18M` version 1 record, canonical JSON, ciphertext, tag, and Ed25519
signature as the production Rust implementation.

The implementation uses repository-owned SHA-256, Ed25519, UUID, and
XChaCha20-Poly1305 primitives. It never falls back to D19 `ACTM`, plaintext, a
host crypto API, or a JSON-only envelope.

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

## Development

```bash
uv venv
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 \
  -e ../uuid -e ../ed25519 -e ../chacha20-poly1305 -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
