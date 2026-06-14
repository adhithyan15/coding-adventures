# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT04
  (`code/specs/VLT04-vault-recipients.md`).
- `Recipient` trait — every recipient has an opaque
  `recipient_id` and `wrap(file_key) -> WrappedKey`. Records carry
  a wrap-set `Vec<(recipient_id, WrappedKey)>` so the same file
  key can be wrapped to many recipients in parallel.
- `Identity` trait — the unwrap side. `try_unwrap(wrapped) ->
  Result<Option<Key>, _>`: `Ok(None)` = "not for me," `Ok(Some)` =
  success, `Err` = "for me but broken." `recipient_id()` is part
  of the trait so `try_unwrap_any` can dispatch wraps only to
  identities they were addressed to (avoids the "two identities of
  the same kind both attempt every wrap, one's failed AEAD looks
  indistinguishable from genuine tamper" failure mode).
- `PassphraseRecipient` — Argon2id-derived KEK, XChaCha20-Poly1305
  AEAD wrap. Wire format:
  `magic(2)"PR" || salt(16) || nonce(24) || ct(32) || tag(16)` =
  90 bytes. Stable per-passphrase recipient_id via HKDF.
- `X25519Recipient` / `X25519Identity` — age-style asymmetric
  wrap. Sender draws a fresh ephemeral X25519 keypair, ECDHs with
  the recipient pubkey, HKDF-derives a wrap key, AEADs the file
  key. Wire format:
  `magic(2)"X1" || ephemeral_pubkey(32) || nonce(24) || ct(32) || tag(16)`
  = 106 bytes. AAD = `magic || ephemeral_pubkey ||
  recipient_pubkey` so a captured wrap can't be rebound to a
  different recipient. Sender holds nothing afterwards (ephemeral
  private key dropped via `Zeroizing`).
- `wrap_for_all(file_key, &[&Recipient])` — convenience for
  building a record's wrap-set in one call.
- `try_unwrap_any(&[&Identity], wrap_set)` — convenience for the
  unwrap side. Dispatches wraps to the identity matching their
  `recipient_id`. Empty `recipient_id()` (default trait impl)
  falls back to "try every wrap" for the unique-per-kind case.
- `RecipientError` typed enum: `UnwrapFailed`,
  `MalformedWrappedKey`, `InvalidParameter`, `Csprng`, `Kdf`,
  `Hkdf`, `X25519`, `Aead`. `Display` strings sourced exclusively
  from this crate's literals.
- `PassphraseRecipient` and `X25519Identity` both implement `Drop`
  that wipes private key material via `Zeroizing`. Ephemeral
  X25519 secret keys inside `wrap` are also wrapped in
  `Zeroizing` so they wipe on every return path including errors.
- 24 unit tests covering: PassphraseRecipient round-trip, distinct
  blobs each wrap (fresh salt + nonce), stable recipient_id,
  wrong-passphrase rejection, body tamper rejection,
  X25519Recipient round-trip, blob length 106, distinct blobs
  each wrap (fresh ephemeral), wrong-identity rejection (Bob's
  identity on Alice's wrap), tamper rejection, recipient_id =
  pubkey, cross-kind isolation (passphrase identity returns None
  on X25519 blob and vice versa), multi-recipient flow (Alice +
  Bob both unwrap; Eve recovers nothing — `Ok(None)`, not error),
  mixed-kind wrap-set (passphrase + X25519), tamper of an
  addressed wrap propagates as `UnwrapFailed`, parameter
  validation (empty passphrase rejected), error-display-from-
  literals invariant, Drop safety, malformed-blob rejection.

### Security review

Round 1 found 1 LOW + 3 INFO. LOW fixed inline; INFO noted:

- **LOW** — `X25519Identity::from_secret_key(sk: [u8; 32])` accepted
  the secret key by value, leaving an unzeroized stack copy after
  return; `generate` compounded this by passing a stack-allocated
  `[u8; 32]` by value into `from_secret_key` and then zeroizing a
  *fresh* copy rather than the original. **Fixed:** signature now
  takes `Zeroizing<[u8; X25519_KEY_LEN]>` by value (moved into the
  identity); `generate` allocates the secret directly inside
  `Zeroizing` from the start so no plain `[u8; 32]` ever lives on
  the stack.
- **INFO** — `with_params` allows weak Argon2id parameters (m=8 KiB,
  t=1) for tests; production callers should use
  `with_default_params`. Documented.
- **INFO** — `PassphraseRecipient::recipient_id` is deterministic
  per passphrase, which means two users with the same passphrase
  have the same id (correlation channel for a passive observer of
  public wrap-sets). Documented as a design tradeoff.
- **INFO** — `recipient_id` allocates a fresh `Vec` each call.
  Minor perf; not security.

Round 2: not run (the LOW fix is a surgical signature change with
no behavior change beyond eliminating the stack copy; tests pass
unchanged).
