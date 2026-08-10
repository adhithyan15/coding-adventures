# Changelog

All notable changes to this package are documented here.

## [0.6.0] - 2026-08-09

### Added

- No-write passphrase rehydration of a durable generation-zero `PreparedInit`
  journal into its repository address and authority-anchored verifier.

### Security

- Wrong passphrases and unauthenticatable root wraps share the closed
  `AuthenticationFailed` result.
- Rehydration proves the decrypted authority, device-signing, and device-wrap
  private seeds reproduce the identities pinned in the signed bootstrap and
  authority-signed certificate before repository access.

## [0.5.0] - 2026-08-09

### Added

- Pure deterministic generation-zero preparation from an owned zeroizing
  passphrase, bounded KDF policy, advisory timestamp, and caller-filled CSPRNG
  block.
- Exact construction of the signed bootstrap, encrypted certificate and empty
  catalog, initial commit and announcement, repository address, recovery
  journal, intended active state, and authority-anchored verifier.

### Security

- Root wrapping uses Argon2id and XChaCha20-Poly1305 with exact AAD binding to
  the suite and vault ID.
- The passphrase, VRK, KEK, authority/device seeds and signing keys, X25519
  secret, local-secret plaintext, object randomness, and source CSPRNG block
  are held in wipe-on-drop containers.
- Preparation performs no external writes, so the complete exact
  `PreparedInit` journal can be atomically persisted before any remote effect.

## [0.4.0] - 2026-08-09

### Added

- An object-safe application repository and factory over any injected
  VLT-PM02 object store.
- Complete delegation for initialization, verified open, by-value publication,
  encrypted-object reads, commit reads, and bounded history.

### Security

- Production construction requires a caller-supplied unlocked
  `RepositoryVerifier`; there is no unchecked repository path.
- Repository and provider failures are translated to a closed payload-free
  application error taxonomy.
- Exact randomized and signed publication batches are consumed by value,
  preserving the crash journal's single-byte-sequence invariant.

## [0.3.0] - 2026-08-09

### Added

- Exact canonical `PreparedInit`, `Active`, and `PendingPublication` owner-state
  codecs with retry-stable repository journals.
- Byte-oriented injected bootstrap and atomic local-state store contracts.
- Domain-separated local-secret XChaCha20-Poly1305 sealing and opening.
- Random bootstrap locators and domain-separated authority fingerprints.

### Security

- State decoding cross-checks bootstrap, vault, authority, device,
  certificate-frame ID, announcement, commit ID, catalog, head, and counter
  relationships before recovery can use persisted bytes.
- Prepared initialization verifies the embedded-authority generation-zero
  bootstrap signature; pending publication rebinds announcement identity to
  the last active vault, device, and certificate.
- Pending mutations retain exact randomized and signed publication bytes, so a
  retry cannot equivocate at one reserved device counter.
- Store and state diagnostics remain closed and payload-free.

## [0.2.0] - 2026-08-09

### Added

- Exact canonical application wrappers for authority-signed device
  certificates and device-signed commits.
- An authority-anchored VLT-PM04 verifier for the single authorized Phase 1A
  device.

### Security

- Commit frames are ID-checked, AEAD-opened as the commit kind, strictly
  decoded, identity-bound, and Ed25519-verified before repository use.
- Verifier construction requires the locally pinned certificate frame to
  reproduce its expected object ID before authority verification.
- Announcements must match the authorized vault, device, certificate object,
  and signing key; all verifier failures remain payload-free.

## [0.1.0] - 2026-08-09

### Added

- Closed canonical codecs for local device secrets, item revisions, and
  bounded catalog snapshots.
- Lossless observed-set persistence, including retained add operations and
  removal tombstones.
- V1 HKDF subkey derivation and domain-separated XChaCha20-Poly1305 object
  framing over caller-provided randomness.

### Security

- Strict kind, vault, suite, bound, and AEAD checks before plaintext parsing.
- Zeroizing live keys, local secret state, object DEKs, and opened plaintext.
- Closed payload-free diagnostics and a capability-free package boundary.
