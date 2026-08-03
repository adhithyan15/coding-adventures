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
decryption also verifies the declared plaintext hash.

Secret key material is held in `Zeroizing` containers. Receiver epoch state
accepts a byte-identical retry of the current grant, rejects conflicts and
decreasing epochs, and retains older CMKs so historic messages remain readable.

The `wire` module provides bounded, versioned binary records for grants and
encrypted messages. Its stable storage keys sort messages by sequence and hash
receiver IDs before putting them into path-like keys, so an external identity
cannot inject storage path segments.

This crate deliberately does not implement storage or actor routing. The caller
must persist a sequence advance before calling message encryption; `SequenceCursor`
provides a fail-closed recovery and reservation protocol for that integration.

## Validation

```sh
cargo test -p chief-of-staff-channel-crypto -- --nocapture
cargo clippy -p chief-of-staff-channel-crypto --all-targets -- -D warnings
```
