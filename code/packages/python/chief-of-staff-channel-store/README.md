# Chief of Staff Channel Store (Python)

This package is the Python implementation of the
[D18P portable durable-channel profile](../../../specs/D18P-chief-of-staff-durable-channel-profile.md).
It consumes the repository's single shared fixture corpus at
`code/fixtures/chief-of-staff-channel/v1/manifest.json`.

It provides exact `D18C`, `D18H`, `D18S`, and `D18A` codecs; deterministic
`chief-channels` keys; an injected atomic storage protocol; reserve-before-
encrypt recovery; immutable messages and grants; ordered paging; independent
receiver cursors; irreversible destruction; and structurally separate
`DurableOriginator` and `DurableReceiver` APIs.

The package delegates `D18M` encryption and verification to the Python
`chief-of-staff-channel-crypto` package. Backend durability, clocks, UUID
generation, entropy, and key custody are injected. `D18G` grant creation,
opening, and rotation remain owned by issue #141: D18P persists opaque grant
bytes only after enforcing membership, then invokes the receiver key provider
only after the exact grant record is found.

## Development

```bash
uv venv
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 \
  -e ../uuid -e ../ed25519 -e ../chacha20-poly1305 \
  -e ../chief-of-staff-channel-crypto -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

The shared fixture's private keys and channel master keys are public test-only
material. Errors, records, and APIs must never log or serialize plaintext,
channel master keys, private keys, or opened grant contents.
