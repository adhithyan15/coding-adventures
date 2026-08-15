# chief-of-staff-channel-crypto

`chief-of-staff-channel-crypto` implements the cryptographic foundation for D18
one-way, append-only Chief of Staff channels.

An originator creates one 256-bit channel master key (CMK) per key epoch. The
same CMK is wrapped independently for every authorized receiver with an
ephemeral X25519 exchange, HKDF-SHA256, and XChaCha20-Poly1305. Every sealed
grant is signed with the originator's Ed25519 key and binds the originator,
receiver, channel, epoch, ephemeral public key, wrapping nonce, and wrapped CMK.
The orchestrator can route grants without learning the CMK.

Messages use XChaCha20-Poly1305 with the canonical 16-byte channel identifier
followed by the globally monotonic 64-bit sequence as the 24-byte nonce. Their
canonical, length-framed header is both AEAD additional data and the Ed25519
signature input. The header includes a SHA-256 plaintext digest, so successful
decryption also verifies the declared plaintext hash. Authenticated message
fields, headers, ciphertext, tags, and signatures are private after construction;
callers receive only read-only borrowed or copied views.

Secret key material is held in `Zeroizing` containers. Receiver epoch state
accepts a byte-identical retry of the current grant, rejects conflicts and
decreasing epochs, and retains older CMKs so historic messages remain readable.

The portable key-grant and rotation contract is specified by
`code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md`. This crate is the
Rust reference implementation for its cryptographic framing and receiver-state
rules; crash-safe activation of a rotated epoch remains a caller-owned durable
composition boundary.

The `grant_profile` module adds validated immutable grant values,
signature-only provenance verification for orchestration layers that do not
hold receiver private keys,
explicit-material sealing, stable cross-language errors, receiver epoch
installation, pure ordered rotation planning, and the Rust `guaranteed`
controlled-destruction declaration without changing D18G version 1 bytes. The
checked-in corpus under `code/fixtures/chief-of-staff-channel-key-grant/v1`
locks every derivation intermediate, record, failure order, state transition,
and A+B to B-only rotation for the remaining language ports.

The `wire` module provides bounded, versioned binary records for grants and
encrypted messages. Its stable storage keys sort messages by sequence and hash
receiver IDs before putting them into path-like keys, so an external identity
cannot inject storage path segments.

The `profile` module implements the portable D18F contract on top of those
unchanged bytes: UUID-v7 and MIME validation, stable cross-language errors,
canonical lossless JSON, epoch-key resolution, and an injectable monotonic
UUID-v7 generator. The checked-in fixtures under
`code/fixtures/chief-of-staff-message/v1` lock the authenticated header, D18M
record, JSON representation, verification order, and failure taxonomy for
other language implementations.

This crate deliberately does not implement storage or actor routing. The caller
must persist a sequence advance before calling message encryption; `SequenceCursor`
provides a fail-closed recovery and reservation protocol for that integration.

## Validation

```sh
cargo test -p chief-of-staff-channel-crypto -- --nocapture
cargo clippy -p chief-of-staff-channel-crypto --all-targets -- -D warnings
```
